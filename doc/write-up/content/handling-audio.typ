= Handling audio samples

== Reading audio samples

To read WAV audio files on the server, I used the `hound` crate. This gives me both the ```rust WavSpec``` and an iterator over the samples, which is exactly what I need. I first serialize the ```rust Spec``` and set it as the *new‑client message* on the TCP server so every new client receives it, and then I also broadcast the same ```rust Spec``` to any already‑connected clients.

*Note:* This approach can lead to a small race condition: if a client connects after I set the new‑client message but before I broadcast it, that client will receive the ```rust Spec``` twice. However, this is harmless because the client supports changing the ```rust Spec``` mid‑stream, and receiving the same ```rust Spec``` twice has no negative effect.

== Saving audio to a WAV file

The client supports saving the received audio to a WAV file. This is done using the `hound` crate on the client side and is straightforward: after receiving a ```rust Spec```, the client creates a ```rust WavWriter```, and each ```rust Samples``` message is appended directly to the file.

== Playing audio samples

The client also supports playing audio samples through a speaker output. To do this, I use the CPAL crate, which handles actual audio playback. CPAL uses a dedicated audio thread, which must meet real‑time constraints. This means the PCM samples received on the main thread need to be transferred to the audio callback thread efficiently and without locks.

I could have used an ```rust Arc<Mutex<VecDeque<i16>>>``` to pass samples between threads, but the audio thread would need to lock the mutex before accessing samples, which can violate real‑time guarantees. Instead, I chose the `ringbuf` crate, which provides a lock‑free ring buffer suitable for multi‑threaded producer/consumer scenarios. Specifically, I use ```rust HeapRb```, which gives me:

- a producer used by the main thread
- a consumer used by the CPAL audio thread

This avoids mutexes entirely on the audio path and ensures smooth playback.

*Note:* Pulling in an external crate is not a small decision, but in this case a lock‑free ring buffer is essential for reliable audio playback, and implementing one from scratch would be out of scope.

== Sending audio samples

To send audio samples to clients, I serialize each chunk into a ```Samples``` message and broadcast it to all connected clients.

=== Chunking audio samples

Sending each sample individually in a TCP packet is extremely inefficient: TCP packet overhead dominates, and throughput becomes poor. To avoid this, I group samples into chunks of 1000 before sending them. This greatly improves efficiency.

While this introduces some latency, the effect is negligible. For example, with a chunk size of 1000 and a sample rate of 44.1 kHz, the added latency is about 22.6 ms, which is acceptable even in real‑time systems.

=== Pacing

The goal of this project is to *stream* audio. By “stream” I mean delivering a time‑ordered sequence of samples at approximately the same rate they would be produced or played, not pushing the file as fast as the network allows.

Even though the audio source is a WAV file, the server sends the audio in *paced chunks* so that the client receives data in a way that resembles real‑time streaming. After sending a chunk, the server waits for the amount of time it would take to play those samples, that is ```rust SAMPLES_PER_GROUP / sample_rate```

The issue with this approach is that it is sensitive to network latency. If the client is only saving the audio to a file, this is not a problem. But if it is playing audio through speakers, latency spikes cause stuttering.

To mitigate this, I tried two things:

- When streaming starts, I send the first *N* seconds (3 seconds in my code) *without* pacing. This gives the client a small buffer of audio so playback can continue smoothly while later packets arrive. This works as long as the network latency is always below that prebuffering window. Any single packet taking longer breaks the pipeline because TCP delivers packets in order.

- I also adjusted the pacing speed by reducing it slightly. Instead of pacing at a real‑time rate, I pace at 80% of real time (a factor of 4/5). This compensates for small latency variations.
  But it has downsides:
  - large latency spikes can still break playback
  - on very good networks, this causes the client’s playback buffer to grow indefinitely, eventually overflowing

These two measures are temporary and not perfect, but they were sufficient to get a working demo during the challenge timeframe. More robust, long‑term solutions are described in the Future Work section.
