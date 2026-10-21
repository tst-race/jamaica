// Copyright 2024 Two Six Technologies.

use crate::quic::get_random_hex_string;

pub struct QuicClient {
    pub destination_connection_id: String, // TODO: or should we use Vec<u8> ???
    pub source_connection_id: String,      // TODO: or should we use Vec<u8> ???
}

impl QuicClient {
    pub fn new() -> Self {
        const SOURCE_CONNECTION_ID_SIZE: usize = 8; // TODO: make this configurable? Instagram uses 3.
        QuicClient {
            destination_connection_id: "".to_string(),
            source_connection_id: get_random_hex_string(SOURCE_CONNECTION_ID_SIZE),
        }
    }

    pub fn set_destination_connection_id(&mut self, destination_connection_id: String) {
        self.destination_connection_id = destination_connection_id;
    }
}
