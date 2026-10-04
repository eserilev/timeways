# Plan: links, proof, and one database

Status: draft 3, 2026-10-04. Draft 3 makes the plan say what the code does. Steps 1 to 8 are built. Step 7 is built for trust only, and step 8 is a first version with no story in a prompt. The Deeds filter of section 7 is not built. Their rules are in `GAMEPLAY.md` 3.6, 4.8, 5.7, and 5.14. When a part is built, its rules move into `GAMEPLAY.md`, and this plan marks the part as done.

Nothing is live, so there is no migration. A change of the tables deletes the test worlds.

## 1. Goal

Each row in the world of a character answers three questions:

1. **What is the proof?** Which game input, or which player, stands behind this row.
2. **Where did it come from?** Which model call wrote this row, and what that call read.
3. **What uses it?** Which accepted model calls read this row.

Three features need these answers:

- **Player stories** (section 9). A player withdraws a story about them only while nothing uses it. The proof of a story is the play that both players share.
- **A "Why?" link in the journal.** A quest, a trust change, or a saga shows the moments behind it.
- **Debugging.** We see what the model saw, and why it said what it said.

## 2. Two databases

| File | Holds | Why |
|---|---|---|
| `worlds/r_<realm>/c_<name>.sqlite` | the world of one character: its rows, inputs, calls, and links | a character is the unit of play |
| `timeways.sqlite`, in the data folder | what all characters share: the alias table (5.11), the budget, and the pace of the narrator | these cross characters |

`Store::Memory` opens both as in-memory SQLite. So every query works in tests, and `Option<Database>` goes away.

## 3. What goes in, and what stays out

**In the world database:**

| Data | Table |
|---|---|
| Hourglass events | `events` (exists) |
| Sagas, flavor, hero, learned, quests | their tables (exist) |
| Each input line of the addon that changes the world or asks a question | `inputs` |
| Each model call: what it was for, the prompt, the answer, the result | `calls` |
| What each call read | `reads` |

Narrator lines, lore answers, talk answers, drafts, and the drafts and pick of a saga all live in `calls`, as answers.

**In `timeways.sqlite`:** the alias table, the budget window, and the pace. A restart then never lets the narrator speak 3 times at once.

**Out, on purpose:**

- **The state of the world.** A replay of `events` builds it. A stored copy can disagree with the history.
- **Memory of one run:** the open calls, the queue, the journal pages of the last request, and the notice that waits. Each one ends with its run, and nothing reads it later.
- **A line that does not parse.** It changes nothing. It goes to stderr, as today.
- **Player tasks (4.7).** They live in the saved variables of the addon, and the addon cannot use SQLite.
- **The lore pack.** It is a SQLite file of its own, shared by all characters. A call stores the version of the pack that it used.
- **The search index of seen text.** It stays in memory. A stored copy can disagree with `learned`. Measure the start time first, and move it only if the start is slow.

## 4. Proof columns and the `reads` table

Every row table (`events`, `chapters`, `flavor`, `hero`, `learned`, `quests`, `stories`) gets two columns:

```sql
input INTEGER REFERENCES inputs (position),  -- the line that made the row
call  INTEGER REFERENCES calls  (position)   -- the model call that made the row
```

- A row from a game event has `input` and no `call`. Example: "defeated Hogger".
- A row from a model answer has `call`. The call has its own `input`: the line that asked for it. Example: a trust change from `/talk`.
- SQLite checks both columns (`PRAGMA foreign_keys = ON`). A link to a row that does not exist is an error, not a bug that waits.
- No link cascades. Only another program deletes a line or a call. The open then clears each link to it, so the row shows as Lost (section 8).

One table holds what each call read. It is the only many-to-many link:

```sql
CREATE TABLE reads (
  call  INTEGER NOT NULL REFERENCES calls (position),
  tab   TEXT    NOT NULL,   -- a row table, or 'calls'
  row   INTEGER NOT NULL
);
CREATE INDEX reads_of_a_row ON reads (tab, row);
CREATE INDEX reads_of_a_call ON reads (call);
```

- `reads` points into the row tables and into `calls`, so SQLite cannot check it with one foreign key. A property test checks it instead (section 12).
- **Links only point back in time.** A call reads rows that exist when it opens. A row names the call or input that came before it. So the graph has no cycles.

## 5. Addresses

- **The address of an event is its `EventId`.** The store writes `position` explicitly for each event, from 0. So a cut and a new write never shift it.
- Every other table gets an explicit position too. A row of a row table has its place in its table, from 0. `inputs` and `calls` take the next position after the largest one at open.
- **A world fact has the address of its opening event** (`Fact.opened`).
- A `read` of a fact is a `read` of that event.

## 6. What a call reads

The prompt builders read derived values, such as deeds, chapters, and moments, not rows. Threading ids through `Journal` and `Page` makes those types bigger, and the addon gets them under a size limit. So the plan does not do that.

**The reads come from rules,** one function for each kind of call, in one module (`story/reads.rs`). Each rule takes the character and what the call was for. It returns rows. The rules read more than the prompt, never less. For "uses", reading too much is the safe side. The rules as built are in the table of `GAMEPLAY.md` 5.14.

- The memory of a saga (earlier chapters) comes from facts, never from earlier sagas (`memory.rs`). So a saga reads events, not sagas.
- A lore passage from the pack gets no `read` row. Its id changes with each pack. The call keeps the pack version.
- **The reads go in when the call opens.** So a row that is withdrawn while the call runs still counts as read.
- **Only an accepted call counts as a use.** A refused or failed call reads, but uses nothing.

## 7. Proof

The proof of a row is the set of its roots. To find them, follow `input` and `call` back. For a call, follow its `reads`. Stop at an input. A recursive CTE does this in one query.

| Root | Means |
|---|---|
| Game | an input from the addon of a game event |
| Player | an input that the player typed: a hero entry, a `/talk` text |
| Shared | a player story from another player (section 9) |
| Lost | a row whose input or call is gone. Only another program makes this. |

- **The weakest root wins** for display: Lost, then Shared, then Player, then Game.
- **Model is not a proof.** It says how a row was made, and the `call` column already says that.
- **Not built: a Deeds page of Game proof alone.** Rule 4 says that the game is the truth, so the Deeds page was to show only rows with Game proof alone. Today it shows every deed. No test checks it yet.
- Example: a trust change from `/talk` has the roots {Player, Game}. Player wins, so it shows as resting on your words.

## 8. Rules that keep the graph whole

- **One transaction for each line.** The rows, the input, the call changes, and the reads of a line go in together. A failed save drops all of them, and the character opens again from the disk (5.7, built).
- **A call writes only to the database of its character.** An answer for a character that is not active writes nothing, and its call stays `open`. A lore call gets no row, because a lore answer changes no world.
- **The cut at open** runs in one transaction. It deletes a row that does not read, every row after it in its table, and every read of them. A link to a line or a call that is gone becomes NULL, so the row shows as Lost.
- **A row is never deleted to withdraw it.** A withdraw is a new row, as `Removed` is for the hero. Tables only grow.
- **The `CallId` of the bridge restarts at 1 in each run.** Each open call keeps its row in `calls` in memory, next to the key of its character. So an answer after a change of character, or after a reopen, still rests on its call.

## 9. Player stories (later)

A player tells a story about another player in the party. The other player accepts or declines it. Before it, three things must exist:

1. **The alias table (5.11).** A story names real players. No real name goes into a prompt, so a story reaches a model only with aliases.
2. **A party input from the addon.** Today the addon tracks the group only for player tasks, and sends nothing to the desktop.
3. **The proof levels of player tasks** (`TaskProof.lua`: Witnessed, Seen, Not confirmed). A story about shared play uses the same levels, from the inputs of the reader in the same stretch of party time.

The internal name is "player story". `flavor::Told` already means a telling of a flavor moment.

## 10. Privacy (5.11)

- `inputs` holds what the player typed in `/talk` and `/lore`, as the world holds today. The file never leaves the computer.
- A prompt holds that text too, so it can hold a player name that the player typed. That is the player's own choice (rule 5).
- **Dropped: a "clear my history" action.** A `/lore` question changes nothing, so it is not kept at all. What is kept is what the proof needs, so nothing is left to clear.
- After 500 calls, the store clears the prompt of the oldest call. The row, its answer, and its links stay.

## 11. Build order

Each step is one commit or a few, with its tests and its rules in `GAMEPLAY.md`.

1. **Done. The base.** In-memory SQLite for `Store::Memory`. Event position = `EventId`. Foreign keys on. The cut at open in one transaction.
2. **Done. `inputs` and the `input` column.** Only the `character_entered` that founds a world is kept, because every batch starts with one. The stored input is the parsed line after the clock check, also for a refused line.
3. **Done. `calls` and the `call` column.** Every call gets a row when it opens, and its answer and result when it ends. A lore call gets no row: it changes no world.
4. **Done. `reads`**, with the rules of section 6, one kind of call at a time: saga, quest, talk, narrator. A lore call has no row, so it has no reads.
5. **Done. The queries:** `proof_of(row)`, `source_of(row)`, and `uses_of(row)`.
6. **Done. `timeways.sqlite`:** the budget and the pace first. The alias table comes with 5.11.
7. **Done, for trust only. "Why?" in the journal.** The reads of a call hold far more than its cause, so only trust shows a why: one step back from its newest change. Quests, chapters, and deeds show none. The Deeds filter of section 7 is not built.
8. **Done, first version. Player stories** (section 9, and `GAMEPLAY.md` 4.8). No story reaches a prompt, so the alias table and a party input wait for the next version.

## 12. Tests that the plan needs

- `an_event_keeps_its_position_after_a_cut`
- `a_refused_line_keeps_its_input_row`
- `a_failed_save_keeps_no_input_call_or_read`
- `an_answer_for_another_character_writes_nothing_here`
- `a_row_whose_line_is_gone_is_lost`
- `a_refused_call_uses_nothing`
- `a_hero_entry_removed_during_a_call_still_counts_as_read`
- `the_pace_of_the_model_comes_back_after_a_restart`
- `a_trust_change_from_talk_rests_on_the_words_of_the_player`
- `an_answer_after_a_relog_rests_on_its_call`
- Not built: `the_deeds_page_shows_only_game_proof`, with the filter of section 7.

Property tests in `properties.rs`, for any play:

- Every `reads` row points to a row that exists, and that was saved before its call.
- A restart at any point leaves the same rows and links. This extends `a_restart_at_any_point_leaves_the_same_story`.
- The proof of each row is the same before and after a reopen.
- Every event has an `input` or a `call`, also when an answer comes after a change of character.
- No row rests on a Lost root after play.

The fuzz target of the store also writes random `reads` rows and proof columns. The open never fails on them.

## 13. Open questions

1. **Prompts kept.** Is 500 right? Measure the file after a long session.
2. **The search index.** Is the start slow enough to store it?
3. **A "Why?" for the addon.** How much of a proof chain fits in one journal reply?
