# Testing in the game

The steps to test Timeways in WoW: Forever, and what to send back.

## The test setup

The game runs a stable copy of the code, so work in the repo does not change the game in the middle of a test.

- **The addon:** the `Timeways` link in the AddOns folder points to `../timeways-test/addon/Timeways`. That folder is a git worktree of commit `c546e70`.
- **The story program:** `~/.local/bin/timeways-story`, built from the same commit. The old program is `~/.local/bin/timeways-story.old`.
- **The world files:** `~/.local/share/gnomish-relay/timeways/story/worlds/r_<realm>/c_<name>.*.jsonl`.

## Before the first test

1. Run `gnomish-relay restart`, so the bridge starts the new story program.
2. Update your Gnomish Relay checkout, or turn off the GnomishRelay addon.
3. Start WoW again. A `/reload` is not enough after a change to the list of addon files.
4. In the game, type `/console scriptErrors 1`, so each Lua error shows.

## The tests

Do them in order. Each batch of events goes out about once a minute, so wait a minute before you check a result.

### 1. The chain works

1. Type `/timeways test`.
2. Wait for the line "3 of 3 checks passed." It can take a minute or two.

The self-test sends two requests that only read your world: the first page of the journal, and one `/lore` question. The lore question can cost one model call. Each check shows as pass or fail in the chat. If a check fails, the line gives the reason.

The last report stays in the saved variables, as `TimewaysDB.selfTest`. WoW writes it to the disk at `/reload` and at logout.

### 2. The text that you read

1. Talk to an NPC with gossip: an innkeeper or a guard.
2. Open a quest. Accept it, or close it.
3. Turn in a quest.
4. Read a book: for example, a book in the library of Stormwind, or a book on a table.
5. Wait a minute.
6. Ask `/lore` about a word from one of those texts.

The answer uses that text, and says where you learned it.

### 3. A side quest

This test needs a model. Without one, the NPC "has no task for you now", and the rest of the test does not apply.

1. Visit two subzones, and talk to two NPCs there.
2. Target one of those NPCs, and type `/quest`.
3. Wait a minute. The narrator line shows the offer.
4. Type `/quest accept`. Open `/journal` on the Quests page.
5. Do the steps of the quest. Check the page after each step.

The quest shows "Done on" with the day, and the giver trusts you more on the People page.

### 4. Your name stays private

Open `c_<name>.seen.jsonl` in the world folder. Your character name is not in the file. `$N` stands in its place.

### 5. Open questions of the design

1. While a book from a table or a shelf is open, type `/dump UnitName("npc")`. Write down what it shows.
2. Kill a rare, and die to a mob. Open `/journal` and check the Deeds page.

## What to send back

- Each Lua error, as text.
- The chat lines of `/timeways test`.
- What the lore book showed for each `/lore` question.
- The offer line of `/quest`, and the Quests page at the end.
- The first 3 lines of the `.seen.jsonl` file.
- The result of each open question.

## After the test

To go back to the working copy of the addon:

```sh
ln -sfn ~/Documents/Code/Personal/timeways/addon/Timeways \
  "$HOME/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_classic_beta_/Interface/AddOns/Timeways"
```

# Mutation checks

`cargo mutants` changes the code in small ways, one at a time, and runs the tests. A change that no test catches shows a rule with a weak test.

- `.cargo/mutants.toml` lists the files whose tests catch every mutant. The nightly job checks them and fails on a mutant that survives.
- To add a file, run `cargo mutants -p timeways-story -f <file>`. Add a test for each mutant that survives, until none does. Then add the file to the list.
- A timeout counts as caught: the mutant loops forever.
