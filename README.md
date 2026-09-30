# Timeways

**Azeroth remembers you.**

Timeways is a story addon for World of Warcraft: Forever. It gives your character lore on demand, a narrator who remembers you, and a journal of your adventure.

An AI model writes the words. A rules engine, [Hourglass](https://github.com/eserilev/rusty-hourglass), decides what is true. The game itself supplies the facts.

`GAMEPLAY.md` is the design and the source of truth.

## Status

Early build. The parts work and have tests, but nobody has run them together in the game yet. The Gnomish Relay bridge now starts the story program and carries its messages (relay SPEC 9.8). Real model calls (relay step 6) come next.

What works:

- **The world of each character.** An append-only history file on your computer, replayed at start.
- **`/lore <question>`.** Passages from a lore pack, under a spoiler limit: you see lore only about places you visited and people you met. A model answer is checked for citations and for names from after the Forever timeline. You build the pack on your own computer from a wiki dump (below).
- **`/journal`.** A book in the look of the classic quest frame, with 5 tabs: Hero, Chronicle (one chapter for each play session), Deeds, Knowledge, and Tasks.
- **The text you read.** The addon keeps the text of each quest, gossip window, and book that you read. `/lore` and `/talk` search it together with the lore pack, so they answer from real game text even with no pack.
- **The narrator.** One short line in the voice of the chronicle at a big moment, such as a first kill of a rare, a level up, or a third death to the same murloc. At most 3 lines each hour.
- **Kills, deaths, and slaps.** Addons cannot read the combat log in this client. So the addon reads kills and deaths from other events, and never sends the name of a real player.

## Install

Timeways has two parts: the addon in the game, and a desktop app on your computer. The addon comes from CurseForge. The desktop app, Gnomish Relay, is not on CurseForge, because CurseForge only ships addon files.

1. Install the **Timeways** addon with the CurseForge app. The project page comes with the first release.
2. Close WoW. The game only finds new addon files when it starts.
3. Run the installer of the desktop app:
   - Windows (PowerShell): `irm https://raw.githubusercontent.com/eserilev/gnomish-relay/main/scripts/install.ps1 | iex`
   - macOS and Linux (Terminal): `curl -fsSL https://raw.githubusercontent.com/eserilev/gnomish-relay/main/scripts/install.sh | sh`
4. Answer the questions of setup. Then start WoW.

When the game can't reach the desktop app, Timeways opens a setup window with these steps. Type `/timeways help` to open it again.

> **Not ready yet: the installer does not get the Timeways programs.** Today the installer sets up Gnomish Relay, and its setup finds the Timeways addon. In a later Gnomish Relay release, setup also downloads `timeways-story` and `timeways-pack` from the Timeways release, and builds the lore pack on your computer. Until then, build the programs from source (see "The story program").

## Layout

| Path | What |
|---|---|
| `addon/Timeways` | The WoW addon, Lua 5.1. `Sha256`, `Codec`, `Saved`, `Health`, `Strip`, `Slots`, and `Messages` are copies of the shared transport of Gnomish Relay, pinned in CI. Change them in the relay first. `Key.lua` comes from the setup of the bridge, and git ignores it. |
| `crates/story` | `timeways-story`, the story program on the desktop |
| `crates/addon-tests` | Runs the addon in Lua 5.1 with a fake WoW API (`addon/tests/wow.lua`) |

## The story program

```sh
cargo run -q --bin timeways-story -- <lore pack> [<data folder>]
```

It reads JSON lines on stdin and writes JSON lines on stdout. The bridge of Gnomish Relay starts it and talks to it. With no data folder, it keeps nothing after it stops.

## A lore pack

The repo holds no lore text. You build the pack on your computer from the public database dump of Wowpedia: the `pages_current` XML file, as the `.7z` archive or unpacked.

```sh
cargo run -q --release --bin timeways-pack -- from-dump wowpedia_pages_current.xml.7z pack.sqlite
```

The builder reads only the pages in `crates/story/data/pack_sources.toml`:

- The History of Warcraft books of chapters I to V. Each book passage is common knowledge.
- Some wiki pages, only the listed sections. A page links to its place, or is common knowledge.
- A paragraph of a wiki page that names a later expansion, or a person or place of one, goes out.

It prints the number of passages from each page, and names each page that the dump lacks. A missing page is skipped, not an error. The same dump always gives the same pack. The builder streams the dump, so it needs little memory, and it takes less than a minute.

### A pack by hand

To try `/lore` with your own passages, write them as JSON lines, one passage for each line. The example is invented. Real passages come only from a source, never from memory:

```sh
cat > passages.jsonl <<'LINES'
{"text": "The tower of Testvale fell to test goblins.", "source": "https://example.test/1", "places": ["Testvale"], "npcs": ["Keeper Stubbs"]}
LINES
cargo run -q --bin timeways-pack -- passages.jsonl pack.sqlite
```

A passage needs at least one place or NPC, or `"common": true`, because a passage with no link passes every spoiler check. The builder never writes over a pack that exists.

## Checks

Run these before each commit. CI runs them too.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check
stylua --check addon
selene addon/Timeways
```

`cargo test` also runs the property tests of `crates/story/tests/properties.rs`: rules that hold for any play, such as a world that reads back the same after any restart.

The fuzz targets in `fuzz/` feed random input to the parts that read text from outside: the input lines, the files on disk, the answers of a model, `Json.lua`, the journal pages, and the wikitext of a wiki dump. They need the nightly toolchain and `cargo-fuzz`:

```sh
scripts/fuzz.sh 60            # each target for 60 seconds
scripts/fuzz.sh 300 answers   # one target
```

A crash leaves its input in `fuzz/artifacts/`. Turn it into a test or a seed first, then fix the bug.

The API gate of Gnomish Relay checks that every WoW function and event that the addon uses exists in the Forever client:

```sh
../gnomish-relay/scripts/wow-api.sh --addon addon/Timeways --lint wow.yml \
  --fake addon/tests/wow.lua --api addon/tests/api.lua --signatures addon/tests/api-signatures.lua
```

## Release

A tag starts `.github/workflows/release.yml`:

1. Set the new version in `crates/story/Cargo.toml`. The tag must match it.
2. Push a tag such as `v0.2.0`.

The job runs the checks, builds `timeways-story` and `timeways-pack` for Linux (x86_64), macOS (arm64 and x86_64), and Windows (x86_64), and makes a GitHub release with these files:

| File | What |
|---|---|
| `timeways-<target>.tar.gz`, or `.zip` for Windows | `timeways-story`, `timeways-pack`, and `LICENSE` |
| `timeways-addon.zip` | The addon, as the BigWigs packager builds it for CurseForge |
| `<file>.sha256` | The SHA-256 sum of each archive |
| `SHA256SUMS` | The sums of all archives |
| `timeways-manifest.json` | The version, the archive and sum for each target, and the addon zip |

The packager reads `.pkgmeta` and writes the tag into `## Version` of the TOC. It uploads the addon to CurseForge only when two things are set:

- `## X-Curse-Project-ID` in `addon/Timeways/Timeways.toc` holds the real project id. With `0`, the packager only builds the zip.
- The repository has the secret `CF_API_KEY`, a CurseForge API token.

`Key.lua` never goes into the zip. The bridge writes it on each computer. `scripts/check-addon-zip.py` checks the zip against the TOC.

## License

MIT
