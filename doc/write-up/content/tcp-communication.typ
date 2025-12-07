= TCP Communication

Before explaining how I handled TCP communication, it’s important to highlight two of its properties:

- *In‑order delivery*: Messages sent on one side are received in the same order on the other side. This is necessary for this use case, and TCP guarantees it, so I don’t need to handle it explicitly.
- *Message‑agnostic*: TCP exposes a continuous byte stream to the receiver and does not preserve the boundaries between the messages sent by the server.

To implement TCP communication, I define two entities:

- *TCP server*, which:
  - handles incoming client connections
  - allows broadcasting messages to them (here, a “message” is a ```rust Vec<u8>```)
  - allows sending a specific message to new clients right after they connect and before they receive any broadcast messages

- *TCP client*, which:
  - initiates a connection to the server
  - receives messages from the server in the same order they were sent, while preserving boundaries between messages

*Note*: Since communication between the server and client happens through the protocol defined in the previous section, it is necessary to preserve message boundaries. Although the size of the `Spec` message is fixed and the `Samples` message contains the number of samples (which could be used to reconstruct the total message size), I think relying on this to recover boundaries is too complex and mixes responsibilities. Instead, I chose to make the TCP layer itself responsible for preserving message boundaries.

*Note*: While TCP is a bidirectional protocol, only communication from the server to the client was implemented in this project. The other direction could be used to counteract latency, this is discussed in the future work section.

== Length prefixing

The goal of length prefixing is to solve the message‑boundary issue. When sending a message (a sequence of bytes) from the server to the client, I first serialize the length of the message into 4 bytes and send that, followed by the message itself. On the client side, I first read these 4 bytes to determine the length of the following message, and then read exactly that many bytes to reconstruct the original message.

== Connection initiation

The connection initiation starts when the client calls ```rust TcpStream::connect```. To handle incoming connections on the server, the server needs to keep listening for new connections. To do this, I use a separate thread dedicated to this task; the goal is to keep the main server thread free for other work, such as reading audio samples and broadcasting them to clients.

=== New‑client message

This feature exists to handle clients that connect to the TCP server after it has already started streaming audio to other client(s). Those new clients need to know the `Spec` of the audio samples the server sends in order to play them correctly. For that reason, the TCP server allows setting a “new client message”; this message is sent by the listening thread to any newly connected client before adding that client to the pool that receives broadcast messages.

To implement this, I used an ```rust Arc<Mutex<Vec<u8>>>``` so that the listening thread can read the new‑client message, while the main thread can still update it after the TCP server has started.

== Testing
To validate the TCP layer, I wrote two end‑to‑end tests that exercise the real ```rust TcpServer``` and ```rust TcpClient``` over localhost. The ```rust broadcast_test``` starts a server, connects a client, checks that exactly one client is registered, then sends a small byte vector via ```rust broadcast``` and verifies that ```rust client.receive``` returns the same bytes with the expected length, confirming that length‑prefixing and message framing work correctly. The ```rust new_client_message_test``` focuses on the “new client message” feature: it configures a server with a predefined message, connects a client, and then asserts that the first frame the client receives matches that message and that the server reports exactly one connected client afterward. Together, these tests give confidence that connection handling, framing, broadcasting, and the new‑client handshake behave as intended.
