#!/usr/bin/env bash
# Runs the release steps after the build on fake programs, and checks the result the way
# the setup of Gnomish Relay reads it. CI runs it on each push, so a tag finds no surprise.
# Usage: release-dry-run.sh ADDON_ZIP
# ADDON_ZIP is the zip of the packager. The Windows archive needs pwsh.
set -euo pipefail
addon=$(realpath "$1")
root=$(git rev-parse --show-toplevel)
cd "$root"
builds=$(mktemp -d)
trap 'rm -rf "$builds"' EXIT
rm -rf dist
for target in x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc; do
  suffix=""
  [[ $target == *windows* ]] && suffix=.exe
  mkdir -p "$builds/$target/release"
  for program in timeways-story timeways-pack; do
    echo "fake $program" > "$builds/$target/release/$program$suffix"
  done
  CARGO_TARGET_DIR=$builds scripts/package.sh "$target"
done
cp "$addon" dist/timeways-addon.zip
scripts/check-addon-zip.py dist/timeways-addon.zip
scripts/release-manifest.py dist v0.0.0-dry-run
scripts/check-release-manifest.py dist
echo "release dry run: ok"
