// Copyright 2024 Two Six Technologies.

/// https://www.rfc-editor.org/rfc/rfc1889#section-5.1
pub struct Header {
    pub version: u8, // 2-bits
    pub padding: bool,
    pub extension: bool,
    pub contributing_source_identifiers_count: u8, // 4-bits
    pub marker: bool,
    pub payload_type: u8, // 7-bits
    pub sequence_number: u16,
    pub timestamp: u32,
    pub synchronization_source_identifier: u32,
}

impl Header {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut header = vec![];

        let mut first_byte: u8 = self.contributing_source_identifiers_count;
        first_byte = first_byte | (self.version << 6);
        if self.padding {
            first_byte = first_byte | 0b0010_0000;
        }
        if self.extension {
            first_byte = first_byte | 0b0001_0000;
        }
        header.push(first_byte);

        let mut second_byte: u8 = self.payload_type;
        if self.marker {
            second_byte = second_byte | 0b1000_0000;
        }
        header.push(second_byte);

        header.extend(self.sequence_number.to_be_bytes());
        header.extend(self.timestamp.to_be_bytes());
        header.extend(self.synchronization_source_identifier.to_be_bytes());

        if self.extension {
            // https://www.rfc-editor.org/rfc/rfc1889#section-5.3.1
            let header_extension_defined_by_profile: u16 = 0b1011_1110_1101_1110; // TODO: Hardcoded value observerd in PCAP. May want to vary this.
            header.extend(header_extension_defined_by_profile.to_be_bytes());

            let extension_length: u16 = 0;
            header.extend(extension_length.to_be_bytes());

            if extension_length > 0 {
                let extension_length = extension_length * 4; // Length is number of 32-byte words.
                let random_bytes: Vec<u8> = (0..extension_length).map(|_| rand::random()).collect();
                header.extend(random_bytes);
            }
        }

        header
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: 2,
            padding: false,
            extension: true,
            contributing_source_identifiers_count: 0,
            marker: false,
            payload_type: 109, // Seen a lot in sample captures, so using as default for now.
            sequence_number: 0, // Should initialize to random value.
            timestamp: 0,      // Should initialize to random value.
            synchronization_source_identifier: 0,
        }
    }
}

pub fn create_rtp_packet(size: usize) -> Vec<u8> {
    let rtp_header = Header {
        sequence_number: rand::random(),
        timestamp: rand::random(), // TODO: use a realish value.
        synchronization_source_identifier: rand::random(),
        ..Default::default()
    };
    let mut payload = rtp_header.to_vec();
    let size = size - payload.len();
    let random_bytes: Vec<u8> = (0..size).map(|_| rand::random()).collect();
    payload.extend(random_bytes);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_size() {
        let header = Header {
            extension: false,
            ..Default::default()
        };

        assert_eq!(
            header.to_vec().len(),
            12,
            "RTP header with no extensions should be 12 bytes."
        );

        let header = Header {
            extension: true,
            ..Default::default()
        };
        let header = header.to_vec();
        let extension_length = u16::from_be_bytes(header[14..16].try_into().unwrap());
        assert_eq!(
            extension_length, 0,
            "RTP header extension length was defaulted to zero. Has this changed?"
        ); // TODO: subject to change.
        let expected_length = 12 + 4 + (extension_length * 4) as usize;
        assert_eq!(header.to_vec().len(), expected_length, "RTP header length should be 12 bytes, plus 4 bytes for the extension header, plus the extension length (x4 since it counts 32 byte words.");
    }
}
