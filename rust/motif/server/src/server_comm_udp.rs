// Copyright 2024 Two Six Technologies.

use log::debug;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::server_comm::ServerComm;

#[derive(Debug)]
pub struct ServerCommUdp {
    host_addr: SocketAddr,
    server_addr: SocketAddr,
    pub transmit_channel_tx: mpsc::SyncSender<Vec<u8>>,
    transmit_channel_rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>, // TODO: does it make sense to hold onto this, or just return it from the spawn method?
    receive_channel_tx: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
    pub receive_channel_rx: mpsc::Receiver<(Vec<u8>, SocketAddr)>, // TODO: use same type for sending and receiving? we'll probably want to return the sender's address, and also don't care about timeout on the receive side. We need to transmit errors to so change this, or wrap in a Result probably.
}

const TRANSMIT_BUFFER_SIZE: usize = 128;
const RECEIVE_BUFFER_SIZE: usize = 128;

impl ServerCommUdp {
    pub fn new(host_addr: SocketAddr, server_addr: SocketAddr) -> Self {
        let (transmit_channel_tx, transmit_channel_rx) = mpsc::sync_channel(TRANSMIT_BUFFER_SIZE);
        let (receive_channel_tx, receive_channel_rx) = mpsc::sync_channel(RECEIVE_BUFFER_SIZE);

        let transmit_channel_rx = Arc::new(Mutex::new(transmit_channel_rx));

        Self {
            host_addr,
            server_addr,
            transmit_channel_tx,
            transmit_channel_rx,
            receive_channel_tx,
            receive_channel_rx,
        }
    }

    /// Spawn threads for sending and receiving.
    pub fn spawn(&self) -> std::io::Result<()> {
        let socket = UdpSocket::bind(self.host_addr)?;
        let read_socket = socket.try_clone().unwrap();
        let transmit_channel_rx = self.transmit_channel_rx.clone();
        let addr = self.server_addr.clone();
        // Send thread.
        let _ = thread::spawn(move || {
            for payload in transmit_channel_rx.lock().unwrap().iter() {
                debug!("UDP send {}", addr);
                match socket.send_to(&payload, addr) {
                    Ok(_) => (),
                    Err(err) => eprintln!("send_to error: {:?}", err), // TODO: send errors over receive channel, or some other mechanism?
                }
            }

            debug!("send thread exit");
        });

        let receive_channel_tx = self.receive_channel_tx.clone();
        // Receive thread.
        let _ = thread::spawn(move || {
            loop {
                let mut buf = [0; 16384]; // TODO: set size to MTU, or can this be dynamic?
                let result = read_socket.recv_from(&mut buf);

                let (num_bytes, src_addr) = match result {
                    Ok((num_bytes, src_addr)) => (num_bytes, src_addr),
                    Err(error) => panic!("{:?}", error),
                };

                let _ = receive_channel_tx.send((buf[..num_bytes].to_vec(), src_addr));
            }
        });

        // TODO: thread that blocks on recv_from, then calls connect on the src addr and spawns a session.

        Ok(())
    }
}

impl ServerComm for ServerCommUdp {
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
