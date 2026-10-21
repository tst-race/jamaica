// Copyright 2024 Two Six Technologies.

use std::io::{Read, Write};
use std::net::SocketAddr;
use std::net::TcpStream;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::server_comm::ServerComm;

use log::{debug, error, info, warn};

#[derive(Debug)]
pub struct ServerCommTcp {
    remote_addr: SocketAddr,
    pub transmit_channel_tx: mpsc::SyncSender<Vec<u8>>,
    transmit_channel_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>, // TODO: does it make sense to hold onto this, or just return it from the spawn method?
    receive_channel_tx: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
    pub receive_channel_rx: mpsc::Receiver<(Vec<u8>, SocketAddr)>, // TODO: use same type for sending and receiving? we'll probably want to return the sender's address, and also don't care about timeout on the receive side. We need to transmit errors to so change this, or wrap in a Result probably.
}

const TRANSMIT_BUFFER_SIZE: usize = 128;
const RECEIVE_BUFFER_SIZE: usize = 128;

impl ServerCommTcp {
    // let message = "What's up?";
    // stream.write_all(message.as_bytes()).expect("come on, man!");

    // let mut buffer = [0;1024];
    // match stream.read(&mut buffer) {
    //     Ok(size) => {
    //         let response = String::from_utf8_lossy(&buffer[0..size]);
    //         println!("server responded with: {}", response);
    //     }
    //     Err(err) => {
    //         eprintln!("CLIENT ERROR: {}", err);
    //     }
    // }

    pub fn new(remote_addr: SocketAddr) -> Self {
        let (transmit_channel_tx, transmit_channel_rx) = mpsc::sync_channel(TRANSMIT_BUFFER_SIZE);
        let (receive_channel_tx, receive_channel_rx) = mpsc::sync_channel(RECEIVE_BUFFER_SIZE);

        let transmit_channel_rx = Arc::new(Mutex::new(transmit_channel_rx));

        Self {
            remote_addr,
            transmit_channel_tx,
            transmit_channel_rx,
            receive_channel_tx,
            receive_channel_rx,
        }
    }

    /// Spawn threads for sending and receiving.
    pub fn spawn(&self) -> std::io::Result<()> {
        debug!("Opening TCP connection to {}", self.remote_addr);
        // TODO: how long should timeout be? Make it configurable?
        let timeout = std::time::Duration::from_millis(20_000);
        let mut stream = match TcpStream::connect_timeout(&self.remote_addr, timeout) {
            Ok(stream) => stream,
            Err(error) => {
                error!("TCP connection failed to {} : {}", self.remote_addr, error);
                return Err(error);
            }
        };

        // TODO: add API to get this info instead of printing it here.
        debug!("Opened TCP connection on {}", stream.local_addr().unwrap());
        // println!("Opened TCP connection on {}:{}", stream.local_addr().unwrap().ip(), stream.local_addr().unwrap().port());

        // let read_socket = socket.try_clone().unwrap();
        // Spawn a thread to write.
        let transmit_channel_rx = self.transmit_channel_rx.clone();
        let mut stream_clone = stream.try_clone().expect("clone failed...");
        let _ = thread::spawn(move || {
            for payload in transmit_channel_rx.lock().unwrap().iter() {
                debug!("Sending payload of size {} ", payload.len());
                match stream.write_all(&payload) {
                    Ok(_) => (),
                    Err(err) => error!("send_to error: {:?}", err), // TODO: send errors over receive channel, or some other mechanism?
                }
            }

            debug!("calling shutdown on stream {:?}", stream);
            // stream.shutdown(std::net::Shutdown::Both);
            let _ = stream.shutdown(std::net::Shutdown::Write);

            debug!("send thread exit");
        });

        // Spawn a thread for reading.
        let receive_channel_tx = self.receive_channel_tx.clone();
        let src_addr = self.remote_addr;
        let _ = thread::spawn(move || {
            loop {
                const MTU: usize = 1500; // TODO: safe to assume MTU of 1500?
                let mut buffer = [0; MTU];
                // TODO: add a timeout to periodically check if conversation has finished?
                match stream_clone.read(&mut buffer) {
                    Ok(size) => {
                        let _ = receive_channel_tx.send((buffer[..size].to_vec(), src_addr));
                        // let response = String::from_utf8_lossy(&buffer[0..size]);
                        // println!("server responded with: {}", response);
                    }
                    Err(err) => {
                        error!("stream read loop: {}", err);
                        break;
                    }
                }
            }
            debug!("read thread exit");
        });

        debug!("spawn returning");

        Ok(())
    }
}

impl ServerComm for ServerCommTcp {
    fn send(&self, payload: Vec<u8>) -> Result<(), mpsc::SendError<Vec<u8>>> {
        self.transmit_channel_tx.send(payload)
    }

    fn receive(
        &self,
        timeout: std::time::Duration,
    ) -> Result<(Vec<u8>, SocketAddr), mpsc::RecvTimeoutError> {
        self.receive_channel_rx.recv_timeout(timeout)
    }

    fn close(&self) -> std::io::Result<()> {
        Ok(())
    }
}
