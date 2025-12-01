# Rust \- Audio Server

## Context

One of the main characteristics of our platform is to be able to process audio streams in real-time. These streams are processed by our Wakeword and Automatic Speech Recognition programs and are the source of all interactions with the platform. The purpose of this challenge is to explore ways to distribute the acquisition and processing of such audio streams.

## Task

Your mission, should you choose to accept it, is to create two programs: a server and a client. When clients connect to the server, the latter should send it a stream of audio samples. The client should then be able to use these samples.

## Technical requirements

All the code should be written in [Rust](https://www.rust-lang.org), the communication between the client and the server should be done using a TCP socket.

## Deliverables

* Source code of both programs (should work on Linux and/or macOS)  
* A write-up explaining what you did, why, what worked, and what did not

## Tips and ideas

Here are a few ideas on how to tackle this challenge:

* Start by defining the protocol that will be used between the client and the server.  
* We are expecting a working demo sending audio between the server & the client. This first demo should be a solid base to further implement additional features.  
* The server could source the audio from a wav file and the client could write the received audio to a wav file.  
* If you have time, implement more features (like a better command-line interface, microphone and speaker support, etc.)  
* **First and foremost,** make sure to apply software engineering best practices and rust idioms, as you will be mainly evaluated on this.

## AI Usage

Coding assistants are allowed, but you must indicate in your write-up how they were leveraged. Keep in mind we will ask questions on implementation details, so you need to understand what you wrote.

## Useful resources

* [Rust documentation](https://doc.rust-lang.org/)  
  * Most notably the [Rust book](https://doc.rust-lang.org/stable/book/index.html)  
* [Hound](https://docs.rs/hound)  
* A Rust crate to get audio from a microphone and play audio to a speaker: [CPAL](https://crates.io/crates/cpal)