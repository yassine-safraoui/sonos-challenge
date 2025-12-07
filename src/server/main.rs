use clap::Parser;
use log::{LevelFilter, error, info};
use sonos_challenge::audio::{AudioMessage, Serializable, WavAudioInput};
use sonos_challenge::cli::ServerCli;
use sonos_challenge::network::tcp::TcpServer;
use std::thread::sleep;
use std::time::Duration;

struct Application {
    tcp: TcpServer,
}
#[derive(Debug)]
enum AppError {
    WavFileRead,
    Serialization,
    Broadcast,
}
impl Application {
    pub fn play_wav_file(&mut self, filepath: &str) -> Result<(), AppError> {
        const SAMPLES_PER_GROUP: usize = 1_000;
        let mut input = match WavAudioInput::init(filepath) {
            Ok(s) => s,
            Err(_) => {
                error!("Couldn't read wav file: {}", filepath);
                return Err(AppError::WavFileRead);
            }
        };
        let mut serialization_buffer = Vec::new();

        let spec = input.get_spec();
        /// Fraction of real‑time used to pace sending, leaving headroom for
        /// network and processing latency.
        const PLAYBACK_PACING_FACTOR: f64 = 0.8;
        /// Amount of audio (in seconds) to preload before pacing
        /// to build up a latency buffer on the client.
        const INITIAL_BUFFER_SECONDS: usize = 3;

        let wait_time_micros = ((SAMPLES_PER_GROUP * 1_000_000) as f64) * PLAYBACK_PACING_FACTOR
            / (spec.sample_rate as f64);
        // We multiply by 0.8 to account for network latency and processing time

        match AudioMessage::Spec(spec).serialize(&mut serialization_buffer) {
            Ok(_) => (),
            Err(_) => {
                error!("Couldn't serialize wav spec for file: {}", filepath);
                return Err(AppError::Serialization);
            }
        };

        self.tcp.set_new_client_message(&serialization_buffer);
        while self.tcp.get_client_count() == 0 {
            info!("No clients connected, waiting for clients to connect...");
            sleep(Duration::from_secs(1));
        }
        match self.tcp.broadcast(&serialization_buffer) {
            Ok(r) => r,
            Err(_) => {
                error!("Couldn't send wav spec to clients");
                return Err(AppError::Broadcast);
            }
        };

        serialization_buffer.clear();
        let samples = input.iter_samples();
        let mut sample_group: Vec<i16> = Vec::with_capacity(SAMPLES_PER_GROUP);
        let mut sent_samples = 0;
        for sample in samples {
            match sample {
                Ok(s) => {
                    sample_group.push(s);
                    if sample_group.len() < SAMPLES_PER_GROUP {
                        continue;
                    }
                    self.play_samples_group(&sample_group)?;
                    sent_samples += sample_group.len();
                    sample_group.clear();
                    if sent_samples > spec.sample_rate as usize * INITIAL_BUFFER_SECONDS {
                        sleep(Duration::from_micros(wait_time_micros as u64));
                    }
                }
                Err(e) => {
                    error!("Error reading sample: {}", e);
                    return Err(AppError::WavFileRead);
                }
            }
        }
        if !sample_group.is_empty() {
            self.play_samples_group(&sample_group)?;
        }
        while self.tcp.get_client_count() > 0 {
            info!("Waiting for clients to finish playback...");
            sleep(Duration::from_secs(1));
        }
        Ok(())
    }

    fn play_samples_group(&mut self, samples: &[i16]) -> Result<(), AppError> {
        let mut serialization_buffer = Vec::new();
        match AudioMessage::Samples(samples.to_vec()).serialize(&mut serialization_buffer) {
            Ok(_) => (),
            Err(_) => {
                error!("Couldn't serialize samples");
                return Err(AppError::Serialization);
            }
        };
        match self.tcp.broadcast(&serialization_buffer) {
            Ok(_) => Ok(()),
            Err(_) => {
                error!("Couldn't send samples to clients");
                Err(AppError::Broadcast)
            }
        }
    }
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let cli = ServerCli::parse();
    let port = cli.port;
    let ip = "0.0.0.0";
    let address = format!("{ip}:{port}");
    println!("Starting server at {address}");

    let tcp = match TcpServer::bind(&address) {
        Ok(t) => t,
        Err(_) => {
            error!("Couldn't connect to server at {address}");
            return;
        }
    };
    let mut app = Application { tcp };
    let filepath = match cli.wav.path.to_str() {
        Some(f) => f,
        None => {
            error!("Unexpected error: Invalid file path");
            return;
        }
    };
    match app.play_wav_file(filepath) {
        Ok(_) => info!("Finished playing WAV file"),
        Err(e) => error!("{:?}", e),
    }
}
