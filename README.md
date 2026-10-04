# Timeways

**Azeroth remembers you.**

Timeways is a story addon for World of Warcraft: Forever. It gives your character lore on demand, a narrator who remembers you, and a journal of your adventure.

An AI model writes the words. A rules engine, [Hourglass](https://github.com/eserilev/rusty-hourglass), decides what is true. The game itself supplies the facts.

`GAMEPLAY.md` is the design and the source of truth.

## Status

Early build. The parts work together through Gnomish Relay, and the bridge runs the model calls. Tests in the game have started (`TESTING.md`).

What works:

- **The world of each character.** An append-only history file on your computer, replayed at start.
- **Setup in the game.** When the game can't reach the desktop app, a setup window shows the install steps. The key of each computer comes from the `Timeways_Key` addon that the desktop app writes.
- **`/lore <question>`.** The answer opens in a small lore book, with the last questions to browse. Passages from a lore pack, under a spoiler limit: you see lore only about places you visited and people you met. A model answer is checked for citations and for names from after the Forever timeline. You build the pack on your own computer from a wiki dump (below).
- **The text you read.** The addon keeps the text of each quest, gossip window, and book that you read. `/lore` and `/talk` search it together with the lore pack, so they answer from real game text even with no pack.
- **`/talk`.** Talk to the NPC that you target. It answers from what it knows, and its trust in you changes. The tooltip of the NPC shows the trust.
- **`/quest`.** An NPC offers a side quest made for you: visit a place, meet an NPC, or kill a creature that you saw. The giver and the steps show as pins on the map of the journal.
- **Player quests.** Give a quest to a player in your party, guild, or friends list who has Timeways. The game checks the steps, and the giver decides at the end. You type each step in plain words, and "Help me write" drafts a whole quest.
- **Roleplay profiles.** Share your character's name, title, looks, and story with players who use Total RP 3, MyRolePlay, or XRP, and see their names in the tooltip.
- **Remembered players.** Right-click a player to mark them Friendly, Neutral, or Avoid, and add a note. Only you see it, in their tooltip.
- **Player stories.** Target someone in your group and type `/story <words>` to tell a story about them. They accept it into their own story, or decline it.
- **`/journal`.** A book in the look of the classic quest frame, with 5 tabs: Hero (your hero in your own words), Chronicle (one chapter for each milestone, with a saga from the narrator), Deeds, Knowledge, and Quests. The map shows where you have been.
- **The narrator.** One short line in the voice of the chronicle at a big moment, such as a first kill of a rare, a level up, or a third death to the same murloc. At most 3 lines each hour.
- **Kills, deaths, and slaps.** Addons cannot read the combat log in this client. So the addon reads kills and deaths from other events, and never sends the name of a real player.

## Install

Timeways has two parts: the addon in the game, and a desktop app on your computer (Gnomish Relay). The addon comes only from CurseForge. The desktop app never installs or changes the addon folder.

### What you need

- WoW: Forever, and the CurseForge app.
- Windows 10 or later, macOS, or Linux (x86_64).
- For the story text, an AI model on your computer: the `claude` program (Claude Code), Ollama, or LM Studio. Without one, Timeways still works: lore shows the passages as they are, and the chapters show plain lists.
- About 200 MB of free disk space for a short time: setup downloads the Wowpedia dump (about 133 MB) to build the lore, then deletes it.

### Steps

1. In the CurseForge app, install **Timeways**. CurseForge installs **Gnomish Relay** with it.
2. Start WoW. At the first login, Timeways checks for its desktop app. When the app is missing, a window shows the one line that installs it.
3. Copy that line, and run it:
   - Windows: press Windows+R, paste the line, and press Enter.
   - Mac or Linux: open Terminal, paste the line, and press Enter.
4. Answer the questions of setup. It asks only what it can't find out itself. With no AI model on your computer, it offers a free one.
5. Restart WoW, so the game loads the key that setup wrote.

The lines, for reference:

- Windows: `powershell -c "irm https://eserilev.github.io/timeways/install.txt | iex"`
- Mac or Linux: `curl -fsSL https://eserilev.github.io/timeways/install.sh | sh`

Setup finds WoW and the Timeways addon, writes the key, installs the story program, and picks the model that it finds. It then builds the lore on your computer. Timeways checks its desktop app at each login, and opens the setup window again only when something is wrong.

### Later

- `gnomish-relay update` updates the desktop app and the Timeways programs.
- `gnomish-relay setup --timeways` installs the story program again, and builds fresh lore.

## Layout

| Path | What |
|---|---|
| `addon/Timeways` | The WoW addon, Lua 5.1. `Sha256`, `Codec`, `Saved`, `Health`, `Strip`, `Slots`, `Messages`, and `KeyHandoff` are copies of the shared transport of Gnomish Relay, pinned in CI. Change them in the relay first. `KeyHandoff.lua` takes the key from `Timeways_Key`, an addon that the desktop app writes outside this folder, so a CurseForge update keeps it. |
| `docs` | The short install scripts on GitHub Pages. They run the Gnomish Relay installer with `--timeways`. |
| `crates/story` | `timeways-story`, the story program on the desktop |
| `crates/addon-tests` | Runs the addon in Lua 5.1 with a fake WoW API (`addon/tests/wow.lua`) |

## The story program

```sh
cargo run -q --bin timeways-story -- <lore pack> [<data folder>]
```

It reads JSON lines on stdin and writes JSON lines on stdout. The bridge of Gnomish Relay starts it and talks to it. With no data folder, it keeps nothing after it stops.

## A lore pack

The repo holds no lore text. You build the pack on your computer from the public database dump of Wowpedia: the `pages_current` XML file, as the `.7z` archive or unpacked. The Gnomish Relay setup does this for players. By hand:

```sh
curl -fLO https://s3.amazonaws.com/wikia_xml_dumps/w/wo/wowpedia_pages_current.xml.7z
```

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

The fuzz targets in `fuzz/` feed random input to the parts that read text from outside: the input lines (`input`), the files on disk (`store`), the answers of a model (`answers`), `Json.lua` (`json_lua`), the journal pages (`pages`), the replies from the desktop in the addon (`replies`), the game text that `Seen.lua` keeps (`seen`), the wikitext of a wiki dump (`wikitext`), the addon messages of player tasks (`task_wire`), and random play with player tasks (`task_play`). They need the nightly toolchain and `cargo-fuzz`:

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
2. Add a short entry for the new version at the top of `CHANGELOG.md`. CurseForge shows it.
3. Push a tag such as `v0.2.0`. A tag with a `-`, such as `v0.2.0-rc.1`, also makes a normal release, so testers install it with the same line as players.

The job runs every check of CI, builds `timeways-story` and `timeways-pack` for Linux (x86_64), macOS (arm64 and x86_64), and Windows (x86_64), and makes a GitHub release with these files:

| File | What |
|---|---|
| `timeways-<target>.tar.gz`, or `.zip` for Windows | `timeways-story`, `timeways-pack`, and `LICENSE` |
| `timeways-addon.zip` | The addon, as the BigWigs packager builds it for CurseForge |
| `<file>.sha256` | The SHA-256 sum of each archive |
| `SHA256SUMS` | The sums of all archives |
| `timeways-manifest.json` | The version, the addon version (`app_version`), the archive and sum for each target, and the addon zip |

The desktop app updates itself from these files. When the CurseForge app updates the addon, the desktop app reads `## Version` in `Timeways.toc` and installs the release with that tag. So two rules hold for each release:

- Keep `## Version: @project-version@` in `Timeways.toc`. The packager writes the tag there.
- Attach `timeways-manifest.json` to every release, at its own tag.

If a job fails, run the workflow again. It keeps the draft release and replaces the files that it uploaded before.

`scripts/check-release-manifest.py` checks the release folder with the rules of the setup of Gnomish Relay. Each push to CI runs the release steps on fake programs (`scripts/release-dry-run.sh`), so a tag finds no surprise. A test checks that `ns.App.version` is in the range of the pinned bridge.

The packager reads `.pkgmeta` and writes the tag into `## Version` of the TOC. It uploads the addon to CurseForge only when two things are set:

- `## X-Curse-Project-ID` in `addon/Timeways/Timeways.toc` holds the real project id. With `0`, the packager only builds the zip.
- The repository has the secret `CF_API_KEY`, a CurseForge API token.

The key of each computer never goes into the zip. The desktop app writes it into the `Timeways_Key` addon, and `KeyHandoff.lua` takes it from there. `scripts/check-addon-zip.py` checks the zip against the TOC, and refuses a zip with a `Key.lua`.

## License

MIT
