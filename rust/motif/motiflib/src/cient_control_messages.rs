// Copyright 2024 Two Six Technologies.

use crate::quic::VariableLengthEncode;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};

// TODO: move this somewhere else. Where? Not sure. Probably need to rethink how to define content
//       type. Should be dynamic based on what content generators are loaded, but at the same time
//       needs to be consistent to avoid client/server versioning mismatches. Need to think about
//       this more.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, FromPrimitive)]
// #[serde(rename_all = "snake_case")]
pub enum ContentType {
    Random,
    #[serde(rename = "quic_initial")]
    QUICInitial,
    #[serde(rename = "quic_handshake")]
    QUICHandshake,
    #[serde(rename = "quic_protected_payload")]
    QUICProtectedPayload,
    // #[serde(rename = "rtp")]
    RTP,
    // #[serde(rename = "rtcp")]
    RTCP,
    #[serde(rename = "DTLSv1.2_client_hello")]
    DTLSv1_2ClientHello,
    #[serde(rename = "DTLSv1.2_server_hello")]
    DTLSv1_2ServerHello,
    #[serde(rename = "DTLSv1.2_application_data")]
    DTLSv1_2ApplicationData,
    STUN,
    #[serde(rename = "STUN_binding_request")]
    STUNBindingRequest,
    #[serde(rename = "STUN_binding_success_response")]
    STUNBindingSuccessResponse,
}

impl ContentType {
    pub fn from_u64(value: u64) -> Option<Self> {
        num::FromPrimitive::from_u64(value)
    }
}

#[derive(Debug, PartialEq)]
pub struct ClientControlMessage {
    pub responses: Vec<(u64, u16)>,
}

impl ClientControlMessage {
    pub fn to_vec(&self) -> Vec<u8> {
        // if self.responses.len() == 0 {
        //     return vec![0x00];
        // }

        let mut result = vec![];

        let num_responses = self.responses.len() as u8; // .to_variable_length_integer();
        result.push(num_responses);

        for response in &self.responses {
            result.extend(response.0.to_variable_length_integer());
            result.extend(response.1.to_be_bytes());
        }

        result.reverse();

        result
    }

    pub fn from_vec(vec: &Vec<u8>) -> Result<Self, &str> {
        debug_assert!(vec.len() > 0);
        let mut responses = vec![];

        // MINOR TODO: do this without copying
        let mut vec = vec.clone();
        vec.reverse();

        // TODO: variable length integer decode
        let num_responses = vec[0];
        if vec.len() < num_responses as usize {
            return Err("invalid control message");
        }

        // Assume any requested response size larger than this is an error.
        const UDP_MTU: u16 = 1500;

        let mut index = 1;
        for _ in 0..num_responses {
            if vec.len() <= index {
                return Err("invalid control message");
            }
            let content_type = decode_variable_length_integer(vec[index..].to_vec().as_ref());
            index += get_integer_size(content_type);
            if vec.len() < index + 2 {
                return Err("invalid control message");
            }
            let response_size = u16::from_be_bytes(vec[index..index + 2].try_into().unwrap());
            if response_size > UDP_MTU {
                // TODO: check with Paul, valid assumption?
                return Err("invalid control message");
            }
            index += 2;

            responses.push((content_type, response_size));
        }

        Ok(Self { responses })
    }
}

fn decode_variable_length_integer(encoded: &Vec<u8>) -> u64 {
    debug_assert!(encoded.len() > 0);
    let first_byte_masked = encoded[0] & 0b1100_0000;
    let mut encoded = encoded.clone();
    if first_byte_masked == 0 {
        return encoded[0] as u64;
    } else if first_byte_masked == 0b0100_0000 {
        encoded[0] &= 0b0011_1111;
        return u16::from_be_bytes(encoded[..2].try_into().unwrap()) as u64;
    } else if first_byte_masked == 0b1000_0000 {
        encoded[0] &= 0b0011_1111;
        return u32::from_be_bytes(encoded[..4].try_into().unwrap()) as u64;
    } else {
        encoded[0] &= 0b0011_1111;
        return u64::from_be_bytes(encoded[..8].try_into().unwrap());
    }
}

fn get_integer_size(input: u64) -> usize {
    if input <= u8::MAX as u64 {
        return 1;
    } else if input <= u16::MAX as u64 {
        return 2;
    } else if input <= u32::MAX as u64 {
        return 4;
    } else {
        return 8;
    }
}

// impl From<JsonPacket> for Packet {
//     fn from(json_packet: JsonPacket) -> Self {

//         ///
//         /// NEW SPEC: FOOTER
//         /// Footer {
//         ///     Num Responses (8), // TODO: also variable size? Would we want more than 256 packets in streaming case?
//         ///     Content Type (i), // variable size integer.
//         ///     Response Size (16), // Can probably make this smaller, but is it worth it?
//         ///     (..) // Optional additional (type, size) tuples
//         /// }
//         ///
//         /// Minimum size 4 bytes
//         /// TODO: byte ordering? Given that this is a footer, we'll need to append it in reverse right?

//         // let payload: Vec<u8> = vec![];

//         // TODO: validate that response size and message size are less than system MTU.
//         // TODO: how to get system MTU?

//         // debug_assert!(json_packet.response_size < 2usize.pow(15));

//         let response_size: usize = 0;//json_packet.response_size as u16;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_message_size() {
        let control_message = ClientControlMessage {
            responses: vec![(1, 64)],
        };

        assert_eq!(control_message.to_vec().len(), 4);
    }

    #[test]
    fn from_vec() {
        // This is a footer, so it's in reverse order.
        let control_message_vector = vec![
            64, // message size (cont.)
            0,  // message size
            4,  // content type
            1,  // number of responses
        ];

        let control_message =
            ClientControlMessage::from_vec(&control_message_vector).expect("oh no");

        assert_eq!(control_message.responses.len(), 1);
        assert_eq!(control_message.responses[0].0, 4);
        assert_eq!(control_message.responses[0].1, 64);

        let control_message_vector = vec![
            0,   // message size (cont.)
            1,   // message size
            3,   // content type
            255, // message size (cont.)
            0,   // message size
            2,   // content type
            64,  // message size (cont.)
            0,   // message size
            1,   // content type
            3,   // number of responses
        ];

        let control_message =
            ClientControlMessage::from_vec(&control_message_vector).expect("oh no");

        assert_eq!(control_message.responses.len(), 3);
        assert_eq!(control_message.responses[0].0, 1);
        assert_eq!(control_message.responses[0].1, 64);
        assert_eq!(control_message.responses[1].0, 2);
        assert_eq!(control_message.responses[1].1, 255);
        assert_eq!(control_message.responses[2].0, 3);
        assert_eq!(control_message.responses[2].1, 256);
    }

    #[test]
    fn to_vec_no_responses() {
        // If there are no responses then the control message will be converted to a byte vector
        // containing a single byte of value zero.
        // TODO: this is a pretty big fingerprint, so think of something better.
        let control_message = ClientControlMessage { responses: vec![] };

        let control_message = control_message.to_vec();
        assert_eq!(control_message.len(), 1);
        assert_eq!(control_message[0], 0x00);
    }

    #[test]
    fn from_vec_invalid_message() {
        // If the message is invalid then the function should return an error.
        let control_message_vector = vec![
            1, // number of responses is invalid since there is nothing after it
        ];

        let control_message = ClientControlMessage::from_vec(&control_message_vector);

        assert_eq!(control_message, Err("invalid control message"));

        let control_message_vector = vec![
            1, // content type is invalid since there is nothing after it
            1, // number of responses
        ];

        let control_message = ClientControlMessage::from_vec(&control_message_vector);

        assert_eq!(control_message, Err("invalid control message"));
    }
}
