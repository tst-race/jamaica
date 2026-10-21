// Copyright 2024 Two Six Technologies.

use serde::Deserialize;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::{fs::File, io::Read};

use motiflib::action::{Action, ReceiveAction, SendAction};
use motiflib::motif_input::MotifInput;
// use motiflib::packet::{Content, Packet};

use motiflib::cient_control_messages::{ClientControlMessage, ContentType};

use motiflib::quic::{
    create_quic_handshake_packet, create_quic_initial_packet, create_quic_protected_payload_packet,
    get_destination_connection_id_from_packet,
};

use motiflib::dtls;
use motiflib::rtp;

use motiflib::quic_client::QuicClient;

use motiflib::stun;

use log::debug;

// impl From<JsonPacket> for Packet {
//     fn from(json_packet: JsonPacket) -> Self {

//         println!("fn from(json_packet: JsonPacket) -> Self ");

//         //
//         // NEW SPEC: FOOTER
//         // Footer {
//         //     Num Responses (8), // TODO: also variable size? Would we want more than 256 packets in streaming case?
//         //     Content Type (i), // variable size integer.
//         //     Response Size (16), // Can probably make this smaller, but is it worth it?
//         //     (..) // Optional additional (type, size) tuples
//         // }
//         //
//         // Minimum size 4 bytes
//         // TODO: byte ordering? Given that this is a footer, we'll need to append it in reverse right?
//         // let payload: Vec<u8> = vec![];

//         // TODO: validate that response size and message size are less than system MTU.
//         // TODO: how to get system MTU?

//         // debug_assert!(json_packet.response_size < 2usize.pow(15));
//         let response_size: usize = 0; //json_packet.response_size as u16;

//         let mut payload = response_size.to_be_bytes().to_vec();

//         // TODO: implement content type. just random for now.

//         let content_type: u8 = 0;

//         payload.push(content_type);

//         debug_assert!(json_packet.payload_size <= std::u16::MAX as usize);

//         // Append the payload size to the header as two separate bytes.
//         let _ = (json_packet.payload_size as u16)
//             .to_be_bytes()
//             .iter()
//             .map(|byte| payload.push(*byte));

//         // Create the message and append.
//         // TODO: should message size include the header information? i.e. create payload_size - header_size bytes of message data???
//         // ^ YES, update this
//         let message: Vec<u8> = (0..json_packet.payload_size)
//             .map(|_| rand::random())
//             .collect();
//         payload.extend(message);

//         Self {
//             content: Content(payload),
//             response_timeout: json_packet.response_timeout,
//         }
//     }
// }

#[derive(Debug, Deserialize)]
struct Response {
    size: usize,
    #[serde(rename = "type")]
    content_type: ContentType,
}

/// The json representation of a packet.
#[derive(Debug, Deserialize)]
struct JsonPacket {
    // /// Payload size to request from the server to send in response.
    // pub response_size: usize,
    // pub response_content_type: ContentType,
    #[serde(default)]
    pub responses: Vec<Response>,
    /// The entire size of the payload (in bytes) sent to the server including all header
    /// information.
    pub payload_size: usize,
    pub content_type: ContentType,
    /// Amount of time to wait for a response before continuing.
    // #[serde(default)]
    pub response_timeout: i32, // negative value indicates not to wait. TODO: negative value doesn't work with mpsc::receiver API. Just set to zero to indicate no wait?
}

impl Default for JsonPacket {
    fn default() -> Self {
        Self {
            responses: vec![],
            payload_size: 0,
            content_type: ContentType::from_u64(0).unwrap(),
            response_timeout: -1,
        }
    }
}

// TODO: has to be a better, more direct way than having intermediate json objects for deserialization
#[derive(Debug, Deserialize)]
struct ConversationJson {
    protocol: String, // TODO: change to enum
    #[serde(default)]
    #[allow(unused)]
    blocking: bool,
    packets: Vec<JsonPacket>,
}

impl Default for ConversationJson {
    fn default() -> Self {
        Self {
            protocol: "".to_string(),
            blocking: true,
            packets: vec![],
        }
    }
}

// TODO: has to be a better, more direct way than having intermediate json objects for deserialization
#[derive(Debug, Deserialize)]
struct ClientControlJson {
    // pub packets: Vec<JsonPacket>,
    pub conversations: Vec<ConversationJson>,
}

pub struct Conversation {
    pub protocol: String,
    pub actions: Vec<Action>,
}

pub struct ClientControlInput {
    pub conversations: Vec<Conversation>,
    // actions: VecDeque<Action>,
}

// impl ClientControlInput {
//     pub fn new(file_name: &str) -> Self {
//         let mut file = match File::open(file_name) {
//             Ok(result) => result,
//             Err(error) => panic!("Failed to open {}: {:?}", file_name, error),
//         };

//         let mut json_str = String::new();
//         match file.read_to_string(&mut json_str) {
//             Ok(result) => result,
//             Err(err) => panic!("Error reading file '{:?}': {:?}", file_name, err),
//         };

//         let json: ClientControlJson = match serde_json::from_str(&json_str) {
//             Ok(result) => result,
//             Err(err) => panic!("Error parsing json: {:?}", err),
//         };

//         let mut conversations: Vec<Conversation> = vec![];

//         for conversation in json.conversations {
//             let mut actions: VecDeque<Action> = vec![].into();

//             let quic_client = Arc::new(Mutex::new(QuicClient::new()));

//             for packet in &conversation.packets {
//                 let control_message = ClientControlMessage {
//                     responses: packet
//                         .responses
//                         .iter()
//                         .map(|x| (x.content_type as u64, x.size as u16))
//                         .collect(),
//                 };

//                 let quic_client = Arc::clone(&quic_client);

//                 match packet.content_type {
//                     // ContentType::QUICInitial => actions.push_back(create_quic_initial_packet().into()),
//                     // ContentType::QUICHandshake => actions.push_back(
//                     //     create_quic_handshake_packet("f25b987a5d73ccdf", "fce5816263165036", 1).into(),
//                     // ),
//                     // ContentType::QUICProtectedPayload => actions.push_back(
//                     //     Self::embed_control_message(
//                     //         create_quic_protected_payload_packet(
//                     //             packet.payload_size,
//                     //             "f25b987a5d73ccdf",
//                     //         )
//                     //         .into(),
//                     //         control_message.to_vec(),
//                     //     )
//                     //     .into(),
//                     // ),
//                     ContentType::Random => {
//                         todo!("implement me!");
//                     }
//                     ContentType::QUICInitial => {
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 let quic_client = quic_client.lock().unwrap();
//                                 create_quic_initial_packet(
//                                     quic_client.source_connection_id.as_str(),
//                                 )
//                             },
//                         ))));
//                     }
//                     ContentType::QUICHandshake => {
//                         actions.push_back(Action::Send(SendAction::new(
//                             Rc::new(move |received_payloads| {
//                                 let mut quic_client = quic_client.lock().unwrap();
//                                 debug_assert!(!received_payloads.is_empty(),
//                                     "Should have received a payload by now. This does not make sense.");
//                                 quic_client.destination_connection_id =
//                                     get_destination_connection_id_from_packet(
//                                         &received_payloads[0],
//                                     );
//                                 debug!(
//                                     "Quic Handshake action DCID = {:?}",
//                                     quic_client.destination_connection_id
//                                 );
//                                 const PACKET_NUMBER: u8 = 1;
//                                 create_quic_handshake_packet(
//                                     quic_client.destination_connection_id.as_str(),
//                                     "fce5816263165036",
//                                     PACKET_NUMBER,
//                                 )
//                             }),
//                         )));
//                     }
//                     ContentType::QUICProtectedPayload => {
//                         let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 Self::embed_control_message(
//                                     create_quic_protected_payload_packet(
//                                         payload_size,
//                                         "f25b987a5d73ccdf",
//                                     )
//                                     .into(),
//                                     &control_message.to_vec(),
//                                 ).unwrap()
//                             },
//                         ))));
//                     }
//                     ContentType::RTP => {
//                         let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 let rtp_header = rtp::Header {
//                                     sequence_number: rand::random(),
//                                     timestamp: rand::random(), // TODO: use a realish value.
//                                     synchronization_source_identifier: rand::random(),
//                                     ..Default::default()
//                                 };
//                                 let mut payload = rtp_header.to_vec();
//                                 let payload_size = payload_size - payload.len();
//                                 let random_bytes: Vec<u8> =
//                                     (0..payload_size).map(|_| rand::random()).collect();
//                                 payload.extend(random_bytes);

//                                 Self::embed_control_message(payload, &control_message.to_vec()).unwrap()
//                             },
//                         ))));
//                     }
//                     ContentType::RTCP => {
//                         let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 debug!("Sending RTCP!");
//                                 (0..payload_size).map(|_| rand::random()).collect()
//                             },
//                         ))));
//                     }
//                     ContentType::DTLSv1_2ClientHello => {
//                         let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 debug!("Sending DTLS client hello!");

//                                 // let record_layer = dtls::RecordLayer {
//                                 //     content_type: 22,
//                                 //     version: 0xfeff,
//                                 //     length: payload_size as u16,
//                                 //     ..Default::default()
//                                 // };

//                                 // let mut payload = record_layer.to_vec();
//                                 // let payload_size = payload_size - payload.len();
//                                 // let mut random_bytes: Vec<u8> = (0..payload_size)
//                                 //     .map(|_| rand::random())
//                                 //     .collect();
//                                 // random_bytes[0] = 1;
//                                 // random_bytes.splice(12..13, 0xfefd_u16.to_be_bytes());
//                                 // payload.extend(random_bytes);

//                                 let payload = dtls::create_client_hello(payload_size);

//                                 Self::embed_control_message(payload, &control_message.to_vec()).unwrap()
//                             },
//                         ))));
//                     }
//                     ContentType::DTLSv1_2ApplicationData => {
//                         let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 let payload = dtls::create_application_data(payload_size);

//                                 Self::embed_control_message(payload, &control_message.to_vec()).unwrap()
//                             },
//                         ))));
//                     }
//                     ContentType::STUN => {
//                         // let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 let stun = stun::StunPacket::create_binding_request();
//                                 // let stun = hex::decode("0074000100602112a44253714077ca2d4b8bd9923b61000600232d4d5159477558763871365869676f4b4141694b4169414245413a37666665323961300000250000002400046d7f80ff802a0008f94c0d9651116027000800149222f013f5b533b5e1c9e83663de7da4eb2606d780280004a9c6d34e").expect("Decoding failed");
//                                 debug!("stun packet length: {}", stun.len());
//                                 stun
//                             },
//                         ))));
//                         // let extension_length = extension_length * 4; // Length is number of 32-byte words.
//                         // let random_bytes: Vec<u8> = (0..extension_length).map(|_| rand::random()).collect();
//                         // header.extend(random_bytes);
//                     }
//                     ContentType::STUNBindingRequest => {
//                         // TODO: how to use size here? randomly pad? probably needs to be more intelligent
//                         // let payload_size = packet.payload_size;
//                         actions.push_back(Action::Send(SendAction::new(Rc::new(
//                             move |_received_payloads| {
//                                 let stun = stun::StunPacket::create_binding_request();
//                                 stun
//                             },
//                         ))));
//                     }
//                     _ => {
//                         todo!(
//                             "[REMOVE THIS CATCH ALL]unsupported content type: {:?}",
//                             packet.content_type
//                         );
//                     }
//                 }

//                 for response in &packet.responses {
//                     // const TIMEOUT: i32 = 5;
//                     match response.content_type {
//                         _ => actions.push_back(Action::Receive(ReceiveAction::new(
//                             packet.response_timeout,
//                         ))),
//                     }
//                 }
//             }

//             conversations.push(Conversation {
//                 protocol: conversation.protocol,
//                 actions: actions.into(),
//             });
//         }

//         Self {
//             conversations,
//             // actions: VecDeque::from(vec![]),
//         }
//     }

//     ///
//     /// Replaces the end of payload with the bytes in the control message.
//     ///
//     pub fn embed_control_message(mut payload: Vec<u8>, control_message: &Vec<u8>) -> Result<Vec<u8>, String> {
//         if payload.len() < control_message.len() {
//             return Err(format!(
//                 "payload of size {} is too small to hold control message of size {}",
//                 payload.len(),
//                 control_message.len()
//             ));
//         }
//         assert!(
//             payload.len() >= control_message.len(),
//             "payload of size {} is too small to hold control message of size {}",
//             payload.len(),
//             control_message.len()
//         );
//         let new_size = payload.len() - control_message.len();
//         payload.truncate(new_size);
//         payload.extend(control_message);
//         Ok(payload)
//     }
// }

// impl MotifInput for ClientControlInput {
//     fn get_next_action(&mut self) -> Action {
//         match self.actions.pop_front() {
//             Some(action) => action,
//             None => Action::Quit,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn embed_control_message_does_not_change_payload_size() {
    //     // Payload bigger than control message.
    //     let payload = vec![99; 256];
    //     let control_message = vec![255; 16];
    //     let old_payload_len = payload.len();
    //     let new_payload = ClientControlInput::embed_control_message(payload, &control_message).unwrap();
    //     assert_eq!(old_payload_len, new_payload.len());

    //     // Control message is one byte smaller than the payload.
    //     let payload = vec![99; 256];
    //     let control_message = vec![255; 255];
    //     let old_payload_len = payload.len();
    //     let new_payload = ClientControlInput::embed_control_message(payload, &control_message).unwrap();
    //     assert_eq!(old_payload_len, new_payload.len());

    //     // Payload and control message are the same size.
    //     let payload = vec![99; 256];
    //     let control_message = vec![255; 256];
    //     let old_payload_len = payload.len();
    //     let new_payload = ClientControlInput::embed_control_message(payload, &control_message).unwrap();
    //     assert_eq!(old_payload_len, new_payload.len());

    //     // Empty vectors.
    //     let payload = vec![];
    //     let control_message = vec![];
    //     let old_payload_len = payload.len();
    //     let new_payload = ClientControlInput::embed_control_message(payload, &control_message).unwrap();
    //     assert_eq!(old_payload_len, new_payload.len());
    // }

    // #[test]
    // #[should_panic]
    // fn embed_control_message_control_message_larger_than_payload_panics() {
    //     let payload = vec![99; 256];
    //     let control_message = vec![255; 512];
    //     let _ = ClientControlInput::embed_control_message(payload, &control_message).unwrap();
    // }
}
