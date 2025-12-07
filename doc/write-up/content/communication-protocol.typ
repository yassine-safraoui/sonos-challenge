= Communication protocol

This section describes the *application‑level protocol* used between the server and the client, on top of TCP. This is necessary because TCP only handles transfering bytes.
At this level there are two message types:

- `Spec` messages – describe the audio format.
- `Samples` messages – carry PCM audio samples.

Each message starts with a one‑byte type tag to indicate the type of the message. In addition, all multi‑byte integers are encoded in little‑endian to avoid issues with CPUs using the big-endian representation.

== Spec message

A `Spec` message carries the audio configuration needed to interpret the raw samples:

- `message_type: u8` (value `1`)
- `channels: u16`
- `sample_rate: u32`
- `bits_per_sample: u16`
- `sample_format: u8`
  - `1` = floating‑point samples
  - `2` = integer samples

*Note:* Although we include the `sample_format` and `channles` fields in the Spec message, we only support playing mono 16-bit PCM audio files.

The server sends a `Spec` message to each client before sending any samples, this applies for new clients that connect in the middle of a stream as well. The client uses this information mainly to configure its WAV writer when writing to a file. But this message could be used in the future to support multiple channels and other sample formats.

== Samples message

A `Samples` message carries a variable‑length chunk of PCM data:

- `message_type: u8` (value `2`)
- `length: u32` – number of samples (not bytes)
- `samples: length × i16` – each sample is a 16‑bit signed integer

Samples are encoded as consecutive `i16` values in little‑endian form.

== implementation

Implementing this protocol essentially means defining the messages to use, implementing a `serialize` method that convert messages to a binary representation and a `deserialize` method that does the opposite.
During this phase, I considered using protocol buffers because they are designed for this kind of use case. However, I decided to implement this manually because protocol buffers would be overkill for such a simple protocol.

== Testing
Since this protocol is a critical component of the project, I wrote unit tests to ensure it behaves correctly and remains stable over time. Rust’s built‑in test support makes this straightforward to set up.

The testing strategy is to define a set of representative AudioMessage values (different Spec configurations and various Samples vectors, including edge values) and verify that each message survives a full round trip:

1. Serialize the message into bytes.
2. Deserialize those bytes back into an AudioMessage.
3. Assert that the result is exactly equal to the original message.
