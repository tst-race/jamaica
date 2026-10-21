// Copyright 2024 Two Six Technologies.

use log::Level::Trace;
use log::{debug, error, info, log_enabled, trace, warn};
// use motiflib::action::{Action, ReceiveAction, SendAction};
// use motiflib::packet;
use remora_lib::generic_parser::OutputFormat;
use rmp_serde::{Deserializer, Serializer};
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::os::macos::raw;
use std::rc::Rc;
use std::{fs::File, io::Read};

// use crate::packet_gen::{self, *};

// use crate::client_control_input::ClientControlInput;

use std::sync::atomic::{AtomicU16, Ordering};
code_gen::motif_code_generation!();

// Protobuf things.
mod client_control_message {
    include!(concat!(env!("OUT_DIR"), "/client_control_message.rs"));
}
use client_control_message::{
    ClientControlMessageEmbeddedDataLocations, ClientControlMessageField, ClientControlMessageList,
    ClientControlMessagePacket, Location,
};
use prost::Message;

use crate::packet;

// impl Default for ClientControlMessageEmbeddedDataLocations {
//     fn default() -> Self {
//         Self {
//             packet_index: 0,
//             data_index: 0,
//         }
//     }
// }

impl From<ServerPacket> for ClientControlMessagePacket {
    fn from(server_packet: ServerPacket) -> Self {
        let mut fields = vec![];
        for field in server_packet.fields {
            let proto_field = ClientControlMessageField {
                raw: field.raw.clone(),
                field: field.field.clone(),
                arg: field.arg,
            };
            fields.push(proto_field);
        }

        Self {
            fields,
            response_index: server_packet.response_index as u64,
        }
    }
}

/// Footer {
///     Num Responses (8),      // TODO: also variable size? Would we want more than 256 packets in streaming case?
///     Num Fields,             // The number of fields in this response
///     Field (i),                  // See Field def below
///     (Num Fields, Fields)    // Optional additional Fields
/// }
///
///
/// Field {
///     Field Type (i),     // variable size string from a known set of types.
///     Content Size (i),   // variable length int giving size of content to follow
///     Content (i),        // varies depending on content type
/// }

///
/// Stripped down version of `Field` in Remora src/generic_parser.rs
///
/// TODO: make this an enum to differentiate between raw and known fields?
/// TODO: should probably be defined by code-gen since it is using it now.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FieldJson {
    /// The name of the field.
    // pub name: String,
    /// The tshark display value. Should only be used for debugging.
    // #[serde(skip_serializing_if = "Option::is_none")]
    // display: Option<String>,
    /// The index (position) in bits of the field in the layer.
    // pub index: usize,
    /// The length of the field in bits.
    ///
    ///
    ///
    ///
    /// A hex string of the field value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// The name of a field if it's known, otherwise None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<u64>,
}

impl From<ClientControlMessageField> for FieldJson {
    fn from(control_message: ClientControlMessageField) -> Self {
        Self {
            raw: control_message.raw,
            field: control_message.field,
            arg: control_message.arg,
        }
    }
}

///
/// Stripped down version of `Packet` in Remora src/generic_parser.rs
///
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct PacketJson {
    /// The direction the packet is sent, either from_src (client to server) or from_dst (server to
    /// client).
    pub direction: String, // TODO: change to enum? YES
    /// The individual fields that make up the packet.
    pub fields: Vec<FieldJson>,
}

///
/// Stripped down version of `OutputFormat` in Remora src/generic_parser.rs
///
#[derive(Deserialize, Debug, PartialEq)]
pub struct ConversationJson {
    // pub src_addr: SocketAddr,
    // pub dst_addr: SocketAddr,
    pub protocol: String, // Protocol, TODO: use enum
    // pub connection_id: usize,
    pub packets: Vec<PacketJson>,
}

/// A client packet with a list of server responses.
#[derive(Debug)]
pub struct ClientPacket {
    /// The client packet content to be sent to the server.
    pub fields: Vec<FieldJson>,
    /// A list of list of fields, i.e. a list of packets, with which the server should respond.
    pub responses: Vec<Vec<FieldJson>>,
    // TODO: include embdeabble field locations
    // pub embeddable_field_locations: Vec<(u16, u16)>,
}

impl ClientPacket {
    pub fn parse_packets(packets: &Vec<PacketJson>) -> Vec<Self> {
        let mut client_packets = vec![];
        let mut packet_index = 0;
        while packet_index < packets.len() {
            let current_packet = &packets[packet_index];
            debug_assert_eq!(current_packet.direction, "from_src");

            let mut responses = vec![];
            // TODO: man this is ugly as hell.
            packet_index += 1;
            while packet_index < packets.len() {
                let next_packet = &packets[packet_index];
                if next_packet.direction != "from_dst" {
                    break;
                }
                responses.push(next_packet.fields.clone());
                packet_index += 1;
            }
            client_packets.push(ClientPacket {
                fields: current_packet.fields.clone(),
                responses,
            });
        }
        client_packets
    }
}

#[derive(Clone, PartialEq, Debug)]
struct ServerPacket {
    fields: Vec<FieldJson>,
    /// The point in the conversation when this packet should be sent. For example,
    /// a response index of 4 means the server should respond with this packet
    /// after it receives 5 packets from the client.
    response_index: u64,
}

impl From<ClientControlMessagePacket> for ServerPacket {
    fn from(control_message: ClientControlMessagePacket) -> Self {
        let fields = control_message
            .fields
            .into_iter()
            .map(|field| FieldJson {
                raw: field.raw,
                field: field.field,
                arg: field.arg,
            })
            .collect::<Vec<_>>();

        Self {
            fields,
            response_index: control_message.response_index,
        }
    }
}

///
/// Struct to build a list of client packets to send. Responsible for embedding client control
/// messages into the packets in the appropriate locations, as well as structuring those messages
/// to include information for the server on where to find embedded data in future client packets.
///
pub struct ClientPacketBuilder {
    client_packets: Vec<Vec<FieldJson>>,
    ///
    /// A flattened list of all the server packets regardless of the client packet to which they
    /// respond.
    ///
    server_packets: Vec<ServerPacket>,
    // packets: Vec<ClientPacket>,
    ///
    /// List of raw packets to send over the socket.
    ///
    pub packets: Vec<Vec<u8>>,
}

impl ClientPacketBuilder {
    pub fn new(conversation: &ConversationJson) -> Self {
        let (client_packets, server_packets) = Self::parse_packets(conversation);
        Self {
            client_packets,
            server_packets,
            packets: vec![],
        }
    }

    ///
    /// Get a list of all the server packets and their response index. These will be used to
    /// generate client control messages and need to be known in advance so that they can
    /// potentially be sent ahead of time when available bandwidth is high.
    ///
    fn parse_packets(conversation: &ConversationJson) -> (Vec<Vec<FieldJson>>, Vec<ServerPacket>) {
        debug_assert!(conversation.packets.first().unwrap().direction == "from_src");

        let mut client_packets = vec![];
        let mut server_packets = vec![];

        for packet in &conversation.packets {
            if packet.direction == "from_src" {
                client_packets.push(packet.fields.clone());
            } else if packet.direction == "from_dst" {
                server_packets.push(ServerPacket {
                    fields: packet.fields.clone(),
                    response_index: client_packets.len() as u64 - 1,
                });
            } else {
                panic!("invalid packet direction: {}", packet.direction);
            }
        }

        (client_packets, server_packets)
    }

    pub fn build(&mut self) {
        info!(
            "ClientPacketBuilder num server packets {}",
            self.server_packets.len()
        );
        info!(
            "ClientPacketBuilder num client packets {}",
            self.client_packets.len()
        );

        let mut raw_client_packets: Vec<Vec<u8>> = vec![];
        // let mut message_header_list: VecDeque<ClientControlMessageHeader> = VecDeque::new();
        // let mut control_message_list: VecDeque<Vec<u8>> = VecDeque::new();
        let mut embeddable_locations_per_packet: HashMap<usize, Vec<(u16, u16)>> = HashMap::new();
        let mut embeddable_locations_queue = VecDeque::new();

        // Build the list of raw client packets and client control message headers.
        for (packet_index, packet) in self.client_packets.iter().enumerate() {
            let (raw_packet, embeddable_locations) = generate_raw_client_packet(packet);
            raw_client_packets.push(raw_packet);

            for location in &embeddable_locations {
                // message_header_list.push_back(ClientControlMessageHeader {
                //     locations: vec![ClientControlMessageEmbeddedDataLocations {
                //         packet_index: packet_index as u64,
                //         locations: vec![Location {
                //             index: location.0 as u32,
                //             length: location.1 as u32,
                //         }],
                //     }]
                // });

                embeddable_locations_queue.push_back((packet_index, location.0, location.1));
            }

            if embeddable_locations.len() > 0 {
                embeddable_locations_per_packet.insert(packet_index, embeddable_locations);
            }
        }

        trace!(
            "ClientPacketBuilder num raw client packets {}",
            raw_client_packets.len()
        );
        trace!(
            "ClientPacketBuilder embed locations {:#?}",
            embeddable_locations_per_packet
        );

        let mut server_packet_queue = VecDeque::<ServerPacket>::from(self.server_packets.clone());

        // let mut embeddable_locations_
        for (packet_index, packet) in raw_client_packets.iter().enumerate() {
            let mut packet = packet.clone();

            // trace!("{} embeddable_locations_queue = {:?}", packet_index, embeddable_locations_queue);
            let mut locations = vec![];
            loop {
                match embeddable_locations_queue.pop_front() {
                    Some(location) => {
                        locations.push(ClientControlMessageEmbeddedDataLocations {
                            packet_index: location.0 as u64,
                            data_index: location.1 as u32,
                        });
                        // TODO: might be multiple locations per packet_index. Need to account for this.
                        if location.0 > packet_index {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // debug_assert!(locations.last().unwrap().packet_index as usize > packet_index, "{} not greater than {}", locations.last().unwrap().packet_index, packet_index);

            // let location = ClientControlMessageEmbeddedDataLocations { packet_index: index as u64, data_index};
            // Convert FieldJson to ClientControlMessageProtobuf.
            let mut proto_client_control_message = ClientControlMessageList {
                // locations: vec![ClientControlMessageEmbeddedDataLocations {
                //     packet_index: client_packet_index as u64,
                //     data_index: 0,
                // }],
                locations,
                packets: vec![],
            };

            // Get the location of embeddable bytes for the current packet in this loop.
            // TODO: support non-contiguous embeddable bytes, right now just grabbing the first.
            if let Some(embeddable_locations) = embeddable_locations_per_packet.get(&packet_index) {
                debug_assert!(!embeddable_locations.is_empty());
                let (embed_index, embed_length) = embeddable_locations.first().unwrap();
                let (embed_index, embed_length) = (*embed_index as usize, *embed_length as usize);
                // debug_assert!(embed_length >= 2, "invalid embed length for packet at index {}", packet_index);
                // TODO: what should this threshold be? Can we know the minimum size of the protobuf?
                if embed_length > 5 {
                    let mut encoded_protobuf_bytes = Vec::new();
                    proto_client_control_message
                        .encode(&mut encoded_protobuf_bytes)
                        .unwrap();
                    // The header takes up 2 bytes, so the protobuf can only take up what's left.
                    let protobuf_embeddable_length = embed_length - 2;
                    if server_packet_queue.len() > 0 {
                        let server_packet = server_packet_queue.pop_front().unwrap();

                        // let mut proto_packet = ClientControlMessagePacket {
                        //     fields: vec![],
                        //     response_index: server_packet.response_index as u64,
                        // };
                        // for field in server_packet.fields {
                        //     let proto_field = ClientControlMessageField {
                        //         raw: field.raw.clone().unwrap_or_default(),
                        //         field: field.field.clone().unwrap_or_default(),
                        //         arg: field.arg.unwrap_or_else(|| 0),
                        //     };
                        //     proto_packet.fields.push(proto_field);
                        // }

                        proto_client_control_message
                            .packets
                            .push(server_packet.into());

                        proto_client_control_message
                            .encode(&mut encoded_protobuf_bytes)
                            .unwrap();

                        // Add more server packets until it's too many, then take one out.
                        while encoded_protobuf_bytes.len() < protobuf_embeddable_length
                            && server_packet_queue.len() > 0
                        {
                            let server_packet = server_packet_queue.pop_front().unwrap();
                            proto_client_control_message
                                .packets
                                .push(server_packet.into());
                            proto_client_control_message
                                .encode(&mut encoded_protobuf_bytes)
                                .unwrap();
                            dbg!(proto_client_control_message.packets.len());
                            dbg!(encoded_protobuf_bytes.len());
                        }

                        // TODO: this is ugly as hell.
                        if encoded_protobuf_bytes.len() > protobuf_embeddable_length {
                            dbg!(encoded_protobuf_bytes.len());
                            server_packet_queue.push_front(
                                proto_client_control_message.packets.pop().unwrap().into(),
                            );
                            encoded_protobuf_bytes.clear();
                            proto_client_control_message
                                .encode(&mut encoded_protobuf_bytes)
                                .unwrap();
                        }

                        dbg!(proto_client_control_message.packets.len());
                        dbg!(encoded_protobuf_bytes.len());
                        dbg!(protobuf_embeddable_length);
                    }

                    trace!(
                        "packet {} embedded {} locations and {} control messages",
                        packet_index,
                        proto_client_control_message.locations.len(),
                        proto_client_control_message.packets.len()
                    );

                    let header = encoded_protobuf_bytes.len() as u16;
                    trace!("{} header value = {}", packet_index, header);
                    let header = header.to_be_bytes();

                    // debug_assert!(embed_length >= 2);
                    packet[embed_index..embed_index + 2].copy_from_slice(&header);

                    packet[embed_index + 2..embed_index + 2 + encoded_protobuf_bytes.len()]
                        .copy_from_slice(&encoded_protobuf_bytes);
                } else {
                    info!("Not enough bytes to embed anything: {}", embed_length);
                }
            } else {
                info!("packet at index {} has no embeddable bytes", packet_index)
            }

            self.packets.push(packet);
        }

        // Build the list of control messages.
        // for packet in &self.server_packets {
        //     control_message_list.push_back(
        //         encode_server_packet_as_protobuf(&vec![packet.fields.clone()], packet.response_index as usize)
        //     );
        // }

        // for index in 1..self.server_packets.len() {
        //     let packet = self.server_packets.get(index).unwrap();
        //     let data_index = embeddable_locations_per_packet.get(&index).unwrap_or(&vec![(0u16,0u16)]).first().unwrap().0 as u32;
        //     control_message_list.push_back(
        //         encode_server_packet_as_protobuf(&vec![packet.fields.clone()], packet.response_index as usize, ClientControlMessageEmbeddedDataLocations { packet_index: index as u64, data_index})
        //     );
        // }

        // // Embed the control headers and messages into the client packets.
        // for (packet_index, packet) in raw_client_packets.iter().enumerate() {
        //     println!("packet_index = {}", packet_index);
        //     let message_header_bytes = match control_message_list.pop_front() {
        //         Some(header) => {
        //             let mut buf = Vec::new();
        //             header.encode(&mut buf).unwrap();
        //             buf
        //         }
        //         None => {
        //             vec![]
        //         }
        //     };

        //     let (embed_index, embed_length) = embeddable_locations_per_packet.get(&packet_index).unwrap().first().unwrap();
        //     let embed_index = *embed_index as usize;
        //     let embed_length = *embed_length as usize;
        //     let header_value = message_header_bytes.len() as u16;
        //     let header = header_value.to_be_bytes();

        //     println!("header_value = {}", header_value);

        //     let mut packet = packet.clone();
        //     // debug_assert!(embed_length >= 2);
        //     packet[embed_index..embed_index+2].copy_from_slice(&header);
        //     debug_assert!(embed_length >= 2+message_header_bytes.len(), "embeddable size is {} but need to embed {}", embed_length, message_header_bytes.len());
        //     packet[embed_index+2..embed_index+2+message_header_bytes.len()].copy_from_slice(&message_header_bytes);
        //     // let mut num_messages_embedded = 0;
        //     // let embeddable_end_index = embed_index + embed_length;
        //     // let mut embeddable_start_pointer = embed_index+2+message_header_bytes.len();
        //     // while control_message_list.len() > 0 {
        //     //     // Check if the next message in the queue will fit in the packet's embeddable data.
        //     //     if control_message_list.front().unwrap().len() <= (embeddable_end_index - embeddable_start_pointer) {
        //     //         warn!("embed control message in packet {}", packet_index);
        //     //         let control_message = control_message_list.pop_front().unwrap();
        //     //         // let start_index = embed_index+2+message_header_bytes.len();
        //     //         debug!("control_message.len() = {}", control_message.len());
        //     //         trace!("packet {} embedding message at index {} of length {}", packet_index, embeddable_start_pointer, control_message.len());
        //     //         packet[embeddable_start_pointer..embeddable_start_pointer+control_message.len()].copy_from_slice(&control_message);
        //     //         embeddable_start_pointer += control_message.len();
        //     //         num_messages_embedded += 1;
        //     //     } else {
        //     //         warn!("no more room to embed control messages in packet {}", packet_index);
        //     //     }
        //     //     break;
        //     // }
        //     // debug!("embedded {} messages in packet {}", num_messages_embedded, packet_index);
        //     self.packets.push(packet);
        // }
    }
}

// pub fn decode_message

pub struct MessageDecoder {
    packet_index: usize,
    embeddable_locations_per_packet: HashMap<usize, Vec<(u16, u16)>>,
}

impl Default for MessageDecoder {
    fn default() -> Self {
        Self {
            packet_index: 0,
            embeddable_locations_per_packet: HashMap::new(),
        }
    }
}

impl MessageDecoder {
    pub fn new(embeddable_locations_per_packet: HashMap<usize, Vec<(u16, u16)>>) -> Self {
        Self {
            embeddable_locations_per_packet,
            ..Default::default()
        }
    }

    pub fn decode_from_payload(
        &mut self,
        payload: &Vec<u8>,
    ) -> HashMap<usize, Vec<Vec<FieldJson>>> {
        debug_assert!(self
            .embeddable_locations_per_packet
            .contains_key(&self.packet_index));
        debug_assert!(
            self.embeddable_locations_per_packet
                .get(&self.packet_index)
                .unwrap()
                .len()
                > 0
        );

        let (message_index, message_length) = self
            .embeddable_locations_per_packet
            .get(&self.packet_index)
            .unwrap()
            .first()
            .unwrap();
        let message_index = *message_index as usize;

        let message_header_size = u16::from_be_bytes(
            payload[message_index..message_index + 2]
                .try_into()
                .unwrap(),
        ) as usize;

        error!("message_header_size = {}", message_header_size);

        let payload_pointer = message_index + 2;
        let message_header_bytes = &payload[payload_pointer..payload_pointer + message_header_size];
        // let message_header = ClientControlMessageHeader::decode(message_header_bytes).unwrap();
        // println!("message_header = {:?}", message_header);

        let message = ClientControlMessageList::decode(message_header_bytes).unwrap();
        // println!("message = {:?}", message);

        for location_info in message.locations {
            // let message_index = location.index as usize;
            // let message_length = location.length as usize;
            let packet_index = location_info.packet_index as usize;
            self.embeddable_locations_per_packet
                .entry(packet_index)
                .or_insert_with(|| vec![])
                .push((location_info.data_index as u16, 0));
            // for location in location_info.locations {
            //     self.embeddable_locations_per_packet.entry(packet_index).or_insert_with(|| vec![]).push((location.index as u16, location.length as u16));
            // }
        }

        trace!(
            "self.embeddable_locations_per_packet = {:#?}",
            self.embeddable_locations_per_packet
        );

        let testing: HashSet<(u16, u16)> = HashSet::new();

        // debug!("message.packets = {:?}", message.packets);

        // match self.embeddable_locations_per_packet.get(&self.packet_index) {
        //     Some(locations) => {
        //         for location in locations {
        //             println!("location = {:?}", location);
        //             let index = location.0 as usize;
        //             let length = location.1 as usize;
        //             let index = 101;
        //             let length = 79;
        //             trace!("decoding control message at index {} of length {}", index, length);
        //             let embedded_message = ClientControlMessageList::decode(&payload[index..index+length]);
        //             println!("decoded embedeed message = {:?}", embedded_message);
        //         }
        //     }
        //     None => {
        //         debug!("no embedded messages signaled for packet at index {}", self.packet_index);
        //     }
        // }

        self.packet_index += 1;

        let mut packets: HashMap<usize, Vec<Vec<FieldJson>>> = HashMap::new();

        for packet in message.packets {
            let fields: Vec<FieldJson> = packet
                .fields
                .into_iter()
                .map(|field| field.into())
                .collect();
            packets
                .entry(packet.response_index as usize)
                .or_insert_with(|| vec![])
                .push(fields);
        }

        packets
    }
}

///
/// Creates a packet with the embedded client control message responses.
///
/// TODO: split out the embedding logic since the server doesn't need it, but also uses this function.
pub fn generate_packet(
    fields: &Vec<FieldJson>,
    responses: &Vec<Vec<FieldJson>>,
    packet_index: usize,
) -> Vec<u8> {
    let mut packet = vec![];

    let control_message = encode_remora_control_message(&responses, packet_index);

    let mut embeddable_field_locations: Vec<(usize, usize)> = vec![];

    for field in fields {
        // The `raw` and `field` fields are mutually exclusive.
        assert!(
            field.raw.is_some() != field.field.is_some(),
            "invalid field detected: {:?}",
            field
        );
        if let Some(raw) = &field.raw {
            let mut hex_bytes = match hex::decode(&raw) {
                Ok(val) => val,
                Err(err) => {
                    error!("failed to decode hex string: {}: {}", err, raw);
                    panic!("{}", err);
                }
            };
            packet.append(&mut hex_bytes);
        } else if let Some(name) = &field.field {
            if is_proto_field_supported(name.as_str()) {
                let (mut result, size) = match generate_field_func(field, fields) {
                    Ok((result, size)) => (result, size),
                    Err(err) => {
                        let err_msg = format!("generate_field_func failed: {}", err);
                        error!("{}", err_msg);
                        panic!("{}", err_msg);
                    }
                };
                trace!("checking can embed for {}", name);
                if can_embed_control_message_in_field(&name) {
                    assert!(size % 8 == 0);
                    embeddable_field_locations.push((packet.len(), size / 8));
                    trace!("can embed!!! {}", name);
                }
                packet.append(&mut result);
            } else {
                error!("field {} is not supported :(", name);
                panic!("field {} is not supported :(", name);
            }
        }
    }

    // debug_assert!(embeddable_field_locations.len() > 0, "{:?}", fields);

    // trace!("embeddable_field_locations = {:?}", embeddable_field_locations);
    // let num_embeddable_bytes: usize = embeddable_field_locations.iter().map(|(_index, length)| length).sum();
    // trace!("num_embeddable_bytes = {}", num_embeddable_bytes);
    // trace!("control message size = {}", control_message.len());

    // // TODO: support striping across non-contiguous fields.
    // let &(embed_start, embed_size) = embeddable_field_locations.first().unwrap();
    // let embed_end = embed_start+control_message.len();
    // if embed_end < packet.len() {
    //     packet[embed_start..embed_end].copy_from_slice(&control_message);
    // } else {
    //     warn!("embeddable field of size {} too small for control message size {}. Extending packet size to {}", embed_size, control_message.len(), embed_end);
    //     packet.truncate(embed_start);
    //     packet.extend(control_message);
    // }

    // assert!(num_embeddable_bytes >= control_message.len(), "control message size {} is too large to embed in field(s) of size {}", control_message.len(), num_embeddable_bytes);

    packet
}

///
/// Generate a client packet with NO embedded data.
///
pub fn generate_raw_client_packet(fields: &Vec<FieldJson>) -> (Vec<u8>, Vec<(u16, u16)>) {
    let mut packet = vec![];
    let mut embeddable_field_locations: Vec<(u16, u16)> = vec![];
    for field in fields {
        // The `raw` and `field` fields are mutually exclusive.
        assert!(
            field.raw.is_some() != field.field.is_some(),
            "invalid field detected: {:?}",
            field
        );
        if let Some(raw) = &field.raw {
            let mut hex_bytes = match hex::decode(&raw) {
                Ok(val) => val,
                Err(err) => {
                    error!("failed to decode hex string: {}: {}", err, raw);
                    panic!("{}", err);
                }
            };
            packet.append(&mut hex_bytes);
        } else if let Some(name) = &field.field {
            if is_proto_field_supported(name.as_str()) {
                let (mut result, size) = match generate_field_func(field, fields) {
                    Ok((result, size)) => (result, size),
                    Err(err) => {
                        let err_msg = format!("generate_field_func failed: {}", err);
                        error!("{}", err_msg);
                        panic!("{}", err_msg);
                    }
                };
                if can_embed_control_message_in_field(&name) {
                    assert!(size % 8 == 0);
                    embeddable_field_locations.push((packet.len() as u16, size as u16 / 8));
                }
                packet.append(&mut result);
            } else {
                let err_msg = format!(
                    "field {} is not supported. Check your function spec JSON.",
                    name
                );
                error!("{}", err_msg);
                panic!("{}", err_msg);
            }
        }
    }

    (packet, embeddable_field_locations)
}

///
/// Encode the client control message as a protobuf.
///
///
/// # Arguments
///
/// **`server_packets`** - A list of server packets, each of which is a list of fields.
///
/// **`client_packet_index`** - Index of the client packet to which this server packet is a
/// response.
///
/// # Errors
///
fn encode_server_packet_as_protobuf(
    server_packets: &Vec<Vec<FieldJson>>,
    client_packet_index: usize,
    location: ClientControlMessageEmbeddedDataLocations,
) -> Vec<u8> {
    // Convert FieldJson to ClientControlMessageProtobuf.
    let mut proto_client_control_message = ClientControlMessageList {
        // locations: vec![ClientControlMessageEmbeddedDataLocations {
        //     packet_index: client_packet_index as u64,
        //     data_index: 0,
        // }],
        locations: vec![location],
        packets: vec![],
    };
    for packet in server_packets {
        let mut proto_packet = ClientControlMessagePacket {
            fields: vec![],
            response_index: client_packet_index as u64,
        };
        for field in packet {
            let proto_field = ClientControlMessageField {
                raw: field.raw.clone(),
                field: field.field.clone(),
                arg: field.arg,
            };
            proto_packet.fields.push(proto_field);
        }

        proto_client_control_message.packets.push(proto_packet);
    }

    // control_message.clear();
    let mut buf = Vec::new();
    proto_client_control_message.encode(&mut buf).unwrap();
    buf
}

///
/// Generate a client control message to signal the server to respond with a given list of
/// responses.
///
/// Input is a list of packets, each one being a list of fields.
pub fn encode_remora_control_message(
    responses: &Vec<Vec<FieldJson>>,
    packet_index: usize,
) -> Vec<u8> {
    // debug_assert!(responses.len() < 256, "Num responses {} exceeds limit", responses.len());
    // // assert!(responses.len() < 256, "Num responses {} exceeds limit", responses.len());
    // let mut control_message = vec![];
    // // For now just write the json as the control message.
    // // TODO: come up with a better more space efficient scheme.
    // if !responses.is_empty() {
    //     serde_json::to_writer(&mut control_message, responses).unwrap();
    // }

    // debug!("control_message size = {}", control_message.len());

    // let mut buf = encode_server_packet_as_protobuf(responses);
    // trace!("Protobuf content size = {}", buf.len());
    // let compression_ratio = if control_message.len() > 0 {
    //     buf.len() as f64 / control_message.len() as f64 * 100.0
    // } else {
    //     0.0
    // };
    // trace!("compression ratio = {:.1}%", compression_ratio);

    // control_message = buf;

    let mut control_message = encode_server_packet_as_protobuf(
        responses,
        packet_index,
        ClientControlMessageEmbeddedDataLocations::default(),
    );

    // trace!("control message content size = {}", control_message.len());
    // debug!("control message content size = {:?}", control_message.len().to_be_bytes());

    // Add a four-byte footer containing the length of the message.
    control_message.extend((control_message.len() as u32).to_be_bytes());
    trace!("control_message = {}", hex::encode(&control_message));
    debug!("control_message.len() = {}", control_message.len());

    control_message
}

///
/// Decode a protobuf into the client control message. The inverse of `encode_server_packet_as_protobuf`.
///
fn decode_protobuf(payload: &[u8]) -> Result<Vec<Vec<FieldJson>>, String> {
    let protobuf_responses = ClientControlMessageList::decode(payload).unwrap();
    let mut responses = vec![];
    for protobuf_packet in protobuf_responses.packets {
        let mut packet = vec![];
        for protobuf_field in protobuf_packet.fields {
            packet.push(FieldJson {
                raw: protobuf_field.raw,
                field: protobuf_field.field,
                arg: protobuf_field.arg,
            });
        }
        responses.push(packet);
    }

    Ok(responses)
}

///
/// Decode a client control message on the server side.
///
pub fn decode_remora_control_message(payload: &Vec<u8>) -> Result<Vec<Vec<FieldJson>>, String> {
    let responses = vec![];

    trace!("decoding payload of size {}", payload.len());
    // trace!("decoding payload: {:?}", payload);

    let payload_size = payload.len();
    if payload_size >= 4 {
        // The first four bytes contain the size of the client control message.
        let control_message_size =
            u32::from_be_bytes(payload[payload_size - 4..payload_size].try_into().unwrap())
                as usize;
        trace!("control_message_size = {}", control_message_size);

        // TODO: this is an arbitary bound. Need to come up with a mechanism to signal NO control
        // message. Something like 0xFFFFFFFF, or some arbitrary value and just pad the client
        // control message by a byte if it happens to be that exact size.
        const CONTROL_MESSAGE_MAX_SIZE: usize = 4096;
        if control_message_size > CONTROL_MESSAGE_MAX_SIZE {
            return Err(format!(
                "control message size {} exceeds max. probably invalid control message.",
                control_message_size
            ));
        }

        if control_message_size > 0 {
            // The end of the client control message is after the footer.
            const CONTROL_MESSAGE_FOOTER_SIZE: usize = 4;
            let end = payload.len() - CONTROL_MESSAGE_FOOTER_SIZE;
            assert!(
                end >= control_message_size,
                "Control message size is too large to be contained in the payload."
            );
            // Compute the start by walking back from the end now that we have the control message
            // size.
            let start = end - control_message_size;
            // debug!("start = {}    end = {}", start, end);

            trace!("start = {}", start);
            trace!("end = {}", end);

            // if cfg!(feature = "msgpack") {

            //     trace!("payload = {:?}", payload);
            //     trace!("control message = {:?}", &payload[start..end]);

            //     // Decode from MsgPack.
            //     let mut de = Deserializer::new(std::io::Cursor::new(&payload[start..end]));

            //     // let value: rmpv::Value = rmpv::Value::deserialize(&mut de).unwrap();
            //     // let value: rmpv::Value = rmpv::decode::from_slice(&payload[start..end]);
            //     let value = rmpv::decode::value::read_value(&mut std::io::Cursor::new(&payload[start..end]));

            //     trace!("msgpack value = {:?}", value);

            //     // return Ok(vec![]);

            //     let responses = Deserialize::deserialize(&mut de).unwrap();
            //     return Ok(responses);

            //     // let deserialized_data: MyData = Deserialize::deserialize(&mut de).unwrap();
            //     // match Deserialize::deserialize(&mut de) {
            //     //     Ok(responses) => return Ok(responses),
            //     //     Err(err) => {
            //     //         error!("failed to decode MsgPack control message JSON: {}", err);
            //     //         return Err("failed to decode MsgPack JSON".into());
            //     //     }
            //     // }
            // }
            // else
            // {

            //     // let debug_payload: Vec<u8> = vec![91, 91, 123, 34, 114, 97, 119, 34, 58, 34, 99, 52, 48, 48, 48, 48, 48, 48, 48, 49, 48, 48, 48, 56, 101, 55, 54, 55, 98, 52, 57, 56, 98, 99, 51, 99, 56, 48, 51, 101, 48, 48, 52, 51, 97, 48, 48, 52, 34, 44, 34, 102, 105, 101, 108, 100, 34, 58, 110, 117, 108, 108, 44, 34, 115, 105, 122, 101, 34, 58, 110, 117, 108, 108, 125, 44, 123, 34, 114, 97, 119, 34, 58, 110, 117, 108, 108, 44, 34, 102, 105, 101, 108, 100, 34, 58, 34, 113, 117, 105, 99, 46, 112, 97, 121, 108, 111, 97, 100, 34, 44, 34, 115, 105, 122, 101, 34, 58, 55, 52, 49, 54, 125, 93, 93];
            //     // debug!("{:?}", &payload[start..end]);
            //     match serde_json::from_slice::<Vec<Vec<FieldJson>>>(&payload[start..end]) {
            //     // match serde_json::from_slice::<Vec<Vec<FieldJson>>>(&debug_payload) {
            //         Ok(responses) => return Ok(responses),
            //         Err(err) => {
            //             error!("failed to decode control message JSON: {}", err);
            //             return Err("failed to decode JSON".into());
            //         }
            //     }
            // }

            return decode_protobuf(&payload[start..end]);
        }
    }

    Ok(responses)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn protobuf() {
        let proto_field = ClientControlMessageFieldProtobuf {
            raw: "".into(),
            field: "really.long.field.name".into(),
            arg: 0,
        };

        // control_message.clear();
        let mut buf = Vec::new();
        proto_field.encode(&mut buf).unwrap();

        println!("protobuf size = {}", buf.len());
        println!("protobuf = {:?}", buf);

        let proto_field = ClientControlMessageFieldProtobuf {
            raw: "".into(),
            field: "s".into(),
            arg: 0,
        };

        // control_message.clear();
        let mut buf = Vec::new();
        proto_field.encode(&mut buf).unwrap();

        println!("protobuf size = {}", buf.len());
        println!("protobuf = {:?}", buf);
    }
}
