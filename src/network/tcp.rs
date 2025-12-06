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
    fn send_frame(stream: &mut TcpStream, data: &[u8], context: &str) -> bool {
        let len_bytes = (data.len() as u32).to_le_bytes();
        let result = stream
            .write_all(&len_bytes)
            .and_then(|_| stream.write_all(data));

        if let Err(e) = result {
            let is_disconnect = matches!(
                e.kind(),
                io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::UnexpectedEof
            );
            if is_disconnect {
                info!("Client disconnected while {}: {}", context, e);
            } else {
                warn!("I/O error while {}: {}", context, e);
            }
            return false;
        }

        true
    }
    pub fn bind(address: &str) -> io::Result<Self> {
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
                            if !Self::send_frame(&mut stream, &data, "sending new client message") {
                                continue;
                            }
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
        if data.len() > u32::MAX as usize {
            info!(
                "New client message length {} exceeds u32 maximum",
                data.len()
            );
        }
        let mut message = self.new_client_message.lock().unwrap_or_else(|poisoned| {
            error!("new_client_message mutex poisoned");
            poisoned.into_inner()
        });
        message.clear();
        message.extend_from_slice(data);
        debug!("Set new client message of {} bytes", data.len());
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
        let mut streams = self.streams.lock().unwrap_or_else(|poisoned| {
            error!("streams mutex poisoned");
            poisoned.into_inner()
        });
        let clients: Vec<TcpStream> = streams.drain(..).collect();
        drop(streams);
        let mut surviving_streams = Vec::with_capacity(clients.len());

        for mut stream in clients {
            if !Self::send_frame(&mut stream, data, "broadcasting data") {
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

#[derive(Debug)]
pub enum TcpClientError {
    ServerDisconnected(io::Error),
    FrameTooLarge { length: usize, max: usize },
    Io(io::Error),
}

impl TcpClient {
    pub fn connect(address: &str) -> io::Result<Self> {
        let stream = TcpStream::connect(address)?;
        Ok(TcpClient { stream })
    }
    fn map_read_error(e: io::Error, context: &'static str) -> TcpClientError {
        if matches!(
            e.kind(),
            io::ErrorKind::UnexpectedEof
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::ConnectionAborted
                | io::ErrorKind::BrokenPipe
        ) {
            debug!("Server closed connection while {}: {}", context, e);
            TcpClientError::ServerDisconnected(e)
        } else {
            error!("I/O error while {}: {}", context, e);
            TcpClientError::Io(e)
        }
    }

    const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024; // 16 MB
    pub fn receive(&mut self, buf: &mut Vec<u8>) -> Result<usize, TcpClientError> {
        let mut length_bytes = [0u8; 4];
        self.stream
            .read_exact(&mut length_bytes)
            .map_err(|e| Self::map_read_error(e, "reading frame length"))?;
        debug!("Length bytes: {:02X?}", length_bytes);
        let length = u32::from_le_bytes(length_bytes) as usize;
        if length > Self::MAX_FRAME_SIZE {
            return Err(TcpClientError::FrameTooLarge {
                length,
                max: Self::MAX_FRAME_SIZE,
            });
        }
        buf.resize(length, 0);
        self.stream
            .read_exact(buf)
            .map_err(|e| Self::map_read_error(e, "reading frame body"))?;
        debug!("Received {} bytes from server", length);
        Ok(length)
    }
}

#[cfg(test)]
mod tests {
    use log::LevelFilter;
    use std::sync::Once;
    use std::thread::sleep;
    use std::time::Duration;
    static INIT: Once = Once::new();
    fn initialize_logger() {
        INIT.call_once(|| {
            env_logger::builder()
                .filter_level(LevelFilter::Trace)
                .init()
        });
    }
    #[test]
    fn broadcast_test() {
        initialize_logger();
        let address = "localhost:50104";
        let mut server = super::TcpServer::bind(address).expect("Failed to start TCP server");
        let mut client =
            super::TcpClient::connect(address).expect("Failed to connect TCP client to server");
        sleep(Duration::from_millis(100)); // Wait for the server to accept the connection
        assert_eq!(server.get_client_count(), 1);

        // Test broadcasting data
        let data = vec![1, 2, 3, 4, 5];
        server
            .broadcast(&data)
            .expect("Failed to broadcast data");
        let mut buffer: Vec<u8> = Vec::new();
        let received_bytes_count = client.receive(&mut buffer).expect("Failed to receive data");
        assert_eq!(received_bytes_count, data.len());
        assert_eq!(buffer, data);
    }
    #[test]
    fn new_client_message_test() {
        initialize_logger();
        let address = "localhost:50104";
        let mut server = super::TcpServer::bind(address).expect("Failed to start TCP server");
        let new_client_message = vec![10, 20, 30, 40, 50];
        server.set_new_client_message(&new_client_message);

        let mut client =
            super::TcpClient::connect(address).expect("Failed to connect TCP client to server");
        sleep(Duration::from_millis(100)); // Wait for the server to accept the connection

        // Verify that the new client received the new_client_message
        let mut buffer: Vec<u8> = Vec::new();
        let received_bytes_count = client.receive(&mut buffer).expect("Failed to receive data");
        assert_eq!(received_bytes_count, new_client_message.len());
        assert_eq!(buffer, new_client_message);
        // Verify that the server has one connected client after acknowledging the new client message
        assert_eq!(server.get_client_count(), 1);
    }
}
