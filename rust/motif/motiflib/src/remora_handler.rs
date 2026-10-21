// Copyright 2024 Two Six Technologies.

use crate::cient_control_messages::{ClientControlMessage, ContentType};
use crate::packet;
use crate::server_handler::ServerHandler;

use std::collections::HashMap;
use std::hash::Hash;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

// use motiflib::
use crate::remora_control_message::{
    decode_remora_control_message, generate_packet, FieldJson, MessageDecoder,
};

use log::{debug, error, info, trace, warn};

// TODO: change this name, maybe GruntHandler or MotifJsonHandler, or just MotifHandler.
#[derive(Debug)]
pub struct RemoraServerHandler {
    socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
    thread_handle: Option<thread::JoinHandle<()>>,
    input_tx: Option<mpsc::SyncSender<(Vec<u8>, SocketAddr)>>,
}

impl RemoraServerHandler {
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
            let mut message_decoder = MessageDecoder::new(HashMap::from([(0, vec![(90, 1160)])]));
            let mut packet_index = 0;
            let mut control_messages: HashMap<usize, Vec<Vec<FieldJson>>> = HashMap::new();

            for (payload, addr) in input_rx.iter() {
                info!(
                    "payload index {} of size {} from {}",
                    packet_index,
                    payload.len(),
                    addr
                );

                {
                    debug!("decoding client control message...");
                    // Try to decode client control message.
                    // let control_message = ClientControlMessage::from_vec(&payload);
                    // debug!(
                    //     "client control message = {:?}",
                    //     control_message
                    // );

                    // if let Ok(control_message) = control_message {
                    //     for response in &control_message.responses {
                    //         let content_type = ContentType::from_u64(response.0);
                    //         let response_size = response.1 as usize;

                    //     }
                    // } else {
                    //     error!("invalid control message");
                    // }

                    let control_message = message_decoder.decode_from_payload(&payload);
                    // debug!("control_message = {:#?}", control_message);
                    for (index, messages) in control_message.into_iter() {
                        control_messages
                            .entry(index)
                            .or_insert_with(|| vec![])
                            .extend(messages);
                    }

                    trace!("control_messages = {:#?}", control_messages);

                    match control_messages.get(&packet_index) {
                        Some(message) => {
                            for response in message {
                                let payload = generate_packet(&response, &vec![], 0);
                                socket_sender
                                    .send((payload, addr))
                                    .expect("[QuicServerHandler] send failed");
                            }
                        }
                        None => {
                            debug!("no responses for client packet with index {}", packet_index);
                        }
                    }

                    // match decode_remora_control_message(&payload) {
                    //     Ok(control_message) => {
                    //         debug!("received control message with {} responses", control_message.len());
                    //         for response in control_message {
                    //             let payload = generate_packet(&response, &vec![], 0);
                    //             socket_sender
                    //                 .send((payload, addr))
                    //                 .expect("[QuicServerHandler] send failed");
                    //         }
                    //     }
                    //     Err(err)=> error!("{}", err),
                    // }
                }

                packet_index += 1;
            }
            debug!("thread exit");
        }));

        self.input_tx = Some(input_tx);
    }
}

impl ServerHandler for RemoraServerHandler {
    fn handle_message(&self, payload: Vec<u8>, addr: &std::net::SocketAddr) -> bool {
        match self.input_tx.as_ref() {
            Some(input) => input
                .send((payload, *addr))
                .expect("failed to send message onto thread channel"),
            None => panic!("thread channel not yet created!"),
        };

        true
    }
}
