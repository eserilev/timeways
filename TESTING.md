# Testing in the game

The steps to test Timeways in WoW: Forever, and what to send back.

## The test setup

The game runs a stable copy of the code, so work in the repo does not change the game in the middle of a test.

- **The addon:** the `Timeways` link in the AddOns folder points to `../timeways-test/addon/Timeways`. That folder is a git worktree of one commit. To see which one, run `git -C ../timeways-test log -1 --oneline`. To move it, run `git -C ../timeways-test checkout --detach <commit>`, and build the story program again.
- **The story program:** `~/.local/bin/timeways-story`, built from the same commit. The old program is `~/.local/bin/timeways-story.old`.
- **The world files:** `~/.local/share/gnomish-relay/timeways/story/worlds/r_<realm>/c_<name>.sqlite`. A world from before SQLite moves into this file at the first login, and its old `.jsonl` files stay.

## Before the first test

1. Run `gnomish-relay setup --timeways`. Setup writes the key into the `Timeways_Key` addon, next to the `Timeways` folder, and deletes an old `Key.lua` in the `Timeways` folder.
2. Run `gnomish-relay restart`, so the bridge starts the new story program.
3. Update your Gnomish Relay checkout, or turn off the GnomishRelay addon.
4. Start WoW again. A `/reload` is not enough after a change to the list of addon files.
5. In the game, type `/console scriptErrors 1`, so each Lua error shows.

## The tests

Do them in order. Each batch of events goes out about once a minute, so wait a minute before you check a result.

### 1. The chain works

1. Type `/timeways test`.
2. Wait for the line "3 of 3 checks passed." It can take a minute or two.

The self-test sends two requests that only read your world: the first page of the journal, and one `/lore` question. The lore question can cost one model call. Each check shows as pass or fail in the chat. If a check fails, the line gives the reason.

The last report stays in the saved variables, as `TimewaysDB.selfTest`. WoW writes it to the disk at `/reload` and at logout.

### 2. Setup and the key

1. Check the AddOns folder. `Timeways_Key` holds `Timeways_Key.toc` and `Key.lua`. The `Timeways` folder holds no `Key.lua`.
2. Log in. The setup window does not open, and test 1 passes: the addon took the key.
3. Type `/dump TimewaysKey`. It shows nothing: the addon cleared the global after it took the key.
4. Type `/timeways help`. The setup window opens with the install steps. Close it with Close or Escape.
5. Stop the desktop app: end the `gnomish-relay` process. Type `/reload`, and wait a minute. The setup window says that the game can't reach the desktop app.
6. Run `gnomish-relay restart`, and type `/reload`.

### 3. The text that you read

1. Talk to an NPC with gossip: an innkeeper or a guard.
2. Open a quest. Accept it, or close it.
3. Turn in a quest.
4. Read a book: for example, a book in the library of Stormwind, or a book on a table.
5. Wait a minute.
6. Ask `/lore` about a word from one of those texts.

The answer uses that text, and says where you learned it. Open `/journal` on the Knowledge tab: the texts show there, with the place and the date.

### 4. The lore book

1. Ask two `/lore` questions.
2. The lore book shows the question as the heading. It says "Asking..." until the answer comes.
3. Click Previous and Next. They step between the two questions.
4. Press Escape. Then type `/lore` with no question. The book opens again on the last question.
5. Close the book, and ask a question. When the answer comes, one line shows in the chat.

### 5. A side quest

This test needs a model. Without one, the NPC "has no task for you now", and the rest of the test does not apply.

1. Visit two subzones, and talk to two NPCs there.
2. Hover your mouse over two hostile creatures, for example two kinds of wolves.
3. Target one of those NPCs, and type `/quest`.
4. Wait a minute. A chat line with the "Timeways:" prefix shows the offer. A task never sends you to talk to a creature that you can attack.
5. Type `/quest accept`. Open `/journal` on the Tasks tab.
6. Check the map of the Tasks tab. The giver has a yellow "?". A step to visit a place or to meet an NPC has a pin with its number, when the journal knows where it is.
7. Do the steps of the quest. For a kill step, hover or target each creature before you kill it. Check the page after each step: a kill step shows "<creature> slain: 1/<count>". The pin of a done step fades.
8. Type `/quest` to another NPC. The new task names no place, NPC, or creature of the last one.

The quest shows "Done on" with the day. Hover the giver: the tooltip has a "Timeways:" line with its trust in you.

### 6. Player tasks

This test needs two characters with Timeways in one party, for example on two computers.

1. On the first character (the giver), open `/journal` on the Tasks tab, and click New task.
2. Type a title. Type a step such as "talk to the innkeeper", and click Add. "Checking..." shows, then the step that the game checks.
3. Type a step that the game can't check, such as "wave at me". It stays as you wrote it.
4. Type 1 in the gold box. Drag a stack from your bags onto a reward slot. Right-click it to take it out, and drag it back.
5. Click Save. The task shows in the list as "Not sent yet". Click it, pick the second character (the doer) under "Send to", and click Send.
6. Click "Help me write" on a new task, and type a short idea. Check the draft, then click "Use this" or "Keep mine". Click Cancel.
7. On the doer, the offer shows on the Tasks tab. Click Accept.
8. Do the steps on the doer. Each step shows as done. Click Done on the step that you wrote.
9. Stand next to the giver, and click Turn in on the doer.
10. On the giver, the turn-in card shows each step with its proof: Witnessed, Seen, or Not confirmed. The written step says "They say it's done. You decide." Click Complete task.
11. Give the doer the gold and the item in a trade. The line of the reward changes to "Reward: paid in trade".

### 7. Your name stays private

In the world folder, run `sqlite3 c_<name>.sqlite "SELECT body FROM learned"`. Your character name is not in the result. `$N` stands in its place.

### 8. Open questions of the design

1. While a book from a table or a shelf is open, type `/dump UnitName("npc")`. Write down what it shows.
2. Kill a rare, and die to a mob. Open `/journal` and check the Deeds tab.
3. Check the Hero and Chronicle tabs. The Chronicle tab shows the newest chapter.

## What to send back

- Each Lua error, as text.
- The chat lines of `/timeways test`.
- What the lore book showed for each `/lore` question.
- The offer line of `/quest`, and the Tasks tab at the end, with its map.
- What each character saw in the test of player tasks.
- The first 3 rows of `sqlite3 c_<name>.sqlite "SELECT body FROM learned LIMIT 3"`.
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
