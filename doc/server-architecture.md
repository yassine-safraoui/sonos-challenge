# **Summary of Protocol & Architecture Decisions**

### **1. Keep the TCP layer dead simple**

The TCP module will do **exactly one thing well**:
**send and receive raw byte frames with length-prefixing.**

Because TCP is a stream protocol, message boundaries can get mixed together.
Length-prefixing fixes that cleanly:

```
[length: u32][payload bytes...]
```

No WAV logic, no message types, nothing audio-specific goes into this layer.

---

### **2. Build a separate protocol layer for audio messages**

Above TCP, I’ll introduce a small protocol that defines the actual message semantics.

This will consist of:

```rust
enum AudioMessageType {
    Spec,
    Samples,
}
```

And an `AudioMessage` struct containing:

* The message type
* The payload (either spec or samples)

**Spec message:**
Contains the audio format (sample rate, channels, bit depth, …).

**Samples message:**
Contains a length field + the actual list of PCM samples.
(The inner length is redundant because TCP already sent a length prefix, but it keeps the message self-describing and clean.)

This protocol layer is the one responsible for serializing/deserializing messages.
TCP only moves bytes.

---

### **3. Abstract audio reading separately**

The WAV streaming logic becomes its own unit (`AudioSource` or similar).
It doesn’t know about TCP at all.
It simply yields:

* The audio spec (once)
* Batches/chunks of PCM samples

Later this same abstraction lets me drop in a microphone source without touching the rest of the pipeline.

---

### **4. Add simple rate control in the TCP send path**

For now, the TCP module’s send function will be blocking and paced.
This prevents the server from blasting the entire WAV file at the client instantly.

Reasoning:

* Technically, TCP would queue and handle congestion itself.
* But pacing the messages avoids overwhelming buffers and keeps the behavior predictable.
* Later, I can offload this pacing into a separate thread or an async task.

This is a practical decision for reliability during early development.

---

# **Final Architecture**

```
[AudioSource] --spec/samples--> [AudioProtocol] --serialized msg--> [TcpTransport] --bytes--> network
```

Each layer does one job:

* **TcpTransport:** framing (length prefix), sending, receiving
* **AudioProtocol:** message types, serialization, parsing
* **AudioSource:** WAV or microphone input, yields PCM samples
* **App:** ties everything together

This approach avoids mixing concerns, keeps the code modular, and makes it trivial to extend later.