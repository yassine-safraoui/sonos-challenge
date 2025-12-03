### Spec handling and new clients

I realized the server needs a per‑connection “handshake”: every new client must receive the current audio `Spec` before
it can decode incoming samples. There were two main approaches:

1. **Static bytes for new clients**
    - Store a blob of bytes (serialized `Spec`) in an `Arc<Mutex<Vec<u8>>>`.
    - The accept loop looks up this buffer and sends it to each new `TcpStream` before pushing the stream into the
      shared list.
    - This is simple and works, but the higher‑level code has almost no control over what happens when a client
      connects (no custom logic per connection) and the spec is essentially “frozen” unless I add more machinery.

2. **Callback on new connection (chosen approach)**
    - Extend `TcpServer::init` to accept a callback `on_new_client(&mut TcpStream)` that runs as soon as a connection is
      accepted.
    - The audio layer owns the current `Spec`, serializes it, and captures it in a `move` closure. The callback sends
      the length prefix and the serialized `Spec` bytes to each new client.
    - This keeps the TCP layer generic (no knowledge of `AudioMessage`) but gives the caller full control over what
      happens when a client joins (sending the spec, logging, auth, etc.).
    - To avoid a truly “frozen” spec in the future, the closure can read from shared state (e.g. `Arc<Mutex<Vec<u8>>>`
      holding the current spec bytes), and spec changes can be broadcast as regular `Spec` messages too. Clients can be
      designed to always use “the last `Spec` they received”, making duplicate or updated specs safe and expected.

For this challenge, I’ll use the callback design with a shared, updatable spec representation, so the server is not
locked into a single spec forever, but I won’t fully implement dynamic spec switching yet.

---

### Mutex, broadcasting, and potential audio jitter

The server currently keeps all client `TcpStream`s in a `Vec` behind a `Mutex`. Both the broadcast path and the accept
loop use this same mutex:

- **Broadcast** locks the `Vec<TcpStream>` while iterating and writing a message to every client.
- **Accept** briefly locks the same vector to push a new `TcpStream`.

This has a few implications:

- If a broadcast is in progress, the accept thread will block on the mutex until the broadcast finishes. That means new
  clients can be delayed slightly before they are added, but existing clients are not harmed by this; they just receive
  the audio as long as the broadcast runs.
- From an audio‑quality point of view, I care more about existing streams not experiencing jitter than about a new
  client missing a few initial samples. With the current design, existing clients are not delayed by the accept; the
  accept waits on broadcast, not the other way around.

I considered more advanced designs:

- Using a dedicated “broadcast thread” that owns the `Vec<TcpStream>` and receives new connections via a channel,
  eliminating the shared mutex entirely.
- Reducing lock scope by snapshotting the stream list before writing (e.g. cloning streams), then writing outside the
  lock.

For now, I’m keeping the simple mutex design but I’m aware of the trade‑offs and how a channel‑based ownership model
could avoid contention and potential audio jitter in a more advanced version.