# Testing in the game

The steps to test Timeways in WoW: Forever, and what to send back.

## The test setup

The game runs a stable copy of the code, so work in the repo does not change the game in the middle of a test.

- **The addon:** the `Timeways` link in the AddOns folder points to `../timeways-test/addon/Timeways`. That folder is a git worktree of one commit. To see which one, run `git -C ../timeways-test log -1 --oneline`. To move it, run `git -C ../timeways-test checkout --detach <commit>`, and build the story program again:

  ```sh
  cargo build --release -p timeways-story --manifest-path ../timeways-test/Cargo.toml
  cp ~/.local/bin/timeways-story ~/.local/bin/timeways-story.old
  cp ../timeways-test/target/release/timeways-story ~/.local/bin/
  ```

- **The story program:** `~/.local/bin/timeways-story`, built from the same commit. The old program is `~/.local/bin/timeways-story.old`.
- **The world files:** `~/.local/share/gnomish-relay/timeways/story/worlds/r_<realm>/c_<name>.sqlite`. A world of an older build (a `.jsonl` file, or a `.sqlite` of another version) is not read: the character starts a new world, and the old file stays.
- **What all characters share:** `timeways.sqlite` in the same data folder, with the narrator's budget and pace.

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

This test needs a model. Without one, the NPC has no quest for you, and the rest of the test does not apply.

1. Visit two subzones, and talk to two NPCs there.
2. Hover your mouse over two hostile creatures, for example two kinds of wolves.
3. Target one of those NPCs, and type `/quest`.
4. Wait a minute. A chat line with the "Timeways:" prefix shows the offer. A quest never sends you to talk to a creature that you can attack.
5. Type `/quest accept`. Open `/journal` on the Quests tab.
6. Check the map of the Quests tab. The giver has a yellow "?". A step to visit a place or to meet an NPC has a pin with its number, when the journal knows where it is.
7. Do the steps of the quest. For a kill step, hover or target each creature before you kill it. Check the page after each step: a kill step shows "<creature> slain: 1/<count>". The pin of a done step fades.
8. Type `/quest` to another NPC. The new quest names no place, NPC, or creature of the last one.

The quest shows "Done on" with the day. Hover the giver: the tooltip has a "Timeways:" line with its trust in you.

### 6. Player quests

This test needs two characters with Timeways in one party, for example on two computers.

1. On the first character (the giver), open `/journal` on the Quests tab, and click New quest.
2. Type a title. Type a step such as "talk to the innkeeper", and click Add. "Checking..." shows, then the step that the game checks.
3. Type a step that the game can't check, such as "wave at me". It stays as you wrote it.
4. Type 1 in the gold box. Drag a stack from your bags onto a reward slot. Right-click it to take it out, and drag it back.
5. Click Save. The quest shows in the list as "Not sent yet". Click it, pick the second character (the doer) under "Send to", and click Send.
6. Click "Help me write" on a new quest, and type a short idea. Check the draft, then click "Use this" or "Keep mine". Click Cancel.
7. On the doer, the offer shows on the Quests tab. Click Accept.
8. Do the steps on the doer. Each step shows as done. Click Done on the step that you wrote.
9. Stand next to the giver, and click Turn in on the doer.
10. On the giver, the turn-in card shows each step with its proof: Witnessed, Seen, or Not confirmed. The written step says "They say it's done. You decide." Click Complete quest.
11. Give the doer the gold and the item in a trade. The line of the reward changes to "Reward: paid in trade".

### 7. Your name stays private

In the world folder, run `sqlite3 c_<name>.sqlite "SELECT body FROM learned"`. Your character name is not in the result. `$N` stands in its place.

### 8. Open questions of the design

1. While a book from a table or a shelf is open, type `/dump UnitName("npc")`. Write down what it shows.
2. Kill a rare, and die to a mob. Open `/journal` and check the Deeds tab.
3. Check the Hero and Chronicle tabs. The Chronicle tab shows the newest chapter.

### 9. NPCs remember you

This test needs a model.

1. Target an innkeeper and type `/talk any news?`. Then `/talk` again with another question. The second answer can bring up what the NPC told you the first time. It brings up at most one memory.
2. `/quest` the same NPC, accept or decline the offer, then `/talk` about the quest. The NPC knows the quest and your answer, such as "you turned it down" or "still on it".
3. More than an hour after you first met an NPC, `/talk` it again. It can recall that it met you ("a while back"), never with a date.
4. Kill a rare, or die to one, in a zone. Then `/talk` an NPC of a town in that zone. It can mention the kill or the death. An NPC that killed you speaks of the fight instead.

### 10. Your Hero sheet shapes some talks and quests

This test needs a model.

1. On the Hero page, answer the Goal question, for example "Find my father's killer".
2. Use `/talk` or `/quest` three times in all. The third answer or quest ties in your goal. The first two do not. Every third call works the same way (the 6th, the 9th).
3. Also answer Flaw, then talk three more times. The next hook uses your flaw. The order is goal, bond, flaw, then traits. Origin and background never show up as a hook.

### 11. New kinds of quests

This test needs a model.

1. `/quest` an NPC a few times, and accept or decline each offer. Each offer differs from the last two in its steps and in its title words.
2. Accept a quest with a talk step. Target its NPC and `/talk`. The step shows "(Complete)", and the NPC plays along with the quest.
3. Accept a quest with "Wait 1 day". The Quests page shows the time left. After the day, any event finishes the wait, and the next step opens.
4. Accept a quest with "In any order". Do its steps in the reverse order. Both complete, and only then does the next step open.
5. Accept a quest that asks you to bring goods, such as "Bring Linen Cloth to X: 0/10". The count follows your bags. Open the gossip window of X with enough cloth: the step completes within the next batch. You keep the cloth.
6. If an offer has a step such as "Visit ... at night", stand in the place as the hour turns. The step completes without you leaving.
7. A mystery quest shows only the steps that you reached.

### 12. Why an NPC trusts you

1. Slap an NPC (`/slap` on your target). Hover it. Under "Timeways: ...", a second line says "Went down when you slapped them."
2. `/talk` an NPC until its trust changes. The second line says "Went up after you talked." or "Went down after you talked."
3. Finish a side quest. Hover its giver: "Went up when you finished their quest."

### 13. Stories about each other

This test needs two characters with Timeways in one party.

1. On the first character, target the second and type `/story`. The scroll "Tell a Story" opens with the second character's portrait, the name, and "Level ... ...". The status line says "Checking...", then goes blank, and Send turns on when you type.
2. Type a title and three paragraphs, with a blank line between two of them. Click Send. The scroll says "Sent to ... They'll decide if it's part of their story."
3. On the second character, the chat says "... told a story about you: <title>. Type /stories to read it." The Stories tab of the journal has a badge with 1.
4. Type `/stories`. The story shows as a page: the title, "By ... · Today", and three paragraphs, with "To report abuse, open Support in the game menu." under them. The bar says "1 waiting".
5. Click Accept. The first character's chat says "... accepted your story." On the second character, the story moves under Accepted, with "By ... · Accepted <day>" and Remove. The name of the second character shows in place of `$N`.
6. Tell a story, and before the second character answers, type `/story` again. The scroll says "... hasn't answered your last story yet.", and Send stays off. Decline the story on the second character, close the scroll, and open it again. Send turns on.
7. Write a story, click Save, and leave the party. Open Stories: the draft shows under Drafts. Click Continue: "Invite ... to your group to send it." Invite the player again, close the scroll, and click Continue again: Send turns on.
8. Type a `|` in the body, and click Send. The error shows, the `|` is selected, and the text stays. Write down whether the selection covers the `|` alone, also after a letter such as "é" before it. This checks whether `HighlightText` counts bytes or letters.
9. Press Enter in the body. A new paragraph starts, and nothing is sent. Write down whether one Enter gives one line break or two: the box inserts a break itself, and the game can add one more. Press Ctrl+Enter. The story is sent.
10. Close the scroll with text in it. `/reload`. Type `/story` for the same player. The text is still there.
11. On the second character, click Block player on a waiting story, then Block. The first character types `/story`: the scroll says "... doesn't take stories from you." The Quests tab of the second character lists the player under Blocked players.
12. Target a player in your group who has no Timeways, and type `/story`. After 5 seconds, the scroll says "... needs Timeways to get stories."
13. Tell a story that names the first character. Leave the party. Accept it the next day. The page shows the story with the real name. In the world file, run `sqlite3 c_<name>.sqlite "SELECT body FROM stories"`: the name shows as an ID such as `{P1}`, never the real name, and the body is a list of paragraphs.
14. Click Remove on an accepted story, then Remove in the dialog. The story leaves the page.
15. A character who is not in your group can't tell you a story: nothing arrives.

### 14. Messages that Blizzard can read

1. Open the game menu (Escape). Check that it has a button named "Support". If the name differs, write it down: the line "To report abuse, open Support in the game menu." needs the real name.
2. In the quest form, under "Send to", check the line "Like chat, Blizzard can read what you send."
3. Send a quest and a story by whisper, in a party, and in a guild. Each one arrives as before.

### 15. Roleplay profiles (MSP)

This test needs a second player who uses Total RP 3 or MyRolePlay, and is of your faction.

1. Open `/hero`. Click "Roleplay Profile". Fill in a name and a title, and click Share.
2. On the other player, mouse over your character. Their roleplay addon shows your name and title. Open your profile there: the history shows your answer to the background question.
3. On your character, mouse over the other player. Your tooltip shows their roleplay name and title.
4. Click "Stop sharing". After a minute, the other player no longer gets updates from you.
5. Mouse over a player of the other faction. No error line shows in the chat.
6. If you use Total RP 3 yourself: the Roleplay Profile section shows your Total RP 3 fields, read only, with "From Total RP 3".

### 16. Remembered players

1. Right-click a player (a party frame, a target, or a name in chat). Click Remember, then Friendly.
2. Click Remember, then "Add a note". Type a note and save it. Paste a tab into the note too: the tab becomes a space, and the note stays.
3. Mouse over the player. The tooltip has a line such as "Friendly: Great healer."
4. Click Remember, then Forget. The line leaves the tooltip.

### 17. The welcome window

1. Type `/timeways help`. Check the install line in the window: it shows one `|`, as in `curl -fsSL ... | sh`, not `||` and not a gap.
2. Copy the line from the window, and paste it in a terminal. It is the exact command.

### 18. The talk window

This test needs a model.

1. Target an innkeeper and type `/talk`. A window opens with the name of the NPC at the top. "Thinking..." shows, then the answer. Nothing goes to the chat.
2. Type a reply in the box under the answer and press Enter. The answer comes in the same window. Target someone else and reply again: the words still go to the innkeeper.
3. Ask "Any work for me?". The NPC answers, then the window says "Thinking of a quest...". A quest card comes: a title, the NPC's text, "Quest Objectives", and Accept and Decline. Click Accept. The Quests tab of `/journal` shows the quest in progress.
4. Ask an NPC for work while you have 3 quests in progress. Its words show, then "You already have 3 quests. Finish one first."
5. Pull a mob while the window is open. The window hides, and comes back after the fight.
6. Press Escape. The window closes. `/talk` the same NPC again: the earlier talk shows lighter at the top, under "Today".
7. `/reload`, then `/talk` the NPC again. The earlier talk is still there.
8. Open the Knowledge tab of `/journal`. It shows no "A rumor from ..." lines.

### 19. The narrator

This test needs a model. The narrator speaks at most 3 times in an hour.

1. Log in, and go into a zone that you never visited. A line shows in the chat after "Narrator:". It tells a piece of the history of the zone, then what you did. It never says "our hero".
2. Read the lines of the next hour. Some lines use the name of your character, some your race or class ("the Forsaken"), and some name nobody. No line shows `$N`.
3. Reach a level that is not a tenth level, such as 13. No line comes. Reach level 20, or another tenth level: a line can come.
4. After the session, print the prompts of your world for review. The tool reads a copy of the world, and the game can stay open:

   ```sh
   cargo run -p timeways-story --bin timeways-narrator-review -- \
     ~/.local/share/gnomish-relay/timeways/story/worlds/r_<realm>/c_<name>.sqlite \
     --pack ~/.local/share/timeways/lore.sqlite --claude
   ```

   With `--claude`, each prompt also goes to Claude Code, and the tool prints the line that you would see. Without it, the tool prints the prompts only.

### 20. Player names never reach a model

This test needs two characters with Timeways in one party, and a model.

1. On the second character, accept a story that names the first character, as in test 13. Open `/hero`: the story shows the real name, and no `{P1}`.
2. On the second character, open `/journal` on the Quests tab, click New quest, then "Help me write". Type an idea that names the first character in lowercase, such as "help ada guard the mill".
3. The draft names the first character as the game writes the name. It shows no `{P1}` and no "my friend".
4. In the world file of the second character, run `sqlite3 c_<name>.sqlite "SELECT prompt FROM calls WHERE kind = 'draft'"`. The prompt holds `{P1}` and a card such as "{P1}: a human paladin", never the name of the first character.
5. Run `sqlite3 c_<name>.sqlite "SELECT position, body FROM aliases"`. The first character has position 0, which is `{P1}`. Accept one more story that names the first character: no new row comes.

### 21. The Hero tab

1. Open `/hero`. The sub-tabs Your Story and Roleplay Profile show on the left. The six answers show as cards, two side by side, and Your Notes shows on the right.
2. Click Answer on an empty card. The card opens in place with the question, a box, and "0 / 200" or "0 / 1000". Type four lines of text: the box grows, and the cards below it move down. Click Save. The next empty card opens.
3. Press Escape in an open card. It closes, and nothing is saved.
4. Open Roleplay Profile. The line says "Only you see this." There is no Name field. "What Others See" shows your name in the game, your title, "Level ... (Player)", and your fields. Description and History stop after 3 lines with "...".
5. Click Share. The line says "Players with roleplay addons like Total RP 3 see this." Origin and Background in Your Story show "Shared".
6. If you use Total RP 3: the page says "Total RP 3 shares your profile. Change it there." "Also shared" and the preview do not show, and the fields have no Edit.

### 22. The Chronicle title page

This test needs a model.

1. Before your first chapter ends, open the Chronicle and click the first row, your name with "Who you've become". The page shows your name, "Level ... ...", and "Fills in when your first chapter ends."
2. Play until a chapter ends and its saga shows, and play a little more, so a batch ends. Open the first row again. A paragraph shows with your name in it, and never "our hero".
3. In the world file, run `sqlite3 c_<name>.sqlite "SELECT prompt FROM calls WHERE kind = 'summary'"`. The prompt holds your answers of Your Story, and no story and no title of the Roleplay Profile.

### 23. The Chronicle book

The chapters, the tales, and the contents (`docs/plans/chapters.md`).

1. Open `/journal` on the Chronicle. The list on the left groups the chapters by level band, such as "Levels 10 to 19", newest first. The open chapter shows "· now".
2. Click a chapter that is over. The page shows "Chapter N", its title, the dates and the levels, the story when a model wrote one, and "In this chapter" with one line for places, people, defeated, quests, deaths, and stories.
3. Take a flight path across two or three zones and land. No new chapter starts for the flight.
4. Run a dungeon, leave it, and wait 30 minutes or more of play. Under the chapter where you entered it, the list shows the dungeon with "Dungeon · 1 run". Its page shows what you defeated inside. After the next batch, it shows a short story of the dungeon, when a model runs.
5. Run the same dungeon again with nothing new. The count says "2 runs", and the story does not change.
6. Die and run back to the dungeon in less than 30 minutes. The count stays the same: a wipe and a corpse run are one run.

### 24. Battlegrounds, PvP rank, inns, and world bosses

These lines use calls of the game that no test can check. Each step answers an open question.

1. Enter a battleground. In the world file, run `sqlite3 c_<name>.sqlite "SELECT body FROM inputs WHERE kind = 'instance_entered'"`. The newest row says `"kind":"pvp"`.
2. Win a battle. Run `sqlite3 c_<name>.sqlite "SELECT body FROM inputs WHERE kind = 'bg_won'"`. One row shows, with the zone of the battleground. Lose a battle: no row comes. Send back whether a win made a row, and whether a loss made one.
3. Run `sqlite3 c_<name>.sqlite "SELECT body FROM inputs WHERE kind = 'pvp_rank'"`. A row of the login shows your rank, or 0. Compare it with the rank of your character sheet. Send back both numbers. If they differ, faction 2800 is not the rank.
4. Log out in an inn and log in again. Run `sqlite3 c_<name>.sqlite "SELECT body FROM inputs WHERE kind = 'rest_changed'"`. Send back whether a row with `"resting":"yes"` came at the login. Walk out of the inn: a row with `"resting":"no"` comes.
5. Take a flight path. Run `sqlite3 c_<name>.sqlite "SELECT body FROM inputs WHERE kind = 'zone_entered' ORDER BY position DESC LIMIT 3"`. The rows of the flight hold `"taxi":"yes"`.
6. Kill a world boss, if you can. Its `npc_defeated` row holds `"kind":"worldboss"`.

### 25. Your own words in the Chronicle

1. Open a chapter that is over, and click Edit. A box opens with the title and the story. Add a line at the end, and click Save. The page says "Saving...", then shows the story and your line under it. The contents say "Edited".
2. Click Edit again, and change the story itself. Save. Only your words show.
3. Click Restore, and confirm. The narrator's story shows again, with no line of yours.
4. Write a note on the open chapter. When the chapter ends and its story comes, the story shows above your note.
5. On the title page, click Edit, write one paragraph, and Save. Turn on sharing in the Roleplay Profile. A friend with Total RP 3 sees your paragraph as your History.
6. Type a very long text, or a `|`, and click Save. The text stays in the box with the reason under it.

## Dev mode

Dev mode makes the moments of hours of play in seconds. It runs the real code: the addon, the bridge, and the story program. Only the moment itself is fake.

### Turn it on and off

Dev mode is off by default. The switch is a file on the desktop, so no player can turn it on in the game by accident.

```sh
timeways-dev on        # writes dev = true into <data>/gnomish-relay/timeways/story/settings.toml
gnomish-relay restart  # the story program reads the file at its start
timeways-dev status
timeways-dev off       # writes dev = false
```

The story program then puts `"dev": true` on each journal. The addon reads it at login and after each journal: `/twdev` works only while it is there. With dev mode off, `/twdev` says "Dev mode is off." and sends nothing.

### What keeps it safe

- Each fake event line carries `"dev": true`. While dev mode is off, the story program refuses every line with a `dev` key (`crates/story/tests/dev_mode.rs`, the property test `a_dev_line_never_changes_a_world_while_dev_mode_is_off`, and the fuzz seeds `fuzz/seeds/input/dev.txt`).
- A line with a reply (`talk_asked`, `lore_asked`, `journal_asked`, `draft_asked`) carries no mark yet, because the bridge reads these lines in a fixed shape. Only `/twdev talk` sends one, and only while the addon knows that dev mode is on.
- A fake player gets its messages in the game itself. Nothing goes on the wire to it (`crates/addon-tests/tests/dev_peer.rs`). Its realm is `Devrealm`, which no server has.
- While dev mode is on, the addon sends nothing to real players and ignores their messages. The chat says "Dev mode is on. Nothing is shared with other players.", and the Roleplay Profile says "Sharing is off in dev mode." The Share setting stays as you set it, and works again when dev mode turns off. Each send goes through `ToPlayers.lua`, and a test fails when a file sends past it (`crates/addon-tests/tests/dev_mode_sharing.rs`).
- The code ships in each release, and does nothing until the desktop turns dev mode on.

### Worlds from scenarios

`timeways-dev seed` builds the world of a character from a scenario: invented lines of the addon, in batches, through the real story program and the checks of the bridge. Model calls go to the model of your config (`[story]` in `~/.config/gnomish-relay/config.toml`). `--no-model` fails each call, as the bridge does with no model.

```sh
timeways-dev scenarios
timeways-dev seed Testpal --realm "Classic Beta PvP 2" --scenario level-30-paladin
timeways-dev seed Testpal --realm "Classic Beta PvP 2" --scenario raider-60 --replace --no-model
timeways-dev snapshot Testpal before-raid --realm "Classic Beta PvP 2"
timeways-dev restore Testpal before-raid --realm "Classic Beta PvP 2"
gnomish-relay restart
```

- The seed refuses a world that exists. With `--replace`, it moves the old file to `<file>.bak-<seconds>` first. It never touches another character.
- A snapshot goes to `<data>/gnomish-relay/timeways/story/dev-snapshots/`. A restore moves the world that exists to a backup first.
- After a seed or a restore, run `gnomish-relay restart`, and log in as that character. The story program keeps the world of the active character open, so it sees the new file only after a restart.
- The times of a scenario end a minute before the seed, so the world reads as play of today.
- A scenario file of your own works too: `--scenario my-test.jsonl`. The format is at the top of `crates/dev/src/scenario.rs`.

| Scenario | What the world holds |
|---|---|
| `fresh` | A human paladin at level 2, in the first open chapter. |
| `level-30-paladin` | 9 chapters from Elwynn Forest to Stranglethorn Vale, tales of the Deadmines and Blackfathom Deeps, deaths to Mor'Ladim and the revenge, a class quest, a side quest done and one in progress, a mount, books and gossip. |
| `raider-60` | An orc warrior at 60: Blackrock Depths 4 runs, Upper Blackrock Spire 3 runs as a raid, Azuregos, a swift wolf, Ironfoe, and a big upgrade. |
| `story-inbox` | A Forsaken priest with two accepted stories from Kobee and Morvane. `/twdev inbox` in the game adds five stories that wait. |
| `edits` | A night elf druid whose chapters, tale, and title page hold the player's own words, and a restore. |
| `side-quests` | A dwarf hunter with side quests of every step kind and in every state: done, in progress, a mystery with a hidden step, an offer, declined, and abandoned. The model answers are fixed in the file. |
| `flavor-and-hero` | A troll shaman with a full Hero sheet and notes, every joke title of the Horde, a quest mark, a battleground won, a PvP rank, an inn, and a flight. |

### Commands in the game

Type `/twdev help` for the list. A name with spaces needs no quotes. A slash separates two parts: `/twdev zone Westfall / Moonbrook`.

| Command | What it fakes |
|---|---|
| `level <n>` | You reach level n. |
| `zone <zone> [/ subzone]` | You walk into a place. |
| `taxi` | A flight over Elwynn Forest and Westfall to Duskwood. |
| `dungeon <name>`, `raid <name>` | You enter an instance of that kind. |
| `bg-win` | You win in Warsong Gulch. |
| `pvp-rank <n>` | Your PvP rank grows. |
| `rest` | You rest at an inn, and walk out. |
| `mount <name> [epic]` | You ride a mount (160), or a swift one (200). |
| `kill <name> [rare\|boss\|worldboss]` | You defeat a rare or a boss. With no kind, a kill for a kill step of a quest. |
| `seen <name> [friendly] [beast]` | You hover an NPC. |
| `death <killer>`, `fall`, `drown`, `lava` | You die to an NPC or to the world. |
| `item <name> epic\|rare` | You put on an item 12 levels above the one before. |
| `game-quest <title> [class]` | You take a quest of the game and turn it in. |
| `mark <quest> / <buff>` | A quest of the game leaves a lasting buff. |
| `chapter-end [new zone]` | 16 errands where you stand, then a quest in a new zone: the open chapter closes. Use it outside an instance. |
| `hour <h>` | The hour of your computer changes, for a "visit at night" step. |
| `meet <npc>` | You open the window of an NPC. |
| `gossip <npc> / <text>`, `quest-text <npc> / <title>`, `book <title> [/ text]` | You read a text of the game. |
| `talk <npc> [/ words]` | `/talk` to that NPC, as if you targeted it. Ask "Any work for me?" for the quest card. |
| `quest <npc>` | `/quest` to that NPC. |
| `slap <npc>`, `emote <emote> [/ npc]` | An emote, with its slap and its trust. |
| `carry <count> <item> / <npc>` | You show an NPC what your bags hold, for a carry step. |
| `journal` | The desktop sends the journal now, and the book opens. |
| `welcome setup\|files\|offline` | The setup window for that reason. |
| `peer <name> story [title]` | A fake player of your group tells a story about you. |
| `peer <name> quest` | A fake player sends you a quest. |
| `peer <name> open\|full\|blocked\|waiting` | The room of its story box, for its answer to your `/story`. |
| `peer <name> write` | The story scroll for that fake player. |
| `peer <name> accept\|decline` | Its answer to the story that you told it. |
| `peer <name> near\|far` | It stands next to you, or not, for the proof of a step. |
| `peer <name> step <n>`, `peer <name> turnin [steps]` | It does a step of the quest that you gave it, and turns it in. |
| `inbox` | Five fake players each tell a story that waits. |
| `msp <name> / <title>` | A fake player shares a roleplay profile. Turn on Share first. |
| `remember <name> friendly\|neutral\|avoid`, `note <name>` | You remember a fake player, and write a note. |
| `tooltip <name>` | The tooltip of a fake player: its profile and your note. |

### Each long-play test, fast

| Test | How to reach it fast | What to look for |
|---|---|---|
| 3. The text that you read | `/twdev book The Kingdom of Stormwind`, `/twdev gossip Innkeeper Farley / Rest a while.`, then `/lore` | The answer cites the text. Knowledge lists it. |
| 5. A side quest | `/twdev seen Prowler beast`, `/twdev meet Thor`, `/twdev quest Thor`, then the step commands: `kill`, `zone`, `meet`, `talk`, `emote`, `slap`, `carry`, `hour`, `level`, `game-quest`, `dungeon` | The offer, the steps that complete, "Prowler slain: 1/3". The `side-quests` scenario shows every step kind with no model. |
| 6. Player quests | `/twdev peer Kobee near`, open New quest, send it to Kobee, then `/twdev peer Kobee step 1` and `/twdev peer Kobee turnin 1`. For a quest to you: `/twdev peer Kobee quest`. | Kobee in "Send to", the turn-in card with Witnessed or Seen, Complete quest. |
| 8. Kills and deaths on Deeds | `/twdev kill Hogger rare`, `/twdev death Mor'Ladim` | The Deeds tab. |
| 9. NPCs remember you | `/twdev talk Innkeeper Farley / Any news?` twice; `/twdev death Hogger`, then `/twdev talk` an NPC of that zone | The second answer recalls the first. |
| 10. Hero hooks | Answer Goal, then `/twdev talk <npc>` three times | The third answer ties in the goal. |
| 11. New kinds of quests | The `side-quests` scenario, then `/twdev hour 22` or `/twdev carry 10 Wool Cloth / Innkeeper Belm` | Waits, sets, carry counts, night visits, a mystery. |
| 12. Why an NPC trusts you | `/twdev slap <an NPC near you>`, then hover it | "Went down when you slapped them." |
| 13. Stories about each other | `/twdev peer Kobee story The Bridge`, `/twdev inbox`, `/twdev peer Kobee full`, `/twdev peer Kobee write` | The badge, Accept, Decline, Block, and the lines of a full or blocked box. |
| 15. Roleplay profiles | Turn on Share, `/twdev msp Kobee / Keeper of the Flame`, `/twdev tooltip Kobee` | "Kobee of the Reef, Keeper of the Flame". |
| 16. Remembered players | `/twdev remember Kobee avoid`, `/twdev note Kobee`, `/twdev tooltip Kobee` | "Avoid: " and the note. |
| 17. The welcome window | `/twdev welcome setup`, `files`, or `offline` | The heading of each reason. |
| 18. The talk window | `/twdev talk Innkeeper Farley / Any work for me?` | The quest card, with a model. |
| 19. The narrator | `/twdev level 20`, `/twdev zone Duskwood`, `/twdev kill Mor'Ladim rare`, `/twdev mount Swift Brown Wolf epic`, `/twdev item Ironfoe epic` | A line for each kind of moment, with a model. |
| 22. The Chronicle title page | `/twdev chapter-end`, then a few more batches | The summary, with a model. |
| 23. The Chronicle book | The `level-30-paladin` or `raider-60` scenario, or `/twdev chapter-end`, `/twdev dungeon The Deadmines`, `/twdev kill Edwin VanCleef boss` | Chapters, tales, run counts, tally lines. |
| 24. Battlegrounds, rank, inns, bosses | `/twdev bg-win`, `/twdev pvp-rank 3`, `/twdev rest`, `/twdev taxi`, `/twdev kill Azuregos worldboss` | The rows of `inputs`, with `"dev":true`. |
| 25. Your own words | The `edits` scenario, or Edit on a chapter of any scenario | Edited, Restore, and the title page. |

Each scenario and each command has a named test. Three tests fail when a new feature has no way in dev mode: `every_input_line_has_a_dev_command_or_a_scenario` and `every_section_of_the_journal_has_a_scenario_that_fills_it` (`crates/addon-tests/tests/dev_mode.rs`), and `every_kind_of_narrator_moment_comes_in_a_scenario` (`crates/dev/tests/scenarios.rs`).

### What dev mode can't fake

- **The answers of a model.** A narrator line, a saga, a tale, a summary, a history of a zone, a talk, a quest offer of `/quest`, and "Help me write" need a model. The scenarios fix the answers of side quests only.
- **The positions on the map.** A fake place has no point on a map of the game, so it gets no pin.
- **The hooks of the game itself:** the menu of a right click on a real player, the tooltip of a real unit, and the real `PARTY_KILL`, auras, and death recap. The commands start right after them.
- **A real second game.** The logged channel, the rate limits of the server, and trades between two players need two characters (tests 6 and 13 in full).

## What to send back

- Each Lua error, as text.
- The chat lines of `/timeways test`.
- What the lore book showed for each `/lore` question.
- The offer line of `/quest`, and the Quests tab at the end, with its map.
- What each character saw in the test of player quests.
- The first 3 rows of `sqlite3 c_<name>.sqlite "SELECT body FROM learned LIMIT 3"`.
- The result of each open question.
- For tests 9 to 11: each `/talk` answer and each quest offer, as text.
- For test 13: the result of the `sqlite3` command of step 6.
- For test 14: the name of the Support button.
- For tests 15 to 17: a screenshot of each tooltip line and of the install line.
- For test 18: a screenshot of the window with a quest card, and of a window with an earlier talk.
- For test 19: each narrator line of the session, as text, and the output of the review tool.
- For test 20: the draft, and the output of the two `sqlite3` commands.
- For test 23: a screenshot of the contents, of a chapter page, and of a tale page.
- For test 24: the answer to each step.
- For test 25: a screenshot of a chapter with your note, and of the History that your friend sees.

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
