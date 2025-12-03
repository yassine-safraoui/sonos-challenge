use log::{debug, error, info};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::{io, thread};

pub struct TcpServer {
    streams: Arc<Mutex<VecDeque<TcpStream>>>,
    new_client_message: Arc<Mutex<Vec<u8>>>,
}

impl TcpServer {
    pub fn init(address: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        let streams = Arc::new(Mutex::new(VecDeque::new()));
        let streams_for_thread = Arc::clone(&streams);
        let new_client_message = Arc::new(Mutex::new(Vec::new()));
        let new_client_message_for_thread = Arc::clone(&new_client_message);
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        match stream.peer_addr() {
                            Ok(addr) => info!("Accepted connection from {}", addr),
                            Err(e) => error!("Could not get peer address: {}", e),
                        }
                        let data =
                            new_client_message_for_thread
                                .lock()
                                .unwrap_or_else(|poisoned| {
                                    error!("new_client_message mutex poisoned");
                                    poisoned.into_inner()
                                });
                        let data_len = (data.len() as u32).to_le_bytes();
                        match stream.write_all(data_len.as_slice()) {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Error: {}", e);
                                continue;
                            }
                        };
                        match stream.write_all(data.as_slice()) {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Error: {}", e);
                                continue;
                            }
                        };
                        // the stream is closed on drop, when the variable goes out of scope

                        match streams_for_thread.lock() {
                            Ok(mut streams) => streams.push_front(stream),
                            Err(_) => {
                                error!("streams mutex poisoned");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        });
        Ok(TcpServer {
            streams,
            new_client_message,
        })
    }

    pub fn set_new_client_message(&mut self, data: &[u8]) {
        let mut message = self.new_client_message.lock().unwrap_or_else(|poisoned| {
            error!("new_client_message mutex poisoned");
            poisoned.into_inner()
        });
        message.clear();
        message.extend_from_slice(data);
    }

    pub fn get_client_count(&self) -> usize {
        let streams = self.streams.lock().unwrap_or_else(|poisoned| {
            error!("streams mutex poisoned");
            poisoned.into_inner()
        });
        streams.len()
    }

    pub fn broadcast(&mut self, data: &[u8]) -> io::Result<()> {
        if data.len() > u32::MAX as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Data length exceeds u32 maximum",
            ));
        }
        let data_len = (data.len() as u32).to_le_bytes();

        let mut streams = self.streams.lock().unwrap_or_else(|poisoned| {
            error!("streams mutex poisoned");
            poisoned.into_inner()
        });
        let mut clients: Vec<TcpStream> = streams.drain(..).collect();
        let mut surviving_streams = Vec::with_capacity(clients.len());

        for mut stream in clients.drain(..) {
            if let Err(e) = stream.write_all(&data_len) {
                error!("Error sending data length to client: {}", e);
                continue;
            }
            if let Err(e) = stream.write_all(data) {
                error!("Error sending data to client: {}", e);
                continue;
            }
            surviving_streams.push(stream);
        }

        for stream in surviving_streams {
            streams.push_back(stream);
        }
        debug!(
            "Broadcasted {} bytes to {} clients",
            data.len(),
            streams.len()
        );
        Ok(())
    }
}

pub struct TcpClient {
    stream: TcpStream,
}

impl TcpClient {
    pub fn init(address: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(address)?;
        Ok(TcpClient { stream })
    }

    const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MB
    pub fn receive(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut length_bytes = [0u8; 4];
        self.stream.read_exact(&mut length_bytes)?;
        let length = u32::from_le_bytes(length_bytes) as usize;
        debug!("Expecting to receive {} bytes from server", length);
        if length > Self::MAX_FRAME_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Frame size {} exceeds maximum {}",
                    length,
                    Self::MAX_FRAME_SIZE
                ),
            ));
        }
        buf.resize(length, 0);
        self.stream.read_exact(buf)?;
        debug!("Received {} bytes from server", length);
        Ok(length)
    }
}
