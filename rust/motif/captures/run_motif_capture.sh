#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

# For automatically running and capturing motif traffic.

set -eu
# set -x

echo "running motif client and server and running tcpdump to capture packets."

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# If things exit early, kill background processes, e.g. the server.
# https://stackoverflow.com/a/2173421/854871
trap 'trap - SIGTERM && kill -- -$$' SIGINT SIGTERM EXIT

SUFFIX=""
if [[ "$#" -ge "1" ]]; then
  SUFFIX="_$1"
  SUFFIX="${SUFFIX// /_}"
fi

PCAP_FILE_NAME="${SCRIPT_DIR}/capture_$(date '+%Y-%m-%d-%H-%M-%S')${SUFFIX}.pcap"

# tcpdump -i en0 -w $PCAP_FILE_NAME
# exit 0
# tcpdump -s 98 -i en0 -w $PCAP_FILE_NAME

# Capture loopback (localhost) traffic
# touch "${PCAP_FILE_NAME}"
sudo sh -c ":"
sudo tcpdump -i lo0 -w "${PCAP_FILE_NAME}" &
TCPDUMP_PID=$!
echo "TCPDUMP PID ${TCPDUMP_PID}"

# cargo build --release --bin server
cargo build --release --features msgpack --bin server > /dev/null
"${SCRIPT_DIR}/../target/release/server" &
SERVER_PID=$!

MOTIF_INPUT="motif-input-many-server-response.json"
MOTIF_INPUT="grunt-output.json"
# cargo run --quiet --release --bin client -- "$MOTIF_INPUT"
cargo run --quiet --release --features msgpack --bin client -- "$MOTIF_INPUT" > /dev/null
echo "SERVER PID ${SERVER_PID}"

# ps aux

# Not sure why the sleep is needed, but the capture will fail without it.
sleep 1

echo "Killing tcpdump ${TCPDUMP_PID}"
sudo kill "${TCPDUMP_PID}"
echo "output saved to ${PCAP_FILE_NAME}"
