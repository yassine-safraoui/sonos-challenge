use log::{LevelFilter, error, info};
use sonos_challenge::audio::{AudioInput, AudioMessage, Serializable, WavAudioInput};
use sonos_challenge::network::tcp::TcpServer;
use std::fmt;
use std::thread::sleep;
use std::time::Duration;

struct Application {
    tcp: TcpServer,
}

#[derive(Debug)]
enum AppError {
    WavFileRead(hound::Error),
    #[allow(dead_code)] // Reserved for future CLI implementation
    TcpInit(std::io::Error),
    Serialization,
    Broadcast(std::io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::WavFileRead(e) => write!(f, "WAV file read error: {}", e),
            AppError::TcpInit(e) => write!(f, "TCP initialization error: {}", e),
            AppError::Serialization => write!(f, "Serialization error"),
            AppError::Broadcast(e) => write!(f, "Broadcast error: {}", e),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::WavFileRead(e) => Some(e),
            AppError::TcpInit(e) => Some(e),
            AppError::Serialization => None,
            AppError::Broadcast(e) => Some(e),
        }
    }
}
impl Application {
    pub fn play_wav_file(&mut self, filepath: &str) -> Result<(), AppError> {
        const SAMPLES_PER_GROUP: usize = 1_000;
        let mut input = match WavAudioInput::init(filepath) {
            Ok(s) => s,
            Err(e) => {
                error!("Couldn't read wav file: {}: {}", filepath, e);
                return Err(AppError::WavFileRead(e));
            }
        };
        let mut serialization_buffer = Vec::new();

        let spec = input.get_spec();
        let wait_time = ((SAMPLES_PER_GROUP * 1_000_000) as f64) * 0.8 / (spec.sample_rate as f64);

        match AudioMessage::Spec(spec).serialize(&mut serialization_buffer) {
            Ok(_) => (),
            Err(_) => {
                error!("Couldn't serialize wav spec for file: {}", filepath);
                return Err(AppError::Serialization);
            }
        };

        self.tcp
            .set_new_client_message(serialization_buffer.as_slice());
        while self.tcp.get_client_count() == 0 {
            info!("No clients connected, waiting for clients to connect...");
            sleep(Duration::from_secs(1));
        }
        match self.tcp.broadcast(&serialization_buffer) {
            Ok(r) => r,
            Err(e) => {
                error!("Couldn't send wav spec to clients: {}", e);
                return Err(AppError::Broadcast(e));
            }
        };

        serialization_buffer.clear();
        let samples = input.iter_samples();
        let mut sample_group: Vec<i16> = Vec::with_capacity(SAMPLES_PER_GROUP);
        let mut sent_samples = 0;
        for sample in samples {
            match sample {
                Ok(s) => {
                    if sample_group.len() < SAMPLES_PER_GROUP {
                        sample_group.push(s);
                        continue;
                    }
                    if let Err(error) = AudioMessage::Samples(sample_group.clone())
                        .serialize(&mut serialization_buffer)
                    {
                        error!("Couldn't serialize sample: {}", error);
                        return Err(AppError::Serialization);
                    }
                    sample_group.clear();
                    if let Err(error) = self.tcp.broadcast(&serialization_buffer) {
                        error!("Couldn't send sample to clients: {}", error);
                        return Err(AppError::Broadcast(error));
                    }
                    sent_samples += sample_group.len();
                    if sent_samples > spec.sample_rate as usize * 3 {
                        sleep(Duration::from_micros(wait_time as u64));
                    }
                    serialization_buffer.clear();
                }
                Err(e) => {
                    error!("Error reading sample: {}", e);
                    return Err(AppError::WavFileRead(e));
                }
            }
        }
        if !sample_group.is_empty() {
            if let Err(e) = AudioMessage::Samples(sample_group.clone())
                .serialize(&mut serialization_buffer)
            {
                error!("Couldn't serialize final samples: {}", e);
                return Err(AppError::Serialization);
            }
            if let Err(e) = self.tcp.broadcast(&serialization_buffer) {
                error!("Couldn't send final samples to client: {}", e);
                return Err(AppError::Broadcast(e));
            }
            serialization_buffer.clear();
        }
        Ok(())
    }
}

fn main() {
    const FILEPATH: &str = "data/song.wav";
    env_logger::builder().filter_level(LevelFilter::Warn).init();

    let tcp = match TcpServer::init("localhost:8080") {
        Ok(t) => t,
        Err(e) => {
            error!("Couldn't bind to localhost:8080: {}", e);
            return;
        }
    };

    let mut app = Application { tcp };
    match app.play_wav_file(FILEPATH) {
        Ok(_) => info!("Finished playing WAV file"),
        Err(e) => error!("{}", e),
    }

    loop {
        sleep(Duration::from_secs(60))
    }
}
