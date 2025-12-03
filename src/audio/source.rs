use hound::{Error, WavSpec};

pub trait AudioSource {
    fn get_spec(&self) -> Result<WavSpec, Error>;
    fn get_all_samples(&mut self, buf: &mut Vec<i16>) -> Result<(), Error>;
}

pub struct WavAudioSource {
    reader: hound::WavReader<std::io::BufReader<std::fs::File>>,
}

impl WavAudioSource {
    pub fn init(filepath: &str) -> Result<Self, Error> {
        let reader = hound::WavReader::open(filepath)?;
        Ok(Self { reader })
    }
    pub fn iter_samples(&mut self) -> impl Iterator<Item = Result<i16, Error>> + '_ {
        self.reader.samples::<i16>()
    }
}

impl AudioSource for WavAudioSource {
    fn get_spec(&self) -> Result<WavSpec, Error> {
        Ok(self.reader.spec())
    }

    fn get_all_samples(&mut self, buf: &mut Vec<i16>) -> Result<(), Error> {
        for sample in self.reader.samples::<i16>() {
            buf.push(sample?);
        }
        Ok(())
    }
}
