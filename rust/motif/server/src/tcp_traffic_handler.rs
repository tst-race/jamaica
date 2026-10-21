// Copyright 2024 Two Six Technologies.

use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use log::{debug, error, info, trace, warn};

const TRANSMIT_BUFFER_SIZE: usize = 128;

pub struct TcpTrafficHandler {
    pub listen_thread_handle: std::thread::JoinHandle<()>,
}

impl TcpTrafficHandler {
    pub fn new(port: u16) -> Self {
        // TCP receive thread.
        let listen_thread_handle = std::thread::spawn(move || {
            let server_addr = "0.0.0.0:19305";
            let listener = TcpListener::bind(server_addr).expect(
                format!("[SERVER] TCP failed to open listener on {}", server_addr).as_str(),
            );

            info!("TCP server listening on port {}", port);

            // let request_router = TcpRequestRouter::new(tcp_sender.clone());

            let pool = threadpool::ThreadPool::new(4);

            // TODO: probably want to loop over accept after setting it to non-blocking so we can exit
            //       gracefully. Maybe? What do real servers do?
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let (tcp_sender, tcp_receiver) =
                            mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(TRANSMIT_BUFFER_SIZE);
                        std::thread::spawn(move || {
                            debug!(
                                "Spawned thread to handle TCP connection from {:?}",
                                stream.peer_addr()
                            );
                            // std::thread::spawn(|| {
                            //     handle_stream(stream);
                            // });
                            let mut buffer = [0; 1500];
                            let mut stream = stream;

                            let server_tcp_connection_timeout = 10_000;
                            match stream.set_read_timeout(Some(std::time::Duration::from_millis(
                                server_tcp_connection_timeout,
                            ))) {
                                Err(error) => {
                                    warn!("{:?}", error);
                                }
                                _ => (),
                            };
                            let request_router = TcpRequestRouter::new(tcp_sender.clone());

                            let mut stream_clone = stream.try_clone().unwrap();
                            let tcp_transmitter_handle = std::thread::spawn(move || {
                                for (payload, addr) in tcp_receiver.iter() {
                                    debug!(
                                        "[TCP sender thread] sending {} bytes to {}",
                                        payload.len(),
                                        addr
                                    );
                                    stream_clone.write_all(payload.as_slice()).unwrap();
                                    // thread_socket.send_to(&payload, addr).expect("ahhh no");
                                }
                            });

                            // TODO: spawn thread to handle this and give it a server_handler.
                            //       OR create a mapping of addr => transmitter(TCP|UDP), use channels, etc.
                            loop {
                                // https://doc.rust-lang.org/std/net/struct.TcpStream.html
                                // https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
                                match stream.read(&mut buffer) {
                                    Ok(size) if size == 0 => {
                                        debug!("stream closed for {:?}", stream.peer_addr());
                                        break;
                                    }
                                    Ok(size) => {
                                        // stream.write_all(&buffer[0..size]).unwrap();
                                        debug!(
                                            "received {} bytes from {:?}",
                                            size,
                                            stream.peer_addr()
                                        );
                                        // stream.write_all("response".as_bytes()).unwrap();
                                        debug!("request router");
                                        // Copy the bytes out of buffer slice into a vec, or create an empty vector if zero bytes.
                                        let payload = if size > 0 {
                                            buffer[..size].to_vec()
                                        } else {
                                            vec![]
                                        };
                                        request_router.sender
                                            .send((payload, stream.peer_addr().expect("noooooooooo")))
                                            .expect(
                                                "[ERROR] [SERVER] failed to route received payload message",
                                            );
                                    }
                                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                                        warn!("TCP stream listener timeout: {:?}", err);
                                    }
                                    Err(err) => {
                                        error!("TCP stream listener loop: {:?}", err);
                                        break;
                                    }
                                }
                            }

                            match tcp_transmitter_handle.join() {
                                Err(error) => {
                                    error!("{:?}", error);
                                    // TODO: should we panic here?
                                }
                                _ => (),
                            };
                        });
                    }
                    Err(err) => {
                        error!("{}", err);
                    }
                }
                trace!("loop back end of for stream in listener.incoming()");
            }

            pool.join();

            debug!("TCP thread exiting.");
        });

        Self {
            listen_thread_handle,
        }
    }
}

// TODO: move to own file
use motiflib::quic_handler::QuicServerHandler;
use motiflib::server_handler::ServerHandler;
use motiflib::stun_handler::StunHandler;
use motiflib::web_rtc_handler::WebRtcHandler;

struct TcpRequestRouter {
    pub sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
}

impl TcpRequestRouter {
    fn new(socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(128);

        let _ = thread::spawn(move || {
            let mut handler = StunHandler::new(socket_sender.clone());
            handler.spawn();

            for (payload, addr) in receiver.iter() {
                handler.handle_message(payload, &addr);
            }
        });

        Self { sender }
    }
}
