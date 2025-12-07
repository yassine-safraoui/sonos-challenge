= Future work

== Sourcing audio from a microphone

One important feature still missing from this project is sourcing audio directly from a microphone on the server. This should be relatively straightforward to implement using CPAL. It would follow the same structure as the speaker‑output code, except that the audio callback running in a dedicated thread would *produce* samples instead of consuming them. Those samples could then be serialized and broadcast to clients just like WAV‑based samples.

== Client‑to‑server communication

As explained earlier, the current measures to keep audio playback smooth are not sufficient in all conditions. A more robust solution would involve using the client‑to‑server direction of the TCP connection.

With two‑way communication, the client could periodically report the amount of buffered audio it has (for example, in seconds). The server could then adjust its pacing accordingly:

- speed up if the client buffer is running low
- slow down if the client buffer is growing too large

Another idea is to periodically send ICMP‑style pings (or simply timestamped messages) from the server to the client to estimate network latency and jitter, and then incorporate this information into pacing decisions.

However, these approaches come with a significant architectural implication: the server would need to track per‑client state and send audio samples individually rather than broadcasting the same message to all clients. This would complicate the system considerably compared to the current stateless broadcast model.

== Integration tests

As the project grows, end‑to‑end integration tests will become essential. A simple starting point would be:

- run both the server and the client in “WAV‑to‑WAV” mode
- after the stream finishes, compare the client’s output WAV file to the server’s source WAV file

This could later be extended by introducing a proxy between the server and client to artificially control bandwidth, inject latency or jitter, or simulate packet delay. This would allow testing how the system behaves under realistic network conditions and whether future pacing or buffering strategies work correctly.
