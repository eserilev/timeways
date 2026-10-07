# The Lean proofs

Aeneas translates the crate `timeways-rules` (`crates/rules`) into pure
Lean functions. The theorems in `Timeways/QuestLog.lean`,
`Timeways/HeroHook.lean`, `Timeways/Budget.lean`,
`Timeways/TrustBand.lean`, `Timeways/Prompts.lean`,
`Timeways/Aliases.lean`, `Timeways/StoryShelf.lean`,
`Timeways/EntryEdits.lean`, `Timeways/Chapters.lean`,
`Timeways/NarratorShapes.lean`, and `Timeways/ThinLore.lean` are about those functions. A theorem holds for every input, with no bound. The
property tests in `crates/story/tests/properties.rs` check the same
rules on random input, and they stay as a second check.

The Lean is generated from the Rust, so it does not drift from the
Rust. The story program calls `timeways-rules` for these rules, so the
proofs speak about the code that runs. After a change
to the crate, run the extraction again. If a law breaks, the build
fails.

## What is proved: the quest log

`quest_log` folds the lines of the quest file into quests. Each
theorem holds for every list of lines, of any length, in any order.
A damaged quest file is such a list too.

| Theorem | The law | Property test |
|---|---|---|
| `quest_log.spec` | The fold never panics and always ends. Each quest has one done time and one kill count for each step. | none |
| `ordered_quest_is_done_in_order` | In a quest with no any-order set, no step is done before the step before it. | `in_an_ordered_quest_no_step_is_done_before_the_step_before_it` |
| `any_order_set_waits_for_the_steps_before_it` | A step of an any-order set is done only after every step before the set. | `an_any_order_set_is_done_only_when_all_its_steps_are_done` |
| `a_step_after_a_set_waits_for_all_of_it` | A step after an any-order set is done only when every step of the set is done. | `an_any_order_set_is_done_only_when_all_its_steps_are_done` |
| `a_wait_never_ends_early` | A done wait step was done at its opening time plus its days, or later. | `a_wait_never_ends_early` |
| `kills_never_pass_the_count` | A kill step counts no more kills than its count. Every other step counts none. | `kills_count_only_for_an_open_kill_step_and_never_past_its_count` |
| `an_offer_has_no_done_steps` | An offer that waits for an answer has no done step and no kill. | none |
| `a_quest_is_done_exactly_when_every_step_is_done` | A quest is done exactly when it has a step and every step is done. | `a_quest_log_never_skips_a_step_and_each_giver_holds_at_most_one_offer` |
| `each_giver_holds_at_most_one_waiting_offer` | Two waiting offers of one giver are the same quest. | `a_quest_log_never_skips_a_step_and_each_giver_holds_at_most_one_offer` |
| `a_kill_counts_only_for_an_open_step` | After any log, one more line adds a kill to a step only when the step was open before the line. | `kills_count_only_for_an_open_kill_step_and_never_past_its_count` |
| `every_quest_points_at_its_offer` | The `line` of each quest is an offer line, with the number and the giver of the quest. | `every_offer_of_a_log_becomes_one_quest_with_its_number_and_giver` |

Each theorem is a Hoare triple `quest_log cs ⦃ qs => ... ⦄`. The triple
also says that `quest_log` gives a result: it never panics and never
loops forever.

The opening time of `a_wait_never_ends_early` is the one that the
property test reads: the latest done time of the steps before the
wait, or the accept. The add in `ready_at` saturates at `u64::MAX`. So
the theorem has two cases: the wait ended at the opening time plus its
days or later, or the sum passes `u64::MAX` and the step was done at
`u64::MAX`. A wait is never in an any-order set, because the log drops
a set that holds one. Without that rule, a wait in a set opens at the
start of the set. A step of the set that is done later does not move
that time, so the wait can end early. The invariant
`NoWaitInSet` holds the rule, so the proof needs it.

A quest with no step comes only from a damaged quest file. It is never
done, so `a_quest_is_done_exactly_when_every_step_is_done` needs a step.

`a_kill_counts_only_for_an_open_step` is a law of two logs: a log, and
the same log with one more line. The property test compares the same
two logs. The log has fewer than `usize::MAX` lines, so the new line
fits.

`every_quest_points_at_its_offer` is the fact that `Tracked::from_rules`
in the story program reads. It finds the words of each quest at its
`line`, so it drops no quest.

The laws of one quest come from one invariant, `Good`, in
`Timeways/QuestLog.lean`. Each kind of line keeps it:
`Progress.offered.spec`, `Progress.finish_step.spec`,
`Progress.count_kill.spec`, `good_accept`, `good_status`, and
`apply.spec`. The laws of the list come from `apply.spec` too. A line
keeps each quest that was there (`Kept`), and adds at most its own
offer at the end (`Grown`). `quest_log_inv` holds the two invariants of
the list: `OneOfferEach` and `LinesOk`.

## What is proved: the hero hook

`pick` chooses which filled answer of the hero sheet a hook call takes
(`hero_hook::hook` in the story program).

| Theorem | The law | Test |
|---|---|---|
| `pick_is_never_out_of_range` | A hook call never picks a field out of range, for every count and every number of filled fields. | none |
| `every_filled_field_takes_its_turn` | Among any `filled` hook calls in a row, each filled field is picked. | `the_hook_rotates_through_the_filled_answers_in_the_order_of_the_sheet` |

The hook calls are the calls with a count of 3k + 2. The second law
holds for a row that starts at any k, while the counts fit a `u64`.

## What is proved: the narrator budget

| Theorem | The law | Test |
|---|---|---|
| `the_narrator_never_speaks_four_times_in_one_hour` | For any sequence of times, any four lines that the narrator speaks in a row span an hour or more. | `the_budget_allows_three_lines_in_an_hour` |

The law holds for a sequence of any length, with any times, also times
that go back. `linesSpoken` applies `take` to
each time of the list, from `fresh`, the budget of `Budget::default()`.

## What is proved: trust

| Theorem | The law | Test |
|---|---|---|
| `next_trust.spec` | The trust after one change is in the band -100..100, for every trust held and every change. When the trust held is in the band, a change never moves it against its sign. | none |
| `trust_stays_between_minus_one_hundred_and_one_hundred` | For any trust held and any sequence of changes, the trust after each change is in the band. | `trust_stays_between_minus_one_hundred_and_one_hundred` |

## What is proved: the prompts kept

`oldest_prompt_kept` gives the oldest call that keeps its prompt
(GAMEPLAY.md 5.14). The store clears the prompts of the calls before it.

| Theorem | The law | Test |
|---|---|---|
| `the_newest_prompts_are_always_kept` | For every position of the newest call, also at the edge of `u64`, the newest 500 calls keep their prompts, or every call when there are fewer. | `only_the_newest_prompts_are_kept` |

## What is proved: the alias table

The alias table gives each player an ID (`GAMEPLAY.md` 5.11). The
story program cuts a text into pieces: words with their keys, other
text, and IDs. The rules compare keys only. `learnLines` learns each
name of each line in turn, with the model of `learn`. The story program
does the same: `AliasLog::learn` calls `learn` once for each new name.
`learn.spec` and `learn_all.spec` tie the model to the Rust.

| Theorem | The law | Test |
|---|---|---|
| `an_id_is_never_reused` | After any sequence of lines, each place of the table holds the player that it held before. A new player goes only at the end. | `an_alias_is_never_reused_over_any_lines` |
| `a_name_keeps_its_id` | Once the table holds a name at a place, every later table holds it at that place. | the same |
| `two_names_never_share_an_id` | From an empty table, no sequence of lines puts one key at two places. | `an_alias_is_never_reused_over_any_lines` |
| `one_id_names_one_player` | Two keys at the same place are the same key. | none |
| `every_name_of_a_line_gets_an_id` | After a line, the table holds each name of the line. | none |
| `no_known_name_after_the_swap` | No word of the text for a model has a key of the table. | `the_text_for_a_model_holds_no_known_name_as_a_word` |
| `every_id_of_the_swap_is_in_the_table` | Each ID of the swap names a player of the table, when the text held no ID before. | none |
| `the_swap_and_back_keeps_the_text` | The swap to IDs and back gives each piece back. A known name comes back with its key, in the form that the table holds. | `the_swap_and_back_gives_each_name_in_the_form_of_the_table` |
| `restore_keeps_what_is_no_known_name` | A piece that is no known name comes back exactly. | the same |

A full round trip of the text is not true. "ADA-Stormrage" comes back
as "Ada": the ID keeps who the player is, not how the text wrote the
name. So the round trip law names the form of the table.

`learn` and `learn_all` need room in the table: fewer than `usize::MAX`
players. The glue in `crates/story/src/aliases.rs` cuts the text into
pieces and folds the case. No proof reads it. Its property tests check
that a text cut and joined again is the same text, and that no word of
the text for a model folds to a known name.

## What is proved: the story shelf

The shelf holds the accepted stories of a player (`GAMEPLAY.md` 4.8).
`lands` decides if a line of the `stories` table may land: an accept of
a new number, or a removal of a story that stands and that no accepted
call used. `standing` gives the numbers that stand, oldest first.
`Landed` says that each line landed against the lines before it. The
story program only appends a line that lands, and a load cuts the table
at the first row that does not land. So every shelf of a world is
`Landed`, also a damaged one.

| Theorem | The law | Test |
|---|---|---|
| `lands.spec`, `standing.spec` | The two functions never panic, always end, and give their pure model. | the unit tests of `story_shelf.rs` |
| `a_number_stands_at_most_once` | On a landed shelf, no number stands twice. | `the_shelf_keeps_each_number_once` |
| `a_removed_story_never_comes_back` | After a removal, the number never stands again, whatever lines come after it. The shelf need not be landed. | the same |
| `a_used_story_always_stands` | A story that a call used and that stands, stands after any line that lands. | the same |

The glue in `crates/story/src/stories.rs` turns each row into a
`ShelfLine`. The glue in `crates/story/src/story/stories.rs` asks the
database which stories an accepted call read (`uses_of`). No proof reads the glue. The property test checks it, with
numbers at the edges of a `u64` and numbers that come again.

`used` comes from the SQL of `accepted_readers_of`, which keeps only the
calls with the result `accepted`. No proof reads SQL, so tests check it:
`a_story_that_a_call_used_cannot_be_removed` (an accepted call uses the
story) and `a_story_that_a_refused_failed_or_open_call_read_can_be_removed`
(a refused, a failed, and an open call use nothing).

## What is proved: the player edits

A player can change what a chapter, a tale, or the summary says
(`docs/plans/chapters.md` 11). `shown` gives the rows that an entry
shows: the edit whose title shows, the narrator row whose text shows,
and the edit whose paragraphs show. It reads row ids and the kind of
each edit, never a string. `shownM` is its pure model: the newest
narrator row and the newest edit decide. The newest row is the one with
the largest id. Of two edits with the same id, the later one in the
list wins (`pickEdit_tie`).

| Theorem | The law | Test |
|---|---|---|
| `newest_row.spec`, `newest_edit.spec`, `shown.spec` | The functions never panic, always end, and give their pure model. | the unit tests of `entry_edits.rs` |
| `the_newest_edit_decides` | `shown` of the whole lists equals `shown` of only the newest narrator row and the newest edit. | `the_newest_edit_by_row_stands_in_any_order` |
| `a_restore_shows_the_newest_narrator_text` | After a restore, the entry shows the newest narrator row, the title of the code, and no words of the player. | `restore_shows_the_newest_narrator_text` |
| `a_restore_shows_a_later_narrator_text` | After a restore, a narrator row newer than all others shows, also one that came after the edit. | the same |
| `a_model_text_never_hides_player_words` | One more narrator row never changes the shown title or the shown player row. Under `Replace`, it changes nothing. | `a_saga_after_a_note_on_the_open_chapter_shows_above_it`, `a_saga_after_a_replace_does_not_show` |

The ids of narrator rows and of edit rows come from two tables, so the
laws never compare them. Theorem 21 holds for any new narrator row, not
only a newer one: the title and the player row never read the narrator
rows.

## What is proved: the chapters

The fold in `crates/rules/src/chapters.rs` decides each chapter, each
visit of an instance, each tale, and the gain of each step
(`docs/plans/chapters.md`). The proofs have three files.

- `Timeways/ChaptersModel.lean` is a pure model of the fold. Each
  function of the model gives back only the fields that it changes, so
  a law sees at once what a step leaves alone. `runM ss` is the fold of
  the log `ss` from the start.
- `Timeways/ChaptersBridge.lean` proves that the Rust fold computes the
  model, function by function. So the laws of the model are laws of the
  Rust code.
- `Timeways/Chapters.lean` holds the laws.

**The room.** A push on a full vector fails in Rust. The bridge needs
room for each step: no vector of the fold that a step can grow is longer
than the steps so far (`Sized`), and the steps so far and the new steps
fit a `usize`. A fold of one log from the start always has this room.
The keys, the foes, and the zones need no room: the fold checks it
(`has_slot`).

**Reachable.** A law of the form "from a reachable fold" is a law about
`runM ss` for a log `ss` that fits a `usize`. No predicate names it: the
law says `runM ss`. Folding one line at a time
gives the same fold as folding the whole log
(`advance_one_line_at_a_time`), so the story program can fold each new
line.

| # | Theorem | The law | Test |
|---|---|---|---|
| 1 | `every_step_is_in_one_chapter` | The closed chapters chain from step 0 to the first step of the open chapter, in order, with no gap, no overlap, and none empty. Each step is in one closed chapter, or in the open chapter. | `a_long_grind_in_one_zone_still_breaks_into_chapters` |
| 2 | `every_instance_step_is_in_one_visit` | Each step in an instance is in exactly one visit, closed or open, and the tale of that visit has the instance of the step. | `a_wipe_and_a_corpse_run_stay_one_run`, `another_instance_closes_the_open_visit` |
| 3 | `a_closed_chapter_never_changes` | The closed chapters of a log are a prefix of the closed chapters of the log with more steps. | `folding_one_line_at_a_time_gives_the_same_fold` |
| 4 | `a_closed_visit_never_changes` | The closed visits of a log are a prefix of the closed visits of the log with more steps. | `folding_one_line_at_a_time_gives_the_same_fold` |
| 5 | `a_tale_changes_only_with_a_visit` | A step keeps the instance and the first step of each tale. It keeps the weight and the runs, or it adds exactly one closed visit of that tale. `taleMoved_grows`: the weight and the runs only grow. | `a_second_dungeon_run_with_nothing_new_changes_only_its_count` |
| 6 | `a_closed_chapter_has_min_weight` | Every closed chapter weighs `MIN` or more, or a rule step closed it. | `a_break_below_the_least_weight_is_spent` |
| 7 | `a_rule_change_closes_at_most_one_chapter` | A rule step closes one chapter at most. A step of play closes none below `MIN`, and none as a rule. | `a_rule_change_closes_the_open_chapter_below_the_least_weight` |
| 8 | `no_chapter_passes_max_and_one_step` | No chapter weighs more than `MAX - 1 + 7`, and the open chapter weighs less than `MAX`. | `a_long_grind_in_one_zone_still_breaks_into_chapters` |
| 9 | `the_weight_of_a_chapter_is_the_sum_of_its_gains` | The weight of a chapter is the sum of the gains of its steps in the open world. | `revenge_outside_an_instance_counts_for_the_chapter` |
| 10 | `a_step_with_no_gain_closes_nothing` | A step of play that gains 0 closes no chapter. | `a_flight_across_four_zones_makes_no_chapter` |
| 11 | `a_repeat_never_adds_weight` | A step with a spent key gains 0, and the key stays spent after any step. | `the_same_thing_done_again_never_adds_weight` |
| 12 | `repeats_alone_never_make_an_entry` | Steps with spent keys or no key close no chapter, gain 0, add no gain to a visit, and add a tale only for an instance that has none. | `a_second_dungeon_run_with_nothing_new_changes_only_its_count` |
| 13 | `entries_grow_only_with_what_is_new` | The bounds of section 7 of the plan, in six parts (below). | none |
| 14 | `an_instance_step_never_adds_world_weight` | A step in an instance leaves the open chapter and the closed chapters as they were. | `an_instance_step_adds_nothing_to_the_chapter_and_closes_nothing` |
| 15 | `an_instance_has_at_most_one_tale` | No two tales share an instance. | `another_instance_closes_the_open_visit` |
| 16 | `a_death_to_a_beaten_foe_weighs_nothing` | Once a foe is beaten, a death to it gains 0, on either track and under every rule, and the foe stays beaten. | `a_death_to_a_beaten_foe_weighs_nothing` |
| 17 | `deaths_to_one_foe_weigh_at_most_three` | The deaths to one foe gain 3 at most in all. | `deaths_to_one_foe_weigh_two_then_one_then_nothing` |
| 18 | `revenge_counts_once` | The steps of a log add revenge for one foe once at most. | `revenge_counts_once` |
| 19 | `advance.spec`, `chapters.spec` | `advance` and `chapters` give the model. So they never panic, never overflow, and always end. | all the unit tests of `chapters/tests.rs` |

Theorem 13 has six parts. The closed chapters that no rule step closed,
times `MIN`, weigh no more than the gain in the open world
(`worldGainOf`). The chapters that rule steps closed are no more than
the rule steps. The gain of all steps is the sum of the gains of the
keys, so it is at most 7 times the keys. The keys are no more than the
distinct key ids of the steps. The tales are no more than the distinct
instances of the steps. The closed visits with gain are no more than
the gain in instances. A tale writes its text only after a closed visit
with gain, so the tale texts have the same bound. The story program
writes the texts, so no proof counts them.

The triples `chapters_cover_the_steps`,
`chapters_weigh_between_min_and_max`, and
`chapters_have_one_tale_for_each_instance` state theorems 1, 6, 8, and
15 on the Rust function `chapters`. `chapters_holds` turns any law of
`runM` into such a triple.

Theorems 5, 7, 10, 11, 12, 14, and 16 hold from any fold, not only a
reachable one. Theorem 5 needs room for one more closed visit.

The constants `MIN` (15) and `MAX` (40) are those of rule 1, the only
rule so far. A new rule changes `weights::limits`, so the model and
theorems 6, 8, and 13 then name the constants of each rule.

The property tests of the story program (`crates/story/tests/properties.rs`)
check the walk from events to steps, which no proof reads: dense ids,
and the steps of a prefix are a prefix of the steps.

## What is proved: the shapes of a narrator line

The model writes the lore of a narrator line, and the code writes the
hero (`docs/plans/narrator-templates.md`). `narrator_shapes` in
`crates/rules/src/narrator_shapes.rs` builds a line of tokens from a
shape of template parts, decides if the shape fits a moment, checks the
table of parts when it loads, and picks a shape in turn. It reads token
ids, never a string. The story program renders the text. The theorems
are in `Timeways/NarratorShapes.lean`.

| Theorem | The law | Test |
|---|---|---|
| `skeleton.spec`, `assemble.spec`, `fits.spec`, `pick.spec`, `table_ok.spec`, `distinct_skeletons.spec` | The functions never panic, never overflow, and always end. A built line is the lore, then tokens of parts of the shape that are in the table, with at most `MOST_TOKENS` tokens. | the unit tests of `narrator_shapes.rs` |
| `a_line_is_one_shape` | With distinct skeletons, two shapes that build the same tokens are the same shape. | `two_shapes_of_one_line_fail_the_table` |
| `the_lore_comes_first` | When the table passes its checks, a built line starts with the lore slot, and holds it once. | `a_line_starts_with_the_lore` |
| `the_hero_stands_only_in_a_deed` | When the table passes its checks, each hero slot of a built line is in a deed part or a coda part of the shape. | `a_group_that_holds_the_hero_fails_the_table` |
| `an_arrival_holds_no_hero` | The line of an arrival is the lore slot alone. | `an_arrival_is_the_lore_alone` |
| `no_group_clause_holds_the_hero` | When the table passes its checks, no group part and no grow part holds a hero slot. | `a_group_that_holds_the_hero_fails_the_table` |
| `nothing_is_inside_the_hero` | When the table passes its checks, no word of `inside_words` stands right before a hero slot in a line of its shapes. | `the_hero_after_in_fails_the_table` |
| `the_hero_is_named_at_most_once` | A fitting shape builds a line with one hero slot on a named turn, and none on another turn. | `an_unnamed_turn_takes_no_part_with_the_hero` |
| `a_fitting_shape_builds_a_line` | A shape that fits always builds a line. | none |
| `every_slot_has_a_value` | Each slot of a built line has a value in the facts, and a hero slot comes only on a named turn. | `a_slot_with_no_value_builds_nothing` |
| `a_pick_fits` | The pick returns a shape that fits, and it returns one whenever a shape fits. | `nothing_fits_picks_nothing` |
| `no_main_repeats_within_n` | When a fitting shape has a main part outside the window, the pick takes such a shape. | `the_pick_starts_at_the_turn_and_skips_recent_mains` |
| `a_run_never_repeats` | In a run of picks where more than 8 distinct main parts fit at each pick, each pick takes a shape, and two picks at most 8 apart never share a main part. | the same |
| `the_pick_is_deterministic_from_the_turn` | When a fitting shape has a fresh main part, the pick is the first such shape from `turn mod count`. | the same |

These laws differ in form from the plan. Each keeps its intent:

- **The specs** give the skeleton as "the lore, then tokens of parts of
  the shape" (`SkelOk`), not as an exact concatenation. The laws need
  no more.
- **`the_lore_comes_first` and `the_hero_stands_only_in_a_deed`** need
  no `s ∈ ss`: the checks of the parts hold for every shape. Law 4
  says that the hero slot is in the tokens of a deed or coda part of
  the shape. The plan named the kind of the part at each position. That
  form needs a map from a position to its part, and says the same.
- **`every_slot_has_a_value`** writes `f.has k` as the value of
  `has` at `k`, in range.
- **`a_pick_fits`, `no_main_repeats_within_n`, and
  `the_pick_is_deterministic_from_the_turn`** need `mains` and `fits`
  to have one length. The Rust returns no shape when they differ, so the
  plan's form of law 10 is false without it. Law 13 needs a fitting
  shape with a fresh main part: with none, the pick takes the main part
  that was used longest ago, and law 10 covers that case.
- **`a_run_never_repeats`** is a Hoare triple over `runPicks`, a run
  that this file defines from the Rust `pick`. Each pick reads the main
  parts of the last 8 picks, oldest first. "More than 8 fitting main parts"
  is `Wide`: a list of more than 8 distinct main parts, each the main
  part of a fitting shape. The window is the Rust `WINDOW`. The file
  writes it as `windowSize`, because no translated function reads
  `WINDOW`, so Aeneas does not translate it.
- **The window of the story program is not the window of the run.**
  The story program reads the last 8 accepted lines of a character. An
  arrival line takes a place in that window but has no main part, and
  a refused line takes none. So the program keeps "no repeat within 8
  accepted lines", and two picks with an arrival between them can be
  further apart than in `runPicks`. The property test
  `no_main_part_repeats_within_eight_lines` checks the window of the
  program.
- **The choice of the shape is glue.** Laws 10 to 13 hold for each call
  of `pick`. `pick_shape` in `crates/story/src/narrator_build.rs` calls
  `pick` up to three times: a kill on a named turn first tries the
  shapes with no hero, and a naming that fits no shape tries the other
  naming. No proof reads that order. The test
  `a_kill_prefers_an_unnamed_part_on_a_named_turn` and the property test
  above check it.

## What is proved: silence when the lore is thin

`is_silent` in `crates/rules/src/thin_lore.rs` decides if a narrator
moment gets no line (GAMEPLAY.md 3.2). It reads ids, never strings.
`subjects` holds the ids of what the moment is about. `lore` holds the
ids of what the passages of the prompt are about: the page, the links,
and each subject that the shown text names. The glue in
`crates/story/src/narrator_lore.rs` gives one id to each distinct name.
No proof reads the glue. Its named tests and property tests check it.

| Theorem | The law | Test |
|---|---|---|
| `is_silent.spec` | The rule ends and gives its pure model: it is true exactly for a deed with no id of `lore` in `subjects`. | the unit tests of `thin_lore.rs` |
| `the_rule_never_panics` | For every kind and every two lists of ids, the rule gives a value. | none |
| `a_deed_with_no_lore_of_its_own_is_silent` | A deed whose passages have no subject among its subjects is silent. No passage is the empty list. | `a_deed_with_lore_about_another_subject_is_always_thin` |
| `a_deed_with_lore_of_its_own_speaks` | A deed with a passage about one of its subjects is not silent. | the same |
| `an_arrival_is_never_silenced_by_this_rule` | An arrival is never silent by this rule, with lore or without. | `an_arrival_is_never_silenced_by_the_thin_lore_rule` |

## What you trust

1. **Charon and Aeneas.** A bug in the translation makes the Lean
   differ from the Rust.
2. **`Timeways/FunsExternal.lean`.** Aeneas does not translate std.
   This file gives a body to each std item that the code uses:
   `Option::clone`, `String == String`, and `String::clone`. Read it
   before you trust a theorem. It is short. The Aeneas library models
   the other std items, such as `saturating_add`, `clamp`, and
   `unwrap_or`.
3. **The three standard axioms of Lean.** `Timeways/Axioms.lean` pins
   the axioms of each theorem with `#guard_msgs`. A `sorry` or a new
   axiom fails the build. Every pin names `propext`,
   `Classical.choice`, and `Quot.sound`, or fewer of them.
4. **The glue in the story program.** `crates/story/src/quest/log.rs`
   turns each line into a `Change` and each `Quest` back into a
   `Tracked`. `narrator::Budget` turns a `Tick` into seconds and back.
   The SQL of `store::database` clears the prompts before
   `oldest_prompt_kept`. No proof reads the glue. The tests of the story program cover it.
5. **Two starts.** `fresh` in `Timeways/Budget.lean` is the budget
   that `Budget::default()` makes. `trustsAfter` in
   `Timeways/TrustBand.lean` applies `next_trust` one change after the
   other, as `Character::change_trust` does.

## How to run

The tools are pinned to Aeneas commit `fd27c97` (see `lakefile.toml`).
Lean is `v4.31.0`, from `lean-toolchain`. Hourglass uses the same
pins, so one Aeneas build serves both repositories.

1. Install opam with OCaml 5, and install the packages that the
   Aeneas README lists.
2. Clone Aeneas and check out the pinned commit.
3. In the Aeneas directory, run `make setup-charon`, and then run
   `make` inside the opam environment.
4. Translate the crate: `AENEAS=<aeneas dir> lean/extract.sh`
5. Check the proofs: `cd lean && lake exe cache get && lake build`

CI (`.github/workflows/ci.yml`) runs all of this on each push. Its
`lean-extract` job fails when the committed Lean differs from the Lean
that Aeneas makes from the current Rust. The extraction also fails
when an item of the Aeneas template has no body in the model, and
when Aeneas cannot translate a function and writes a partial file.

The extraction writes `Timeways/Types.lean` and `Timeways/Funs.lean`.
Never edit these two files by hand. The extraction never overwrites
`Timeways/FunsExternal.lean`. When Aeneas needs a new std item, it
writes `Timeways/FunsExternal_Template.lean`. Compare that file with
the model, and add a body for each new item.

## How to verify one more function

Put the function in `crates/rules`, and put this mark on it:

```rust
#[cfg_attr(charon, verify::start_from)]
pub fn quest_log(changes: &[Change]) -> Vec<Quest> {
```

The extraction translates each marked function and everything it
calls. The mark exists only under `--cfg charon`, so a normal build
never sees it.

## Known Aeneas limits

Each limit has a workaround in the code. Keep the workarounds.

1. **No closure and no iterator adapter.** Aeneas translates no
   `iter().filter()`, `map()`, `any()`, or `find()`. The code walks by
   index with `while`. Each such function says so in its doc comment.
   This is an approved exception to the readability rules. The same
   goes for the clippy fixes that bring a closure or a range back:
   `lib.rs` allows `manual_map` and `manual_range_contains`.
2. **No `?` on an `Option`, and no `saturating_mul`.** Aeneas writes
   each one as an axiom with no body. The code uses `match`, and a
   plain `*` where the product never overflows (`u8` days times
   `DAY_SECONDS`).
3. **A field named `done`.** In Lean, `done` is a tactic, and the
   generated code does not parse. The field is `done_times`.
4. **A local with the name of a module.** A local named `quest` hides
   the module `quest` in the generated Lean. The module is `quest_log`.
5. **A `match` that gives a tuple.** `matches!` on a field of a
   mutable borrow makes Aeneas give back the field and the answer as a
   pair, and the proof cannot see through the pair. The code compares
   with `==` instead.
6. **A `match` on a slot of an array.** Aeneas stops with "Not
   allowed to expand enumerations with several variants" on
   `match self.spoken[0]` and on an `if let` chain over it. The code
   copies the slot to a local first. Aeneas then writes a partial file
   and exits with 0, so `extract.sh` checks its output.
7. **No `rotate_left` of an array.** `Budget::take` builds the new
   array from the old slots.
8. **No crate outside std.** Aeneas sees only the code it translates.
   So `timeways-rules` holds no serde, no Hourglass, and no `Tick`. A
   time is a `u64` of seconds, and the story program converts.
