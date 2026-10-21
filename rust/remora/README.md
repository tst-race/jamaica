# Remora

Tool to convert PCAPs into a list of connections, each containing a lit of packets that are segmented by protocol fields (JSON). 

https://github.com/wireshark/wireshark/blob/master/epan/dissectors/packet-quic.c

## Running

```bash
cargo run -- <YOUR_INPUT>.pcap
```

### Log Level

The log level can be using the `RUST_LOG` environment variable.

```bash
export RUST_LOG="Debug" 
cargo run --release -- my_capture.pcap

# OR

RUST_LOG="Warn" cargo run --release -- my_capture.pcap
```

## tshark

Adding the option `-e _ws.col.info` will print the packet type.

```bash
tshark -r /Users/gregory.paton/projects/thing/captures/capture_2023-11-10-11-56-48_eth_instagram.pcap -T json -e _ws.col.info > output.json
```

Get raw bytes `-e data.data`.

rtshark command:

```bash
tshark -r /Users/gregory.paton/projects/thing/captures/capture_2023-11-10-11-56-48_eth_instagram.pcap -Tpdml -n -Q -l
```

Get QUIC short headers:
EDIT: doesn't work, includes long headers.
```bash
tshark -r /Users/gregory.paton/projects/thing/captures/capture_2023-11-10-11-56-48_eth_instagram.pcap -Y "quic.short" -T json > quic-short-header-out.json
```

https://www.wireshark.org/docs/man-pages/tshark.html

## Wireshark Dissectors

[quic](https://github.com/wireshark/wireshark/blob/master/epan/dissectors/packet-quic.c)

## TODO

### Other protocol support, provided by Paul

```
arp
data
data
icmp
icmp:data
icmp:ip
igmp:igmp
tcp
tcp:data
tcp:http
tcp:http:data
tcp:http:data:data
tcp:http:image-gif
tcp:http:image-jfif
tcp:irc
tcp:mptcp
tcp:rtps
tcp:stun
tcp:tls
tcp:xmpp
udp
udp:a21
udp:acdr
udp:acdr:ip
udp:acdripv6
udp:adwin
udp:ajns
udp:amt
udp:ardp
udp:asap
udp:asterix
udp:ath
udp:ax4000
udp:ayiya
udp:bat:data
udp:bat.gw:data
udp:bat.gw:ip
udp:bat.vis:data
udp:bfd
udp:bfd_echo
udp:c1222
udp:capwap
udp:capwap.data
udp:capwap.data:dtls
udp:capwap:dtls
udp:cflow
udp:cipio
udp:cnip
udp:coap
udp:collectd
udp:corosync_totemnet:data
udp:data
udp:db-lsp-disc
udp:dcerpc
udp:dcp-etsi:dcp-af
udp:dcp-etsi:dcp-pft
udp:dhcp
udp:dis
udp:dmp
udp:dns
udp:dof
udp:doip
udp:dpnet
udp:dtls
udp:dvb_ipdc:alc:rmt-lct
udp:dvb_ipdc:data
udp:ecatf:ams
udp:ecatf:data
udp:ecatf:ecat
udp:ecatf:ecat_mailbox
udp:ecatf:ioraw
udp:ecatf:tc_nv
udp:echo
udp:ecmp
udp:egd
udp:elasticsearch
udp:enip
udp:enrp
udp:epl
udp:eth
udp::data
udp:eth:llc
udp:ff
udp:geneve:data
udp:gprs-ns
udp:gprs-ns:bssgp
udp:gre:data
udp:gsmtap:data
udp:gtp
udp:gtpprime
udp:h248
udp:h501
udp:hart_ip
udp:hcrt
udp:hiqnet
udp:iapp
udp:iax2:data
udp:icp
udp:idn
udp:ieee1722:data
udp:ieee1722:iec61883
udp:infiniband
udp:ipvs
udp:isakmp
udp:jmirror:ip
udp:jmirror:ipv6
udp:kdp
udp:kink
udp:kip
udp:knet
udp:lisp-data
udp:lisp:data
udp:lisp-data:ip
udp:lisp-data:ipv6
udp:llc
udp:llc:data
udp:llmnr
udp:lwapp
udp:lwapp:data
udp:lwapp:wlan
udp:lwapp:wlan:data
udp:lwres
udp:manolito
udp:mdns
udp:megaco:data
udp:memcache
udp:mih
udp:mikey
udp:mint
udp:mipv6
udp:mndp
udp:mp2t
udp:mpls
udp:mpls:data
udp:mpls-echo
udp:mpls:ip
udp:mpls:ipv6
udp:mpls:pwach
udp:mpls:pwach:data
udp:mpls:pwethheuristic
udp:mpls:pwethheuristic:pwethcw:eth
udp:mpls:pwethheuristic:pwethnocw:eth
udp:msproxy
udp:nano
udp:ntp
udp:nxp_802154_sniffer
udp:nxp_802154_sniffer:wpan
udp:opensafety_udp:data
udp:openvpn
udp:pfcp
udp:pktc
udp:pktc.mtafqdn
udp:pn_rt
udp:pn_rt:pn_io
udp:pn_rt:pn_ptcp
udp:portcontrol
udp:quake
udp:quake2
udp:quake2:data
udp:quake3
udp:quake3:data
udp:quake:data
udp:quakeworld
udp:quakeworld:data
udp:quic
udp:radius
udp:raknet
udp:raknet:raknet
udp:rdt
udp:rsip
udp:rsvp
udp:rtcp
udp:sabp
udp:sap
udp:scop
udp:scop:data
udp:scop:wpan
udp:scop:wpan:data
udp:sctp
udp:ssdp
udp:steam_ihs_discovery
udp:stun:data
udp:stun:rtcp
udp:teredo
udp:tetra
udp:tipc
udp:ts2
udp:tzsp
udp:tzsp:data
udp:uaudp
udp:udpencap
udp:udpencap:esp
udp:udpencap:isakmp
udp:ulp
udp:vrt
udp:vuze-dht
udp:vxlan:eth
udp:wg
udp:wisun.eapol_relay
udp:wisun.eapol_relay:eapol
udp:wisun.eapol_relay:eapol:data
udp:wlccp
udp:wsp
udp:wtls
udp:wtp
udp:wtp:wsp
```

### dissecting bit fields

```
        quic.connection.number
                42    0
                0
        quic.packet_length
                42    0
                1232
        quic.short
                42    1232

        quic.header_form
                42    1
                0
        quic.fixed_bit
                42    1
                True
        quic.spin_bit
                42    1
                False
```

## Random

### rtshark limitations

There are various limitation with rtshark (lack of layer position, metadata flattening, etc.). Possible alternative: https://github.com/rusticata/pcap-parser

### PCAPs

```
../motif/captures/capture_2024-02-16-12-13-44_eth_random_quic.pcap
```

### tshark

```
tshark -r ../motif/capture_2024-05-24-10-54-45_eth_short_rtp.pcap -Tpdml -n -Q -l > capture_2024-05-24-10-54-45_eth_short_rtp.pdml
```

---
Copyright 2024 Two Six Technologies.
