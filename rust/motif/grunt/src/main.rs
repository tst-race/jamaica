// Copyright 2024 Two Six Technologies.

use clap::{CommandFactory, Parser};
use const_format::formatcp;
use log::{debug, info, warn};
use motiflib::remora_control_message::FieldJson;
use motiflib::remora_control_message::{is_proto_field_supported, FUNC_SPEC_FILE_HASH};
use serde::ser::Serializer;
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use std::io::{BufWriter, Write};
use std::{clone, fs::File, io::Read};

// TODO: need a func spec for unit testing.
//       or can we override `is_proto_field_supported()` for testing?

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
static LONG_VERSION: &str = formatcp!(
    "{} - built against function spec with hash 0x{:x}",
    CARGO_PKG_VERSION,
    FUNC_SPEC_FILE_HASH
);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(long_version = LONG_VERSION)]
struct Args {
    /// The input Remora JSON file.
    input: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Field {
    /// The name of the field.
    name: String,
    /// The tshark display value. Should only be used for debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    display: Option<String>,
    /// The index (position) in bits of the field in the layer.
    index: usize,
    /// The length of the field in bits.
    length: usize,
    /// A hex string of the field value.
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

// TODO: this is copied from Remora. Turn this into a lib and depdend on it instead.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Packet {
    direction: String, // TODO: change to enum? YES
    raw: String,
    length: usize,
    packet_number: usize,
    frame_number: usize,
    timestamp: f64,
    // Size of the packet layer in bits.
    size_in_bits: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<u32>,
    fields: Vec<Field>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Session {
    src_addr: String,
    dst_addr: String,
    protocol: String,
    connection_id: u32,
    packets: Vec<Packet>,
}

impl Session {
    pub fn read_sessions_from_file(file_name: &str) -> Vec<Self> {
        let mut file = match File::open(file_name) {
            Ok(result) => result,
            Err(error) => panic!("Failed to open {}: {:?}", file_name, error),
        };

        let mut json_str = String::new();
        match file.read_to_string(&mut json_str) {
            Ok(result) => result,
            Err(err) => panic!("Error reading file '{:?}': {:?}", file_name, err),
        };

        match serde_json::from_str(&json_str) {
            Ok(result) => return result,
            Err(err) => panic!("Error parsing json: {:?}", err),
        };

        // println!("{:?}", json);

        // Self {
        //     src_addr: "".into(),
        //     dst_addr: "".into(),
        //     protocol: "".into(),
        //     connection_id: u32::MAX,
        //     packets: vec![],
        // }
    }
}

///
/// A field that has not been specified/understood by the ML pipeline.
///
#[derive(Debug, Serialize, PartialEq)]
struct RawField {
    /// Hex string of raw data.
    raw: String, // Vec<u8>
    /// Optional field name. Only to be used for debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_name: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
struct KnownField {
    /// The name of the field. Used for generation.
    field: String,
    /// Argument needed to generate the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    arg: Option<u64>,
}

#[derive(Debug, PartialEq)]
enum GruntField {
    Raw(RawField),
    Known(KnownField),
}

#[derive(Debug, Serialize)]
struct GruntPacket {
    direction: String,
    fields: Vec<GruntField>,
}

#[derive(Debug, Serialize)]
struct GruntOutput {
    protocol: String,
    packets: Vec<GruntPacket>,
}

impl Serialize for GruntField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // let mut seq = serializer.serialize_seq(Some(self.len()))?;
        // for e in self {
        //     seq.serialize_element(e)?;
        // }
        // seq.end()

        match self {
            GruntField::Raw(field) => field.serialize(serializer),
            GruntField::Known(field) => field.serialize(serializer),
        }
    }
}

fn refine_packet(packet: &Packet, unknown_field_warnings: &mut Vec<String>) -> Vec<GruntField> {
    // In the JSON, check that the raw fields is the same length as the fields, i.e. the last field
    // index plus the last field length should give you the total packet size, so compare it to the
    // length of the raw field. The raw field is just a hex dump of the entire packet. We need to
    // multiply the raw string length by four to get the length in bits.
    // debug_assert_eq!(packet.fields.last().unwrap().index + packet.fields.last().unwrap().length, packet.raw.len() * 4);

    // println!("{:?}", packet);
    // panic!("oh no");

    let mut new_packet = packet.clone();
    let mut fields = vec![];

    let mut raw_field_start_index = 0;
    let mut debug_unknown_fields = vec![];
    for field in new_packet.fields.iter_mut() {
        // println!("field: {:?}\n\n", field);

        let quic_bit_fields = vec![
            "quic.header_form",
            "quic.fixed_bit",
            "quic.long.packet_type",
            "quic.long.reserved",
            "quic.packet_number_length",
            "quic.unknown",
            "quic.token_length",
            "quic.spin_bit",
        ];

        // If the field is known then parse and add it to the output.
        // if is_proto_field_supported(field.name.as_str()) {
        // TODO: just debugging, starting with the easiest fields to generate.
        let name = field.name.as_str();
        // if name == "quic.payload" || name == "quic.remaining_payload" || name == "quic.version" || name == "srtp.enc_payload" {
        if is_proto_field_supported(&name) && !quic_bit_fields.contains(&name) {
            // // TODO: handle bit fields
            // assert_eq!(field.index % 8, 0);
            // assert_eq!(field.length % 8, 0);

            // If the raw index is less than the current index that means we have already seen some
            // unknown fields. Add the cumulative raw data to the output.
            if raw_field_start_index < field.index {
                // field.value = None;
                // Convert from bit indexes to hex string indexes.
                let start_index = raw_field_start_index / 4;
                let end_index = field.index / 4;
                debug_assert!(packet.raw.len() >= end_index, "packet {:?} index {} for field {} greater than length of raw packet with length {}: {}", packet, end_index, field.name, packet.raw.len(), packet.raw);
                let raw_data = packet.raw[start_index..end_index].to_string();
                fields.push(GruntField::Raw(RawField {
                    raw: raw_data,
                    debug_name: Some(debug_unknown_fields.join(", ")),
                }));
                debug_unknown_fields.clear();
                // fields.push(GruntField::Raw(RawField { raw: hex::decode(raw_data).expect("Decoding failed") }));
            }

            fields.push(GruntField::Known(KnownField {
                field: field.name.clone(),
                arg: Some(field.length as u64),
            }));

            raw_field_start_index = field.index + field.length;
        } else {
            debug_unknown_fields.push(field.name.clone());
            // Don't log duplicate warnings per field name.
            if !unknown_field_warnings.contains(&field.name) {
                unknown_field_warnings.push(field.name.clone());
                warn!("unknown field: {:?}", field);
            }
        }
        // // If the field is not known then just add raw data to the output.
        // else {
        //     // Convert from bit indexes to hex string indexes.
        //     let start_index = raw_field_start_index / 4;
        //     let end_index = field.index / 4;
        //     let raw_data = packet.raw[start_index..end_index].to_string();
        //     fields.push(GruntField::Raw(RawField { raw: raw_data }));
        //     // fields.push(GruntField::Raw(RawField { raw: hex::decode(raw_data).expect("Decoding failed") }));
        // }
    }

    // Get whatever is left.

    if raw_field_start_index < packet.raw.len() * 4 {
        let start_index = raw_field_start_index / 4;
        let raw = packet.raw[start_index..].to_string();
        fields.push(GruntField::Raw(RawField {
            raw,
            debug_name: Some(debug_unknown_fields.join(", ")),
        }));
    }

    // if fields.is_empty() {
    //     debug!("was not able to refine packet");
    //     fields.push(GruntField::Raw(RawField { value: hex::decode(packet.raw.clone()).expect("Decoding failed") }));
    // }

    // println!("{:?}", fields);

    fields
}

fn main() {
    // Setup logging.
    simple_logger::init_with_env().unwrap();

    info!("{}", Args::command().get_version().unwrap());

    // let result = is_proto_field_supported("quic.payload");

    let args = Args::parse();

    // let sessions = Session::read_sessions_from_file("input/test.json");
    let sessions = Session::read_sessions_from_file(args.input.as_str());

    let mut grunt_output = vec![];
    // TODO: fix this mess
    let mut unknown_field_warnings = vec![];

    for session in sessions {
        let mut output = GruntOutput {
            protocol: session.protocol,
            packets: vec![],
        };
        for packet in session.packets {
            let refined_packet = refine_packet(&packet, &mut unknown_field_warnings);

            let direction = packet.direction;
            output.packets.push(GruntPacket {
                direction,
                fields: refined_packet,
            });

            // break; //debug
        }
        grunt_output.push(output);
    }

    // println!("\n\n\n{:?}", refined_packet);

    // println!("{:?}", sessions);

    let formatted = serde_json::to_string_pretty(&grunt_output).expect("failed to prettify JSON");
    let output_filename = "grunt-output.json";
    info!("writing output to {}", output_filename);
    let file = std::fs::File::create(output_filename).unwrap();
    let mut writer = BufWriter::new(file);

    writer
        .write_all(formatted.as_bytes())
        .expect("failed to save JSON to file");

    // let formatted = serde_json::to_string_pretty(&sessions).expect("failed to prettify JSON");
    // let file = std::fs::File::create("grunt-output.json").unwrap();
    // let mut writer = BufWriter::new(file);

    // writer.write_all(formatted.as_bytes()).expect("failed to save JSON to file");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refine_packet_test() {}

    #[test]
    fn refine_packet_handles_unknown_fields() {
        let raw = String::from("abcdef012345");
        let packet = Packet {
            direction: "from_src".into(),
            length: raw.len() / 2, // hex string, so every two chars in one byte
            size_in_bits: raw.len() / 4, // hex string, so every char is 4 bits
            raw,
            packet_number: 0,
            frame_number: 0,
            timestamp: 0.0,
            position: None,
            fields: vec![
                Field {
                    name: "proto1.field1".into(),
                    display: None,
                    index: 0,
                    length: 24,
                    value: Some("abcdef".into()),
                },
                Field {
                    name: "proto1.field2".into(),
                    display: None,
                    index: 24,
                    length: 24,
                    value: Some("012345".into()),
                },
            ],
        };

        let mut unknown_field_warnings = vec![];
        let result = refine_packet(&packet, &mut unknown_field_warnings);

        assert_eq!(result.len(), 1);
        let field = &result[0];
        assert!(matches!(field, GruntField::Raw { .. }));
        if let GruntField::Raw(raw_field) = field {
            assert_eq!(raw_field.raw.len(), 12);
        } else {
            panic!("this should not be reachable...");
        }
    }

    #[test]
    fn refine_packet_handles_known_and_unknown_fields() {
        let raw = String::from("abcdef012345fedcba");
        let packet = Packet {
            direction: "from_src".into(),
            length: raw.len() / 2, // hex string, so every two chars in one byte
            size_in_bits: raw.len() / 4, // hex string, so every char is 4 bits
            raw,
            packet_number: 0,
            frame_number: 0,
            timestamp: 0.0,
            position: None,
            fields: vec![
                Field {
                    name: "proto1.field1".into(),
                    display: None,
                    index: 0,
                    length: 24,
                    value: Some("abcdef".into()),
                },
                Field {
                    name: "test_proto.payload".into(), // TODO: use a func spec for testing
                    display: None,
                    index: 24,
                    length: 24,
                    value: Some("012345".into()),
                },
                Field {
                    name: "proto1.field2".into(),
                    display: None,
                    index: 48,
                    length: 24,
                    value: Some("fedcba".into()),
                },
            ],
        };

        let mut unknown_field_warnings = vec![];
        let result = refine_packet(&packet, &mut unknown_field_warnings);

        assert_eq!(result.len(), 3);

        let field = &result[0];
        assert!(matches!(field, GruntField::Raw { .. }));
        if let GruntField::Raw(raw_field) = field {
            assert_eq!(raw_field.raw.as_str(), "abcdef");
        } else {
            panic!("this should not be reachable...");
        }

        let field = &result[1];
        assert!(matches!(field, GruntField::Known { .. }));
        if let GruntField::Known(known_field) = field {
            assert_eq!(known_field.field.as_str(), "test_proto.payload");
            assert_eq!(known_field.arg.unwrap(), 24);
        } else {
            panic!("this should not be reachable...");
        }

        let field = &result[2];
        assert!(matches!(field, GruntField::Raw { .. }));
        if let GruntField::Raw(raw_field) = field {
            assert_eq!(raw_field.raw.as_str(), "fedcba");
        } else {
            panic!("this should not be reachable...");
        }
    }

    #[test]
    fn refine_packet_handles_bit_fields() {
        let raw = String::from("59");
        let packet = Packet {
            direction: "from_src".into(),
            length: raw.len() / 2, // hex string, so every two chars in one byte
            size_in_bits: raw.len() / 4, // hex string, so every char is 4 bits
            raw,
            packet_number: 0,
            frame_number: 0,
            timestamp: 0.0,
            position: None,
            fields: vec![
                Field {
                    name: "test_proto.unknown".into(),
                    display: None,
                    index: 0,
                    length: 3,
                    value: Some("2".into()),
                },
                Field {
                    name: "test_proto.payload".into(),
                    display: None,
                    index: 3,
                    length: 5,
                    value: Some("19".into()),
                },
            ],
        };

        let mut unknown_field_warnings = vec![];
        let result = refine_packet(&packet, &mut unknown_field_warnings);

        assert_eq!(result.len(), 2);
        let field = &result[0];
        assert!(matches!(field, GruntField::Raw { .. }));
        if let GruntField::Raw(raw_field) = field {
            assert_eq!(raw_field.raw.as_str(), "2");
        } else {
            panic!("this should not be reachable...");
        }

        let field = &result[1];
        assert!(matches!(field, GruntField::Known { .. }));
        if let GruntField::Known(known_field) = field {
            assert_eq!(known_field.field.as_str(), "quic.payload");
            assert_eq!(known_field.arg.unwrap(), 5);
        } else {
            panic!("this should not be reachable...");
        }
    }
}
