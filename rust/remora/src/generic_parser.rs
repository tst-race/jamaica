// Copyright 2024 Two Six Technologies.

use log::{error, info, trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::net::SocketAddr;

// mod connection_tuple;
use crate::connection_tuple;
use crate::connection_tuple::Protocol;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Field {
    /// The name of the field.
    pub name: String,
    /// The tshark display value. Should only be used for debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    display: Option<String>,
    /// The index (position) in bits of the field in the layer.
    pub index: usize,
    /// The length of the field in bits.
    pub length: usize,
    /// A hex string of the field value.
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Packet {
    pub direction: String, // TODO: change to enum? YES
    raw: String,
    length: usize,
    packet_number: usize,
    frame_number: usize,
    timestamp: f64,
    // Size of the packet layer in bits.
    size_in_bits: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<u32>,
    pub fields: Vec<Field>,
}

// Remora connection JSON object.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct OutputFormat {
    pub src_addr: SocketAddr,
    pub dst_addr: SocketAddr,
    pub protocol: Protocol,
    pub connection_id: usize,
    pub packets: Vec<Packet>,
}

pub struct GenericParser {
    protocol: String,
    /// Map of connection IDs to packets within that connection. Ultimately a list of these will be
    /// saved as the json output.
    connection_to_packets: HashMap<String, OutputFormat>,
    /// Name of the output json file.
    output_file_name: String,
    /// Flag to indicate if showname display value should be included in the output.
    include_display: bool,
    connection_to_packet_number_counter: HashMap<String, usize>,
}

impl Drop for GenericParser {
    fn drop(&mut self) {
        self.save();
    }
}

impl GenericParser {
    pub fn new(file_stem: &String, protocol: String, include_display: bool) -> Self {
        let output_file_name = format!("{}-{}.json", file_stem, protocol);
        Self {
            protocol,
            connection_to_packets: HashMap::new(),
            output_file_name,
            include_display,
            connection_to_packet_number_counter: HashMap::new(),
        }
    }

    pub fn parse(&mut self, packet: &rtshark::Packet) {
        debug_assert!(packet.layer_name(self.protocol.as_str()).is_some());

        // let frame_number = match packet.layer_name("frame") {
        //     Some(layer) => {
        //         match layer.metadata("frame.number") {
        //             Some(value) => value.value().parse().unwrap(),
        //             None => 0,
        //         }
        //     }
        //     None => 0
        // };

        // let frame_number = match (|| {
        //     Some(packet
        //         .layer_name("frame")?
        //         .metadata("frame.number")?
        //         .value())
        // })() {
        //     Some(value) => value,
        //     None => "error getting frame number"
        // };

        let frame_number = (|| {
            Some(
                packet
                    .layer_name("frame")?
                    .metadata("frame.number")?
                    .value(),
            )
        })()
        .unwrap_or("error getting frame number")
        .parse()
        .unwrap();

        //frame.time_epoch
        let frame_timestamp = (|| {
            Some(
                packet
                    .layer_name("frame")?
                    .metadata("frame.time_epoch")?
                    .value(),
            )
        })()
        .unwrap_or("error getting frame.time_epoch")
        .parse()
        .unwrap();

        let raw_payload = match get_raw_payload(packet) {
            Ok(result) => result,
            Err(_) => {
                error!("Failed to parse raw payload for frame {}", frame_number);
                String::new()
            }
        };

        let connection = match connection_tuple::packet_to_tuple(packet) {
            Some(connection) => connection,
            None => {
                // error!(
                //     "Failed to parse connection for packet with frame number: {}",
                //     frame_number
                // );
                trace!("Failed to parse connection for {:?}", packet);
                return;
            }
        };
        // println!("{:?}", connection);

        // self.connection_to_packets.entry(connection.clone()).or_insert(vec![]);
        // self.connection_to_packets.get_mut(&connection).unwrap().push(packet);

        // match self.connection_to_packets.entry(connection) {
        //     Entry::Vacant(e) => { e.insert(vec![]); },
        //     Entry::Occupied(mut e) => {e.get_mut().push(packet);},
        // }
        let src_addr = connection.src_addr;
        // let mut value = match self.connection_to_packets.entry(connection) {
        //     std::collections::hash_map::Entry::Vacant(e) => { e.insert(vec![]); }, // o.into_mut(),
        //     std::collections::hash_map::Entry::Occupied(mut e) => e,
        // };

        let mut previous_layer_offset = 0;

        for layer in packet.clone() {
            if layer.name() != self.protocol {
                continue;
            }

            // if layer.size() < 3 {
            //     warn!("{:?}", packet);
            //     panic!("wtf");
            // }
            let does_connection_key_exist = self
                .connection_to_packets
                .contains_key(&connection.get_id());
            if !does_connection_key_exist {
                if let Some(parsed_packet) = self.parse_to_json(
                    &layer,
                    "from_src".into(),
                    raw_payload.clone(),
                    previous_layer_offset,
                    frame_number,
                    0,
                    frame_timestamp,
                ) {
                    let output = OutputFormat {
                        src_addr: connection.src_addr,
                        dst_addr: connection.dst_addr,
                        protocol: connection.protocol,
                        connection_id: self.connection_to_packets.keys().len(),
                        packets: vec![parsed_packet],
                    };
                    let existing = self
                        .connection_to_packets
                        .insert(connection.get_id(), output);
                    debug_assert_eq!(existing, None);
                    self.connection_to_packet_number_counter
                        .insert(connection.get_id(), 0);
                } // TODO: what about the else case???
            } else {
                let connection_src_addr = self
                    .connection_to_packets
                    .get_mut(&connection.get_id())
                    .unwrap()
                    .src_addr;
                let direction: String = if connection_src_addr == src_addr {
                    "from_src".into()
                } else {
                    "from_dst".into()
                };
                if let Some(parsed_packet) = self.parse_to_json(
                    &layer,
                    direction,
                    raw_payload.clone(),
                    previous_layer_offset,
                    frame_number,
                    self.connection_to_packet_number_counter[&connection.get_id()],
                    frame_timestamp,
                ) {
                    self.connection_to_packets
                        .get_mut(&connection.get_id())
                        .unwrap()
                        .packets
                        .push(parsed_packet);
                } // TODO: what about the else case???
            }

            previous_layer_offset = layer.position();
        }

        self.connection_to_packet_number_counter.insert(
            connection.get_id(),
            self.connection_to_packet_number_counter[&connection.get_id()] + 1,
        );
    }

    /// Parse an rtshark::Layer into a Packet struct which can be saved as a JSON object.
    /// rtshark::Layer -> struct Packet
    ///
    /// # Arguments
    ///
    /// * `packet` - The packet to parse.
    /// * `protocol` - The protocol to parse, i.e. the name of the layer to parse within the
    /// packet.
    /// TODO: BUG! There can be multiple layers of the same name in a single packe, e.g. QUIC
    /// * `direction` - The direction of the packet, either `from_src` or `from_dst`.
    ///
    /// # Examples
    ///
    ///
    fn parse_to_json(
        &self,
        // packet: &rtshark::Packet,
        layer: &rtshark::Layer,
        direction: String,
        raw_payload: String,
        previous_layer_offset: u32,
        frame_number: usize,
        packet_number: usize,
        timestamp: f64,
    ) -> Option<Packet> {
        debug_assert!(direction == "from_src" || direction == "from_dst");

        let mut fields = vec![];
        // The expected position of the meta data field in bits (instead of bytes).
        // let mut expected_position_in_bits = 0; //layer.position() * 8;
        // let mut layer_size = 0;
        // for layer in packet.clone() {
        debug_assert!(layer.name() == self.protocol);

        let mut expected_field_position = 0;

        // layer_size += layer.size();

        let layer_size_in_bits = layer.size() * 8;

        // trace!("found layer for {}", protocol);
        for metadata in layer.iter() {
            // trace!("    {}\t{}", metadata.name(), metadata.size());
            // TODO: can these filters be enforced generically across all protocol/packet types?

            // let actual_position_in_bits = metadata.position() * 8;
            // if actual_position_in_bits != expected_position_in_bits {
            //     warn!(
            //         "Expected metadata position {} vs. actual {} for {} frame num {}",
            //         expected_position_in_bits,
            //         actual_position_in_bits,
            //         metadata.name(),
            //         frame_number,
            //     );
            // }

            if metadata.hide() {
                // debug_assert!(metadata.size() == 0, "shoot"); // This is NOT true. Example in remora-source-pcaps/two-six-manually-generated/capture_2024-04-25-11-45-47_eth_google_meet.pcap
                // trace!("skipping field with hide set to yes: {}", metadata.name());
                // TODO: do we need to increment expected_position_in_bits? or is size always zero?
                continue;
            }

            // NOTE: ignoring metadata of size zero. I'm not sure how it's even deriving
            // these. I don't see them at all in the RFC, e.g. QUIC connection number.
            if metadata.size() == 0 {
                // if position != metadata.position() {
                //     trace!("positions not equal! {} {} {}", metadata.name() , position, metadata.position())
                // }
                // position = metadata.position();
                continue;
            }

            // NOTE!!!! hacky stuff to workaround tshark things. This is a derived field that doesn't
            // exist as bytes inside the packet, but is derived based on the values of other fields.
            if metadata.name() == "rtp.extseq" {
                // this doesn't matter here since the position of this derived field is the
                // same as the previous field.
                // position = metadata.position();
                continue;
            }

            // // NOTE: QUIC frames are located inside the protected payload, so we really
            // // don't care about parsing them (validate this assumption). Also, metadata
            // // formatting will fail for frame fields at the moment.
            // // This check MUST be done before the value check since the value for this
            // // field is zero.
            // // TODO: rtshark is flattening the structure which it makes it difficult to detect fields
            // // nested under a field with an empty value. May have to modify the rtshark source to
            // // support some flag or other indication that a field is a child. For now just settling
            // // with ad-hoc filtering.
            // if metadata.name() == "quic.frame" {
            //     break;
            // }

            // This is a hack to handle the above, commented out case generically.
            // TODO: Need to thoroughly test this on other protocls. Should really find a better way of
            // detecting nested sections. Don't necessarily want to break here.
            // TODO: may require changes to rtshark to have some indication of depth/nestedness.
            // Manually skip logging the problematic fields we already know about.
            // let known_problems = ["quic.frame", "quic.frame_type"];
            // if actual_position_in_bits < expected_position_in_bits {
            //     // trace!(
            //     //     "breaking frame num {} because position {}",
            //     //     frame_number,
            //     //     metadata.name()
            //     // );
            //     if !known_problems.contains(&metadata.name()) {
            //         trace!(
            //                 "WARNING: frame num {} has metadata position {} less than prev position {} for {}",
            //                 frame_number,
            //                 actual_position_in_bits,
            //                 expected_position_in_bits,
            //                 metadata.name()
            //             );
            //         // trace!("{:?}", metadata);
            //     }
            //     // break; // TODO: or should we continue to get the potentially valid lower metadata?
            //     // or just remove this check altogether?
            // }

            // NOTE: skipping metadata with empty values. This implies the metadata is
            // actually a section with values under it (e.g. QUIC short header), but
            // rtshark is flattening the structure.
            if metadata.value().is_empty() {
                continue;
            }

            // expected_position_in_bits = metadata.position() + metadata.size() as u32;
            // position = metadata.position() + metadata.size();

            let field = match format_metadata_field(
                metadata,
                layer.position(),
                fields.last(),
                self.include_display,
            ) {
                Ok(field) => field,
                Err(err) => {
                    warn!(
                        "skipping malformed field at frame num {}: {}",
                        frame_number, err
                    );
                    continue;
                }
            };

            if expected_field_position != field.index as u32 {
                // TODO: in the case of a gap get the missing bits from the raw payload.
                warn!(
                    "Expected metadata field position {} vs. actual {} for {} frame num {}",
                    expected_field_position, field.index, field.name, frame_number,
                );
                if expected_field_position < field.index as u32 {
                    let num_missing_bits = field.index as u32 - expected_field_position;
                    assert!(num_missing_bits < (u16::MAX as u32 * 8));

                    let start_position_bytes = expected_field_position as usize / 4;
                    let end_position_in_bytes = (field.index / 4) + 1;

                    let mut missing_bytes = String::from(
                        match raw_payload.get(start_position_bytes..end_position_in_bytes) {
                            Some(value) => value.to_string(),
                            None => {
                                warn!("No raw payload for adding missing bytes at frame num {}. Filling with zeros.", frame_number);
                                String::from("0")
                                    .repeat(end_position_in_bytes - start_position_bytes)
                            }
                        },
                    );
                    assert!(missing_bytes.len() > 0);
                    assert!(missing_bytes.len() < u16::MAX as usize);
                    if missing_bytes.len() & 1 != 0 {
                        missing_bytes.push_str("0");
                    }

                    let mut missing_bytes = hex::decode(missing_bytes.as_str())
                        .expect(format!("Decoding failed {}", missing_bytes).as_str());

                    // let bitmask =

                    // if expected_field_position is not divisible by 8 then mask the first byte.
                    if expected_field_position % 8 != 0 {
                        let mut bitmask = 0;
                        let mask_size = 8 - (expected_field_position % 8);
                        for i in 0..mask_size {
                            bitmask |= 1 << i;
                        }

                        missing_bytes[0] &= bitmask;
                    }

                    let missing_bytes = hex::encode(missing_bytes);

                    // if field.index is not divisible by 8 then mask the last byte.

                    // warn!("GAP! Missing {} bits {:?}", num_missing_bits, missing_bytes);
                    trace!(
                        "Gap detected. Adding field for missing {} bits",
                        num_missing_bits
                    );

                    let missing_field = Field {
                        name: format!("{}.unknown", self.protocol),
                        display: None,
                        index: expected_field_position as usize,
                        length: num_missing_bits as usize,
                        value: missing_bytes,
                    };
                    fields.push(missing_field);
                } else {
                    // warn!(
                    //     "OVERLAP! Expected metadata field position {} vs. actual {} for {} frame num {}",
                    //     expected_field_position,
                    //     field.index,
                    //     field.name,
                    //     frame_number,
                    // );
                }
            }

            expected_field_position = (field.index + field.length) as u32;

            let current_size_in_bits = field.index + field.length;
            let field_name = field.name.clone();
            fields.push(field);

            // When you hit the end of the layer just break. This cuts out some "meta" fields like
            // QUIC frames. See
            // remora-source-pcaps/two-six-manually-generated/capture_2023-11-28-09-26-51_eth_google_meet_chrome.pdml
            // for an example in frame number 1714 where QUIC frames show up with a position at the
            // beginning of the layer. Hoping these can be safely ignored.
            // TODO: investigate what the hell is going on with QUIC frames.
            if current_size_in_bits >= layer_size_in_bits {
                // If the size doesn't line up with the layer size then log an error and figure out
                // what's going wrong.
                if current_size_in_bits != layer_size_in_bits {
                    error!(
                        "Actual size {} does not equal expected size {} for frame {} {} {}",
                        current_size_in_bits,
                        layer_size_in_bits,
                        frame_number,
                        self.protocol,
                        field_name
                    );
                }
                break;
            }
        }
        // break;
        // }

        // TODO: stream this to file instead of storing in memory? Could become a
        // bottleneck for larger files.
        // Could also allow us to kill the process mid-run and still get _some_ output.

        let adjusted_payload = if layer.size() != (raw_payload.len() / 2) {
            // NOTE: tshark can report a layer position that is smaller than the previous
            // layer's offset for some frames (e.g. TCP segment reassembly/retransmission
            // oddities). Guard against underflow instead of assuming offsets always increase.
            let start = if previous_layer_offset == 0 {
                Some(0)
            } else if layer.position() >= previous_layer_offset {
                Some(layer.position() - previous_layer_offset)
            } else {
                None
            };

            match start {
                Some(start) => {
                    let end = start + layer.size() as u32;
                    let start = (start * 2) as usize;
                    let end = (end * 2) as usize;
                    match raw_payload.get(start..end) {
                        Some(value) => value.to_string(),
                        None => {
                            warn!("Failed to slice raw payload at frame num {}", frame_number);
                            raw_payload
                        }
                    }
                }
                None => {
                    warn!(
                        "layer position {} is less than previous layer offset {} at frame num {}, using full raw payload instead of slicing",
                        layer.position(),
                        previous_layer_offset,
                        frame_number
                    );
                    raw_payload
                }
            }
        } else {
            raw_payload
        };

        Some(Packet {
            direction,
            raw: adjusted_payload,
            length: layer.size(),
            packet_number,
            frame_number,
            timestamp,
            size_in_bits: layer_size_in_bits,
            position: Some(layer.position()),
            fields,
        })
    }

    pub fn save(&self) {
        // Wrap the json object in an array.
        // self.writer.write("]".as_bytes()).unwrap();

        // If nothing was parsed then don't create an output file.
        if !self.connection_to_packets.is_empty() {
            let mut connections = vec![];

            for packets in self.connection_to_packets.values() {
                // let output = OutputFormat {
                //     src_addr: connection.src_addr,
                //     dst_addr: connection.dst_addr,
                //     protocol: connection.protocol,
                //     packets: packets.clone(),
                // };
                connections.push(packets);
            }

            let formatted = serde_json::to_string_pretty(&connections).expect("ah shucks");
            // let formatted = serde_json::to_string_pretty(&self.connection_to_packets).expect("ah shucks");
            // // let mut file = File::create(output_file_name).unwrap();
            // // file.write_all(formatted.as_bytes()).expect("come on man");

            // let output_file_name = format!("{}-{}.json", file_stem, file_suffix);
            let file = std::fs::File::create(self.output_file_name.clone()).unwrap();
            let mut writer = BufWriter::new(file);

            writer.write_all(formatted.as_bytes()).expect("come on man");

            info!("Saved output to {}", self.output_file_name);
        } else {
            warn!("Nothing parsed for {}", self.output_file_name);
        }
    }
}

/// Formats rtshark metadata into a Field struct. The input represents a single `field` element in
/// the underlying PDML.
///
/// # Arguments
///
/// * `field` - The field to format.
/// * `index_offset` - The offset of the input field's layer in the packet in which it is
/// contained. Used to adjust the field index to be relative to the layer, not the packet.
/// * `include_display` - Include the showname display value in the returned Field.
///
/// # Examples
///
/// ```ignore
/// let metadata = rtshark::Metadata::new(
///     "my_metadata_field_name".into(),
///     "some_value".into(),
///     "raw_value".into(),
///     "display".into(),
///     12,
///     4,
///     false
/// );
/// let mut field = format_metadata_field(metadata);
/// assert_eq!(field.name(), metadata.name());
/// ```
fn format_metadata_field(
    field: &rtshark::Metadata,
    index_offset: u32,
    _previous_field: Option<&Field>,
    include_display: bool,
) -> Result<Field, String> {
    fn _to_bits(value_in_bytes: u32) -> usize {
        (value_in_bytes * 8) as usize
    }
    // Make sure the supplied index is less than or equal to the current field index. This _should_
    // never hit, but who tf knows.
    if field.position() < index_offset {
        return Err(format!(
            "offset of {} is greater than the position {} for metadata {}",
            index_offset,
            field.position(),
            field.name()
        ));
    }

    let (mut index, mut length) =
        if parse_field_type(field.value(), field.display()) == FieldType::Bits {
            let (index, length) = decode_bit_index_from_display(field.display())?;
            (index + _to_bits(field.position() - index_offset), length)
        } else {
            (
                _to_bits(field.position() - index_offset),
                _to_bits(field.size()),
            )
        };

    let name = field.name().into();
    let display = if include_display {
        Some(field.display().into())
    } else {
        None
    };

    // NOTE!!! Hacky stuff to deal with annoying, non-obvious bit fields.
    if name == "rtp.ext.rfc5285.id" {
        length = 4;
    } else if name == "rtp.ext.rfc5285.len" {
        index += 4;
        length = 4;
    } else if name == "rtp.p_type" {
        index += 1;
        length = 7;
    }

    Ok(Field {
        name,
        display,
        index,
        length,
        value: ensure_hex_string(field.raw_value()),
    })

    // TODO: cache the field type based on name so we don't have to parse every time?
    // let field_type = parse_field_type(field.value(), field.display());
    // let name = field.name().into();
    // let display = if include_display {
    //     Some(field.display().into())
    // } else {
    //     None
    // };
    // let field = match field_type {
    //     // Hex value represented as a string.
    //     FieldType::HexColon => Field {
    //         name,
    //         display,
    //         index,
    //         length,
    //         value: format_hex(field.value()),
    //     },
    //     FieldType::Bits => {
    //         let (bit_field_index, length) =
    //             decode_bit_index_from_display(field.display()).expect("nopo");
    //         Field {
    //             name,
    //             display,
    //             index: index + bit_field_index,
    //             length,
    //             value: format_binary_value(field.value())?,
    //         }
    //     }
    //     FieldType::Integer => Field {
    //         name,
    //         display,
    //         index,
    //         length,
    //         value: integer_string_to_hex_string(field.value().to_string()),
    //     },
    //     FieldType::Hex => Field {
    //         name,
    //         display,
    //         index,
    //         length,
    //         value: field.value().trim_start_matches("0x").into(),
    //     },
    //     // Catch all.
    //     FieldType::Default => Field {
    //         name,
    //         display,
    //         index,
    //         length,
    //         value: field.value().into(),
    //     },
    // };

    // Ok(field)
}

///
/// # Returns
///
/// Result tuple of index and length, both in bits. Index is relative to the start of this metadata
/// field.
fn decode_bit_index_from_display(display: &str) -> Result<(usize, usize), &str> {
    let tokens: Vec<_> = display.split(" = ").collect();
    if let Some(bit_field) = tokens.first() {
        let bit_field = bit_field.replace(' ', "");
        // Find the first bit in the field.
        let index = bit_field
            .chars()
            .position(|c| c != '.')
            .unwrap_or_else(|| panic!("failed to parse bit field display {}", display));
        // Using a slice starting from the first found bit in the field, find the distance to the
        // last period. If none is found then the bits go all the way to the end of the string,
        // so the length is just the length of the entire bit field minus the starting index.
        let length = bit_field[index..]
            .chars()
            .position(|c| c == '.')
            .unwrap_or_else(|| bit_field.len() - index);

        return Ok((index, length));
    }

    Err("Invalid binary display string")
}

#[derive(Debug, PartialEq)]
enum FieldType {
    Hex,
    HexColon,
    Bits,
    Integer,
    Default,
}

fn parse_field_type(value: &str, display: &str) -> FieldType {
    // TODO: what about field values that are a single byte of hex (e.g. ff) and have no colon?
    //       ADD UNIT TESTS!!!
    let re = Regex::new(r"[0-9a-fA-F][0-9a-fA-F]:").unwrap();
    if re.captures(value).is_some() {
        return FieldType::HexColon;
    }

    let re = Regex::new(r"[.0-1][.0-1][.0-1][.0-1] [.0-1][.0-1][.0-1][.0-1]").unwrap();
    if re.captures(display).is_some() {
        return FieldType::Bits;
    }

    // TODO: parse hex of 0xFF style.
    if value.starts_with("0x") {
        return FieldType::Hex;
    }

    if value.parse::<i64>().is_ok() {
        return FieldType::Integer;
    }

    // TODO: add case for human readable string
    // TODO: or maybe just fix for tls.handshake.random_time

    FieldType::Default
}

fn format_hex(input: &str) -> String {
    input.replace(':', "")
}

/// Downstream consumers (e.g. field-qualification, correlator) assume every `Field::value` is a
/// hex string. Some tshark dissectors (e.g. x509 timestamps like "Apr 28, 2080 07:12:05.000000000
/// EDT", or ALPN protocol strings like "h2") have no underlying hex representation and rtshark's
/// `raw_value()` falls back to that display text. Detect that case and hex-encode the raw bytes
/// instead of passing the non-hex text through, so the "value" field always honors its documented
/// contract.
fn ensure_hex_string(raw_value: &str) -> String {
    // NOTE: hex strings are commonly odd-length, e.g. a single nibble value like "0" or "1"
    // for sub-byte bit fields, so length is intentionally not checked here.
    let is_valid_hex = raw_value.is_empty() || raw_value.chars().all(|c| c.is_ascii_hexdigit());

    if is_valid_hex {
        return raw_value.to_owned();
    }

    warn!(
        "field value {:?} is not a hex string, hex-encoding raw bytes instead",
        raw_value
    );
    hex::encode(raw_value.as_bytes())
}

fn get_raw_payload(packet: &rtshark::Packet) -> Result<String, String> {
    if let Some(udp_layer) = packet.layer_name("udp") {
        match udp_layer.metadata("udp.payload") {
            Some(payload) => return Ok(format_hex(payload.value())),
            None => {
                // panic!("Failed to parse udp payload!");
                let error_message = "Failed to parse udp payload!";
                error!("{}", error_message);
                return Err(error_message.into());
            }
        }
    }
    if let Some(tcp_layer) = packet.layer_name("tcp") {
        // TODO: fix this, doesn't work.
        match tcp_layer.metadata("tcp.payload") {
            Some(payload) => return Ok(format_hex(payload.value())),
            None => {
                // let frame_number = (|| {
                //     Some(
                //         packet
                //             .layer_name("frame")?
                //             .metadata("frame.number")?
                //             .value(),
                //     )
                // })()
                // .unwrap_or("error getting frame number");
                // warn!(
                //     "Failed to parse tcp payload for frame number {}",
                //     frame_number
                // );
                // trace!(
                //     "Failed to parse tcp payload for frame number {}!\n{:?}",
                //     frame_number,
                //     tcp_layer
                // );
            }
        }
    }

    Err("No layer found for UDP or TCP".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_field_type_test() {
        assert_eq!(
            FieldType::Bits,
            parse_field_type("1", "1... .... .... ....")
        );
        assert_eq!(
            FieldType::Bits,
            parse_field_type("0x1", "1... .... .... ....")
        );
        assert_eq!(
            FieldType::Bits,
            parse_field_type("0x01", "1... .... .... ....")
        );
        assert_eq!(FieldType::Bits, parse_field_type("0x00", "..00 ...."));

        // assert_eq!(FieldType::Hex, parse_field_type("0", "0x00"));
    }

    #[test]
    fn decode_bit_index_from_display_test() {
        let (index, length) = decode_bit_index_from_display("1... ....").expect("");
        assert_eq!(index, 0);
        assert_eq!(length, 1);

        let (index, length) =
            decode_bit_index_from_display("..0. .... = Spin Bit: False").expect("");
        assert_eq!(index, 2);
        assert_eq!(length, 1);

        let (index, length) = decode_bit_index_from_display("..01 11.. = garbage text").expect("");
        assert_eq!(index, 2);
        assert_eq!(length, 4);

        let display =
            ".... ..0. .... .... .... .... = LG bit: Globally unique address (factory default)";
        let (index, length) = decode_bit_index_from_display(display).expect("");
        assert_eq!(index, 6);
        assert_eq!(length, 1);

        let display = ".... ...0 .... .... .... .... = IG bit: Individual address (unicast)";
        let (index, length) = decode_bit_index_from_display(display).expect("");
        assert_eq!(index, 7);
        assert_eq!(length, 1);

        let display = "..00 .... = Packet Type: Initial (0)";
        let (index, length) = decode_bit_index_from_display(display).expect("");
        assert_eq!(index, 2);
        assert_eq!(length, 2);

        let display = ".... ..00 = Packet Number Length: 1 bytes (0)";
        let (index, length) = decode_bit_index_from_display(display).expect("");
        assert_eq!(index, 6);
        assert_eq!(length, 2);
    }

    #[test]
    fn format_metadata_field_test() {
        // Create metadata from the sample PDML.
        // <field name="gquic.version" showname="Version: Q043" size="4" pos="51" show="Q043" value="51303433"/>
        let metadata = rtshark::Metadata::new(
            "gquic.version".to_string(),
            "Q043".to_string(),
            "51303433".to_string(),
            "Version: Q043".to_string(),
            4,
            51,
            false,
        );
        let field = format_metadata_field(&metadata, 0, None, false).unwrap();
        assert_eq!("51303433".to_string(), field.value);

        // Create metadata from the sample PDML.
        // <field name="quic.header_form" showname="1... .... = Header Form: Long Header (1)" size="1" pos="42" show="1" value="1" unmaskedvalue="cc"/>
        let metadata = rtshark::Metadata::new(
            "quic.header_form".to_string(),                         // name
            "1".to_string(),                                        // show
            "1".to_string(),                                        // value
            "1... .... = Header Form: Long Header (1)".to_string(), // showname
            1,                                                      // size
            42,                                                     // pos
            false,                                                  // hide
        );
        let field = format_metadata_field(&metadata, 0, None, false).unwrap();
        assert_eq!("quic.header_form", field.name);
        assert_eq!(336, field.index);
        assert_eq!(
            1, field.length,
            "checking that the length of a bit field is correct"
        );
        assert_eq!("1", field.value);

        // Create metadata from the sample PDML.
        //     <field name="quic.long.reserved" showname=".... 00.. = Reserved: 0" size="1" pos="42" show="0" value="0" unmaskedvalue="cc"/>
        let metadata = rtshark::Metadata::new(
            "quic.long.reserved".to_string(),      // name
            "0".to_string(),                       // show
            "0".to_string(),                       // value
            ".... 00.. = Reserved: 0".to_string(), // showname
            1,                                     // size
            42,                                    // pos
            false,                                 // hide
        );
        let field = format_metadata_field(&metadata, 0, None, false).unwrap();
        assert_eq!(
            2, field.length,
            "checking that the length of a bit field is correct"
        );
        // Index is metadata position in bits (42 * 8 = 336) plus the bit field offset (4) minus
        // the index offset (0).
        assert_eq!(340, field.index);

        // Test the same with an index offset.
        let field = format_metadata_field(&metadata, 12, None, false).unwrap();
        assert_eq!(
            2, field.length,
            "checking that the length of a bit field is correct"
        );
        // Index is metadata position in bits (42 * 8 = 336) plus the bit field offset (4) minus
        // the index offset (12 * 8 = 96).
        assert_eq!(244, field.index);
    }
}
