# Plan: slots and templates for narrator lines

Status: spec only, 2026-10-05. Nothing is built. The templates in section 9 wait for the approval of the user. The plan follows the style guide of the narrator (`narrator-style.md`, approved 2026-10-05): its section 9 dropped 8 parts, added 2 outcomes with no hero, and made the lore end on the present.

## 0. The decision

The user decided on 2026-10-05: **the model writes the lore, the code writes the hero.**

For a short narrator line, the model returns JSON. The JSON holds one free sentence of history and a few choices from closed lists. The code builds the line from a template. So a bad shape such as "its power grows in the druid" or "$N came to Elwynn" cannot exist. The code does not refuse it after the fact: no template can build it.

Today the model writes the whole line, and `line_check.rs` refuses the bad ones. The checks stay, but for the hero part they become a second guard, not the only one.

## 1. Scope

**Templates (short lines).** Each moment of `moments.rs` that names the hero, and the arrivals:

| Moment | Today | With templates |
|---|---|---|
| Arrival: a new zone, a capital, a dungeon, a raid | `NewZone`, `FirstCapital`, `FirstInstance` | Lore only. The code adds nothing. |
| Tenth level | `LevelUp` | Lore + group clause + level fact (`level-lines.md`) |
| Kill of a rare or a boss | `FirstKill` | Lore + deed |
| Revenge: the first kill of a foe that killed you | (none, `chapters.md` 4) | Lore + two deed sentences |
| Death to the same NPC again | `SlainAgain` | Lore + deed |
| Finished side quest | (none, 3.4) | Lore + deed |
| Class quest | `ClassQuestDone` | Lore + deed |
| Joke title | `Titled` | Lore + deed (question 2) |
| Slap | `Slapped` | Lore + deed |
| Quest mark | `QuestMarked` | Lore + deed |
| First mount, first epic mount | `FirstMount`, `FirstEpicMount` | Lore + deed |
| First epic item, big upgrade | `FirstEpicItem`, `BigUpgrade` | Lore + deed |

**Free text (no templates).** Sagas, tales, the summary, zone histories, NPC talk, and `/lore` answers. Why:

- A paragraph needs its own order of sentences, and the hero takes many roles in it. A template of a paragraph is a form letter.
- NPC talk is a reply to the player's words. No closed list can hold a reply.
- A `/lore` answer is the voice of a historian about a question, with no deed of the hero.
- These texts keep their checks: slop, arrival of the hero, the cutoff, and copies.

**Flavor moments** (5.4.1) stay free text for now (question 3).

## 2. The slot JSON

### 2.1 Common to every moment

| Field | Kind | Check |
|---|---|---|
| `lore` | Free text: the history | All the checks of a line today, on this sentence alone: cutoff, emoji, banned words, slop, copy of a sample, new numbers, grounded, brackets, and the checks of the style guide (`prose.rs`). New: no `$N`, at most one sentence (two for an arrival), at most the char budget of the moment (2.4). |
| other fields | Closed choices | Each value must be in the list that the prompt offered. Some choices also need the lore to name a word (below). |

- `lore` comes first in the JSON. So the model writes the history first and chooses after it.
- `lore` follows the style guide (`narrator-style.md` 3 and 5): one turn of history, no hero, and no source. It **ends on the present**: what holds in the place, the people, or the order now. The template adds the deed after it.
- `{"lore": "SILENCE"}` is silence. A bare `SILENCE` answer stays silence too.
- The parser is strict: one JSON object, no unknown field, no missing field. It allows one code fence around the object, because small models add one. Anything else is the fault `BadAnswer`.
- A fault gets one retry with the reasons, as today. A second fault is silence.

### 2.2 Fields for each moment

| Moment | Closed fields | Values | Extra check |
|---|---|---|---|
| Arrival | none | | `lore` holds at most 2 sentences. The code adds no hero and no deed. |
| Tenth level | `group` | the group ids of the pairing (3.4) | The lore names an anchor of the group, such as "Silver Hand" or "Light". |
| Kill | `there`, `leads`, `leads_number` | `true`, `false`; a group that the lore names, or `none`; `one`, `many` | `true` needs the lore to name the zone of the moment. `leads` needs the lore to name the group, as whole words, in at most 4 words. |
| Revenge | `tone` | `plain`, `dry` | none |
| Death | `killer`, `tone`, `there` | `one`, `kind`; `plain`, `dry`; `true`, `false` | `there` as for a kill. |
| Side quest | none | | |
| Class quest | none | | |
| Title | none | | |
| Slap | `tone`, `there` | as above | `there` as for a kill. |
| Quest mark | none | | |
| Mount, epic mount | `breed` | `true`, `false` | `true` needs the lore to name the breed noun of the mount ("ram" for "Gray Ram"). |
| Epic item, upgrade | `there` | `true`, `false` | `there` as for a kill. |

What each choice means, in the words of the prompt:

- `group`: which group the history tells of.
- `there`: the history names the place where it happened.
- `killer`: `one` when the killer is one named person ("Hogger"), `kind` when it is one of many ("Murloc Coastrunner").
- `tone`: `dry` allows the dry parts. If no dry shape fits, the code takes a plain one.
- `breed`: the history tells of the breed of the mount.
- `leads`: the group that the foe led, as the lore names it: "the Riverpaw". `leads_number` says if the group is one ("the Brotherhood") or many ("the Riverpaw"), so the outcome takes the right verb.

**The model never picks the verb, the frame, or the naming.** The code picks them by rotation (3.5). A model choice is only a fact about the lore that the code cannot see. So a wrong choice gives a line that is still grammatical, and at worst plain.

Examples:

```json
{"lore": "Uther the Lightbringer founded the Silver Hand in the Second War, and its paladins still carry the Light against the Scourge.", "group": "o.silver_hand"}
{"lore": "The murlocs of the Westfall coast have no kingdom and no history.", "killer": "kind", "tone": "plain", "there": false}
{"lore": "Thrall named Durotar for a father he never knew. The orcs who spent years in human camps hold its red canyons now."}
{"lore": "For years Hogger led the Riverpaw gnolls in raids on the farms of Elwynn, and Stormwind still has a price on his head.", "there": false, "leads": "the Riverpaw", "leads_number": "many"}
```

### 2.3 What the moments need

Some fields need facts that the moments do not hold today:

| Need | Change |
|---|---|
| `there` for a kill, a death, a slap | A `zone` on `FirstKill`, `SlainAgain`, and `Slapped`, as `LevelUp` has. |
| Foe-type parts | The creature type on `npc_defeated` (`UnitCreatureType`): beast, undead, demon, dragonkin, elemental. An event line with no reply, so the relay needs no message. |
| Revenge | A moment `Revenge { foe, deaths, zone }` from the foe record of `chapters.md` 4. |
| Item parts | The slot class of the item: weapon (slots 16 to 18), or worn (every other slot). `upgraded` holds the slot. The first epic item needs it too. |
| Side quest | A moment for a finished Timeways quest (3.4). |

### 2.4 The char budget

A line has at most 300 characters (`MAX_LINE_CHARS`). Before the call, the code renders the longest shape that fits the moment, with the real values of the moment. The budget of the lore is 300, minus that length, minus 1. The prompt gives the budget as a count of words (chars / 6). The check uses chars.

## 3. The template model

### 3.1 Parts

A line is a list of parts. Each part is a list of tokens: a word, a punctuation mark, or a typed slot.

| Kind | What it is | Hero slot | Example |
|---|---|---|---|
| Frame | The order of the sentences, and the lore slot | never | `{lore}` then the deed sentence |
| Connective | An opener of the deed sentence | never | "Now,", "There,", "In the end," |
| Deed | The clause of the deed | yes, or none for an unnamed part | "{Foe} fell to {hero}." |
| Group | The clause of an order or a people | never | "{group} {grow}" |
| Grow | A verb that a group takes, in the singular and the plural | never | "grows stronger" / "grow stronger" |
| Coda | The level fact | yes, or none | "{Hero} has reached level {level}." |

**Slots** are typed: `lore`, `hero`, `foe`, `killer`, `zone`, `item`, `mount`, `breed`, `group`, `order`, `class`, `count`, `count_num`, `ordinal`, `level`, `quest`, `title`, `npc`, `mark`. The data says which slots each moment allows. A part with a slot that the moment lacks never fits.

**Tags** stop a doubled word. A part can carry tags such as `now`, `end`, `place`, `last`. Two parts of one shape never share a tag. So "Now, $N is level 20 now." and "In the end, that was the end of Hogger." cannot be built.

**Needs** are facts that a part requires: `there`, `zone`, `breed`, `order`, `unbeaten`, `dry`, a foe type, a slot class, or a singular group.

**Unnamed parts** hold no hero slot. The naming rotation of `narrator::naming` decides: an `Unnamed` turn takes only unnamed parts, and any other turn takes parts with exactly one hero slot.

### 3.2 The hero slot

The code fills `{hero}` from the naming of the turn (GAMEPLAY.md 3.2.1): `$N`, "the paladin", "the Forsaken", or a title. The model never sees the naming and never writes the hero.

- `{Hero}` with a capital renders a capital at the start of a sentence: "The paladin".
- A title naming renders "the" and the title: "the Bookworm".
- A title moment never takes the title naming. It takes the name.

### 3.3 Frames and clause order

| Frame | Order | For |
|---|---|---|
| `f.place` | lore | arrivals |
| `f.deed` | lore. [connective] deed. | kill, death, quests, title, slap, mark, mounts, items |
| `f.revenge` | lore. before-clause. turn-clause. | revenge |
| `f.level` | lore. [connective] group grow. coda. | tenth level |
| `f.level_joined` | lore. [connective] group grow, and coda. | tenth level, named coda only |

The lore always comes first: the history, then the deed (GAMEPLAY.md 3.2.1). The order inside the deed varies by part: the hero as subject ("$N defeated Hogger"), or the foe as subject ("Hogger fell to $N"). The voice prefers the foe as subject, so most kill parts keep the foe first.

### 3.4 Groups: race x class, faction-aware

A tenth level and a class quest use groups. The group set of a pairing is built, not listed for each of the 72 pairs:

1. **The class of the faction:** "the {class_pl} of the {faction}". Every pair. The faction comes from the race, as in Classic.
2. **The people:** a phrase for each race, such as "Ironforge" or "the Forsaken".
3. **Named orders:** each order lists the classes, factions, and races that it fits. The Silver Hand fits an Alliance paladin only. So a Horde paladin never gets the Silver Hand as its own order.
4. **Strange pairings:** a phrase for a pairing that the lore finds strange, with a `{people_pl}` slot: "the orcs who wield the Light". A strange pairing gets this phrase, and never a named order of the other faction.

Each group has a number (one or many), so a grow verb takes its singular or plural form. A group with a `short` form ("the order" for the Silver Hand) renders it when the lore already names the group. So "The Silver Hand ... The Silver Hand grows stronger." cannot happen.

### 3.5 Rotation: no repeat within N

Each shape has one **main part**: the deed part, the grow part of a level, or the turn-clause of a revenge. The rotation keys on the main part. So two lines of one character never share a main clause within the last N lines. That implies that no whole shape repeats within N.

- `N = 8`, the length of the naming rotation.
- The code enumerates the shapes of a moment in a fixed order from the data.
- `start = turn mod count`, where `turn` is the call number, as `narrator::naming` uses today.
- The pick walks the shapes from `start`, and takes the first shape that fits and whose main part is not in the window.
- If every fitting shape has a recent main part, the pick takes the fitting shape whose main part was used longest ago.
- If no shape fits, the moment is silence and the log says why. A test proves that this cannot happen with the approved data for any moment that has a lore sentence (section 6).
- **A kill prefers no hero** (`narrator-style.md` 4 and 9). A dead foe is news, and who killed it is mostly not. So on a named turn, the pick of a kill first walks the unnamed kill parts (`k.fallen_u`, `k.dead_u`, `k.lost_u`), and takes a named part only when no unnamed one fits and is fresh. A quest, a title, a level, a mount, and an item still take the naming of their turn.
- The window is the main parts of the last 8 accepted narrator lines of the character, in any moment. It lives in a new column `shape` of the `calls` row of each accepted line. The SQLite store starts fresh, so no migration (memory: "SQLite, fresh start").

### 3.6 The count of shapes

With the draft of section 9 (83 parts):

| Moment | Formula | Shapes |
|---|---|---|
| Kill | 12 deeds x 5 connectives | 60 |
| Death | 4 deeds x 3 connectives x 2 killer phrases | 24 |
| Revenge | 3 before-clauses x 5 turn-clauses, minus 2 with two heroes | 13 |
| Tenth level | 2 frames x 2 connectives x 5 grow verbs x 4 codas, minus tag clashes and unnamed joined codas | 44 |
| Class quest | 5 deeds x 3 connectives, minus 2 | 13 |
| Side quest | 3 deeds x 2 connectives | 6 |
| Title | 2 deeds x 2 connectives | 4 |
| Slap | 3 deeds x 2 connectives | 6 |
| Quest mark | 3 deeds x 1 connective | 3 |
| First mount | 4 deeds x 2 connectives, minus 1 tag clash | 7 |
| First epic mount | 2 deeds x 2 connectives | 4 |
| Epic item, upgrade (each) | 3 deeds x 4 connectives | 12 |
| Arrival | lore alone | 1 |

That is **187 distinct skeletons**, and 209 pairs of moment and shape (some parts serve two moments). A skeleton counts no slot value and no naming. On top:

- A tenth level takes 2 or 3 groups for each pairing, so a pairing has 108 to 162 level forms.
- The naming gives 4 renders of each named shape: name, race, class, title.
- The foe type picks 1 of 5 type-specific kill parts.

Each new part multiplies. One more connective for kills adds 12 shapes. The data grows without code.

### 3.7 Rendering

Rendering turns tokens and values into text, in the story crate:

- One space between words, none before punctuation.
- A capital at the start of each sentence, the lore included.
- "a" or "an" before a mount name from its first letter, with an exception list in the data.
- Counts: "twice", "three times" to "ten times", then "11 times". Ordinals: "second" to "tenth", then "11th", "21st", "22nd", "101st", "111th". `count_num`: "two" to "ten", then digits.
- A quest title and a joke title render in double quotes. A mount and an item render bare.

## 4. The data format

`crates/story/data/narrator_templates.toml`. The story crate reads it with `include_str!` and `toml`, into typed structs with `deny_unknown_fields`. TOML, because `pack_sources.toml` uses it already, and its lists read well.

```toml
# Templates of narrator lines (docs/plans/narrator-templates.md). The user approves every
# part, as with the samples. {Hero} with a capital starts a sentence.

[[connective]]
id = "c.now"
text = "Now,"
tags = ["now"]

[[deed]]
id = "k.fell"
moments = ["kill"]
text = "{Foe} fell to {hero}."

[[deed]]
id = "k.rest"
moments = ["kill"]
text = "{Hero} laid {foe} to rest."
needs = ["undead"]

[[grow]]
id = "v.stronger"
one = "grows stronger"
many = "grow stronger"

[[group]]
id = "o.silver_hand"
text = "the Silver Hand"
short = "the order"
number = "one"
anchors = ["Silver Hand"]
classes = ["paladin"]
factions = ["alliance"]

[[group]]
id = "s.light"
text = "the {people_pl} who wield the Light"
number = "many"
anchors = ["Light", "paladin"]
classes = ["paladin"]
factions = ["horde"]
not_races = ["forsaken"]
```

**Loading.** A `LazyLock` parses the file once. The loader then checks every part: a known slot, a slot that each moment of the part allows, known tags and needs. Then it tokenizes each part into words, builds the shapes of each moment, and calls `table_ok` of `crates/rules` (section 5). A failure makes the narrator silent and logs the reason. The test `the_templates_load` fails CI first, so a bad file never ships.

**Approval.** Templates are data, approved by the user like samples. A new flag `--templates` of `timeways-narrator-review` prints every shape of a pairing with fixed values, so the user reviews them in one list.

## 5. Proofs (Lean)

### 5.1 What goes in `crates/rules`

A new module `crates/rules/src/narrator_shapes.rs`, in loop style (lib.rs: no closure, no iterator adapter, walk by index):

- `Token`: `Word(u16)` (an id of a word or a mark), `Slot(Slot)`.
- `Part { kind: PartKind, tokens: Vec<Token>, tags: u32, needs: u32, main: bool }`.
- `fits(table, shape, facts) -> bool`: the needs hold, no two tags clash, and the hero count matches the naming.
- `assemble(table, shape, facts) -> Option<Vec<Token>>`: the tokens of the parts in order, or `None` if a slot has no value.
- `assemble_arrival() -> Vec<Token>`: exactly `[Slot(Lore)]`. An arrival never reads the table.
- `pick(fits: &[bool], mains: &[u16], recent: &[u16], turn: u64) -> Option<usize>`.
- `table_ok(table, shapes) -> bool` and `distinct_skeletons(table, shapes) -> bool`: checks of the data, run at load.

Strings stay out. Aeneas translates `String` comparison (`aliases.rs`), but not formatting, case, or splitting. So the rules crate works on token ids, and the story crate renders text. The render has property tests, not proofs.

### 5.2 Theorems

| # | Theorem | In plain words |
|---|---|---|
| 1 | `assemble.spec`, `pick.spec`, `table_ok.spec` | Assembly, the pick, and the checks never panic, never overflow, and always end. |
| 2 | `a_line_is_one_shape` | With distinct skeletons, two shapes that build the same tokens are the same shape. So each built line is an instance of exactly one template. |
| 3 | `the_lore_comes_first` | Every built line starts with the lore slot, and holds it once. |
| 4 | `the_hero_stands_only_in_a_deed` | Each hero slot of a built line comes from a deed part or a coda. |
| 5 | `an_arrival_holds_no_hero` | The line of an arrival is the lore slot alone. |
| 6 | `no_group_clause_holds_the_hero` | No group part and no grow part holds a hero slot. |
| 7 | `nothing_is_inside_the_hero` | No word of the list "in", "inside", "within", "through", "into" stands right before a hero slot. |
| 8 | `the_hero_is_named_at_most_once` | A built line holds at most one hero slot, and none on an unnamed turn. |
| 9 | `every_slot_has_a_value` | Each slot of a built line has a value in the facts. So no "{foe}" ever shows. |
| 10 | `a_pick_fits` | The pick returns a shape that fits, and returns one whenever a shape fits. |
| 11 | `no_main_repeats_within_n` | If a fitting shape has a main part outside the window, the pick takes such a shape. |
| 12 | `a_run_never_repeats` | Over a run of picks, each with more than N fitting main parts, two picks at most N apart never share a main part. So no shape repeats within N. |
| 13 | `the_pick_is_deterministic_from_the_turn` | The pick is the first fitting fresh shape from `turn mod count`. |

Sketches, in the style of `HeroHook.lean`:

```lean
namespace timeways_rules.narrator_shapes

@[step] theorem assemble.spec (t : Table) (s : Shape) (f : Facts) :
    assemble t s f ⦃ r => ∀ o, r = some o → o.val = skeleton t s ⦄

theorem a_line_is_one_shape (t : Table) (ss : Slice Shape) (i j : Usize)
    (f : Facts) (o : alloc.vec.Vec Token)
    (hd : distinct_skeletons t ss = ok true)
    (hi : assemble t ss[i] f = ok (some o)) (hj : assemble t ss[j] f = ok (some o)) :
    i = j

theorem the_lore_comes_first (t ss s f o) (hok : table_ok t ss = ok true)
    (hs : s ∈ ss) (h : assemble t s f = ok (some o)) :
    o.val.head? = some (Token.Slot Slot.Lore) ∧ o.val.count (Token.Slot Slot.Lore) = 1

theorem the_hero_stands_only_in_a_deed (t ss s f o) (hok : table_ok t ss = ok true)
    (hs : s ∈ ss) (h : assemble t s f = ok (some o)) :
    ∀ i, o.val[i]? = some (Token.Slot Slot.Hero) →
      kind_at t s i = PartKind.Deed ∨ kind_at t s i = PartKind.Coda

theorem an_arrival_holds_no_hero :
    assemble_arrival ⦃ o => o.val = [Token.Slot Slot.Lore] ⦄

theorem no_group_clause_holds_the_hero (t ss) (hok : table_ok t ss = ok true) :
    ∀ p ∈ t.parts.val, p.kind = PartKind.Group ∨ p.kind = PartKind.Grow →
      Token.Slot Slot.Hero ∉ p.tokens.val

theorem nothing_is_inside_the_hero (t ss s f o) (hok : table_ok t ss = ok true)
    (hs : s ∈ ss) (h : assemble t s f = ok (some o)) :
    ∀ i, o.val[i + 1]? = some (Token.Slot Slot.Hero) →
      ∀ w, o.val[i]? = some (Token.Word w) → w ∉ t.inside_words.val

theorem the_hero_is_named_at_most_once (t s f o) (hf : fits t s f = ok true)
    (h : assemble t s f = ok (some o)) :
    o.val.count (Token.Slot Slot.Hero) = (if f.named then 1 else 0)

theorem every_slot_has_a_value (t s f o) (h : assemble t s f = ok (some o)) :
    ∀ k, Token.Slot k ∈ o.val → f.has k

theorem a_pick_fits (fits : Slice Bool) (mains recent : Slice U16) (turn : U64) :
    pick fits mains recent turn ⦃ r =>
      (r.isSome ↔ ∃ i, fits.val[i]? = some true) ∧
      ∀ i, r = some i → fits.val[i.val]? = some true ⦄

theorem no_main_repeats_within_n (fits mains recent turn) (i : Usize)
    (hfresh : ∃ k, fits.val[k]? = some true ∧ mains.val[k]! ∉ recent.val)
    (h : pick fits mains recent turn = ok (some i)) :
    mains.val[i.val]! ∉ recent.val

theorem a_run_never_repeats (run : List PickInput) (outs : List Usize)
    (h : picks run = ok outs)
    (hwide : ∀ step ∈ run, N < fitting_mains step) :
    ∀ a b, a < b → b - a ≤ N → main_of run outs a ≠ main_of run outs b
```

**Provability.** Every function is a loop over vectors of small enums and `u16` ids, like `chapters.rs`. Theorems 3 to 9 follow from `table_ok` by an invariant of the assembly loop: the output is the concatenation of the parts so far. Theorem 12 is an induction over the run, with the window as the last N picks. Theorem 2 needs a lemma that the token equality loop is equality. None needs a string. Proofs are Aeneas and Lean only.

**Outside Lean.** The render, the TOML loader, the JSON parser, and the lore checks read text, so property tests and fuzz targets cover them. The skeletons are distinct as tokens. Distinct as strings, with real values, is a property test (`two_shapes_never_render_one_line`).

## 6. Tests

### 6.1 Property tests (`crates/story/tests/properties.rs`)

- `every_shape_renders_a_line_that_passes_every_check`. For every shape of every moment, every naming, and edge values, render the line with a fixed good lore sentence of the budget's length, and run `checked_line`. Edge values, each drawn often:
  - the longest names: a 64-char NPC name, an item name, a mount name, a quest title, a joke title;
  - `$N`, the longest title, and a title that starts with a vowel;
  - all 72 race x class pairs, both factions, with each group of each pair;
  - counts and levels: 2, 3, 10, 11, 12, 13, 21, 22, 60, 101, 111, `i64::MAX`;
  - names with an apostrophe or a hyphen ("Gath'Ilzogg", "Mor'Ladim").
  The line passes, fits 300 chars and 1000 bytes, and names the hero as the naming says.
- `no_rendered_line_holds_an_arrival_of_the_hero`: `arrival_in` finds nothing in any rendered line.
- `a_group_never_holds_a_hero_word`: no group render of any pair holds `$N`, "hero", "stranger", or the singular race or class word of the hero.
- `two_shapes_never_render_one_line`: with fixed values, distinct shapes give distinct strings.
- `no_main_part_repeats_within_eight_lines`: a random run of moments, namings, and choices.
- `every_moment_has_a_fitting_shape`: for any moment, any naming, and any valid choices, at least one shape fits. Silence comes only from the model.

### 6.2 Fuzz (`fuzz/fuzz_targets/narrator_slots.rs`)

The target parses any bytes as an answer, with any offered choices, and checks it. It never panics. Seeds in `fuzz/seeds/narrator_slots/`:

- one valid answer for each moment;
- `{"lore": "SILENCE"}` and a bare `SILENCE`;
- the answer inside a code fence, and inside two fences;
- an unknown field, a missing field, a duplicate key, a null, a number for a string, `"true"` for `true`;
- a value outside the offered list, and a value in another case ("Silver_Hand");
- `$N`, a newline, a fence mark, an emoji, and 5000 chars in `lore`;
- nested JSON in `lore`, two objects, trailing text, invalid UTF-8.

### 6.3 Unit tests

- `the_templates_load`
- `every_template_part_passes_the_line_checks`
- `an_arrival_answer_becomes_its_lore_alone`
- `the_model_never_names_the_hero` (a `$N` in the lore is a fault)
- `a_choice_outside_the_list_is_refused`
- `there_needs_the_lore_to_name_the_zone`
- `breed_needs_the_lore_to_name_the_breed`
- `leads_needs_the_lore_to_name_the_group`
- `a_kill_prefers_an_unnamed_part_on_a_named_turn`
- `every_assembled_example_passes_the_checks_of_the_style_guide`
- `a_group_needs_the_lore_to_name_its_anchor`
- `a_horde_paladin_never_gets_the_silver_hand`
- `a_forsaken_paladin_gets_the_dead_who_wield_the_light`
- `a_group_takes_its_short_form_when_the_lore_names_it`
- `a_singular_group_takes_a_singular_verb`
- `now_never_shows_twice_in_a_line`
- `an_unnamed_turn_builds_no_hero`
- `a_title_moment_never_names_the_hero_by_the_title`
- `a_dry_tone_with_no_dry_shape_takes_a_plain_one`
- `the_lore_budget_leaves_room_for_the_longest_shape`
- `counts_and_ordinals_render_in_words_to_ten`
- `a_sample_answer_builds_its_sample_line`
- `the_shape_of_a_line_is_stored_with_its_call`

## 7. The local model

**Grammar-constrained decoding helps, for small models.** Each local server can force the JSON:

| Server | How |
|---|---|
| Ollama | `format` with a JSON schema |
| LM Studio | `response_format` with `json_schema` |
| llama.cpp server | `json_schema`, or a GBNF `grammar` |

- **The gain:** the answer always parses, the fields come in order (`lore` first), and each closed field is a valid value. Format faults are the most common faults of a 2 to 3 GB model, and each one costs a retry.
- **The limit:** a grammar cannot check meaning. It cannot know that the lore names the zone. The checks of section 2 stay.
- **The risk:** a grammar can push a small model into worse words when it blocks its first choice. So the schema constrains only the shape and the enums. `lore` is a plain string. Its `maxLength` is the budget plus 50, so the model ends its sentence, and the char check refuses an overrun.
- **Agents** (Claude, Codex, Gemini): no grammar. They follow a JSON format well. The parser stays strict.

The schema is built for each call, because the offered values differ: a level offers the groups of the pairing.

```json
{"type": "object",
 "properties": {
   "lore": {"type": "string", "maxLength": 230},
   "group": {"enum": ["o.silver_hand", "g.class_faction", "g.people"]}},
 "required": ["lore", "group"],
 "additionalProperties": false}
```

**A relay request, not made.** The relay owns model calls (Gnomish Relay SPEC 9.7 and 9.8). The request, for later: an optional `schema` field (at most 4 KB) on the model call of the app protocol. The bridge passes it as Ollama `format`, LM Studio `response_format`, or llama.cpp `json_schema`, and ignores it for an agent. The story program never depends on it: without it, the parse and the checks are the same. The main session decides when to send it.

## 8. Migration

### 8.1 The samples

`crates/story/data/samples/narrator_lines.txt` changes its pairs. A sample shows the moment, the lore, the offered choices, the answer JSON, and the built line. A new `shape:` field fixes the shape, so the test `a_sample_answer_builds_its_sample_line` can rebuild the line.

- **The 12 arrival samples** keep their line as `lore`. Nothing else changes.
- **The 14 deed samples** split: the first sentence becomes `lore`, and a template builds the rest. Examples:

| Old line | New `lore` | Built deed |
|---|---|---|
| "For years Hogger led the Riverpaw gnolls in raids on the farms of Elwynn. The Riverpaw have no leader now." | the first sentence | "The Riverpaw have lost their leader." |
| "The murlocs of the coast have no kingdom and no history. They have now killed $N three times." | the first sentence | "One Murloc Coastrunner or another has now killed $N three times." |
| "Ironforge's Mountaineers hold the passes of Khaz Modan on ramback. Now the dwarf has a ram of their own." | the first sentence, with "rams" for "ramback" | "One such ram now carries the dwarf." |

The contrast endings ("The paladin did.", "The rogue settled the account instead.") are gone already: the style guide replaced those samples on 2026-10-05 with lines that end on the world (question 9).

### 8.2 The prompt

- **The task** of a deed: "Write one sentence of history about the moment, from the lore, at most N words. Do not tell the deed, and do not name the hero: the game adds both." An arrival keeps its task, with "at most 2 sentences".
- **The choices**: a short section that lists each field, its values, and one line of meaning (2.2).
- **The answer**: "Answer with JSON only", and one example of the moment kind.
- **Gone**: "Name the hero: ...", "$N stands for the name of the hero", and the hero line ("The hero: a Forsaken warlock"), except for a tenth level and a class quest, where the pairing picks the groups.
- **The note** at the end: "Remember: one sentence of history, from the lore. Add nothing. When the lore gives you nothing true, answer {\"lore\": \"SILENCE\"}."
- The size test of 3.2.1 measures the new prompts, with the longest choices list.

### 8.3 The checks

`line_check.rs` splits in two:

- **Checks of the lore sentence:** grounded, copy of a sample, new numbers, `$N`, the count of sentences, the budget, and the choice checks. The copy check reads only the lore, because every line now shares template words with the samples.
- **Checks of the built line:** length, bytes, cutoff, emoji, banned words, slop, brackets, at most one `$N`, no hero at a place, and no arrival of the hero. They never fail on template text, by the tests of section 6. They stay as a second guard.

New faults: `BadAnswer`, `UnknownChoice(field)`, `ChoiceNotInLore(field)`, `TooManySentences`, `HeroInHistory`, `OverBudget`.

### 8.4 Order of the build

1. The rules module and its Lean theorems.
2. The TOML data, the loader, the render, and the property tests.
3. The new moment fields of 2.3.
4. The JSON parser, its fuzz target, and the split checks.
5. The prompt and the samples.
6. The `shape` column and the window.
7. GAMEPLAY.md 3.2 and 3.2.1 change with the build: the naming moves out of the prompt, and the samples become answers.

## 9. A first draft of templates, for approval

83 parts. The text of the parts is in the narrator's voice, and follows the style guide.

### 9.1 Frames (5)

`f.place`, `f.deed`, `f.revenge`, `f.level`, `f.level_joined` (3.3).

### 9.2 Connectives (6)

| Id | Text | Tags | Needs |
|---|---|---|---|
| `c.none` | (nothing) | | |
| `c.now` | Now, | now | |
| `c.there` | There, | place | there |
| `c.in_zone` | In {zone}, | place | zone |
| `c.in_the_end` | In the end, | end | |
| `c.at_last` | At last, | last | |

### 9.3 Deeds (37)

**Kill (12).** "Named" parts hold the hero. The unnamed parts tell what changed, and a kill prefers them (3.5).

| Id | Text | Needs |
|---|---|---|
| `k.fell` | {Foe} fell to {hero}. | |
| `k.defeated` | {Hero} defeated {foe}. | |
| `k.brought` | {Hero} brought {foe} down. | |
| `k.fallen` | {Foe} has fallen to {hero}. | |
| `k.hunted` | {Hero} hunted {foe} down. | beast |
| `k.rest` | {Hero} laid {foe} to rest. | undead |
| `k.dragon` | {Hero} brought down the dragon {foe}. | dragonkin |
| `k.broke` | {Hero} broke {foe} apart. | elemental |
| `k.demon` | {Hero} slew the demon {foe}. | demon |
| `k.fallen_u` | {Foe} has fallen. | |
| `k.dead_u` | {Foe} is dead. | |
| `k.lost_u` | One: {Led} has lost its leader. Many: {Led} have lost their leader. | leads |

**Death (4)**, with a killer phrase (2): `kr.one` "{killer}", `kr.kind` "one {killer} or another".

| Id | Text | Needs |
|---|---|---|
| `d.killed` | {Killer} has now killed {hero} {count}. (tag: now) | |
| `d.died` | {Hero} has now died to {killer} {count}. (tag: now) | |
| `d.ordinal` | For the {ordinal} time, {killer} killed {hero}. | |
| `d.won_u` | {Killer} has won this fight {count}. | |

**Revenge (8):** a before-clause, then a turn-clause.

| Id | Text | Needs |
|---|---|---|
| `r.had_killed` | {Foe} had killed {hero} {count}. | |
| `r.had_died` | {Hero} had died to {foe} {count}. | |
| `r.had_won_u` | {Foe} had won {count}. | |
| `rb.this_time` | This time, the fight went the other way. | |
| `rb.ordinal` | The {ordinal} fight went the other way. | |
| `rb.at_last` | At last, {foe} fell. | |
| `rb.dry` | {Foe} did not win the {ordinal}. | dry |
| `rb.in_the_end` | In the end, {foe} fell to {hero}. | (only after `r.had_won_u`) |

**Quests (6):** `q.*` serve the side quest and the class quest. `cq.*` serve the class quest.

| Id | Text | Needs |
|---|---|---|
| `q.finished` | {Hero} has finished {quest}. | |
| `q.saw` | {Hero} saw {quest} through. | |
| `q.done_u` | {Quest} is finished. | |
| `cq.for` | {Hero} finished {quest} for {order}. | order |
| `cq.for_front` | For {order}, {hero} finished {quest}. (no connective) | order |
| `cq.work_u` | The work of {quest} is done. | |

**Title (2):** `t.earned` "{Hero} has earned the title {title}.", `t.goes` "The title {title} goes to {hero}."

**Slap (3):** `s.slapped` "{Hero} has now slapped {npc} {count}.", `s.been_u` "{Npc} has now been slapped {count}.", `s.makes_u` "That makes {count_num} slaps for {npc}." (dry)

**Quest mark (3):** `m.carries` "{Hero} still carries {mark} from {quest}.", `m.left` "{Quest} left {mark} on {hero}.", `m.lingers_u` "{Mark} lingers after {quest}."

**Mounts (4):** `mt.*` serve the first mount. `mt.rides` and `mt.carries` serve the epic mount too.

| Id | Text | Needs |
|---|---|---|
| `mt.rides` | {Hero} rides {mount_a}. | |
| `mt.carries` | {Mount_a} carries {hero}. | |
| `mt.such` | One such {breed} now carries {hero}. | breed |
| `mt.such_u` | One such {breed} has a new rider. | breed |

**Items (3):** for the first epic item and a big upgrade. No item part is unnamed: an item never acts like a person, so an unnamed turn of an item takes the name.

| Id | Text | Needs |
|---|---|---|
| `i.in_hand` | {Item} is in the hand of {hero}. | weapon |
| `i.carries` | {Hero} carries {item}. | weapon |
| `i.wears` | {Hero} wears {item}. | worn |

### 9.4 Grow verbs (5)

| Id | One | Many |
|---|---|---|
| `v.stronger` | grows stronger | grow stronger |
| `v.strength` | gains strength | gain strength |
| `v.now` | is stronger now (tag: now) | are stronger now |
| `v.little` | stands a little stronger | stand a little stronger |
| `v.grown` | has grown stronger | have grown stronger |

### 9.5 Codas (4)

| Id | Alone | Joined (after "and") |
|---|---|---|
| `co.reached` | {Hero} has reached level {level}. | {hero} has reached level {level}. |
| `co.is_now` | {Hero} is level {level} now. (tag: now) | {hero} is level {level} now. |
| `co.makes_u` | That makes level {level}. | (none) |
| `co.none_u` | (nothing) | (none) |

### 9.6 Groups (16)

| Id | Text | Number | Fits |
|---|---|---|---|
| `g.class_faction` | the {class_pl} of the {faction} | many | every pair |
| `g.people` | the race phrase: Stormwind, Ironforge, the gnomes of Gnomeregan, the night elves, the orcs of Durotar, the Forsaken, the tauren of Mulgore, the Darkspear | per race | every pair |
| `o.silver_hand` | the Silver Hand (short: the order) | one | Alliance paladin |
| `o.church` | the Church of the Holy Light (short: the Church) | one | human, dwarf, or gnome priest |
| `o.kirin_tor` | the Kirin Tor (anchors: Kirin Tor, Dalaran) | one | Alliance mage |
| `o.cenarion` | the Cenarion Circle (short: the Circle) | one | every druid |
| `o.earthen_ring` | the Earthen Ring | one | Horde shaman (question 5) |
| `o.shattered_hand` | the Shattered Hand | one | Horde rogue (question 5) |
| `o.si7` | SI:7 | one | Alliance rogue |
| `o.ravenholdt` | the rogues of Ravenholdt | many | every rogue |
| `o.elune` | the priests of Elune | many | night elf priest |
| `s.dead_light` | the dead who wield the Light | many | Forsaken paladin |
| `s.light` | the {people_pl} who wield the Light | many | paladin of orc, tauren, troll, night elf, or gnome |
| `s.elements` | the {people_pl} who speak to the elements | many | shaman of an Alliance race |
| `s.demons` | the {people_pl} who bind demons | many | warlock of a dwarf, night elf, tauren, or troll |
| `s.circle` | the {people_pl} who walk with the Circle | many | druid of a race other than night elf or tauren |

The groups of the pairs of section 9.7:

| Pair | Faction | Groups |
|---|---|---|
| Human paladin | Alliance | the paladins of the Alliance, Stormwind, the Silver Hand |
| Orc paladin | Horde | the paladins of the Horde, the orcs of Durotar, the orcs who wield the Light |
| Forsaken paladin | Horde | the paladins of the Horde, the Forsaken, the dead who wield the Light |
| Tauren druid | Horde | the druids of the Horde, the tauren of Mulgore, the Cenarion Circle |
| Orc warlock | Horde | the warlocks of the Horde, the orcs of Durotar |
| Dwarf shaman | Alliance | the shaman of the Alliance, Ironforge, the dwarves who speak to the elements |
| Night elf priest | Alliance | the priests of the Alliance, the night elves, the priests of Elune |
| Gnome mage | Alliance | the mages of the Alliance, the gnomes of Gnomeregan, the Kirin Tor |

A Horde paladin and an Alliance paladin differ by construction: the Silver Hand fits only the Alliance. The orc's lore can tell of the Silver Hand as a foe of the Horde, and the group stays "the orcs who wield the Light".

### 9.7 Assembled examples

The lore sentences come from the model, so these show the voice that the samples teach. Each names its shape.

1. Human paladin, level 30. `f.level`, `o.silver_hand` (short), `v.stronger`, `co.reached`, name.
   "Uther the Lightbringer founded the Silver Hand in the Second War, and its paladins still carry the Light against the Scourge. The order grows stronger. $N has reached level 30."
2. Orc paladin, level 10. `f.level_joined`, `c.now`, `s.light`, `v.strength`, `co.reached`, class.
   "The Silver Hand rode against the Horde of Orgrim Doomhammer in the Second War, and Thrall leads a new Horde from Orgrimmar today. Now, the orcs who wield the Light gain strength, and the paladin has reached level 10."
3. Forsaken paladin, level 20. `f.level`, `s.dead_light`, `v.grown`, `co.is_now`, name.
   "The Silver Hand fought the Scourge as it raised the dead of Lordaeron, and the Forsaken hold those lands now. The dead who wield the Light have grown stronger. $N is level 20 now."
4. Tauren druid, level 20. `f.level`, `o.cenarion` (short), `v.stronger`, `co.reached`, class.
   "The Cenarion Circle keeps the balance of nature from Moonglade, where tauren and night elf druids study side by side. The Circle grows stronger. The druid has reached level 20."
5. Orc warlock, level 40. `f.level`, `g.class_faction`, `v.now`, `co.none_u`, unnamed.
   "The Shadow Council taught the orcs to bargain with demons on Draenor, and the Burning Blade still serves those demons in Ragefire Chasm. The warlocks of the Horde are stronger now."
6. Dwarf shaman, level 30. `f.level_joined`, `s.elements`, `v.grown`, `co.reached`, name.
   "In the vaults of Uldaman, the dwarves learned that the titans shaped their forefathers from living stone, and the Explorers' League still digs there. The dwarves who speak to the elements have grown stronger, and $N has reached level 30."
7. Night elf priest, level 30. `f.level`, `c.now`, `o.elune`, `v.stronger`, `co.reached`, race.
   "Tyrande Whisperwind leads the night elves as the high priestess of Elune. Now, the priests of Elune grow stronger. The night elf has reached level 30."
8. Gnome mage, level 20. `f.level`, `o.kirin_tor`, `v.grown`, `co.reached`, name.
   "Dalaran lies behind a violet dome in the hills above Hillsbrad, where its magi rebuild their city. The Kirin Tor has grown stronger. $N has reached level 20."
9. Kill, unnamed. `f.deed`, `k.lost_u` (many).
   "For years Hogger led the Riverpaw gnolls in raids on the farms of Elwynn, and Stormwind still has a price on his head. The Riverpaw have lost their leader."
10. Kill, undead. `f.deed`, `k.rest`, name.
    "Baron Silverlaine held Shadowfang Keep until Arugal's worgen overran it, and his ghost still walks its halls. $N laid Baron Silverlaine to rest."
11. Revenge. `f.revenge`, `r.had_killed`, `rb.this_time`, name.
    "Gath'Ilzogg holds Stonewatch Keep for the Blackrock orcs, above the town of Lakeshire. Gath'Ilzogg had killed $N twice. This time, the fight went the other way."
12. Death, a kind. `f.deed`, `kr.kind`, `d.killed`, name.
    "The murlocs of the Westfall coast have no kingdom and no history. One Murloc Coastrunner or another has now killed $N three times."
13. Death, a kind. `f.deed`, `kr.kind`, `d.won_u`, unnamed.
    "The Defias took Westfall farm by farm, and the militia of Sentinel Hill holds little more than its hill now. One Defias Pillager or another has won this fight twice."
14. Class quest, dwarf paladin. `f.deed`, `cq.for` (short), race.
    "Uther the Lightbringer founded the Silver Hand to carry the Light into war, and its paladins still train in Ironforge. The dwarf finished \"The Tome of Divinity\" for the order."
15. First mount. `f.deed`, `mt.such`, race.
    "The Mountaineers of Ironforge patrol the passes of Khaz Modan on rams bred in the snows of Dun Morogh. One such ram now carries the dwarf."
16. First epic mount. `f.deed`, `c.now`, `mt.rides`, name.
    "The warhorses of Lordaeron died of the plague, and the Royal Apothecary Society raises them again as mounts for the Forsaken. Now, $N rides a Green Skeletal Warhorse."
17. First epic item. `f.deed`, `c.there`, `i.carries`, class.
    "The summoning of Ragnaros blackened Searing Gorge, and the Dark Iron dwarves work its mines with slaves. There, the rogue carries Gutwrencher."
18. Big upgrade. `f.deed`, `c.in_zone`, `i.carries`, title.
    "Gryan Stoutmantle left the Third War to save the farms of Westfall, and his militia holds Sentinel Hill. In Westfall, the Bookworm carries Cruel Barb."
19. Slap. `f.deed`, `s.slapped`, race.
    "Every caravan between Orgrimmar and Ratchet stops at the Crossroads. The tauren has now slapped Innkeeper Boorand Plainswind twice."
20. Arrival. `f.place`.
    "Thrall named Durotar for a father he never knew. The orcs who spent years in human camps hold its red canyons now."

## 10. Open questions

1. **The verb.** The spec lets the code pick the verb by rotation, and the model only picks facts about the lore. Do you want the model to pick the verb too? Then the rotation loses control of repeats.
2. **Titles.** A joke title has no lore today. Use the lore of the zone where it came (a `zone` on `Titled`), or keep titles as a deed sentence alone, or silence?
3. **Flavor moments.** Keep them as free text with checks, or give them templates too?
4. **Items.** `item-stories.md` waits. Until it is built, does a big upgrade keep a templated line, or go silent?
5. **The cutoff of orders.** Are the Earthen Ring and the Shattered Hand known in 25 ADP? Without them, a Horde shaman and a Horde rogue keep the class and people groups.
6. **`v.ranks`.** "The Kirin Tor counts one more mage in its ranks." Is it too close to a fame claim? **Answered** (2026-10-05, `narrator-style.md` 9): yes, a fame claim. `v.ranks` is dropped.
7. **The window.** Is N = 8 right? A smaller N gives more freedom, and a larger N more variety.
8. **Unnamed level codas.** Keep "That makes level 40.", or leave the level out of every unnamed line?
9. **Contrast endings.** "The paladin did." needs the lore to end on a failure. Add a closed choice `contrast` with a check, or drop the shape? The style guide dropped the two contrast samples (2026-10-05), so no part needs it.
10. **New phrases.** Do you approve "one {killer} or another" and the four strange-pairing phrases? ("leads, two to none" is dropped by the style guide.)
11. **Revenge and the side quest.** Both need new moments. What rank does each take in `moments.rs`?
12. **The relay schema.** Send the `schema` request to the relay session now, or after the first build with plain JSON?
