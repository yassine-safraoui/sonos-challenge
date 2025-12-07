# Sonos Coding Challenge – Rust Audio Streamer

This project implements a small audio‑streaming system in Rust:

- A **server** that reads audio from a WAV file and streams it over **TCP**.
- A **client** that:
    - either writes the received audio to a WAV file,
    - or plays it back in real time through the system’s audio output.

The focus is on **streaming**: the server sends PCM samples in paced chunks that follow
the audio’s sample rate, so the client receives data in a way that closely matches
real‑time input rather than a bulk file transfer.

The code is written in Rust and targets Linux and macOS.

---

## Features

- TCP server that broadcasts audio to any number of clients.
- WAV input on the server (mono, 16‑bit PCM).
- Two client output modes:
    - **WAV**: save received audio to a WAV file.
    - **Speaker**: play received audio through the default or a selected speaker.
- Command‑line interface for both server and client (using `clap`).
- Graceful shutdown of the client with Ctrl‑C.
- Simple pacing and buffering to approximate real‑time streaming.

---

## Project structure

```text
sonos-challenge/
├── src/
│   ├── audio/        # WAV I/O, audio messages, speaker output
│   ├── cli/          # CLI definitions (server + client)
│   ├── network/      # TCP client/server abstraction
│   ├── server/       # server binary entrypoint
│   ├── client/       # client binary entrypoint
│   └── lib.rs        # library root
├── data/
│   └── song.wav      # example input file
├── doc/              # write-up and notes
├── Cargo.toml
└── challenge.md
```

Binaries:

- `server` – TCP audio server.
- `client` – TCP audio client.

---

## Dependencies

Main crates used:

- **[clap](https://crates.io/crates/clap)**
  Command‑line parsing for server and client.

- **[cpal](https://crates.io/crates/cpal)**
  Cross‑platform audio I/O, used on the client to play audio to speakers.

- **[hound](https://crates.io/crates/hound)**
  Reading the input WAV file on the server and writing WAV output on the client.

- **[ringbuf](https://crates.io/crates/ringbuf)**
  Lock‑free ring buffer used between the client’s network thread and CPAL’s audio
  callback thread.

- **[log](https://crates.io/crates/log)** and **[env_logger](https://crates.io/crates/env_logger)**
  Logging and configurable log levels via `RUST_LOG`.

- **[ctrlc](https://crates.io/crates/ctrlc)**
  Handling Ctrl‑C (SIGINT) on the client for clean shutdown.

You don’t need to install these manually; `cargo` will fetch them automatically from
`crates.io`.

---

## Requirements

- Rust 1.91.0.
- Linux or macOS with:
    - a working TCP stack (standard),
    - an audio output device for speaker mode,
    - support for ALSA / CoreAudio through CPAL.

---

## Building

Clone the repository and build both binaries:

```bash
git clone <repo-url> sonos-challenge
cd sonos-challenge

# Build in debug mode
cargo build

# Or optimized build
cargo build --release
```

The binaries will be under `target/debug/` or `target/release/`:

- `target/release/server`
- `target/release/client`

---

## Audio format & limitations

- Input: WAV file (server side).
- Supported format:
    - mono (1 channel),
    - 16‑bit signed PCM (`i16`),
    - any reasonable sample rate (e.g. 44100 Hz).

Other formats are rejected or not handled correctly.

---

## Running the server

The server streams a WAV file to any connected clients.

Basic usage:

```bash
target/release/server --wav data/song.wav --port 8080
```

Options:

- `-w, --wav <FILE>` (required)
  Path to the **existing** input WAV file.

- `-p, --port <PORT>` (optional, default `8080`)
  TCP port to listen on. The server binds to `0.0.0.0:<port>`.

Examples:

```bash
# Stream the example file on port 8080
target/release/server --wav data/song.wav

# Stream a custom file on a custom port
target/release/server --wav /path/to/input.wav --port 5000
```

The server:

- reads the WAV spec and samples using `hound`,
- sends a `Spec` message to all connected clients,
- then repeatedly sends `Samples` messages in chunks of 1000 samples,
- paces sending to approximate real‑time streaming.

New clients:

- receive the current `Spec` immediately on connection,
- then join the regular stream of `Samples` messages.

**Note**: If the client is running on a different machine than the server, make sure
the server’s port is reachable through any firewalls or NAT.

---

## Running the client

The client connects to the server and either:

- writes audio to a WAV file, or
- plays audio through a speaker.

The client CLI has:

- a **subcommand**: `list-available-speakers`.
- a **main mode**: streaming (WAV‑to‑WAV or WAV‑to‑speaker).

### 1. Listing available speakers

```bash
target/release/client list-available-speakers
```

This prints all output devices that CPAL can see, e.g.:

```text
Available speaker devices:
 - Built-in Output
 - External USB Audio
 - Unknown Device 0
```

Use these names with the `--speaker` option when running in speaker mode.

---

### 2. Streaming modes

Main syntax:

```bash
target/release/client --ip <SERVER_IP> --port <PORT> \
  (--file <OUTPUT_WAV> | --default-speaker | --speaker <DEVICE_NAME>)
```

You must pick exactly **one** of:

- `--file <OUTPUT_WAV>`
- `--default-speaker`
- `--speaker <DEVICE_NAME>`

Common arguments:

- `--ip <SERVER_IP>` (required)
  Server IP address (e.g. `127.0.0.1` or your server’s LAN IP).

- `-p, --port <PORT>` (required)
  Server port (must match the port used by the server).

#### a) WAV‑to‑WAV (save to file)

Save the stream into a local WAV file:

```bash
# Save to out.wav, connecting to localhost:8080
target/release/client --ip 127.0.0.1 --port 8080 --file out.wav
```

Notes:

- The directory of `out.wav` must exist.
- The extension must be `.wav` (validated by the CLI).
- The client:
    - waits for the `Spec` message,
    - creates a `WavWriter`,
    - appends each `Samples` message to the file,
    - finalizes the file cleanly on server disconnect or Ctrl‑C.

#### b) WAV‑to‑Speaker (default device)

Play audio through the system’s default output device:

```bash
target/release/client --ip 127.0.0.1 --port 8080 --default-speaker
```

The client:

- receives `Spec` then `Samples`,
- configures CPAL using the OS default device/config,
- pushes samples into a ring buffer,
- the CPAL audio thread consumes the ring buffer and plays the sound.

#### c) WAV‑to‑Speaker (specific device)

First list devices:

```bash
target/release/client list-available-speakers
```

Then choose one of the printed names:

```bash
target/release/client \
  --ip 127.0.0.1 \
  --port 8080 \
  --speaker "Built-in Output"
```

If the device name is invalid, the CLI will emit an error and point you to
`list-available-speakers`.

---

## Logging

The logging level is set to INFO by default.

---

## Testing

There are unit tests for:

- TCP framing (basic send/receive and new‑client message).
- Audio message serialization/deserialization round‑trips.

Run tests with:

```bash
cargo test
```

**Note**: TCP tests use the 50104 and 50105 ports; make sure they are free.

---

## Caveats & future work

- Audio is mono 16‑bit PCM only.
- Pacing and buffering are basic; large network jitter may still cause playback stutters.
- Client→server communication is not yet used; feedback channels (e.g. buffer fullness,
  latency estimates) would help adjust pacing in real time.
- Microphone input on the server is not implemented yet,