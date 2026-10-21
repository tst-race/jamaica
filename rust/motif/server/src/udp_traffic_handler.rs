// Copyright 2024 Two Six Technologies.

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;

use motiflib::quic_handler::QuicServerHandler;
use motiflib::remora_handler::RemoraServerHandler;
use motiflib::server_handler::ServerHandler;
use motiflib::web_rtc_handler::WebRtcHandler;

use log::Level::Trace;
use log::{debug, error, info, log_enabled, trace, warn};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const TRANSMIT_BUFFER_SIZE: usize = 128;

const MTU_BYTES: usize = 1500;

pub struct UdpTrafficHandler {
    socket: UdpSocket,
    router: UdpRequestHandler,
    pub udp_recv_thread: JoinHandle<()>,
}

impl UdpTrafficHandler {
    pub fn new(port: u16) -> Self {
        let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
            Ok(socket) => socket,
            Err(error) => {
                error!("{:?}", error);
                panic!("{:?}", error);
            }
        };
        info!("UDP server listening on port {}", port);
        // UDP socket send thread.
        let (udp_sender, udp_receiver) =
            mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(TRANSMIT_BUFFER_SIZE);
        let thread_socket = socket.try_clone().unwrap();
        // TODO: save this thread handle as struct member and join.
        let udp_recv_thread = thread::spawn(move || {
            for (payload, addr) in udp_receiver.iter() {
                debug!(
                    "[UDP socket thread] sending {} bytes to {}",
                    payload.len(),
                    addr
                );
                thread_socket.send_to(&payload, addr).expect("ahhh no");
            }
        });

        Self {
            socket,
            router: UdpRequestHandler::new(udp_sender.clone()),
            udp_recv_thread,
        }
    }

    /// Start the receive loop. Will block indefinitely.
    // TODO: thread this instead of blocking and join on handle.
    pub fn recv(&self) {
        loop {
            let mut buf = [0; MTU_BYTES]; // TODO: set size to MTU, or can this be dynamic?
            let (amt, src) = match self.socket.recv_from(&mut buf) {
                Ok((amt, src)) => (amt, src),
                Err(error) => {
                    error!("{:?}", error);
                    panic!("{:?}", error);
                }
            };
            debug_assert!(amt <= MTU_BYTES);
            if log_enabled!(Trace) {
                let mut hasher = DefaultHasher::new();
                hasher.write(&buf[0..amt]);
                trace!(
                    "received {:?} bytes from {:?} with hash {}",
                    amt,
                    src,
                    hasher.finish()
                );
            } else {
                debug!("received {:?} bytes from {:?}", amt, src);
            }

            let payload = if amt > 0 { buf[..amt].to_vec() } else { vec![] };
            self.router
                .sender
                .send((payload, src))
                .expect("[ERROR] [SERVER] failed to route received payload message");
        }
    }
}

// TODO: put in own file.
struct UdpRequestHandler {
    pub sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
}

// TODO: rename. RequestRouter? SessionManager?
impl UdpRequestHandler {
    fn new(socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>) -> Self {
        // let (receive_channel_tx, receive_channel_rx) = mpsc::sync_channel(128);

        // let receive_channel_rx = self.receive_channel_rx;
        let (sender, receiver) = mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(128);

        let _ = thread::spawn(move || {
            let handlers: Vec<Box<dyn ServerHandler>> = vec![
                Box::new(QuicServerHandler::new(socket_sender.clone())),
                Box::new(WebRtcHandler::new(socket_sender.clone())),
            ];
            // let mut handler = QuicServerHandler::new(socket_sender.clone());
            // let mut handler = WebRtcHandler::new(socket_sender.clone());
            let mut handler = RemoraServerHandler::new(socket_sender.clone());
            handler.spawn();

            info!("Server configured with UDP handler {:?}", handler);

            for (payload, addr) in receiver.iter() {
                // println!("[UdpRequestHandler] received payload of size {}. Sending stuff back...", payload.len());

                // let action = handler.get_next_action(payload, &addr);
                // match action {
                //     Action::Send(send_action) => send(&socket, &src, send_action.payload),
                //     Action::Receive(_) => continue, //receive(&socket, receive_action.timeout as u64),
                //     Action::Quit => break,
                // }

                // socket_sender.send((vec![0;99], addr));

                handler.handle_message(payload, &addr);
            }
        });

        Self {
            // receive_channel_tx,
            // receive_channel_rx,
            sender,
            // handler: QuicServerHandler::new(),
            // socket,
        }
    }
}
