#!/usr/bin/env bash
# Copyright 2024 Two Six Technologies.

# set -o errexit
set -o nounset
set -o pipefail

# I'm sure there's a better way, but it works for now.
find . -name '*.rs' | grep -v "./target" |  xargs grep -iL copyright
find . -name '*.sh' |  xargs grep -iL copyright
