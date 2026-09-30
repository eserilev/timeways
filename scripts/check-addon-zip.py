#!/usr/bin/env python3
"""Checks the addon zip of the packager against the TOC.

Usage: check-addon-zip.py ZIP

The zip holds one folder, Timeways, with each file of the TOC. A Key.lua never goes in: the
key of each computer lives in the Timeways_Key addon that the desktop app writes, and a
packaged key would be the same for every player.
"""

import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# The packager adds these two next to the code.
EXTRAS = {"LICENSE", "CHANGELOG.md"}
KEY = "Key.lua"


def toc_files():
    lines = (ROOT / "addon/Timeways/Timeways.toc").read_text().splitlines()
    return {line.strip() for line in lines if line.strip() and not line.startswith("#")}


def problems(names):
    wanted = toc_files() | {"Timeways.toc"}
    files = {name for name in names if not name.endswith("/")}
    outside = [name for name in files if not name.startswith("Timeways/")]
    inside = {name.removeprefix("Timeways/") for name in files} - set(outside)
    found = []
    found += [f"outside the Timeways folder: {name}" for name in sorted(outside)]
    found += [f"missing: {name}" for name in sorted(wanted - inside)]
    found += [f"not in the TOC: {name}" for name in sorted(inside - wanted - EXTRAS - {KEY})]
    if KEY in inside:
        found.append(f"holds {KEY}, the key of one computer")
    return found


def main(path):
    with zipfile.ZipFile(path) as archive:
        found = problems(archive.namelist())
    for problem in found:
        print(f"error: {path}: {problem}", file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
