use crate::cli::{SpeakerDevice, WavFile};
use clap::Parser;
use clap::{ArgGroup, Subcommand};
use std::net::IpAddr;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, subcommand_negates_reqs = true)]
#[clap(group(ArgGroup::new("output").required(true).args(&["file", "speaker", "default_speaker"])))]
pub struct ClientCli {
    #[clap(subcommand)]
    pub command: Option<ClientCliSubCommand>,

    /// Port of the server
    #[arg(short, long, required = true)]
    pub port: Option<u16>,

    /// IP address of the server
    #[arg(long, value_parser = clap::value_parser!(IpAddr), required = true)]
    pub ip: Option<IpAddr>,

    #[clap(long, value_parser = clap::value_parser!(WavFile))]
    pub file: Option<WavFile>,

    #[clap(long, value_parser = clap::value_parser!(SpeakerDevice))]
    pub speaker: Option<SpeakerDevice>,

    #[clap(short, long)]
    pub default_speaker: bool,
}

#[derive(Subcommand, Debug)]
pub enum ClientCliSubCommand {
    ListAvailableSpeakers,
}
