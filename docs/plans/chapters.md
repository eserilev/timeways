# Plan: where a chapter of the chronicle ends

Status: spec only, 2026-10-05. Nothing is built. The user approved the direction in principle: weight, natural breaks, a minimum and a maximum, a pure fold, and a zone history in the atlas. This plan refines it. When a part is built, its rules move into `GAMEPLAY.md` 3.3, and this plan marks the part as done.

Related plans: `docs/plans/readable-chronicle.md` (the Chronicle book and the Knowledge atlas), `docs/plans/hero-stories.md` 3.5 and 3.6 (the summary, and the migration notes).

## 1. Goal

A chapter is one stretch of the story that a reader can name: "The Deadmines", "Return to Tirisfal Glades", "The road to level 20". The code cuts the history into chapters. A model never does.

The new rule has four aims:

1. A flight, a walk, or a hearthstone makes no chapter.
2. A long grind in one zone still breaks into chapters.
3. A return to an old zone can open its own chapter.
4. The same thing done again, such as the weekly raid, never makes a new chapter (section 7).

## 2. Problems of the rule of today

The rule of today (`crates/story/src/chapters.rs`, `journal.rs`, `GAMEPLAY.md` 3.3): a later chapter begins at a milestone, which is the first visit of a zone, every tenth level, a first kill of a rare or a boss, or a finished class quest. A milestone after less than 45 minutes of play joins the chapter that runs.

| Problem | Cause |
|---|---|
| A flight across 4 zones makes up to 4 thin chapters. | The first visit of a zone is a milestone, also from the air. Only the 45 minutes stop it. |
| A long grind in one zone never breaks. | Quests, deaths, and levels other than every tenth are no milestones. |
| A return to an old zone means nothing. | Only a first visit counts. |
| A chapter is named by a zone that you only flew over. | `zones` holds every first visit in the chapter. |
| The cut depends on the clock of play (45 minutes, 30 minutes of quiet). | A slow player and a fast player get different stories for the same deeds. |
| A change of the rule moves the beginnings, and an old saga loses its chapter. | A saga is stored by the tick where its chapter begins (3.3, 5.7). |

## 3. The rule in short

1. Each event of note adds **weight**, but only the first time its **key** happens. A repeat weighs 0.
2. A **break** is a natural pause: settling in a new zone, a return to a zone, every tenth level, leaving a dungeon or a city, and 8 hours away.
3. A chapter closes at the first break after its weight reaches **MIN** (15). The cut goes just before the next event with weight.
4. A chapter also closes once its weight reaches **MAX** (40), with no break.
5. A pure fold over the event log decides this, in `crates/rules`. Lean proves its laws.

## 4. Keys and weight

### 4.1 From an event to a step

The story program walks the history once, oldest first, and turns each Hourglass event into one **step** for the fold. A step has:

- `key`: what the event is about, or none. A key is a kind and an id: `(Kill, <entity of the foe>)`, `(Level, 20)`.
- `zone`: the zone where you stood, from the newest `located_in` of you at that point. A subzone gives its zone.
- `mark`: a break that the history shows, or none (section 5).

The weight is a function of the kind of the key alone. The table lives in `crates/rules`, so a proof can read it.

Only the events up to a step decide that step. The walk never looks ahead. So one more event never changes an earlier step, and the fold stays stable (theorem 2).

### 4.2 The weight table

Every event of note is a fact of you in `vocabulary.rs`. The first guess of the user, set against the real facts:

| Event in the log | Fact (`vocabulary.rs`) | Key | Weight |
|---|---|---|---|
| A quest of the game turned in | `FactStart` of `GAME_QUEST_DONE` | `(GameQuest, <thing>)` | 1 |
| A class quest turned in | the same, on a thing with `CLASS_QUEST` | `(ClassQuest, <thing>)` | 3 |
| A side quest of Timeways finished (3.4) | `FactStart` of `QUEST_DONE` | `(SideQuest, <thing>)` | 2 |
| A new subzone, on foot | `FactStart` of `VISITED` to a place with a zone around it | `(Subzone, <place>)` | 1 |
| A level up | `FactUpdate` of `LEVEL` | `(Level, <the new level>)` | 2 |
| A death | `DEATHS` of you | `(DeathBy, <killer>)`, or `(DeathIn, <zone>)` with no known killer | 2 for the first, 1 for the second, then 0. Always 0 once you beat the killer (7.2). |
| A kill of a rare or a boss | `DEFEATED`, from you to the foe | `(Kill, <foe>)` | 3, and 2 more for revenge on a foe that killed you before (7.2) |
| A kill of a raid boss | the same, in a zone with `RAID` | `(RaidKill, <foe>)` | 5, and 2 more for revenge |
| The first entry into a dungeon | `FactStart` of `DUNGEON` on a zone | `(Dungeon, <zone>)` | 3 |
| The first entry into a raid | `FactStart` of `RAID` on a zone | `(Raid, <zone>)` | 5 |
| A lasting mark of a quest | `FactStart` of `MARKED_BY` | `(Mark, <thing>)` | 1 |
| A joke title (5.4.1) | `FactStart` of `TITLE` | `(Title, <thing>)` | 1 |
| A dungeon cleared | none yet | `(Cleared, <zone>)` | 5, later (5.2) |

So the largest weight of one step, `W_MAX`, is 7: the first kill of a raid boss that killed you before. The most that one key can give in all, `CAP_MAX`, is 7 too (7.4).

**Weight 0, with no key:** a new zone (`VISITED` of a zone, also a capital), walking (`LOCATED_IN`), a new entity, `SEEN`, `MET`, `HOSTILE`, `ANIMAL`, `TRUSTS`, `KNOWS_LORE`, `SLAPPED`, `QUEST_OFFERED`, `QUEST_ACCEPTED`, `GAME_QUEST_TAKEN`, `RACE`, `CLASS`, `MEMBER_OF`, `LEADER_OF`, `ON_MAP`, `MAP_X`, `MAP_Y`, every `FactEnd`, the first `LEVEL` at the first login, and the `DEFEATED` from a killer to you (the death carries it). `DEAD` is 0 too: it marks an NPC of your own story, and the side quest that killed it carries the weight. `NEMESIS` waits for 4.1.

Notes on the table:

- **A dungeon cleared** has no event. The game sends `ENCOUNTER_END` for each boss, but no "last boss" mark. Until 5.2 is built, the entry (3) and the boss kills (3 each) stand in for it. A dungeon with 3 bosses gives 12.
- **A subzone counts only on foot.** In a flight, the game also sends a new subzone, and `VISITED` is an up flag, so a subzone that you fly over never counts later. Until the taxi flag of 5.2 exists, a subzone gets a key only after an event with a key of another kind in the same zone, in the same stay. A stay is the time from the entry into a zone until you leave it. So a flight over a new zone weighs 0.
- **The rough size.** An hour of quests gives about 8 quests, 1 level, 3 subzones, and maybe a death: 15. So a chapter is one to three hours of new play.

## 5. Breaks

### 5.1 Each break, and what it needs

A break never cuts at once. It waits, **pending**, until the next step with weight. Then the fold cuts just before that step, if the open chapter has MIN or more. The step with weight, not the break, begins the new chapter. So a chapter never begins with a flight, and its first zone is a zone where something happened.

A break comes in two places. A mark comes from the walk of the story program. Settling and returning come from the fold, because only the fold knows what is new.

| Break | Where it comes from | Detected today? | What it needs |
|---|---|---|---|
| **Settling in a new zone** | The fold: the first step with weight in a zone where no step had weight before | Yes | Nothing new. A flight adds no weight, so it never settles. |
| **A return to a zone** | The fold: a step with weight in a settled zone, after time away. Away means that the zone got no weight in the open chapter, nor in the one before (`RETURN_AWAY_CHAPTERS` = 2). | Yes | Nothing new. |
| **Every tenth level** | Mark `BreakAfter` on the `LEVEL` update to 10, 20, ... | Yes | Nothing new. The level ends its chapter, as a climax. |
| **Leaving a dungeon or a raid** | Mark `BreakBefore` on the `LOCATED_IN` that leaves a zone with `DUNGEON` or `RAID` | Yes | Nothing new. The walk keeps the set of instance zones that it saw so far, never the final world. |
| **Leaving a capital** | Mark `BreakBefore` on the `LOCATED_IN` that leaves one of the six capitals (`places.rs`) | Yes | Nothing new. |
| **8 hours away** | Mark `BreakBefore` on the first event 8 hours or more after the event before it (`AWAY_SECONDS`) | Yes | Nothing new. The gap between two ticks shows it. It also catches a long AFK, which is a break too. The addon cannot send `PLAYER_LOGOUT` (5.4), and it does not need to. |
| **Resting at an inn** | Mark `BreakBefore` on the end of `resting` | **No** | A new event (below). |
| **A dungeon cleared** | Would add weight, not a break | **No** | New data (below). |

### 5.2 The new events

**Rest at an inn.** The addon watches `PLAYER_UPDATE_RESTING` and reads `IsResting()`. A change sends a new line `rest_changed {at, resting: bool}`. The story program holds a new fact `resting` (flag, free, solo) on you: a `FactStart` when rest begins, a `FactEnd` when it ends. The end is the break: the chapter closes as you walk out of the inn. A city is a rest area too, so the capital rule then stays only as a fallback. Needs:

- the API gate of Gnomish Relay for `PLAYER_UPDATE_RESTING` and `IsResting` (5.4);
- a new name in `vocabulary.rs`, so the next version of the vocabulary and a migration note (section 11);
- a step in the game: does a login at an inn fire the event, and does a flight from an inn end the rest at once?

**A dungeon cleared.** `ENCOUNTER_END` gives an `encounterID`. A data file `crates/story/data/dungeon_ends.toml` would name the last encounter of each dungeon of WoW Forever, and `npc_defeated` would carry `encounter`. The kill of the last encounter adds `(Cleared, <zone>)`, weight 5. Needs the data source (open question 5) and the API gate.

**Optional: the taxi flag.** `zone_entered` gets `taxi: bool` from `UnitOnTaxi("player")`. A subzone seen from a flight then gets no key, and it counts later on foot. This replaces the stay rule of 4.2. Needs the API gate.

## 6. The fold

### 6.1 Constants

All in `crates/rules/src/chapters.rs`. Play tests tune them.

| Name | First value | Meaning |
|---|---|---|
| `MIN_WEIGHT` | 15 | A break closes a chapter only from this weight on. |
| `MAX_WEIGHT` | 40 | A chapter closes at this weight, with no break. |
| `W_MAX` | 7 | The largest weight of one step. A test checks it against the table. |
| `DEATH_WEIGHTS` | [2, 1] | The weight of the first and the second death to a foe that you have not beaten. Every later death weighs 0. |
| `REVENGE_WEIGHT` | 2 | The extra weight of the first kill of a foe that killed you before. |
| `RETURN_AWAY_CHAPTERS` | 2 | A zone with no weight in this many chapters, the open one included, is "away". |
| `AWAY_SECONDS` (story crate) | 8 hours | A gap this long is a break. |

`MIN_WEIGHT` ≥ 1 and `MAX_WEIGHT` ≥ `MIN_WEIGHT`. A const assert holds both, and the proofs use both.

### 6.2 One step

The state holds the closed chapters, the open chapter (its first step, its weight, what opened it), the seen keys, the settled zones with the newest chapter that gave each one weight, the record of each killer (the deaths to it so far, which stop counting at 2, and whether you beat it), and the pending break.

For each step, in order:

1. If the step has a `BreakBefore` mark, the break is pending.
2. The **gain** of the step is the weight of its key when the key is not seen yet, and else 0. A death and a kill follow 7.2 instead. The key goes into the seen keys, and a death or a kill updates the record of its killer.
3. If the gain is above 0:
   - A zone with no weight before makes "settling" pending. A settled zone that is away makes "return" pending.
   - If a break is pending and the open chapter has `MIN_WEIGHT` or more, the open chapter closes before this step, and a new one opens at this step.
   - The pending break ends either way: a break before MIN is spent.
   - The zone keeps the number of the open chapter.
4. The gain goes into the open chapter.
5. If the open chapter has `MAX_WEIGHT` or more, it closes after this step. The next step opens a new chapter.
6. If the step has a `BreakAfter` mark, the break is pending.

A step with gain 0 changes only the pending break, the seen keys, and the record of a killer. It never closes a chapter.

### 6.3 What a chapter is

```rust
pub struct Cut {
    pub first: usize,      // index of its first step = position of its first event
    pub end: usize,        // one past its last step
    pub weight: u16,
    pub close: Close,      // Open, Break, Full
    pub opened_by: Opening, // First, NewZone, Return, Level, LeftInstance, LeftCity, Rest, Away, Full
}
```

When two breaks wait at one cut, `opened_by` takes the first of: Return, NewZone, Level, LeftInstance, LeftCity, Rest, Away. A place names a chapter better than a pause.

A chapter is a range of events, by position in the `events` table. Position is the `EventId` (5.7). The key of a chapter becomes the `EventId` of its first event, not its tick: two chapters can begin at one tick, because one input line can make several events. A flavor moment or a hero entry at tick T belongs to the newest chapter whose first event has a tick of T or less.

### 6.4 Where the code lives

- `crates/rules/src/chapters.rs`: the types `Key`, `KeyKind`, `Step`, `Mark`, `Cut`, `Close`, `Opening`, the weight table, the constants, and the fold. Loop style, as `story_shelf.rs`: index loops, no closure, no iterator adapter. Roots for Charon: `start`, `advance`, and `chapters`. `chapters(steps)` is `advance(start(), steps)`.
- `crates/story/src/chapters.rs`: the walk from the history to the steps, and `AWAY_SECONDS`. `sessions`, `chapter_starts`, `MIN_CHAPTER_PLAY_SECONDS`, and `SESSION_GAP_SECONDS` go away.
- `journal.rs`: the chapters come from the cuts. The story program keeps the fold state of the active character in memory, and folds only the new events of each line. Stability (theorem 2) makes this exact. A replay at the open runs the whole fold once.
- The seen keys are a sorted list with a binary search, as an index loop. If Aeneas does not translate `Vec::insert`, they are a plain list with a scan. A world of 50,000 events with 5,000 keys then costs about 10^8 compares at the open: measure it in step 3.

## 7. No chapter bloat

A raider clears the same raid each week. A farmer kills the same rare each day. Neither must get a new chapter each time. **Chapters grow with what is new, never with time played.**

### 7.1 The rule

- **Novel weight only.** A step weighs its table weight only the first time its key happens. `(RaidKill, Ragnaros)` counts once in the whole life of the character. The second kill weighs 0.
- **Breaks need new weight.** A break cuts only at a step with gain, and only from `MIN_WEIGHT` on. Repeats have no gain, so they never meet a break.
- **The forced close counts novel weight only.** So repeats never reach `MAX_WEIGHT` either.
- **A return makes a chapter only with something new.** A return break waits for a step with gain. If the return brings nothing new, it never cuts, and the return folds into the open chapter. A "Return to Tirisfal Glades" chapter needs new weight in Tirisfal Glades.

### 7.2 Deaths and revenge

The user decided on 2026-10-05: once you kill something, dying to it matters less.

**The record of a killer.** The fold keeps two small values for each foe that killed you: the deaths to it so far (a count that stops at 2, because no later death weighs anything), and whether you beat it. The foe is the entity of `DEFEATED` from the killer to you. So the record of a death and the key of a kill name the same entity.

| Event | Gain |
|---|---|
| A death to a foe that you never beat | `DEATH_WEIGHTS[deaths so far]`: 2 for the first, 1 for the second, 0 after that |
| A death to a foe that you beat | 0 |
| The first kill of a foe | its table weight (3, or 5 for a raid boss), and `REVENGE_WEIGHT` (2) more when the foe killed you before |
| A later kill of the same foe | 0 |
| A death with no known killer (a fall, the water, a player in 5.11) | Keyed on its zone, `(DeathIn, <zone>)`, with the same decay. There is no foe to beat. |

Why:

- **The first death is a story, the tenth is a pattern.** The first wolf that kills you belongs in the saga. The tally tells the rest: "Died to the Defias Pillager again, 9 times".
- **A beaten foe is behind you.** After you beat it, a death to it is a mishap, not a turn of the story. A small fixed weight would let a farmer of one rare make chapters from deaths alone, so it is 0.
- **Revenge is a beat.** The first kill of a foe that killed you closes an arc. The deed carries `revenge: true`, and the saga gets the fact "Defeated Hogger, who had killed them 3 times". The narrator ranks it as a big moment, with a first kill.
- **The total is bounded.** Deaths to one foe weigh at most 2 + 1 = 3 in all, before or after the kill. So 100 deaths to the same mob give at most 3, a fifth of a chapter.

The record is a list of (foe, deaths, beaten). It grows by one entry for each foe that killed you, not for each death.

### 7.3 Repeats still show

A repeat makes no chapter, but it is not lost:

- **In the chapter.** Each repeated key of a chapter becomes one tally line, not one deed per kill: "Defeated Ragnaros again, 6 times". The tallies live in a new list `again` of the chapter, at most 20 entries, like the other lists (3.3). The saga gets the tally as one fact: "Defeated Ragnaros again, 6 times in this chapter".
- **In the atlas.** The zone page counts them: "Molten Core: cleared 7 times". The Deeds page keeps each kill, as today.
- **They never cut.**

An endgame character that only repeats keeps one open chapter for a long time. Its plain list shows the tallies as they grow. Open question 2 asks if that is right.

### 7.4 The bound

Every closed chapter has weight `MIN_WEIGHT` or more: a break closes only from MIN on, and a full close needs `MAX_WEIGHT`, which is MIN or more. The weight of a chapter is the sum of the gains of its steps. Each key gains at most its **cap** in all: its table weight for most kinds, 3 for a death key (2 + 1), and its weight plus `REVENGE_WEIGHT` for a kill. `CAP_MAX`, the largest cap, is 7. So:

> **closed chapters × MIN_WEIGHT ≤ total novel weight ≤ distinct keys × CAP_MAX**

In plain words: the number of closed chapters is at most the new weight divided by 15, and so at most the number of different things that happened times 7 divided by 15. The open chapter adds 1. Nothing that repeats moves either side.

This is stronger than the first draft of the bound (novel weight / MIN + forced closes + 1). A forced close has MAX ≥ MIN weight, so it needs no term of its own. The bound is exact for one fold. A change of the rule (section 11) adds at most one short chapter for each change.

## 8. The proofs

As for the story shelf, `chapters.spec` ties the Rust function to a Lean function `cutsOf : List Step → List Cut`, and the laws speak about `cutsOf`. `advance.spec` does the same for `advanceOf : State → List Step → State`. Each law that holds from `start` also holds from any state that the fold can reach (`Reachable st`), so the story program may fold one line at a time.

Helpers of the sketches: `closed cs` is the cuts with `close ≠ .open`. `gainIn st x` is the gain of step `x` in state `st` (6.2 and 7.2). `gain s i` is `gainIn (advanceOf start (s.take i)) s[i]`. `novel s` is the sum of `gain s i` over all `i`. `keys s` is the list of distinct keys of `s`. `cap k` is the cap of 7.4. `Spent st k` means that key `k` can gain nothing more in state `st`: a seen key of any kind but a death, or a death key whose foe is beaten or whose deaths reached 2.

| # | Theorem | In plain words | Lean sketch |
|---|---|---|---|
| 1 | `every_event_is_in_one_chapter` | The chapters cover the log with no gap and no overlap, in order, and none is empty. | `theorem every_event_is_in_one_chapter (s : List Step) (h : s ≠ []) : Partition (cutsOf s) s.length` where `Partition` says: the first cut starts at 0, each cut ends where the next starts, the last ends at `s.length`, and `first < end` for each. |
| 2 | `a_closed_chapter_never_changes` | Adding events after a log keeps every closed chapter of the log exactly as it was. Sagas and summaries hang on these chapters. | `theorem a_closed_chapter_never_changes (s t : List Step) : closed (cutsOf s) <+: closed (cutsOf (s ++ t))` (a list prefix). The proof is `advanceOf_append : advanceOf st (s ++ t) = advanceOf (advanceOf st s) t` and `closed_only_grows : (advanceOf st t).closed = st.closed ++ _`. |
| 3a | `a_closed_chapter_has_min_weight` | Every closed chapter has weight `MIN_WEIGHT` or more. This is stronger than "unless it is the first chapter or a forced close": the rule never closes a chapter below MIN. | `theorem a_closed_chapter_has_min_weight (s : List Step) (c : Cut) (h : c ∈ closed (cutsOf s)) : MIN_WEIGHT ≤ c.weight` |
| 3b | `no_chapter_passes_max_and_one_step` | No chapter weighs more than `MAX_WEIGHT - 1 + W_MAX`. | `theorem no_chapter_passes_max_and_one_step (s : List Step) (c : Cut) (h : c ∈ cutsOf s) : c.weight ≤ MAX_WEIGHT - 1 + W_MAX` |
| 3c | `the_weight_of_a_chapter_is_the_sum_of_its_gains` | The weight of a chapter is the sum of the gains of its events. | `theorem the_weight_of_a_chapter_is_the_sum_of_its_gains (s : List Step) (c : Cut) (h : c ∈ cutsOf s) : c.weight = ∑ i ∈ Finset.Ico c.first c.end, gain s i` |
| 3d | `a_step_with_no_gain_closes_nothing` | A flyover, a walk, or any step that adds no new weight never closes a chapter. | `theorem a_step_with_no_gain_closes_nothing (st : State) (x : Step) (h : Reachable st) (h0 : gainIn st x = 0) : (advanceOf st [x]).closed = st.closed` |
| 4 | `a_repeat_never_adds_weight` | A step whose key is spent weighs 0. For every kind but a death, a key is spent once it was seen. | `theorem a_repeat_never_adds_weight (st : State) (x : Step) (k : Key) (h : Reachable st) (hk : x.key = some k) (hs : Spent st k) : gainIn st x = 0` |
| 5 | `repeats_alone_never_make_a_chapter` | From any state, a log of spent keys and steps with no key closes no chapter. 1000 weekly clears change nothing. | `theorem repeats_alone_never_make_a_chapter (st : State) (t : List Step) (h : Reachable st) (hr : ∀ x ∈ t, x.key = none ∨ ∃ k, x.key = some k ∧ Spent st k) : (advanceOf st t).closed = st.closed`. The proof needs `spent_stays_spent`: no step makes a spent key gain again. |
| 6 | `chapters_grow_only_with_what_is_new` | The bloat bound of 7.4. | `theorem chapters_grow_only_with_what_is_new (s : List Step) : (closed (cutsOf s)).length * MIN_WEIGHT ≤ novel s ∧ novel s ≤ (keys s).length * CAP_MAX`. Its middle step is `a_key_never_gains_past_its_cap : ∑ i with s[i].key = some k, gain s i ≤ cap k`. |
| 6a | `a_death_to_a_beaten_foe_weighs_no_more` | A death to a foe that you beat never weighs more than a death to a foe that you did not beat, with the same deaths so far. In fact it weighs 0. | `theorem a_death_to_a_beaten_foe_weighs_no_more (r : KillerRecord) (x : Step) (hx : x.key = some (.deathBy r.foe)) : deathGain { r with beaten := true } x ≤ deathGain { r with beaten := false } x ∧ deathGain { r with beaten := true } x = 0` |
| 6b | `deaths_to_one_foe_weigh_at_most_three` | However often you die to one foe, before or after you beat it, those deaths weigh at most 3 in all. 100 deaths to the same mob never make a chapter by themselves. | `theorem deaths_to_one_foe_weigh_at_most_three (s : List Step) (f : EntityId) : ∑ i with s[i].key = some (.deathBy f), gain s i ≤ DEATH_WEIGHTS.sum` (= 3). The same holds for `(DeathIn, zone)`. |
| 6c | `revenge_counts_once` | The revenge weight comes only with the first kill of a foe, and only when the foe killed you before. | `theorem revenge_counts_once (s : List Step) (f : EntityId) : ∑ i with s[i].key = some (.kill f), gain s i ≤ killWeight f + REVENGE_WEIGHT` (`killWeight` is 3, or 5 for a raid boss) |
| 7 | `a_return_with_nothing_new_cuts_nothing` | A return to a zone that brings no new weight closes no chapter. | A case of theorem 5, stated for the reader: `theorem a_return_with_nothing_new_cuts_nothing ...` with `t` all in one settled zone and no gain. |
| 8 | (determinism) | The same log always gives the same chapters. | Free in Lean: `cutsOf` is a function, so `s = s' → cutsOf s = cutsOf s'` is `congrArg`. `chapters.spec` gives one result for one input, so the Rust function is one too. No theorem needs writing. |
| 9 | `chapters.spec`, `advance.spec` | The fold never panics, never overflows, and always ends. | `theorem chapters.spec (s : Slice Step) : chapters s ⦃ cs => cs.val = cutsOf s.val ⦄`. The weight is a `u16`, and 3b bounds it far below the top. The number of cuts is at most the number of steps, so a `usize` holds it. |

Notes:

- 3d and 5 need `Reachable`: a state that `start` and `advance` built. A state made by hand can hold a pending close.
- Theorem 2 is the key one. A saga, its footnotes, the summary, and the zone history (section 10) are written only for a closed chapter. Theorem 2 says that the chapter under them never moves while the rule stays the same. Section 11 covers a change of the rule.
- The walk of the story program (4.1) is not in `crates/rules`: it reads Hourglass. Its one law, "the steps of a prefix are a prefix of the steps", is a property test (9.2). Theorem 2 needs it to speak about the real program.
- `lean/README.md` gets a table "What is proved: the chapters", in the form of the other tables, with the property test of each law.

## 9. Tests

No test needs the game or a model.

### 9.1 Unit tests

In `crates/rules/src/chapters.rs` (`#[cfg(test)]`, as `story_shelf.rs`):

- `a_chapter_closes_at_the_first_break_after_min`
- `a_break_before_min_is_spent`
- `a_chapter_closes_at_max_with_no_break`
- `a_repeat_weighs_nothing`
- `the_cut_waits_for_the_next_step_with_weight`
- `a_return_after_two_chapters_away_opens_a_chapter`
- `a_return_with_nothing_new_folds_into_the_open_chapter`
- `settling_in_a_new_zone_is_a_break`
- `a_tenth_level_ends_its_chapter`
- `a_place_names_the_chapter_before_a_pause` (the order of `opened_by`)
- `the_weight_table_never_passes_w_max`
- `folding_in_two_parts_gives_the_same_chapters`

In `crates/story/tests/chapters.rs` (the walk):

- `a_flight_across_four_zones_makes_no_chapter`
- `a_subzone_seen_from_the_air_weighs_nothing`
- `leaving_a_dungeon_is_a_break`
- `leaving_a_capital_is_a_break`
- `eight_hours_away_is_a_break`
- `seven_hours_away_is_no_break`
- `the_first_level_at_login_weighs_nothing`
- `the_first_death_to_a_foe_weighs_two_and_the_second_one`
- `a_third_death_to_the_same_foe_weighs_nothing`
- `a_death_to_a_beaten_foe_weighs_nothing`
- `the_first_kill_of_a_foe_that_killed_you_is_revenge`
- `revenge_marks_the_deed_for_the_saga`
- `a_fall_counts_once_in_each_zone`
- `a_raid_boss_weighs_more_than_a_rare`
- `the_walk_never_reads_the_final_world` (an instance flag set later does not mark an earlier leave)
- `two_chapters_can_begin_at_one_tick`
- `a_flavor_moment_belongs_to_the_newest_chapter_at_its_tick`

In `crates/story/tests/journal.rs`:

- `a_long_grind_in_one_zone_breaks_into_chapters`
- `a_return_chapter_is_named_for_its_zone`
- `a_repeat_shows_as_a_tally_in_its_chapter`
- `a_chapter_lists_only_the_zones_where_something_happened`

### 9.2 Property tests

In `crates/story/tests/properties.rs`. The generators make the edges likely, not uniform: a weight sum is drawn from {MIN-1, MIN, MAX-1, MAX, MAX+1, MAX+W_MAX-1} more often than from the rest.

- `chapters_cover_the_log_once`: any play. The cuts partition the events.
- `a_closed_chapter_never_changes_when_play_goes_on`: any play, cut at a random point, compared with the whole play. Also a cut inside one tick.
- `the_steps_of_a_prefix_are_a_prefix_of_the_steps`: the walk is stable too.
- `closed_chapters_weigh_at_least_min_and_at_most_max_and_one_step`: plays with chapter weights at MIN-1, MIN, MAX, and MAX+1.
- `zero_weight_runs_never_close_a_chapter`: long runs (up to 10,000) of zone entries, flights, sightings, and meetings.
- `breaks_in_a_row_cut_once`: many breaks in a row, and logs that are only breaks: 8-hour gaps, leaves, tenth levels.
- `a_thousand_raid_clears_make_as_many_chapters_as_one`: one clear, then 999 weeks of entry, the same boss kills, leaving, and 7 days away. The count of chapters equals the count after the first week.
- `repeats_mixed_into_edge_weights_change_nothing`: a play with a chapter at MIN-1, MIN, and MAX, then the same play with repeats put in at random places. Same cuts, by event of note.
- `chapters_grow_only_with_what_is_new`: closed × MIN ≤ novel weight ≤ keys × CAP_MAX.
- `a_death_to_a_beaten_foe_never_weighs_more`: any play with deaths and kills of a few foes, so that a foe often kills you both before and after you beat it. Each death to a beaten foe gains 0, and never more than the same death would gain before the kill.
- `a_hundred_deaths_to_one_mob_weigh_at_most_three`: 1 to 100 deaths to one foe, with its kill at a random place or never, mixed with other play. The deaths to that foe gain 3 or less in all. The chapters equal those of the same play with only its first 2 deaths to that foe. Also with no known killer, in one zone.
- `folding_one_line_at_a_time_gives_the_whole_fold`: the in-memory fold of the story program equals `chapters` over the whole history.

### 9.3 Fuzz targets

The fold reads no text from outside. The new parts that do:

- `rest_changed` is a new line of the bridge: seeds in `fuzz/seeds/input/`. The `input` target covers it.
- A `zone_histories` row is a new row table: seeds for the `store` target.
- The answer of the zone history is model text: `checked_zone_history` joins the `answers` target, with the limits of 10.3.
- `dungeon_ends.toml`, if built (5.2), is a data file of the project, read by a test, as `pack_sources.toml`.

## 10. The zone history: "Your history here"

The atlas page of each zone (`docs/plans/readable-chronicle.md` 12) gets two parts:

- **Your history here.** A short paragraph of the narrator about what the character did in this zone, over all visits.
- **Chapters here.** Every chapter with weight in this zone, oldest first, each a link to its page in the Chronicle. The chapter carries a new list `zones_played`: the zones where its steps had gain. The addon builds this list from the chapters, so no new list goes over the wire.

### 10.1 When it is rewritten

- After a chapter closes and its saga round and its summary are done, the zone history of **one** zone is due: the zone with the most gain in that chapter, if that gain is `ZONE_HISTORY_MIN_WEIGHT` (5) or more. A tie goes to the zone of the newest step.
- So a return to a zone rewrites its history when the return chapter closes. A flight through a zone never does.
- The call opens with the same gates as the summary: no other call open, the pace window not tight, no older chapter waiting for its saga, and no summary due.
- With a backlog, only the newest due chapter of each zone gets a call. A restart forgets a due history, and the next chapter in that zone makes it due again.
- So a chapter costs at most 5 calls: 2 drafts, the judge, the summary, and one zone history. With a tight window, it costs 3.

### 10.2 The prompt

`zone_history.rs`, in the form of `summary.rs`: the persona and the house rules; the zone and its kind ("The Deadmines (a dungeon)"); the history before, if any; the sagas of the newest 4 chapters with gain in the zone (a chapter with no saga gives its plain list); the deeds of note in the zone, at most 10; and the tallies of repeats in the zone ("Defeated Ragnaros 7 times"). No hero sheet, no story, and no Roleplay Profile. The note asks for at most 60 words, and for what changed since the history before. The answer is JSON: `{"history": "..."}`.

### 10.3 The check

The checks of a saga (3.3): one paragraph of plain text, at most 400 characters (`MAX_ZONE_HISTORY_CHARS`), no emoji, no banned word, no slop that the facts lack, no name past the cutoff that the player did not write, `$N` at most twice, and never "our hero". Also no 8 words in a row of any saga of the zone: the history retells, it does not paste. A refused answer or a failed call keeps the history before. There is no second draft.

### 10.4 The storage and the reads

- A new row table `zone_histories`: one row `{"zone": "<zone name>", "after": <EventId of the first event of the chapter>, "text": "..."}`. The newest row of each zone stands.
- The reads of the call (5.14): the saga row and the events of each chapter in its prompt, the events behind each deed of note and each tally, and the `zone_histories` row before.
- The journal: each zone of `places` gets `history: Option<Box<str>>`. A zone with no history shows only its chapters. Empty-state copy: nothing, the section is left out.

## 11. Migration notes

### 11.1 What a re-cut breaks

Under any new rule, or any new value of MIN or MAX, the old chapters move.

| Stored thing | Keyed by | What happens under a re-cut |
|---|---|---|
| Saga and its footnotes (`chapters` table) | the tick that began the chapter | Orphaned when no new chapter begins at that tick. The row stays (tables only grow), but no page shows it. |
| Summary (`summaries`) | `after`: the tick of the chapter | The newest summary still shows, as text. Its `after` can point at no chapter. The next saga round makes a new one due. |
| Footnote tellings (`flavor`, `Teller::Chronicle`) | time | Not affected. |
| Hero entries of a chapter | time | Move with the chapter. Not affected. |
| Zone history (new) | `after`: the chapter | As the summary. |

### 11.2 Is a rule version on each chapter enough?

No, not alone. A version on a saga row says which rule wrote it. It does not say where the chapters of the new rule begin, and a fold of the new rule over the whole log still moves every old chapter.

**Rule epochs solve it.** A new row table `chapter_rules`: one row `{"rule": <number>, "from": <EventId>}` for each change. The chapters of a world are the chapters of each epoch, in order:

- Each epoch folds only its own events, with its own constants.
- It starts with the seen keys, the records of killers, and the settled zones of all events before it, so an old kill does not count as new again.
- The open chapter of the old epoch closes at the change, even below MIN (`Close::RuleChanged`). This is the one closed chapter that can weigh less than MIN, one for each change. The theorems hold inside each epoch.
- So every old chapter keeps its range, every old saga keeps its chapter, and a play test may tune MIN and MAX at any time.

Each saga row also stores `rule`, `first`, and `last` (`EventId`s). On open, a saga row whose range matches no chapter is orphaned: it stays, no page shows it, and the log names it. A test makes sure that this never happens without a change of the stored rows.

### 11.3 Each schema change

The rule of `docs/plans/hero-stories.md` 3.6: each change says what its migration needs. Today nothing is live (no world has a release), so step 1 of section 12 may change the rule and refuse old worlds by version (`GAMEPLAY.md` 5.7).

| Change | What a migration from the version before needs |
|---|---|
| A saga row gets `rule`, `first`, `last`, and its key becomes `first` (an `EventId`) | Set `rule` 1 on each old row. Find `first` as the position of the first event at its tick, and `last` from the old cut. Write the rows again. Then add a `chapter_rules` row `{"rule": 1, "from": 0}` and one `{"rule": 2, "from": <position of the next event>}`, so the old chapters stay as they were. |
| New row table `chapter_rules` | Create it with the columns of a row table, with the one row `{"rule": 1, "from": 0}` for an old world. |
| `summaries.after` becomes an `EventId` | Map each tick to the first event of its chapter under rule 1. |
| New row table `zone_histories` | Create it, empty. The first history comes after the next chapter. |
| New fact `resting` (5.2) | The next version of the vocabulary. Hourglass migrates the world (`hourglass::migrate`): no old event changes, because no old world holds the name. |
| `npc_defeated` gets `encounter`, `zone_entered` gets `taxi` | None. Both fields are optional on the wire, and the world stores neither. |

## 12. Build steps, in order

1. **The fold in `crates/rules`.** `chapters.rs` with the types, the weight table, the constants, and the fold, in loop style. Its unit tests (9.1). Charon roots on `start`, `advance`, `chapters`.
2. **The Lean proofs.** Run `lean/extract.sh`. Prove section 8 in `lean/Timeways/Chapters.lean`, from `chapters.spec` up to the bloat bound. Add the table to `lean/README.md`.
3. **The walk in the story program.** `crates/story/src/chapters.rs` turns the history into steps. `journal.rs` takes the cuts. The chapter key becomes the first `EventId`: `Pending::Chronicle`, `Round`, `chapter_waiting_for_saga`, `summaries.rs`, `reads.rs`, and the tick ranges of flavor moments and hero entries. The fold state stays in memory per character. Measure the open of a world of 50,000 events.
4. **Rule epochs.** The `chapter_rules` table, and `rule`, `first`, `last` on each saga row, with one rule. Then a change of a constant never orphans a saga.
5. **What the chapter carries.** `opened_by`, `zones_played`, and `again` in `Chapter`. The deeds of a chapter leave out the repeats. The addon: chapter titles ("Return to Tirisfal Glades"), and the tally lines. Lua tests.
6. **Property tests and fuzz seeds** (9.2, 9.3).
7. **The zone history** (section 10): the table, the prompt, the check, the call and its gates, the reads, and `places[].history`. The atlas page shows it when the atlas is built (`readable-chronicle.md` 12.8).
8. **The new events** (5.2): rest first, then the taxi flag. The API gate first, then a step in the game for each.
9. **Later: a dungeon cleared,** after open question 5.
10. **Docs.** Move the built rules into `GAMEPLAY.md` 3.3, 3.6, 5.4, 5.7, and 5.14, and mark the steps here as done.

## 13. Open questions for the user

1. **The numbers.** MIN 15, MAX 40, death weights 2 and 1, revenge 2, 2 chapters away for a return, 8 hours away. These are first guesses for play tests.
2. **The endgame.** A level 60 character that only raids keeps one open chapter for weeks, with tallies and no saga. Is that right, or does a long open chapter need its own end (for example, a saga of a "season" after 30 days)? Any such end must still count only new weight, or the bloat bound breaks.
3. **Talks.** A talk with an NPC (3.5) and a meeting weigh 0 now. A roleplay evening of talks then makes no chapter. Give a first talk with each NPC 1?
4. **A return below MIN.** The user's design said "Return to Tirisfal Glades is its own chapter". The bloat rule says a return cuts only with MIN in the chapter before. So a quick return in the middle of a thin chapter does not get its own chapter. Keep the bloat rule?
5. **A dungeon cleared.** Build a list of the last boss of each dungeon from the game files (5.10), with the license question of `GAMEPLAY.md` 9.4? Or keep entry plus boss kills?
6. **The taxi flag.** Worth one more field and one more API gate check, to count a subzone first seen from the air?
7. **The title after a full close.** A chapter that opens because the one before reached MAX has no break to name it. The zone name alone, or "Tirisfal Glades, continued"?
8. **The zone history check.** Refuse 8 words in a row of a saga of the zone, as for sagas? A summary of sagas reuses their words often, so many answers can fail.
