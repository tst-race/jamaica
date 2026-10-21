// Copyright 2024 Two Six Technologies.

use clap::Parser; // Parsing command line args - https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_0/index.html
use dialoguer::Confirm;
use log::{debug, error, info, log_enabled};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::str::FromStr;

mod connection_tuple;
mod generic_parser;
use generic_parser::GenericParser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The input PCAP file.
    input: String,

    /// The directory where output files are stored.
    #[arg(short, long, default_value = "output")]
    output: String,

    /// A list of protocols to parse. Will parse all protocols by default.
    #[arg(long, value_delimiter = ' ', num_args = 1..)]
    parsers: Vec<String>,

    /// Log additional debug information.
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// List all the layer names found in the capture. Will list the layers and exit. No output
    /// will be produced.
    #[arg(long, default_value_t = false)]
    list_layers: bool,

    /// Filter the capture before processing, e.g. "udp". Can help speed up processing large
    /// captures.
    #[arg(long)]
    filter: Option<String>,

    /// Overwrite any existing output.
    #[arg(short, long, default_value_t = false)]
    force: bool,

    /// Do not prompt for user input.
    #[arg(long, default_value_t = false)]
    headless: bool,

    /// Include the showname display value in the output json. This will appear as the "display"
    /// field inside each field object. Note that this should only be used for debugging and
    /// manual labeling purposes.
    #[arg(long, default_value_t = false)]
    showname: bool,
}

fn main() -> std::io::Result<()> {
    // Setup logging.
    // simple_logger::init_with_env().unwrap();

    let args = Args::parse();

    let file_stem = std::path::Path::new(&args.input)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let output_dir = format!("{}/{}", args.output, file_stem);

    // If output directory already exists then confirm it should be overwritten.
    if std::path::Path::new(&output_dir).exists() {
        if !args.force {
            let confirmation = !args.headless
                && Confirm::new()
                    .with_prompt(format!(
                        "Output directory \"{}\" already exists. Overwrite it?",
                        output_dir
                    ))
                    .interact()
                    .unwrap();

            if !confirmation {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "output dir already exists",
                ));
            }
        }

        std::fs::remove_dir_all(&output_dir).unwrap();
    }
    std::fs::create_dir_all(&output_dir)?;

    let remora_log_file_name = format!("{}/remora.log", output_dir);
    println!("Saving logs in {}", remora_log_file_name);

    let rust_log_env_var = env::var("RUST_LOG").unwrap_or("Trace".into());
    let log_level = log::LevelFilter::from_str(rust_log_env_var.as_str())
        .unwrap_or_else(|err| panic!("invalid log level \"{}\" - {}", rust_log_env_var, err));

    simplelog::CombinedLogger::init(vec![
        simplelog::SimpleLogger::new(log_level, simplelog::Config::default()),
        simplelog::WriteLogger::new(
            log::LevelFilter::Trace,
            simplelog::Config::default(),
            File::create(remora_log_file_name).unwrap(),
        ),
    ])
    .unwrap();

    debug!("{:?}", args);
    info!("input file: {:?}", std::fs::canonicalize(&args.input));

    // let input_file_size = std::fs::File::open(args.input.clone())
    //     .unwrap()
    //     .metadata()
    //     .unwrap()
    //     .len() as f64;
    // debug!("input file size: {}", input_file_size);

    let filter = match args.filter {
        Some(filter) => filter,
        None => "".to_string(),
    };

    let builder = rtshark::RTSharkBuilder::builder()
        .input_path(args.input.as_ref())
        .display_filter(&filter);
    // TODO: why does whitelisting this meta data make rtshark fail silently?
    // .metadata_whitelist("_ws.col.info"); // adds the "Info" column displayed in WireShark, e.g.
    // things like "Protected Payload (KP0) ..."

    // Start a new TShark process
    let mut rtshark = match builder.spawn() {
        Err(err) => {
            error!("Error running tshark: {err}");
            return Ok(());
        }
        Ok(rtshark) => rtshark,
    };

    if args.list_layers {
        if !log_enabled!(log::Level::Info) {
            error!(
                "List layers is logged as Info, but log level is set as {}. Increase log level \
                and run again",
                env::var("RUST_LOG").unwrap_or_default()
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "log level incompatible with command line option",
            ));
        }
        let mut layers = HashSet::new();
        // read packets until the end of the PCAP file
        while let Some(packet) = rtshark.read().unwrap_or_else(|e| {
            error!("Error parsing TShark output: {e}");
            None
        }) {
            for layer in packet {
                layers.insert(layer.name().to_string());
            }
        }
        info!("{:?}", layers);
        return Ok(());
    }

    let mut packet_number = 0;

    // Have the output file name match the input file name with a suffix and a different extension.
    // Store the output files in their own directory.

    // The file base name is the output directory followed by a directory named after the input
    // file followed by the input file name stem.
    let file_base_name = format!(
        "{}/{}",
        output_dir,
        std::path::Path::new(&args.input)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
    );

    let mut parsers: HashMap<String, GenericParser> = HashMap::new();
    let allowed_parsers = args.parsers;

    // let mut failed_parse_layers = HashSet::new();

    // read packets until the end of the PCAP file
    while let Some(packet) = rtshark.read().unwrap_or_else(|e| {
        error!("Error parsing TShark output: {e}");
        None
    }) {
        packet_number += 1;
        // if packet_number % 100 == 0 {
        //     print!("*");
        // }
        if args.debug {
            for layer in packet.clone() {
                // TODO: use frame number instead
                debug_packet_layer(&layer, packet_number);
            }
        }

        // A packet can have multiple layers with the same name. Iterate over the unique names.
        // TODO: could a packet have non-contiguous duplicate layers?
        //       e.g. QUIC->OTHER_PROTO->QUIC
        let unique_layer_names: HashSet<String> =
            HashSet::from_iter(packet.iter().map(|x| x.name().to_owned()));
        for layer_name in unique_layer_names {
            // If we should create the parser and the parse does not exist then create it.
            if (allowed_parsers.is_empty() || allowed_parsers.contains(&layer_name))
                && !parsers.contains_key(&layer_name)
            {
                info!("Creating default generic parser for {}", layer_name);
                let parser =
                    GenericParser::new(&file_base_name, layer_name.to_owned(), args.showname);
                // parser.parse(&packet);
                parsers.insert(layer_name.clone(), parser);
            }

            match parsers.get_mut(&layer_name) {
                Some(parser) => parser.parse(&packet),
                None => continue,
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn debug_packet_layer(layer: &rtshark::Layer, packet_number: usize) {
    // let layers_to_skip = vec!["frame", "eth", "ip", "tcp", "udp", "dns", "ipv6", "tls"];
    // if layers_to_skip.contains(&layer.name()) {
    //     return;
    // }

    info!("Layer: {}", layer.name());
    info!(
        "\tnumber: {}\tsize: {}\tposition: {}",
        packet_number,
        layer.size(),
        layer.position()
    );
    // info!("Layer: {}\tsize: {}\tposition: {}", layer.name(), layer.size(), layer.position());
    for metadata in layer.iter() {
        info!("\t{}\t{}", metadata.name(), metadata.value());
        info!(
            "\t\tsize: {}\tposition: {}",
            metadata.size(),
            metadata.position()
        );
        // info!("\t\t{}    {}", metadata.position(), metadata.size());
        // info!("\t\t{}", metadata.display());
    }
    info!("\n==================\n");
}

fn format_binary_value(value: &str) -> Result<String, String> {
    debug_assert!(value.len() <= 32); // TODO: support 64 bit fields?

    // debug!("format_binary_value = {}", value);

    // TODO: will the value ever be integer and not hex?
    match value {
        "True" => Ok("1".into()),
        "False" => Ok("0".into()),
        _ => Ok(value.trim_start_matches("0x").into()),
    }
}

fn integer_string_to_hex_string(input: String) -> String {
    let int_value = input
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("Failed to decode integer value {}", input));

    format!("{:x}", int_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_binary_value_test() {
        assert_eq!("0", format_binary_value("False").unwrap());
        assert_eq!("1", format_binary_value("True").unwrap());
        assert_eq!("0", format_binary_value("0").unwrap());
        assert_eq!("1", format_binary_value("1").unwrap());
        assert_eq!("00", format_binary_value("0x00").unwrap());
        assert_eq!("01", format_binary_value("0x01").unwrap());
    }

    #[test]
    fn integer_string_to_hex_string_test() {
        assert_eq!("0", integer_string_to_hex_string("0".to_string()));
    }
}
