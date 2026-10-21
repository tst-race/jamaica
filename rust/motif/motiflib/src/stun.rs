// Copyright 2024 Two Six Technologies.

/// STUN Network Version: RFC-5389/8489 (3)
/// https://www.rfc-editor.org/rfc/rfc5389
/// https://www.rfc-editor.org/rfc/rfc8445.html
extern crate derivative;
use derivative::Derivative;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

// TODO: remove logging from here. Log at higher level.
use log::{debug, error, info, warn};

/// The Magic Cookie field MUST contain the fixed value 0x2112A442 in network byte order.
const MAGIC_COOKIE_VALUE: u32 = 0x2112A442;

/// https://www.rfc-editor.org/rfc/rfc8489.html#section-5
#[derive(Debug, Derivative)]
#[derivative(Default)]
pub struct Header {
    ///  0x0001  :  Binding Request
    ///  0x0101  :  Binding Response
    ///  0x0111  :  Binding Error Response
    ///  0x0002  :  Shared Secret Request
    ///  0x0102  :  Shared Secret Response
    ///  0x0112  :  Shared Secret Error Response
    pub message_type: u16,
    /// count, in bytes, of the size of the message, not including the 20 byte header.
    pub message_length: u16,
    #[derivative(Default(value = "MAGIC_COOKIE_VALUE"))]
    pub magic_cookie: u32,
    pub transaction_id: [u8; 12],
}

impl Header {
    fn to_vec(&self) -> Vec<u8> {
        let mut header = vec![];

        header.extend(self.message_type.to_be_bytes());

        header.extend(self.message_length.to_be_bytes());

        header.extend(self.magic_cookie.to_be_bytes());

        header.extend(self.transaction_id);

        header
    }
}

/// https://www.rfc-editor.org/rfc/rfc5389#section-15
#[derive(Default)]
pub struct Attribute {
    /// https://www.iana.org/assignments/stun-parameters/stun-parameters.xhtml#stun-parameters-4
    /// 0x0000: (Reserved)
    /// 0x0001: MAPPED-ADDRESS
    /// 0x0002: (Reserved; was RESPONSE-ADDRESS)
    /// 0x0003: (Reserved; was CHANGE-ADDRESS)
    /// 0x0004: (Reserved; was SOURCE-ADDRESS)
    /// 0x0005: (Reserved; was CHANGED-ADDRESS)
    /// 0x0006: USERNAME
    /// 0x0007: (Reserved; was PASSWORD)
    /// 0x0008: MESSAGE-INTEGRITY
    /// 0x0009: ERROR-CODE
    /// 0x000A: UNKNOWN-ATTRIBUTES
    /// 0x000B: (Reserved; was REFLECTED-FROM)
    /// 0x0014: REALM
    /// 0x0015: NONCE
    /// 0x0020: XOR-MAPPED-ADDRESS
    /// 0x0024 	PRIORITY
    /// 0x8022: SOFTWARE
    /// 0x8023: ALTERNATE-SERVER
    /// 0x8028: FINGERPRINT
    pub attribute_type: u16,
    pub length: u16,
    pub value: Vec<u8>,
}

impl Attribute {
    fn to_vec(&self) -> Vec<u8> {
        debug_assert!(self.value.len() == self.length as usize);

        let mut attribute = vec![];

        attribute.extend(self.attribute_type.to_be_bytes());

        attribute.extend(self.length.to_be_bytes());

        attribute.extend(self.value.clone());

        // Since STUN aligns attributes on 32-bit boundaries, attributes whose contentis not a
        // multiple of 4 bytes are padded with 1, 2, or 3 bytes of padding so that its value
        // contains a multiple of 4 bytes.  The padding bits are ignored, and may be any value.
        const BYTE_ALIGNMENT: usize = 4;
        let remainder = self.length as usize % BYTE_ALIGNMENT;
        if remainder > 0 {
            let padding = BYTE_ALIGNMENT - remainder;
            attribute.extend(vec![0; padding]);
        }

        attribute
    }

    fn username() -> Self {
        let length = 35; // TODO: copied value from PCAP.

        /// From RFC; [username] MUST contain a UTF-8-encoded [RFC3629] sequence of fewer than 509
        /// bytes and MUST have been processed using the OpaqueString profile [RFC8265].
        // TODO: opaque string profile = https://www.rfc-editor.org/rfc/rfc8265#section-4.2
        // TODO: clean up this mess.
        // let value: Vec<char> = rand::thread_rng()
        //     .sample_iter::<char, _>(rand::distributions::Standard)
        //     .take(length as usize)
        //     .collect();
        // let value: Vec<u8> = value.iter().map(|c| *c as u8).collect::<Vec<_>>();
        Self {
            attribute_type: 0x0006,
            length,
            // value: (0..length).map(|_| rand::random()).collect(),
            // value,
            value: hex::decode(
                "2d4d5159477558763871365869676f4b4141694b4169414245413a3766666532396130",
            )
            .expect("Decoding failed"), // TODO: hardcoded from PCAP for now. Change to be random/dynamic
        }
    }

    fn use_candidate() -> Self {
        Self {
            attribute_type: 0x0025,
            length: 0,
            ..Default::default()
        }
    }

    fn priority() -> Self {
        // TODO: use the formula documented here - https://www.rfc-editor.org/rfc/rfc8445.html#section-5.1.2
        let length = 4;
        Self {
            attribute_type: 0x0024,
            length,
            // value: (0..length).map(|_| rand::random()).collect(),
            value: 0x6d7f80ff_u32.to_be_bytes().to_vec(), // TODO: hardcoded from PCAP
        }
    }

    fn ice_controlling() -> Self {
        let length = 8;
        Self {
            attribute_type: 0x802a,
            length,
            value: (0..length).map(|_| rand::random()).collect(),
        }
    }

    fn message_integrity() -> Self {
        let length = 20;
        Self {
            attribute_type: 0x0008,
            length,
            value: (0..length).map(|_| rand::random()).collect(),
        }
    }

    fn fingerprint() -> Self {
        // TODO: this is totally wrong, obviously.
        // https://www.rfc-editor.org/rfc/rfc8489.html#section-14.7
        // https://www.rfc-editor.org/rfc/rfc1952#section-8
        // https://docs.rs/crc/latest/crc/
        let length = 4;
        Self {
            attribute_type: 0x8028,
            length,
            value: (0..length).map(|_| rand::random()).collect(),
        }
    }
}

pub struct StunPacket {}

impl StunPacket {
    pub fn create_binding_request() -> Vec<u8> {
        let mut stun_packet = vec![];

        // TCP frame length.
        // TODO: should this be added here or at a higher level?
        stun_packet.push(0);
        stun_packet.push(0x74);

        stun_packet.extend(
            Header {
                message_type: 0x0001,
                message_length: 96,
                transaction_id: (0..12)
                    .map(|_| rand::random())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .expect("oh no..."),
                ..Default::default()
            }
            .to_vec(),
        );

        stun_packet.extend(Attribute::username().to_vec());
        stun_packet.extend(Attribute::use_candidate().to_vec());
        stun_packet.extend(Attribute::priority().to_vec());
        stun_packet.extend(Attribute::ice_controlling().to_vec());
        stun_packet.extend(Attribute::message_integrity().to_vec());
        stun_packet.extend(Attribute::fingerprint().to_vec());

        stun_packet
    }
    pub fn create_binding_success_response() -> Vec<u8> {
        let mut stun_packet = vec![];

        // TCP frame length.
        // TODO: should this be added here or at a higher level?
        stun_packet.push(0);
        stun_packet.push(0x74);

        stun_packet.extend(
            Header {
                message_type: 0x0101,
                message_length: 96,
                transaction_id: (0..12)
                    .map(|_| rand::random())
                    .collect::<Vec<u8>>()
                    .try_into()
                    .expect("oh no..."),
                ..Default::default()
            }
            .to_vec(),
        );

        // stun_packet.extend(Attribute::xor_mapped_address().to_vec());
        stun_packet.extend(Attribute::message_integrity().to_vec());
        stun_packet.extend(Attribute::fingerprint().to_vec());

        stun_packet
    }

    pub fn is_stun_binding_request(payload: &Vec<u8>) -> bool {
        // First two bits should both be zero.
        let first_byte = payload[0];

        debug!("first byte = {}", first_byte);

        // if first_byte & 0b1100_0000 != 0 {
        //     return false;
        // }

        // Check that is the request message type.
        // https://www.rfc-editor.org/rfc/rfc5389#appendix-A
        let first_two_bytes = u16::from_be_bytes(payload[0..2].try_into().unwrap());
        debug!("first two bytes = {}", first_two_bytes);
        if first_two_bytes & 0x0110 != 0x0000 {
            return false;
        }

        let magic_cookie = u32::from_be_bytes(payload[4..8].try_into().unwrap());
        if magic_cookie != MAGIC_COOKIE_VALUE {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size() {
        let header = Header::default();

        assert_eq!(
            header.to_vec().len(),
            20,
            "STUN header should ALWAYS be 20 bytes"
        );
    }

    #[test]
    fn header_magic_cookie_value() {
        assert_eq!(Header::default().magic_cookie, MAGIC_COOKIE_VALUE);
    }

    #[test]
    fn stun_packet_magic_cookie_value() {
        let stun_packet = StunPacket::create_binding_request();

        assert_eq!(
            u32::from_be_bytes(stun_packet[6..10].try_into().expect("oh no...")),
            MAGIC_COOKIE_VALUE
        );
    }

    #[test]
    fn attribute_size() {
        let attribute = Attribute::default();

        assert_eq!(attribute.to_vec().len(), 4);

        let length = 1234;
        let attribute = Attribute {
            length,
            value: vec![0; length as usize],
            ..Default::default()
        };

        // Expected size is length (1234) + padding (2) + header size (4)
        // Padding is 2 to align value field on 4 byte boundary.
        assert_eq!(attribute.to_vec().len(), 1234 + 2 + 4);

        let length = 1;
        let attribute = Attribute {
            length,
            value: vec![0; length as usize],
            ..Default::default()
        };
        // Padding should be 3.
        assert_eq!(attribute.to_vec().len(), 8);

        let length = 2;
        let attribute = Attribute {
            length,
            value: vec![0; length as usize],
            ..Default::default()
        };
        // Padding should be 2.
        assert_eq!(attribute.to_vec().len(), 8);

        let length = 3;
        let attribute = Attribute {
            length,
            value: vec![0; length as usize],
            ..Default::default()
        };
        // Padding should be 1.
        assert_eq!(attribute.to_vec().len(), 8);

        let length = 4;
        let attribute = Attribute {
            length,
            value: vec![0; length as usize],
            ..Default::default()
        };
        // Padding should be 0.
        assert_eq!(attribute.to_vec().len(), 8);
    }
}
