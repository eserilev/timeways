#!/usr/bin/env python3
"""Writes SHA256SUMS and timeways-manifest.json into the release folder.

Usage: release-manifest.py DIST TAG

The setup of Gnomish Relay reads the manifest to pick the archive for its computer.
The script fails when an archive is missing, so a release never goes out half built.
"""

import hashlib
import json
import re
import sys
from pathlib import Path

TARGETS = [
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
PROGRAMS = ["timeways-story", "timeways-pack"]
ADDON = "timeways-addon.zip"
ROOT = Path(__file__).resolve().parent.parent


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def archive(target):
    if "windows" in target:
        return f"timeways-{target}.zip"
    return f"timeways-{target}.tar.gz"


# The bridge accepts a range of these versions, so setup can refuse a release that it cannot
# talk to. The addon tests check the version against the range of the pinned bridge.
def app_version():
    text = (ROOT / "addon/Timeways/App.lua").read_text()
    found = re.search(r"^\s*version = (\d+),", text, re.MULTILINE)
    if found is None:
        sys.exit("error: addon/Timeways/App.lua has no line 'version = <number>,'")
    return int(found.group(1))


def main(dist, tag):
    assets = [archive(target) for target in TARGETS] + [ADDON]
    sums = {name: sha256(dist / name) for name in assets}
    (dist / "SHA256SUMS").write_text("".join(f"{sums[name]}  {name}\n" for name in assets))
    manifest = {
        "version": tag.removeprefix("v"),
        "tag": tag,
        "app_version": app_version(),
        "targets": {
            target: {
                "asset": archive(target),
                "sha256": sums[archive(target)],
                # Plain names: the bridge adds ".exe" on Windows itself.
                "programs": PROGRAMS,
            }
            for target in TARGETS
        },
        "addon": {"asset": ADDON, "sha256": sums[ADDON]},
    }
    (dist / "timeways-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main(Path(sys.argv[1]), sys.argv[2])
