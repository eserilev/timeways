# The Lean proofs

Aeneas translates the crate `timeways-rules` (`crates/rules`) into pure
Lean functions. The theorems in `Timeways/QuestLog.lean`,
`Timeways/HeroHook.lean`, `Timeways/Budget.lean`,
`Timeways/TrustBand.lean`, `Timeways/Prompts.lean`,
`Timeways/Aliases.lean`, and `Timeways/StoryShelf.lean` are about those
functions. A theorem holds for every input, with no bound. The
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
text, and IDs. The rules compare keys only. `learnLines` applies
`learn_all` one line after the other, as the story program does.

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
story program only appends a line that lands, so every shelf of a
world is `Landed`.

| Theorem | The law | Test |
|---|---|---|
| `lands.spec`, `standing.spec` | The two functions never panic, always end, and give their pure model. | the unit tests of `story_shelf.rs` |
| `a_number_stands_at_most_once` | On a landed shelf, no number stands twice. | `the_shelf_keeps_each_number_once` |
| `a_removed_story_never_comes_back` | After a removal, the number never stands again, whatever lines come after it. The shelf need not be landed. | the same |
| `a_used_story_always_stands` | A story that a call used and that stands, stands after any line that lands. | the same |

The glue in `crates/story/src/stories.rs` turns each row into a
`ShelfLine`, and asks the database which stories an accepted call read.
No proof reads the glue. The property test checks it, with numbers at
the edges of a `u64` and numbers that come again.

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
