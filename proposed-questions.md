# Proposed Interview Questions for Sonos Rust Audio Streaming Challenge

This document contains potential interview questions based on the audio streaming server/client implementation. The questions are organized by topic and designed to assess understanding of design decisions, technical implementation, and Rust best practices.

---

## 1. Architecture & Design Decisions

### High-Level Design
1. **Why did you choose a client-server architecture over peer-to-peer for this audio streaming system?**
   - Follow-up: What trade-offs did you consider?

2. **Walk me through the high-level architecture of your solution. How do the server and client communicate?**

3. **You've structured the codebase into modules (`audio`, `network`, `cli`). Why did you organize it this way?**
   - Follow-up: How does this modular design make the code more maintainable?

### Protocol Design
4. **Explain the communication protocol you designed between the server and client.**
   - What are the message types?
   - How do you handle message framing?
   - Why did you choose this approach?

5. **Your protocol has two message types: `Spec` and `Samples`. Why separate these?**
   - Follow-up: What happens when a client connects after the server has already started streaming?

6. **Why did you use a length-prefixed framing protocol (4-byte length header)?**
   - What are the advantages over alternatives like delimiters or fixed-size frames?

7. **The maximum frame size is set to 16 MB. How did you arrive at this number?**
   - What would be the consequences of setting it too low or too high?

---

## 2. Rust-Specific Implementation

### Memory Management & Ownership
8. **In your `TcpServer`, you use `Arc<Mutex<VecDeque<TcpStream>>>`. Explain this design choice.**
   - Why `Arc`?
   - Why `Mutex` instead of `RwLock`?
   - Why `VecDeque` instead of `Vec`?

9. **I see you're using `Vec<u8>` as a buffer that gets reused. Why is this better than creating a new buffer for each message?**

10. **In the client's main loop, you call `buffer.clear()` before receiving. Why not just create a new `Vec`?**

### Concurrency & Threading
11. **The `TcpServer` spawns a separate thread for accepting connections. Why did you design it this way?**
    - Follow-up: How does the thread communicate with the main application?

12. **How do you handle graceful shutdown of the server's listener thread?**
    - Walk me through the `AtomicBool` and `Drop` implementation.

13. **The client uses `ctrlc` to handle Ctrl-C. Explain how this works with `Arc<AtomicBool>`.**
    - Why is `Arc` necessary here?

14. **In `SpeakerOutput`, you use a lock-free ring buffer from the `ringbuf` crate. Why?**
    - What problem does this solve?
    - What would happen if you used a `Mutex` instead?

### Error Handling
15. **You have custom error types like `TcpClientError`, `SpeakerOutputError`, and `WavOutputError`. Why create custom error types instead of using `io::Error` everywhere?**

16. **In `TcpClient::map_read_error`, you distinguish between different `io::ErrorKind` values. Why is this important?**
    - Follow-up: How does this help the client application?

17. **The server's `broadcast` method filters out disconnected clients. Walk me through how this works.**
    - Why do you drain the streams, try to send, and then re-add only the successful ones?

### Serialization
18. **You implemented a custom serialization protocol for `AudioMessage`. Why not use an existing library like `serde` with bincode or JSON?**
    - What are the trade-offs?

19. **In `AudioMessage::serialize`, you use `to_le_bytes()` for integers. Why little-endian?**
    - How would you handle clients/servers on different architectures?

20. **The `Samples` variant stores samples as `Vec<i16>`. Why `i16` specifically?**
    - Follow-up: How does this relate to the WAV file format requirements?

---

## 3. Network & TCP Communication

### Connection Management
21. **The server broadcasts to all connected clients simultaneously. What happens if one client is slow to receive?**
    - Does it block other clients?
    - How did you design around this?

22. **When a new client connects, they receive the current `Spec` message. What happens if they connect mid-stream?**
    - Do they receive audio that's already been played?
    - Is there any synchronization mechanism?

23. **The client retries connection in a loop if the server isn't ready. Why implement this retry logic?**
    - What's the downside of this approach?

### Flow Control & Pacing
24. **Explain the pacing mechanism in your server. Why do you need it?**
    - Walk me through the `PLAYBACK_PACING_FACTOR` (0.8) calculation.

25. **You have an `INITIAL_BUFFER_SECONDS` of 3 seconds. What problem does this solve?**
    - How did you determine this value?

26. **What happens if network jitter causes packets to arrive late at the client?**
    - How does your ring buffer help (or not help) with this?

27. **In `SpeakerOutput::play_samples`, you have a busy-wait loop: `while self.producer.vacant_len() < samples.len() {}`. Why?**
    - What are the implications of this approach?
    - Could this cause issues? If so, what would be a better solution?

---

## 4. Audio Processing

### WAV Format & Specs
28. **Your implementation only supports mono, 16-bit PCM audio. Why these limitations?**
    - What would it take to support stereo or different bit depths?

29. **The `WavSpec` includes `sample_rate`, `channels`, `bits_per_sample`, and `sample_format`. How do these relate to the actual audio data?**

30. **In `fill_from_consumer`, you process audio in stereo frames (`chunks_mut(2)`). Why, given that the input is mono?**
    - Follow-up: What happens if the output device expects mono?

### Real-Time Streaming
31. **Your server sends samples in groups of 1000. How did you choose this chunk size?**
    - What would happen with very small chunks (e.g., 10 samples)?
    - What about very large chunks (e.g., 100,000 samples)?

32. **The client has two output modes: file and speaker. Compare the challenges of each.**
    - Which is more forgiving of network delays?

33. **In speaker mode, what happens when the ring buffer gets full?**
    - How does your code handle this? (Check the `warn!` message in `play_samples`)

### CPAL Integration
34. **Explain how CPAL's audio callback works. Why can't you directly write to the speaker from the network thread?**

35. **You support three sample formats in `SpeakerOutputBuilder::build`: F32, I16, and U16. Why these three?**
    - How do you convert from the incoming i16 samples to the output format?

---

## 5. CLI & User Experience

36. **You used `clap` for command-line parsing. Walk me through the CLI design for the client.**
    - Why use subcommands and argument groups?

37. **The client requires exactly one output mode (`--file`, `--default-speaker`, or `--speaker`). How does `clap` enforce this?**
    - Look at the `ArgGroup` configuration.

38. **Why did you create custom value parsers like `WavFile` and `SpeakerDevice`?**
    - What validation do they provide?

39. **The `list-available-speakers` subcommand is separate from the main streaming mode. Why design it this way?**

---

## 6. Testing & Quality

40. **You have unit tests for TCP framing and audio message serialization. Why focus on these areas?**

41. **The TCP tests use hard-coded ports (50104, 50105). What could go wrong with this approach?**
    - How would you improve this?

42. **I notice there are no integration tests that actually stream audio end-to-end. Why?**
    - How would you test the full system?

43. **Looking at your test for `broadcast_test`, you sleep for 100ms. Why?**
    - Is this reliable? What could make it fail?

44. **How would you test error conditions like network failures, corrupted data, or out-of-memory scenarios?**

---

## 7. Code Quality & Rust Idioms

45. **In several places, you use `unwrap_or_else` with a closure that logs an error and handles poisoned mutexes. Explain this pattern.**

46. **You use `#[repr(u8)]` for `AudioMessageType`. What does this do and why is it important?**

47. **The code uses `match` extensively for error handling rather than `?` operator in some places. Why?**
    - When would you choose one over the other?

48. **You implement `TryFrom<u8>` for `AudioMessageType`. Why use `TryFrom` instead of a simple function?**

49. **Looking at `WavAudioOutput::finalize`, it consumes `self`. Why this design?**
    - What does this prevent?

50. **You use `if let` chains with `&&` (e.g., `if let Some(output) = speaker_output && let Err(e) = output.pause()`). What Rust edition feature is this?**

---

## 8. Performance & Scalability

51. **How would your server perform with 100 concurrent clients? 1000 clients?**
    - What would be the bottlenecks?

52. **The server broadcasts the same data to all clients. How much memory does this use?**
    - Could you optimize this?

53. **You serialize the audio message into a buffer each time before sending. Could this be optimized?**

54. **What's the impact of the `VecDeque` `drain` operation in the broadcast method?**
    - Is there a more efficient approach?

---

## 9. Security & Robustness

55. **What happens if a malicious client sends a frame length of `u32::MAX`?**
    - Walk through how your code handles this.

56. **The server binds to `0.0.0.0`. What are the security implications?**

57. **There's no authentication or encryption. How would you add these if required?**

58. **What happens if someone sends random bytes to the server or client?**
    - How does your deserialization handle invalid data?

---

## 10. Trade-offs & Alternatives

59. **You chose TCP over UDP. Why?**
    - For real-time audio, what are the trade-offs?
    - When might UDP be better?

60. **The protocol is custom binary. What if you had used WebSocket or gRPC?**
    - What would you gain or lose?

61. **Currently, clients can only receive audio. What would it take to support bidirectional communication?**
    - What use cases would this enable?

62. **The server reads from a WAV file. How would you modify it to accept microphone input?**
    - What challenges would this introduce?

---

## 11. Future Improvements

63. **Looking at your "Caveats & future work" section in the README, you mention better pacing and buffering. What specific improvements would you make?**

64. **You mentioned feedback channels from client to server. What information would you send?**
    - How would this improve the system?

65. **How would you add support for multiple audio streams or channels?**

66. **The README mentions that microphone input isn't implemented. What would be the main challenges?**

67. **If you had another week to work on this, what would you prioritize?**
    - Why those features?

---

## 12. AI Usage & Learning

68. **You used coding assistants for this project. Which parts did you rely on AI for?**
    - Which parts did you write yourself?
    - How did you verify the AI-generated code was correct?

69. **Did you learn any new Rust concepts while working on this project?**
    - Which ones were most challenging?

70. **If you were to start this project over, what would you do differently?**

---

## 13. Real-World Considerations

71. **How would you deploy this in a production environment?**
    - What infrastructure would you need?

72. **What monitoring or logging would you add for production use?**

73. **How would you handle version compatibility if you update the protocol?**
    - What if old clients connect to a new server?

74. **The code uses `env_logger` with a default INFO level. How would you configure logging in different environments (dev, staging, prod)?**

75. **What documentation would you add before handing this off to another team?**

---

## 14. Problem-Solving Approach

76. **When you started this challenge, what was your first step?**
    - How did you break down the problem?

77. **Did you encounter any blocking issues during development?**
    - How did you debug them?

78. **What resources (documentation, examples, etc.) did you find most helpful?**

79. **How did you test your implementation during development?**
    - Before you wrote the unit tests?

80. **If we asked you to add a feature you've never implemented before (e.g., audio compression), how would you approach it?**

---

## Notes for Interviewers

These questions are designed to:
- **Assess technical depth**: Understanding of Rust, networking, audio processing
- **Evaluate design thinking**: Why certain decisions were made, trade-off analysis
- **Gauge problem-solving**: How they approach challenges and debugging
- **Check learning ability**: How they handle unknowns and use resources
- **Test communication**: Can they explain complex technical concepts clearly

**Recommended approach:**
1. Start with high-level architecture questions (1-7)
2. Dive deep into 2-3 areas based on candidate's strengths/interests
3. Include at least a few "future improvements" questions to see how they think about evolution
4. End with reflection/meta-questions about their process

**Red flags to watch for:**
- Can't explain their own design decisions
- Defensive about trade-offs or limitations
- Claims AI wrote everything without understanding
- Can't discuss alternatives or improvements

**Green flags:**
- Acknowledges limitations and trade-offs honestly
- Shows curiosity and willingness to learn
- Can explain concepts at multiple levels of detail
- Demonstrates systematic problem-solving approach
