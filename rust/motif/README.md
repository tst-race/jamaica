# motif

```bash
cargo doc --open
```

## Running

Start the server. This is a long running process, just kick it off in a terminal and let it go.
```bash
cd server
cargo run
```

In a separate terminal run the client. This will send and receive some messages, then exit when it is out of messages to send and doesn't expect to receive any more.
```bash
cd client
cargo run
```

## Design Goals and Decisions

* The client will **always** initiate the connection. The server will never reach out first. If such a scenario is required then it can be simulated by running the client from the servers physical location and vice-versa.
* Motif should work reasonably well out of the box, but should also allow for superusers to customize different pieces of the program to suit their specific needs. The pieces include but aren't limited to:
  * Client control messages (e.g. encoding and decoding)
  * Overriding auto-generated field generation functions
  * Providing new inputs types to drive the program

## Generating PCAPs

A [script](captures/run_capture.sh) has been added to easily generate PCAPs of a client/server session. The sctip will start `tcpdump`, then build and run both the client and server. The script will run until the client exits. The script takes a single optional string argument which will be appended to the output file name. The PCAP will be saved to the current working directory.

```bash
cd captures

./run_captures.sh

./run_captures.sh quic_convo
```

Currently the client is hardcoded to run QUIC with client control input. You can try modifying the values in the [input file](client/input/client_control_sample.json) and see the change in the resulting PCAP.

## Testing

```bash
cargo test --workspace
```

## Linting

```bash
cargo clippy --all
```

## Random

Get system info about UDP
```bash
sysctl -a | grep net.inet.udp
```

Send UDP packet from the command line.
```bash
echo "hello, server!" | nc -cu localhost $PORT
```

Viewing WebRTC in WireShark: [instructions](https://webrtc.github.io/webrtc-org/testing/wireshark/).

Maybe interesting [crate](https://github.com/rust-pcap/pcap).

https://msgpack.org/

### Quic

https://github.com/wireshark/wireshark/blob/wireshark-4.2.0/epan/dissectors/packet-quic.c#L4583

### Potentially useful crates

* [PCAP](https://docs.rs/pcap/latest/pcap/)
* [rtshark](https://docs.rs/rtshark/latest/rtshark/)

## Tasks

* Server side config json
  * ports to open

### Stretch

* potential optimizations
  * https://idndx.com/writing-highly-efficient-udp-server-in-rust/

---
Copyright 2024 Two Six Technologies.
