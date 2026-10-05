# Plan: NPCs know only the lore of their place and their work

Status: draft 1, 2026-10-03. The user approved the idea. Nothing is built. When a part is built, its rules move into `GAMEPLAY.md` (section 15 of this plan), and this plan marks the part as done.

The idea comes from the Skyrim mod CHIM: each NPC gets only the lore that its character knows.

This plan builds on three plans and does not repeat them:

- `docs/plans/npc-memory.md` owns the memory block of a talk.
- `docs/plans/talk-window.md` owns the conversation, its turns, and the options.
- `docs/plans/standing.md` owns the town of an NPC, standing, and the talk of the town.

This plan owns one thing: **which lore passages go into a talk prompt, and how.**

## 1. Goal

Today `/talk` gives an NPC the best 3 passages for the player's words, from the whole pack and from the text that the player read. Only the spoiler limit (3.1) filters them. So a farmer of Goldshire explains the politics of Ironforge.

After this plan, each passage gets one of three answers for the NPC of the talk:

| Answer | In the prompt |
|---|---|
| Knows | in the block "Lore that you know" |
| Rumor | in a second block, as hearsay |
| Does not know | not in the prompt |

Three inputs decide:

1. **Place.** An NPC knows its own zone well, its continent less, and the wider world only as rumor or not at all.
2. **Kind.** A scholar knows history. A guard knows the dangers of its land. An innkeeper hears the news of travellers. Any other NPC is a local: it knows its land.
3. **Faction.** An NPC does not know the inner affairs of the other faction.

When a question goes past what the NPC knows, the NPC says so in character: "Ironforge? Never been. Ask a dwarf." It never invents an answer.

`/lore` does not change. The lore book is a local historian, and it keeps its own rules (3.1, 3.1.1).

## 2. What changes, in short

| Part | Change |
|---|---|
| Addon, `Sightings.lua` | `npc_seen` gets two optional fields: `title` (the title line of the NPC) and `faction` (`"alliance"` or `"horde"`). |
| Relay | No change. `npc_seen` is a game event, and the bridge passes events as free JSON (section 3.3). |
| `input.rs` | `Input::NpcSeen` gets `title` and `faction`. A bad `faction` reads as none. |
| `vocabulary.rs` | New fact `npc_title`. The declared but unused fact `member_of` now holds the faction of an NPC. `VERSION` goes up by one. |
| `character.rs` | `see_npc` keeps the title and the faction. New readers `title_of` and `faction_of`. |
| `npc_kind.rs` (new) | The kind of an NPC from its title or its name. Pure. |
| `zones.rs` (new) | The continent and the faction of each zone, from a data file. Pure. |
| `npc_knowledge.rs` (new) | The reach of a passage, and the rule of section 6. Pure. |
| `pack.rs` | `Passage` gets `scope: Scope` (faction and history). The pack format goes from 1 to 2. The reader takes 1 and 2. |
| `pack_sources.rs`, `pack_sources.toml` | A page can name a `faction` and set `history`. Each book passage is history. |
| `story.rs` | `passages_for` splits in two. `fn talk` filters the candidates by knowledge before the cap of 3. |
| `talk.rs` | `Scene` gets the title and the kind. The prompt gets a rumor block and the rule to say "I don't know". |
| Data | `crates/story/data/npc_kinds.toml` and `crates/story/data/zones.toml`. |
| Samples | One new golden sample in `npc_replies.txt`: an NPC that does not know. |

No new table, and no migration of the store. The fact vocabulary changes, so `VERSION` goes up (section 4.3).

## 3. Where the kind and the faction of an NPC come from

### 3.1 What the addon can read

The Forever client is the Mainline client at the classic expansion level (`WOW_PROJECT_MAINLINE`, `LE_EXPANSION_CLASSIC`, build 1.60.1). Checked in the API dump of `target/wow-api` (KethoDoc, 1.60.1.69913):

| Need | API | In `addon/tests/api.lua` today |
|---|---|---|
| The title line ("Innkeeper") | `C_TooltipInfo.GetUnit(unit)`: `data.lines[i].leftText` | no. `TooltipDataProcessor` and `Enum.TooltipDataType.Unit` are there, for the trust line. |
| Which line is the level line | `TOOLTIP_UNIT_LEVEL` ("Level %s", one string for each client language) | no |
| The faction | `UnitFactionGroup(unit)`: `"Alliance"`, `"Horde"`, or nothing | no |

There is no `UnitTitle` for an NPC. The title exists only as a line of the tooltip. So the addon reads the tooltip data, not the frame `GameTooltip`. `C_TooltipInfo.GetUnit` works with no tooltip on the screen.

**Which line is the title.** Line 1 is the name. For an NPC with a title, line 2 is the title, and the level line comes after it. For an NPC with no title, line 2 is the level line. The rules:

- The title is line 2, when line 2 does not start with the level words. The level words are `TOOLTIP_UNIT_LEVEL` up to `%s`: "Level " in English.
- A pair of angle brackets around the title goes: "<Innkeeper>" becomes "Innkeeper". The classic client shows them, and Mainline does not. The rule takes both.
- A secret value (`issecretvalue`), a value that is not a string, or an empty string gives no title.
- A title that holds the name of the player is never sent (5.11). The client never puts one there for an NPC, so this is a guard only.

The addon sends the title as it reads it, in the language of the client. The desktop maps it to a kind (section 5).

**The faction.** `UnitFactionGroup(unit)` gives `"Alliance"` or `"Horde"` for an NPC of a faction. A neutral NPC gives nothing, or `"Neutral"`. The addon sends `"alliance"` or `"horde"`, and nothing in every other case.

**What to check in the game first** (open question 1): the title line of an innkeeper, a guard, and an NPC with no title; the faction of a Stormwind guard, a Booty Bay goblin, and a Crossroads innkeeper.

### 3.2 The line: `npc_seen`, not `talk_asked`

```json
{"type":"npc_seen","at":1790000000,"name":"Innkeeper Farley","reaction":"friendly","creature":"humanoid","title":"Innkeeper","faction":"alliance"}
```

- `title` is optional: 1 to 96 bytes (`MAX_NAME_BYTES`), with no control character. A title over the limit, or with a control character, is dropped by the addon. The story program refuses a line with a bad title, with `StoryError::BadName`, as for a bad name.
- `faction` is optional: `"alliance"` or `"horde"`. The story program reads any other value as none (`faction::lenient`, the same pattern as `spot::lenient`). So a new value from a later addon never refuses the line.

**Why `npc_seen`.** Each `/talk` needs a target. A target is a sighting, so the `npc_seen` of the NPC goes into the outbox before the `talk_asked` line. `Talk.Ask` flushes the outbox, so both go in the same batch, in this order. The addon sends each NPC once in a session (`sent[id]`), so a talk later in the session finds the title in the world already.

**Why not `npc_met`.** `npc_met` comes from the gossip window. To open a gossip window you hover the NPC first, and the hover sends `npc_seen`. So `npc_met` adds nothing. It does not change.

**Why not `talk_asked`.** See 3.3.

### 3.3 The relay

- `talk_asked` has a fixed shape in the bridge (`Known::TalkAsked`, `deny_unknown_fields`, `app-protocol/src/addon_lines.rs`). A new field there needs a release of the relay first.
- `npc_seen` is a game event. The bridge checks only its size, its depth, its key count, and its strings, and passes it as free JSON ("So a new event of Timeways needs no change here"). Two more keys stay far under the limits: 4096 bytes and 64 keys.

So this plan needs **no relay change**. Following the project rule, tell the Gnomish Relay session anyway that `npc_seen` gets `title` and `faction`, so its fake addon can send them.

## 4. The world: title and faction of an NPC

### 4.1 Facts

| Fact | Holder, target | Shape | Meaning |
|---|---|---|---|
| `npc_title` (new) | Person to Thing | flag that can end | The title line of the NPC. The Thing is named by the title, as the game gives it. |
| `member_of` (declared today, unused) | Person to Faction | flag that can end | The faction of the NPC. The Faction entities are "Alliance" and "Horde". |

The world keeps the raw title, not the kind. So a change of `npc_kinds.toml` applies to every NPC at once, with no migration.

**The last sighting decides**, as for `hostile`. Two NPCs can share a name, and Blizzard can change a title. So a sighting with another title ends the old `npc_title` fact and starts the new one. A sighting with no title changes nothing: the client hid it, or the NPC has none, and the code cannot tell the two apart. The same holds for `member_of`.

**The trap of a shared Thing.** The Thing "Innkeeper" is one entity for every innkeeper. A quest of the game with the same name is the same Thing. That does no harm: the code reads the title only through `npc_title`.

### 4.2 `character.rs`

```rust
/// The newest title line of the NPC, as the game gave it.
pub fn title_of(&self, npc: &str) -> Option<&str>

/// The faction that the game gave the NPC. None for a neutral NPC, or one seen before the
/// addon sent factions.
pub fn faction_of(&self, npc: &str) -> Option<Faction>
```

`see_npc` takes `title: Option<&str>` and `faction: Option<Faction>`, and calls two new private functions, `set_title` and `set_faction`. Each one ends the old fact and starts the new one only when the value changes.

### 4.3 The version

`VERSION` of `vocabulary.rs` goes up by one. `standing.md` also takes version 8. The plan that lands first takes 8, and the other takes 9. `member_of` changes no declaration, so only `npc_title` needs the version.

## 5. The kind of an NPC

### 5.1 The kinds

```rust
/// What an NPC knows by its work (docs/plans/npc-knowledge.md 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum NpcKind {
    #[default]
    Local,
    Guard,
    Host,
    Scholar,
}
```

| Kind | Who | What it knows (section 6) |
|---|---|---|
| Local | every NPC with no known title word: farmers, smiths, vendors, trainers of most kinds | its land |
| Guard | guards, watchmen, officers | its land and its dangers |
| Host | innkeepers, flight masters, stable masters, barkeeps | its land, and the news of travellers |
| Scholar | librarians, historians, mages | history, and far lands |

The order of the enum is the order of priority (5.3).

### 5.2 The data file: `crates/story/data/npc_kinds.toml`

A data file, not code. A person checks each word. The first list is small on purpose.

```toml
# The words of a title line or a name that give an NPC its kind (docs/plans/npc-knowledge.md 5).
# A word matches as a whole word or a whole phrase, in any case. An NPC with no word is a local.
# The words are English, as the client of the player shows them. Open question 3 covers other languages.

scholar = [
    "Librarian", "Historian", "Archivist", "Scholar", "Lorekeeper", "Chronicler",
    "Mage", "Archmage", "Magus", "Portal Trainer",
]
host = [
    "Innkeeper", "Barkeep", "Bartender", "Flight Master", "Gryphon Master",
    "Hippogryph Master", "Wind Rider Master", "Bat Handler", "Stable Master",
]
guard = [
    "Guard", "Sentinel", "Watchman", "Deputy", "Marshal", "Sheriff", "Captain",
    "Lieutenant", "Sergeant", "Grunt", "Deathguard", "Bluffwatcher", "Protector",
]
```

`standing.md` 9.2 has `lawful_titles.txt`, with words that overlap the guard list. The two lists answer different questions: who refuses a quest, and what an NPC knows. They stay two files. Open question 5 asks whether to merge them.

### 5.3 The rules: `crates/story/src/npc_kind.rs`

```rust
/// The kind of an NPC from its title line, then its name.
pub fn kind_of(kinds: &Kinds, title: Option<&str>, name: &str) -> NpcKind

/// The kinds of the bundled data file. A test reads it, so it is not broken.
pub fn bundled() -> Result<Kinds, KindsError>

/// The kind of the words of one text, or None.
fn kind_in(kinds: &Kinds, text: &str) -> Option<NpcKind>
```

1. The title line comes first. "Innkeeper" gives Host.
2. With no title, or a title with no known word, the name decides. "Marshal Dughan" gives Guard, and "Innkeeper Farley" gives Host. Many classic NPCs carry their work in the name and have no title line.
3. With no word in either, the NPC is a Local.
4. **Two kinds in one text** ("Mage Trainer and Innkeeper"): the kind with the highest priority wins, Scholar first. The NPC with two works knows the most of both.
5. A title line with a known word wins over a name with another known word. The title line is what the game says the NPC does now.

A word matches as a whole word: "Guard" matches "Stormwind City Guard", and not "Guardian". A phrase matches as whole words in order: "Flight Master". The match ignores ASCII case. The split is the split of `pack::match_query`: any character that is not a letter or a digit.

## 6. The knowledge rule

### 6.1 The continent and the faction of a zone: `crates/story/data/zones.toml`

The pack has no continent and no faction. The game files hold them (`AreaTable`, 5.10), but nothing in the repo reads the game files yet. So a short data file holds them, and a person checks it. It holds names only, no lore text.

```toml
# The continent and the faction of each zone (docs/plans/npc-knowledge.md 6.1).
# `faction` is "alliance", "horde", or "contested". An instance names the zone of its door in `near`.
# A zone that is not here has no continent: an NPC knows it only when it lives there.

"Elwynn Forest" = { continent = "Eastern Kingdoms", faction = "alliance" }
"Stormwind City" = { continent = "Eastern Kingdoms", faction = "alliance" }
"Tirisfal Glades" = { continent = "Eastern Kingdoms", faction = "horde" }
"Stranglethorn Vale" = { continent = "Eastern Kingdoms", faction = "contested" }
"Durotar" = { continent = "Kalimdor", faction = "horde" }
"The Deadmines" = { continent = "Eastern Kingdoms", faction = "contested", near = "Westfall" }
# ...every zone, capital, and instance of the classic world, about 70 lines.
```

The first list holds the zones and capitals of the classic world, and the instances of the Forever phase. The list of zone phases (5.10) keeps the zones that are not open out of the pack, so a zone here does not leak anything.

```rust
pub struct ZoneInfo { pub continent: Continent, pub faction: Option<Faction>, pub near: Option<String> }

/// The zones of the bundled data file.
pub fn bundled() -> Result<Zones, ZonesError>

impl Zones {
    pub fn get(&self, zone: &str) -> Option<&ZoneInfo>
}
```

`Continent` is a newtype over the name. `Faction` is `enum Faction { Alliance, Horde }`, in `zones.rs`. "contested" reads as `None`.

### 6.2 The inputs

```rust
/// What the NPC of a talk is, for the rule.
pub struct Knower {
    pub kind: NpcKind,
    pub faction: Option<Faction>,
    pub in_capital: bool,
}

/// How far the place of a passage is from the NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reach { Home, Continent, World }

/// What a passage is, for the rule.
pub struct About {
    pub reach: Reach,
    pub common: bool,
    pub history: bool,
    /// The faction of the land of its nearest place.
    pub land_of: Option<Faction>,
    /// The inner affairs of this faction, from the pack.
    pub affairs_of: Option<Faction>,
    /// The NPC itself said it: a gossip or a quest text of this NPC.
    pub said_by_npc: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Knowledge { DoesNotKnow, Rumor, Knows }
```

The order of `Knowledge` lets the rule take the lower or the higher of two answers with `min` and `max`.

### 6.3 The rule: `crates/story/src/npc_knowledge.rs`

```rust
/// What the NPC of a talk knows of a passage (GAMEPLAY.md 3.5.1).
pub fn knowledge(npc: &Knower, about: &About) -> Knowledge {
    if about.said_by_npc {
        return Knowledge::Knows;
    }
    let level = by_place(npc, about.reach).max(by_topic(npc.kind, about));
    level.min(faction_cap(npc.faction, about))
}
```

**By place** (`fn by_place`):

| Kind | Home | Continent | World |
|---|---|---|---|
| Local | Knows | Rumor | Does not know |
| Guard | Knows | Rumor | Does not know |
| Host | Knows | Rumor | Rumor |
| Scholar | Knows | Knows | Rumor |

- **A capital hears more.** In a capital (`in_capital`), the World column of a Local and a Guard is Rumor. Ships, trams, and trade bring news.
- A Guard and a Local share the table. They differ in the persona line (section 7.1). A topic of "danger" for passages is open question 4.

**By topic** (`fn by_topic`):

- A history passage: Knows for a Scholar, Rumor for every other kind.
- A common passage that is not history: Knows. It is what everyone knows in 25 ADP (5.10).
- Any other passage: Does not know. The place decides alone.

**The faction cap** (`fn faction_cap`). With no faction for the NPC (neutral), there is no cap. Otherwise, for the other faction:

| The passage is | Cap |
|---|---|
| about the inner affairs of the other faction, and common | Rumor |
| about the inner affairs of the other faction, and not common | Does not know |
| about a place in the land of the other faction | Rumor |
| anything else | no cap |

So a Stormwind guard has heard of the Banshee Queen (common, Horde affairs), never knows the politics of Brill (Horde affairs, not common), and has only heard of Darkshore when it is a Horde guard.

### 6.4 The reach of a passage

```rust
/// The home of the NPC: where it lives, its zone, and its continent.
pub struct Home<'a> { pub place: Option<&'a str>, pub zone: Option<&'a str>, pub continent: Option<&'a Continent> }

/// Where one place of a passage is.
pub struct Located<'a> { pub zone: &'a str, pub continent: Option<&'a Continent> }

/// The nearest place of the passage decides. With no place, the reach is World.
pub fn reach(home: &Home<'_>, places: &[Located<'_>]) -> Reach
```

- **Home**: the place is in the zone of the NPC. A subzone counts by its zone, so Goldshire and Northshire Abbey are both home to an NPC of Elwynn Forest. An instance counts as the zone of its `near`.
- **Continent**: the zones differ, and both have the same continent in `zones.toml`.
- **World**: any other case, also a zone with no continent, and a passage with no place.
- **The nearest place decides.** A passage about Goldshire and Ironforge is home to an NPC of Goldshire: it is about its own land in part.

The glue reads the world, so it lives in `story/knowledge.rs`, not in the pure module:

```rust
/// The rule's view of a passage, for the NPC of a talk.
fn about(passage: &Passage, character: &Character, zones: &Zones, npc: &str) -> About
```

- The places of a passage are its `Link::Place` links, and the place of each `Link::Npc` that the world holds (`place_of`). The zone of each place is `zone_of_place`. Every place of a passage that passed the spoiler limit is in the world, so its zone is known.
- `land_of` is the faction of the zone of the nearest place.
- `affairs_of` is `passage.scope.faction`.
- `history` is `passage.scope.history`. `common` is true when the links hold `Link::Common`.
- `said_by_npc` is true when the passage is a seen text (`Origin::Read`) with `Link::Npc(npc)`.

And the NPC:

```rust
fn knower(character: &Character, zones: &Zones, kinds: &Kinds, npc: &str) -> (Knower, Home<'_>)
```

- `kind` is `kind_of(kinds, character.title_of(npc), npc)`.
- `faction` is `character.faction_of(npc)`. With none, it is the faction of the land of the NPC's zone. A contested zone gives none: the NPC is neutral.
- `in_capital` is `places::is_capital` of the zone of the NPC.
- `Home` comes from `place_of`, `zone_of_npc`, and `zones.toml`.

## 7. The prompt

### 7.1 The persona

`persona` gets the title in the fence, after the place:

```
<<<
Name: Innkeeper Farley
Place: Goldshire
Title: Innkeeper
>>>
```

The title comes from the game, through the addon, so it is data in the fence. A title cannot close the fence (`house::fenced`, test in section 11).

After the fence, one sentence for the kind. These are internal prompt text, not UI copy:

| Kind | Sentence |
|---|---|
| Local | You know your own land and its people well. Of far places you know little. |
| Guard | You keep watch here. You know the dangers of this land. |
| Host | Travellers pass through, and you hear their news. |
| Scholar | You have read much of the history of the world. |

`quest.rs` also calls `persona`. It passes no title, and the quest prompt does not change (open question 6).

### 7.2 The two blocks

`what_you_know` takes the passages with their answers. A passage that the NPC does not know is not there at all.

```
Lore that you know:
<<<
- The inn of Goldshire stands at the crossroads of Elwynn.
>>>

What you only heard from travellers. Speak of it as hearsay, and you can be wrong:
<<<
- They say the dwarves of Ironforge dig ever deeper.
>>>
```

The rumor block shows only when it has a passage.

### 7.3 The rule to say "I don't know"

The sentence "Stay true to the lore below. When you do not know, say so as this person would." becomes:

```
Stay true to the lore below. You know only that, and what anyone of your place knows. When the player asks about anything else, say in your own voice that you do not know: you never went there, or it is none of your business. Never make up an answer.
```

### 7.4 A golden sample

`crates/story/data/samples/npc_replies.txt` gets one sample. Like every sample, it names no real place or person:

```
The capital? Never been past the river, and I never will. Ask someone who has seen it.
```

The samples rotate by turn, so the model sees this sample in some talks. It shows the manner, not the facts.

### 7.5 Size

| Part | Tokens, at most |
|---|---|
| The title line in the persona | 30 (a title of 96 bytes) |
| The kind sentence | 20 |
| The heading of the rumor block | 20 |
| The new rule, less the old sentence | 30 |

So a talk prompt grows by about 100 tokens. The first turn of talk-window.md with the talk of the town of standing.md takes 1112 tokens. With this plan it takes about 1212, under the limit of 1773 (`tokens::Call::Talk`). The passages stay at 3, so their room does not change.

## 8. The choice of passages

Today `passages_for` searches, applies the spoiler limit, and cuts to the size. It serves `/lore` and `/talk`. It splits in two:

```rust
/// The seen texts first, then the pack passages that pass the spoiler limit.
fn candidates(pack, seen, character, words, target) -> Result<Vec<Passage>, StoryError>

/// The candidates that fit the limits of the bridge and the room of an answer.
fn fitted(found: Vec<Passage>) -> Vec<Passage>
```

- `/lore`: `fitted(candidates(...))`. The same as today.
- `/talk`: `candidates`, then the knowledge filter, then `fitted`, then the cap of 3.

**The filter comes before the cap.** Today the cap of 3 comes after the cut to 8. With the filter after the cap, 3 far passages leave the prompt empty while a near one waits at place 4. So the filter runs on all candidates (`CANDIDATES` = 50).

**The order in a talk.** The passages that the NPC knows come first, then the rumors. Inside each group, the order of the search stays: seen text first, then the pack by rank. **At most 1 rumor** goes into a talk prompt (`MAX_TALK_RUMORS`). A rumor is a hint, not the body of an answer.

```rust
/// The passages of a talk, each with what the NPC knows of it.
fn talk_passages(candidates: Vec<Passage>, judge: impl Fn(&Passage) -> Knowledge) -> Vec<(Passage, Knowledge)>
```

`talk_passages` is pure, so the property tests reach it without a world.

## 9. How seen text fits

A seen text is the game text that the player read (5.10). It is canon, and it passes the spoiler limit. The NPC of a talk does not know all of it:

- **A text of this NPC** (its gossip, the quest that it gave you): Knows, always. The NPC said it. This holds also when the quest text names a far place: "Take this letter to Ironforge" stays known to the giver.
- **A text read in the zone of the NPC** (a book in Goldshire, the gossip of a neighbour): Home, so Knows.
- **A text read in another zone:** the rule of section 6, by its zone.
- **A text with no zone:** World. Only a Host or a Scholar has heard of it.

A quest text names a far place, and the NPC knows the text. That gives the NPC no other passage about the far place. Ironforge passages of the pack still go through the rule.

## 10. Proof (5.14)

The answer of the rule rests on rows of the world, so the talk call reads them. More is the safe side.

| Input of the rule | Row | Read today? |
|---|---|---|
| The title and the faction of the NPC | the events that opened `npc_title` and `member_of` on the NPC | yes: `events_about(npc)` reads every fact that the NPC holds |
| The place of the NPC | the event that opened its `located_in` | yes, the same way |
| The places of each passage in the prompt | `events_about` of each place, and of each linked NPC | **new**: `reads::places_read(active, &passages)` |
| A seen text in the prompt | its `learned` row | yes: `passages_read` |

- The rule reads `zones.toml` and `npc_kinds.toml`. These are data of the program, not rows of a world, like the pack. The call keeps the pack label already. It needs no data label: the raw title is in the world, so the kind can always be found again.
- A passage that the NPC does not know is not in the prompt. The call does not read its rows.
- The table of 5.14 gets one more cell. "Talk: the events behind the NPC, the hero entries about it, the `learned` rows of its passages, and the events behind the places of its passages."

## 11. Edge cases

| Case | What happens |
|---|---|
| An NPC with two kinds in its title or name | The kind with the highest priority wins: Scholar, then Host, then Guard (5.3). |
| A title with a word that is in no list ("Blacksmith", "General Goods") | The name decides (5.3, rule 2). "Guard Thomas" with the title "Blacksmith" is a guard. With no word in the name either, a Local. |
| An NPC with no title | The name decides. With no word in the name, a Local. |
| A title in another client language ("Aubergiste") | No word matches, so a Local (open question 3). |
| A title that the client hides | No `title` field. The fact from an older sighting stays. |
| An NPC in a capital | Home is the whole capital, also its districts. The World column of a Local or a Guard is Rumor (6.3). |
| An NPC in an instance | Its home is the instance. With `near` in `zones.toml`, it is the zone of the door. |
| An NPC with no place in the world | `Home` is empty. Every passage is World, except its own text. |
| An NPC met only by a sighting | The sighting carries the title and the faction, so the NPC has them. A talk is a meeting anyway. |
| An NPC seen before this change | No title, no faction. The first sighting of the next session fills them. Until then: a Local by its name, with the faction of its land. |
| A neutral NPC (Booty Bay) | No faction, and a contested zone. No faction cap. |
| A quest giver whose quest text names a far place | The giver knows its own text (9). Pack passages about the far place go through the rule. |
| An NPC that walks to another zone | Its home follows its newest place (`place_of`), as standing does. |
| A zone that is not in `zones.toml` | No continent. Only Home counts: every other place is World. |
| A passage about two places | The nearest place decides (6.4). |
| A pack of format 2 | It reads with no scope: no history mark, no faction affairs. Books then count as common, so every NPC knows them. The next build of the pack fixes it. |
| The player names a far place in the question | The words still search the pack. The NPC gets what its rule allows. The model gets the rule to say "I don't know". |
| The model knows the place from its training | The prompt rule and the sample are the only guard in this plan. A check of the answer is open question 2. |
| A later turn of a conversation (talk-window.md) | Each turn calls the same filter. The rule is the same for each turn, so the NPC never knows more in turn 5 than in turn 1. |
| The player's hero text and the memories | They do not change. They are the player's story and the NPC's past with the player, not lore. |

## 12. The pack: scope

### 12.1 `pack.rs`

```rust
/// What a passage is about, past its links. The spoiler limit never reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    /// The inner affairs of this faction.
    pub faction: Option<Faction>,
    /// The deep past, such as a History of Warcraft book.
    pub history: bool,
}
```

- `Passage` gets `#[serde(skip)] pub scope: Scope`.
- The scope is in the table `link`, as two new kinds: `faction` (with the name `alliance` or `horde`) and `history` (with an empty name). The reader puts them in `scope`, never in `links`.
- **Why not in `links`.** A link gates the spoiler limit, and `Pack::write` refuses a passage with no link. A scope row is not a gate. A passage with only a scope must still be refused, or it leaks.
- `FORMAT_VERSION` goes to 3: format 2 is taken by `about`, the subject of a page (GAMEPLAY.md 5.10, 2026-10-05). The writer writes 3. The reader takes 2 and 3 (`READABLE_VERSIONS`). Format 2 is a known older format, not a guess, so the rule "never guessed at" holds. A pack of format 2 has no scope rows.
- **Why read format 2.** `lore_start::open_lore` fails the start of the program when `Pack::open` fails. A player with a pack of format 2 and a new story program then has no Timeways until the desktop builds a new pack. Reading format 2 avoids that.

### 12.2 `pack_sources.toml`

- A book passage gets `history`. Books are common already.
- A page can set `faction = "horde"` or `faction = "alliance"`: the page is about the inner affairs of that faction. A page can set `history = true`.
- An unknown faction is refused when the list is read, before the dump, as a page with no link is.

The first marks, for the pages of the list today:

| Page | Mark |
|---|---|
| Forsaken | `faction = "horde"` |
| Undercity | `faction = "horde"` |
| Sylvanas Windrunner | `faction = "horde"` |
| Brill | `faction = "horde"` |
| Deathknell | `faction = "horde"` |
| Tirisfal Glades | `history = true` (most of its sections tell of the past) |

## 13. Tests

Every test name is a sentence. No test needs the game or a real model, except the one marked `#[ignore]`.

### 13.1 `crates/story/tests/npc_knowledge.rs` (new)

The rule:

- `a_local_knows_its_own_zone_well`
- `a_local_knows_a_subzone_of_its_zone_well`
- `a_local_knows_its_continent_only_as_rumor`
- `a_local_does_not_know_another_continent`
- `a_guard_knows_its_zone_and_hears_of_its_continent`
- `a_host_hears_of_another_continent_as_rumor`
- `a_scholar_knows_its_continent_well`
- `a_scholar_knows_another_continent_only_as_rumor`
- `a_scholar_knows_history_well`
- `a_local_knows_history_only_as_rumor`
- `common_knowledge_is_known_by_every_kind`
- `an_npc_in_a_capital_hears_of_other_continents_as_rumor`
- `an_npc_knows_every_text_that_it_said_itself`
- `a_quest_giver_knows_its_quest_text_that_names_a_far_place`
- `text_read_in_the_zone_of_the_npc_is_known_well`
- `text_with_no_zone_is_known_only_by_a_host_or_a_scholar`

The faction:

- `an_npc_does_not_know_the_inner_affairs_of_the_other_faction`
- `common_affairs_of_the_other_faction_are_rumor`
- `the_land_of_the_other_faction_is_at_most_rumor`
- `a_neutral_npc_has_no_faction_cap`
- `an_npc_with_no_faction_takes_the_faction_of_its_zone`
- `an_npc_in_a_contested_zone_with_no_faction_is_neutral`

The reach:

- `the_nearest_place_of_a_passage_decides`
- `a_passage_with_no_place_is_world`
- `a_zone_with_no_continent_is_world_unless_it_is_home`
- `an_instance_is_home_to_the_npcs_of_its_door_zone`

### 13.2 `crates/story/tests/npc_kind.rs` (new)

- `the_title_line_names_the_kind`
- `the_name_names_the_kind_when_the_title_has_no_known_word`
- `a_title_with_a_known_word_wins_over_the_name`
- `a_title_with_two_kinds_takes_the_kind_that_knows_most`
- `an_npc_with_no_title_and_no_known_word_is_a_local`
- `a_word_matches_only_as_a_whole_word` ("Guardian" is no Guard)
- `a_phrase_matches_its_words_in_order` ("Flight Master")
- `a_word_matches_in_any_case`
- `the_bundled_kind_list_reads`
- `no_word_names_two_kinds`

### 13.3 `crates/story/tests/zones.rs` (new)

- `the_bundled_zone_list_reads`
- `every_capital_has_a_continent_and_a_faction`
- `an_instance_names_a_zone_of_the_list_as_its_door`
- `contested_reads_as_no_faction`
- `an_unknown_faction_in_the_zone_list_is_refused`

### 13.4 Existing test files

`tests/pack.rs`:

- `a_written_passage_comes_back_with_its_scope`
- `a_pack_of_format_one_reads_with_no_scope`
- `a_pack_of_an_unknown_format_is_refused`
- `a_passage_with_only_a_scope_is_refused`

`tests/pack_sources.rs`:

- `every_book_passage_is_history`
- `a_page_with_a_faction_gives_its_passages_that_faction`
- `a_page_with_an_unknown_faction_is_refused`

`tests/talk.rs`:

- `a_rumor_goes_in_its_own_block_as_hearsay`
- `a_prompt_with_no_rumor_has_no_rumor_block`
- `a_prompt_tells_the_npc_to_say_when_it_does_not_know`
- `the_title_of_the_npc_is_in_its_fence`
- `the_title_cannot_close_its_fence`
- `each_kind_gets_its_own_sentence`

`tests/input.rs`:

- `an_npc_seen_line_reads_with_a_title_and_a_faction`
- `an_npc_seen_line_without_a_title_still_reads`
- `an_unknown_faction_reads_as_none`

`tests/character.rs`:

- `a_sighting_gives_the_npc_its_title_and_its_faction`
- `a_newer_sighting_with_another_title_replaces_the_title`
- `a_sighting_with_no_title_keeps_the_old_title`

`tests/story.rs`:

- `a_farmer_of_goldshire_does_not_explain_another_continent`
- `an_innkeeper_hears_of_far_lands_as_rumor`
- `a_far_passage_does_not_take_the_room_of_a_near_one`
- `at_most_one_rumor_reaches_a_talk`
- `lore_still_answers_from_the_whole_pack`
- `a_title_with_a_control_character_is_refused`
- `a_title_past_the_name_limit_is_refused`
- `a_talk_reads_the_events_behind_the_places_of_its_passages`
- `a_talk_does_not_read_a_passage_that_the_npc_does_not_know`

`tests/samples.rs`: the existing check that no sample names a real place covers the new sample.

`tests/voice.rs`: the talk moments get a title and a rumor, so their prompts are measured with the new parts. One live moment, `#[ignore]`: a Goldshire farmer is asked about Ironforge, with no Ironforge passage, for review by a person.

### 13.5 Addon tests: `crates/addon-tests/tests/sightings.rs`

- `a_sighting_sends_the_title_line_of_the_tooltip`
- `a_level_line_is_no_title`
- `the_angle_brackets_of_a_title_go`
- `a_hidden_title_is_never_read`
- `a_sighting_sends_the_faction_of_the_npc`
- `a_neutral_npc_sends_no_faction`
- `a_title_past_the_limit_is_not_sent`

The fake client in `addon/tests/wow.lua` gets `C_TooltipInfo.GetUnit`, `UnitFactionGroup`, and `TOOLTIP_UNIT_LEVEL`. The API gate (`addon/tests/api.lua`, written by the relay script `wow-api.sh`) gets the three names, so a check before a release confirms them on the Forever client.

### 13.6 Property tests: `crates/story/tests/properties.rs`

- **`a_passage_that_the_npc_does_not_know_never_reaches_a_talk_prompt`.** The play strategy gets sightings with a title from a short list (each kind, an unknown word, none) and a faction (alliance, horde, none). The pack holds passages with marker words ("Zqp0" to "Zqp9"), each with a random place of a short list, random `common`, `history`, and `faction`. After the plays, a talk asks with all the marker words. For each marker in the prompt of the talk call, the test computes `knowledge` from the world, and asserts that it is not `DoesNotKnow`. A marker in the rumor block must be `Rumor`.
  - **The edges come often.** The place list holds the zone of the NPC, a subzone of it, a zone of the same continent, a zone of the other continent, a zone with no continent, and an instance. The NPC stands in a capital in one draw of four.
- **`a_scholar_knows_at_least_what_a_local_knows`.** Pure: for any `About` and faction, `knowledge` of a Scholar is at least `knowledge` of a Local.
- **`the_inner_affairs_of_the_other_faction_are_never_known_well`.** Pure: for any `About` with `affairs_of` set to the other faction, the answer is at most `Rumor`, and `DoesNotKnow` when it is not common, unless the NPC said it itself.
- **`a_talk_holds_at_most_three_passages_and_one_rumor`.** Through `talk_passages`.

### 13.7 Fuzz

- `fuzz/fuzz_targets/input.rs` reads any line, so it covers the new fields already. New seeds in `fuzz/seeds/input/`: an `npc_seen` with a title of 96 bytes, of 97 bytes, with a `|`, with angle brackets only, and with `faction` set to `"neutral"`, `7`, and `null`.
- `fuzz/fuzz_targets/play.rs`: sightings with titles from the kind list and from random bytes.
- The title is text from outside, and `kind_of` reads it. The new fuzz target `fuzz/fuzz_targets/npc_kind.rs` sends any bytes as a title and a name to `kind_of`: it never panics, and the same input gives the same kind.
- `zones.toml` and `npc_kinds.toml` are bundled data, not text from outside. Their tests read them.

## 14. Build order

Each step is one commit with its tests, and each one ships.

1. `zones.rs` and `zones.toml`, with `tests/zones.rs`. Nothing calls them yet.
2. `npc_kind.rs` and `npc_kinds.toml`, with `tests/npc_kind.rs` and the fuzz target. Nothing calls them yet.
3. `npc_knowledge.rs`: `Knower`, `About`, `Reach`, `reach`, and `knowledge`, with `tests/npc_knowledge.rs` and the pure property tests.
4. `pack.rs`: `Scope`, format 2, and the reader of 1 and 2. `pack_sources.rs` and `.toml`: the history mark of the books, and `faction` and `history` on pages.
5. `input.rs`, `vocabulary.rs`, and `character.rs`: the two fields of `npc_seen`, `npc_title`, `member_of` on NPCs, `title_of`, and `faction_of`. The fuzz seeds. Tell the Gnomish Relay session about the two new fields of `npc_seen`.
6. The addon: the title and the faction in `Sightings.lua`, the fake client, the API gate, and the addon tests.
7. `story.rs`: `candidates`, `fitted`, `talk_passages`, the glue of `story/knowledge.rs`, and `places_read`. The talk prompt gets the two blocks, the title, the kind sentence, and the rule. The golden sample. The story tests and the play property test.
8. The `GAMEPLAY.md` text of section 15. This plan marks the steps as done.

Steps 1 to 4 change no behaviour. Step 7 is the one that changes what an NPC says.

## 15. GAMEPLAY.md text

### 15.1 In 3.5, replace the line "What the NPC knows"

- **What the NPC knows:** its place, its title, your level, your slaps (5.4.1), its trust in you, and up to 3 lore passages, under the spoiler limit (3.1) and under what the NPC knows (3.5.1).

### 15.2 New section 3.5.1

#### 3.5.1 An NPC knows only its own world

An NPC knows the lore of its place and its work. It does not know the whole pack. `/lore` does not change: the lore book is a local historian.

- **Each passage gets one answer for the NPC:** it knows it, it heard it as a rumor, or it does not know it. A passage that the NPC does not know never goes into the prompt. A rumor goes into the prompt as hearsay.
- **Place.** An NPC knows its own zone well, and every subzone of it. It knows its continent only as rumor. It does not know other continents.
- **Kind.** The title line of the NPC gives its kind. With no known word in the title, the name gives it: "Marshal Dughan" is a guard. A list in the repo holds the words (`crates/story/data/npc_kinds.toml`).
  - A local knows its land. Every NPC with no known word is a local.
  - A guard knows its land and its dangers.
  - An innkeeper or a flight master hears the news of travellers. It knows other continents as rumor.
  - A scholar or a mage knows history well, and its continent well. It knows other continents as rumor.
  - Every other kind knows history only as rumor.
- **A capital hears more.** A local or a guard of a capital knows other continents as rumor.
- **Faction.** An NPC never knows the inner affairs of the other faction. When those affairs are common knowledge, it knows them as rumor. It knows the land of the other faction at most as rumor. A neutral NPC has no such limit.
- **Its own words.** An NPC always knows the text that it said to you: its gossip and its quests. This holds also when its quest names a far place.
- **What you read.** A text that you read in the zone of the NPC is home to it. A text from another zone follows the rules of place.
- **When it does not know.** The NPC says so, in its own voice: "Ironforge? Never been. Ask a dwarf." It never makes up an answer.
- **Where the data comes from.** The addon sends the title line and the faction of each NPC that you see. The continent and the faction of each zone come from a list in the repo (`crates/story/data/zones.toml`).

### 15.3 In 5.10, "The pack", add

- A passage has a scope: the inner affairs of a faction, and a mark for history. The scope never gates the spoiler limit. It tells an NPC what it knows (3.5.1).
- The pack has format 2. The story program also reads format 1, which has no scope.

In "The list is data", add:

- A page can name the faction whose inner affairs it tells, and can mark itself as history. Each book passage is history.

### 15.4 In 5.14, the row "Talk" of the reads table

| Talk | the events behind the NPC, the hero entries about it, the `learned` rows of its passages, and the events behind the places of its passages |

## 16. Open questions

1. **The game check.** Does line 2 of the tooltip data of a Forever NPC hold its title, with or without angle brackets? Does `UnitFactionGroup` give "Alliance" for a Stormwind guard and nothing for a Booty Bay goblin? A test in the game decides before step 6.
2. **A check of the answer.** The model knows Ironforge from its training. The prompt rule and the sample guard against that, but nothing checks the answer. One option: drop an answer that names a zone of `zones.toml` that the NPC does not know, when the player's words and the prompt do not name it. That costs some good answers. Wanted in a later step?
3. **Other client languages.** The kind words are English. A French client gives "Aubergiste", so every NPC there is a local. Add the words of each language to `npc_kinds.toml`, or wait until a player of another language asks?
4. **Guards and danger.** A guard and a local share the place table. A passage topic of "danger" (bandits, gnolls, the Defias) gives a guard a real difference. It needs a mark in `pack_sources.toml`. Wanted?
5. **Two word lists.** `npc_kinds.toml` (guard) and `lawful_titles.txt` of standing.md overlap. Merge them into one list of guard words, or keep two lists for two questions?
6. **Quest givers.** A `/quest` prompt has no lore passages today. When it gets some, the same rule applies to the giver. Agreed?
7. **Leaders.** Kings, lords, and the leaders of a faction know the affairs of their faction across continents. A fifth kind, "leader", fits them. The first list leaves it out, because few leaders talk to a player at low level. Add it now?
