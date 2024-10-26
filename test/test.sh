#!/bin/bash

set -euo pipefail

# Go to the root of the project
CDPATH='' cd -- "$(dirname -- "$0")/.."

# Check if make is available
if ! command -v make &>/dev/null; then
  echo "Error: make is not installed. Please install it to proceed." >&2
  exit 1
fi

# Build the test binary
make test

MODEL=./test/data/3DBenchy.stl
if {
  if [ "$(uname)" = "Linux" ]; then
    TOOLCHAIN="$(rustc --version --verbose | grep "host" | awk '{ print $2 }')"
    TARGET_DIR="$(CDPATH='' cd -- "target/${TOOLCHAIN}/release" && pwd -P)"
    PACKAGE_VERSION=$(cargo pkgid | cut -d "#" -f2 | awk -F. '{printf "%s.%s",$1,$2}')

    # Ensure the versioned symlink for lib exists
    ln -sf "libstl_thumb.so" "${TARGET_DIR}/libstl_thumb.so.${PACKAGE_VERSION}"

    export LD_LIBRARY_PATH="${TARGET_DIR}"
  fi
  ./test/test "$MODEL"
} | cmp -s - <(cargo run --release -p stl-thumb-cli "$MODEL" - --md5); then
  echo "Test succeeded"
else
  echo "Test failed" >&2
  exit 1
fi
