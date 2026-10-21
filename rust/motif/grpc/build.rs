// Copyright 2024 Two Six Technologies.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tonic_build::compile_protos("protos/ta1_sensor.proto")?;
    // WARNING: if there is no package name in the proto then the output file will be named `_.rs`.
    tonic_build::configure()
        .build_server(true)
        // .out_dir("./src")
        .compile(&["protos/ta1_sensor.proto"], &["."])?;
    Ok(())
}
