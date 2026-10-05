# Plan: chapters and tales of the Chronicle

Status: spec only, 2026-10-05. Nothing is built. The long first draft is in git history (`6e8b857`). This version adds tales and cuts the text.

## 1. Goal

The Chronicle is one book in time order. It holds two kinds of entries:

- **Chapters** tell the open world: quests, travel, levels, and world PvP.
- **Tales** tell one instance each: a dungeon, a raid, or a battleground.

The code decides every chapter and every tale. A model only writes their text.

Four rules hold for any play, and Lean proves them:

1. A flight, a walk, or a hearthstone makes no entry.
2. A long grind in one zone still breaks into chapters.
3. The same thing done again never makes a new entry.
4. A closed entry never changes when play goes on.

## 2. Today, and why it fails

Today a chapter starts at a milestone: the first visit of a zone, every tenth level, or a first kill of a rare or a boss (`GAMEPLAY.md` 3.3).

| Problem | Cause |
|---|---|
| A flight across 4 zones makes thin chapters. | A first visit counts, also from the air. |
| A long grind never breaks. | Quests and deaths are no milestones. |
| A raid night cuts the leveling story in two. | An instance is one more zone. |
| A rule change orphans old sagas. | A saga is keyed by the tick of its chapter. |

## 3. Steps, keys, and tracks

The story program walks the history once, oldest first. Each event becomes one **step**. A step has:

- `key`: what the event is about, or none. Example: `(Kill, Hogger)`, `(Level, 20)`.
- `zone`: the zone where you stood.
- `track`: `World`, or `Instance(zone)` when the zone is a dungeon, a raid, or a battleground.
- `mark`: a break, or none (section 5).

The walk reads only the events up to the step. It never looks ahead.

A dungeon or raid is known from `instance_entered` (built). **A battleground sends nothing today.** It needs `instance_entered` with `"pvp"` (section 9).

## 4. Weight

A step adds its weight **only the first time its key happens**. A repeat adds 0.

| Event | Key | Weight |
|---|---|---|
| A game quest turned in | `(GameQuest, quest)` | 1 |
| A Timeways side quest finished | `(SideQuest, quest)` | 2 |
| A class quest turned in | `(ClassQuest, quest)` | 3 |
| A new subzone, on foot | `(Subzone, place)` | 1 |
| A level up | `(Level, n)` | 2 |
| A first talk with an NPC | `(Talk, npc)` | 1 |
| A first kill of a rare or a boss | `(Kill, foe)` | 3, +2 revenge |
| A first kill of a raid boss | `(RaidKill, foe)` | 5, +2 revenge |
| A death to a foe you never beat | `(DeathBy, foe)` | 2, then 1, then 0 |
| A death to a foe you beat | `(DeathBy, foe)` | 0 |
| A death with no known killer | `(DeathIn, zone)` | 2, then 1, then 0 |
| A lasting mark, a joke title | `(Mark, …)`, `(Title, …)` | 1 |
| A flight, a walk, a new zone, a sighting, trust | none | 0 |

**Deaths and revenge.** Once you beat a foe, dying to it matters less. A death to a beaten foe weighs 0. The first kill of a foe that killed you before adds 2 for revenge. The saga gets "Defeated Hogger, who had killed them 3 times."

`W_MAX`, the largest weight of one step, is 7. `CAP_MAX`, the most one key can add in all, is 7.

## 5. Chapters

### 5.1 Breaks

A break is a natural pause.

| Break | Source | Today? |
|---|---|---|
| Settling in a new zone | The first step with weight in a zone | Yes |
| A return to a zone | A step with weight in a zone that had none in the last 2 chapters | Yes |
| Every tenth level | The `LEVEL` update | Yes |
| Leaving an instance or a capital | The `LOCATED_IN` that leaves it | Yes |
| 8 hours away | The gap between two events | Yes |
| Leaving an inn | A new line `rest_changed` | No (section 9) |

### 5.2 The rule

1. A break waits until the next step with weight.
2. At that step, the chapter closes if its weight is **MIN** (15) or more. Else the break is spent.
3. A chapter also closes once its weight reaches **MAX** (40), with no break.
4. Only steps on the `World` track add weight to a chapter. An instance step adds nothing.

The title of a chapter is the zone where it opened, or "Return to Tirisfal Glades". After a close at MAX, it is "Tirisfal Glades, continued".

## 6. Tales

A tale is one entry for one instance. Each instance has **at most one tale** in the whole life of the character.

- **It opens** at the first step with weight inside the instance.
- **It closes** when you leave the instance.
- **It grows on a later visit,** but only with new weight, such as a raid boss you never killed before. A visit with nothing new changes only its count: "Molten Core, 7 runs."
- **Its text is rewritten** after a visit that added new weight. A visit with nothing new costs no model call.

| Instance | What gives a tale weight |
|---|---|
| Dungeon | The first entry (3), first boss kills, first deaths, quests done inside |
| Raid | The first entry (5), first raid boss kills, first deaths |
| Battleground | The first entry, the first win, each new PvP rank |

A battleground win and a PvP rank need new events (section 9).

In the book, a tale sits between chapters at the time of its first visit. The contents list marks it as a dungeon, a raid, or a battleground, and can filter by kind. The atlas page of the instance shows its tale, its bosses, and its count of runs.

## 7. No bloat

**Entries grow with what is new, never with time played.**

- A repeat has no weight. So it never meets a break, never reaches MAX, and never opens a tale.
- A repeat still shows. Its chapter or its tale gets one tally line: "Defeated Ragnaros again, 6 times."
- A long endgame of repeats keeps one chapter open. That is correct.

**The bound.** Every closed chapter weighs at least MIN. So:

> closed chapters × MIN ≤ new world weight ≤ distinct keys × CAP_MAX
>
> tales ≤ distinct instances

## 8. The proofs

The fold lives in `crates/rules/src/chapters.rs`, in loop style. Aeneas extracts it to Lean, and `lean/Timeways/Chapters.lean` proves these laws. `cutsOf` is the Lean function of the fold.

| # | Theorem | In plain words |
|---|---|---|
| 1 | `every_event_is_in_one_entry` | Chapters cover the world steps once, with no gap or overlap. Each instance step is in exactly one tale. |
| 2 | `a_closed_entry_never_changes` | Adding events keeps every closed chapter and every closed tale as it was. |
| 3 | `a_closed_chapter_has_min_weight` | Every closed chapter weighs MIN or more. |
| 4 | `no_chapter_passes_max_and_one_step` | No chapter weighs more than MAX − 1 + W_MAX. |
| 5 | `the_weight_of_a_chapter_is_the_sum_of_its_gains` | A chapter's weight is the sum of its steps' gains. |
| 6 | `a_step_with_no_gain_closes_nothing` | A flight or a repeat never closes a chapter. |
| 7 | `a_repeat_never_adds_weight` | A spent key gains 0. |
| 8 | `repeats_alone_never_make_an_entry` | Any run of repeats closes no chapter and opens no tale. |
| 9 | `chapters_grow_only_with_what_is_new` | The chapter bound of section 7. |
| 10 | `an_instance_never_adds_world_weight` | An instance step adds 0 to a chapter. |
| 11 | `an_instance_has_at_most_one_tale` | Tales ≤ distinct instances. A repeat run changes only the count. |
| 12 | `a_death_to_a_beaten_foe_weighs_nothing` | And never more than a death to an unbeaten foe. |
| 13 | `deaths_to_one_foe_weigh_at_most_three` | 100 deaths to one mob never make a chapter. |
| 14 | `revenge_counts_once` | Only the first kill of a foe that killed you adds revenge. |
| 15 | `chapters.spec` | The fold never panics, never overflows, and always ends. |

Determinism needs no theorem. `cutsOf` is a function, so the same log always gives the same entries.

The walk in the story program reads Hourglass, so Lean does not see it. Its one law, "the steps of a prefix are a prefix of the steps", is a property test.

## 9. New events

| Event | Why | Needs |
|---|---|---|
| `instance_entered` with `"pvp"` | Battleground tales | The API gate, a test in the game |
| `bg_won` | A battleground win | `UPDATE_BATTLEFIELD_STATUS` and the winner, through the API gate |
| `pvp_rank` | A new PvP rank | `UnitPVPRank`, through the API gate |
| `rest_changed` | The inn break | `PLAYER_UPDATE_RESTING`, `IsResting`, a new fact `resting` |
| `taxi` on `zone_entered` | A subzone seen from the air never counts | `UnitOnTaxi` |

Each new line gets a fuzz seed for the `input` target.

## 10. Zone history

Each zone page in the atlas gets "Your history here": one narrated paragraph about what you did there, over all visits.

- After a chapter closes, its zone with the most new weight (5 or more) gets a rewrite. One model call.
- The call waits behind sagas and the summary. A chapter costs at most 5 calls.
- The check is the saga check: at most 400 characters, no slop, never "our hero". It must not copy 8 words in a row from a saga. A refused answer gets one retry, then the old text stays.
- The page also lists every chapter that had weight in the zone.

## 11. Tests

- **Unit tests** in `crates/rules` and `crates/story`, named as sentences. Examples: `a_flight_across_four_zones_makes_no_chapter`, `a_raid_night_does_not_cut_the_open_chapter`, `a_second_dungeon_run_with_nothing_new_changes_only_its_count`.
- **Property tests** in `crates/story/tests/properties.rs`. Make the edges likely: chapter weights at MIN−1, MIN, MAX, and MAX+1, runs of 10,000 zero-weight steps, and logs made only of breaks. They also cover:
  - 1000 weekly raid clears give the same entries as 1;
  - 100 deaths to one mob weigh at most 3;
  - folding one line at a time gives the same entries as folding the whole log.
- **Fuzz:** seeds for each new line, the `zone_histories` rows, and the zone-history model answer.

## 12. Migration

A new rule, or new values of MIN and MAX, re-cut the log. That orphans old sagas.

**Rule epochs fix it.** A row table `chapter_rules` holds `{rule, from: EventId}` for each change. Each epoch folds only its own events, with the seen keys of all events before it. The open chapter closes at a change, even below MIN. That is the one exception to theorem 3, at most once per change. Old chapters and their sagas never move.

| Schema change | What a migration needs |
|---|---|
| Saga rows keyed by first `EventId`, with `rule`, `first`, `last` | Map each old tick to its first event under rule 1, then add the epoch rows. |
| New tables `chapter_rules`, `tales`, `zone_histories` | Create them. `chapter_rules` gets `{rule: 1, from: 0}`. |
| New facts `resting`, `pvp_rank` | A vocabulary version. No old event changes. |

## 13. Build order

1. The fold, the weight table, tracks, and tales in `crates/rules`, with unit tests.
2. The Lean proofs of section 8.
3. The walk in the story program, chapter keys as `EventId`, and the fold state kept in memory.
4. Rule epochs.
5. Titles, tally lines, and tales in the journal and the addon.
6. Property tests and fuzz seeds.
7. The zone history.
8. The new events: battleground entry, win, and rank first, then rest and taxi.

## 14. Open questions

Proposed answers wait for the user.

| Question | Proposed |
|---|---|
| Start with MIN 15, MAX 40, deaths 2 then 1, revenge 2, 8 hours away? | Yes, and tune in play tests. |
| Is one open chapter for weeks of endgame repeats right? | Yes. |
| Does a first talk with an NPC weigh 1? | Yes (in the table). |
| Does a return need MIN in the chapter before? | Yes, or quick visits bloat the book. |
| Build a list of each dungeon's last boss for "cleared"? | Later. Boss kills stand in for it. |
| Add the taxi flag? | Yes. |
| Does a battleground tale count wins after the first? | No. Only the first win and new ranks. |
