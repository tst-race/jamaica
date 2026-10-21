#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

# set -o errexit
set -o nounset
set -o pipefail

# find . -name '*.json'  -not -path "./target/*" | xargs -L1 python3 -mjson.tool > /dev/null
# find . -name '*.json'  -not -path "./target/*" -exec "echo {} >> /dev/stderr" \; -exec python3 -mjson.tool {} \; > /dev/null


while IFS='' read -r -d '' filename; do
  echo "$filename"
  python3 -mjson.tool "$filename" > /dev/null
done < <(find . -name '*.json'  -not -path "./target/*" -print0)
