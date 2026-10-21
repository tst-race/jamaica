#!/bin/bash

set -eu

file="$1"

echo "running tshark for $file ..."
BASE_NAME=${file%.pcap}

tshark -r "$file" -Tpdml -n -P -l > "$BASE_NAME.pdml"
# tshark -r "$file" -Tpdml -n -Q -l > "$BASE_NAME.pdml"

echo "output written to $BASE_NAME.pdml"