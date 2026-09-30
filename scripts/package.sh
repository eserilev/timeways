#!/usr/bin/env bash
# Packs the release build of TARGET into dist/, with its SHA-256 sum. The release job runs
# it on each OS. The setup of Gnomish Relay downloads what it makes.
set -euo pipefail
target=$1
root=$(git rev-parse --show-toplevel)
cd "$root"
mkdir -p dist
build=target/$target/release
if [[ $target == *windows* ]]; then
  name=timeways-$target.zip
  pwsh -NoProfile -Command "Compress-Archive -Force -Path '$build/timeways-story.exe', '$build/timeways-pack.exe', 'LICENSE' -DestinationPath 'dist/$name'"
else
  name=timeways-$target.tar.gz
  tar -czf "dist/$name" -C "$build" timeways-story timeways-pack -C "$root" LICENSE
fi
cd dist
if command -v sha256sum > /dev/null; then
  sha256sum "$name" > "$name.sha256"
else
  shasum -a 256 "$name" > "$name.sha256"
fi
echo "dist/$name"
