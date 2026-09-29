#!/usr/bin/env bash
# Runs the fuzz targets side by side for SECONDS (default 30): the TARGETs, or every target.
# The targets share the cores: each one runs as many libFuzzer workers (-fork) as its share.
# CI runs each target short on a machine of its own, and the nightly job runs them all long.
# A crash leaves its input in fuzz/artifacts/. Turn it into a regression test first.
# Overflow checks stay on, because a number that wraps is a bug that a release build hides.
set -euo pipefail
fuzz="$(cd "$(dirname "$0")/../fuzz" && pwd)"
cd "$fuzz"
seconds="${1:-30}"
shift || true
targets=("$@")
if [ ${#targets[@]} -eq 0 ]; then
  targets=(input store answers json_lua pages replies)
fi
# The prebuilt cargo-fuzz of CI is a musl build, and it builds for its own target by
# default. The address sanitizer needs the dynamic libc of the host target.
host=$(rustc +nightly -vV | sed -n 's/^host: //p')
cores=$(nproc 2>/dev/null || sysctl -n hw.ncpu)
workers=$((cores / ${#targets[@]}))
if [ "$workers" -lt 1 ]; then
  workers=1
fi
# libFuzzer starts with short inputs and grows them slowly, so a short run from few seeds
# never makes a long play. These start long at once, up to a play of some hundred steps.
max_len=16384
runs=$(mktemp -d)
trap 'rm -rf "$runs"' EXIT

# One build first, so the runs never wait for each other's builds.
cargo +nightly fuzz build --target "$host" --debug-assertions >"$runs/build.log" 2>&1 || {
  tail -40 "$runs/build.log"
  exit 1
}

# Fork mode writes the logs of its workers into the working folder, so each target runs in
# a folder of its own.
run() {
  local target=$1
  local dict=()
  if [ -f "dicts/$target.dict" ]; then
    dict=(-dict="$fuzz/dicts/$target.dict")
  fi
  mkdir -p "corpus/$target" "seeds/$target" "artifacts/$target" "$runs/$target"
  (cd "$runs/$target" && cargo +nightly fuzz run --fuzz-dir "$fuzz" --target "$host" \
    --debug-assertions "$target" "$fuzz/corpus/$target" "$fuzz/seeds/$target" -- \
    -max_total_time="$seconds" -fork="$workers" -artifact_prefix="$fuzz/artifacts/$target/" \
    -len_control=0 -max_len="$max_len" \
    "${dict[@]}" >"$runs/$target.log" 2>&1)
}

for target in "${targets[@]}"; do
  run "$target" &
  pids+=("$!")
done

failed=0
for i in "${!targets[@]}"; do
  target=${targets[$i]}
  if wait "${pids[$i]}"; then
    echo "$target: $(grep -E 'exec/s' "$runs/$target.log" | tail -1)"
  else
    # The panic comes before the long notes of cargo-fuzz at the end of the log.
    grep -m1 -A8 "panicked at\|ERROR: libFuzzer\|ERROR: AddressSanitizer" "$runs/$target.log" || true
    tail -15 "$runs/$target.log"
    echo "fuzz: $target failed" >&2
    failed=1
  fi
done
if [ "$failed" -ne 0 ]; then
  exit 1
fi
echo "fuzz ok"
