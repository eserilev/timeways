#!/usr/bin/env bash
# Fetches the pinned release of MyRolePlay (MRP) into target/mrp, for the MSP check
# (docs/plans/msp.md, section 4). MRP is GPLv3, so its source never goes into this repo.
# To move the pin, change the version, the URL, and the hash together.
set -euo pipefail

VERSION=12.1.0.657
URL="https://www.wowinterface.com/downloads/getfile.php?id=4990&aid=170341"
SHA256=ff656b746c818f1c15d2e16bc0124361f711a80de408e8fc1d7ab3d335a53c92

root=$(git rev-parse --show-toplevel)
dir=$root/target/mrp
zip=$dir/MyRolePlay-$VERSION.zip
mkdir -p "$dir"
if [ ! -f "$zip" ]; then
  curl -fsSL -o "$zip.part" "$URL"
  mv "$zip.part" "$zip"
fi
echo "$SHA256  $zip" | sha256sum -c --quiet
rm -rf "$dir/MyRolePlay"
unzip -q "$zip" -d "$dir"
echo "MyRolePlay $VERSION is in $dir/MyRolePlay"
