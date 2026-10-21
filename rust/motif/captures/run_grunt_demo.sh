#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

# For automatically running and capturing motif traffic.

set -e
set -u

if [[ "${TRACE-0}" == "1" ]]; then
    set -o xtrace
fi

echo "running grunt, motif client, and motif server and running tcpdump to capture packets."

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# If things exit early, kill background processes, e.g. the server.
# https://stackoverflow.com/a/2173421/854871
trap 'trap - SIGTERM && kill -- -$$' SIGINT SIGTERM EXIT

# Run grunt.
echo "Running grunt to convert Remora output JSON into Motif input conversation JSON"
REMORA_JSON_FILE="grunt/input/quic-input-1.json"
# REMORA_JSON_FILE="grunt/input/rtp-input-1.json"
# REMORA_JSON_FILE="../data/remora-datasets/capture_2023-11-28-09-26-51_eth_google_meet_chrome/capture_2023-11-28-09-26-51_eth_google_meet_chrome-rtp.json"
echo "Converting Remora JSON $REMORA_JSON_FILE ..."
cargo run --bin grunt -- ${REMORA_JSON_FILE}
echo "Converted Remora JSON"
# TODO: add ability to set this on command line. hardcoded in grunt atm
GRUNT_OUTPUT="grunt-output.json"


SUFFIX=""
if [[ "$#" -ge "1" ]]; then
  SUFFIX="_$1"
  SUFFIX="${SUFFIX// /_}"
fi

CAPTURE_DIR="${SCRIPT_DIR}/captures/"
mkdir -p $CAPTURE_DIR
PCAP_FILE_NAME="${CAPTURE_DIR}/capture_$(date '+%Y-%m-%d-%H-%M-%S')${SUFFIX}.pcap"

# tcpdump -i en0 -w $PCAP_FILE_NAME
# exit 0
# tcpdump -s 98 -i en0 -w $PCAP_FILE_NAME

# Capture loopback (localhost) traffic
# touch "${PCAP_FILE_NAME}"
sudo sh -c ":"
sudo tcpdump -i lo0 -w "${PCAP_FILE_NAME}" &
TCPDUMP_PID=$!
echo "TCPDUMP PID ${TCPDUMP_PID}"

export RUST_LOG="warn"
export RUST_LOG="trace"

BUILD_TYPE="release"
BUILD_TYPE="debug"
BUILD_FLAGS=""
if [[ "$BUILD_TYPE" == "release" ]]; then
  BUILD_FLAGS="--release"
fi

cargo build $BUILD_FLAGS --bin server &> /dev/null
"${SCRIPT_DIR}/../target/$BUILD_TYPE/server" &
SERVER_PID=$!

cargo build $BUILD_FLAGS --bin client &> /dev/null
"${SCRIPT_DIR}/../target/$BUILD_TYPE/client" ${GRUNT_OUTPUT}
# cargo run --quiet --$BUILD_TYPE --bin client -- ${GRUNT_OUTPUT}
echo "SERVER PID ${SERVER_PID}"

# ps aux

# Not sure why the sleep is needed, but the capture will fail without it.
sleep 1

echo "Killing tcpdump ${TCPDUMP_PID}"
sudo kill "${TCPDUMP_PID}"
echo "output saved to ${PCAP_FILE_NAME}"

# NUM_QUIC_PACKETS=$(tshark -r ${PCAP_FILE_NAME} -Y quic -T fields -e frame.number 2>/dev/null | wc -l)
# echo "Found $NUM_QUIC_PACKETS in capture"

tshark -r ${PCAP_FILE_NAME} -q -z io,phs
