// Copyright 2024 Two Six Technologies.

/// SERVER
use std::net::SocketAddr;
// use hex;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

mod server_comm;
use crate::server_comm::ServerComm;
mod server_comm_udp;
use crate::server_comm_udp::ServerCommUdp;
mod server_comm_tcp;
use crate::server_comm_tcp::ServerCommTcp;
mod udp_traffic_handler;
use crate::udp_traffic_handler::UdpTrafficHandler;
mod tcp_traffic_handler;
use crate::tcp_traffic_handler::TcpTrafficHandler;

mod client_control_responder;

use log::{debug, error, info, trace, warn};
use simple_logger::SimpleLogger;
use simplelog::format_description;
// use time::macros::format_description;
use std::env;
use std::fs::File;
use std::str::FromStr;

/// SERVER
fn main() -> std::io::Result<()> {
    // SimpleLogger::new().env().init().unwrap();
    // Setup logging.
    // simple_logger::init_with_env().unwrap();
    let log_filename = "server.log";
    let rust_log_env_var = env::var("RUST_LOG").unwrap_or("Warn".into());
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

    // error!("server log level {}", log_level);
    // trace!("SERVER TESTING");

    // Handle UDP traffic on port 9999.
    let udp_port = 9999;
    let udp_traffic_handler = UdpTrafficHandler::new(udp_port);

    // Handle TCP traffic on port 19305
    let tcp_traffic_handler = TcpTrafficHandler::new(19305);

    // Start loop to receive UDP traffic.
    udp_traffic_handler.recv();

    match tcp_traffic_handler.listen_thread_handle.join() {
        Err(error) => {
            error!("{:?}", error);
        }
        _ => (),
    };

    udp_traffic_handler.udp_recv_thread.join().unwrap();

    Ok(())
}
