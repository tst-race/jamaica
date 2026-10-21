use core::fmt;
use log::{trace, warn};

use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum Protocol {
    UDP,
    TCP,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct ConnectionTuple {
    pub src_addr: SocketAddr,
    pub dst_addr: SocketAddr,
    pub protocol: Protocol,
}

impl ConnectionTuple {
    // pub fn is_same_connection(&self, other: &ConnectionTuple) -> bool {
    //     if self == other {
    //         return true;
    //     }

    //     if self.protocol == other.protocol
    //         && self.src_addr == other.dst_addr
    //         && self.dst_addr == other.src_addr
    //     {
    //         return true;
    //     }

    //     false
    // }

    ///
    /// Get the ID of the connection. Note that equivalent connections (i.e. a connection whose
    /// source equals destination and vice versa) will have the same ID.
    ///
    /// TODO: rename to something like get_directionless_id?
    pub fn get_id(&self) -> String {
        let mut id = vec![self.src_addr.to_string(), self.dst_addr.to_string()];
        id.sort();
        id.push(self.protocol.to_string());

        id.join(":")
    }
}

// impl Hash for ConnectionTuple {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         // println!("HASH FUNC");
//         // let x = self.x.unsigned_abs() as u64;
//         // let y = self.y.unsigned_abs() as u64;

//         // /* szudziks function */
//         // let hash_val = if x >= y { x * x + x + y } else { x + y * y };
//         // state.write_u64(hash_val);

//         let mut addresses = vec![self.src_addr, self.dst_addr];
//         addresses.sort();

//         println!("HASH FUNC: {:?}", addresses);

//         for addr in addresses {
//             state.write(addr.to_string().as_bytes());
//         }
//         state.write_u64(self.protocol as u64);
//     }
// }

// TODO: return Result<Option<>> instead? Can report errors, or return None when we just don't care.
pub fn packet_to_tuple(packet: &rtshark::Packet) -> Option<ConnectionTuple> {
    // BUG! There can be multiple IP layers. This function returns the higher layer, and we want the lower.
    let (ip_str, ip_layer) = match packet.layer_name("ip") {
        Some(layer) => ("ip", layer),
        None => match packet.layer_name("ipv6") {
            Some(layer) => ("ipv6", layer),
            None => return None,
        },
    };
    let src_ip = ip_layer
        .metadata(&format!("{}.src", ip_str))
        .unwrap()
        .value();
    let dst_ip = ip_layer
        .metadata(&format!("{}.dst", ip_str))
        .unwrap()
        .value();

    let proto_field_str = match ip_str {
        "ip" => "ip.proto",
        "ipv6" => "ipv6.nxt",
        &_ => return None,
    };
    // A list of protocols that we can safely ignore.
    let protocols_to_ignore = [
        1,  // ICMP
        47, // Generic Routing Encapsulation
    ];
    let (proto_str, proto_layer, protocol) = match ip_layer
        .metadata(proto_field_str)
        .unwrap()
        .value()
        .parse::<u32>()
    {
        // https://en.wikipedia.org/wiki/List_of_IP_protocol_numbers
        // 17 - UDP
        Ok(17) => ("udp", packet.layer_name("udp").unwrap(), Protocol::UDP),
        // 6 - TCP
        Ok(6) => ("tcp", packet.layer_name("tcp").unwrap(), Protocol::TCP),
        Ok(proto) if protocols_to_ignore.contains(&proto) => {
            trace!("ignoring protocol {}", proto);
            return None;
        }
        Ok(proto) => {
            warn!("return None due to Proto {}", proto);
            return None;
        }
        Err(err) => {
            warn!("return None due to error {}", err);
            return None;
        }
    };

    let src_port = proto_layer
        .metadata(&(format!("{}.{}", proto_str, "srcport")))
        .unwrap()
        .value()
        .parse::<u16>()
        .unwrap();
    let dst_port = proto_layer
        .metadata(&(format!("{}.{}", proto_str, "dstport")))
        .unwrap()
        .value()
        .parse::<u16>()
        .unwrap();

    Some(ConnectionTuple {
        // src_addr: SocketAddr::new(
        //     IpAddr::V4(
        //         Ipv4Addr::from_str(src_ip)
        //             .unwrap_or_else(|e| panic!("Failed to parse src addr {} - {}", src_ip, e)),
        //     ),
        //     src_port,
        // ),
        // dst_addr: SocketAddr::new(
        //     IpAddr::V4(
        //         Ipv4Addr::from_str(dst_ip)
        //             .unwrap_or_else(|e| panic!("Failed to parse dst addr {} - {}", dst_ip, e)),
        //     ),
        //     dst_port,
        // ),
        src_addr: SocketAddr::new(
            IpAddr::from_str(src_ip)
                .unwrap_or_else(|e| panic!("Failed to parse src addr {} - {}", src_ip, e)),
            src_port,
        ),
        dst_addr: SocketAddr::new(
            IpAddr::from_str(dst_ip)
                .unwrap_or_else(|e| panic!("Failed to parse dst addr {} - {}", dst_ip, e)),
            dst_port,
        ),
        protocol,
    })
}

#[cfg(test)]
mod tests {
    // use core::hash;

    // use rtshark::Packet;

    // use super::*;

    // #[test]
    // fn packet_to_tuple_test() {
    //     let mut packet = Packet::new();

    //     // TODO: no public way of adding layers to a packet. Not sure how to test this. Ugh.

    //     // packet.push("ip".into(), 0, 0);
    //     // if let Some(ip_layer) = packet.last_layer_mut() {

    //     // }

    //     let connection_tuple = packet_to_tuple(&packet);

    //     // assert_eq!(connection_tuple.protocol, Protocol::UDP);
    // }

    // #[test]
    // fn hash_test() {
    //     let connection_a = ConnectionTuple {
    //         src_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
    //         dst_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 9999),
    //         protocol: Protocol::UDP,
    //     };

    //     let connection_b = ConnectionTuple {
    //         src_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 9999),
    //         dst_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
    //         protocol: Protocol::UDP,
    //     };

    //     let mut hasher_a = std::collections::hash_map::DefaultHasher::new();
    //     connection_a.hash(&mut hasher_a);

    //     let mut hasher_b = std::collections::hash_map::DefaultHasher::new();
    //     connection_b.hash(&mut hasher_b);

    //     assert_eq!(hasher_a.finish(), hasher_b.finish());

    //     // s.finish()
    // }
}
