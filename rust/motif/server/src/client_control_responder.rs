// Copyright 2024 Two Six Technologies.

use rand;

// TODO: move into common library for client and server.
#[derive(Debug)]
pub struct Content(pub Vec<u8>);

pub struct ClientControlResponder {
    pub response: Option<Content>,
}

impl From<Vec<u8>> for ClientControlResponder {
    fn from(content: Vec<u8>) -> Self {
        debug_assert!(content.len() >= 5);

        let response_size = u16::from_be_bytes(
            content[..2]
                .try_into()
                .expect("failed to get response size"),
        );

        if response_size == 0 {
            return Self { response: None };
        }

        let _content_type = content[2];

        // Create the response.
        // TODO: should message size include the header information? i.e. create message_size - header_size bytes of message data???
        let message: Vec<u8> = (0..response_size).map(|_| rand::random()).collect();

        Self {
            response: Some(Content(message)),
        }
    }
}
