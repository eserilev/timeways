# Timeways: rules for agents

Read `GAMEPLAY.md` before you change code. It is the source of truth for the design.

The two priorities of this project are readability and test coverage.
If a change makes the code harder to read or less tested, do not make it.

## UI copy

UI copy is every text a player reads: buttons, tabs, labels, page lines, empty states, hints, popups, chat lines, errors, and the README. It does NOT follow the simple-English (ASD-STE100) rule. That rule is for docs, comments, and commits. UI copy follows this section.

- **Name the player's goal, not the mechanism.** A button says what happens for the player, not what the code does.
- **Use words players already know.** Take the words of the WoW UI (Accept, Decline, Abandon, Okay, Cancel, Delete) and of common apps (Send, Retry, Save, Edit). Never invent a term when a common one exists.
- **Buttons: one or two words, starting with a verb.** "Accept", "Add a note", "Save".
- **Talk like a person, like a modern game UI.** Short everyday words. Contractions are fine. Sentence case for lines, title case only where WoW uses it (tabs, headings). No robot form language, and no fake old-timey flourish.
- **No internal words.** Never show "story program", "bridge", "slot", "strip", "batch", "fact", "tick", "model", or "bard" to a player.
- **Errors say what went wrong and what to do next, with no blame.** Keep the player's input. Leave out the fix when a button next to the error already offers it.
- **Cut every word that does not help.** Key fact first. A status is a few words: "Saving...", "Loading...".
- **One name for each thing everywhere.** Quests are "quests" on every page and in every chat line, also the ones that players give. Never show "task". The Knowledge tab is "Knowledge".
- **UI copy is not the story voice.** The narrator's voice (the keeper of time) lives only in model output. The UI never performs a voice.

Bad and good, from this project:

| Bad | Good | Why |
|---|---|---|
| Who you are, in your own hand. | Who You Are | Fake flourish. Nobody talks like that. |
| What did you do before the road called? | What did your character do before adventuring? | A plain question, the way a character creator asks it. |
| Your hero in your own words. The narrator and the bard read it. | Tell us about your character. It shapes your story. | Robot form language, and "the bard" is an internal name. |
| Leave it empty to clear it, Enter saves it. | (only the question, with Save and Cancel) | The buttons already say it. |
| `[1] wowpedia.fandom.com/wiki/...` under a `/lore` answer | (the answer alone) | The player asked a question, not for sources. |
| Thinks little of you. (for a neutral NPC) | Neutral | The words must match the feeling. |

Sources: [Microsoft Style Guide, "simple and human"](https://learn.microsoft.com/en-us/style-guide/brand-voice-above-all-simple-human), [Nielsen Norman Group, error-message guidelines](https://www.nngroup.com/articles/error-message-guidelines/), [Material Design, writing](https://m3.material.io/foundations/content-design/style-guide/ux-writing-best-practices), [Apple HIG, writing](https://developer.apple.com/design/human-interface-guidelines/writing).

When you write copy with an LLM, give it where the text shows, a length limit, the tone, and these examples. Examples work better than rules.

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
- A rule that holds for any play gets a property test in `crates/story/tests/properties.rs`. Make the edges of a number likely: a uniform draw almost never reaches them.
- Code that reads text from outside (the bridge, a file, a model) gets a fuzz target in `fuzz/`. A bug that a fuzzer missed becomes a seed in `fuzz/seeds/`.

## Checks

Run these before each commit. CI runs them too.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check
```
