use log::{LevelFilter, error, info};
use sonos_challenge::audio::{AudioInput, AudioMessage, Serializable, WavAudioInput};
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
        let wait_time = ((SAMPLES_PER_GROUP * 1_000_000) as f64) / (spec.sample_rate as f64);

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
                    if sample_group.len() < SAMPLES_PER_GROUP {
                        sample_group.push(s);
                        continue;
                    }
                    if let Err(error) = AudioMessage::Samples(sample_group.clone())
                        .serialize(&mut serialization_buffer)
                    {
                        error!("Couldn't serialize sample: {:?}", error);
                        return Err(AppError::Serialization);
                    }
                    sample_group.clear();
                    if let Err(error) = self.tcp.broadcast(&serialization_buffer) {
                        error!("Couldn't send sample to clients: {}", error);
                        return Err(AppError::Broadcast);
                    }
                    sent_samples += sample_group.len();
                    if sent_samples > spec.sample_rate as usize * 3 {
                        sleep(Duration::from_micros(wait_time as u64));
                    }
                    serialization_buffer.clear();
                }
                Err(e) => {
                    error!("Error reading sample: {}", e);
                    return Err(AppError::WavFileRead);
                }
            }
        }
        if !sample_group.is_empty() {
            if AudioMessage::Samples(sample_group.clone())
                .serialize(&mut serialization_buffer)
                .is_err()
            {
                error!("Couldn't serialize final samples");
                return Err(AppError::Serialization);
            }
            if self.tcp.broadcast(&serialization_buffer).is_err() {
                error!("Couldn't send final samples to client");
                return Err(AppError::Broadcast);
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
        Err(_) => {
            error!("Couldn't connect to server at localhost:8080");
            return;
        }
    };

    let mut app = Application { tcp };
    match app.play_wav_file(FILEPATH) {
        Ok(_) => info!("Finished playing WAV file"),
        Err(e) => error!("{:?}", e),
    }

    loop {
        sleep(Duration::from_secs(60))
    }
}
