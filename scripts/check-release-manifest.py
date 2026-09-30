#!/usr/bin/env python3
"""Checks the release folder the way the setup of Gnomish Relay reads it.

Usage: check-release-manifest.py DIST

The rules are a copy of `crates/bridge/src/timeways_release.rs` of the pinned relay commit.
If the relay changes them, change this copy too. The addon tests check `app_version`
against the range of the pinned bridge.
"""

import hashlib
import json
import sys
import tarfile
import zipfile
from pathlib import Path

# The targets of `update::target` in the relay: each computer that setup knows.
TARGETS = [
    "x86_64-unknown-linux-gnu",
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
PROGRAMS = ["timeways-story", "timeways-pack"]


def is_plain_name(name):
    return (
        isinstance(name, str)
        and name != ""
        and not name.startswith(".")
        and not any(mark in name for mark in "/\\:")
    )


def program_file(name, target):
    suffix = ".exe" if "windows" in target else ""
    return name if name.endswith(suffix) else name + suffix


def sum_in(sums, name):
    for line in sums.splitlines():
        words = line.split()
        if len(words) > 1 and words[1].lstrip("*") == name:
            return words[0]
    return None


def archive_names(path):
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            return set(archive.namelist())
    with tarfile.open(path) as archive:
        return {name.removeprefix("./") for name in archive.getnames()}


def target_problems(dist, target, entry, sums):
    if not isinstance(entry, dict):
        return [f"{target}: no build"]
    asset = entry.get("asset")
    programs = entry.get("programs")
    if not isinstance(programs, list):
        return [f"{target}: no list of programs"]
    if not is_plain_name(asset) or not all(is_plain_name(p) for p in programs):
        return [f"{target}: a name with a folder"]
    found = [f"{target}: no {name}" for name in PROGRAMS if name not in programs]
    have = hashlib.sha256((dist / asset).read_bytes()).hexdigest()
    if entry.get("sha256") != have or sum_in(sums, asset) != have:
        found.append(f"{target}: a wrong SHA-256 sum of {asset}")
    inside = archive_names(dist / asset)
    found += [
        f"{target}: {asset} has no {program_file(name, target)}"
        for name in programs
        if program_file(name, target) not in inside
    ]
    return found


def problems(dist):
    manifest = json.loads((dist / "timeways-manifest.json").read_text())
    sums = (dist / "SHA256SUMS").read_text()
    found = []
    if not isinstance(manifest.get("version"), str):
        found.append("no version")
    version = manifest.get("app_version")
    if not isinstance(version, int) or version < 0:
        found.append("no app_version")
    targets = manifest.get("targets", {})
    for target in TARGETS:
        found += target_problems(dist, target, targets.get(target), sums)
    return found


def main(dist):
    found = problems(dist)
    for problem in found:
        print(f"error: {dist}: {problem}", file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
