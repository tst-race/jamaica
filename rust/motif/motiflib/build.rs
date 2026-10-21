// Copyright 2024 Two Six Technologies.

fn main() {
    match prost_build::compile_protos(
        &[
            // "protos/client_control_header.proto",
            "protos/client_control_message.proto",
        ],
        &["protos/"],
    ) {
        Ok(_) => (),
        Err(err) => {
            panic!("failed to generate protobuf: {}", err);
        }
    }
}
