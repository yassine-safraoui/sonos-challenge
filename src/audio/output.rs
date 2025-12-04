use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, OutputCallbackInfo, Sample, SampleFormat, Stream, StreamConfig};
use hound::WavSpec;
use log::{debug, warn};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use std::fmt;
use std::fs::File;
use std::io::BufWriter;

pub struct WavAudioOutput {
    writer: hound::WavWriter<BufWriter<File>>,
}

#[derive(Debug)]
pub enum SpeakerOutputError {
    NoOutputDevice,
    DeviceNameUnavailable,
    DefaultConfigUnavailable,
    UnsupportedSampleFormat(SampleFormat),
    StreamBuildFailed(String),
    StreamPlayFailed,
    StreamPauseFailed,
}

#[derive(Debug)]
pub enum WavOutputError {
    HoundError(hound::Error),
    IoError(std::io::Error),
}
impl fmt::Display for WavOutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WavOutputError::HoundError(e) => write!(f, "Hound error: {}", e),
            WavOutputError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl WavAudioOutput {
    pub fn init(filepath: &str, spec: WavSpec) -> Result<Self, WavOutputError> {
        match hound::WavWriter::create(filepath, spec) {
            Ok(w) => Ok(WavAudioOutput { writer: w }),
            Err(e) => Err(WavOutputError::HoundError(e)),
        }
    }
    pub fn write_samples(&mut self, samples: &[i16]) -> Result<(), WavOutputError> {
        for &sample in samples {
            if let Err(e) = self.writer.write_sample(sample) {
                return Err(WavOutputError::HoundError(e));
            }
        }
        Ok(())
    }
    pub fn finalize(self) -> Result<(), WavOutputError> {
        match self.writer.finalize() {
            Ok(_) => Ok(()),
            Err(e) => Err(WavOutputError::HoundError(e)),
        }
    }
}

pub struct SpeakerOutput {
    _host: Host,     // keep alive
    _device: Device, // keep alive
    stream: Stream,
    producer: HeapProd<i16>,
}

impl SpeakerOutput {
    pub fn init() -> Result<Self, SpeakerOutputError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(SpeakerOutputError::NoOutputDevice)?;
        match device.name() {
            Ok(name) => debug!("Using output device: {}", name),
            Err(_) => return Err(SpeakerOutputError::DeviceNameUnavailable),
        }
        let supported_config = device
            .default_output_config()
            .map_err(|_| SpeakerOutputError::DefaultConfigUnavailable)?;
        let sample_format = supported_config.sample_format();
        debug!("Default supported output config: {:?}", supported_config);
        debug!("Sample format: {:?}", sample_format);
        let config: StreamConfig = supported_config.into();
        debug!("Default output config: {:?}", config);

        // 10 second buffer at 44.1k mono
        let rb = HeapRb::<i16>::new(44100 * 2 * 10);
        let (producer, consumer) = rb.split(); // NO locks needed

        let err_fn = |err| eprintln!("Stream error: {}", err);

        // Build stream using a single fill function
        let stream = match sample_format {
            SampleFormat::F32 => {
                let mut cons = consumer;
                device
                    .build_output_stream(
                        &config,
                        move |out: &mut [f32], info| fill_from_consumer(&mut cons, out, info),
                        err_fn,
                        None,
                    )
                    .map_err(|e| SpeakerOutputError::StreamBuildFailed(e.to_string()))?
            }
            SampleFormat::I16 => {
                let mut cons = consumer;
                device
                    .build_output_stream(
                        &config,
                        move |out: &mut [i16], info| fill_from_consumer(&mut cons, out, info),
                        err_fn,
                        None,
                    )
                    .map_err(|e| SpeakerOutputError::StreamBuildFailed(e.to_string()))?
            }
            SampleFormat::U16 => {
                let mut cons = consumer;
                device
                    .build_output_stream(
                        &config,
                        move |out: &mut [u16], info| fill_from_consumer(&mut cons, out, info),
                        err_fn,
                        None,
                    )
                    .map_err(|e| SpeakerOutputError::StreamBuildFailed(e.to_string()))?
            }
            other => return Err(SpeakerOutputError::UnsupportedSampleFormat(other)),
        };
        stream
            .play()
            .map_err(|_| SpeakerOutputError::StreamPlayFailed)?;

        Ok(SpeakerOutput {
            _host: host,
            _device: device,
            stream,
            producer,
        })
    }
    pub fn push_slice(&mut self, samples: &[i16]) -> usize {
        while self.producer.vacant_len() < samples.len() {}
        let pushed_count = self.producer.push_slice(samples);
        if pushed_count < samples.len() {
            warn!(
                "SpeakerOutput buffer full, dropped {} samples",
                samples.len() - pushed_count
            );
        }
        pushed_count
    }
    pub fn push(&mut self, sample: i16) -> Result<(), i16> {
        while self.producer.is_full() {}
        match self.producer.try_push(sample) {
            Ok(_) => Ok(()),
            Err(s) => Err(s),
        }
    }
    pub fn pause(&self) -> Result<(), SpeakerOutputError> {
        self.stream
            .pause()
            .map_err(|_| SpeakerOutputError::StreamPauseFailed)
    }
    pub fn start(&self) -> Result<(), SpeakerOutputError> {
        self.stream
            .play()
            .map_err(|_| SpeakerOutputError::StreamPlayFailed)
    }
}

/// Single shared callback implementation
fn fill_from_consumer<T>(consumer: &mut HeapCons<i16>, out: &mut [T], _info: &OutputCallbackInfo)
where
    T: Sample + cpal::FromSample<i16>,
{
    for frame in out.chunks_mut(2) {
        let s = consumer.try_pop().unwrap_or(Sample::EQUILIBRIUM);
        for sample in frame {
            *sample = Sample::from_sample::<i16>(s);
        }
    }
}
