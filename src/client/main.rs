use clap::Parser;
use log::{LevelFilter, debug, error, info};
use sonos_challenge::audio::{
    AudioMessage, Serializable, SpeakerOutput, SpeakerOutputBuilder, WavAudioOutput,
};
use sonos_challenge::cli;
use sonos_challenge::cli::{ClientCli, WavFile};
use sonos_challenge::network::TcpClientError;
use sonos_challenge::network::tcp::TcpClient;
use std::io;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::thread::sleep;
use std::time::Duration;

struct Application {
    tcp_client: TcpClient,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

enum ReceiveOutcome {
    Data,
    ServerDisconnected,
}
#[derive(Debug)]
enum ApplicationError {
    TcpClientError,
    WavAudioOutputError,
}
impl Application {
    fn receive(&mut self, buffer: &mut Vec<u8>) -> Result<ReceiveOutcome, ApplicationError> {
        match self.tcp_client.receive(buffer) {
            Ok(_) => Ok(ReceiveOutcome::Data),
            Err(TcpClientError::ServerDisconnected(_)) => {
                info!("Server disconnected");
                Ok(ReceiveOutcome::ServerDisconnected)
            }
            Err(error) => {
                error!("Error receiving data from server: {:?}", error);
                Err(ApplicationError::TcpClientError)
            }
        }
    }
    fn write_audio_to_file(&mut self, filename: &str) -> Result<(), ApplicationError> {
        let mut buffer = Vec::new();
        let mut output: Option<WavAudioOutput> = None;
        loop {
            buffer.clear();
            if self.stop.load(SeqCst) {
                if let Some(output) = output {
                    match output.finalize() {
                        Ok(_) => info!("WAV file finalized successfully"),
                        Err(e) => error!("Error finalizing WAV file: {}", e),
                    };
                }
                info!("Stopping client");
                return Ok(());
            }
            if let ReceiveOutcome::ServerDisconnected = self.receive(&mut buffer)? {
                return Ok(());
            }
            let audio_message = AudioMessage::deserialize(&buffer);
            match audio_message {
                Ok(AudioMessage::Spec(spec)) => {
                    debug!("Received audio spec: {:?}", spec);
                    if let Some(output) = output {
                        match output.finalize() {
                            Ok(_) => info!("Previous WAV file finalized successfully"),
                            Err(e) => error!("Error finalizing previous WAV file: {}", e),
                        }
                    }
                    output = match WavAudioOutput::new(filename, spec) {
                        Ok(result) => Some(result),
                        Err(e) => {
                            error!("Failed to create WAV output: {}", e);
                            return Err(ApplicationError::WavAudioOutputError);
                        }
                    }
                }
                Ok(AudioMessage::Samples(samples)) => {
                    debug!("Received {} samples", samples.len());
                    if let Some(output) = output.as_mut() {
                        if let Err(error) = output.write_samples(&samples) {
                            error!("Failed to write samples to WAV file: {}", error);
                            return Err(ApplicationError::WavAudioOutputError);
                        }
                    }
                }
                Err(e) => {
                    error!("Error deserializing audio message: {:?}", e);
                }
            }
        }
    }
    fn play_audio(&mut self, speaker: Option<String>) -> Result<(), ApplicationError> {
        let mut buffer = Vec::new();
        let mut speaker_output: Option<SpeakerOutput> = None;
        sleep(Duration::from_secs(1));
        loop {
            buffer.clear();
            if self.stop.load(SeqCst) {
                if let Some(output) = speaker_output
                    && let Err(e) = output.pause()
                {
                    error!("Error pausing speaker output: {:?}", e);
                }
                info!("Stopping client");
                return Ok(());
            }
            if let ReceiveOutcome::ServerDisconnected = self.receive(&mut buffer)? {
                return Ok(());
            }
            let audio_message = AudioMessage::deserialize(&buffer);
            match audio_message {
                Ok(AudioMessage::Spec(spec)) => {
                    debug!("Received audio spec: {:?}", spec);
                    let mut speaker_builder = SpeakerOutputBuilder::new();
                    if let Some(device_name) = &speaker {
                        speaker_builder.with_output_device(device_name);
                    }
                    match speaker_builder.build() {
                        Ok(so) => speaker_output = Some(so),
                        Err(e) => {
                            error!("Error initializing speaker output: {:?}", e);
                            continue;
                        }
                    }
                }
                Ok(AudioMessage::Samples(samples)) => {
                    if let Some(output) = speaker_output.as_mut() {
                        output.play_samples(&samples);
                    }
                }
                Err(e) => {
                    error!("Error deserializing audio message: {:?}", e);
                }
            }
        }
    }
    fn list_available_speakers() {
        let speakers = SpeakerOutputBuilder::new().list_output_devices();
        println!("Available speaker devices:");
        for speaker in speakers {
            println!(" - {}", speaker);
        }
    }
}

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init();
    let cli = ClientCli::parse();
    match cli.command {
        Some(cli::ClientCliSubCommand::ListAvailableSpeakers) => {
            Application::list_available_speakers();
            return;
        }
        None => {}
    }

    let address = format!("{}:{}", cli.ip.unwrap(), cli.port.unwrap());
    let mut tcp: io::Result<TcpClient>;
    loop {
        tcp = TcpClient::connect(&address);
        if tcp.is_err() {
            info!(
                "Couldn't connect to server at {}. Retrying after 1 second...",
                address
            );
            sleep(Duration::from_secs(1));
            continue;
        }
        break;
    }
    let tcp = tcp.unwrap();
    let stop: Arc<std::sync::atomic::AtomicBool> =
        Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || {
            debug!("Ctrl-C received, stopping client");
            stop.store(true, SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }
    let mut app = Application {
        tcp_client: tcp,
        stop,
    };
    if cli.default_speaker || cli.speaker.is_some() {
        let speaker_name = cli.speaker.map(|s| s.name);
        if let Err(error) = app.play_audio(speaker_name) {
            error!("Error during audio playback: {:?}", error);
            return;
        }
    } else if let Some(WavFile { path }) = cli.file {
        let file = match path.to_str() {
            Some(f) => f,
            None => {
                error!("Unexpected error: Invalid file path");
                return;
            }
        };

        if let Err(error) = app.write_audio_to_file(file) {
            error!("Error writing audio to file: {:?}", error);
            return;
        }
    }
}
