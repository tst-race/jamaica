// Copyright 2024 Two Six Technologies.

use core::num;
use csv::Writer;
use log::Level::Trace;
use log::{debug, error, info, log_enabled, trace, warn};
use motiflib::action::{Action, ReceiveAction, SendAction};
use motiflib::packet;
use remora_lib::generic_parser::OutputFormat;
use serde::{Deserialize, Serialize};
use std::clone;
use std::collections::hash_map::DefaultHasher;
use std::fmt::format;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::{fs::File, io::Read};

use crate::packet_gen::{self, *};

// code_gen::motif_code_generation!();

use paste::paste;

use crate::client_control_input::ClientControlInput;

use motiflib::remora_control_message::{
    can_embed_control_message_in_field, generate_packet, ClientPacket, ClientPacketBuilder,
    ConversationJson, FieldJson, PacketJson,
};

// ///
// /// Stripped down version of `Field` in Remora src/generic_parser.rs
// ///
// /// TODO: make this an enum to differentiate between raw and known fields?
// #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
// pub struct FieldJson {
//     /// The name of the field.
//     // pub name: String,
//     /// The tshark display value. Should only be used for debugging.
//     // #[serde(skip_serializing_if = "Option::is_none")]
//     // display: Option<String>,
//     /// The index (position) in bits of the field in the layer.
//     // pub index: usize,
//     /// The length of the field in bits.
//     ///
//     ///
//     ///
//     ///
//     /// A hex string of the field value.
//     pub raw: Option<String>,
//     /// The name of a field if it's known, otherwise None.
//     pub field: Option<String>,
//     ///
//     pub size: Option<usize>,
// }

// ///
// /// Stripped down version of `Packet` in Remora src/generic_parser.rs
// ///
// #[derive(Deserialize, Clone, Debug, PartialEq)]
// pub struct PacketJson {
//     pub direction: String, // TODO: change to enum? YES
//     pub fields: Vec<FieldJson>,
// }

// impl ConversationJson {
//     fn new(protocol: String) -> Self {
//         Self {
//             protocol,
//             packets: vec![],
//         }
//     }
// }

#[derive(Debug)]
pub struct Conversation {
    pub protocol: String,
    pub actions: Vec<Action>,
}

impl Conversation {
    fn new(protocol: String) -> Self {
        Self {
            protocol,
            actions: vec![],
        }
    }
}

// /// A client packet with a list of server responses.
// #[derive(Debug)]
// struct ClientPacket {
//     /// The client packet content to be sent to the server.
//     fields: Vec<FieldJson>,
//     /// A list of list of fields, i.e. a list of packets, with which the server should respond.
//     responses: Vec<Vec<FieldJson>>,
// }

// impl ClientPacket {
//     fn parse_packets(packets: &Vec<PacketJson>) -> Vec<Self> {
//         let mut client_packets = vec![];
//         let mut packet_index = 0;
//         while packet_index < packets.len() {
//             let current_packet = &packets[packet_index];
//             debug_assert_eq!(current_packet.direction, "from_src");

//             let mut responses = vec![];
//             // TODO: man this is ugly as hell.
//             packet_index += 1;
//             while packet_index < packets.len() {
//                 let next_packet = &packets[packet_index];
//                 if next_packet.direction != "from_dst" {
//                     break;
//                 }
//                 responses.push(next_packet.fields.clone());
//                 packet_index += 1;
//             }
//             client_packets.push(ClientPacket {
//                 fields: current_packet.fields.clone(),
//                 responses,
//             });
//         }
//         client_packets
//     }
// }

#[derive(Serialize, Default)]
struct BandwidthCsvRecord {
    session_index: usize,
    packet_index: usize,
    packet_size: usize,
    packet_bandwidth: usize,
    control_message_size: usize,
}

///
/// Client input. Contains a list of conversations read from an input JSON file.
///
/// TODO: change name. Maybe GruntInput or just MotifInput, MotifJsonClientInput.
///
pub struct RemoraInput {
    pub conversations: Vec<Conversation>,
    // actions: VecDeque<Action>,
}

impl RemoraInput {
    pub fn new(file_name: &str, strict: bool) -> Self {
        let mut file = match File::open(file_name) {
            Ok(result) => result,
            Err(error) => panic!("Failed to open {}: {:?}", file_name, error),
        };

        let mut json_str = String::new();
        match file.read_to_string(&mut json_str) {
            Ok(result) => result,
            Err(err) => panic!("Error reading file '{:?}': {:?}", file_name, err),
        };

        // let remora_sessions: Vec<OutputFormat> = match serde_json::from_str(&json_str) {
        let input_json: Vec<ConversationJson> = match serde_json::from_str(&json_str) {
            Ok(result) => result,
            Err(err) => panic!("Error parsing json: {:?}", err),
        };

        // let mut bandwidth_csv = match Writer::from_path("client_bandwidth.csv") {
        //     Ok(writer) => writer,
        //     Err(err) => {
        //         let err_msg = format!("failed to create CSV writer: {}", err);
        //         error!("{}", err_msg);
        //         panic!("{}", err_msg);
        //     }
        // };

        let mut conversations = vec![];
        for (session_index, session) in input_json.iter().enumerate() {
            let mut current_session = Conversation::new(session.protocol.to_string());

            // A list of the packets that the client will send.
            let client_packets = ClientPacket::parse_packets(&session.packets.clone());
            // trace!("{:?}", client_packets);

            let mut packet_builder = ClientPacketBuilder::new(&session);
            packet_builder.build();

            for packet in &packet_builder.packets {
                // println!("{:?}", packet);

                let payload = packet.clone();
                let num_responses = 0; // TODO: have ClientPacketBuilder return this info
                current_session
                    .actions
                    .push(Action::Send(SendAction::new(Rc::new(
                        move |_received_payloads| {
                            let log_message = format!("creating payload of size {}", payload.len());
                            if log_enabled!(Trace) {
                                let mut hasher = DefaultHasher::new();
                                hasher.write(&payload.as_slice());
                                trace!("{} with hash {}", log_message, hasher.finish());
                            } else {
                                info!("{}", log_message);
                            }

                            // Return the payload to be sent to the server.
                            payload.clone()
                        },
                    ))));

                current_session.actions.append(&mut vec![
                    Action::Receive(ReceiveAction::new(5));
                    num_responses
                ]);
            }
            current_session.actions.push(Action::Quit);

            // for (packet_index, packet) in client_packets.into_iter().enumerate() {
            //     // Get the length before this gets moved inside the closure.
            //     let num_responses = packet.responses.len();

            //     trace!("generating packet from fields: {:?}", packet.fields);
            //     let payload = generate_packet(&packet.fields, &packet.responses, packet_index);

            //     // let num_embeddable_bytes: usize = match packet.fields.last() {
            //     //     Some(field) => match field.field.as_ref() {
            //     //         Some(field_name) => {
            //     //             if can_embed_control_message_in_field(field_name.as_str()) {
            //     //                 field.arg.unwrap_or_else(|| panic!("invalid motif input. Explicit field should have an `arg` field")) as usize / 8
            //     //             } else {
            //     //                 0
            //     //             }
            //     //         }
            //     //         None => 0,
            //     //     },
            //     //     None => 0,
            //     // };
            //     // info!(
            //     //     "BANDWIDTH: packet index {} num embeddable bytes: {}",
            //     //     packet_index, num_embeddable_bytes
            //     // );

            //     // // Create the client control message.
            //     // let control_message = generate_remora_control_message(&packet.responses);

            //     // // Save bandwidth information to a CSV file.
            //     // match bandwidth_csv.serialize(BandwidthCsvRecord {
            //     //     session_index,
            //     //     packet_index,
            //     //     packet_size: payload.len(),
            //     //     packet_bandwidth: num_embeddable_bytes,
            //     //     control_message_size: control_message.len(),
            //     // }) {
            //     //     Ok(_) => (),
            //     //     Err(_) => error!(
            //     //         "failed to write CSV entry for packet index {}",
            //     //         packet_index
            //     //     ),
            //     // };

            //     // Embed the client control message in the payload.
            //     // TODO: if the control message doesn't fit, do we fail hard or just
            //     // send a larger message??????
            //     // let payload = match ClientControlInput::embed_control_message(
            //     //     payload.clone(),
            //     //     &control_message,
            //     // ) {
            //     //     Ok(payload) => payload,
            //     //     Err(err) => {
            //     //         let err_msg = format!(
            //     //             "Failed to embed payload for session {} packet {}: {}",
            //     //             session_index, packet_index, err
            //     //         );
            //     //         error!("{}", err_msg);
            //     //         if strict {
            //     //             panic!("{}", err_msg);
            //     //         }
            //     //         warn!("sending control message anyway");
            //     //         const MTU: usize = 1500;
            //     //         control_message.clone()
            //     //     }
            //     // };

            //     current_session
            //         .actions
            //         .push(Action::Send(SendAction::new(Rc::new(
            //             move |_received_payloads| {
            //                 let log_message = format!("creating payload of size {}", payload.len());
            //                 if log_enabled!(Trace) {
            //                     let mut hasher = DefaultHasher::new();
            //                     hasher.write(&payload.as_slice());
            //                     trace!("{} with hash {}", log_message, hasher.finish());
            //                 } else {
            //                     info!("{}", log_message);
            //                 }

            //                 // Return the payload to be sent to the server.
            //                 payload.clone()
            //             },
            //         ))));

            //     current_session.actions.append(&mut vec![
            //         Action::Receive(ReceiveAction::new(5));
            //         num_responses
            //     ]);
            // }

            conversations.push(current_session);
        }

        trace!("{:?}", conversations);

        Self { conversations }
    }

    ///
    /// Replaces the embeddable field(s) of payload with the bytes in the control message.
    ///
    fn embed_control_message(
        mut payload: Vec<u8>,
        control_message: &Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        if payload.len() < control_message.len() {
            return Err(format!(
                "payload of size {} is too small to hold control message of size {}",
                payload.len(),
                control_message.len()
            ));
        }
        assert!(
            payload.len() >= control_message.len(),
            "payload of size {} is too small to hold control message of size {}",
            payload.len(),
            control_message.len()
        );
        let new_size = payload.len() - control_message.len();
        payload.truncate(new_size);
        payload.extend(control_message);
        Ok(payload)
    }
}
