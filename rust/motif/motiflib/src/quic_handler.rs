// Copyright 2024 Two Six Technologies.

use crate::cient_control_messages::{ClientControlMessage, ContentType};
use crate::quic::{
    create_quic_handshake_packet, create_quic_protected_payload_packet,
    get_destination_connection_id_from_packet, get_random_hex_string,
    get_source_connection_id_from_initial_packet,
};
use crate::server_handler::ServerHandler;

use std::net::SocketAddr;
use std::sync::mpsc;
use std::thread;

use log::{debug, error, info, warn};

#[derive(Debug)]
pub struct QuicServerHandler {
    socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>,
    thread_handle: Option<thread::JoinHandle<()>>,
    input_tx: Option<mpsc::SyncSender<(Vec<u8>, SocketAddr)>>,
}

impl QuicServerHandler {
    pub fn new(socket_sender: mpsc::SyncSender<(Vec<u8>, SocketAddr)>) -> Self {
        Self {
            socket_sender,
            thread_handle: None,
            input_tx: None,
        }
    }
}

impl ServerHandler for QuicServerHandler {
    fn handle_message(&self, payload: Vec<u8>, addr: &std::net::SocketAddr) -> bool {
        // self.input_tx.unwrap().send((payload, *addr));
        match self.input_tx.as_ref() {
            Some(input) => input
                .send((payload, *addr))
                .expect("[QuicServerHandler] failed to send message onto thread channel"),
            None => panic!("thread channel not yet created!"),
        };
        // self.input_tx.as_ref().unwrap().clone().send((payload, *addr));

        true
    }
}

impl QuicServerHandler {
    pub fn spawn(&mut self) {
        if !self.thread_handle.is_none() {
            panic!("thread has already been spawned!");
        }

        let (input_tx, input_rx) = mpsc::sync_channel::<(Vec<u8>, SocketAddr)>(128);

        // TODO: not sure how the server is coming up with a connection ID. Just create
        //       a random one for now. Quinn uses an 8-byte CID, whereas Instagram (in a capture I
        //       generated) uses a 3-byte. Not sure if we want this configurable, or may be
        //       something we can tweak to adjust traffic shape?
        let server_connection_id = get_random_hex_string(8);

        // let receive_channel_rx = self.receive_channel_rx;
        // let (sender, receiver) = mpsc::sync_channel(128);
        let socket_sender = self.socket_sender.clone();
        self.thread_handle = Some(thread::spawn(move || {
            for (payload, addr) in input_rx.iter() {
                println!(
                    "[QuicServerHandler] [RECEIVE] payload of size {} from {}",
                    payload.len(),
                    addr
                );
                // for action in &self.actions {

                // }
                // loop {
                //     match actions.pop_front().expect("ahh dammit") {
                //         Action::Send(send_action) => socket_sender.send((send_action.payload, addr)).expect("[QuicServerHandler] send failed"),
                //         Action::Receive(_) => break, //receive(&socket, receive_action.timeout as u64),
                //         Action::Quit => break 'outer,
                //     };
                // }

                if Self::is_quic_initial_packet(&payload) {
                    let destination_connection_id =
                        get_source_connection_id_from_initial_packet(&payload);
                    println!(
                        "[QuicServerHandler] [SEND] responding to initial packet from {} with handshake and protected payload", destination_connection_id
                    );
                    let payload = create_quic_handshake_packet(
                        destination_connection_id.as_str(),
                        server_connection_id.as_str(),
                        0,
                    );
                    socket_sender
                        .send((payload, addr))
                        .expect("[QuicServerHandler] send failed");

                    let payload = create_quic_protected_payload_packet(
                        138,
                        destination_connection_id.as_str(),
                    );
                    socket_sender
                        .send((payload, addr))
                        .expect("[QuicServerHandler] send failed");
                } else if let Some(responses) = Self::get_canned_response(&payload) {
                    for response in responses {
                        println!("[QuicServerHandler] [SEND] responding to known payload with canned response");
                        socket_sender
                            .send((response, addr))
                            .expect("[QuicServerHandler] send failed");
                    }
                } else if Self::is_quic_handshake_packet(&payload) {
                    // Do nothing. No response required for handshake packet from the client.
                    // TODO: not true. Found example of server responding to handshake with a
                    //       handshake in an Instagram capture.
                    //       Need to add support for control messages in the handshake.
                    println!("[QuicServerHandler] Received client handshake. Doing nothing.");
                } else {
                    println!("[QuicServerHandler] decoding client control message...");
                    // Try to decode client control message.
                    let control_message = ClientControlMessage::from_vec(&payload);
                    println!(
                        "[QuicServerHandler] client control message = {:?}",
                        control_message
                    );

                    let destination_connection_id =
                        get_destination_connection_id_from_packet(&payload);
                    if let Ok(control_message) = control_message {
                        for response in &control_message.responses {
                            let content_type = ContentType::from_u64(response.0);
                            let response_size = response.1 as usize;
                            match content_type {
                                Some(ContentType::QUICProtectedPayload) => {
                                    println!("[QuicServerHandler] [SEND] sending client controlled response");
                                    socket_sender
                                        .send((
                                            create_quic_protected_payload_packet(
                                                response_size,
                                                destination_connection_id.as_str(),
                                            ),
                                            addr,
                                        ))
                                        .expect("[QuicServerHandler] send failed")
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
            }
            debug!("thread exit");
        }));

        self.input_tx = Some(input_tx);
    }

    fn is_quic_initial_packet(payload: &Vec<u8>) -> bool {
        // TODO: this is NOT a valid assumption. Need to check RFC, I swore it mentioned 1200
        //       bytes, but Instagram captures show larger values.
        const QUIC_INITIAL_PACKET_SIZE: usize = 1200;
        if payload.len() != QUIC_INITIAL_PACKET_SIZE {
            return false;
        }

        // The first bit of the paylod should be 1 to indicate long header form.
        if payload[0] & 0b1000_0000 == 0 {
            return false;
        }

        // TODO: this is not accurate according to the RFC and real-world instagram traffic.
        // // The second bit of the payload should be 1 to indicate a fixed bit of 1.
        // if payload[0] & 0b0100_0000 == 0 {
        //     return false;
        // }

        // The packet type, stored in the 3rd and 4th bits, should be 0 to indicate an initial
        // packet.
        if payload[0] & 0b0011_0000 != 0 {
            return false;
        }

        // TODO: Looks like there are a _lot_ of versions: https://github.com/wireshark/wireshark/blob/master/epan/dissectors/packet-quic.c#L505
        //       In practice (Instagram, Facebook) I've only seen version 1 being used.
        const EXPECTED_QUIC_VERSION: u32 = 1;
        let version = u32::from_be_bytes(payload[1..5].try_into().unwrap());
        if version != EXPECTED_QUIC_VERSION {
            return false;
        }

        true
    }

    fn is_quic_handshake_packet(payload: &Vec<u8>) -> bool {
        // const QUIC_INITIAL_PACKET_SIZE: usize = 1200;
        // if payload.len() != QUIC_INITIAL_PACKET_SIZE {
        //     return false
        // }

        // The first bit of the paylod should be 1 to indicate long header form.
        let first_byte = payload[0];
        if first_byte & 0b1000_0000 == 0 {
            return false;
        }

        // TODO: apparently this is not true. Check the RFC and other sample captures.
        // // The second bit of the payload should be 1 to indicate a fixed bit of 1.
        // if first_byte & 0b0100_0000 == 0 {
        //     return false;
        // }

        // The third and fourth bits should be 01 to indicate a handshake packet type.
        if first_byte & 0b0010_0000 == 0 || first_byte & 0b0001_0000 != 0 {
            return false;
        }

        true
    }

    fn get_canned_response(payload: &Vec<u8>) -> Option<Vec<Vec<u8>>> {
        let payload_hex_string = hex::encode(payload);

        // Canned quic initial packet from client. Respond with handshake and protected payload.
        let known_payload_headers = vec![
            "02000000450004ccd4900000401100007f0000017f000001cb95270f04b802ccc100000001141dc1dac288e35aa1a5f00b5ade222f32ff4649d808fce581626316503600448a82",
            "ad0000000108f25b987a5d73ccdf08fce581626316503600",
        ];
        if known_payload_headers
            .iter()
            .any(|&x| payload_hex_string.starts_with(x))
        {
            // No need to respond to the client handshake packet. It will be followed by protected
            // payloads containing client control messages.
            return Some(vec![]);
        }

        None
    }
}
