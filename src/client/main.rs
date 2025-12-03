use log::{LevelFilter, debug, error, info};
use sonos_challenge::audio::{AudioMessage, Serializable};
use sonos_challenge::network::tcp::TcpClient;
use std::fs::File;
use std::io::{BufWriter, Result};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .init();

    let mut tcp: Result<TcpClient>;
    loop {
        tcp = TcpClient::init("localhost:8080");
        if tcp.is_err() {
            info!("Couldn't connect to server at localhost:8080");
            sleep(Duration::from_secs(1));
            continue;
        }
        break;
    }
    let mut tcp = tcp.unwrap();
    let mut buffer = Vec::new();
    let mut writer: Option<hound::WavWriter<BufWriter<File>>> = None;
    use std::sync::atomic::{AtomicBool, Ordering};

    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    ctrlc::set_handler(move || {
        debug!("Ctrl-C received, stopping client");
        s.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    sleep(Duration::from_secs(1));
    loop {
        buffer.clear();
        if stop.load(Ordering::SeqCst) {
            if let Some(writer) = writer {
                match writer.finalize() {
                    Ok(_) => info!("WAV file finalized successfully"),
                    Err(e) => error!("Error finalizing WAV file: {}", e),
                };
            }
            info!("Stopping client");
            break;
        }
        match tcp.receive(&mut buffer) {
            Ok(_) => (),
            Err(e) => {
                error!("Error receiving data from server: {}", e);
                return;
            }
        };
        let audio_message = AudioMessage::deserialize(&buffer);
        match audio_message {
            Ok(AudioMessage::Spec(spec)) => {
                debug!("Received audio spec: {:?}", spec);
                if let Some(writer) = writer {
                    match writer.finalize() {
                        Ok(_) => info!("Previous WAV file finalized successfully"),
                        Err(e) => error!("Error finalizing previous WAV file: {}", e),
                    }
                }
                writer = Some(hound::WavWriter::create("output.wav", spec).unwrap());
            }
            Ok(AudioMessage::Samples(samples)) => {
                debug!("Received {} samples", samples.len());
                if let Some(w) = writer.as_mut() {
                    for sample in samples {
                        w.write_sample(sample).unwrap();
                    }
                }
            }
            Err(e) => {
                error!("Error deserializing audio message: {:?}", e);
            }
        }
    }
}
