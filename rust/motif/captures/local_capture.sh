#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

set -eu

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# If things exit early, kill background processes, e.g. the server.
# https://stackoverflow.com/a/2173421/854871
trap 'trap - SIGTERM && kill -- -$$' SIGINT SIGTERM EXIT

SUFFIX=""
if [ "$#" -ge "1" ]; then
  SUFFIX="_$1"
  SUFFIX="${SUFFIX// /_}"
fi

PCAP_FILE_NAME="capture_$(date '+%Y-%m-%d-%H-%M-%S')$SUFFIX.pcap"

# tcpdump -i en0 -w $PCAP_FILE_NAME
# exit 0
# tcpdump -s 98 -i en0 -w $PCAP_FILE_NAME

echo "Capturing traffic on localhost and saving to $PCAP_FILE_NAME"

# Capture loopback (localhost) traffic
touch "$PCAP_FILE_NAME"
sh -c ":"
tcpdump -i eth0 -w "$PCAP_FILE_NAME"
