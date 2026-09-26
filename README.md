# Timeways

**Azeroth remembers you.**

Timeways is a story addon for World of Warcraft: Forever. It gives your character lore on demand, a narrator who remembers you, and a journal of your adventure.

An AI model writes the words. A rules engine, [Hourglass](https://github.com/eserilev/rusty-hourglass), decides what is true. The game itself supplies the facts.

`GAMEPLAY.md` is the design and the source of truth.

## Status

Early build. The parts work and have tests, but nobody has run them together in the game yet. The Gnomish Relay bridge now starts the story program and carries its messages (relay SPEC 9.8). Real model calls (relay step 6) come next.

What works:

- **The world of each character.** An append-only history file on your computer, replayed at start.
- **`/lore <question>`.** Passages from a lore pack, under a spoiler limit: you see lore only about places you visited and people you met. A model answer is checked for citations and for names from after the Forever timeline. No real lore pack exists yet.
- **`/journal`.** A book in the look of the classic quest frame, with 4 pages: Chronicle (one chapter for each play session), Places, People, and Deeds.
- **The text you read.** The addon keeps the text of each quest, gossip window, and book that you read. `/lore` and `/talk` search it together with the lore pack, so they answer from real game text before any pack exists.
- **The narrator.** One short line in the voice of the chronicle at a big moment, such as a first kill of a rare, a level up, or a third death to the same murloc. At most 3 lines each hour.
- **Kills, deaths, and slaps.** Addons cannot read the combat log in this client. So the addon reads kills and deaths from other events, and never sends the name of a real player.

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

## A lore pack by hand

No real lore pack exists yet (GAMEPLAY.md 5.10). To try `/lore`, write passages as JSON lines, one passage for each line, and build a pack. The example is invented. Real passages come only from a source, never from memory:

```sh
cat > passages.jsonl <<'LINES'
{"text": "The tower of Testvale fell to test goblins.", "source": "https://example.test/1", "places": ["Testvale"], "npcs": ["Keeper Stubbs"]}
LINES
cargo run -q --bin timeways-pack -- passages.jsonl pack.sqlite
```

A passage needs at least one place or NPC, because a passage with no link passes every spoiler check. The builder never writes over a pack that exists.

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

The fuzz targets in `fuzz/` feed random input to the parts that read text from outside: the input lines, the files on disk, the answers of a model, `Json.lua`, and the journal pages. They need the nightly toolchain and `cargo-fuzz`:

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

## License

MIT
