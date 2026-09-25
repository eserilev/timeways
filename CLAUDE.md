# Timeways: rules for agents

Read `GAMEPLAY.md` before you change code. It is the source of truth for the design.

The two priorities of this project are readability and test coverage.
If a change makes the code harder to read or less tested, do not make it.

## Comments

A comment says why, never what. If a comment repeats the code, delete it.

- Most functions get no comment. A good name does the job.
- A doc comment is one line. Add more only for a surprise, a trap, or a contract that the types do not show.
- Do not write "This function...", "Note that...", "It is important to...", "robust", "ensures", "handles".
- A `TODO` says what is missing and when it goes away.
- Write comments in simple English: short sentences, active voice, no "should", "may", "might", "could", "would".

## Readability

- Names over comments.
- One job per function. If you need "and" to describe it, split it.
- Flat control flow: early returns and `?`. No deep nesting.
- No clever code. No macro where a function works. No generic with one caller. No iterator chain longer than three steps.
- Newtypes for IDs. Enums in place of `bool` flags.
- One module, one idea. The file name says what is inside.
- No `unwrap()` or `expect()` outside tests.
- Fact names are the constants of `vocabulary.rs`. Never type a fact name as a string literal.
- Errors: `thiserror` in library code, `anyhow` only in `main.rs`.

## Tests

- Every rule in `GAMEPLAY.md` that the code implements has at least one named test.
- Test names are sentences: `the_dead_stay_dead`, not `test_dead_2`.
- Each test reads top to bottom: arrange, act, assert.
- No test needs the game or a real model, except tests marked `#[ignore]` for live runs.
- A bug fix starts with a failing test.

## Checks

Run these before each commit. CI runs them too.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check
```
