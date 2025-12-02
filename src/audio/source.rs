use hound::{Error, WavSpec};

pub trait AudioSource {
    fn get_spec(&self) -> Result<WavSpec, Error>;
    fn get_samples(&mut self, buf: &mut Vec<i16>) -> Result<(), Error>;
}

pub struct WavAudioSource {
    reader: hound::WavReader<std::io::BufReader<std::fs::File>>,
}

impl WavAudioSource {
    pub fn init(filepath: &str) -> Result<Self, Error> {
        let reader = hound::WavReader::open(filepath)?;
        Ok(Self { reader })
    }
}

impl AudioSource for WavAudioSource {
    fn get_spec(&self) -> Result<WavSpec, Error> {
        Ok(self.reader.spec())
    }

    fn get_samples(&mut self, buf: &mut Vec<i16>) -> Result<(), Error> {
        for sample in self.reader.samples::<i16>() {
            buf.push(sample?);
        }
        Ok(())
    }
}
