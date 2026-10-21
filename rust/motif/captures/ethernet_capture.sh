#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

# Capture broad internet traffic on your local machine. Use this if you are, for example, navigating websites or on a video call.

set -eux

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# If things exit early, kill background processes, e.g. the server.
# https://stackoverflow.com/a/2173421/854871
trap 'trap - SIGTERM && kill -- -$$' SIGINT SIGTERM EXIT

SUFFIX=""
if [ "$#" -ge "1" ]; then
  SUFFIX="_$1"
  SUFFIX="${SUFFIX// /_}"
fi

PCAP_FILE_NAME="capture_$(date '+%Y-%m-%d-%H-%M-%S')_eth$SUFFIX.pcap"

echo "Capturing traffic on ethernet and saving to $PCAP_FILE_NAME"

# tcpdump -i en0 -w $PCAP_FILE_NAME udp portrange 16384-32767
tcpdump -i en0 -w $PCAP_FILE_NAME
# tcpdump -s 98 -i en0 -w $PCAP_FILE_NAME
