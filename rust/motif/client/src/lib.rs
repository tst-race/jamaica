// Copyright 2024 Two Six Technologies.

use std::net::ToSocketAddrs;
use std::time::Duration;

mod client_control_input;
use crate::client_control_input::ClientControlInput;

mod remora_input;
use crate::remora_input::RemoraInput;

mod network_comm;
use crate::network_comm::NetworkComm;

mod network_comm_udp;
use crate::network_comm_udp::NetworkCommUdp;
mod network_comm_tcp;
use crate::network_comm_tcp::NetworkCommTcp;

use motiflib::action::Action;
// use motiflib::motif_input::MotifInput;

mod packet_gen;
// mod remora_control_message;

use log::{
    debug,
    // info,
    // warn,
    error,
    info,
};

pub fn run_input(file_name: &str, strict: bool, dry_run: bool) -> std::io::Result<()> {
    // let input = JsonInput::new("input/sample.json");

    // let file_name = "/Users/gregory.paton/projects/thing/dev/motif/grunt/input/test.json";
    // let file_name = "/Users/gregory.paton/projects/thing/dev/motif/grunt-output.json";
    // let file_name = "/Users/gregory.paton/projects/thing/dev/motif/grunt/grunt-output.json";
    // let file_name = "/Users/gregory.paton/projects/thing/dev/motif/grunt-output-quic-payload-debugging.json";
    // let file_name = "/Users/gregory.paton/projects/thing/dev/motif/grunt-output-quic-payload.json";

    debug!("running {}", file_name);
    // let input = ClientControlInput::new(file_name);
    let input = RemoraInput::new(file_name, strict);
    if dry_run {
        info!("dry run: exiting early");
        return Ok(());
    }
    // let mut input = QuicInput::new();

    // TODO: make this bounded, i.e. start dropping oldest payloads once some max size is reached.
    let mut received_payloads: Vec<Vec<u8>> = vec![];

    // let has_tcp = input.conversations.iter().any(|x| x.protocol.as_str() == "TCP");
    // let has_udp = input.conversations.iter().any(|x| x.protocol.as_str() == "UDP");

    for conversation in input.conversations {
        // TODO: create comm link, either UDP or TCP

        // TODO: Cleaner way to do this? This seems ridiculous.
        let tcp_server_addr = format!("{}:{}", "localhost", 19305);
        // let tcp_server_addr = format!("{}:{}", "motifserver", 19305);
        let tcp_server_addr = match tcp_server_addr.as_str().to_socket_addrs().unwrap().next() {
            Some(addr) => addr,
            None => {
                return Err(std::io::Error::other(format!(
                    "Failed to parse TCP server address: {}",
                    tcp_server_addr
                )));
            }
        };

        // TODO: do we care about explicitly setting client port?
        let protocol = conversation.protocol.as_str();
        let comm: Box<dyn NetworkComm> = match protocol {
            "UDP" => {
                // TODO: Cleaner way to do this? This seems ridiculous.
                let udp_server_addr = format!("{}:{:?}", "0.0.0.0", 9999);
                // let udp_server_addr = format!("{}:{:?}", "motifserver", 9999);
                let udp_server_addr =
                    match udp_server_addr.as_str().to_socket_addrs().unwrap().next() {
                        Some(addr) => addr,
                        None => {
                            return Err(std::io::Error::other(format!(
                                "Failed to parse TCP server address: {}",
                                udp_server_addr
                            )));
                        }
                    };
                debug!("sending UDP traffic to {}", udp_server_addr);

                let mut udp_comm =
                    NetworkCommUdp::new("0.0.0.0:0".parse().unwrap(), udp_server_addr);
                let _handle = udp_comm.spawn()?;
                Box::new(udp_comm)
            }
            "TCP" => {
                let tcp_comm = NetworkCommTcp::new(tcp_server_addr);
                match tcp_comm.spawn() {
                    Ok(_) => (),
                    Err(error) => return Err(error),
                }
                Box::new(tcp_comm)
            }
            _ => {
                panic!("invalid protocol: {}", conversation.protocol);
            }
        };

        for action in conversation.actions {
            match action {
                Action::Send(send_action) => {
                    // (send_action.create)(&received_payloads);
                    // TODO: configure IP and port in input json.
                    match comm.send((send_action.create)(&received_payloads)) {
                        Err(error) => {
                            let error_message = format!("Failed to perform action: {}", error);
                            error!("{}", error_message);
                            return Err(std::io::Error::other(error_message));
                        }
                        _ => (),
                    }
                }
                Action::Receive(receive_action) => {
                    match comm.receive(Duration::new(receive_action.timeout as u64, 0)) {
                        Ok((payload, src)) => {
                            debug!("received {} bytes from {}", payload.len(), src);
                            received_payloads.push(payload);
                        }
                        Err(error) => {
                            error!("{}", error);
                            return Err(std::io::Error::other(format!(
                                "Did not receive expected response: {}",
                                error
                            )));
                        }
                    };
                }
                Action::Quit => break,
            }
        }

        // TODO: remove this API in favor of Drop?
        comm.close().unwrap();
    }

    debug!("done with input: {}", file_name);

    Ok(())
}
