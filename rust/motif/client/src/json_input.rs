// Copyright 2024 Two Six Technologies.

use serde::Deserialize;
use std::{fs::File, io::Read};

/// The content to be sent in a packet over a given connection.
#[derive(Debug)]
pub struct Content(pub Vec<u8>);

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex_string = String::deserialize(deserializer)?;
        let hex_bytes = hex::decode(&hex_string).map_err(serde::de::Error::custom)?;
        Ok(Content(hex_bytes))
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonPacket {
    pub size: u32,
    pub content: Content,
}

#[derive(Debug, Deserialize)]
pub struct JsonInput {
    pub packets: Vec<JsonPacket>,
}

impl JsonInput {
    #[allow(dead_code)]
    pub fn new(file_name: &str) -> Self {
        let mut file = match File::open(file_name) {
            Ok(result) => result,
            Err(error) => panic!("{:?}", error),
        };

        let mut json_str = String::new();
        match file.read_to_string(&mut json_str) {
            Ok(result) => result,
            Err(err) => panic!("Error reading file '{:?}': {:?}", file_name, err),
        };

        let json: Self = match serde_json::from_str(&json_str) {
            Ok(result) => result,
            Err(err) => panic!("Error parsing json: {:?}", err),
        };

        println!("{:?}", json);

        json
    }
}
