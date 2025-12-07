use crate::audio::SpeakerOutputBuilder;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct WavFile {
    pub path: PathBuf,
}

impl FromStr for WavFile {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);

        match path.parent() {
            Some(parent) if parent.as_os_str().is_empty() => {}
            Some(parent) => {
                if !parent.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Parent directory '{}' does not exist", parent.display()),
                    ));
                }
                let meta = fs::metadata(parent)?;
                if !meta.is_dir() {
                    return Err(io::Error::other(format!(
                        "Parent '{}' is not a directory",
                        parent.display()
                    )));
                }
            }
            None => {}
        }

        if path.extension().and_then(|os| os.to_str()) != Some("wav") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "File '{}' does not have a .wav extension. Only wav files are supported.",
                    path.display()
                ),
            ));
        }

        Ok(WavFile { path })
    }
}

#[derive(Debug, Clone)]
pub struct SpeakerDevice {
    pub name: String,
}

impl FromStr for SpeakerDevice {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let device_name = s.to_string();
        if !SpeakerOutputBuilder::new()
            .list_output_devices()
            .contains(&device_name)
        {
            return Err(format!(
                "Speaker device '{}' not found.\n Use the list-available-speakers command to see available devices.",
                device_name
            ));
        }
        Ok(SpeakerDevice {
            name: s.to_string(),
        })
    }
}

pub fn parse_existing_wav(s: &str) -> Result<WavFile, String> {
    let wav: WavFile = s.parse().map_err(|e: io::Error| e.to_string())?;

    if !wav.path.exists() {
        return Err(format!("File '{}' does not exist", wav.path.display()));
    }

    Ok(wav)
}
