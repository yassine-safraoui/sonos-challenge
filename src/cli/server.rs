use crate::cli::WavFile;
use crate::cli::parsers::parse_existing_wav;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct ServerCli {
    /// Port of the server
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Audio WAV file to play
    #[arg(short, long, value_parser=parse_existing_wav)]
    pub wav: WavFile,
}
