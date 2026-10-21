#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

set -eu

# If things exit early, kill background processes, e.g. the server.
# https://stackoverflow.com/a/2173421/854871
trap 'trap - SIGTERM && kill -- -$$' SIGINT SIGTERM EXIT

SUFFIX=""
if [ "$#" -ge "1" ]; then
  SUFFIX="_$1"
  SUFFIX="${SUFFIX// /_}"
fi

PCAP_FILE_NAME="capture_$(date '+%Y-%m-%d-%H-%M-%S')_quinn$SUFFIX.pcap"

# Capture loopback (localhost) traffic
touch "$PCAP_FILE_NAME"
sudo sh -c ":"
sudo tcpdump -i lo0 -w "$PCAP_FILE_NAME" &
TCPDUMP_PID=$!
echo "TCPDUMP PID $TCPDUMP_PID"

QUINN_DIR="/Users/gregory.paton/projects/thing/dev/quinn/quinn/examples"

pushd $QUINN_DIR

# Run the server.
cargo run --example server -- ./ --listen 127.0.0.1:4433 &
SERVER_PID="$!"
echo "SERVER PID $SERVER_PID"

# Run the client.
cargo run --example client https://localhost:4433/server.rs > /dev/null

popd

# Not sure why the sleep is needed, but the capture will fail without it.
sleep 1

sudo kill $TCPDUMP_PID
sudo kill $SERVER_PID

echo $PCAP_FILE_NAME
