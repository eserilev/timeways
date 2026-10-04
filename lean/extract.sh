#!/usr/bin/env bash
# Translate the crate timeways-rules to Lean with Charon and Aeneas.
#
# Set AENEAS to the root of an Aeneas checkout at the commit in
# lakefile.toml, with `make setup-charon` and `make` done. For example:
#   AENEAS=~/.cache/verif/aeneas lean/extract.sh
# The script writes lean/Timeways/Types.lean and lean/Timeways/Funs.lean.
# Never edit those two files by hand. Run this script again.
set -euo pipefail

: "${AENEAS:?set AENEAS to the root of an Aeneas checkout}"
here="$(cd "$(dirname "$0")" && pwd)"
crate="$(dirname "$here")/crates/rules"
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
mkdir -p "$cache"
work="$(mktemp -d "$cache/timeways-extract.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The roots of the translation are the functions with the mark
# `#[cfg_attr(charon, verify::start_from)]` in crates/rules/src/. Charon
# also translates everything a root calls. To verify one more function,
# put the mark on it.
#
# --duplicate-defaulted-methods: without it, Aeneas passes a whole
# PartialOrd instance where its Lean library takes one function.
args=(--preset=aeneas --duplicate-defaulted-methods --start-from-attribute)

# The crate has its own target directory, so the normal build and the
# Charon build never share one.
(cd "$crate" && RUSTFLAGS="--cfg charon" CARGO_TARGET_DIR="$work/target" \
  "$AENEAS/charon/bin/charon" cargo "${args[@]}" --dest-file "$work/rules.llbc")

"$AENEAS/bin/aeneas" -backend lean -split-files -loops-to-rec \
  -subdir Timeways -dest "$work/out" "$work/rules.llbc" 2>&1 | tee "$work/aeneas.log"

# Aeneas writes a partial file and exits 0 when it cannot translate a
# function. A partial file holds an axiom where the code was, so fail.
if grep -q "Generated the partial file" "$work/aeneas.log"; then
  echo "Aeneas could not translate all of the crate: see the errors above" >&2
  exit 1
fi

mkdir -p "$here/Timeways"
cp "$work/out/Timeways/Types.lean" "$work/out/Timeways/Funs.lean" "$here/Timeways/"

# The external model: the std items the code calls. Aeneas writes a
# template of opaque declarations with no laws. The first run copies
# the template. After that, the model file is yours: the script never
# overwrites it, and it writes the new template beside it for a diff.
src="$work/out/Timeways/FunsExternal_Template.lean"
if [ -f "$src" ]; then
  if [ -f "$here/Timeways/FunsExternal.lean" ]; then
    cp "$src" "$here/Timeways/FunsExternal_Template.lean"
  else
    cp "$src" "$here/Timeways/FunsExternal.lean"
  fi
fi

# Every item of the template needs a body in the model. A missing item
# fails the run, so a new std call never slips in as an unchecked axiom.
missing=0
if [ -f "$src" ]; then
  for name in $(grep -hoE '^axiom [^ ]+' "$src" | cut -d' ' -f2); do
    if ! grep -qE "^(def|abbrev) ${name//./\\.}( |$)" "$here/Timeways/FunsExternal.lean"; then
      echo "missing in the model: $name" >&2
      missing=1
    fi
  done
fi
exit "$missing"
