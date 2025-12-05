use crate::audio::DeserializationError::UnknownWaveSpecSampleFormat;
use crate::audio::message::LengthError::TooLong;
use hound::{SampleFormat, WavSpec};

#[derive(Debug)]
pub enum LengthError {
    TooLong { len: usize },
}

#[derive(Debug, PartialEq)]
pub enum DeserializationError {
    IncorrectAudioMessageType {
        kind: u8,
    },
    DataLengthMismatch {
        expected_length: usize,
        current_length: usize,
    },
    UnknownWaveSpecSampleFormat,
}
pub trait Serializable: Sized {
    fn serialize(&self, buf: &mut Vec<u8>) -> Result<(), LengthError>;
    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializationError>;
}

#[derive(Debug, PartialEq)]
pub enum AudioMessage {
    Spec(WavSpec),
    Samples(Vec<i16>),
}

#[repr(u8)]
enum AudioMessageType {
    Spec = 1,
    Samples = 2,
}

impl TryFrom<u8> for AudioMessageType {
    type Error = DeserializationError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AudioMessageType::Spec),
            2 => Ok(AudioMessageType::Samples),
            _ => Err(DeserializationError::IncorrectAudioMessageType { kind: value }),
        }
    }
}

const SPEC_MSG_LEN: usize = 1 + 2 + 4 + 2 + 1;
// message_type(1) + channels(2) + sample_rate(4) + bits_per_sample(2) + sample_format(1)
const SAMPLES_HEADER_LEN: usize = 1 + 4;
const SAMPLE_SIZE: usize = 2;

impl Serializable for AudioMessage {
    fn serialize(&self, buf: &mut Vec<u8>) -> Result<(), LengthError> {
        match self {
            AudioMessage::Spec(spec) => {
                buf.reserve(SPEC_MSG_LEN);
                buf.push(AudioMessageType::Spec as u8);
                buf.extend_from_slice(&spec.channels.to_le_bytes());
                buf.extend_from_slice(&spec.sample_rate.to_le_bytes());
                buf.extend_from_slice(&spec.bits_per_sample.to_le_bytes());

                let format_tag: u8 = match spec.sample_format {
                    SampleFormat::Float => 1,
                    SampleFormat::Int => 2,
                };
                buf.push(format_tag);
            }
            AudioMessage::Samples(samples) => {
                if samples.len() > (u32::MAX / 2) as usize {
                    // Prevent overflow when calculating length in bytes
                    return Err(TooLong { len: samples.len() });
                }
                buf.reserve(SAMPLES_HEADER_LEN + samples.len() * SAMPLE_SIZE);
                buf.push(AudioMessageType::Samples as u8);
                let len = (samples.len() as u32).to_le_bytes();
                buf.extend_from_slice(&len);
                buf.extend(samples.iter().flat_map(|s| s.to_le_bytes()));

                // Implementation choice: using slice to copy the data via a memory copy instead of iterating through
                // the elements one by one

                // Implementation choice: we're converting the samples into a vec of u8 using a flat map, this does
                // use CPU, we could do something unsafe (in the rust sense of the word) and
                // reinterpret the vec as a vec of u8 using either from_raw_parts, but that would
                // make the samples u8 bytes <<<<<<endian>>>>>>ness be that of the host machine's endianness,
                // since this program may run on different machines(MacOS and/or Linux), this is
                // not a hypothesis we can afford to make.
            }
        }
        Ok(())
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, DeserializationError> {
        if bytes.is_empty() {
            return Err(DeserializationError::DataLengthMismatch {
                current_length: 0,
                expected_length: 1,
            });
        }
        let msg_type = AudioMessageType::try_from(bytes[0]);
        match msg_type {
            Ok(AudioMessageType::Spec) => {
                if bytes.len() != SPEC_MSG_LEN {
                    return Err(DeserializationError::DataLengthMismatch {
                        current_length: bytes.len(),
                        expected_length: SPEC_MSG_LEN,
                    });
                }
                let channels =
                    u16::from_le_bytes(bytes[1..3].try_into().expect("length checked above"));
                let sample_rate =
                    u32::from_le_bytes(bytes[3..7].try_into().expect("length checked above"));
                let bits_per_sample =
                    u16::from_le_bytes(bytes[7..9].try_into().expect("length checked above"));
                let sample_format = match bytes[9] {
                    1 => SampleFormat::Float,
                    2 => SampleFormat::Int,
                    _ => return Err(UnknownWaveSpecSampleFormat),
                };

                Ok(AudioMessage::Spec(WavSpec {
                    channels,
                    sample_rate,
                    bits_per_sample,
                    sample_format,
                }))
            }
            Ok(AudioMessageType::Samples) => {
                if bytes.len() < SAMPLES_HEADER_LEN {
                    return Err(DeserializationError::DataLengthMismatch {
                        current_length: bytes.len(),
                        expected_length: 5,
                    });
                }
                let length =
                    u32::from_le_bytes(bytes[1..5].try_into().expect("length checked above"));
                let expected_length = SAMPLES_HEADER_LEN + (length as usize) * SAMPLE_SIZE;
                if bytes.len() != expected_length {
                    return Err(DeserializationError::DataLengthMismatch {
                        current_length: bytes.len(),
                        expected_length,
                    });
                }
                let samples: Vec<i16> = bytes[5..]
                    .chunks_exact(2)
                    .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();

                Ok(AudioMessage::Samples(samples))
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::audio::{AudioMessage, DeserializationError, Serializable};
    use hound::{SampleFormat, WavSpec};
    use log::{LevelFilter, debug};

    fn round_trip(msg: AudioMessage) {
        let mut buf = Vec::new();
        msg.serialize(&mut buf).expect("serialize failed");
        debug!("Serialized bytes: {:?}", buf);
        let decoded = AudioMessage::deserialize(&buf).expect("deserialize failed");
        assert_eq!(decoded, msg);
    }

    #[test]
    fn audio_message_round_trips() {
        env_logger::builder()
            .filter_level(LevelFilter::Trace)
            .init();
        let messages = vec![
            AudioMessage::Spec(WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: SampleFormat::Int,
            }),
            AudioMessage::Spec(WavSpec {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
            }),
            AudioMessage::Samples(vec![]),
            AudioMessage::Samples(vec![0]),
            AudioMessage::Samples(vec![i16::MIN, -1, 0, 1, i16::MAX]),
        ];

        for msg in messages {
            debug!("Testing round trip for message: {:?}", msg);
            round_trip(msg);
        }
    }

    #[test]
    fn unknown_sample_format_tag_yields_correct_error() {
        let mut bytes = Vec::new();
        bytes.push(1);
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.push(99);

        let err = AudioMessage::deserialize(&bytes).unwrap_err();
        assert_eq!(err, DeserializationError::UnknownWaveSpecSampleFormat);
    }
}
