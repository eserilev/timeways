#!/bin/sh
# Installs Timeways on macOS and Linux: the Gnomish Relay installer with --timeways.
#   curl -fsSL https://eserilev.github.io/timeways/install.sh | sh
set -eu
# A variable and not a pipe: sh has no pipefail, so a failed download in a pipe looks like success.
installer=$(curl -fsSL https://raw.githubusercontent.com/eserilev/gnomish-relay/main/scripts/install.sh)
sh -c "$installer" install.sh --timeways
