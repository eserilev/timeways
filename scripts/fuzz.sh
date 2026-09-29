#!/usr/bin/env bash
# Runs the fuzz targets for SECONDS each (default 30): the TARGETs, or every target.
# CI runs each target short, and the nightly job runs them all long. A crash leaves its
# input in fuzz/artifacts/. Turn it into a regression test first. Overflow checks stay on,
# because a number that wraps is a bug that a release build hides.
set -euo pipefail
cd "$(dirname "$0")/../fuzz"
seconds="${1:-30}"
shift || true
targets=("$@")
if [ ${#targets[@]} -eq 0 ]; then
  targets=(input store answers json_lua pages)
fi
# The prebuilt cargo-fuzz of CI is a musl build, and it builds for its own target by
# default. The address sanitizer needs the dynamic libc of the host target.
host=$(rustc +nightly -vV | sed -n 's/^host: //p')
log=$(mktemp)
trap 'rm -f "$log"' EXIT

for target in "${targets[@]}"; do
  mkdir -p "corpus/$target" "seeds/$target"
  dict=()
  if [ -f "dicts/$target.dict" ]; then
    dict=(-dict="dicts/$target.dict")
  fi
  if ! cargo +nightly fuzz run --target "$host" --debug-assertions "$target" "corpus/$target" "seeds/$target" -- \
      -max_total_time="$seconds" "${dict[@]}" >"$log" 2>&1; then
    tail -40 "$log"
    echo "fuzz: $target failed" >&2
    exit 1
  fi
  echo "$target: $(grep -E '^Done' "$log")"
done
echo "fuzz ok"
