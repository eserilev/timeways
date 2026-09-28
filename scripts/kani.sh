#!/usr/bin/env bash
# Runs the Kani proofs, or only the HARNESS that you name.
# The memory cap stops a big proof before it takes all the memory of the machine.
set -euo pipefail
cd "$(dirname "$0")/.."
exec systemd-run --user --scope --quiet -p MemoryMax=8G -p MemorySwapMax=0 \
    cargo kani -p timeways-story ${1:+--harness "$1"}
