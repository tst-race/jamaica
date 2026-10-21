// Copyright 2024 Two Six Technologies.

use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

use crate::server_handler::ServerHandler;

use crate::cient_control_messages::{ClientControlMessage, ContentType};
use crate::stun;

use log::{debug, error, info, warn};

pub struct StunHandler {
    socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
    thread_handle: Option<thread::JoinHandle<()>>,
    input_tx: Option<mpsc::SyncSender<(Vec<u8>, SocketAddr)>>,
}

impl StunHandler {
    pub fn new(socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>) -> Self {
        Self {
            socket_sender,
            thread_handle: None,
            input_tx: None,
        }
    }

    pub fn spawn(&mut self) {
        if !self.thread_handle.is_none() {
            let error_message = "thread has already been spawned!";
            error!("{}", error_message);
            panic!("{}", error_message);
        }

        let (input_tx, input_rx) = mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(128);

        let socket_sender = self.socket_sender.clone();
        self.thread_handle = Some(thread::spawn(move || {
            for (payload, addr) in input_rx.iter() {
                debug!("[RECEIVE] payload of size {} from {}", payload.len(), addr);

                // Try to decode client control message.
                let control_message = ClientControlMessage::from_vec(&payload);
                debug!("client control message = {:?}", control_message);

                let stun_packet = stun::StunPacket::create_binding_success_response();
                match socket_sender.send((stun_packet, addr)) {
                    Ok(_) => (),
                    Err(error) => {
                        error!("{}", error);
                        panic!("{}", error);
                    }
                }

                continue;

                if stun::StunPacket::is_stun_binding_request(&payload) {
                    let stun_packet = stun::StunPacket::create_binding_success_response();
                    socket_sender
                        .send((stun_packet, addr))
                        .expect("[StunHandler] send failed");
                } else if let Ok(control_message) = control_message {
                    for response in &control_message.responses {
                        let content_type = ContentType::from_u64(response.0);
                        let response_size = response.1 as usize;
                        debug!(
                            "[SEND] sending client controlled response {:?}",
                            content_type
                        );
                        match content_type {
                            Some(ContentType::STUNBindingSuccessResponse) => {
                                let stun_packet =
                                    stun::StunPacket::create_binding_success_response();
                                socket_sender
                                    .send((stun_packet, addr))
                                    .expect("[StunHandler] send failed")
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

impl ServerHandler for StunHandler {
    fn handle_message(&self, payload: Vec<u8>, addr: &std::net::SocketAddr) -> bool {
        match self.input_tx.as_ref() {
            Some(input) => input
                .send((payload, *addr))
                .expect("[StunHandler] failed to send message onto thread channel"),
            None => panic!("thread channel not yet created!"),
        };
        true
    }
}
