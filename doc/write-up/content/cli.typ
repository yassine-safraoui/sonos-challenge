= Command line interface

To allow users to easily interact with the client and server and access the various features, I implemented a command‑line interface for both components. I used the `clap` crate for argument parsing and the `ctrlc` crate to handle the `SIGINT` signal on the client so the program can terminate cleanly when the user presses Ctrl‑C.

The CLI supports the following:

- *Server side:*
  - Passing the path of the WAV file that the server should stream.

- *Client side:*
  - Listing available speaker device names.
  - Selecting the output mode:
    - *WAV file output mode:* the user provides a path where the received audio will be saved.
    - *Speaker output mode:* the user can either play audio through the default speaker or select a specific speaker device from the list of available devices.
