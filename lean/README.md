# The Lean proofs

Aeneas translates the crate `timeways-rules` (`crates/rules`) into pure
Lean functions. The theorems in `Timeways/QuestLog.lean` are about
those functions. A theorem holds for every input, with no bound. The
property tests in `crates/story/tests/properties.rs` check the same
rules on random input, and they stay as a second check.

The Lean is generated from the Rust, so it does not drift from the
Rust. The story program calls `timeways-rules` for each question about
a quest, so the proofs speak about the code that runs. After a change
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

All seven laws come from one invariant, `Good`, in
`Timeways/QuestLog.lean`. Each kind of line keeps it:
`Progress.offered.spec`, `Progress.finish_step.spec`,
`Progress.count_kill.spec`, `good_accept`, `good_status`, and
`apply.spec`.

## What you trust

1. **Charon and Aeneas.** A bug in the translation makes the Lean
   differ from the Rust.
2. **`Timeways/FunsExternal.lean`.** Aeneas does not translate std.
   This file gives a body to each std item that the code uses:
   `Option::clone`, `String == String`, and `String::clone`. Read it
   before you trust a theorem. It is short.
3. **The three standard axioms of Lean.** `Timeways/Trust.lean` pins
   the axioms of each theorem with `#guard_msgs`. A `sorry` or a new
   axiom fails the build. Every pin names exactly `propext`,
   `Classical.choice`, and `Quot.sound`.
4. **The glue in the story program.** `crates/story/src/quest/log.rs`
   turns each line into a `Change` and each `Quest` back into a
   `Tracked`. No proof reads the glue. The tests of the story program
   cover it.

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
when an item of the Aeneas template has no body in the model.

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
6. **No crate outside std.** Aeneas sees only the code it translates.
   So `timeways-rules` holds no serde, no Hourglass, and no `Tick`. A
   time is a `u64` of seconds, and the story program converts.
