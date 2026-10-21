// Copyright 2024 Two Six Technologies.

// DTLS v1.2 - https://www.rfc-editor.org/rfc/rfc6347
// DTLS is based off of TLS - https://www.rfc-editor.org/rfc/rfc5246.html

/// https://www.rfc-editor.org/rfc/rfc6347#section-4.1
pub struct RecordLayer {
    pub content_type: u8,
    pub version: u16,
    pub epoch: u16,
    pub sequence_number: u64, // only 48 bits
    pub length: u16,
}

impl RecordLayer {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut record_layer = vec![];

        record_layer.push(self.content_type);

        record_layer.extend(self.version.to_be_bytes());

        record_layer.extend(self.epoch.to_be_bytes());

        debug_assert!(self.sequence_number < 0xffffffffffff); // TODO: or should we panic?
        let sequence_number: Vec<u8> = self.sequence_number.to_be_bytes()[..6].try_into().unwrap();
        record_layer.extend(sequence_number);

        record_layer.extend(self.length.to_be_bytes());

        record_layer
    }
}

impl Default for RecordLayer {
    fn default() -> Self {
        // TODO: provide reasonable defaults and comments.
        Self {
            content_type: 0,
            version: 0,
            epoch: 0,
            sequence_number: 0,
            length: 0,
        }
    }
}

pub struct ApplicationData {
    content_type: u8,
    version: u16,
    epoch: u16,
    pub sequence_number: u64, // only 48 bits
    length: u16,
}

impl Default for ApplicationData {
    fn default() -> Self {
        Self {
            content_type: 23,
            version: 0xfefd,
            epoch: 1, // TODO: may need/want to vary this.
            sequence_number: 0,
            length: 0,
        }
    }
}

pub struct HandshakeHeader {}

pub fn create_client_hello(size: usize) -> Vec<u8> {
    let record_layer = RecordLayer {
        content_type: 22,
        version: 0xfeff,
        length: size as u16,
        ..Default::default()
    };

    let mut payload = record_layer.to_vec();
    let size = size - payload.len();
    let mut random_bytes: Vec<u8> = (0..size).map(|_| rand::random()).collect();

    // TODO: hacking the Handshake Protocol Client Hello. Improve this, read the RFC.
    // Type
    random_bytes[0] = 1; // Client hello

    // Length
    random_bytes[1] = 0;
    random_bytes[2] = 0;
    random_bytes[3] = 164; // TODO: compute actual size

    // Message sequence
    random_bytes[4] = 0;
    random_bytes[5] = 0;

    // Fragment offset
    random_bytes[6] = 0;
    random_bytes[7] = 0;
    random_bytes[8] = 0;

    // Fragment length
    random_bytes[9] = 0;
    random_bytes[10] = 0;
    random_bytes[11] = 164;

    // Version = DTLS v1.2
    random_bytes.splice(12..13, 0xfefd_u16.to_be_bytes());
    payload.extend(random_bytes);

    payload
}

pub fn create_server_hello(size: usize) -> Vec<u8> {
    let record_layer = RecordLayer {
        content_type: 22,
        version: 0xfeff,
        length: size as u16,
        ..Default::default()
    };

    let mut payload = record_layer.to_vec();
    let size = size - payload.len();
    let mut random_bytes: Vec<u8> = (0..size).map(|_| rand::random()).collect();

    // TODO: hacking the Handshake Protocol Client Hello. Improve this, read the RFC.
    // Type
    random_bytes[0] = 2; // Server hello

    // Length
    random_bytes[1] = 0;
    random_bytes[2] = 0;
    random_bytes[3] = 164; // TODO: compute actual size

    // Message sequence
    random_bytes[4] = 0;
    random_bytes[5] = 0;

    // Fragment offset
    random_bytes[6] = 0;
    random_bytes[7] = 0;
    random_bytes[8] = 0;

    // Fragment length
    random_bytes[9] = 0;
    random_bytes[10] = 0;
    random_bytes[11] = 164;

    // Version = DTLS v1.2
    random_bytes.splice(12..13, 0xfefd_u16.to_be_bytes());
    payload.extend(random_bytes);

    payload
}

pub fn create_application_data(size: usize) -> Vec<u8> {
    let application_data = ApplicationData {
        length: size as u16,
        ..Default::default()
    };

    let mut payload = vec![];

    payload.push(application_data.content_type);

    payload.extend(application_data.version.to_be_bytes());

    payload.extend(application_data.epoch.to_be_bytes());

    debug_assert!(application_data.sequence_number < 0xffffffffffff); // TODO: or should we panic?
    let sequence_number: Vec<u8> = application_data.sequence_number.to_be_bytes()[..6]
        .try_into()
        .unwrap();
    payload.extend(sequence_number);

    // The length field itself takes up two bytes, so subtract 2.
    let length: u16 = (size - payload.len() - 2) as u16;
    // From RFC: As in TLS 1.2, the length should not exceed 2^14.
    debug_assert!(length <= 2_u16.pow(14));

    payload.extend(length.to_be_bytes());

    let mut random_bytes: Vec<u8> = (0..length).map(|_| rand::random()).collect();
    // TODO: not sure what this is, some application data header? Observed for all application data
    //       packets in a google meet (chrome) capture, but can't find any reference to it in
    //       either the DTLS or TLS RFC.
    random_bytes.splice(0..7, vec![00, 01, 00, 00, 00, 00, 00]);
    payload.extend(random_bytes);

    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_layer_size() {
        let record_layer = RecordLayer::default();

        assert_eq!(record_layer.to_vec().len(), 13);
    }

    #[test]
    fn record_layer_sequence_number_length() {
        let record_layer = RecordLayer {
            sequence_number: u64::MAX,
            ..Default::default()
        };

        // Should trigger a debug_assert
        // record_layer.to_vec();
    }

    #[test]
    fn application_data_size() {
        let app_data = create_application_data(100);

        assert_eq!(app_data.len(), 100);
    }

    #[test]
    fn header_size() {
        // let header = Header {
        //     extension: false,
        //     ..Default::default()
        // };

        // assert_eq!(
        //     header.to_vec().len(),
        //     12,
        //     "RTP header with no extensions should be 12 bytes."
        // );

        // let header = Header {
        //     extension: true,
        //     ..Default::default()
        // };
        // let header = header.to_vec();
        // let extension_length = u16::from_be_bytes(header[14..16].try_into().unwrap());
        // assert_eq!(
        //     extension_length, 0,
        //     "RTP header extension length was defaulted to zero. Has this changed?"
        // ); // TODO: subject to change.
        // let expected_length = 12 + 4 + (extension_length * 4) as usize;
    }
}
