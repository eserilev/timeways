# Plan: the alias table

Status: built, first version, 2026-10-04. The rules are in `GAMEPLAY.md` 5.11, 4.7, and 4.8. This plan keeps the decisions and their reasons.

## 1. Goal

A model never sees the name of a real player. A story or an idea still keeps its meaning. Each player gets a stable ID, `P1`, `P2`, and so on, and the ID is never reused. The model sees the ID with a card: "{P7}: a Forsaken mage". The player's own screen shows the real name again. An ID never shows to the player.

## 2. Where the table lives

**Decision:** in the world of each character, as the row table `aliases` of its SQLite file. The story program keeps it.

The reasons:

1. **No relay change.** `GAMEPLAY.md` 5.11 said "a local table of the bridge". The bridge belongs to Gnomish Relay, which another session owns. The story program runs on the same computer as the game, so the table stays local and rule 5 holds.
2. **The proofs.** The rules of the table are Rust in `crates/rules`, so Aeneas and Lean prove them. Rules in the addon are Lua, and no proof reads Lua.
3. **One transaction.** A story and the IDs that it holds go into the same file, in the transaction of their line (5.7). A crash never leaves a story with an ID that the table does not hold.
4. **One world, one table.** The world of a character is the unit of play. Later, a `Person` entity of Hourglass takes the ID as its name, in the same world.

Other choices, and why not:

- **The addon keeps the table.** No proof reads it. Any addon can read and write saved variables. The journal then needs the table to swap the IDs back, so the table goes to the desktop anyway.
- **`timeways.sqlite`**, shared by all characters (`docs/plans/links.md` planned this). A story and its IDs then land in two files, in two transactions. And two players with the same name on two realms share one key.

## 3. What the addon sends

The addon already finds the players that it knows in a text (`TaskNames.lua`). It now marks each one in place of "my friend":

- "tell corvin, CORVIN-Stormrage" becomes "tell {Corvin}, {Corvin}". The mark holds the name as the game writes it, with no realm.
- Your own name becomes `$N`, as before.
- A brace that a player typed becomes a parenthesis. So a typed text never holds a mark or an ID.
- For each marked player in sight (the target, the mouseover, or a member of the group), a `player_described` line goes first, with the file tokens of the race and the class. The card comes from it.

**Why marks, and not a list of names on the line.** The bridge checks `draft_asked` with an exact shape: `at` and `idea`, and no other key (relay SPEC 9.8). A new key needs a relay change. A mark lives inside the idea, so the shape stays. `story_accepted` and `player_described` are game events, and the bridge checks only their envelope.

The desktop measures the limits of a story and an idea on the text without its marks. So a mark never makes a story too long. This also fixes a part of point 2 of `docs/plans/hero-stories.md`: "my friend" made a story longer than the limit, and a mark does not.

## 4. Scope of version 1

- `story_accepted`: the world keeps the story with IDs. The journal gives it back with the names.
- `draft_asked`, for "Help me write" and for a typed step: the prompt holds IDs and cards. The draft goes back with the names.
- Out: the words of `/talk` and `/lore`. They are the player's own words (5.11). A swap there is cheap, because the table exists. But a common word can be a player name ("Bread"), so the swap waits for a decision of the user.
- Kept: "No story reaches a model" (4.8). The story keeps IDs, so a later version can give it to the summary or the narrator. The user decides.

## 5. Name matching

The same rules as `TaskNames.lua`:

- A word is a run of ASCII letters and digits and of bytes outside ASCII, so "é" is part of a word.
- A name matches a whole word, in any case. The story program folds the case with `to_lowercase`, which covers every letter that `TaskNames.Fold` covers.
- "Ada-Stormrage" is one word when the table holds "Ada". "Bob-Ada" with only Ada known keeps "Bob-" and swaps "Ada".
- A name holds only letters, at most 48 bytes, with the realm cut off. "P7" holds a digit, so it is never a name.

## 6. The proofs

Lean proves these laws for every input, with no bound (`lean/Timeways/Aliases.lean`):

| Theorem | In plain words |
|---|---|
| `an_id_is_never_reused` | After any sequence of lines, each ID names the player that it named before. A new player gets a new ID at the end. |
| `a_name_keeps_its_id` | Once a name has an ID, it keeps that ID over any sequence of lines. |
| `two_names_never_share_an_id` | From an empty table, no sequence of lines gives one name two IDs, or two names one ID. |
| `one_id_names_one_player` | Two names with the same ID are the same name. |
| `every_name_of_a_line_gets_an_id` | After a line, each name of the line has an ID. |
| `no_known_name_after_the_swap` | The text for a model holds no word whose name the table knows. |
| `every_id_of_the_swap_is_in_the_table` | Each ID in the text for a model names a player of the table. |
| `the_swap_and_back_keeps_the_text` | The swap to IDs and back gives the text back, piece by piece. A known name comes back in the form that the table holds. |
| `restore_keeps_what_is_no_known_name` | A piece that is no known name comes back exactly. |

**A full round trip is not true**, so the law is weaker on purpose. "ADA-Stormrage" comes back as "Ada": the ID keeps who the player is, not how the text wrote the name. The law says exactly what comes back.

**What no proof reads:** the cut of a text into words in `crates/story/src/aliases.rs`, the case fold, and the marks. Property tests and a fuzz target cover them.

## 7. Tests

- Property tests (`crates/story/tests/properties.rs`): `an_alias_is_never_reused_over_any_lines`, `the_text_for_a_model_holds_no_known_name_as_a_word`, `the_swap_and_back_gives_each_name_in_the_form_of_the_table`, and `no_prompt_holds_a_known_player_name`. The names sit often at the edges: empty, a prefix of another name, in another case, with a realm, outside ASCII, "P7", a text of names only, and 48 and 49 bytes.
- Fuzz target `player_names`: a random text, random names, a random answer of a model, and a random row. The cut joins back to the same text, the text for a model holds no known name, and the player never reads an ID.
- Unit tests in `crates/rules/src/aliases.rs`, `crates/story/src/aliases.rs`, `crates/story/tests/stories.rs`, and `crates/story/tests/drafts.rs`.
- Lua tests in `crates/addon-tests/tests/task_names.rs`, `player_stories.rs`, and `task_form.rs`.
- In the game: test 20 of `TESTING.md`.

## 8. Later

1. **A fresh card.** The first card of a player stays. A later `player_described` changes nothing. A newer card needs a new kind of row, because rows only grow.
2. **Stories in a prompt**, after the decision of the user (4.8).
3. **Typed words** of `/talk` and `/lore`, after the decision of the user (5.11).
4. **The guild and Hourglass:** a `Person` entity for each ID, and the swap for shared text (5.11, point 8).
5. **Two players with one name** on two realms share one key. In one world this is rare: a character plays on one realm.
