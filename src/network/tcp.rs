use log::{debug, error, info};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread};

pub struct TcpServer {
    streams: Arc<Mutex<VecDeque<TcpStream>>>,
    new_client_message: Arc<Mutex<Vec<u8>>>,
    handle: Option<thread::JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
}

impl TcpServer {
    pub fn init(address: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;

        let streams = Arc::new(Mutex::new(VecDeque::new()));
        let streams_for_thread = Arc::clone(&streams);

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_thread = Arc::clone(&shutdown);

        let new_client_message = Arc::new(Mutex::new(Vec::new()));
        let new_client_message_for_thread = Arc::clone(&new_client_message);
        let handle = thread::spawn(move || {
            loop {
                if shutdown_for_thread.load(Ordering::Relaxed) {
                    info!("Shutting down TCP server listener thread");
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
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
                        if !data.is_empty() {
                            debug!("Sending {} bytes to new client", data.len());
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
                        }
                        match streams_for_thread.lock() {
                            Ok(mut streams) => streams.push_front(stream),
                            Err(_) => {
                                error!("streams mutex poisoned");
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // no pending connection; give up the CPU a bit
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(e) => {
                        error!("Could not accept connection: {}", e);
                        continue;
                    }
                }
            }
        });
        Ok(TcpServer {
            streams,
            new_client_message,
            handle: Some(handle),
            shutdown,
        })
    }

    pub fn set_new_client_message(&mut self, data: &[u8]) {
        let mut message = self.new_client_message.lock().unwrap_or_else(|poisoned| {
            error!("new_client_message mutex poisoned");
            poisoned.into_inner()
        });
        message.clear();
        message.extend_from_slice(data);
        debug!("Set new client message of {} bytes", data.len());
        for byte in &data[..data.len().min(10)] {
            debug!("{:02X} ", byte);
        }
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
        let clients: Vec<TcpStream> = streams.drain(..).collect();
        drop(streams);
        let mut surviving_streams = Vec::with_capacity(clients.len());

        for mut stream in clients {
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
        let mut streams = self.streams.lock().unwrap_or_else(|poisoned| {
            error!("streams mutex poisoned");
            poisoned.into_inner()
        });
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

impl Drop for TcpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take()
            && let Err(e) = handle.join()
        {
            error!("TCP listener thread panicked: {:?}", e);
        }
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
        let mut length_bytes = [1u8; 4];
        self.stream.read_exact(&mut length_bytes)?;
        debug!("Length bytes: {:02X?}", length_bytes);
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
