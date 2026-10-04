# Plan: quest variety

Status: in build, 2026-10-03. The user approved the design. When a part is built, its rules move into `GAMEPLAY.md` (section 19 of this plan), and this plan marks the part as done.

Built so far (section 17):

- Step 1, per-step state. Changed from this plan: an offer shows its steps plainly, not faded. Only a later step of a quest in progress shows faded, because a faded offer reads as a quest that you cannot take.
- Step 2, one place moves the quests. `Here` and `Encounter` grow with the steps that need them.
- Step 3, talk. The talk window of talk-window.md is not built, so "the later turns of the conversation" are the talks to the same NPC in the 10 minutes after the talk step. The talk calls `advance_quests` itself before its prompt, so the first talk already has the quest line.
- Step 4, come back later. `MAX_STEPS` became 4 in this step. A choice quest is not built, so its rule (never the giver in a step) waits for standing.md.
- Step 5, any order. `QuestSteps.lua` reads the open kill steps and asks for the journal while a watched step waits. `Foes.Hunt` takes the set of creatures from it.
- Step 6, carry items. `items_held` waits for the next flush, as `npc_met` does. A done carry step shows its whole count on the page.
- Step 7, genre and variety. `variety` is `quest::variety`. `Recent` also keeps the number of its quest, so the offer can read its rows. `ShapePart::Choice` exists as the hook for standing.md, and no shape holds it yet. The property `an_offered_task_only_names_allowed_targets` read the offer from the narrator line, where it no longer comes, so it checked nothing. It now reads the notice, and its model knows that a gossip window ends `hostile`.

This plan builds on four plans and does not repeat them:

- `docs/plans/links.md` owns rows, calls, reads, and proof (5.14).
- `docs/plans/npc-memory.md` owns the memory block of a talk, and the hero hook of a quest offer.
- `docs/plans/talk-window.md` owns the conversation of `/talk`.
- `docs/plans/standing.md` owns choice quests (two endings), `Encounter`, `town`, the reward check, and standing. This plan only names them.

## 1. Goal

Side quests (3.4) feel the same today: 1 to 3 steps of `visit`, `meet`, and `kill`, often in the same order, and often with the same kind of title. This plan does two things:

1. **No repeated quests.** The prompt shows your last 3 quests. The check refuses an offer with the shape of one of the last 2, or with a main word of one of the last 3 titles. A refused offer gets one retry with the reasons.
2. **New steps and new structure.** Nine new step kinds, each one checked by an event that the game shows the addon. Steps in any order, and hidden steps for a mystery.

Every rule of `GAMEPLAY.md` 2 still holds:

- **The AI never plays.** A step names a goal. The player does it, and a game event proves it.
- **No advantage.** No step gives an item, gold, or power. A carry step takes nothing from your bags either.
- **The game is the truth.** Each step completes only from an event of the addon, or from a fact of the world that an event made.

## 2. What changes, in short

| Part | Change |
|---|---|
| `quest.rs` | 9 new `Step` variants, `Genre`, `Shape`, `AnyOrder`, `Recent`. `MAX_STEPS` goes from 3 to 4. `Tracked` keeps the state of each step. `Encounter` and `Here` decide when a step holds. |
| `variety.rs` (new) | Shapes, main words, and the variety faults. Pure functions. |
| `story/quests.rs` | One retry of a refused offer. Progress runs after every line with a time. |
| `story.rs` | Each line gives its `Encounter`. One call of `advance_quests` after the line changed the world. |
| `input.rs` | New `items_held` and `hour_changed`. New optional `hour` on `zone_entered`. |
| `character.rs` | `dungeons_entered`, `bosses_defeated`, `game_quests_open`, `game_quests_done`. |
| `journal.rs` | A quest goes out as `QuestView`: each step with its state. Hidden steps stay out. |
| `talk.rs` | A talk to the NPC of a talk step gets one line about the quest. |
| Data | `stop_words.txt`, `quest_emotes.txt`, `carry_items.txt` in `crates/story/data/`. |
| Addon | Step lines for each kind, `QuestSteps.lua` (new), `Carry.lua` (new), the local hour, `C_Item.GetItemCount` through the API gate. |
| Relay | Nothing. Section 16 says why. |

## 3. No repeated quests

### 3.1 The genre

The model gives each quest a genre. The final set has six genres:

| Genre | A quest about | Why it is in the set |
|---|---|---|
| `errand` | a favor or a delivery | The quest of today. It stays, so the model always has a safe choice. |
| `hunt` | a creature or a boss | Most kill quests are hunts. Without this genre, the model calls them errands, and the variety rule cannot tell a hunt from a delivery. |
| `mystery` | a question with clues | It hides its later steps (5.3). The genre is the switch, so the model never sets a flag by hand. |
| `rescue` | a person who is lost or in trouble | A strong story with the steps of today: go there, find them, talk. |
| `rivalry` | a feud between two people | It holds every choice quest of standing.md (5.2). A rivalry with no choice is a contest or a quarrel. |
| `comic` | a joke | The only genre with a slap step (4.10). |

The user approved "errand, mystery, rescue, rivalry, or comic". This plan adds `hunt` for the reason in the table.

**The word.** The user asked for a "tone". standing.md already uses `Tone` for kind, cunning, and cruel (standing.md 4.2). One word means one thing, so this plan calls it a **genre**: `Genre` in the code, `"genre"` in the JSON. The player never sees it.

```rust
/// The kind of story of a side quest. The player never sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Genre { Errand, Hunt, Mystery, Rescue, Rivalry, Comic }
```

The answer carries the genre as a string. The check parses it, so an unknown word gives the fault `UnknownGenre(String)`, and the retry (3.6) can name it. A missing genre gives `NoGenre`.

### 3.2 The shape

The shape of a quest is the kinds of its steps, in order.

- Each step gives its goal word: `visit`, `visit_at`, `meet`, `talk`, `emote`, `slap`, `kill`, `defeat`, `enter`, `level`, `game_quest`, `carry`, `wait`.
- An any-order set (5.1) is one part of the shape. Its kinds are sorted, so "meet, then kill" and "kill, then meet" in a set give the same part.
- A choice quest (standing.md) adds `choice` as its last part.

```rust
/// The kinds of the steps of a quest, in order. Two quests with the same shape feel the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape(Vec<ShapePart>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapePart {
    One(&'static str),
    /// The kinds of an any-order set, sorted.
    AnyOrder(Vec<&'static str>),
    Choice,
}
```

`Shape` shows in the prompt as plain words: `visit, wait, talk` or `any order (kill, meet), carry, choice`.

The shape holds no target and no count. "Kill 3 Duskbats" and "Kill 6 Kobolds" are the same shape. That is the point: the player does the same thing.

### 3.3 Main words

A main word of a title is a word of the title that is not a stop word.

- A word is a run of letters and digits, as `check::words_of` splits it: "Farley's Lost Lantern" gives `farley`, `s`, `lost`, `lantern`.
- A word is folded to lower case with `to_lowercase`. So "LOST" and "lost" are one word.
- A stop word is a word of `crates/story/data/stop_words.txt`, or a word of one letter.

`crates/story/data/stop_words.txt`:

```
# Words that never make two quest titles the same (docs/plans/quest-variety.md 3.3). Whole
# words, lower case. A word of one letter is a stop word too, and needs no line here.
the
an
of
in
on
at
to
for
and
or
but
with
from
by
into
over
under
my
your
his
her
its
our
their
me
you
him
them
us
is
are
was
be
no
not
this
that
quest
task
```

`quest` and `task` are stop words, because nearly every title can hold them, and they say nothing about the story.

The name of the giver is no stop word. Two quests in a row named for one giver ("Farley's Lantern", "Farley's Goose") feel the same.

```rust
/// The words of a title that make it what it is (docs/plans/quest-variety.md 3.3).
pub fn main_words(title: &str) -> Vec<String>
```

### 3.4 The recent quests

```rust
/// A quest of the log, as the variety rules see it.
pub struct Recent {
    pub title: String,
    pub shape: Shape,
    /// None for a quest from before genres.
    pub genre: Option<Genre>,
}

/// The newest offers of the log, newest first, in any state.
pub fn recent_quests(quests: &[Tracked], count: usize) -> Vec<Recent>
```

- The constants: `RECENT_IN_PROMPT = 3`, `SHAPES_TO_AVOID = 2`, `TITLES_TO_AVOID = 3`.
- "Recent" means the newest offers by number, from any giver, in any state. A declined offer and a replaced offer count too. This is the rule of `last_targets` today (3.4, "No target twice in a row"): the player saw the offer, so the next one is different.

### 3.5 The check

`checked_quest` gets two new faults after its other rules:

| Fault | Rule |
|---|---|
| `SameShape(String)` | The shape equals the shape of one of the newest 2 quests. The string is the shape in words. |
| `SameTitleWord(String)` | A main word of the title is a main word of the title of one of the newest 3 quests. The string is the word. |

`Known` gets `recent: Vec<Recent>`. The story program fills it with `recent_quests(&quests, RECENT_IN_PROMPT)`.

The functions of `variety.rs`:

```rust
/// The first variety rule that the quest breaks.
pub fn variety_fault(quest: &Quest, recent: &[Recent]) -> Option<QuestFault>
```

### 3.6 One retry

A refused offer gets one more call, as `/lore` does (`lore.rs`).

1. The first answer breaks a rule of `checked_quest`. The story program opens a second call. Its prompt is the first prompt, the first answer in a fence, and the reason: the `Display` text of the fault. `prompt::retry` already builds this text for `/lore`. It takes the reasons as `&[String]`, so both callers share it.
2. The second answer passes: the offer shows as today.
3. The second answer breaks a rule too, or the second call fails: "Farley has no quest for you now.", as today.

Rules:

- `Pending::Quest` gets `attempt: Attempt` (`First` or `Retry`). The enum moves from `lore.rs` to `calls.rs`, so both kinds use it.
- Only a fault of the answer gets a retry. A refusal of the limits (`fn refusal`: the giver waits for you, or you hold 3 quests) asks no model at all. A failed first call (no model, a timeout, a spent budget) gets no retry either: the bridge decided.
- The retry keeps the batch of the first call. The deadline of 3.4 holds: an offer that comes 55 seconds or more after the `batch_end` waits for the next answer.
- The retry call has its own row in `calls`, with the kind `quest_retry`. It names the first call as the call that opened it (5.14). It reads what the first call read.
- The hook of npc-memory.md (10.2) counts rows of the kinds `talk` and `quest`. A `quest_retry` row does not count, so a retry never moves the hook. The retry keeps the hook of its first call, because its prompt is the first prompt.
- The first call ends `refused`, the second `accepted` or `refused`.

### 3.7 Why this giver needs it

The text of each quest says why this giver needs it: who they are and where they live.

- **The prompt does the work.** The persona already says who the giver is and where they stand (`talk::persona`). A new rule: "Start the text with why you need this: who you are, and what it means for you in {place}."
- **A light check.** The text speaks for the giver: it holds a word of the first person (`I`, `me`, `my`, `we`, `us`, `our`), as whole words in any case. Else the fault `NotFromGiver`. The retry fixes most misses.

The check cannot judge a reason. It only refuses a text that never speaks for the giver, such as "Visit the mill and kill 6 bats."

### 3.8 The prompt

The prompt gets a block after the lists and before "Rules:". It is internal text, not UI copy.

```
The player's last quests, newest first:
<<<
- "The Lost Lantern": visit, meet, kill. An errand.
- "Rats in the Cellar": kill. A hunt.
- "Old Debts": any order (meet, visit), choice. A rivalry.
>>>
Make this quest different from these: other kinds of steps or another order, another genre, and no word of their titles.
```

- A quest from before genres shows no genre sentence.
- The titles are model text that passed the check. They still go in a fence, as every model text in a prompt does.
- With no earlier quest, the block is left out.

New lines under "Rules:":

```
- Give the quest a genre: "errand" (a favor or a delivery), "hunt" (a creature or a boss), "mystery" (a question with clues), "rescue" (a person who is lost or in trouble), "rivalry" (a feud between two people), or "comic" (a joke).
- Start the text with why you need this: who you are, and what it means for you in {place}.
- In a mystery, the text names only the first step. The player finds the rest.
```

The JSON line:

```
Reply with JSON only: {"title": "...", "genre": "...", "text": "...", "steps": [...]}
```

The goals of the prompt follow the rule of today: **the prompt lists only what the check allows.** A goal shows only when its list has a name, so a player who never entered a dungeon gets no `enter` line. `wait` and `any_order` always show. `level` shows when the player is below level 60.

**Size.** The quest prompt is 406 tokens today (npc-memory.md 10.8). standing.md adds about 230, and the hook about 125. This plan adds about 120 for the recent block and the rules, and about 180 for the new goals and lists at their fullest. The total stays near 1060, under the budget of 1733 in `tests/voice.rs`. `voice.rs` gets `quest_offer_with_every_goal()` at full length, and the budget test covers it.

## 4. The new step kinds

### 4.1 The steps in the JSON

`Step` is the enum of today with nine new variants. The tag stays `goal`.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "goal", rename_all = "snake_case")]
pub enum Step {
    Visit { place: String },
    Meet { npc: String },
    Kill { creature: String, #[serde(deserialize_with = "whole_number")] count: u8 },
    /// `/talk` to the NPC (4.2). `about` is a topic in a few words.
    Talk { npc: String, #[serde(default)] about: Option<String> },
    /// Come back later: the steps after it open only after the wait (4.4).
    Wait { #[serde(deserialize_with = "whole_number")] days: u8 },
    /// Have the items in your bags when you meet the NPC (4.9).
    Carry { item: String, #[serde(deserialize_with = "whole_number")] count: u8, npc: String },
    /// An emote at an NPC, or in a place. Exactly one of the two (4.3).
    Emote { emote: String, #[serde(default)] npc: Option<String>, #[serde(default)] place: Option<String> },
    /// Be in the place at a time of day, in the local time of the player (4.5).
    VisitAt { place: String, time: TimeOfDay },
    Level { #[serde(deserialize_with = "whole_number")] level: u8 },
    /// Enter a dungeon or a raid (4.7).
    Enter { dungeon: String },
    /// Defeat a boss of a dungeon or a raid (4.7).
    Defeat { boss: String },
    /// Turn in a quest of the game, by its exact title (4.8).
    GameQuest { title: String },
    /// `/slap` the NPC. Only in a comic quest (4.10).
    Slap { npc: String },
}
```

An any-order set is not a `Step`. It comes in the JSON as `{"goal": "any_order", "steps": [...]}`, and the parse flattens it (5.1).

`Step::target` now returns `Option<&str>`: `Wait` and `Level` name no target. Every rule that uses a target (each step names a different target, no target of the newest quest, the reads of 5.14) skips a step with none.

A full answer:

```json
{
  "title": "A Cask Gone Missing",
  "genre": "mystery",
  "text": "I brew for every traveler on this road, and my best cask vanished last night. Ask Farmer Bram what he saw. I'll need time to think after that.",
  "steps": [
    {"goal": "talk", "npc": "Farmer Bram", "about": "the missing cask"},
    {"goal": "any_order", "steps": [
      {"goal": "visit_at", "place": "Old Mill", "time": "night"},
      {"goal": "emote", "emote": "bow", "npc": "Sister Ada"}
    ]},
    {"goal": "wait", "days": 1}
  ]
}
```

This answer breaks a rule: a wait is never the last step (4.4). The check refuses it, and the retry names the reason.

### 4.2 Talk

| | |
|---|---|
| JSON | `{"goal": "talk", "npc": "Farmer Bram", "about": "the missing cask"}` |
| Limits | `npc` is a name of the people list. `about` is optional: plain text (`check::plain_text`), at most 60 characters and 240 bytes, no `\|`, no name after the cutoff. |
| Targets | The people that a meet step can name (`Known::people`): met or seen friendly, not hostile, not an animal, not dead in your story, not the giver, not in a game quest that you read. |
| Event | `talk_asked` for that NPC: any turn of a conversation of talk-window.md. |
| Not by | A gossip window of the game (`npc_met`), a slap, or `/quest`. Those still do a `meet` step. |

**The tie-in with the talk window.** A talk to the NPC of a talk step gets one more line in its prompt, after the memory block of npc-memory.md:

```
The player comes to you for a quest of {giver}: "{title}". {about, when there is one: "They want to ask about the missing cask."}
Play along with it. Say what you know, and make nothing up about {giver}.
```

- The line shows for each quest whose talk step names this NPC, and was done in the last 10 minutes (`conversation::IDLE_SECONDS`). The step is done by the first turn, before the model answers, because `advance_quests` runs first (npc-memory.md 3.2). So the later turns of the same conversation keep the line.
- The title and the topic go in the data fence, as model text that passed a check.
- The talk call reads the `quests` rows of each such quest (9.2).

### 4.3 Emote

| | |
|---|---|
| JSON | `{"goal": "emote", "emote": "bow", "npc": "Sister Ada"}` or `{"goal": "emote", "emote": "dance", "place": "Goldshire"}` |
| Limits | `emote` is a token of `quest_emotes.txt`, in lower case. Exactly one of `npc` and `place`. Else the fault `EmoteTarget`. |
| Targets | `npc`: the people list. `place`: the places list. |
| Event | `emote_done` with that token. With `npc`: its `target` is that NPC. With `place`: you stand in the place (the subzone or the zone of `Character::place_names`). |

`crates/story/data/quest_emotes.txt`:

```
# Emotes that a quest step can ask for (docs/plans/quest-variety.md 4.3). The tokens of
# the game, in lower case, as Emotes.lua sends them. No rude emote: a quest never asks you
# to insult an NPC. A slap has its own step (4.10).
applaud
bow
cheer
dance
flex
greet
hug
kneel
pray
salute
sing
thank
wave
```

Most of these are on the kind list of standing.md (6.2), so an emote step at an NPC of a town raises Manners there, as any kind emote does. The quest changes nothing in that rule.

### 4.4 Come back later (wait)

| | |
|---|---|
| JSON | `{"goal": "wait", "days": 2}` |
| Limits | `days` from 1 to 3. At most one wait in a quest. A wait is never the first step, never the last step, and never in an any-order set. Else `WaitDays(u8)` or `WaitPlace`. |
| Clock | The `at` of the lines from the addon: `time()` in seconds, the clock that the world uses (`Tick`). A day is 86 400 seconds of that clock, not a calendar day. |
| Opens | When the step before it is done. The log holds that time (6.2). |
| Done | By the first line with a time at or after `opened_at + days * 86 400`. Any line counts: a zone, a meeting, the login. |

**Why 1 to 3 days.** Less than a day is the same evening, so it is no "later". More than 3 days holds a slot of your 3 open quests too long, and the giver waits for you all that time (3.4, limits).

**Why not first or last.** A wait first only delays the quest: nothing comes before it to wait on. A wait last ends the quest with nothing to do.

**The giver after a wait.** A `talk`, `meet`, or `carry` step can name the giver, but only when a wait comes before it in the quest. "Come back in two days and tell me what you found." The rule of today (no step sends you back to the giver) exists because the giver stands next to you at the accept. After a wait of a day, that is no problem. A choice quest (standing.md) never names the giver in a step, because the gossip window of the giver ends its choice (12).

The fault `MeetGiver` keeps its name and gets a new rule: a step names the giver with no wait before it.

### 4.5 Time of day

| | |
|---|---|
| JSON | `{"goal": "visit_at", "place": "Old Mill", "time": "night"}` |
| Limits | `place` is a name of the places list. `time` is one of the four times below. An unknown time fails the parse: `NotJson`. |
| Event | A line with an `hour`, while you stand in the place: `zone_entered`, `emote_done`, `died`, or `hour_changed`. |

```rust
/// A time of day, in the local hours of the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeOfDay { Dawn, Noon, Dusk, Night }

impl TimeOfDay {
    /// Does the hour, from 0 to 23, fall in this time?
    pub fn holds_at(self, hour: u8) -> bool
}
```

| Time | Hours | UI copy |
|---|---|---|
| `dawn` | 5 to 7 (5:00 to 7:59) | at dawn (5 AM to 8 AM) |
| `noon` | 11 to 13 | at noon (11 AM to 2 PM) |
| `dusk` | 18 to 20 | at dusk (6 PM to 9 PM) |
| `night` | 21 to 23, and 0 to 4 | at night (9 PM to 5 AM) |

**Why named times, and not a range from the model.** A small model writes "dusk" well, and a range of hours badly. The code owns the hours, so a test states each edge. Each time is at least 3 hours long, so a player can plan for it.

**Where the hour comes from: the addon.** The addon sends the local hour of the computer (`date("%H")`) on `emote_done` and `died` today (5.4.1). This plan adds it to `zone_entered`, and adds a new line `hour_changed`:

```json
{"type": "hour_changed", "at": 1790000000, "hour": 21}
```

- The addon sends `hour_changed` when the local hour changes, only while the journal shows an open `visit_at` step (10.3). So a player who stands in the Old Mill as night falls gets the step, with no walk out and back.
- The story program keeps no hour. Each line carries its own hour, or none. A line with no hour never does a `visit_at` step.
- Why the addon and not the desktop: the desktop has no time zone code today, and one hour source keeps the odd hour of 5.4.1 and this step the same. The addon and the desktop share one computer (5.14, `checked_time`), so they share its time zone too.

### 4.6 Level

| | |
|---|---|
| JSON | `{"goal": "level", "level": 14}` |
| Limits | 1 to 3 levels above your level now, and at most 60 (`vocabulary::LEVELS`). Else `LevelOutOfReach(u8)`. A player with no level fact gets no level step. |
| Event | `level_reached`. The step holds while your level is at least its level. |

The prompt gets one line, only when the level goal shows: "The player is level 12."

### 4.7 Dungeon

Two kinds:

| | Enter | Defeat |
|---|---|---|
| JSON | `{"goal": "enter", "dungeon": "The Deadmines"}` | `{"goal": "defeat", "boss": "Edwin VanCleef"}` |
| Targets | A zone that holds the `dungeon` or `raid` mark of your world (`Character::dungeons_entered`). So you entered it before. | A foe that you defeated (`defeated`), whose place is a zone with that mark (`Character::bosses_defeated`). So you killed it before, in a dungeon or a raid. |
| Event | You stand in the zone: the `zone_entered` of the dungeon. `instance_entered` follows it and changes nothing for the step. | `npc_defeated` with that name: `PARTY_KILL` of a notable unit, or `ENCOUNTER_END` with `success` 1. |

**Why only bosses that you defeated.** `npc_defeated` comes only for a rare, a rare elite, or a boss (5.4). The addon sends no classification with `npc_seen`, so the world cannot tell a boss that you only saw from a common mob. A boss that you defeated once has the name that the addon sends, so the step can match it byte for byte. A second kill is an echo (5.13), which fits a story.

**Why "enter" checks the zone, not `instance_entered`.** The addon sends `zone_entered` with the dungeon name first, and the world moves you there. So "you stand in the dungeon" is a fact of the world, and the step holds at once when it opens while you stand inside, as a visit does.

The overlap rule of 3.4 holds for both: a dungeon or a boss in the text of a game quest that you read is no target.

### 4.8 Game quest

| | |
|---|---|
| JSON | `{"goal": "game_quest", "title": "The Defias Brotherhood"}` |
| Limits | The exact title, byte for byte. |
| Targets | A quest of the game that you hold (`game_quest_taken` with no `game_quest_done`) or that you read (a `text_seen` of kind `quest`), and that you did not turn in. Else `UnknownGameQuest(String)` or `GameQuestDone(String)`. |
| Event | `game_quest_done` with that title. The step holds while the world holds `game_quest_done` for it. |

**A change of a rule of 3.4.** "No overlap with the quests of the game" says that a side quest never sends you to the goal of a game quest. This step does: it asks you to finish one. The user approved the step, so the rule gets one exception, in the words of 19:

- The step names the quest of the game as a whole, by its title. It never names one of its goals.
- The side quest never changes it, continues it, or claims its deed. The deed and its reward stay the game's.
- The other steps of the quest keep the overlap rule. The title of the side quest still never has the words of the title of a game quest that you read.

The overlap check (`in_game_quests`) skips the target of a `game_quest` step, because the target is a game quest by design.

### 4.9 Carry items

| | |
|---|---|
| JSON | `{"goal": "carry", "item": "Linen Cloth", "count": 10, "npc": "Farmer Bram"}` |
| Limits | `item` is a good of `carry_items.txt` whose level band holds your level. `count` from 1 to 20 (`MAX_CARRY`). `npc` is a name of the people list (or the giver after a wait, 4.4). Else `UnknownGood(String)` or `CarryCount(u8)`. |
| Event | `items_held` for that NPC and that item, with a count at least the step count. |
| What you lose | Nothing. You keep the items. No trade opens. |

**The goods.** The world knows no items: the addon never watches your bags (5.4). So the goods come from a list of common trade goods, food, and drink, each with the levels where a player meets it. A vendor sells it, or many mobs drop it, so the player can always get it.

`crates/story/data/carry_items.txt`, a first list:

```
# Goods that a carry step can ask for (docs/plans/quest-variety.md 4.9): the exact English
# name of the game, the lowest level, and the highest level. A test in the game confirms
# each name with C_Item.GetItemCount.
Linen Cloth | 1 | 20
Wool Cloth | 15 | 30
Silk Cloth | 25 | 40
Mageweave Cloth | 35 | 50
Runecloth | 45 | 60
Light Leather | 5 | 20
Medium Leather | 15 | 30
Heavy Leather | 25 | 40
Rough Stone | 1 | 15
Coarse Stone | 10 | 25
Copper Ore | 1 | 15
Tin Ore | 10 | 25
Peacebloom | 1 | 15
Silverleaf | 1 | 15
Briarthorn | 10 | 25
Refreshing Spring Water | 1 | 10
Ice Cold Milk | 5 | 20
Melon Juice | 15 | 30
Tough Jerky | 1 | 10
Haunch of Meat | 5 | 20
```

**Why goods are exempt from the overlap rule.** "Bring 10 Linen Cloth" is the goal of many game quests. The rule protects the goal, and the goal of a game quest is linen for its own NPC. The NPC of the carry step still passes the overlap check, so a carry step never sends linen to the NPC of a game quest that you read. A zone is exempt for the same kind of reason today.

**The reward check of standing.md** (4.6) takes out the names of the prompt first. The goods are names of the prompt, so "Linen Cloth" never trips it.

**The new event.** The addon counts one item in your bags when you meet the NPC of an open carry step (10.4):

```json
{"type": "items_held", "at": 1790000000, "npc": "Farmer Bram", "item": "Linen Cloth", "count": 6}
```

- `count` is `C_Item.GetItemCount(item)`: the bags only, not the bank. The story program takes 0 to 65535 (`u16`), and refuses a line outside that.
- The line goes only for an open carry step, only at a meeting with its NPC (a gossip window or `/talk`), and only for its item. It tells nothing else about your bags.

### 4.10 Slap

| | |
|---|---|
| JSON | `{"goal": "slap", "npc": "Farmer Bram"}` |
| Limits | Only in a quest of the genre `comic`. Else `SlapOutsideComic`. |
| Targets | The people list, except the giver, and except an NPC whose name holds a word of `cruel_words.txt` (standing.md 4.6): no quest asks you to slap a child or an orphan. Else `SlapGiver` or `CruelTarget(String)`. |
| Event | `npc_slapped` with that name. |

**A slap step is a slap.** The quest changes none of its costs: the NPC loses 10 trust, the `slapped` fact counts up, Slap Happy comes closer (5.4.1), and the town counts it as rude conduct in the Slaps part of standing (standing.md 6.1). The quest page says so before you accept (11).

A slap still does a `meet` step too, as today (`a_slap_counts_as_meeting_the_npc_of_a_step`).

### 4.11 The limits of a quest

| Limit | Value | Why |
|---|---|---|
| Steps | 1 to 4 (`MAX_STEPS`, was 3) | An any-order set and a wait need room. A wait or a level step adds little play, so 4 keeps at most 3 real goals around a wait. |
| Steps of a choice quest | 1 or 2 plain steps, as standing.md 4.5 says | Unchanged. |
| Any-order sets | At most 1, of 2 or 3 steps | 5.1 |
| Waits | At most 1, 1 to 3 days | 4.4 |
| Kills | 1 to 10 | Unchanged |
| Carry | 1 to 20 | A stack of 20 is the common stack of cloth. |
| Levels ahead | 1 to 3 | More is a grind, not a quest. |
| Talk topic | at most 60 characters | One short phrase in a step line. |
| Targets | Each step a different target, as today | One event never does two steps of one quest. |

## 5. New structure

### 5.1 Any order

In the JSON, a set is one entry of `steps`:

```json
{"goal": "any_order", "steps": [{"goal": "visit", "place": "Old Mill"}, {"goal": "kill", "creature": "Duskbat", "count": 4}]}
```

The parse flattens it. The steps of the set join the list in their order, and the quest keeps the span:

```rust
/// The steps from `first` to `last` (both in) can be done in any order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnyOrder { pub first: usize, pub last: usize }
```

- A set holds 2 or 3 steps, and no `wait`. A quest holds at most one set. A set inside a set fails the parse. Else `AnyOrderSize(usize)`, `AnyOrderWait`, or `AnyOrderTwice`.
- The steps of a set count toward `MAX_STEPS`.
- The whole quest can be one set.

**Why flat in the log.** `StepDone { step }` names a step by its index today. A flat list keeps that, and the span adds one small value to `Offered`. A nested list needs two indexes in every line.

### 5.2 Two endings

standing.md owns the choice quest: two NPCs, the first one met decides (standing.md 4.3 to 5.4). This plan adds only:

- A choice comes only in a quest of the genre `rivalry`. Else `ChoiceGenre`.
- The shape of a choice quest ends with the part `choice` (3.2).
- A choice quest never names the giver in a plain step, also after a wait (4.4).
- standing.md calls the encounter of a gossip window and of `/talk` both `Encounter::Talk`. This plan splits it in two (6.4). `settle_choices` takes both, as standing.md says.

### 5.3 Hidden steps

A quest of the genre `mystery` shows only its done steps and its open steps. The later steps stay hidden until they open.

- **The journal leaves them out.** The story program sends only the steps that the player can see, and the number of hidden steps (9). The addon never gets them, so no other addon can read them from the saved variables.
- **The offer** shows only its first open steps too: the first step, or the whole set when the quest starts with a set.
- **The prompt** asks the model to name only the first step in the text (3.8).
- No other genre hides steps. The genre is the switch, so there is no flag for the model to forget.

```rust
impl Genre {
    /// A mystery shows its steps one at a time.
    pub fn hides_later_steps(self) -> bool
}
```

## 6. Progress

### 6.1 The open steps

A step is **open** when the player can do it now:

- The quest is accepted.
- The step is not done.
- Every step before its stage is done. A stage is one step, or the whole any-order set.

So in an ordered quest, the open step is the first step that is not done. When that step is in a set, every step of the set that is not done is open.

```rust
impl Tracked {
    /// The steps that the player can do now, by index.
    pub fn open_steps(&self) -> Vec<usize>
    /// When the step became open: the accept, or the time of the last step done before
    /// its stage. None while it is not open.
    pub fn opened_at(&self, step: usize) -> Option<Tick>
    /// When a wait step is over. None for another step, or a wait that is not open.
    pub fn ready_at(&self, step: usize) -> Option<Tick>
    pub fn steps_done(&self) -> usize
}
```

### 6.2 The log

`QuestChange` changes:

```rust
Offered {
    number: u64,
    at: Tick,
    giver: String,
    title: String,
    text: String,
    steps: Vec<Step>,
    /// None in an old quest file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    genre: Option<Genre>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    any_order: Option<AnyOrder>,
    // `town` and `choice` of standing.md 4.3 stay as that plan writes them.
},
```

The other lines stay. `StepDone { number, step, at }` and `Killed { number, step, at }` keep their shape.

`Tracked` changes:

| Field | Was | Now |
|---|---|---|
| `steps_done: usize` | the done steps, from the first | gone: `done` holds it, and `steps_done()` counts it |
| `kills: u8` | the kills of the next step | `kills: Vec<u8>`, one count for each step. Two kill steps of one set count apart. |
| — | — | `done: Vec<Option<Tick>>`: when each step was done |
| — | — | `accepted_at: Option<Tick>` |
| — | — | `genre: Option<Genre>`, `any_order: Option<AnyOrder>` |

The rules of `quest_log`:

- `StepDone` counts only for an open step. A `StepDone` of a step that is not open changes nothing, as today.
- **A wait never ends early.** A `StepDone` of a wait step counts only when its `at` is at least `ready_at`. An earlier one changes nothing.
- `Killed` counts only for an open kill step, and only up to its count.
- The quest is `Done` when every step is done, at the time of the last one. A choice quest waits for its choice (standing.md 4.3).

The checks of the log hold even when the story program is right, because a quest file is read again at each start, and a file can be damaged.

### 6.3 What the world shows: `Here`

```rust
/// What the world shows when a line arrives, as far as a step can use it.
pub struct Here<'a> {
    pub at: Tick,
    /// Where you stand: the subzone, then the zone.
    pub places: Vec<&'a str>,
    pub level: Option<i64>,
    /// The local hour of this line, when it carries one.
    pub hour: Option<u8>,
    /// The titles of the game quests that you turned in.
    pub game_quests_done: Vec<&'a str>,
}
```

### 6.4 What the line did: `Encounter`

standing.md 5.4 brings `Encounter` with `None`, `Talk`, and `Slap`. This plan moves it to `quest.rs` (pure code), makes it own its names (the line is gone after `dispatch`), and adds variants:

```rust
/// What one line of the addon did, as far as a step can see it.
pub enum Encounter {
    None,
    /// A gossip window of the game: `npc_met`.
    Gossip(String),
    /// `/talk`: `talk_asked`.
    Talk(String),
    Slap(String),
    Emote { emote: String, target: Option<String> },
    /// `npc_defeated`: a rare or a boss.
    Defeat(String),
    /// `items_held`.
    Carry { npc: String, item: String, count: u16 },
}
```

### 6.5 When a step holds

```rust
impl Tracked {
    /// Does the open step hold now?
    pub fn step_holds(&self, step: usize, here: &Here<'_>, met: &Encounter) -> bool
}
```

| Step | Holds when |
|---|---|
| `visit` | `here.places` holds the place |
| `visit_at` | `here.places` holds the place, and `here.hour` falls in its time |
| `meet` | `Gossip`, `Talk`, or `Slap` of the NPC |
| `talk` | `Talk` of the NPC |
| `emote` with `npc` | `Emote` with the token and that target |
| `emote` with `place` | `Emote` with the token, and `here.places` holds the place |
| `slap` | `Slap` of the NPC |
| `kill` | `kills[step]` is at least the count |
| `defeat` | `Defeat` of the boss |
| `enter` | `here.places` holds the dungeon |
| `level` | `here.level` is at least the level |
| `game_quest` | `here.game_quests_done` holds the title |
| `carry` | `Carry` with the NPC, the item, and a count at least the step count |
| `wait` | `here.at` is at least `ready_at(step)` |

A step that reads the world (`visit`, `visit_at` with an hour, `enter`, `level`, `game_quest`, `wait`) holds at once when it opens, if the world already shows it. This is the rule of today for a visit (3.4): the addon sends a zone only when it changes, and a level or a turn-in comes only once.

### 6.6 One place moves the quests

Today `enter_zone`, `meet_npc`, `slap_npc`, `talk`, and `count_kill` each call `advance_quests`. A wait needs every line with a time. So:

1. `handle_unsaved` takes the `Encounter` of the line before `dispatch` (`fn encounter(input: &Input) -> Encounter`), and its hour.
2. `dispatch` changes the world, as today.
3. For every kept line with a time (`Input::at_mut` is `Some`), `advance_quests(at, hour, &encounter)` runs once.
4. Then `settle_choices` of standing.md runs.

`advance_quests` loops as today: it finds the first quest and open step that holds, adds `StepDone`, and looks again with the same line. So one line can finish a wait and the talk after it. Each turn of the loop adds one done step, so the loop ends.

`answer_quest` still calls `advance_quests` at the accept, with `Encounter::None`.

`/quest` (`quest_asked`) gets `Encounter::None`: asking moves no `meet` step, as standing.md 5.4 says. A wait or a level can still finish on it.

## 7. The story program

### 7.1 New lines in `input.rs`

```rust
/// The local hour changed while a time-of-day step is open (docs/plans/quest-variety.md 4.5).
HourChanged { at: Tick, hour: u8 },
/// The count of one item in your bags, at a meeting with the NPC of an open carry step.
ItemsHeld { at: Tick, npc: String, item: String, count: u16 },
```

`ZoneEntered` gets `#[serde(default)] hour: Option<u8>`.

- Both new lines are kept inputs with the root Game (5.14). Neither is a question.
- `checked_hour` checks each hour, as for emotes. `checked_name` checks the NPC and the item.
- `HourChanged` changes no fact. `ItemsHeld` changes no fact either: a count is no fact of the world. They only move steps.

### 7.2 `Known`

`Known` gets the new lists, each from one function of `Character`, newest first and near the giver first as today:

| Field | Source |
|---|---|
| `level: Option<i64>` | `Character::level` |
| `dungeons: Vec<&str>` | `Character::dungeons_entered` |
| `bosses: Vec<&str>` | `Character::bosses_defeated` |
| `game_quests: Vec<&str>` | `Character::game_quests_open`, and the titles of the quests that you read, minus `game_quests_done` |
| `goods: Vec<&str>` | `carry_items.txt`, for the level now |
| `recent: Vec<Recent>` | `recent_quests` (3.4) |

Each list has a function next to `places`, `people`, and `prey`, so the prompt and the check use the same rule (3.4: "One function of the code decides both the lists and the check"): `dungeons()`, `bosses()`, `game_quests()`, `goods()`. The emotes come from the data file. The lists hold at most 20 names each, as today.

### 7.3 The check, in order

`checked_quest` runs:

1. The JSON, the title, the genre, and the text (`NotJson`, `BadTitle`, `NoGenre`, `UnknownGenre`, `BadText`, `NotFromGiver`).
2. The flat steps and the set (`StepCount`, `AnyOrderSize`, `AnyOrderWait`, `AnyOrderTwice`).
3. The title against the game quests that you read (`GameQuest`), as today.
4. Each step: its target and its limits (`step_fault`, with the new faults of 4).
5. The structure: one wait, not first, not last (`WaitPlace`); the giver only after a wait (`MeetGiver`); a slap only in a comic (`SlapOutsideComic`); a choice only in a rivalry (`ChoiceGenre`); different targets (`RepeatedStep`).
6. Variety (`SameShape`, `SameTitleWord`).
7. The length of the offer line (`TooLong`), as today.

`step_fault` stays one `match` over the step, with one small function for each new kind (`talk_fault`, `carry_fault`, and so on), so no arm grows deep.

## 8. The journal (desktop)

Today the journal sends `Tracked` as it is. It now sends a view, so the hidden steps stay out:

```rust
/// A side quest as the book shows it.
#[derive(Serialize)]
pub struct QuestView {
    pub number: u64,
    pub offered_at: Tick,
    pub giver: String,
    pub title: String,
    pub text: String,
    pub status: Status,
    pub done_at: Option<Tick>,
    /// The steps that the player can see, in order.
    pub steps: Vec<StepView>,
    /// The span of the set, when every step of it is in `steps`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_order: Option<AnyOrder>,
    /// The steps of a mystery that stay hidden.
    #[serde(skip_serializing_if = "is_zero")]
    pub hidden_steps: usize,
    /// True when a step asks you to slap someone (11).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub has_slap: bool,
}

#[derive(Serialize)]
pub struct StepView {
    #[serde(flatten)]
    pub step: Step,
    pub state: StepState,
    /// The kills so far of a kill step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kills: Option<u8>,
    /// When an open wait is over.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_at: Option<Tick>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState { Done, Open, Later }
```

- An offer shows its steps as `Later`, but a mystery offer shows only its first stage.
- The genre stays out of the journal: the player never sees it.
- The pages hold a little more per quest. `Size::of` measures the view, so the paging of the journal stays right.
- A field of the journal needs no change in the bridge (16).

## 9. Proof (5.14)

### 9.1 Rows

Each `StepDone` row rests on the input of its line, as today:

| Step | The line that does it | Root |
|---|---|---|
| `visit`, `visit_at`, `enter` | `zone_entered`, or `hour_changed` for a time | Game |
| `meet` | `npc_met` (Game), `talk_asked` (Player), or `npc_slapped` (Game) | by the line |
| `talk` | `talk_asked` | Player |
| `emote`, `slap` | `emote_done`, `npc_slapped` | Game |
| `kill`, `defeat` | `npc_killed`, `npc_defeated` | Game |
| `level` | `level_reached` | Game |
| `game_quest` | `game_quest_done` | Game |
| `carry` | `items_held` | Game |
| `wait` | any line with a time | by the line |

A step that holds at once when it opens rests on the line that opened it, because that line made the `StepDone` row.

A talk step makes a finished quest rest on Player proof, because the player typed `/talk`. That is the rule of 5.14: the weakest root shows.

### 9.2 Reads

The quest offer reads more, because its prompt holds more:

| Call | Reads (new parts in bold) |
|---|---|
| Quest offer | the events behind the giver and every place, NPC, and creature that the prompt can offer, **every dungeon, boss, and game quest that it can offer, the events behind your level fact, and every `quests` row of the offers of the recent block** |
| Quest retry | **what its first call read, and the first call** |
| Talk | as npc-memory.md says, **and every `quests` row of each quest whose talk line it holds (4.2)** |

The `quests` rows use `Source::Quest` of npc-memory.md 3.1. The goods and the emotes are data files, not rows, so a read names none of them.

## 10. The addon

### 10.1 The journal page

`Journal.lua` reads `QuestView`:

- `StepText(step)` becomes a table of one small function for each goal, so each kind is one entry. The copy is in 11.
- The mark of a step comes from `step.state`: " (Complete)" for `done`, as today. A `later` step shows faded, as the later steps of a choice do (standing.md 10).
- An any-order set gets the line "In any order:" before its first step.
- `hidden_steps` above 0 adds the line "More to come." after the last step.
- The list mark: "2 of 4" as today, or "2 done" when steps are hidden.
- A wait that is open shows the time left from `ready_at` and `time()`. When `time()` passes `ready_at`, the line shows " (Complete)" at once: the desktop records it with the next line, and the next step counts on that same line (6.6).
- A carry step shows the count in your bags now (`C_Item.GetItemCount`), and the page draws again on `BAG_UPDATE_DELAYED` while it shows a carry step.
- A quest with `has_slap` gets one more line under Rewards (11).
- `StepsDone`, `StepKills`, and the old `kills` field go away.

`TaskPins.lua` pins the new steps that have a position: an NPC step (`talk`, `emote` at an NPC, `slap`, `carry`) at its NPC, and a place step (`emote` in a place, `visit_at`, `enter`) at its place. `level`, `wait`, `defeat`, and `game_quest` get no pin. A hidden step gets no pin, because the journal never sent it.

### 10.2 `QuestSteps.lua` (new): what the open steps need from the addon

`Foes.lua` reads the journal for the next kill step today, and asks for a new journal after a batch while a later kill step waits. More kinds need the addon now: kills, carries, and times. One module owns that:

```lua
-- The open steps of your side quests that the addon must watch (docs/plans/quest-variety.md 10.2).
QuestSteps.Read(quests)       -- from each journal: the open kill, carry, and time steps
QuestSteps.Hunted()           -- the creatures of open kill steps, for Foes
QuestSteps.CarriesFor(npc)    -- the items of the open carry steps of this NPC
QuestSteps.WatchesHour()      -- true while a visit_at step is open
QuestSteps.EventsSeen()       -- asks for the journal after a batch, at most once a minute,
                              -- while a later step or a hidden step can need the addon
```

- `Foes.Hunt` takes the creatures of every open kill step, not only the "next" one, because a set can hold two.
- A hidden step can be a kill, a carry, or a time. So `hidden_steps` above 0 counts as "a later step waits", and the addon asks for the journal after a batch.

### 10.3 The local hour

- `Inputs.Zone` adds `hour = Inputs.Hour()`.
- A ticker (`C_Timer.NewTicker`, each 60 seconds) compares `Inputs.Hour()` with the last hour. When it changed and `QuestSteps.WatchesHour()` is true, it adds `hour_changed` to the outbox. The line waits for the next flush, as every small event does (5.4). Its `at` is the time of the change, so the order of the world stays right.

### 10.4 `Carry.lua` (new): the count of an item

```lua
-- The count of the item of an open carry step, at a meeting with its NPC
-- (docs/plans/quest-variety.md 4.9). Only that item, only then: Timeways never watches your bags.
Carry.Met(npc)
```

- `Watch.Npc` calls `Carry.Met(npc)` before its 5-minute rule, so a player who comes back with more cloth a minute later still counts. `Talk.Ask` calls it too.
- For each item of `QuestSteps.CarriesFor(npc)`, it reads `C_Item.GetItemCount(item)` and adds `items_held`.
- A count that `issecretvalue` hides, or that is not a whole number, sends nothing.
- At most one line for each NPC and item in 10 seconds, so a gossip window that opens and closes fast sends one.
- `C_Item.GetItemCount` and `BAG_UPDATE_DELAYED` go through the API gate (`scripts/wow-api.sh` of Gnomish Relay) before use. Both exist in the Forever build 1.60.1.70009: the function in `ItemDocumentation.lua` (`itemInfo`, then four optional flags, returns `count`), and the event in `Events.lua`. The run writes them into `addon/tests/api.lua` and `api-signatures.lua`.

### 10.5 What the addon sends, in short

| Line | When | New |
|---|---|---|
| `zone_entered` | as today | `hour` |
| `hour_changed` | the local hour changes while a `visit_at` step is open | new line |
| `items_held` | a gossip window or `/talk` with the NPC of an open carry step | new line |
| `emote_done`, `npc_slapped`, `npc_defeated`, `level_reached`, `game_quest_done`, `talk_asked` | as today | nothing: the story program reads them for steps now |

## 11. UI copy

The copy follows the UI copy rules of `CLAUDE.md`. Every line that a player reads:

| Step | Line | Done |
|---|---|---|
| visit | Visit Old Mill. | Visit Old Mill. (Complete) |
| meet | Speak with Farmer Bram. | (as today) |
| kill | Duskbat slain: 2/6 | Duskbat slain: 6/6 (Complete) |
| talk | Talk to Farmer Bram (/talk). | Talk to Farmer Bram (/talk). (Complete) |
| talk with a topic | Ask Farmer Bram about the missing cask (/talk). | (Complete) |
| emote at an NPC | Use /bow on Sister Ada. | (Complete) |
| emote in a place | Use /dance in Goldshire. | (Complete) |
| time of day | Visit Old Mill at night (9 PM to 5 AM). | (Complete) |
| wait, later | Wait 2 days. (faded) | |
| wait, open | Wait 2 days: 1 day left. / Wait 2 days: 5 hours left. / Wait 2 days: less than an hour left. | Wait 2 days. (Complete) |
| level | Reach level 14. | (Complete) |
| enter | Enter The Deadmines. | (Complete) |
| defeat | Edwin VanCleef slain: 0/1 | Edwin VanCleef slain: 1/1 (Complete) |
| game quest | Complete "The Defias Brotherhood". | (Complete) |
| slap | Use /slap on Farmer Bram. | (Complete) |
| carry | Bring Linen Cloth to Farmer Bram: 6/10 | Bring Linen Cloth to Farmer Bram: 10/10 (Complete) |
| any-order set | In any order: | |
| hidden steps | More to come. | |
| list mark, hidden steps | 2 done | |
| Rewards, a quest with a slap | Farmer Bram will like you less. | |

- The `/talk`, `/bow`, and `/slap` in a line are the commands of the game, so the player knows what to type. WoW writes emote objectives the same way.
- A carry step shows the count in your bags, as the game shows a collect objective. The player keeps the items, and the line never says "hand over".
- The hours of a time step use AM and PM, as the clock of the game does for an English client.
- The topic of a talk step and the text of a quest are model text: `ns.Plain` cleans them, as today.

Bad and good:

| Bad | Good | Why |
|---|---|---|
| Talk step: talk_asked to Farmer Bram | Talk to Farmer Bram (/talk). | "Step" and the line name are internal words. |
| Wait 172800 seconds. | Wait 2 days: 1 day left. | A person counts in days and hours. |
| Come back when the sands of time allow. | Wait 2 days. | Fake flourish. The UI never performs a voice. |
| Be present at Old Mill during hours 21–4 local. | Visit Old Mill at night (9 PM to 5 AM). | Robot form language. |
| Carry 10 Linen Cloth (count checked on meeting). | Bring Linen Cloth to Farmer Bram: 6/10 | Name the goal, not the mechanism. |
| Hidden steps: 2 | More to come. | A mystery keeps its secret. A count is a mechanism. |
| Emote objective: DANCE @ Goldshire | Use /dance in Goldshire. | The token is internal. The command is what the player types. |
| Slapping costs 10 trust and 1 standing. | Farmer Bram will like you less. | No numbers. The words of trust (3.5). |
| Genre: mystery | (nothing) | The genre is internal. The player feels it in the story. |
| Turn in game quest #1234 | Complete "The Defias Brotherhood". | The title the player knows, in the words of the game. |

A refused offer and a failed retry keep the line of today: "Farley has no quest for you now." The player never sees a retry.

## 12. Edge cases

| Case | Rule |
|---|---|
| A level step that you already reached | The check refuses a level at or below yours (`LevelOutOfReach`). A level reached after the offer, or before the step opens, holds at once when the step opens. A level never goes down: `reach_level` refuses a lower one. |
| A wait across a restart of the desktop | The quest file holds the times of the done steps, so `ready_at` comes back. The first line after it ends the wait: the `zone_entered` of the login at the latest. |
| A wait while the game is closed | The same. The wait ends at the first line of the next session, and the step after it can count on that same line. |
| A clock that went back | `checked_time` moves the line to the time of the last event. A wait never ends before its time. |
| A clock that jumped ahead, by less than a day | `checked_time` accepts it, so a wait can end early by that jump. It is story only (rule 2), so this plan accepts it. A jump of more than a day is refused, as today. |
| A game quest that you already turned in | The check refuses it (`GameQuestDone`). A turn-in after the offer, or before the step opens, holds at once when the step opens. |
| A game quest that you abandon in the game | The step waits. You abandon the side quest, as for a kill step of a creature that turned friendly. |
| Two quests of the game with one title | The step matches the title. The first turn-in of that title counts. |
| An item count that drops before the meeting | The count is read at the meeting. Below the step count, the step stays open, and the page shows the count of your bags now. |
| An item count that drops after the step is done | The step stays done: you had them when you met the NPC. |
| The items are in the bank | The count reads the bags only. The page shows the bag count, so the player sees why. |
| An item name that the client does not know yet | `GetItemCount` gives 0 for an item that is not in the bags. The step stays open. |
| An instance with no zone name | The zone text is empty during a loading screen, and `Watch.Zone` sends nothing then. The name comes with the next `ZONE_CHANGED_NEW_AREA`, and the step counts then. A dungeon becomes a target only after the world holds its mark, so the check never offers a name that the addon never sent. |
| A boss whose `ENCOUNTER_END` name differs from its unit name | The step takes the name from `defeated`, which came from the addon. Either event gives the name that it gave before. |
| You stand in the dungeon at the accept | The enter step holds at once, as a visit does. |
| A line with no hour, from an older addon | No time step counts. The step waits until the addon sends hours. |
| The hour changes while you stand in the place | `hour_changed` does the step. |
| A time step and the change to daylight saving | The hour is the hour of the computer. The step uses it as it comes. |
| A talk step, and no model answers | The step is done: you talked. The NPC "looks at you and says nothing", as today. |
| A talk step, and the conversation is full | The addon never sends that turn (talk-window.md 4.2). A new `/talk` starts a new conversation and does the step. |
| A carry step to the giver after a wait | The `items_held` of a gossip window with the giver does it. |
| The NPC of a talk, carry, emote, or slap step dies in your story | The step waits until you abandon the quest. |
| The giver dies during a wait before a step to the giver | The same. |
| An any-order set with two kill steps of one creature | Not possible: each step names a different target. |
| A kill of a creature of an open set step, before the stage before it is done | It counts nothing, as a kill before the step opens counts nothing today. |
| A mystery with an any-order set that is not open yet | The set stays hidden, and `any_order` stays out of the view. |
| An old quest file with no genre, no span, and `steps_done` lines | `genre` and `any_order` default to none. Every old step is a plain step in order, so the rules of 6.2 read the old lines as before. |
| An old quest in the recent block | It shows with its title and shape, and no genre. |
| A new player with few targets | The shape rule compares the whole order. "visit", "visit, meet", and "meet, visit" are three shapes, so even a small world has room. When the model still fails twice, the giver has no quest now, and asks again later. |
| The retry answer repeats the first answer | The check refuses it again. No quest. |
| A slap step at an NPC that became hostile | The addon sends `npc_slapped` only for a target that you can see as an NPC. The step counts as any slap. |
| A slap step in a choice quest | Not possible: a choice is a rivalry, and a slap is comic. |

## 13. Tests

Each test is a sentence, and reads arrange, act, assert. No test needs the game or a model.

### 13.1 `crates/story/tests/quest.rs`: the check and the log

Talk:

- `a_talk_step_names_an_npc_that_you_can_meet`
- `a_talk_step_never_names_a_foe_or_an_animal`
- `a_talk_topic_over_sixty_characters_is_refused`
- `a_talk_topic_with_a_name_after_the_cutoff_is_refused`

Wait:

- `a_wait_lasts_one_to_three_days`
- `a_wait_is_never_the_first_or_the_last_step`
- `a_quest_has_at_most_one_wait`
- `a_step_after_a_wait_can_name_the_giver`
- `a_step_with_no_wait_before_it_never_names_the_giver`
- `a_choice_quest_never_names_the_giver_in_a_step`
- `a_wait_opens_when_the_step_before_it_is_done`
- `a_wait_done_before_its_time_changes_nothing`
- `a_wait_done_at_its_time_counts`

Any order:

- `an_any_order_set_holds_two_or_three_steps`
- `a_quest_has_at_most_one_any_order_set`
- `an_any_order_set_holds_no_wait`
- `a_set_inside_a_set_is_refused`
- `an_any_order_set_is_flattened_with_its_span`
- `each_step_of_an_open_set_can_be_done_in_any_order`
- `the_step_after_a_set_opens_only_when_the_whole_set_is_done`
- `two_kill_steps_in_a_set_count_their_kills_apart`

Carry:

- `a_carry_step_names_a_good_of_your_level_band`
- `a_good_outside_your_level_band_is_refused`
- `a_carry_step_asks_for_one_to_twenty_items`
- `a_good_in_the_text_of_a_game_quest_is_still_a_goal`
- `the_npc_of_a_carry_step_keeps_the_overlap_rule`
- `a_carry_step_holds_for_a_count_at_its_number`
- `a_count_below_the_number_leaves_the_carry_step_open`
- `a_count_for_another_npc_or_item_leaves_the_carry_step_open`

Variety:

- `an_offer_with_the_shape_of_the_last_quest_is_refused`
- `an_offer_with_the_shape_of_the_quest_before_the_last_is_refused`
- `an_offer_with_the_shape_of_the_third_newest_quest_passes`
- `the_order_inside_an_any_order_set_does_not_change_the_shape`
- `the_order_of_ordered_steps_changes_the_shape`
- `a_choice_adds_its_part_to_the_shape`
- `a_title_that_shares_a_main_word_with_one_of_the_last_three_titles_is_refused`
- `a_title_that_shares_a_main_word_only_with_the_fourth_newest_title_passes`
- `a_title_that_shares_only_stop_words_passes`
- `a_main_word_matches_in_any_case`
- `a_word_of_one_letter_is_never_a_main_word`
- `a_declined_offer_counts_as_a_recent_quest`
- `an_offer_with_no_genre_is_refused`
- `an_offer_with_an_unknown_genre_names_it_in_the_fault`
- `a_text_that_never_speaks_for_the_giver_is_refused`
- `the_prompt_lists_the_last_three_quests_with_shape_and_genre`
- `the_prompt_leaves_out_the_recent_block_with_no_earlier_quest`
- `the_prompt_shows_a_goal_only_when_its_list_has_a_name`
- `an_old_offer_with_no_genre_or_span_still_reads`

Emote:

- `an_emote_step_takes_a_token_of_the_quest_emote_list`
- `a_rude_emote_is_no_quest_emote`
- `an_emote_step_names_an_npc_or_a_place_and_never_both`

Time of day:

- `each_time_of_day_starts_and_ends_at_its_hours` (hours 4, 5, 7, 8, 10, 11, 13, 14, 17, 18, 20, 21, 23, 0)
- `night_wraps_past_midnight`
- `a_time_step_with_an_unknown_time_is_refused`

Level:

- `a_level_step_asks_for_one_to_three_levels_above_yours`
- `a_level_step_never_asks_past_sixty`
- `a_level_that_you_already_have_is_refused`
- `a_player_with_no_level_gets_no_level_goal`

Dungeon:

- `an_enter_step_names_a_dungeon_or_a_raid_that_you_entered`
- `an_outdoor_zone_is_no_dungeon`
- `a_defeat_step_names_a_boss_that_you_defeated_in_a_dungeon`
- `a_rare_defeated_outside_a_dungeon_is_no_boss`

Game quest:

- `a_game_quest_step_names_a_quest_that_you_hold_or_read`
- `a_game_quest_that_you_turned_in_is_refused`
- `a_game_quest_step_skips_the_overlap_rule_for_its_own_title`
- `the_title_of_a_quest_with_a_game_quest_step_keeps_the_overlap_rule`

Slap:

- `a_slap_step_comes_only_in_a_comic_quest`
- `a_slap_step_never_names_the_giver`
- `a_slap_step_never_names_an_npc_with_a_cruel_word`

Structure:

- `a_quest_has_one_to_four_steps`
- `a_choice_comes_only_in_a_rivalry`
- `a_mystery_hides_its_later_steps`
- `no_other_genre_hides_a_step`

### 13.2 `crates/story/tests/quests.rs`: the story program

- `a_talk_does_a_talk_step`
- `a_gossip_window_never_does_a_talk_step`
- `a_slap_never_does_a_talk_step`
- `a_talk_to_the_npc_of_a_talk_step_tells_the_npc_about_the_quest`
- `the_quest_line_stays_in_the_later_turns_of_the_conversation`
- `a_wait_ends_with_the_first_line_after_its_time`
- `a_wait_and_the_talk_after_it_end_on_one_line`
- `a_wait_survives_a_restart_of_the_story_program`
- `a_wait_never_ends_on_a_clock_that_went_back`
- `the_steps_of_a_set_end_in_the_order_that_you_do_them`
- `items_held_at_its_npc_does_a_carry_step`
- `items_held_below_the_count_leaves_the_carry_step_open`
- `an_emote_at_the_npc_of_the_step_does_it`
- `an_emote_at_another_npc_does_nothing`
- `an_emote_in_the_place_of_the_step_does_it`
- `a_zone_at_the_right_hour_does_a_time_step`
- `a_zone_at_the_wrong_hour_leaves_the_time_step_open`
- `an_hour_change_while_you_stand_in_the_place_does_the_time_step`
- `a_line_with_no_hour_never_does_a_time_step`
- `a_level_reached_before_the_step_opens_does_it_at_once`
- `standing_in_the_dungeon_when_the_step_opens_does_it_at_once`
- `a_boss_defeated_does_a_defeat_step`
- `a_game_quest_turned_in_before_the_step_opens_does_it_at_once`
- `a_slap_step_costs_trust_as_any_slap`
- `a_slap_step_counts_in_the_slaps_of_the_town`
- `asking_for_a_quest_moves_no_meet_step`
- `a_refused_first_answer_asks_once_more_with_the_reason`
- `a_second_refused_answer_gives_no_quest`
- `a_failed_first_call_asks_no_retry`
- `a_refusal_of_the_limits_asks_no_model`
- `the_retry_keeps_the_batch_of_its_first_call`
- `the_retry_call_rests_on_the_first_call`
- `a_retry_does_not_count_for_the_hero_hook`
- `the_offer_prompt_reads_the_rows_of_the_recent_quests`
- `a_mystery_shows_only_its_done_and_open_steps_in_the_journal`
- `a_mystery_offer_shows_only_its_first_stage`
- `the_journal_counts_the_hidden_steps`
- `the_journal_leaves_out_the_genre`

### 13.3 Property tests (`crates/story/tests/properties.rs`)

The strategies grow:

- `quest_step()` makes every kind. Counts make the edges likely: 0, 1, the limit, and the limit plus 1 (kills 0, 1, 10, 11; carry 0, 1, 20, 21; days 0, 1, 3, 4; levels the player level, plus 1, plus 3, plus 4, 60, 61).
- `quest_change()` makes `Accepted` and `StepDone` with times near the end of a wait likely: `ready_at - 1`, `ready_at`, `ready_at + 1`. It makes offers with an `any_order` span at the start, the middle, and the end.
- Hours make 4, 5, 7, 8, 20, 21, 23, and 0 likely.
- New plays: `Play::Wait(seconds)` (one day made likely), `Play::HourChanged(hour)`, `Play::ItemsHeld(npc, item, count)`, and quest answers of every step kind and genre.

The properties:

- `no_offer_repeats_the_shape_of_the_last_two`. Pure: any `Known` with any recent quests, and any answer built from any steps. When `checked_quest` passes, the shape is not the shape of `recent[0]` or `recent[1]`, and no main word of the title is a main word of the first 3 recent titles.
- `in_an_ordered_quest_no_step_is_done_before_the_step_before_it`. Any list of `QuestChange` with no span: the done steps of each quest are always the first steps, with no gap.
- `an_any_order_set_is_done_only_when_all_its_steps_are_done`. Any list with spans: no step after a set is done while a step of the set is not.
- `a_wait_never_ends_early`. Any list with waits and random times: each done wait has a done time at least `opened_at + days * 86400`.
- `a_quest_log_never_skips_a_step_and_each_giver_holds_at_most_one_offer` (today) changes to the per-step state: `steps_done()` is at most the step count, and a quest is `Done` exactly when every step is done.
- `kills_count_only_for_the_next_kill_step_of_an_accepted_quest_and_never_past_its_count` (today) becomes `kills_count_only_for_an_open_kill_step_and_never_past_its_count`.
- `every_offer_in_play_differs_in_shape_from_the_two_before_it`. Any play: the `Offered` lines of the log, in order, each have a shape that differs from the two before it.

### 13.4 Addon tests (`crates/addon-tests/tests/`)

`quest.rs`:

- `each_step_kind_shows_its_line` (one row for each line of 11)
- `a_later_step_shows_faded`
- `an_open_wait_shows_the_time_left_in_days_and_hours`
- `a_wait_whose_time_passed_shows_complete`
- `an_any_order_set_shows_under_in_any_order`
- `a_mystery_shows_more_to_come`
- `the_list_mark_of_a_mystery_counts_the_done_steps`
- `a_carry_step_shows_the_count_in_your_bags`
- `the_page_draws_again_when_the_bags_change`
- `a_quest_with_a_slap_says_the_npc_will_like_you_less`
- `a_step_of_an_unknown_goal_shows_a_gap_and_no_error`

`quest_steps.rs` (new):

- `every_open_kill_step_hunts_its_creature`
- `a_later_kill_step_makes_the_addon_ask_for_the_journal_after_a_batch`
- `a_hidden_step_makes_the_addon_ask_for_the_journal_after_a_batch`
- `the_hour_goes_out_when_it_changes_while_a_time_step_is_open`
- `no_hour_goes_out_with_no_open_time_step`
- `a_zone_line_carries_the_local_hour`

`carry.rs` (new):

- `meeting_the_npc_of_a_carry_step_sends_the_count`
- `talking_to_the_npc_of_a_carry_step_sends_the_count`
- `a_meeting_within_five_minutes_still_sends_the_count`
- `a_meeting_with_another_npc_sends_no_count`
- `a_hidden_count_is_never_sent`
- `one_npc_and_item_send_at_most_once_in_ten_seconds`

`task_pins.rs`: `the_new_npc_and_place_steps_get_pins`, `a_wait_a_level_and_a_game_quest_get_no_pin`.

## 14. Fuzz

- `fuzz/fuzz_targets/answers.rs`: `known()` gets every new list (a dungeon, a boss, a game quest, a level of 12, two goods, and 3 recent quests with shapes and titles). `assert_quest` checks each step of a passed offer: its target is in its list, its count or level or days is in its range, an emote is on the list with exactly one target, a slap comes only in a comic, a wait is never first or last and at most once, a set holds 2 or 3 steps and no wait, the giver comes only after a wait, the shape differs from the first 2 recent shapes, and no main word of the title is a main word of a recent title. It also runs `variety::main_words` on the raw text, and checks that each word is lower case and not a stop word.
- New seeds in `fuzz/seeds/answers/`, one for each kind and rule: `talk_quest.txt`, `talk_topic_long.txt`, `wait_quest.txt`, `wait_last.txt`, `any_order_quest.txt`, `any_order_nested.txt`, `carry_quest.txt`, `carry_count_21.txt`, `emote_npc_quest.txt`, `emote_both_targets.txt`, `visit_at_quest.txt`, `level_quest.txt`, `enter_quest.txt`, `defeat_quest.txt`, `game_quest_step.txt`, `slap_comic.txt`, `slap_errand.txt`, `mystery_quest.txt`, `same_shape.txt`, `same_title_word.txt`, `no_genre.txt`.
- `fuzz/fuzz_targets/input.rs`: `hour_changed` and `items_held` with random fields (an hour of 24, a count past `u16`, a long item name, a string for a number), and `zone_entered` with a random `hour`. New seeds in `fuzz/seeds/input/`: `hours.txt`, `items_held.txt`.
- `fuzz/fuzz_targets/play.rs` and `pages.rs`: plays with waits, hours, item counts, and quests of every kind. After each line, the model of the fuzzer checks that no hidden step is in a page, and that the done steps of each quest follow the rules of 6.2.
- `fuzz/fuzz_targets/store.rs`: quest files with odd `genre` and `any_order` fields, a span past the steps, and `StepDone` lines of a wait at random times. The open never fails on them.
- `fuzz/fuzz_targets/replies.rs`: a journal with random `state`, `ready_at`, `hidden_steps`, and step goals. The addon never raises a Lua error.

## 15. A note for the npc-memory and standing sessions

- npc-memory.md: the hook count (10.2) ignores `quest_retry` rows. The talk prompt gets one quest line (4.2), after the memory block.
- standing.md: `Encounter` moves to `quest.rs`, owns its names, and splits `Talk` into `Gossip` and `Talk` (6.4). `settle_choices` takes both. `Tracked::steps_done` becomes a method, so `awaits_choice` uses `steps_done()`. A choice needs the genre `rivalry`.

## 16. Relay impact

**No relay change.**

- `items_held` and `hour_changed` are game events with no reply. Gnomish Relay SPEC.md says: "Any other `type` is a game event with no reply ... The story program checks its fields, and ignores a type that it does not know. So a new Timeways event needs no change in the bridge." The bridge knows only `character_entered`, `lore_asked`, `journal_asked`, `talk_asked`, and `draft_asked` (`app-protocol/src/addon_lines.rs`).
- The new `hour` of `zone_entered` is a field of such an event. The bridge passes it as it is.
- The new fields of the journal need no change: "Only `page` and `pages` have a fixed shape. The rest is bounded JSON, so a new field of the journal needs no change in the bridge" (`story_lines.rs`). The view adds no level of depth: `any_order` sits next to `steps`.
- The retry is one more `model_call` of the story pool, with no new field. It spends the budget of that pool (talk-window.md 7.2).

Per the rule of this project, tell the Gnomish Relay session about the new lines anyway, when step 3 and step 6 of the build order land, so its docs list them.

## 17. Build order

The user's order first: talk, come back later, any order, carry items. Then the rest. Each step is one commit with its tests, and each one ships.

1. **Per-step state.** `Tracked` gets `done`, `kills` per step, and `accepted_at`. `open_steps`, `opened_at`. `QuestView` and `StepView` in the journal, and the addon reads `state`. No new kind yet, and play is as before.
2. **One place moves the quests.** `Encounter` in `quest.rs`, `Here`, `step_holds`, and one call of `advance_quests` after each line (6.6). Play is as before.
3. **Talk.** `Step::Talk`, its check, its prompt line, its UI line, and the quest line in the talk prompt (4.2). Tell the npc-memory session.
4. **Come back later.** `Step::Wait`, `ready_at`, the log rule, the giver after a wait, and the line with the time left.
5. **Any order.** The parse of a set, `AnyOrder`, open steps of a set, kills per step in the addon (`QuestSteps.lua`, `Foes.Hunt`), and "In any order:".
6. **Carry items.** `carry_items.txt`, `Step::Carry`, `items_held`, `Carry.lua`, the API gate for `C_Item.GetItemCount` and `BAG_UPDATE_DELAYED`, and the bag count on the page. Tell the relay session (16).
7. **Genre and variety.** `Genre`, `Shape`, `main_words`, `stop_words.txt`, `recent_quests`, the recent block, the faults, and the reason rule (3.7).
8. **The retry.** `Attempt` in `calls.rs`, `prompt::retry` with string reasons, `quest_retry` rows, and the hook rule.
9. **Emote and slap.** `quest_emotes.txt`, `Step::Emote`, `Step::Slap` (comic only), and the Rewards line of a slap.
10. **Time of day.** `TimeOfDay`, `Step::VisitAt`, `hour` on `zone_entered`, `hour_changed`, and the ticker. Tell the relay session.
11. **Level, dungeon, and game quest.** `Step::Level`, `Step::Enter`, `Step::Defeat`, `Step::GameQuest`, and the four `Character` lists.
12. **Hidden steps.** The mystery view, the offer of its first stage, "More to come.", and the journal refresh for hidden steps.
13. **Choice link.** After standing.md step 4: a choice needs a rivalry, and its shape part.
14. **`GAMEPLAY.md`.** The text of section 19. This plan marks the steps as done.

Steps 9 to 12 do not depend on each other, and can come in any order after step 8.

## 18. Open questions

1. **Genre or tone.** This plan says "genre", because standing.md owns "tone". Is the extra genre `hunt` right?
2. **A genre twice in a row.** The prompt asks for another genre, but the check refuses only the shape and the title words, as approved. Refuse the genre of the last quest too?
3. **The goods.** The list of 4.9 is a guess at the level bands. A test in the game confirms each name. Later, the loot event of 5.4 (`CHAT_MSG_LOOT`) can let the check offer only goods that you looted once.
4. **The time of a wait.** The wait uses the clock of the computer, so a clock set a day ahead ends a wait early. Is that fine for a story-only step?
5. **AM and PM.** The hours of a time step show in a 12-hour clock. Do players of a 24-hour locale want "21:00 to 05:00"?
6. **The emote tokens.** A test in the game confirms each token of `quest_emotes.txt` through `PerformEmote`, as standing.md asks for its lists.

## 19. GAMEPLAY.md text

In 3.4, replace the paragraph "**The first slice.**" with:

> **Steps.** A step has one of these goals. Each one completes only from an event of the addon, or from a fact of the world that an event made:
>
> | Goal | You | The addon sees it from |
> |---|---|---|
> | `visit` | go to a place | `zone_entered` |
> | `visit_at` | are in a place at dawn, noon, dusk, or night, in your local time | `zone_entered` or `hour_changed`, with the local hour |
> | `meet` | speak with an NPC | `npc_met`, `talk_asked`, or `npc_slapped` |
> | `talk` | use `/talk` with an NPC | `talk_asked` |
> | `emote` | use an emote on an NPC, or in a place | `emote_done` |
> | `slap` | use `/slap` on an NPC. Only in a comic quest. | `npc_slapped` |
> | `kill` | kill 1 to 10 of a creature | `npc_killed` |
> | `defeat` | defeat a boss of a dungeon or a raid | `npc_defeated` |
> | `enter` | enter a dungeon or a raid | `zone_entered` |
> | `level` | reach a level, 1 to 3 above yours | `level_reached` |
> | `game_quest` | turn in a quest of the game, by its title | `game_quest_done` |
> | `carry` | have 1 to 20 of a common good in your bags when you meet an NPC. You keep them. | `items_held` |
> | `wait` | come back after 1 to 3 days | any line after the time |
>
> A quest has 1 to 4 steps. A set of 2 or 3 steps can come in any order. A quest has at most one set, and at most one wait. A wait is never the first or the last step, and never in a set.

In 3.4, in "**No overlap with the quests of the game.**", add after the first sentence:

> One step is the exception: a `game_quest` step asks you to turn in a quest of the game that you hold or read, by its title. It never names a goal of that quest. The side quest never changes it, continues it, or claims its deed. The deed and its reward stay the game's.

In 3.4, in "**The check**", add these rules:

> - The answer also has a genre: errand, hunt, mystery, rescue, rivalry, or comic. The player never sees it.
> - The text speaks for the giver: it holds "I", "me", "my", "we", "us", or "our". The prompt asks the giver to say why it needs the quest: who it is, and what the quest means where it lives.
> - A `talk` step names an NPC that a meet step can name. Its optional topic has at most 60 characters.
> - An `emote` step takes an emote of a fixed list, with no rude emote, at an NPC or in a place.
> - A `visit_at` step names a place that you visited.
> - A `slap` step comes only in a comic quest. It never names the giver, or an NPC whose name holds a word of the cruelty list.
> - An `enter` step names a dungeon or a raid that you entered. A `defeat` step names a boss that you defeated in one.
> - A `game_quest` step names a quest of the game that you hold or read, and did not turn in.
> - A `carry` step names a good of a fixed list for your level, and an NPC that a meet step can name.
> - A step can name the giver only after a wait. A choice quest never names the giver in a step.
> - **No repeated quests.** The shape of a quest is the kinds of its steps, in order. A set counts as one part, in any order. A quest never has the shape of one of the 2 newest offers. Its title shares no main word with the titles of the 3 newest offers. A main word is any word in any case, except a stop word of a fixed list and a word of one letter. The newest offers count in any state.
> - **One retry.** An answer that breaks a rule gets one more call, with the answer and the reason. A second bad answer, or a failed call, gives no quest. A refusal of the limits asks no model.

In 3.4, in "**The prompt lists only what the check allows**", add:

> The prompt also lists the dungeons, the bosses, the quests of the game, and the goods that the check allows, at most 20 of each, and shows a goal only when its list has a name. It shows the 3 newest offers with their titles, shapes, and genres, and asks for a quest that is different.

In 3.4, replace the first sentences of "**Accept and progress**" up to "in order." with:

> The story program checks the open steps against each later line. A step is open when every step before its stage is done. A stage is one step, or a whole set. So the steps of a set can come in any order, and the step after a set waits for the whole set. A wait opens when the step before it is done, and ends with the first line after its time. The clock is the time of the lines from the addon. A mystery shows only its done and open steps. The journal never carries a hidden step.

In 3.4, in "**Kills**", replace "the creature of the next step of each task in progress, when that step is a kill" with "the creatures of the open kill steps of each task in progress", and replace "The story program counts a kill only for the next step" with "The story program counts a kill only for an open kill step". Add:

> Two kill steps of one set count apart.

In 3.4, add after "**Kills**":

> - **Items** (built): while a carry step is open, a gossip window or `/talk` with its NPC makes the addon count its item in your bags (`C_Item.GetItemCount`), and send `items_held`. The addon counts only that item, and only then. You keep the items. The Tasks page shows the count in your bags: "Bring Linen Cloth to Farmer Bram: 6/10".
> - **The local hour** (built): the addon sends the hour of the computer with `zone_entered`, and sends `hour_changed` when the hour changes while a `visit_at` step is open.

In 3.5, add:

> - **A quest talk.** When a `talk` step of a quest names the NPC, the NPC gets one line: the giver, the title of the quest, and the topic. The line stays for the later turns of the conversation.

In 5.4, in the table, add a row:

> | Item count for a quest | `C_Item.GetItemCount` of the item of an open carry step, at a gossip window or `/talk` with its NPC (3.4) |

In 5.4, replace "your gold and bags, except notable loot" with:

> your gold and bags, except notable loot and the count of one item for an open carry step (3.4)

In 5.14, in the table of reads, replace the row of "Quest offer, task draft" with:

> | Quest offer, task draft | the events behind the giver and every place, NPC, creature, dungeon, boss, and quest of the game that the prompt can offer, the events behind your level, and the rows of the newest offers that the prompt shows |
> | Quest retry | what its first call read, and the first call |
