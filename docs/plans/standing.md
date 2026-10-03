# Plan: standing and choices

Status: draft 1, 2026-10-03. The user approved the design. Nothing is built. When a part is built, its rules move into `GAMEPLAY.md` (section 16 of this plan), and this plan marks the part as done.

This plan builds on three plans and does not repeat them:

- `docs/plans/links.md` owns rows, calls, reads, and proof (5.14).
- `docs/plans/npc-memory.md` owns the memory block of a talk, and the hero hook.
- `docs/plans/talk-window.md` owns the conversation, the options, and the daily cap of talk trust.

## 1. Goal

Each town has its own view of the player: its **standing**. Standing comes only from facts of the world. Normal play is neutral: killing mobs, grinding, faction, and race never move it. The player never sees a number, only words: "Goldshire is wary of you."

Standing changes through what the player does to people:

- the trust of the NPCs of the town;
- slaps, and kind and rude emotes at friendly NPCs there;
- side quests finished there, and side quests abandoned after accept (a broken promise);
- the side that the player takes in a **choice quest** there;
- a little, the tone of a reply that the player picks in the talk window.

A choice quest comes from a real conflict between two NPCs that the player knows. It ends at one of two NPCs, and the first one that the player meets decides. The world keeps the conflict as a `rivals` fact, so feuds grow.

The effects are flavor only (rule 2): how NPCs greet you, which NPCs give you quests, titles, the facts of the saga, and the talk of the town. No gold, item, or power ever comes from Timeways. Standing can always recover.

## 2. What changes, in short

| Part | Change |
|---|---|
| `vocabulary.rs` | 5 new facts: `rivals`, `sided_with`, `sided_against`, `spoke_kindly`, `spoke_cruelly`. `VERSION` goes from 7 to 8. |
| `tone.rs` (new) | `Tone`: kind, cunning, or cruel. Its weights. |
| `standing.rs` (new) | The town of an NPC, the standing of a town, its step, its words, and the sum for the journal look. Pure functions. |
| `choice.rs` (new) | The second NPC of a choice, the reward check, and the cruelty check. Pure functions. |
| `quest.rs` | `Known::rival`, `Choice` in the answer and in `QuestChange::Offered`, the new change `Sided`, the prompt block, and the checks. |
| `story/quests.rs` | The pick of the rival, the lawful refusal, the end of a choice, and the reads. |
| `character.rs` | `side_with`, `town_of`, `rivals_of`, `speak_with_tone`, and the facts behind each source. |
| `talk.rs`, `conversation.rs` | A tone for each option. The standing line and the talk of the town in the prompt. |
| `titles.rs`, `moments.rs`, `narrator.rs`, `chronicle.rs`, `journal.rs` | The effects (section 9). |
| `story/reads.rs`, `story/why.rs` | `standing_read`, and the cause "choice" for trust. |
| Data | `kind_emotes.txt`, `rude_emotes.txt`, `reward_words.txt`, `cruel_words.txt`, `lawful_titles.txt` in `crates/story/data/`. |
| Addon | The choice on the Quests page, a chat line when a town changes step, a line on the map pane, and two new "Why?" lines. |

No new table. No line of the relay protocol changes: the journal page and the talk answer are free JSON for the bridge, and the tone of an option stays on the desktop. So the Gnomish Relay session needs no notice, with one exception in section 8.3.

## 3. Town: a subzone, with two exceptions

**Decision.** The town of an NPC is the place where the NPC lives in your world (`located_in`), with two exceptions:

1. An NPC in a capital city, or in a subzone of one, belongs to the capital. The Trade District is part of Stormwind City.
2. An NPC in a dungeon or a raid belongs to no town. An instance has no townsfolk.

An NPC that lives in a zone with no subzone belongs to the zone. So a guard on the road of Elwynn Forest belongs to "Elwynn Forest".

**Why a subzone.** The code already keeps the subzone of each NPC (`Character::place_of`): you meet an NPC where you stand, and you mostly stand in a subzone (3.6). Goldshire and Northshire Abbey are both in Elwynn Forest, and they do not share a view of you. The user's own example is Goldshire. A capital is one town, because its districts are one city.

**The trap.** An NPC lives where you met or fought it last. So an NPC that walks between two subzones moves its trust to the other town. Standing is computed each time and never stored, so it follows. A choice quest keeps the town of its offer (5.3), so a choice never moves.

**A town with an odd name.** A subzone such as "Brackwell Pumpkin Patch" is a town too. A neutral town shows nothing, so such a name shows only after you did something there. Open question 1 asks the user to confirm.

```rust
/// The town of an NPC (docs/plans/standing.md 3): its subzone, its capital, or its zone.
/// None in an instance, or for an NPC that the world does not hold.
pub fn town_of(&self, npc: &str) -> Option<&str>
```

`town_of` lives in `character.rs`, next to `place_of` and `zone_of_npc`. It uses `places::is_capital` and the `dungeon` and `raid` flags.

## 4. The data model

### 4.1 New facts in `vocabulary.rs`

`VERSION` goes from 7 to 8. Nothing is live, so the test worlds start again (links.md, top).

```rust
/// A conflict between two NPCs that a choice quest settled. The giver holds it, linked to
/// the other NPC. It counts the choices between them, so a feud grows.
pub const RIVALS: &str = "rivals";
/// The choices that you settled for an NPC.
pub const SIDED_WITH: &str = "sided_with";
/// The choices that you settled against an NPC.
pub const SIDED_AGAINST: &str = "sided_against";
/// The kind replies that you picked in a talk with an NPC.
pub const SPOKE_KINDLY: &str = "spoke_kindly";
/// The cruel replies that you picked in a talk with an NPC.
pub const SPOKE_CRUELLY: &str = "spoke_cruelly";
```

| Fact | Shape | Direction | Links | Meaning |
|---|---|---|---|---|
| `rivals` | number, 0 to 1000 | up | person to person | Choice quests between the giver and the other NPC that you settled. |
| `sided_with` | number, 0 to 1000 | up | person to person | Choices that you settled for this NPC. |
| `sided_against` | number, 0 to 1000 | up | person to person | Choices that you settled against this NPC. |
| `spoke_kindly` | number, 0 to 1000 | up | person to person | Kind replies that you picked in a talk with this NPC. |
| `spoke_cruelly` | number, 0 to 1000 | up | person to person | Cruel replies that you picked in a talk with this NPC. |

All five are tallies, as `slapped` is: `linked(up_tally(), Person, &[Person])`. Each count stops at 1000.

**No fact for emotes.** An emote stays a flavor row (5.4.1). GAMEPLAY.md says that an emote adds no event to the world, and the sessions and chapters rely on it (3.3). So standing reads the emote rows of the `flavor` table (6.2).

**No fact for cunning words.** A sly reply harms nobody, so it does not move standing (6.1). With no weight, it needs no fact.

**No fact for standing.** Standing is a pure function of the world and the logs. A stored value can disagree with the history (links.md 3).

### 4.2 `tone.rs` (new)

```rust
//! The tone of a choice or of a reply (docs/plans/standing.md 4.2). The model gives it,
//! and the code weighs it.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Kind,
    Cunning,
    Cruel,
}
```

One word for one concept: the model, the code, and this plan all say "kind", "cunning", and "cruel". The player never sees a tone.

### 4.3 A choice in `quest.rs`

```rust
/// The two ends of a choice quest (docs/plans/standing.md 5). The player ends the quest
/// with the giver or with the rival, and the first one met decides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub rival: String,
    /// The town of the giver when the offer came. The choice counts there for good.
    pub town: String,
    pub giver_side: Side,
    pub rival_side: Side,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Side {
    /// What the player does on this side, in a few words: "Give the ledger back."
    pub text: String,
    pub tone: Tone,
}

/// Which NPC the player ended a choice with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Party {
    Giver,
    Rival,
}
```

`QuestChange` changes:

```rust
Offered {
    number: u64,
    at: Tick,
    giver: String,
    title: String,
    text: String,
    steps: Vec<Step>,
    /// The town of the giver at the offer. None in an old quest file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    town: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    choice: Option<Choice>,
},
// ...the other changes stay...
/// The player ended a choice quest with one side. It finishes the quest.
Sided {
    number: u64,
    with: Party,
    at: Tick,
},
```

`Tracked` gets `town: Option<String>`, `choice: Option<Choice>`, `sided_with: Option<Party>`, and `accepted: bool`. `accepted` is true once an `Accepted` row of the quest exists, so an abandoned quest says whether it broke a promise.

The rules of `quest_log`:

- `steps` holds only the plain steps. The choice is the last goal, after them.
- A `StepDone` of the last plain step finishes a quest with no choice, as today. A quest with a choice stays `Accepted`, and waits for its choice.
- `Sided` counts only when the quest is `Accepted`, every plain step is done, and `sided_with` is `None`. It sets `sided_with`, `status = Done`, and `done_at`.
- A second `Sided` of the same quest changes nothing. So a choice quest ends at most once.

`Tracked` gets one function:

```rust
/// True when the next goal is the choice: every plain step is done.
pub fn awaits_choice(&self) -> bool
```

### 4.4 The answer of the model

A plain quest stays as today. A choice quest adds `choice`:

```json
{
  "title": "The Missing Ledger",
  "text": "Someone took my ledger. Marshal Dughan says the law wants it first. I say it's mine. Find it at the pumpkin patch, then decide who gets it.",
  "steps": [{"goal": "visit", "place": "Brackwell Pumpkin Patch"}],
  "choice": {
    "rival": "Marshal Dughan",
    "giver_side": {"text": "Give the ledger back to Farley.", "tone": "kind"},
    "rival_side": {"text": "Hand the ledger to the Marshal.", "tone": "cunning"}
  }
}
```

The code adds `town` from the world. The model never writes it.

### 4.5 The checks of `checked_quest`

New faults in `QuestFault`:

| Fault | Rule |
|---|---|
| `NoRival` | The answer has a `choice`, but the prompt offered no rival. |
| `WrongRival(String)` | `choice.rival` is not the rival of the prompt, byte for byte. |
| `RivalInStep(String)` | A plain step names the rival. The rival comes only at the end. |
| `ChoiceSteps(usize)` | A choice quest has 1 or 2 plain steps, not this many. |
| `BadSide` | A side text is empty, longer than 80 characters or 320 bytes, has a control character or a `\|`, or names something after the cutoff. |
| `SameSides` | The two side texts are the same words (`same_words`). |
| `Reward(String)` | The title, the text, or a side text promises a reward of the game. |
| `Cruelty(String)` | In a choice quest, the title, the text, or a side text has a word of the cruelty list. |

**Why 1 or 2 plain steps.** The addon forgets the NPCs that you met when you accept (3.4), and the giver stands next to you then. With no plain step, the next gossip window of the giver ends the choice by accident. The total of 3 goals stays (`MAX_STEPS`).

**The tone** is part of the JSON. A tone that is not `kind`, `cunning`, or `cruel` fails to parse, so the offer gets `NotJson`.

**The offer line** stays `offer_line`, and the `MAX_OFFER_BYTES` check stays. The text of the quest says what the choice is about.

### 4.6 `choice.rs` (new): the reward and cruelty checks

```rust
//! The checks of a choice quest that no other quest needs, and the reward check that every
//! quest needs (docs/plans/standing.md 4.6 and 5).

/// The first word of the reward list in `text`, after the names of `names` are taken out.
pub fn reward_word(text: &str, names: &[&str]) -> Option<&'static str>

/// The first word of the cruelty list in `text`, after the names of `names` are taken out.
pub fn cruel_word(text: &str, names: &[&str]) -> Option<&'static str>
```

Both use `check::data_lines` and the whole-word match of `banned_words_in` (any case, whole words only).

**The names go out first.** "Gold Coast Quarry" is a subzone of Westfall, and "Goldtooth" is an NPC. So the check takes out each name of the prompt (the giver, the rival, and every place, NPC, and creature of the lists) before it looks for a word. A name of the game never trips the check.

`crates/story/data/reward_words.txt`, a first guard:

```
# Words that promise a reward of the game (docs/plans/standing.md 4.6). Rewards of
# Timeways are story only (GAMEPLAY.md 2, rule 2). Whole words, any case. Write each
# plural form on its own line. Names of the prompt are taken out before the check.
gold
silver
copper
coin
coins
money
pay
paid
payment
reward
rewards
item
items
loot
gear
weapon
weapons
armor
experience
xp
```

`crates/story/data/cruel_words.txt`, a first guard, for choice quests only:

```
# Words that a choice quest never holds (docs/plans/standing.md 5.5). No choice harms a
# child, a helpless or innocent person, or uses torture. Betrayal, greed, theft, lies,
# and mercy against justice are fine. Whole words, any case.
child
children
kid
kids
orphan
orphans
baby
babies
infant
infants
innocent
innocents
helpless
defenseless
torture
tortured
torturing
torment
slave
slaves
slavery
massacre
```

Why only choice quests: a plain quest to find a lost child is kind, and nobody chooses to harm anyone. A refused offer costs only one offer: "Farley has no quest for you now." So a list that refuses too much is the safe side.

## 5. Choice quests

### 5.1 The second NPC

When a `/quest` call opens, the story program looks for one rival for the giver. A pure function of `choice.rs` decides:

```rust
/// The NPC that a choice of `giver` can name, or None. `open` are the NPCs of the choices
/// that wait for an end.
pub fn rival_for<'a>(character: &'a Character, giver: &str, known: &Known<'a>, open: &[&str]) -> Option<&'a str>
```

A candidate:

- is in the same town as the giver (`town_of`);
- has `met` from you: seeing is not meeting, and a stranger has no feud that you know of;
- is not the giver, not hostile, not an animal, and not dead in your story;
- passes the meet check of 3.4 (`known.people()`): not in a quest of the game that you read, and not a target of your newest quest;
- is not in a choice that waits for its end (`open`), so one NPC is never in two open choices.

The order:

1. An NPC that is already a rival of the giver, in either direction (`rivals_of`). A feud grows.
2. Then the newest meeting first, as for the lists of the prompt.

The first candidate is the rival. With none, the prompt is the prompt of today. There is no quota: every offer with a candidate asks the model, and the model decides.

### 5.2 The prompt

`quest::prompt` gets `rival: Option<RivalCard>`, and a standing line (9.2). The block goes after the three lists and before "Rules:". It is internal text, not UI copy.

```rust
pub struct RivalCard<'a> {
    pub name: &'a str,
    pub town: &'a str,
    /// "They like the player." The trust band of the rival, in words.
    pub feeling: &'static str,
    /// The choices between the giver and this NPC so far.
    pub earlier_choices: i64,
    /// Whom the player sided with in the newest of them.
    pub last_sided_with: Option<&'a str>,
}
```

With a rival:

```
Another person of {town} that the player knows:
<<<
Name: {name}
{feeling}
{history}
>>>
If you and this person could both want the same thing, such as one object, one place, or one favor, make the task a choice: the player ends it with you or with them. If not, write a normal task and leave out "choice".
A choice has 1 or 2 steps before it. Each side is one short sentence of what the player does, at most 80 characters. Give each side its tone: "kind", "cunning", or "cruel".
No side harms a child, a helpless or innocent person, or uses torture. Betrayal, greed, theft, lies, and mercy against justice are fine.
```

`{history}` is one of:

- "You two have no quarrel that the player knows of." (no `rivals` fact)
- "You two are rivals. The player settled 2 quarrels between you, and last sided with {name}."

Every prompt, with a rival or not, gets one more rule under "Rules:":

```
- Promise no gold, item, or other reward of the game. The reward is the story: your trust, and a place in the player's chronicle.
```

The JSON line of a prompt with a rival:

```
Reply with JSON only: {"title": "...", "text": "...", "steps": [...], "choice": {"rival": "<the name above>", "giver_side": {"text": "...", "tone": "..."}, "rival_side": {"text": "...", "tone": "..."}}}. Leave out "choice" for a normal task.
```

**Size.** The quest offer prompt is 406 tokens of 1733 today (npc-memory.md 10.8). The rival block, the reward rule, and the standing line add about 230 tokens. The hook of npc-memory.md adds 125. The total stays under 800. `tests/voice.rs` gets a `quest_offer_with_a_rival()` at full length, and the budget test covers it.

### 5.3 The offer

`fn offer` in `story/quests.rs` checks the answer with `known.rival` set. It stores `town` and `choice` in `Offered`. The `quest_offered` fact lands as today. No `rivals` fact lands yet: an offer is the word of the model, and only the end of the choice is a deed.

### 5.4 The end of a choice

`advance_quests` takes an `Encounter` in place of `Option<&str>`:

```rust
pub(super) enum Encounter<'a> {
    None,
    /// A talk: a gossip window of the game (`npc_met`) or `/talk` (`talk_asked`).
    Talk(&'a str),
    /// A slap meets the NPC for a plain step, but never decides a choice.
    Slap(&'a str),
}
```

After the plain steps move, a new function ends the choices:

```rust
/// Each accepted quest that waits for its choice ends with the first of its two NPCs
/// that you talk to.
pub(super) fn settle_choices(&mut self, at: Tick, npc: &str) -> Result<Vec<Output>, StoryError>
```

For each quest that `awaits_choice`, when `npc` is the giver or the rival:

1. `QuestChange::Sided { number, with, at }` goes into the quest log.
2. In the world, in one `change`, by `Character::side_with(at, quest, chosen, passed_over)`:
   - `quest_done` on you, as today;
   - `sided_with` +1 on you, linked to the chosen NPC;
   - `sided_against` +1 on you, linked to the other NPC;
   - `rivals` +1 on the giver, linked to the rival;
   - the trust of the chosen NPC goes up by `QUEST_TRUST` (10), and the trust of the other goes down by `CHOICE_TRUST` (10).

The chosen NPC gets the same trust as a finished quest. The giver who is chosen gets 10, not 20. Why 10 down: a broken loyalty costs as much as one slap.

Rules:

- A slap never decides. A slap of the rival is no answer to a quarrel.
- `quest_asked` meets an NPC but moves no step today, and it decides no choice either.
- Both NPCs in one encounter is not possible: one encounter names one NPC.
- When the rival is dead or hostile later, the player can still end the quest with the giver. When the giver is, with the rival. When both are, the quest waits until you abandon it.
- A choice ends the quest. The quest counts as finished on the Deeds page and in the chronicle (9.5).

### 5.5 No cruelty to innocents

Three guards, from the cheapest:

1. **The prompt rule** of 5.2.
2. **The cruelty list** of 4.6, on the title, the text, and both sides of a choice quest.
3. **The steps.** A side ends only with a talk to a real NPC that is not hostile, not an animal, and alive. A kill step targets only creatures that you saw hostile (3.4). So no step of a choice asks you to kill a friendly NPC.

The list is a first guard. Later, a judge call can read the sides, as the judge of a saga does (3.3). Open question 4.

## 6. Standing

### 6.1 The weights

Standing is the sum of six parts. Each part has a cap, so no one source decides a town.

| Part | Each source | Cap of the part | Why |
|---|---|---|---|
| **People** | Each NPC of the town by its trust band (3.5): trusts you +2, likes you +1, neutral 0, wary of you -1, distrusts you -2. | -2 to +2 | How the people that you know feel. Trust already holds talks, slaps, and quests, so the part stays small: the deeds have their own parts. |
| **Slaps** | -1 for each slap of an NPC of the town | -4 to 0 | A slap is public. The town saw it, apart from the trust of the NPC. |
| **Manners** | +1 for each kind emote, -1 for each rude emote, at a friendly NPC of the town (6.2) | -2 to +2 | A gesture is small. Spam reaches the cap fast. |
| **Words** | +1 for each kind reply that you picked, -1 for each cruel one, in a talk with an NPC of the town. A cunning reply counts 0. | -1 to +1 | Talk alone never decides standing (6.3). |
| **Promises** | +2 for each side quest of a giver of the town that you finished, -2 for each one that you abandoned after you accepted it. A choice quest counts under Choices, not here. | -6 to +6 | A finished quest is a deed. A quest abandoned after accept is a broken promise. A declined offer counts 0: saying no is fine. |
| **Choices** | For each choice quest of the town that you ended, the tone of your side: kind +3, cunning -1, cruel -3 | -9 to +9 | The strongest source, because it is a real moral choice. A cunning side is a theft or a lie that worked, so the town is a little wary. |

The standing of a town is the sum, clamped to the band of -20 to 20 (`STANDING`). The sum of the caps is -24 to +20, so only the low end clamps.

**Which NPCs count.** An NPC of the town that is not hostile and not an animal. A dead NPC of your story still counts for its slaps, but not for People: the dead have no view.

**Which quests count.** A quest counts in the town of its offer (`Tracked::town`). An old quest with no town counts in the town of its giver now.

**The constants**, in `standing.rs`:

```rust
pub const STANDING: Band = Band { min: -20, max: 20 };
const PEOPLE_CAP: i64 = 2;
const SLAPS_CAP: i64 = 4;
const MANNERS_CAP: i64 = 2;
const WORDS_CAP: i64 = 1;
const PROMISE: i64 = 2;
const PROMISES_CAP: i64 = 6;
const CHOICES_CAP: i64 = 9;
```

`Tone` gives its weight for a choice:

```rust
impl Tone {
    /// The weight of a side that the player chose (docs/plans/standing.md 6.1).
    pub fn choice_weight(self) -> i64 { match self { Tone::Kind => 3, Tone::Cunning => -1, Tone::Cruel => -3 } }
}
```

### 6.2 Kind and rude emotes

The addon sends every emote as `emote_done` with its token in lower case and its NPC target (5.4.1, `Emotes.lua`). A target counts only when it is the current target and an NPC, never a player. A `/slap` also sends `npc_slapped`.

An emote row counts for Manners when:

- its token is on `kind_emotes.txt` or `rude_emotes.txt`;
- it has a target, and the target is an NPC of the town, not hostile and not an animal now.

`crates/story/data/kind_emotes.txt`:

```
# Emotes that are kind to an NPC (docs/plans/standing.md 6.2). The tokens of the game, in
# lower case, as Emotes.lua sends them. Check each token with the API gate before a release.
applaud
bow
cheer
comfort
congratulate
greet
hug
praise
salute
soothe
thank
wave
welcome
```

`crates/story/data/rude_emotes.txt`:

```
# Emotes that are rude to an NPC (docs/plans/standing.md 6.2). `slap` is not here: a slap
# has its own part and its own fact.
belch
chicken
fart
insult
mock
moon
rasp
rude
shoo
spit
taunt
threaten
```

Every other emote is neutral: `/dance`, `/kiss`, `/laugh`, and `/point` are silly, not kind or rude. `/kiss` at a guard stays a joke (5.4.1).

The tokens in the files are the English tokens of the classic emote list. A test in the game confirms each one (open question 3).

### 6.3 The cap of talk

Two parts come from talk: the trust of People, and Words.

- talk-window.md caps the talk trust of one NPC at 10 up and 10 down in 24 hours. 10 trust is one band, so talk moves an NPC at most one band a day.
- People is capped at ±2, and Words at ±1.

So talk alone moves a town at most 3 from neutral. The neutral step is -3 to 3 (6.4). **Talk alone never takes a town off neutral.** A property test proves it (12.2).

### 6.4 Steps and words

| Standing | Step | Chat line on a change | Map line |
|---|---|---|---|
| 10 to 20 | `Trusts` | Goldshire now trusts you. | Goldshire trusts you. |
| 4 to 9 | `Likes` | Goldshire now likes you. | Goldshire likes you. |
| -3 to 3 | `Neutral` | Goldshire feels neutral about you now. | (nothing) |
| -9 to -4 | `Wary` | Goldshire is now wary of you. | Goldshire is wary of you. |
| -20 to -10 | `Distrusts` | Goldshire now distrusts you. | Goldshire distrusts you. |

The words are the words of NPC trust (3.5), with the town in place of the NPC. One name for each feeling, everywhere.

In a prompt, the model gets a line in the third person, never a number:

| Step | Prompt line |
|---|---|
| `Trusts` | People in Goldshire trust the player. |
| `Likes` | People in Goldshire like the player. |
| `Neutral` | (no line) |
| `Wary` | People in Goldshire are wary of the player. |
| `Distrusts` | People in Goldshire distrust the player. |

### 6.5 Recovery

Every part can come back:

- People: trust rises with talk and finished quests.
- Manners and Words: a kind act offsets a rude one inside the part.
- Promises: a finished quest offsets a broken one.
- Choices: a kind side offsets a cruel one.
- Slaps: a slap stays for good (`slapped` only rises), but the part stops at -4. Promises and Choices together reach +15.

So from any standing, enough kind deeds bring a town back to neutral or better. Nothing fades with time: a deed counts as long as the town remembers it, and only later deeds balance it. That keeps the rule exact and testable. A property test proves recovery (12.2).

### 6.6 The functions of `standing.rs`

```rust
//! What each town thinks of the player (docs/plans/standing.md 6). A pure function of the
//! world and the logs. Nothing is stored.

/// What standing reads, as plain data.
pub struct Conduct<'a> {
    pub character: &'a Character,
    pub quests: &'a [Tracked],
    pub emotes: &'a [Flavor],
}

/// The six parts of one town, each inside its cap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parts { pub people: i64, pub slaps: i64, pub manners: i64, pub words: i64, pub promises: i64, pub choices: i64 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step { Distrusts, Wary, Neutral, Likes, Trusts }
```

| Function | Job |
|---|---|
| `pub fn towns(conduct: &Conduct<'_>) -> Vec<&str>` | Every town that holds an NPC or a quest, sorted by name. |
| `pub fn parts(conduct: &Conduct<'_>, town: &str) -> Parts` | The six parts of one town. |
| `pub fn standing(parts: Parts) -> i64` | The sum, clamped to `STANDING`. |
| `pub fn step(standing: i64) -> Step` | The step of 6.4. |
| `pub fn chat_words(town: &str, step: Step) -> String` | Not used by the desktop. The addon holds the chat words. The test of the addon checks the same table. |
| `pub fn prompt_line(town: &str, step: Step) -> Option<String>` | The prompt line of 6.4. None for `Neutral`. |
| `pub fn overall(conduct: &Conduct<'_>) -> i64` | The sum of the standing of every town (6.7). |
| `fn people(...)`, `fn slaps(...)`, `fn manners(...)`, `fn words(...)`, `fn promises(...)`, `fn choices(...)` | One part each, inside its cap. |

Each part is a short function with one job. `parts` calls the six. No part calls another.

### 6.7 The value that the journal look reads

The look of the journal book follows standing. The look is a separate design task, and this plan does not design it. It reads one value:

- **`overall`**: the sum of the standing of every town. No global value is stored. A town that you never touched adds 0.
- In five named steps, for the look only:

| Sum | Look step |
|---|---|
| -30 and less | `Feared` |
| -29 to -10 | `Doubted` |
| -9 to 9 | `Unknown` |
| 10 to 29 | `Respected` |
| 30 and more | `Beloved` |

`journal.rs` carries `look: LookStep` in the first page. The player never sees the name of a look step. The addon uses it only to pick the art.

## 7. Talk-window tones

talk-window.md gives each turn 3 options. This plan adds a tone to each option.

### 7.1 The answer

The model writes each option as an object:

```json
{"say": "Work? The mill has rats.", "trust": 1, "options": [
  {"text": "Rats? I can handle rats.", "tone": "kind"},
  {"text": "What's it worth to you?", "tone": "cunning"},
  {"text": "Do your own dirty work.", "tone": "cruel"}]}
```

- `checked_options` takes an object with `text` and `tone`, and also a plain string, with no tone. So a small model that forgets the tone still gives options.
- A tone that is not one of the three drops the tone, not the option.
- An option also fails the cruelty list of 4.6. A cruel reply is fine; cruelty to the helpless is not.
- The `talk_answer` line to the addon keeps plain strings. The tone stays on the desktop. So the relay check of talk-window.md 3.6 does not change.

The prompt of a turn adds to its note: "Give each reply a tone: "kind", "cunning", or "cruel". Make the three tones differ when it fits."

### 7.2 The pick

- `Conversation` keeps the options of the newest answer with their tones, in memory: `pub offered: Vec<(String, Option<Tone>)>`. talk-window.md 4.1 says that the options are not kept; this plan changes that one rule.
- A turn whose words are the same as an offered option, byte for byte after a trim, has that tone. Typed words have no tone. The player picked the option, so the tone is the player's act.
- `Pending::Talk` gets `picked: Option<Tone>`.
- **When it lands.** The tone lands with the answer of that turn, in `talk_answered`, next to the trust change: `spoke_kindly` +1 or `spoke_cruelly` +1 on you, linked to the NPC. A cunning pick lands nothing. With no answer that passes, nothing lands, as for trust.
- **Why with the answer.** The tone came from the call of the turn before. The answer of this turn rests on its call, and that call reads the calls of the earlier turns (talk-window.md 4.6). So the proof of the tone fact reaches both the pick of the player and the call that offered it.

The cap of Words is ±1 for each town (6.3).

## 8. Proof (5.14)

### 8.1 Rows

| Row | Made by | Root |
|---|---|---|
| `Offered` with a choice (`quests`) | the quest call | Player and Game: `/quest` and what the call read |
| `Sided` (`quests`) | the line of the encounter: `npc_met` or `talk_asked` | Game for `npc_met`, Player for `talk_asked` |
| `quest_done`, `sided_with`, `sided_against`, `rivals`, and both trust changes | the same line | the same |
| `spoke_kindly`, `spoke_cruelly` | the talk call of the turn | Player |
| An emote (`flavor`) | the `emote_done` line | Game |

A trust change of a choice rests on the encounter alone, as a finished quest does today. The game decides when the choice ends (rule 4). The words of the offer come from a call, but the deed is the encounter.

### 8.2 Reads

A new rule in `story/reads.rs`:

```rust
/// Every row behind the standing of a town: the trust, slap, and tone facts of its NPCs,
/// the emote rows at them, and every `quests` row of its quests.
pub(super) fn standing_read(active: &Active, town: &str) -> Vec<Node>
```

- The events that opened `trusts`, `slapped`, `spoke_kindly`, and `spoke_cruelly` of each NPC of the town.
- The `flavor` rows of the emotes that counted.
- Every `quests` row of each quest of the town: the offer and each change.

A call reads it for each town whose standing line its prompt holds, also for a neutral town whose prompt shows no line: the call still read the state. More reads are the safe side.

| Call | New reads |
|---|---|
| Quest offer | `standing_read` of the town of the giver. With a rival: `events_about` the rival, and every `quests` row of the earlier choices between the two. |
| Talk | `standing_read` of the town of the NPC, and the rows behind each line of the talk of the town (9.3). |
| Saga draft | `standing_read` of each town whose line the chapter prompt holds (9.5). |
| Narrator | For a `Sided` moment, the `quests` row of the `Sided` change and its offer. |

### 8.3 Why the trust changed

`TrustCause` gets `Choice`. `why::cause_of` finds it when the input that made the trust event also made a `Sided` row in `quests`. It needs one query in `store/database.rs`:

```rust
/// True when this input made a `quests` row of this line kind.
pub fn input_made_quest_row(&self, input: u64, line: &str) -> Result<bool, StoreError>
```

`trust_why.by` can then be `choice`. The addon gets two new lines (10.1). This is a new value in the journal, which is free JSON for the bridge. Tell the Gnomish Relay session anyway, so its fake story program knows the value.

The journal shows no "Why?" for standing. A town has many causes, and 5.14 keeps "Why?" for trust.

## 9. Effects

Every effect is flavor (rule 2). Each one names where it hooks in.

### 9.1 How NPCs greet you

- `talk::Scene` gets `standing: Option<String>`: the prompt line of the town of the NPC (6.4). `who_you_are` adds it after the trust words. A neutral town adds nothing.
- `Scene` also gets `sided: Option<SidedWords>`, from the `sided_with` and `sided_against` facts about this NPC: "The player sided with you in a quarrel once." or "The player sided against you in a quarrel 2 times." These are facts between you and the NPC, so they sit with the slaps in `who_you_are`, not in the memory block.

### 9.2 Who gives you a quest

- **The standing line** goes into every quest prompt, after the persona (6.4).
- **A lawful giver refuses** at `Distrusts`. A giver is lawful when its name holds a word of `crates/story/data/lawful_titles.txt` as a whole word: Guard, Captain, Marshal, Sheriff, Deputy, Lieutenant, Sergeant, Commander, Watchman, Magistrate. The refusal is a line of the code, with no model call, in `fn refusal`: "Marshal Dughan won't give you a quest while Goldshire distrusts you." Asking still meets the NPC.
- **A shady offer.** At `Wary` or `Distrusts`, a giver that is not lawful gets one more line: "If you work outside the law, their distrust can make you more willing to ask them for help, not less." The model decides who it plays. So a known thief gets offers from people who need a thief.
- The candidate rival comes from 5.1, also for a lawful giver below `Distrusts`.

### 9.3 The world retells your deeds: the talk of the town

NPCs of a town talk about what you did there, also NPCs that you never met. "Watch that one. Robbed Farley blind." Each line comes from a row of the world, so the NPC repeats gossip; it never invents a deed.

**Who learns what.** Every NPC of a town hears of the deeds and choices of that town, and nothing else. It needs no meeting: the first talk with a stranger of Goldshire already gets the block. An NPC of another town hears nothing of Goldshire, except the deeds near it that npc-memory.md gives (below).

**The block.** `talk::Scene` gets `town_talk: Vec<String>`, worded by `standing::town_talk(conduct, town, npc, now)`. At most 3 lines, newest first:

1. the standing line of the town (6.4), when the town is not neutral;
2. up to 2 conduct lines, from these rows of the town:

| Source | Line |
|---|---|
| A `Sided` quest | `{When}: the player sided with {with} against {against} over "{title}".` |
| A quest abandoned after accept | `{When}: the player gave up on a quest for {giver}.` |
| A quest finished, with no choice | `{When}: the player did a quest for {giver}.` |
| The newest slap of each NPC | `{When}: the player slapped {npc}.` |

`{When}` is `npc_memory::when` (npc-memory.md 5.2): "Three weeks ago", never a date.

The block in the prompt, after the memories:

```
What people in Goldshire say about the player. You heard it from others, so you were not there:
<<<
- People in Goldshire are wary of the player.
- Two days ago: the player sided with Marshal Dughan against Innkeeper Farley over "The Missing Ledger".
>>>
Bring up at most one of these, and only when it fits. You can be unsure that it was this player. Never add a deed that is not written here.
```

With no line, the block is absent. The rule "Never speak of a past with the player that is not written here." of npc-memory.md stays.

**No line twice.** The memory block of npc-memory.md is what is between you and this NPC. The talk of the town is what is between you and the others of its town. So:

- a conduct line whose NPC is the NPC of the talk is left out: its quests are memories already (`GaveQuest`), and its slaps are in the scene;
- big deeds stay in npc-memory.md. `DefeatedNear` already gives "the player defeated Hogger in Elwynn Forest" to any NPC of the zone, also on a first talk. The talk of the town adds no kill line;
- a `Sided` quest of this NPC is a memory (`GaveQuest`, `Finished`) and a scene line (9.1), never a town line.

**Tell the npc-memory session:** `GaveQuest` of a choice quest gets the side in its ending: "They finished it, and sided with you." or "They finished it, and sided with {rival}." That is the only change to npc-memory.md.

**When.** Only the first turn of a conversation gets the block, as for the memories (talk-window.md 4.3).

**Size.** 3 lines of at most 280 characters, and about 60 characters of rules: about 230 tokens. The first turn takes at most 882 + 230 = 1112 tokens, under 1773 (talk-window.md 4.3).

**Reads.** The talk call reads the rows behind each line: the `quests` rows of the quest, and the event that opened the `slapped` count (8.2).

### 9.4 Titles

Four joke titles from choices. `titles::earned` takes the quest log too: `earned(moments, character, quests)`.

| Title | Rule |
|---|---|
| The Turncoat | 3 choices against the giver |
| Silver Tongue | 3 cunning sides |
| Heart of Gold | 3 kind sides |
| The Ruthless | 3 cruel sides |

A title stays for good, as every title does. Standing can recover; a title is a deed of the past.

### 9.5 The narrator and the saga

- **A big moment.** `Moment::Sided { quest, with, against }`, rank 3, with a slap. `what_happened`: "The player ended the quest "{quest}" on the side of {with}, against {against}."
- **The deed.** `Deed::QuestDone` gets `sided_with: Option<String>` and `against: Option<String>`. The facts of a chapter say: "The Missing Ledger (a quest for Innkeeper Farley; the player sided with Marshal Dughan against Farley)".
- **The tone of a chapter.** The chapter prompt gets the standing line of each town of the chapter that is not neutral, under the facts: "People in Goldshire are wary of our hero." The persona and the note do not change. The narrator stays serious and tells the facts; the facts carry the tone.

### 9.6 The journal

- `Journal` gets `towns: Vec<TownStanding>`: each town that is not neutral, with its zone and its step. No number goes out.
- The addon prints a chat line when the step of a town changes, as `Trust.lua` does for an NPC. The first journal of a session prints nothing.
- The map pane gets one line under the line of subzones: the towns of the zone on the map that are not neutral. "Goldshire likes you. Northshire Abbey is wary of you."
- The journal carries `look` (6.7) on the first page.

## 10. UI copy

The copy follows the UI copy rules of `CLAUDE.md`. Every line that a player reads:

| Where | Copy |
|---|---|
| Quests page, the choice heading | Choose one: |
| Quests page, the giver side | Speak with Innkeeper Farley: Give the ledger back to Farley. |
| Quests page, the rival side | Speak with Marshal Dughan: Hand the ledger to the Marshal. |
| Quests page, the choice before the plain steps are done | (the two sides, faded, as a later step is) |
| Quests page, after the choice | You sided with Marshal Dughan. |
| Quest pins | A pin with the number of the choice on each of the two NPCs |
| Chat, a town changes step | Goldshire is now wary of you. (and the other lines of 6.4) |
| Map pane, under the subzones | Goldshire likes you. Northshire Abbey is wary of you. |
| A lawful giver refuses (notice) | Marshal Dughan won't give you a quest while Goldshire distrusts you. |
| Tooltip "Why?", trust up | Went up when you sided with them. |
| Tooltip "Why?", trust down | Went down when you sided against them. |
| Titles | The Turncoat, Silver Tongue, Heart of Gold, The Ruthless |

Bad and good:

| Bad | Good | Why |
|---|---|---|
| Goldshire standing: -6 | Goldshire is wary of you. | Never a number. The words of trust, for a town. |
| Your alignment shifted toward evil. | Goldshire is now wary of you. | Name the town and its feeling, not a mechanism. |
| Choice step: meet giver OR meet rival | Choose one: | "Step", "giver", and "rival" are internal words. |
| Rival side (cunning): Hand the ledger to the Marshal. | Speak with Marshal Dughan: Hand the ledger to the Marshal. | The tone is internal. The player judges the side. |
| The townsfolk of Goldshire whisper thy name with scorn. | Goldshire distrusts you. | Fake flourish. The UI never performs a voice. |
| Quest refused: standing too low (lawful NPC). | Marshal Dughan won't give you a quest while Goldshire distrusts you. | Says what went wrong and how it changes, with no internal words. |
| You chose Marshal Dughan. Farley trust -10. Dughan trust +10. | You sided with Marshal Dughan. | The tooltip already shows trust in words. No numbers. |
| Thinks less of you. (for a wary town) | Goldshire is wary of you. | The words must match the feeling, as for NPCs. |

The model writes the side texts, and a player reads them, so the prompt gives what `CLAUDE.md` asks for: where they show (a line of the quest that the player picks), the length (80 characters), the tone (a plain sentence of what the player does), and an example (4.4).

## 11. Edge cases

| Case | Rule |
|---|---|
| The giver and the rival are in one town, and the player ends with the rival | The giver loses 10 trust, the rival gains 10, and the choice counts in the town of the offer by the tone of the rival side. |
| The player talks to the giver by accident, to sell | The gossip window decides. The Quests page shows "Choose one:" with both NPCs, and pins both. Open question 2. |
| The giver dies in your story before the end | The rival can still end it. The dead have no trust part. |
| A second `Sided` of one quest (a replay, a bug) | Changes nothing. |
| A choice quest abandoned after accept | A broken promise: -2 in Promises, no `rivals`, no `sided_*`. |
| A choice quest declined | Nothing. No `rivals` fact: an offer is only words of the model. |
| An NPC moves to another town | Its trust, slaps, emotes, and words move with it. A quest stays in the town of its offer. |
| Two NPCs with one name | The world keeps one person per name (npc-memory.md 8). They share a town, as they share trust. |
| A town in two zones with the same subzone name | `place` keeps a subzone by its name and its zone (`character.rs`). `town_of` gives the subzone name, so two towns of one name merge their standing. Rare, and it never invents a deed. |
| An emote at a player | The addon sends no player target (5.11). It never counts. |
| An emote at an NPC that became hostile later | It stops counting while the NPC is hostile. |
| A pick of an option after a restart | The conversation is gone, so the words have no tone. |
| No model | No choice quest and no tones. Slaps, emotes, and finished quests still move standing. |

## 12. Tests

Each test is a sentence, and reads arrange, act, assert.

### 12.1 Unit and story tests

`crates/story/tests/standing.rs` (new):

- `a_town_is_the_subzone_of_the_npc`
- `a_district_of_a_capital_belongs_to_the_capital`
- `an_npc_in_a_dungeon_has_no_town`
- `an_npc_in_a_zone_with_no_subzone_belongs_to_the_zone`
- `a_new_character_is_neutral_everywhere`
- `each_trust_band_gives_its_weight`
- `the_people_part_stops_at_two`
- `each_slap_costs_one_down_to_four`
- `a_kind_emote_at_a_friendly_npc_raises_manners`
- `a_rude_emote_at_a_friendly_npc_lowers_manners`
- `an_emote_at_a_hostile_npc_or_an_animal_counts_nothing`
- `an_emote_off_both_lists_counts_nothing`
- `manners_stop_at_two_either_way`
- `a_cunning_reply_counts_nothing`
- `words_stop_at_one_either_way`
- `a_finished_quest_raises_the_town_of_its_offer`
- `a_quest_abandoned_after_accept_is_a_broken_promise`
- `a_declined_or_unanswered_offer_counts_nothing`
- `a_choice_counts_by_the_tone_of_the_side_and_not_as_a_promise`
- `choices_stop_at_nine_either_way`
- `standing_stays_between_minus_twenty_and_twenty`
- `each_step_starts_at_its_edge` (standing -20, -10, -9, -4, -3, 3, 4, 9, 10, 20)
- `a_neutral_town_has_no_prompt_line`
- `the_overall_value_is_the_sum_of_the_towns`
- `each_look_step_starts_at_its_edge` (-30, -29, -10, -9, 9, 10, 29, 30)

`crates/story/tests/quest.rs`:

- `a_choice_quest_parses_with_its_rival_and_two_sides`
- `a_choice_with_no_rival_in_the_prompt_is_refused`
- `a_choice_that_names_another_rival_is_refused`
- `a_plain_step_that_names_the_rival_is_refused`
- `a_choice_quest_needs_one_or_two_plain_steps`
- `a_side_over_eighty_characters_is_refused`
- `two_sides_with_the_same_words_are_refused`
- `a_side_with_an_unknown_tone_is_refused`
- `an_offer_that_promises_gold_is_refused`
- `a_place_named_gold_coast_quarry_passes_the_reward_check`
- `a_choice_that_harms_a_child_is_refused`
- `a_plain_quest_to_find_a_child_passes`
- `the_prompt_with_a_rival_fences_the_card_and_states_the_rules`
- `the_prompt_with_no_rival_is_as_before_but_for_the_reward_rule`
- `the_quest_log_keeps_a_choice_open_after_its_plain_steps`
- `a_sided_change_finishes_a_choice_quest`
- `a_second_sided_change_changes_nothing`
- `a_sided_change_before_the_plain_steps_changes_nothing`
- `an_old_offer_with_no_town_or_choice_still_reads`

`crates/story/tests/choice.rs` (new):

- `the_rival_is_met_in_the_same_town`
- `a_seen_npc_is_never_a_rival`
- `the_giver_a_foe_an_animal_or_the_dead_are_never_a_rival`
- `an_npc_in_an_open_choice_is_never_a_rival`
- `an_old_rival_of_the_giver_comes_first`
- `with_no_old_rival_the_newest_meeting_comes_first`

`crates/story/tests/story.rs`:

- `talking_to_the_giver_ends_a_choice_on_the_giver_side`
- `talking_to_the_rival_ends_a_choice_on_the_rival_side`
- `a_slap_never_ends_a_choice`
- `asking_for_a_quest_never_ends_a_choice`
- `the_end_of_a_choice_moves_both_trusts_the_opposite_way`
- `the_end_of_a_choice_starts_or_counts_up_the_rivals_fact`
- `a_later_offer_of_the_giver_names_its_rival_first`
- `a_lawful_giver_refuses_a_quest_in_a_town_that_distrusts_you`
- `a_lawful_giver_still_offers_in_a_town_that_is_wary_of_you`
- `a_quest_prompt_reads_the_rows_behind_the_standing_of_its_town`
- `a_talk_prompt_holds_the_standing_line_of_a_town_that_is_not_neutral`
- `a_picked_kind_option_lands_spoke_kindly_with_the_answer`
- `a_typed_reply_has_no_tone`
- `a_pick_with_no_answer_lands_no_tone`
- `the_trust_why_of_a_choice_is_choice`
- `the_journal_carries_only_towns_that_are_not_neutral`
- `the_narrator_tells_the_end_of_a_choice`

`crates/story/tests/talk.rs`:

- `an_option_keeps_its_tone`
- `an_option_as_a_plain_string_has_no_tone`
- `an_option_with_an_unknown_tone_keeps_its_words`
- `an_option_with_a_cruel_word_is_dropped`
- `the_talk_of_the_town_holds_at_most_three_lines_newest_first`
- `the_talk_of_the_town_leaves_out_the_npc_of_the_talk`
- `the_talk_of_the_town_holds_no_kill`
- `a_stranger_of_the_town_hears_the_talk_of_the_town`
- `an_npc_of_another_town_hears_nothing_of_it`
- `a_line_of_the_talk_of_the_town_cannot_close_its_fence`

`crates/story/tests/titles.rs` (or the titles part of `flavor.rs`): one test for each new title, and `a_title_of_choices_stays_after_standing_recovers`.

`crates/story/tests/voice.rs`: `quest_offer_with_a_rival()` at full length, and `npc_talk()` with a full talk of the town. The budget test covers both.

`crates/addon-tests/tests/`:

- `a_choice_shows_both_sides_under_choose_one`
- `a_finished_choice_shows_the_side_that_you_took`
- `a_choice_pins_both_npcs`
- `a_town_that_changes_step_prints_one_chat_line`
- `the_first_journal_prints_no_town_line`
- `the_map_pane_names_the_towns_of_its_zone_that_are_not_neutral`
- `the_why_line_of_a_choice_names_the_side`
- `a_look_step_that_this_addon_does_not_know_is_ignored`

### 12.2 Property tests (`crates/story/tests/properties.rs`)

New plays: `Play::Choice(giver, rival, steps, giver_tone, rival_tone)` for a choice quest that the model offers, `Play::Pick(index)` for an option of the talk window, and `Play::Abandon`. The emote strategy makes a token of each list likely, with a target of a friendly or a hostile NPC.

- `normal_play_leaves_every_town_neutral`. Any play of only `Zone`, `Defeat`, `Kill`, `Level`, `See` of a hostile NPC, `Instance`, `GameQuest`, and `Wait`: every town has standing 0, the journal has no town, and `overall` is 0.
- `standing_stays_in_its_band`. Pure: any `Parts` from any counts, with 0, 1, each cap, the cap plus 1, and 1000 made likely. Each part stays in its cap, and the sum stays in `STANDING`.
- `a_choice_quest_ends_at_most_once`. Pure: any list of `QuestChange` with many `Sided` rows of one number. At most one `Sided` counts, the status is `Done` at most once, and `sided_with` never changes after it is set.
- `no_offer_that_promises_a_reward_passes_the_check`. Any quest answer, with a word of `reward_words.txt` put into the title, the text, or a side, in any case: `checked_quest` refuses it, unless the word is only inside a name of the prompt.
- `talk_alone_never_moves_a_town_off_neutral`. Any play of only `Meet`, `Talk`, `Pick`, and `Wait` (24 hours made likely), with trust answers of -5 and 5 made likely: every town stays `Neutral`.
- `standing_can_always_recover`. Any play, then 3 finished kind choice quests and 3 finished plain quests in each town that is below neutral: every town is `Neutral` or better.
- `every_standing_line_in_a_prompt_rests_on_rows_that_the_call_read`. For each quest, talk, and saga call with a standing line or a talk of the town line, the rows behind it are in the `reads` of the call.

### 12.3 Fuzz

- `fuzz/fuzz_targets/answers.rs`: `assert_quest` gets a `known` with `rival: Some("Farmer Bram")`. A random answer with a random `choice` (missing, not an object, a wrong rival, long sides, odd tones). Every offer that passes keeps its limits: 1 or 2 plain steps with a choice, the rival of the prompt, no plain step names it, two side texts of at most 80 characters that differ, a known tone, no reward word outside a name, and no cruel word. `talk::checked_answer` with options as objects, strings, and other types. New seeds in `fuzz/seeds/answers/`: `choice_quest.txt`, `choice_reward.txt`, `choice_cruel.txt`, `choice_wrong_rival.txt`, `choice_no_plain_step.txt`, `talk_options_tone.txt`.
- `fuzz/fuzz_targets/play.rs` and `pages.rs`: choice quests, picks, and emotes. After each line, the model of the fuzzer checks that every town of the journal has a known step, and that no page holds a number for a town.
- `fuzz/fuzz_targets/store.rs`: damaged `Sided` rows and offers with odd `town` and `choice` fields at open. The open never fails on them.
- `fuzz/fuzz_targets/replies.rs`: a journal with random `towns`, `look`, and `trust_why.by`. The addon never raises a Lua error.

## 13. Build order

Each step is one commit with its tests, and each one ships.

1. `tone.rs`, the data files, and `choice::reward_word`. Every quest gets the reward check and the reward rule in its prompt. No choice yet.
2. The five facts in `vocabulary.rs` (version 8), and `Character::town_of`, `side_with`, `rivals_of`, and `speak_with_tone`.
3. `standing.rs`: the parts, the band, the steps, and `overall`, with `tests/standing.rs` and the pure property tests. Nothing calls it yet.
4. `quest.rs`: `Choice`, `Side`, `Party`, `Sided`, the `town` of an offer, and the quest log rules. `tests/quest.rs`, and the property `a_choice_quest_ends_at_most_once`.
5. `choice::rival_for`, the prompt block, and the choice checks. The answers fuzz target and its seeds.
6. `story/quests.rs`: the rival in the call, `Encounter`, `settle_choices`, and `standing_read`. The story tests, and the play property tests.
7. Emotes in standing, and the journal: `towns`, `look`, and `trust_why` with `choice`. Tell the Gnomish Relay session about the new value of `trust_why.by`.
8. The addon: the choice on the Quests page, the pins, the chat line, the map line, and the "Why?" lines.
9. The effects: the standing line in talk and quest prompts, the lawful refusal, the shady line, the titles, the narrator moment, and the chapter facts.
10. The talk of the town (9.3). Tell the npc-memory session about the side in `GaveQuest`.
11. After talk-window.md step 4: the tones of the options, the pick, and the Words part.
12. The `GAMEPLAY.md` text of section 16. This plan marks the steps as done.

## 14. Open questions for the user

1. **Town = subzone.** The plan takes the subzone of the NPC, with capitals as one town (3). The other choice is the zone: fewer and bigger towns ("Elwynn Forest is wary of you"), and no odd names like "Brackwell Pumpkin Patch". Which one?
2. **An accidental choice.** A gossip window of the giver or the rival ends the choice, also a visit to sell. Is that right, or does a choice end only with `/talk`?
3. **The emote tokens.** The two lists of 6.2 use the tokens of the classic emote list. A test in the game confirms each token through `PerformEmote`.
4. **A judge for cruelty.** The word list is a first guard. Add a short judge call for each choice quest later, at one more call per choice offer?
5. **The four titles.** Are titles from choices wanted, and are these names right?

## 15. What this plan does not do

- No global value is stored. `overall` is a sum that the look reads.
- No design of the look of the book.
- No faction, race, or mob kill ever moves standing.
- No decay with time. Later deeds balance earlier ones.
- No reward of the game, ever.

## 16. GAMEPLAY.md text

Add a new section after 3.7:

> ### 3.8 Standing and choices
>
> Each town has its own view of you: its standing. The town of an NPC is the subzone where it lives. A capital city is one town, and a dungeon or a raid is no town. Standing comes only from facts of your world. The code computes it each time, and stores nothing.
>
> - **Normal play is neutral.** Kills, grinding, levels, zones, faction, and race never move standing.
> - **Six parts make the standing of a town.** Each part has a cap, so no one source decides it:
>
>   | Part | Each | Cap |
>   |---|---|---|
>   | People | each NPC of the town by its trust band: +2, +1, 0, -1, or -2 | ±2 |
>   | Slaps | -1 for each slap | -4 |
>   | Manners | +1 for a kind emote, -1 for a rude emote, at a friendly NPC | ±2 |
>   | Words | +1 for a kind reply that you picked, -1 for a cruel one | ±1 |
>   | Promises | +2 for a finished side quest, -2 for one that you abandoned after you accepted it | ±6 |
>   | Choices | the tone of the side that you took: kind +3, cunning -1, cruel -3 | ±9 |
>
>   The sum stays between -20 and 20. A declined offer counts nothing.
> - **Talk alone never moves a town off neutral.** Talk moves People and Words by at most 3, and neutral is -3 to 3.
> - **Words, never numbers.** 10 and up: "Goldshire trusts you". 4 and up: "likes you". -3 to 3: neutral, and nothing shows. -4 and down: "is wary of you". -10 and down: "distrusts you". The chat says when a town changes step. The map pane names the towns of its zone that are not neutral.
> - **Redemption.** Every part comes back with later deeds. Slaps stay, but their part stops at -4. Nothing fades with time.
> - **Choice quests.** When an NPC offers a side quest, the code looks for a second NPC that you met in the same town, that lives and is no foe. An old rival of the giver comes first. The prompt asks the model for a choice only when the two can want the same thing. A choice quest has 1 or 2 steps, then the choice: talk to the giver or to the other NPC. The first one that you talk to decides. A slap never decides. Your side gains 10 trust, and the other side loses 10. The world keeps the quarrel as `rivals`, so later quests of either NPC build on it.
> - **No cruelty to innocents.** No choice harms a child, a helpless or innocent person, or uses torture. Betrayal, greed, theft, lies, and mercy against justice are fine. The prompt says it, and a list of words refuses the rest.
> - **Rewards are story only.** No offer promises gold, an item, or anything of the game. A list of words refuses such an offer. The names of the game do not count.
> - **Effects, all flavor:** NPCs greet you by the standing of their town. A guard, a captain, or a marshal gives you no quest in a town that distrusts you, and other NPCs in a town that distrusts you can ask you for work outside the law. NPCs repeat the talk of their town: your choices, broken promises, finished quests, and slaps there, also NPCs that you never met. Choices earn titles. The saga tells your choices, and the standing of each town of a chapter.
> - **The look of the journal** reads the sum of the standing of every town.

Add to 3.4, under "The check":

> - **No reward of the game.** The title, the text, and each side hold no word of `reward_words.txt`, after the names of the prompt are taken out.
> - **A choice** (3.8): the other NPC is the one of the prompt, no plain step names it, a choice quest has 1 or 2 plain steps, and each side is at most 80 characters with a tone. The two sides differ, and hold no word of `cruel_words.txt`.

Add to 3.5, after the bullet of the reply options (talk-window.md 11):

> - Each reply has a tone from the model: kind, cunning, or cruel. You never see it. A kind or cruel reply that you click moves the standing of the town a little (3.8). A reply that you type has no tone.
> - **The talk of the town.** The first turn of a talk gets up to 3 lines of what the town says about you: its standing, and your choices, promises, and slaps there. The NPC heard them from others. It never hears a deed of another town, and it never adds one.

Add to the table of 5.1:

> | `rivals` | number, 0 to 1000 | up | person to person | The quarrels between the giver and another NPC that you settled in choice quests (3.8). |
> | `sided_with`, `sided_against` | number, 0 to 1000 | up | person to person | The choices that you settled for or against this NPC. |
> | `spoke_kindly`, `spoke_cruelly` | number, 0 to 1000 | up | person to person | The kind or cruel replies that you clicked in a talk with this NPC. |

Add to 5.4.1, after "With consequences":

> - **Kind and rude emotes.** A kind emote, such as `/hug` or `/thank`, or a rude one, such as `/spit` or `/rude`, at a friendly NPC moves the standing of its town a little (3.8). The lists are data. Every other emote is neutral, and an emote stays a flavor row, never a fact.

In 5.14, add to the table of reads:

> | Quest offer | ... and the rows behind the standing of the town of the giver, and with a rival, the events behind it and the rows of the earlier choices between the two |
> | Talk | ... and the rows behind the standing of the town of the NPC, and behind each line of the talk of the town |
> | Saga draft | ... and the rows behind the standing of each town of the chapter that its prompt names |

In 5.14, change "Why? shows only for trust": the cause can also be a choice: "Went up when you sided with them." or "Went down when you sided against them."
