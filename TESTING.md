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

The answer uses that text, and says where you learned it. Open `/journal` on the Knowledge tab: the texts show under "Read and heard" of the zone where you read them, and a click opens each one with its date.

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
2. Kill a rare, and die to a mob. Open `/journal`: the kill and the death show in the open chapter of the Chronicle, and under "Deaths and kills" of the zone in Knowledge.
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
3. Ask "Any work for me?". In dev mode, `/twdev talk Innkeeper Farley / Any work for me?` does it with no NPC near. The NPC answers, then the window says "Thinking of a quest...". A quest card comes: a title, the NPC's text, "Quest Objectives", and Accept and Decline. Click Accept. Your words, the NPC's answer, the title, and the NPC's text stay, and "Quest accepted." shows under them. The reply box still sends. The Quests tab of `/journal` shows the quest in progress.
4. Reply to the NPC after Accept. The new turn shows under "Quest accepted.". Ask for work again, and click Decline on the new card. Both quests stay in the window, each under its own turn.
5. Ask an NPC for work while you have 3 quests in progress. Its words show, then "You already have 3 quests. Finish one first."
6. Pull a mob while the window is open. The window hides, and comes back after the fight.
7. Press Escape. The window closes. `/talk` the same NPC again: the earlier talk shows lighter at the top, under "Today".
8. `/reload`, then `/talk` the NPC again. The earlier talk is still there.
9. Open the Knowledge tab of `/journal`. It shows no "A rumor from ..." lines.

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
2. Click a chapter that is over. The page shows "Chapter N", its title, the dates and the levels, the story when a model wrote one, and "In this chapter" with one line for places, people, the people that you talked to, defeated, quests, deaths, and stories, then the other deeds: titles, mounts, gear, and battles. The map shows pins for the places and people of the chapter.
7. Click a place or a person in "In this chapter". Knowledge opens on its page.
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

### 26. Lore of your own deeds

The wiki tells some quests as history: "the adventurers killed Edwin VanCleef". Such a passage shows only after you did that deed yourself.

1. Enter the Deadmines, and leave before you kill Edwin VanCleef.
2. Ask `/lore What happened to Edwin VanCleef?`. The answer does not tell of his death.
3. Kill Edwin VanCleef, and ask again. The answer can tell of his death now.

### 27. Dungeon setups and later entries

The first entry into a dungeon tells who wants what done there, while that deed still waits. Each later entry tells lore of the dungeon that no line told before, and then the narrator stays quiet.

1. Before you kill Edwin VanCleef, enter the Deadmines for the first time. The narrator line tells who wants VanCleef dead, and ends on "Edwin VanCleef is still alive."
2. Leave, and enter again. Do it twice more. Each line tells something new about the Deadmines, and no line repeats the lore of another. No line is only about the life of a person, such as who trained VanCleef. No line says who holds the Deadmines now unless its lore says so.
3. Kill Edwin VanCleef, leave, and enter again. No line tells that someone wants him dead, and no line tells him as alive.
4. Run `gnomish-relay restart`, and enter the Deadmines again. No line repeats the lore of an earlier entry, also after a restart.

### 28. Lore of a foe that is no rare

Some lore waits for the kill of a foe that is neither rare nor a boss, such as Mor'Ladim in Duskwood.

1. Ask `/lore What happened to Mor'Ladim?` before you kill him. The answer does not tell of his death.
2. Hover or target Mor'Ladim, kill him, and ask again. The answer can tell of his death now.
3. Kill a common mob of Duskwood. The Deeds tab shows no deed for it.

### 29. Game names and the ends of bosses

The wiki and the game name some bosses in two ways. A kill under the game name counts for the lore of the wiki name. Lore that tells the death of a boss waits for your kill.

1. Enter Scarlet Monastery, and ask `/lore What happened to Sally Whitemane?` before you kill her. The answer tells nothing of her death, or of the deaths of the other leaders.
2. Kill High Inquisitor Whitemane, and ask again. The answer can tell more now.
3. Check two encounter names of the game. They wait for a check in the game: Aku'mai in Blackfathom Deeps, and the fight of High Inquisitor Whitemane in the Cathedral. After each kill, open the Chronicle, and read the name of the kill in the deeds of the chapter. Send it back when it is not "Aku'mai" or "High Inquisitor Whitemane". A row of `crates/story/data/game_names.toml` then needs that name.
4. Enter Shadowfang Keep, and ask `/lore What happened to Arugal?` before you kill him. The answer tells nothing of his death.

### 30. Knowledge, place by place

Knowledge is an atlas: the map is the index.

1. Open `/journal` on Knowledge. It opens on the zone where you stand: its name, the first visit and its chapter, and boxes with counts, such as "9 people" and "11 quests". No count says "of".
2. The map shows a pin for each place of the zone where you stood with a position, and a skull where you died. Hover a pin: the tooltip names the place and its counts. Click it: the page of the place opens, its pin stands out, and the pins of its people show.
3. Click a person on a page. Their page shows how they feel about you, in the words of their tooltip, when you met, what they said to you, and the quests between you. Back returns to the page before.
4. Click "< Eastern Kingdoms" (or your continent) on the map. The page lists each zone that you visited, with two counts.
5. With a lore pack, a zone shows "What you know". Meet an NPC that its lore names, and open the zone again: a new passage can show. "There's more to learn here." shows only while some lore of the zone waits.
6. Open the Chronicle, and click a chapter in the list of a zone. The chapter opens.

### 32. A prologue at the first login

A character that Timeways first sees at level 10 or more, with a long past, gets a prologue at once.

1. Log in with a character of level 10 or more that never played with Timeways. The chat shows the time played once: the addon asks the game for it.
2. Within a few minutes, the chat says "Your Chronicle has a prologue now."
3. Open the Chronicle. The first entry is "Prologue": the history of the lands and the people of the character. The title page holds a summary.
4. Log out and in again. No second time played shows, and the prologue stays as it was.
5. Read the past: `sqlite3 c_<name>.sqlite "SELECT body FROM past"`. The professions are Mining, Fishing, and the like, never a weapon skill. The zones are the ones that your map shows. Report a profession or a zone that is missing: the category ids of the skill lines and the map of explored zones need this test.

### 33. No invented names

This test needs a model. A model text names only what its prompt gave: the lore, the facts of the moment, and your own words.

1. Play for an hour, or run `timeways-dev bench-model --local`. Read each narrator line, chapter, tale, title page, zone history, talk, and `/lore` answer.
2. Each person, place, and group that a text names is in its lore or in what you did. No text names a person that the game and the lore don't know, such as a new owner of the Deadmines.
3. In the bench report, `ungrounded-name` counts the texts that named something that their prompt did not give.

### 34. The lore of a boss after its kill

A boss that you killed is known to you. The lore of his end shows wherever the wiki files it, also on the page of a town that you never visited.

1. Enter the Deadmines, kill Edwin VanCleef, and leave. Don't visit Moonbrook.
2. The narrator line of the kill tells something of VanCleef, unless the narrator already spoke three times in the last hour.
3. Ask `/lore What happened to Edwin VanCleef?`. The answer tells of his life and of his end.
4. Ask the same question on a new character that never killed him. The answer tells nothing of his end.

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

- Each fake line carries `"dev": true`. While dev mode is off, the story program refuses every line with a `dev` key (`crates/story/tests/dev_mode.rs`, the property test `a_dev_line_never_changes_a_world_while_dev_mode_is_off`, and the fuzz seeds `fuzz/seeds/input/dev.txt`).
- A line with a reply (`talk_asked`, `lore_asked`, `journal_asked`, `draft_asked`) carries the mark too. The bridge passes it on since relay commit 7dceef3, so the story program refuses a fake question while dev mode is off (`a_dev_line_with_a_reply_gets_no_answer_while_dev_mode_is_off`).
- A fake player gets its messages in the game itself. Nothing goes on the wire to it (`crates/addon-tests/tests/dev_peer.rs`). Its realm is `Devrealm`, which no server has.
- While dev mode is on, the addon sends nothing to real players and ignores their messages. The chat says "Dev mode is on. Nothing is shared with other players.", and the Roleplay Profile says "Sharing is off in dev mode." The Share setting stays as you set it, and works again when dev mode turns off. Each send goes through `ToPlayers.lua`, and a test fails when a file sends past it (`crates/addon-tests/tests/dev_mode_sharing.rs`).
- The code ships in each release, and does nothing until the desktop turns dev mode on.

### Worlds from scenarios

`timeways-dev seed` builds the world of a character from a scenario: invented lines of the addon, in batches, through the real story program and the checks of the bridge. Model calls go to the model of your config (`[story]` in `~/.config/gnomish-relay/config.toml`). `--no-model` fails each call, as the bridge does with no model.

```sh
timeways-dev on
timeways-dev scenarios
timeways-dev seed Testpal --realm "Classic Beta PvP 2" --scenario level-30-paladin
timeways-dev seed Testpal --realm "Classic Beta PvP 2" --scenario raider-60 --replace --no-model
timeways-dev snapshot Testpal before-raid --realm "Classic Beta PvP 2"
timeways-dev restore Testpal before-raid --realm "Classic Beta PvP 2"
timeways-dev export-ratings Testpal --realm "Classic Beta PvP 2"
gnomish-relay restart
```

- `seed` and `restore` change a world, so they run only while dev mode is on. The benches (`bench-model` and `bench-fps`) need dev mode too. Else they say "Dev mode is off." and change nothing. With `--data`, the switch is `settings.toml` in that folder. `on`, `off`, `status`, `scenarios`, `snapshot`, and `export-ratings` always work.
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
| `outcome-lore` | A human warrior at level 18 who fought through the Deadmines to Mr. Smite and left before Edwin VanCleef. |
| `dungeon-setups` | A human warrior at level 18 in Westfall who met Gryan Stoutmantle and never entered the Deadmines. |
| `ratings` | A human paladin who rated two narrator lines by their IDs, one up and one down as boring, and a first chapter with a story of the narrator. The answers of the narrator are fixed in the file. |
| `prologue-35` | A human paladin at level 35 in Stranglethorn Vale whom Timeways sees for the first time, with a long past: quests, zones, standings, professions, a mount, and rare gear. The first batch asks for the prologue. |

### Commands in the game

Type `/twdev help` for the list. A name with spaces needs no quotes. A slash separates two parts: `/twdev zone Westfall / Moonbrook`.

| Command | What it fakes |
|---|---|
| `level <n>` | You reach level n. |
| `zone <zone> [/ subzone]` | You walk into a place. |
| `taxi` | A flight over Elwynn Forest and Westfall to Duskwood. |
| `dungeon <name>`, `raid <name>` | You enter an instance of that kind. |
| `dungeon-again <name>` | You step out to the zone where you really stand, and enter the dungeon again. |
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
| `atlas` | You walk into three places of the zone where you stand, at points on its real map, and meet one person in each. Someone tells you something, you read a quest and turn it in, you read a book, you die to "Dev Wolf", and you defeat "Dev Rare". Knowledge opens. |
| `atlas empty` | Knowledge opens on a place that you never visited. |
| `welcome setup\|files\|offline` | The setup window for that reason. |
| `fps start [label]`, `fps stop` | No fake: it samples the real frame rate once a second, and sends the run to the desktop at the stop. It stops by itself after 15 minutes. |
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
| 3. The text that you read | `/twdev book The Kingdom of Stormwind`, `/twdev gossip Innkeeper Farley / Rest a while.`, then `/lore` | The answer cites the text. Knowledge lists it under "Read and heard". |
| 5. A side quest | `/twdev seen Prowler beast`, `/twdev meet Thor`, `/twdev quest Thor`, then the step commands: `kill`, `zone`, `meet`, `talk`, `emote`, `slap`, `carry`, `hour`, `level`, `game-quest`, `dungeon` | The offer, the steps that complete, "Prowler slain: 1/3". The `side-quests` scenario shows every step kind with no model. |
| 6. Player quests | `/twdev peer Kobee near`, open New quest, send it to Kobee, then `/twdev peer Kobee step 1` and `/twdev peer Kobee turnin 1`. For a quest to you: `/twdev peer Kobee quest`. | Kobee in "Send to", the turn-in card with Witnessed or Seen, Complete quest. |
| 8. Deeds in the Chronicle | `/twdev kill Hogger rare`, `/twdev death Mor'Ladim`, `/twdev mount Brown Horse`, then the Chronicle. The `level-30-paladin` scenario has deeds in every chapter. | "In this chapter" of the open chapter: "Defeated: Hogger.", "Deaths: Mor'Ladim at ...", and "Rode your first mount, Brown Horse." Each place and person is a link. |
| 9. NPCs remember you | `/twdev talk Innkeeper Farley / Any news?` twice; `/twdev death Hogger`, then `/twdev talk` an NPC of that zone | The second answer recalls the first. |
| 10. Hero hooks | Answer Goal, then `/twdev talk <npc>` three times | The third answer ties in the goal. |
| 11. New kinds of quests | The `side-quests` scenario, then `/twdev hour 22` or `/twdev carry 10 Wool Cloth / Innkeeper Belm` | Waits, sets, carry counts, night visits, a mystery. |
| 12. Why an NPC trusts you | `/twdev slap <an NPC near you>`, then hover it | "Went down when you slapped them." |
| 13. Stories about each other | `/twdev peer Kobee story The Bridge`, `/twdev inbox`, `/twdev peer Kobee full`, `/twdev peer Kobee write` | The badge, Accept, Decline, Block, and the lines of a full or blocked box. |
| 15. Roleplay profiles | Turn on Share, `/twdev msp Kobee / Keeper of the Flame`, `/twdev tooltip Kobee` | "Kobee of the Reef, Keeper of the Flame". |
| 16. Remembered players | `/twdev remember Kobee avoid`, `/twdev note Kobee`, `/twdev tooltip Kobee` | "Avoid: " and the note. |
| 17. The welcome window | `/twdev welcome setup`, `files`, or `offline` | The heading of each reason. |
| 18. The talk window | `/twdev talk Innkeeper Farley / Any work for me?`, then Accept or Decline | The quest card, with a model. After Accept or Decline, the talk and the quest text stay, with "Quest accepted." or "Quest declined.". |
| 19. The narrator | `/twdev level 20`, `/twdev zone Duskwood`, `/twdev kill Mor'Ladim rare`, `/twdev mount Swift Brown Wolf epic`, `/twdev item Ironfoe epic` | A line for each kind of moment, with a model. |
| 22. The Chronicle title page | `/twdev chapter-end`, then a few more batches | The summary, with a model. |
| 23. The Chronicle book | The `level-30-paladin` or `raider-60` scenario, or `/twdev chapter-end`, `/twdev dungeon The Deadmines`, `/twdev kill Edwin VanCleef boss` | Chapters, tales, run counts, tally lines. |
| 24. Battlegrounds, rank, inns, bosses | `/twdev bg-win`, `/twdev pvp-rank 3`, `/twdev rest`, `/twdev taxi`, `/twdev kill Azuregos worldboss` | The rows of `inputs`, with `"dev":true`. |
| 25. Your own words | The `edits` scenario, or Edit on a chapter of any scenario | Edited, Restore, and the title page. |
| 26. Lore of your own deeds | The `outcome-lore` scenario, `/lore What happened to Edwin VanCleef?`, then `/twdev kill Edwin VanCleef boss` and the same question | The first answer tells nothing of his death. The second one does. |
| 27. Dungeon setups and later entries | The `dungeon-setups` scenario, then `/twdev dungeon-again The Deadmines` three times. Then seed again with `--replace`, `/twdev kill Edwin VanCleef boss`, and `/twdev dungeon-again The Deadmines` | Three lines with three different pieces of lore, the first one a setup that ends on "Edwin VanCleef is still alive." No line of pure biography. After the kill, no setup, and no line tells VanCleef as alive. |
| 28. Lore of a foe that is no rare | Kill Mor'Ladim in Duskwood, or `/twdev kill Mor'Ladim` (no kind), with a pack that tags him. Then `/lore What happened to Mor'Ladim?` | The answer tells of his death only after the kill. A kill of a common mob adds no deed. |
| 29. Game names and the ends of bosses | `/twdev dungeon Shadowfang Keep`, `/lore What happened to Arugal?`, then `/twdev kill Archmage Arugal boss` and the same question. For a game name: `/twdev dungeon Scarlet Monastery`, `/twdev kill High Inquisitor Whitemane boss`, and `/lore What happened to Sally Whitemane?` | The first answer tells nothing of the beheading. The second one can. The kill under the game name opens the lore of Sally Whitemane. |
| 30. Knowledge, full | `/twdev atlas` in any zone with a map | The zone page with its counts, People, Quests, "Read and heard", "Deaths and kills", and Chapters. Three pins on the real map ("Dev Camp", "Dev Ruins", "Dev Tower") and a skull at "Dev Ruins". Hover a pin for its counts, and click it. Click "Dev Scout", then Back. "< Eastern Kingdoms" (or your continent) opens the world page. With a lore pack, the zone shows "What you know". |
| 30. Knowledge, empty | `/twdev atlas empty`, or the `fresh` scenario | "You haven't been here yet." for a place that you never saw, or "Nothing yet. People you meet, books you read, and quests you finish show up here." |
| 31. Ratings | The `ratings` scenario, then `timeways-dev export-ratings <character> --realm <realm>`. In the game: `/timeways ratings on`, the Chronicle of that world, and the Dislike thumb on chapter 1 with "Too long". `/twdev level 30` for a narrator line, then click its `[Rate]` and pick Dislike > Boring. In combat, click `[Rate]` on another line. Then `/reload` and open chapter 1 again | The file holds two ratings of narrator lines with `new_zone`, the second with `boring`, and after the thumb a rating of the chapter with `too_long`, with `$N` and no name. The `[Rate]` link turns into "Disliked: Boring" in place, with no new chat line, and the menu opens in combat with no error. After the reload the Dislike thumb of chapter 1 is still filled. With ratings off, no `[Rate]` link and no thumb shows. |
| 32. A prologue at the first login | The `prologue-35` scenario with a model, then log in and open the Chronicle | The prologue as chapter 0, about the lands and the people of the hero, and the title page with a summary. |
| 33. No invented names | `timeways-dev bench-model --local`, then `/twdev dungeon The Deadmines` and `/twdev talk Gryan Stoutmantle / Who holds the Deadmines?` | `ungrounded-name` under refused calls by fault, and no shown text that names someone its lore lacks. |
| 34. The lore of a boss after its kill | `/twdev dungeon Deadmines`, `/twdev kill Edwin VanCleef boss`, then `/lore What happened to Edwin VanCleef?`. Also on a fresh character with no dungeon: `/twdev kill Edwin VanCleef boss` and the same question | The entry line has lore of the Deadmines. The kill gets a narrator line about VanCleef. The answer tells of his end. With no visit, it still tells of his end, from the pages of the Deadmines and Moonbrook. |

Each scenario and each command has a named test. Three tests fail when a new feature has no way in dev mode: `every_input_line_has_a_dev_command_or_a_scenario` and `every_section_of_the_journal_has_a_scenario_that_fills_it` (`crates/addon-tests/tests/dev_mode.rs`), and `every_kind_of_narrator_moment_comes_in_a_scenario` (`crates/dev/tests/scenarios.rs`).

### Testing the local model and the frame rate

Two risks need numbers: the quality of the narrator with the small local model that most players run, and the frame rate of the game while that model works on the same computer. Two benches measure them.

#### The local model

`timeways-dev bench-model` plays a fixed set of moments through the real prompts and the real checks of the story program, in a scratch world of invented inputs. It never opens the world of a real character. The set `v1` (`crates/dev/bench/moments-v1.jsonl`) holds an arrival, a tenth level, a dungeon setup, a revenge, a deed, a tale, the end of a chapter, a saga, a summary, a zone history, a talk, and a `/lore` answer. A set never changes after a result names it. A new set gets a new file, so results compare over time.

1. Turn dev mode on: `timeways-dev on`.
2. Start the local model. Ollama runs in the background after its install. If it does not run, start it with `ollama serve`.
3. Run one model, or two side by side:

   ```sh
   timeways-dev bench-model --local                        # the local model
   timeways-dev bench-model --claude                       # Claude Code with no tools
   timeways-dev bench-model --model "<shell command>"      # any model that reads the prompt on stdin
   timeways-dev bench-model --compare local,claude --runs 3
   ```

- `--local` is `local_url` and `local_model` of `[story]` in the config. With none, it is `llama3.2:3b` in Ollama at `http://127.0.0.1:11434`, the model that the setup of the relay installs. It calls `/v1/chat/completions` with `curl`, as the bridge does.
- `--runs N` plays the set N times, each time in a new scratch world. `--moments <file.jsonl>` takes a set of your own.
- It needs no network when the local model is installed.
- The first call only loads the model. Its time shows as the warm-up, and no other number counts it.

How to read the results:

- One row for each moment and kind of call, such as `tale / tale` or `chapter-end / narrator`.
- **shown**: the player reads the line. **silence**: the model answered SILENCE, as the prompt allows. **refused**: the checks refused the last try. **failed**: the model gave no answer.
- **retries**: the asks that needed a second call.
- **refused calls by fault**: the reasons of the story program, in short names: `slop`, `copy`, `cutoff`, `arrival`, `inside-hero`, `json-shape`, `ungrounded`, `ungrounded-name` (a name that the prompt did not give), and so on. A refused first answer takes the reasons of its retry prompt. A last answer takes a check of its text alone, which misses the faults that need the moment. Those show as `other`.
- **p50 s** and **p95 s**: the time of a call. **first s**: the time to the first byte, for a shell command that prints as it goes. **tok/s**: the tokens of the answers over the time of their calls, when the runner counts tokens. Ollama counts them. The time holds the reading of the prompt too.
- "Moments with no model call": a moment whose lore is too thin, so the narrator stays quiet with no call.
- Then every shown line, and the last refused answer of each refused ask. Read the lines against the narrator voice (`docs/plans/narrator-style.md`).
- The results go to `<data>/gnomish-relay/timeways/story/dev-bench/model-<time>.json` and `.txt`.

#### The frame rate

`timeways-dev bench-fps` runs a fixed timeline: N seconds idle (the baseline), N seconds of model calls back to back (the load), and N seconds idle again (the recovery). The desktop can't tell the addon when a phase starts, because the bridge only answers batches. So the addon samples all the time with the clock of the computer, and the desktop splits the samples by the times of its phases. This needs no change of the relay.

1. Turn dev mode on, and run `gnomish-relay restart`. Log in, and stand in a busy place, such as a capital.
2. On the desktop:

   ```sh
   timeways-dev bench-fps --model local --seconds 60
   ```

3. It tells you when: type `/twdev fps start bench` in the game. The baseline starts 15 seconds later (`--lead`).
4. Don't move the camera. Wait until the desktop says "Done".
5. Type `/twdev fps stop` in the game. The run goes to the desktop as one dev line. The desktop waits up to 5 minutes for it (`--wait`), and then prints the report.

- `--model` is `local`, `claude`, or a shell command. `--seconds` is the length of each phase. Keep it at 80 or below: a run of 240 seconds or less sends every sample, and a longer run sends the means of groups.
- For each phase: the min, the 5th percentile ("low 5%"), the median, and the mean frame rate, and the drop of the median and of the mean from the baseline, in percent.
- The time of the model calls in the load.
- The CPU, the memory, and the GPU of the model processes (`ollama`, `llama-server`, or the program of the command) in each phase. The GPU numbers come from `nvidia-smi` when it runs, and else from the DRM `fdinfo` in `/proc`, which the Intel and AMD drivers fill. On another system, or with no GPU use, they show as `-`.
- Ollama skips an integrated GPU unless `OLLAMA_IGPU_ENABLE=1` is set. So on a computer with only an integrated GPU, such as an Intel Arc laptop, the model runs on the CPU, and the cost shows in the CPU column, not the GPU column.
- The memory of Timeways, and its CPU time in the run when script profiling is on. To turn it on, type `/console scriptProfile 1`, then `/reload`. Profiling costs frame rate itself, so turn it off after the test: `/console scriptProfile 0`, then `/reload`.
- The results go to `dev-bench/fps-<time>.json` and `.txt`, and each run of the addon goes to `dev-bench/fps.jsonl`.

`/twdev fps` works alone too: `start`, play, and `stop` print the numbers of the run in the chat. A hidden frame rate (`issecretvalue`) counts as hidden and in no number. A run stops by itself after 15 minutes. It ends with no line when dev mode turns off.

### What dev mode can't fake

- **The answers of a model.** A narrator line, a saga, a tale, a summary, a history of a zone, a talk, a quest offer of `/quest`, and "Help me write" need a model. The scenarios fix the answers of side quests only.
- **The positions on the map.** A fake place of a command has no point on a map of the game, so it gets no pin. Only `/twdev atlas` gives its places points on the map of the zone where you stand.
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
- For test 26: both answers, as text.
- For test 27: each narrator line, as text.
- For test 30: a screenshot of a zone page with its pins, of a person page, and of the world page.

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
