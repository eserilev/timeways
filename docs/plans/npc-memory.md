# Plan: NPCs remember you in /talk

Status: 2026-10-03. The user approved the design. Steps 1 to 5 of section 11 are done: NPC memory is built, and its rules are in `GAMEPLAY.md` 3.5, 5.11, and 5.14. Step 6 (the hook, section 10) is open.

Changes from this plan, in the build:

- `a_talk_remembers_the_quest_that_the_npc_gave` is in `tests/quests.rs`, because the quest helpers are there. `a_talk_reads_the_rows_behind_its_memories` is in `tests/links.rs`, because it reads the `reads` table of a world on disk.
- More tests than 12.1 lists: `a_finished_quest_is_remembered_as_finished` and `a_rare_that_you_defeated_is_remembered_by_the_rare_as_a_fight`. `tests/character.rs` tests `Character::first_met`.
- The property `a_talk_prompt_holds_at_most_five_memories_and_never_the_name_of_the_character` checks the name only in the lines of a rumor ("you told the player"). The code drops a rumor that names the character. A random foe or place can have the name "Ada" in a property run. That name comes from the game, and the code does not filter it.
- In `fuzz/fuzz_targets/play.rs`, `assert_memory_block` checks the fence, the count, and the length of each line, but no name. The model of the fuzz target does not know which character plays, and a rumor that names the other character is allowed. The property checks the name.
- `memory_play()` gives the quests of Farley one visit step to "Goldshire" or "Westfall", so the check lets most of them through.
- The talk prompt of `tests/voice.rs` with 5 memories at full length is 776 tokens of 1823.

No table changes. All the data exists today, so there is no migration.

## 1. Goal

An NPC that you talk to remembers your history with it. The talk prompt gets a short block of at most 5 memories. The NPC brings up at most one memory in an answer, and only when it fits.

Each memory comes from one stored row of your world: an event, a `quests` row, or a `learned` row. So the model cannot invent a past (rule 4 of `GAMEPLAY.md` 2). The talk call reads every row behind its memories (5.14). No memory holds the name of a real player (5.11).

## 2. The four kinds of memory

| Kind | Source today | Rows behind it |
|---|---|---|
| What the NPC told you | `Rumor { at, npc, text }` in `LearnedLog` (`learned.rs`, `store/logs.rs`) | the `learned` row of the rumor |
| Its quests | `QuestChange` in `QuestLog`, folded by `quest::quest_log` | every `quests` row of that quest number |
| Your first meeting | the `met` fact of you, linked to the NPC (`vocabulary::MET`) | the event that opened `met` |
| Your deeds with it or near it | the `defeated` and `deaths` facts (`journal::deeds`) | the events of the deed |

Seeing is not meeting. The `seen` fact makes no memory: the NPC never noticed you.

## 3. Types and functions

### 3.1 New module `crates/story/src/npc_memory.rs`

The name `memory.rs` is taken by chapter memory (3.3), so the new module is `npc_memory.rs`. It holds no store code. It takes plain data and gives memories.

```rust
//! What an NPC remembers of the player in /talk (GAMEPLAY.md 3.5). Each memory is a row
//! of the world, so the model gets no past to invent.

/// The memories of one prompt. More costs tokens, and the NPC uses one at most.
pub const MAX_MEMORIES: usize = 5;

/// The answers of the NPC that it remembers.
const REMEMBERED_ANSWERS: usize = 2;

/// About 40 tokens. A rumor is cut at a whole word past this.
pub const RUMOR_CHARS: usize = 160;

/// The longest line of a memory: the time words, the template, and the longest rumor or
/// two names of `MAX_NAME_BYTES`.
pub const MAX_MEMORY_CHARS: usize = 280;

/// The first meeting needs this age to be a memory. A younger one is this visit.
const FIRST_MEETING_AGE: u64 = 3_600;

/// What the player did, as far as an NPC can remember it.
pub struct Past<'a> {
    pub character: &'a Character,
    /// Each rumor with its `learned` row, oldest first.
    pub rumors: Vec<(u64, &'a Rumor)>,
    pub quests: &'a [QuestChange],
    /// The name of the character. A rumor that holds it is no memory (5.11).
    pub own_name: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    pub at: Tick,
    pub recall: Recall,
    /// The rows behind the memory. The talk call reads each one.
    pub sources: Vec<Source>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recall {
    Said { text: String },
    GaveQuest { title: String, ending: QuestEnding },
    FirstMet,
    KilledYou,
    YouDefeatedIt,
    DefeatedNear { foe: String, place: String },
    DiedNear { place: String, killer: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestEnding { Waiting, OnIt, Finished, TurnedDown, GaveUp }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source { Event(EventId), Learned(u64), Quest(u64) }

/// How close a memory is to the NPC. A closer tier comes first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tier { Between, Met, Near }
```

Functions, one job each:

| Function | Job |
|---|---|
| `pub fn memories(past: &Past<'_>, npc: &str, now: Tick) -> Vec<Memory>` | Gathers the candidates of the five functions below, and returns `chosen(...)`. |
| `fn answers_heard(past: &Past<'_>, npc: &str) -> Vec<Memory>` | The newest `REMEMBERED_ANSWERS` rumors of this NPC that do not name the character. The text goes through `learned::cut_at_word(text, RUMOR_CHARS)`. |
| `fn quests_given(changes: &[QuestChange], npc: &str) -> Vec<Memory>` | Each quest of this giver that has an ending (`quest_ending`). `at` is the time of the offer. The sources are the rows of every change of that quest number. |
| `fn quest_ending(quest: &Tracked, answered: bool) -> Option<QuestEnding>` | The ending of a status. `answered` is true when a `Declined` or `Abandoned` row of the quest exists. A `Declined` status with no such row is an offer that a newer one replaced, so it gives `None`. |
| `fn first_meeting(character: &Character, npc: &str, now: Tick) -> Option<Memory>` | The `met` fact of this NPC, when it is `FIRST_MEETING_AGE` old or more. |
| `fn deeds_with(deeds: &[DeedRow], npc: &str) -> Vec<Memory>` | The newest death that this NPC caused (`KilledYou`), and the newest defeat of this NPC (`YouDefeatedIt`). At most one of each. |
| `fn deeds_near(deeds: &[DeedRow], npc: &str, character: &Character) -> Vec<Memory>` | In the zone of the NPC: the newest defeat of each other foe, and the newest death that this NPC did not cause. |
| `fn chosen(candidates: Vec<Memory>) -> Vec<Memory>` | Applies the caps and the order of section 4. |
| `fn tier(recall: &Recall) -> Tier` | The tier of a kind. |
| `pub fn line(memory: &Memory, now: Tick) -> String` | One line of the prompt (section 5). |
| `pub fn when(then: Tick, now: Tick) -> String` | The age in words (section 5.2). |
| `fn number_word(n: u64) -> &'static str` | "two" to "twelve". |

### 3.2 Changes in other files

| File | Change |
|---|---|
| `talk.rs` | `Scene` gets `pub memories: Vec<String>`: the lines, already worded. `what_you_know` calls a new `fn what_you_remember(memories: &[String]) -> String`. |
| `learned.rs` | `excerpt` splits into `pub fn cut_at_word(text: &str, max_chars: usize) -> String`, and `excerpt` calls it with `EXCERPT_CHARS`. |
| `store/logs.rs` | `LearnedLog` gets `rumor_rows: Vec<u64>`, filled as `read_rows` is, and `pub fn rumors_with_rows(&self) -> impl Iterator<Item = (u64, &Rumor)>`. Today a rumor has no row in memory. |
| `journal.rs` | `fn deeds` becomes `pub(crate) fn deeds_with_events(world, you) -> Vec<DeedRow>`, with `pub struct DeedRow { pub deed: Deed, pub events: Vec<EventId> }`. A `Died` with a killer holds two events: the `defeated` fact of the killer and the `deaths` fact. `journal()` maps the rows to `deed`. One walk of the history serves both. |
| `character.rs` | `pub fn first_met(&self, npc: &str) -> Option<(EventId, Tick)>`: the event and the time that opened `met`. |
| `store.rs` | `CharacterKey` gets `pub fn name(&self) -> &str`. |
| `story/reads.rs` | `pub(super) fn memories_read(memories: &[Memory]) -> Vec<Node>`: each `Source` as a row (`Event` to `Table::Events`, `Learned` to `Table::Learned`, `Quest` to `Table::Quests`). |
| `story.rs`, `fn talk` | After the passages: build `Past`, call `npc_memory::memories(&past, npc, at)`, word each one with `npc_memory::line(memory, at)` into `scene.memories`, and extend `read` with `reads::memories_read`. |
| `lib.rs` | `pub mod npc_memory;` |

`fn talk` keeps its order: `meet_npc` and `advance_quests` run first. So the prompt shows the quest states after this talk moved them.

## 4. Choice and order

1. **Candidates.** The five functions of 3.1 give at most 2 answers, every quest of the giver, 1 first meeting, 2 deeds with the NPC, and the deeds near it.
2. **Caps by kind.** At most 2 quests (the newest offers) and at most 2 deeds near it. At most 1 of the deeds near it is a death.
3. **Tiers.** `Between`: what the NPC said, its quests, and your fights with it. `Met`: the first meeting. `Near`: deeds in its zone. These rank most personal first.
4. **Sort** by tier, then newest first. A tie in time keeps the order of the kinds in `Recall`.
5. **Cut** to `MAX_MEMORIES`.
6. **The prompt** shows the kept memories newest first, without the tiers. The other prompt lists (hero entries, quest targets) are newest first too.

So a long history keeps what is between you and the NPC, and drops the first meeting and the deeds nearby first. A short history shows all of it.

## 5. The prompt

### 5.1 The block

`what_you_remember` adds the block at the end of `what_you_know`, after the player's own lore. The lines go in one fence, as all data of the prompt does (`house::fenced`). So a rumor cannot close the fence.

With memories:

```
What you remember of the player, newest first. Each line is true:
<<<
- Two days ago: you told the player "The gnolls grow bold near the mill."
- Three weeks ago: you gave the player your quest "The Lost Lantern". They finished it.
- A month ago: you met the player for the first time.
>>>
Bring up at most one of these, and only when it fits what the player says. Never speak of a past with the player that is not written here.
```

With none:

```
Never speak of a past with the player that is not written here.
```

The empty case keeps the rule, because the slaps and the trust above are a past too.

The lines, one template for each kind:

| Recall | Line |
|---|---|
| `Said` | `{When}: you told the player "{text}"` |
| `GaveQuest` | `{When}: you gave the player your quest "{title}". {ending}` |
| `FirstMet` | `{When}: you met the player for the first time.` |
| `KilledYou` | `{When}: you killed the player in a fight.` |
| `YouDefeatedIt` | `{When}: the player defeated you in a fight.` |
| `DefeatedNear` | `{When}: the player defeated {foe} in {place}.` |
| `DiedNear` | `{When}: the player died in {place}.` or `{When}: the player died in {place}, killed by {killer}.` |

| Ending | Words |
|---|---|
| `Waiting` | They have not answered yet. |
| `OnIt` | They are still on it. |
| `Finished` | They finished it. |
| `TurnedDown` | They turned it down. |
| `GaveUp` | They gave it up. |

Every line says "the player", never a name. The prompt words are internal, not UI copy, so the UI copy rules of `CLAUDE.md` do not apply. No text that a player reads changes.

### 5.2 Time in words

The clock of the prompt is the tick of the talk (`at` of `talk_asked`). A tick is seconds since the Unix epoch. The age is `now - then`, and 0 when `then` is later (a clock that went back).

| Age | Words |
|---|---|
| under 1 hour | Less than an hour ago |
| 1 hour to under 1 day | Hours ago |
| 1 day to under 2 days | Yesterday |
| 2 to 6 days | Two days ago ... Six days ago |
| 7 to 13 days | A week ago |
| 14 to 29 days | Two weeks ago ... Four weeks ago (days / 7) |
| 30 to 59 days | A month ago |
| 60 to 364 days | Two months ago ... Twelve months ago (days / 30) |
| 365 days and more | Over a year ago |

Why words and not a date: the world is in 25 ADP, and a real date breaks it (house rules). A small model also uses "three weeks ago" better than a date. "Today" is not used, because 1 to 23 hours can cross midnight.

## 6. Reads (5.14)

The talk call reads what it read before, and the rows behind each memory:

| Memory | Rows |
|---|---|
| `Said` | the `learned` row of the rumor |
| `GaveQuest` | every `quests` row of that quest number: the offer and each change |
| `FirstMet` | the event that opened `met` |
| `KilledYou` | the `defeated` event of the NPC, and the `deaths` event |
| `YouDefeatedIt` | your newest `defeated` event of the NPC |
| `DefeatedNear` | your newest `defeated` event of that foe |
| `DiedNear` | the `deaths` event, and the `defeated` event of the killer when there is one |

Most events of `FirstMet`, `KilledYou`, and `YouDefeatedIt` are in `events_about(npc)` already. The memory still names them, so the rule stays simple: a call reads every source of every memory in its prompt. More reads are the safe side.

A rumor and a quest change are rows of `learned` and `quests`, not Hourglass events. Both are stored facts of the world with proof (5.14). A rumor rests on the talk call that wrote it, so a memory of it shows Player proof.

## 7. Limits

- At most `MAX_MEMORIES` = 5 lines.
- A rumor is cut at a whole word past `RUMOR_CHARS` = 160 characters, with "...".
- A quest title has at most 60 characters (`quest::MAX_TITLE_CHARS`). A name has at most 96 bytes (`MAX_NAME_BYTES`).
- Each line has at most `MAX_MEMORY_CHARS` = 280 characters. A unit test builds the longest line of each kind and checks it.
- The block costs at most 5 x 280 characters and about 60 characters of rules: about 365 tokens. The talk prompt of `tests/voice.rs` is 457 tokens today, with a budget of 1823 (`tokens::Call::Talk`). So the longest block leaves the prompt near 820 tokens.

## 8. Edge cases

| Case | Rule |
|---|---|
| No history | No lines. The prompt keeps the rule "Never speak of a past with the player that is not written here." |
| First talk | The talk meets the NPC just before the prompt, so the first meeting is under an hour old and is no memory. |
| NPC met only by sighting | `seen` makes no memory. The talk is the first meeting, so it is no memory either. Deeds near it still count. |
| Quest declined | `TurnedDown`, from its `Declined` row. |
| Offer replaced by a newer offer | `quest_log` marks it `Declined`, but no row says that you declined it. It is no memory: you did nothing. |
| Quest given up before or after accept | `GaveUp`, from its `Abandoned` row. |
| Death caused by this NPC | `KilledYou`, tier `Between`. It never shows again as `DiedNear`, also when it was in the same zone. |
| Death in its zone, by another killer or none | `DiedNear`, tier `Near`. Only the newest one. A killer that was a player is `None` already (5.4), so it names no killer. |
| Rare or boss defeated in its zone | `DefeatedNear`, the newest kill of each foe. A foe that is this NPC is `YouDefeatedIt` instead. |
| A deed with no place, or in another zone | No memory. The zone of a deed is `zone_of_place(place)`, and the zone of the NPC is `zone_of_npc(npc)`. |
| Subzone name in two zones ("The Great Sea") | `zone_of_place` gives one of them. A deed can then miss its zone. It is still a true deed, so the risk is a lost memory, never a false one. |
| A rumor that holds the name of the character | No memory, and the next older rumor does not take its place. The check is `check::mentions(text, own_name)`: whole words, any case. |
| A rumor that holds another player's name | The story program has no alias table yet, so it cannot see it. The rumor is the NPC's words, never the player's words. A name gets there only when the player typed it and the NPC said it back. See open question 1. |
| An NPC with the same name in two zones | The world keeps one person for each name (`Character::find`). Trust, slaps, and now memories belong to the name. A "Stormwind City Guard" remembers what another guard of that name said. "Near it" uses the zone where you talk, because the talk moves the NPC there. See open question 2. |
| Two talks to the same NPC at once | The second prompt opens before the first answer, so it lacks that rumor. |
| Another character on the same computer | Memories come from the world of the active character only. |

## 9. GAMEPLAY.md text

Add to 3.5, after "What the NPC knows":

> - **What the NPC remembers** (built). The prompt holds up to 5 memories of the NPC about you. Each memory is a row of your world, so the model cannot invent a past. The kinds:
>   - its last 2 answers to you in `/talk`;
>   - each side quest that it gave you, with its state: not answered, in progress, finished, given up, or turned down;
>   - your first meeting, when it is an hour old or more;
>   - a fight between you: it killed you, or you defeated it;
>   - near it: a rare or a boss that you defeated, or your death, in its zone.
>
>   What is between you and the NPC comes first, then the first meeting, then the deeds nearby. Inside each group, the newest comes first. The NPC brings up at most one memory in an answer, and only when it fits. Each memory says when, in words: "Three weeks ago", never a date. The clock is the time of the talk.
>
>   An offer that a newer offer replaced is no memory, because you did nothing. A death that the NPC caused counts as a fight, never also as a death nearby. Seeing an NPC is no memory: it never noticed you. The world keeps one person for each name, so two NPCs with one name share their memories, as they share trust.

Change the Talk row of the table in 5.14:

> | Talk | the events behind the NPC, the hero entries about it, the `learned` rows of its passages, and the rows behind each memory (3.5): the `learned` row of each answer, every `quests` row of each quest, and the events of each deed and of the first meeting |

Add to 5.11, after "The card":

> A memory of an NPC (3.5) calls you "the player", never by name. A rumor that holds the name of your character is no memory.

## 10. The hero sheet as a hook

The user approved this addition. The sheet of the hero (3.7) also shapes `/talk` and side-quest offers (3.4), but only sometimes: at most one call in three. When it applies, the prompt gets one answer of the sheet as a hook. The steps of a quest stay real, checked targets. The model never claims a fact about the sheet that the player did not write.

### 10.1 Which answers

Only four fields are hooks: `goal`, `bond`, `flaw`, and `traits`. They say what the hero wants and who the hero is now, so an NPC or a task can use them. `origin` and `background` are the past of the hero. They stay with the narrator, and are never a hook.

### 10.2 When: one call in three, from a count

The choice uses no random number, so a test can pin it.

- **The count** is the number of rows in `calls` of the active character with the kind `talk` or `quest`, before the new call opens. A failed, refused, or open call counts too. So a model that fails again and again never gets a hook more often than one call in three. A task draft (4.7) and every other kind do not count.
- **A hook applies** when `count % HOOK_EVERY == HOOK_EVERY - 1`, with `HOOK_EVERY = 3`: the 3rd, 6th, 9th call, and so on. Talks and quest offers share one count.
- The `calls` table lives in the world database, so the count stays the same after a restart.

### 10.3 Which answer, and how it rotates

- **The filled hook fields** are the hook fields of `hero::hero(changes)` that hold a text, in the order of `hero::FIELDS`: goal, bond, flaw, traits.
- **The pick** is `filled[(count / HOOK_EVERY) % filled.len()]`. Each hook call takes the next filled field, and the list starts again after the last one.
- **The text** is `hero::cut(text)`: its first `PROMPT_TEXT_CHARS` (300) characters, as for the narrator.

### 10.4 Types and functions

A new module `crates/story/src/hero_hook.rs`, with no store code:

```rust
//! One answer of the hero sheet as a hook for a talk or a task, one call in three
//! (GAMEPLAY.md 3.7).

/// A hook applies to one call in this many.
pub const HOOK_EVERY: u64 = 3;

/// The fields that say what the hero wants and is now. The past stays with the narrator.
pub const HOOK_FIELDS: [&str; 4] = ["goal", "bond", "flaw", "traits"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hook<'a> {
    pub field: &'a str,
    pub text: &'a str,
}
```

| Function | Job |
|---|---|
| `pub fn applies(count: u64) -> bool` | True for the calls of 10.2. |
| `pub fn hook(hero: &Hero, count: u64) -> Option<Hook<'_>>` | `None` when `applies` is false or no hook field holds a text. Otherwise the pick of 10.3. |
| `fn filled(hero: &Hero) -> Vec<&Field>` | The hook fields that hold a text, in the order of `FIELDS`. |
| `pub fn hook_words(field: &str) -> &'static str` | The words of a field in the prompt: "their goal", "a bond of theirs", "a flaw of theirs", "their traits". |

Changes in other files:

| File | Change |
|---|---|
| `store/database.rs` | `pub fn count_calls(&self, kinds: &[&str]) -> Result<u64, StoreError>`: the rows of `calls` with one of these kinds. |
| `story/reads.rs` | `pub(super) fn hook_read(active: &Active, field: &str) -> Vec<Node>`: the `hero` row of the newest `Change::Set` of that field. |
| `talk.rs` | `Scene` gets `pub hook: Option<Hook<'a>>`. `what_you_know` adds the talk block of 10.5 after the player's own lore and before the memories. |
| `quest.rs` | `quest::prompt` takes `hook: Option<Hook<'_>>`, and adds the quest block of 10.5 after the three lists and before "Rules:". |
| `story.rs`, `fn talk` | Counts the calls, builds the hook from the hero that it builds already, and extends `read` with `reads::hook_read`. |
| `story/quests.rs`, `fn quest_call` | The same. |

The count, the hook, and the read go together in one helper of `story/calls.rs`: `fn hook_for(active: &Active, hero: &Hero) -> Result<(Option<Hook>, Vec<Node>), StoreError>`. Both calls use it.

### 10.5 The prompt

The text goes in a fence, as all player text does. The label says whose words they are, and the rule says what the model can do with them.

In a talk:

```
Something the player wrote about their hero, as {hook_words}. It is their story, not canon:
<<<
{text}
>>>
Let it shape your answer only when it fits what the player says. Never claim more about it than these words say.
```

In a quest offer:

```
Something the player wrote about their hero, as {hook_words}. It is their story, not canon:
<<<
{text}
>>>
Let it shape the reason for the task, if it fits. The steps still use only the lists above. Never claim more about it than these words say.
```

With no hook, neither block shows, and the prompt is the same as today.

### 10.6 The quest check

The check of an offer (`quest::checked_quest`) does not change. Each step still names a target of the lists, and the hook adds no target. A name that the hook holds and that the lists do not hold fails the step check. The title and the text still get the cutoff check of `plain_text`: a later name that the hook holds and the model copies refuses the offer, and the line says that the NPC has no task for you now, as today.

A talk answer is different: its check (`voice_text`) already allows a later name that the sheet holds (3.7).

### 10.7 Reads

| Call | New reads |
|---|---|
| Talk, with a hook | the `hero` row of the newest `Set` of the hook field |
| Quest offer, with a hook | the same |

A call with no hook reads no hero row for it. The talk call still reads the hero entries about the NPC, as today.

### 10.8 Limits

The block adds at most 300 characters of text and about 200 characters of words: about 125 tokens. The quest offer prompt is 406 tokens of 1733 today. The talk prompt stays under about 950 of 1823, with 5 memories and a hook.

### 10.9 Edge cases

| Case | Rule |
|---|---|
| An empty sheet, or one with only `origin` and `background` | No hook. The call still counts, so the rhythm stays the same when the player fills the sheet. |
| An answer that the player just changed | The hook uses the sheet as it stands when the call opens. So the new text goes out, and the read names the row of the new text. |
| An answer that the player cleared | An empty text clears the field (3.7). It is not filled, so the rotation skips it. |
| A field filled or cleared between two hooks | `filled` changes its length, so the pick moves. Each pick is still a filled field. A field can then come twice in a row. That is fine: the rule is "sometimes", not a fair share. |
| A hook that holds a real player name | The words go as the player wrote them, as `/talk` words do (5.11): it is the player's own choice. |
| A hook call that the model fails | The call counts, so the next hook comes 3 calls later. |
| Another character | The count and the sheet belong to the world of the active character. |

### 10.10 GAMEPLAY.md text

Add to 3.7, under "Who reads it":

> - **A hook, now and then.** One talk (3.5) or side-quest offer (3.4) in three gets one answer of your sheet as a hook: your goal, a bond, a flaw, or your traits. The NPC uses it only when it fits. Your origin and background stay with the narrator. The calls of a character count together, talks and offers, and the third, sixth, and ninth call each get a hook. Each hook takes the next answer that you filled, in the order of the sheet, and starts again after the last one. A failed call counts too. The hook is your own words, fenced as data, with the rule that the model claims nothing more about it. A quest still names only real targets that the check allows.

In 5.14, add to the Talk row and to the Quest offer row:

> and the hero row of the hook, when the call has one (3.7)

### 10.11 Tests

In `crates/story/tests/hero_hook.rs` (new):

- `the_third_call_gets_a_hook_and_the_two_before_it_do_not`
- `a_hook_comes_once_in_every_three_calls`
- `the_hook_rotates_through_the_filled_answers_in_the_order_of_the_sheet`
- `an_empty_sheet_gives_no_hook`
- `origin_and_background_are_never_a_hook`
- `a_cleared_answer_is_never_a_hook`
- `a_hook_takes_the_first_three_hundred_characters_of_its_answer`

In `crates/story/tests/talk.rs`:

- `a_talk_prompt_with_a_hook_fences_it_with_its_rule`
- `a_talk_prompt_with_no_hook_is_as_before`

In `crates/story/tests/quest.rs`:

- `a_quest_prompt_with_a_hook_keeps_its_lists_and_its_rules`
- `a_step_that_names_only_the_hook_fails_the_check`

In `crates/story/tests/story.rs`:

- `talks_and_quest_offers_share_one_count_of_calls`
- `a_failed_call_still_counts_toward_the_next_hook`
- `a_changed_answer_goes_out_with_its_new_text`
- `a_hook_reads_the_hero_row_that_wrote_its_text`
- `the_count_of_calls_survives_a_restart`
- `a_task_draft_does_not_count_toward_a_hook`

In `crates/story/tests/voice.rs`: `npc_talk()` and `quest_offer()` get a hook of 300 characters, so the budget test covers it.

In `crates/story/tests/properties.rs`:

- `at_most_one_talk_or_quest_call_in_three_has_a_hook`. Any play, then the prompts of the `talk` and `quest` calls in order: a hook block shows only at positions 3, 6, 9, and so on.
- `every_hook_is_the_words_of_the_player_and_its_row_is_read`. The fenced text of each hook block starts a `Set` row of a hook field in the `hero` rows that the call read, and that row is the newest `Set` of its field before the call.

The plays of `play()` already set fields (`Play::HeroSet`). The new properties make a hook field likely: `memory_play()` (12.5) also sets `goal` or `flaw` one time in four.

Fuzz: no new target. In `fuzz/fuzz_targets/play.rs`, `assert_memory_block` also checks that a hook block holds at most `PROMPT_TEXT_CHARS` characters in one fence.

## 11. Build order

Each step is one commit, with its tests.

1. `learned::cut_at_word`, `LearnedLog::rumors_with_rows`, `CharacterKey::name`, `Character::first_met`, and `journal::deeds_with_events`. No behavior changes. The journal tests stay green.
2. `npc_memory.rs`: the types, the five gather functions, `chosen`, `line`, and `when`, with their unit tests.
3. `talk.rs`: `Scene::memories` and `what_you_remember`. `tests/talk.rs` and `tests/voice.rs` change.
4. `fn talk` and `reads::memories_read`. The story tests and the property tests.
5. The GAMEPLAY.md text of section 9. This plan marks the steps as done.
6. The hook (section 10): `Database::count_calls`, `hero_hook.rs`, the two prompt blocks, `reads::hook_read`, and their tests. Then the GAMEPLAY.md text of 10.10.

No line of the relay protocol changes, so the Gnomish Relay session needs no notice.

## 12. Tests

### 12.1 `crates/story/tests/npc_memory.rs` (new)

- `the_npc_remembers_its_last_two_answers_to_you`
- `an_answer_of_another_npc_is_no_memory`
- `a_rumor_that_names_your_character_is_no_memory`
- `a_long_rumor_is_cut_at_a_whole_word`
- `a_quest_from_the_npc_shows_its_state` (one case for each ending)
- `a_declined_quest_is_remembered_as_turned_down`
- `an_offer_that_a_newer_offer_replaced_is_no_memory`
- `a_quest_of_another_giver_is_no_memory`
- `the_first_meeting_is_a_memory_only_after_an_hour`
- `an_npc_that_you_only_saw_has_no_first_meeting`
- `a_death_that_the_npc_caused_is_a_fight_and_not_a_death_nearby`
- `a_rare_that_you_defeated_in_its_zone_is_a_memory`
- `a_deed_in_another_zone_or_with_no_place_is_no_memory`
- `only_the_newest_death_nearby_is_a_memory`
- `the_npc_keeps_five_memories_and_drops_the_deeds_nearby_first`
- `the_memories_come_newest_first`
- `an_npc_with_no_past_has_no_memories`
- `each_memory_names_the_rows_behind_it`
- `a_memory_says_when_in_words_at_each_edge` (ages 3599, 3600, 86399, 86400, 172799, 172800, 604799, 604800, 1209600, 2591999, 2592000, 5183999, 5184000, 31535999, and 31536000 seconds)
- `a_memory_from_a_later_tick_reads_as_less_than_an_hour_ago`
- `the_longest_line_of_each_kind_fits_its_limit`

### 12.2 `crates/story/tests/talk.rs`

- `a_prompt_lists_the_memories_in_one_fence_with_the_rule_of_one`
- `a_prompt_with_no_memories_still_forbids_a_made_up_past`
- `a_rumor_cannot_close_the_fence_of_the_memories`

`farley()` gets `memories: Vec::new()`.

### 12.3 `crates/story/tests/story.rs`

- `a_second_talk_remembers_what_the_npc_said_in_the_first`
- `a_talk_remembers_the_quest_that_the_npc_gave`
- `a_talk_reads_the_rows_behind_its_memories`
- `another_character_shares_no_memories_with_the_first`
- `an_npc_of_the_same_name_in_another_zone_shares_the_memories_of_the_name`

### 12.4 `crates/story/tests/voice.rs`

`npc_talk()` gets 5 memories at full length: two rumors of `RUMOR_CHARS`, a quest with a title of 60 characters, and two deeds with names of 96 bytes. The test `every_prompt_fits_a_local_model_with_a_context_of_2048_tokens` then covers the budget.

### 12.5 `crates/story/tests/properties.rs`

A new strategy `memory_play()`: `play()`, and also:

- `Play::Wait` of exactly 1 hour, 1 day, 2 days, 7 days, 30 days, and 365 days, so each edge of `when` comes often;
- a talk to "Innkeeper Farley" whose words hold "Ada" (the character) one time in four;
- a quest of "Innkeeper Farley", then `Accept` or `Decline`.

Properties:

- `every_memory_in_a_talk_prompt_rests_on_a_row_that_the_call_read`. For each talk call in `calls`, take the NPC from the `Name:` line and the lines of the memory fence. Each line matches a row in the `reads` of that call:
  - "you told the player" matches a `learned` rumor of that NPC, and the quoted text starts its text (without "...");
  - "your quest" matches a `quests` offer of that NPC with that title;
  - "for the first time" matches an event that opens `met` for that NPC;
  - "in a fight", "defeated", and "died" match an event of `defeated` or `deaths`.
- `a_talk_prompt_holds_at_most_five_memories_and_never_the_name_of_the_character`.
- `the_age_of_a_memory_is_always_words` (pure: any two ticks, with 0, `u64::MAX`, and each edge of 5.2 likely). The words hold no digit and are one of the phrases of 5.2.

### 12.6 Fuzz

No new target. The memories read only rows that the story program wrote and checked, and `fuzz/fuzz_targets/store.rs` already fuzzes damaged `learned` and `quests` rows at open.

One change: in `fuzz/fuzz_targets/play.rs`, the model checks each talk prompt before it answers, with `assert_memory_block(prompt)`: at most 5 lines, each line within `MAX_MEMORY_CHARS`, one fence, and no "Ada" or "Bea" as a word. The `pages`, `replies`, and `task_play` targets then cover it, because their talk answers take any text now and then (`Prose`). A bug that a fuzzer finds becomes a seed in `fuzz/seeds/pages/`.

## 13. Open questions

1. **Names of other players in a rumor.** The story program cannot see them before the alias table (5.11). The choice today: accept the gap, as 5.11 accepts typed words. A stricter choice: drop a rumor that holds a capital word that the words of its talk held and that the world does not know. That needs the `inputs` row of the talk. Which one?
2. **NPCs that share a name.** Guards and vendors share names. Memories follow trust, so they key on the name. A key on the NPC id of the GUID needs a new field in `talk_asked`. Is that worth it later?
3. **More than one memory.** The rule allows one memory per answer. Measure with the voice suite whether a small model keeps it.
