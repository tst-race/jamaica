// Copyright 2024 Two Six Technologies.

use std::env;
mod json_input;
use clap::{CommandFactory, Parser}; // Parsing command line args - https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_0/index.html
use log::info;
use std::fs::File;
use std::str::FromStr;

// mod client_control_input;
// use crate::client_control_input::ClientControlInput;

// use crate::quic_input::QuicInput;
// use motiflib::action::Action;
// use motiflib::motif_input::MotifInput;
// use motiflib::quic::QuicInput;

use const_format::formatcp;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
static LONG_VERSION: &str = formatcp!(
    "{} - built against function spec with hash 0x{:x}",
    CARGO_PKG_VERSION,
    motiflib::remora_control_message::FUNC_SPEC_FILE_HASH
);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(long_version = LONG_VERSION)]
struct Args {
    /// The input conversation file.
    input: String,
    /// If true, fail immediately if control messages do not fit within the embeddable packet
    /// payload. Otherwise, append the control message anyway and increase the packet size.
    /// Defaults to false.
    #[clap(long, action)]
    strict: bool,
    /// If true, just generate the packets, but don't actually send any traffic to the server. Can
    /// be useful for validation and check bandwidth without having to start a server instance.
    /// Defaults to false.
    #[clap(long, action)]
    dry_run: bool,
}

// CLIENT
fn main() -> std::io::Result<()> {
    // let input = JsonInput::new("input/sample.json");

    // Setup logging.
    // simple_logger::init_with_env().unwrap();
    let log_filename = "client.log";
    let rust_log_env_var = env::var("RUST_LOG").unwrap_or("Trace".into());
    let log_level = log::LevelFilter::from_str(rust_log_env_var.as_str())
        .unwrap_or_else(|err| panic!("invalid log level \"{}\" - {}", rust_log_env_var, err));
    let log_config = simplelog::ConfigBuilder::new()
        .set_target_level(log::LevelFilter::Error)
        .build();
    simplelog::CombinedLogger::init(vec![
        simplelog::SimpleLogger::new(log_level, log_config.clone()),
        simplelog::WriteLogger::new(
            log::LevelFilter::Trace,
            log_config,
            File::create(log_filename).unwrap(),
        ),
    ])
    .unwrap();

    info!("{}", Args::command().get_version().unwrap());

    // error!("client log level {}", log_level);
    // trace!("CLIENT TESTING");

    // proc_macro_motif::motif_code_generation!();
    // code_gen::motif_code_generation!();

    // TODO: pass into config via command line.
    // Get the absolute path of the input file.
    let mut dir = env::current_exe()?;
    dir.pop();
    // dir.push("../../client/input/webrtc-5.json");
    // dir.push("../../client/input/webrtc-7.json");
    // dir.push("../../client/input/quic-1.json");
    dir.push("../../client/input/quic-2.json");
    // dir.push("../../client/input/client_control_sample.json");
    // let mut input = ClientControlInput::new(dir.to_str().unwrap());
    // let mut input = QuicInput::new();

    let args = Args::parse();

    clientlib::run_input(args.input.as_str(), args.strict, args.dry_run)
    // clientlib::run_input(dir.to_str().unwrap())
}
