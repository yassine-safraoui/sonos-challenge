use std::fs::File;
use hound::{Error, WavSpec};

pub trait AudioInput {
    fn get_spec(&self) -> WavSpec;
    fn get_all_samples(&mut self, buf: &mut Vec<i16>) -> Result<(), Error>;
}

pub struct WavAudioInput {
    reader: hound::WavReader<std::io::BufReader<File>>,
}

impl WavAudioInput {
    pub fn init(filepath: &str) -> Result<Self, Error> {
        let reader = hound::WavReader::open(filepath)?;
        Ok(Self { reader })
    }
    pub fn iter_samples(&mut self) -> impl Iterator<Item = Result<i16, Error>> + '_ {
        self.reader.samples::<i16>()
    }
}

impl AudioInput for WavAudioInput {
    fn get_spec(&self) -> WavSpec {
        self.reader.spec()
    }

    fn get_all_samples(&mut self, buf: &mut Vec<i16>) -> Result<(), Error> {
        for sample in self.reader.samples::<i16>() {
            buf.push(sample?);
        }
        Ok(())
    }
}
