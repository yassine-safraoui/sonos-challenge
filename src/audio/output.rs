use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BuildStreamError, DefaultStreamConfigError, DeviceNameError, Host, OutputCallbackInfo,
    PauseStreamError, PlayStreamError, Sample, SampleFormat, Stream, StreamConfig,
};
use hound::WavSpec;
use log::{debug, error, warn};
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
    DeviceNotFound(String),
    DeviceNameUnavailable(DeviceNameError),
    DefaultConfigUnavailable(DefaultStreamConfigError),
    UnsupportedSampleFormat(SampleFormat),
    StreamBuildFailed(BuildStreamError),
    StreamPlayFailed(PlayStreamError),
    StreamPauseFailed(PauseStreamError),
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
    pub fn new(filepath: &str, spec: WavSpec) -> Result<Self, WavOutputError> {
        match hound::WavWriter::create(filepath, spec) {
            Ok(w) => Ok(WavAudioOutput { writer: w }),
            Err(e) => Err(WavOutputError::HoundError(e)),
        }
    }
    pub fn write_samples(&mut self, samples: &[i16]) -> Result<(), WavOutputError> {
        samples.iter().try_for_each(|&s| {
            self.writer
                .write_sample(s)
                .map_err(WavOutputError::HoundError)
        })
    }
    pub fn finalize(self) -> Result<(), WavOutputError> {
        self.writer.finalize().map_err(WavOutputError::HoundError)
    }
}

pub struct SpeakerOutputBuilder {
    host: Host,
    device_name: Option<String>,
}
impl Default for SpeakerOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeakerOutputBuilder {
    pub fn new() -> Self {
        SpeakerOutputBuilder {
            host: cpal::default_host(),
            device_name: None,
        }
    }
    pub fn list_output_devices(&self) -> Vec<String> {
        let mut device_names = Vec::new();
        match self.host.output_devices() {
            Ok(devices) => {
                for (index, device) in devices.enumerate() {
                    match device.name() {
                        Ok(name) => device_names.push(name),
                        Err(e) => {
                            warn!("Could not get device name: {}", e);
                            device_names.push(format!("Unknown Device {}", index));
                        }
                    }
                }
            }
            Err(e) => warn!("Could not get output devices: {}", e),
        }
        device_names
    }
    pub fn with_output_device(&mut self, name: &str) -> &Self {
        self.device_name = Some(name.to_string());
        self
    }
    pub fn build(&mut self) -> Result<SpeakerOutput, SpeakerOutputError> {
        let device = match self.device_name.as_deref() {
            None => self
                .host
                .default_output_device()
                .ok_or(SpeakerOutputError::NoOutputDevice)?,
            Some(target_name) => {
                let devices = self.host.output_devices().map_err(|e| {
                    warn!("Could not get output devices: {}", e);
                    SpeakerOutputError::NoOutputDevice
                })?;

                let device = devices.enumerate().find_map(|(index, d)| match d.name() {
                    Ok(n) if n == target_name => Some(d),
                    Err(_) if format!("Unknown Device {}", index) == target_name => Some(d),
                    _ => None,
                });

                device.ok_or_else(|| SpeakerOutputError::DeviceNotFound(target_name.to_string()))?
            }
        };
        let supported_config = device
            .default_output_config()
            .map_err(SpeakerOutputError::DefaultConfigUnavailable)?;
        let sample_format = supported_config.sample_format();
        debug!("Default supported output config: {:?}", supported_config);
        debug!("Sample format: {:?}", sample_format);
        let config: StreamConfig = supported_config.into();
        debug!("Default output config: {:?}", config);

        // 10-second buffer at 44.1k mono
        let rb = HeapRb::<i16>::new(44100 * 2 * 10);
        let (producer, consumer) = rb.split();

        let err_fn = |err| error!("Stream error: {}", err);

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
                    .map_err(SpeakerOutputError::StreamBuildFailed)?
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
                    .map_err(SpeakerOutputError::StreamBuildFailed)?
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
                    .map_err(SpeakerOutputError::StreamBuildFailed)?
            }
            other => return Err(SpeakerOutputError::UnsupportedSampleFormat(other)),
        };
        stream
            .play()
            .map_err(SpeakerOutputError::StreamPlayFailed)?;

        Ok(SpeakerOutput { stream, producer })
    }
}
pub struct SpeakerOutput {
    stream: Stream,
    producer: HeapProd<i16>,
}

impl SpeakerOutput {
    pub fn play_samples(&mut self, samples: &[i16]) -> usize {
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
    pub fn play_sample(&mut self, sample: i16) -> Result<(), i16> {
        while self.producer.is_full() {}
        match self.producer.try_push(sample) {
            Ok(_) => Ok(()),
            Err(s) => Err(s),
        }
    }
    pub fn pause(&self) -> Result<(), SpeakerOutputError> {
        self.stream
            .pause()
            .map_err(SpeakerOutputError::StreamPauseFailed)
    }
    pub fn start(&self) -> Result<(), SpeakerOutputError> {
        self.stream
            .play()
            .map_err(SpeakerOutputError::StreamPlayFailed)
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
