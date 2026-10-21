// Copyright 2024 Two Six Technologies.

use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

use crate::server_handler::ServerHandler;

use crate::cient_control_messages::{ClientControlMessage, ContentType};
use crate::dtls;
use crate::rtp;

use log::{debug, error, info, warn};

#[derive(Debug)]
pub struct WebRtcHandler {
    socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
    thread_handle: Option<thread::JoinHandle<()>>,
    input_tx: Option<mpsc::SyncSender<(Vec<u8>, SocketAddr)>>,
}

impl WebRtcHandler {
    pub fn new(socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>) -> Self {
        Self {
            socket_sender,
            thread_handle: None,
            input_tx: None,
        }
    }

    pub fn spawn(&mut self) {
        if !self.thread_handle.is_none() {
            panic!("thread has already been spawned!");
        }

        let (input_tx, input_rx) = mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(128);

        let socket_sender = self.socket_sender.clone();
        self.thread_handle = Some(thread::spawn(move || {
            for (payload, addr) in input_rx.iter() {
                debug!(
                    "[WebRtcServerHandler] [RECEIVE] payload of size {} from {}",
                    payload.len(),
                    addr
                );

                // Try to decode client control message.
                let control_message = ClientControlMessage::from_vec(&payload);
                debug!("client control message = {:?}", control_message);

                // let payload = (0..100).map(|_| rand::random()).collect();
                // socket_sender
                //     .send((payload, addr))
                //     .expect("[WebRtcServerHandler] send failed");

                if let Ok(control_message) = control_message {
                    for response in &control_message.responses {
                        let content_type = ContentType::from_u64(response.0);
                        let response_size = response.1 as usize;
                        debug!(
                            "[SEND] sending client controlled response {:?}",
                            content_type
                        );
                        match content_type {
                            Some(ContentType::RTP) => {
                                let rtp_packet = rtp::create_rtp_packet(response.1 as usize);
                                socket_sender
                                    .send((rtp_packet, addr))
                                    .expect("[WebRtcServerHandler] send failed")
                            }
                            Some(ContentType::DTLSv1_2ServerHello) => {
                                let server_hello = dtls::create_server_hello(response_size);
                                socket_sender
                                    .send((server_hello, addr))
                                    .expect("[WebRtcServerHandler] send failed")
                            }
                            Some(ContentType::DTLSv1_2ApplicationData) => {
                                let application_data = dtls::create_application_data(response_size);
                                socket_sender
                                    .send((application_data, addr))
                                    .expect("[WebRtcServerHandler] send failed")
                            }
                            _ => error!(
                                "don't know how to generate response for content type {:?}",
                                content_type
                            ),
                        }
                    }
                } else {
                    error!("invalid control message");
                }
            }
            info!("thread exit");
        }));

        self.input_tx = Some(input_tx);
    }
}

impl ServerHandler for WebRtcHandler {
    fn handle_message(&self, payload: Vec<u8>, addr: &std::net::SocketAddr) -> bool {
        match self.input_tx.as_ref() {
            Some(input) => input
                .send((payload, *addr))
                .expect("[WebRtcServerHandler] failed to send message onto thread channel"),
            None => panic!("thread channel not yet created!"),
        };
        true
    }
}
