#!/bin/bash

set -euo pipefail

# Go to the root of the project
CDPATH='' cd -- "$(dirname -- "$0")/.."

# Check if make is available
if ! command -v make &> /dev/null; then
  echo "Error: make is not installed. Please install it to proceed." >&2
  exit 1
fi

# Build the test binary
make test

MODEL=./test/data/3DBenchy.stl
if ./test/test "$MODEL" | cmp -s - <(cargo run --release -p stl-thumb-cli "$MODEL" - --md5); then
  echo "Test succeeded"
else
  echo "Test failed" >&2
  exit 1
fi
