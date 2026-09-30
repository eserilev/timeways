# Timeways

**Azeroth remembers you.**

Timeways is a World of Warcraft: Forever addon. It adds a story layer to the game: lore on demand, a narrator that remembers you, a chronicle of your adventure, personal side quests, and, with a guild, a shared saga and world PvP feuds.

An AI model writes the words. A rules engine decides what is true. The game itself supplies the facts.

Status: early build. The story program keeps the world of each character in a file, and serves `/lore`, `/talk`, `/quest`, the journal, the narrator, the chronicle, and player tasks. The addon talks to it through the shared transport of Gnomish Relay, and the bridge runs its model calls. Tests in the game have started (`TESTING.md`). The seed of common lore (3.1.1) waits.

## 1. The four parts

| Part | Job | Where |
|---|---|---|
| **Timeways addon** | Watches the game, shows the story, and holds the windows. A separate addon, with its own listing. | This repo, `addon/Timeways` |
| **`timeways-story`** | The story program on the desktop: the world, the lore pack, the scoring, and the model calls. | This repo, `crates/story` |
| **Gnomish Relay** | The one desktop program. It owns the screenshots, the slots, and the keys, and it routes each strip to its app. Its own addon, for the coding agents, is optional for a Timeways player. | `~/Documents/Code/Personal/gnomish-relay` |
| **Hourglass** | Keeps the history of each world, and checks each change that the AI proposes. | Its own public repo: [rusty-hourglass](https://github.com/eserilev/rusty-hourglass), locally `~/Documents/Code/Personal/rusty-hourglass`. The library name is `hourglass`. |

The names tell one story. In WoW lore, the Bronze Dragonflight guards the timeways and keeps the true history of Azeroth. Hourglass is the engine that keeps that history. The emblem is a bronze hourglass.

```
 WoW client                          desktop
 ┌───────────────┐  strip signed   ┌───────────────────────┐  stdio  ┌────────────────────────┐
 │ Timeways      │  with the       │ Gnomish Relay bridge  │  JSON   │ timeways-story         │
 │  (addon)      │ ─ Timeways key ▶│  routes by key        │ ──────▶ │  Hourglass world       │
 │  lore window  │                 │  owns screenshots,    │         │  lore pack, scoring    │
 │  narrator     │ ◀── Timeways ── │  slots, keys          │ ◀────── │  model calls           │
 │  chronicle    │     slots       │                       │         │                        │
 └───────────────┘                 │  coding agents (only  │         └────────────────────────┘
 ┌───────────────┐                 │  for the relay key)   │
 │ Gnomish Relay │ ─ relay key ──▶ │                       │
 │  (optional)   │ ◀─ relay slots ─│                       │
 └───────────────┘                 └───────────────────────┘
```

## 2. Rules of play

These rules come before every feature.

1. **The AI never plays.** It presses no key, and it takes no action in the game. An addon cannot act without a click of the player anyway.
2. **No advantage.** Everything is story and flavor. Nothing helps in combat, in trade, or in a race to a goal.
3. **Story after the action.** A round trip takes 5 to 10 seconds. So the story reacts after a fight, never during it.
4. **The game is the truth.** A kill, a quest, a zone, and a death come from game events. The AI only proposes what these events mean for the story.
5. **Private by default.** The story shows only on your screen. The real name of another player never leaves your computer unless you send it yourself. No model ever sees the name of a real player (5.11).
6. **Only players with the addon take part.** Nobody else sees anything, and nobody else gets a message.
7. **Talk less, and mean more.** The narrator speaks only at big moments. Each line refers to your own history.
8. **Only WoW Forever lore.** The story never goes past where WoW Forever is in the storyline. Section 5.9 says how.

## 3. The solo level

### 3.1 Lore on demand (the first slice)

You target an NPC, stand in a place, or hold a quest, and you ask a question: `/lore why is this tower in ruins?`

- **Context comes with no typing:** your zone, your subzone, your target, your active quests, and your level.
- **Answers come from sources.** A local search of the lore pack (5.10) finds the best passages, and the answer names their sources. No question needs a web request.
- **No invented facts.** When no page supports a claim, the answer says "legend says" or "nobody knows".
- **The spoiler limit.** The agent tells only what your world already holds. Your world holds the places that you visited, the NPCs that you met, and the quests that you finished. The lore of later expansions and of quests that you have not reached stays hidden.
- **A voice in the world.** The answer comes from a local historian.
- **A follow-up question** continues the same conversation.
- **Limits.** The story program refuses a question that is empty, longer than 255 bytes, or holds a control character, and a target that breaks the name limit (5.11). The addon gets an empty answer.
- **The lore book** (built): the answer shows in a small window in the look of the journal. The question is the heading, and the answer is the page, which scrolls when it is long. The page says "Asking..." while the answer comes, and "Nobody here knows." when nothing does. With no model, the page shows the passages, with no sources. Previous and Next step through the last 10 questions of the session. Close and Escape close the book, and `/lore` with no question opens it again. An answer that comes while the book is closed gets one line in the chat. An error reply says so on the page too.

This slice tests the whole chain with one question and one answer: the addon, the relay, the world, the spoiler limit, and the agent.

#### 3.1.1 Emergent lore, on a seeded floor

Most lore comes from play. You learn what your character read or heard: quests, gossip, and books (5.10). Two players know different things, and what you know is part of your story.

- **What you read comes first.** A question uses your own text first. The lore pack only fills the gaps. (Built)
- **The answer says where you learned it:** "You read in *The Kingdom of Stormwind* that…". (Built)
- **The Knowledge page** of the journal lists what you read and heard (3.6).
- **The narrator notices** a first book about a place, or a first story from an NPC, as a small moment.
- **Only game text is canon.** The words of an NPC in `/talk` come from a model. They go into your journal as a rumor, never as a fact, and `/lore` never cites them.

**The seed is the floor.** Some lore is common knowledge in 25 ADP: the kingdoms, the factions, the big names, and the zones. The seed holds it, so `/lore` is not empty on the first day. It stays small and general. The deep lore of the pack stays behind the spoiler limit (3.1), and opens as you visit places and meet people. The zones come from the game files (5.10), and the rest is a short list that a person checks (open question 6).

### 3.2 The narrator

No invented companion rides along. A narrator tells the big moments as they happen, in one short line about "our hero". The same narrator writes each finished chapter of the chronicle (3.3). There is no other storyteller, and no bard.

**Who it is.** The narrator is a keeper of time, and nobody knows more than that. It never names itself or what it serves. It has seen how things end, and it never tells the future: that is the spoiler rule. Its voice is serious, concrete, and sparing, with a dry edge at most. It makes no jokes and no silly lines.


- It reacts to big moments. The story program finds them in the events that each batch adds to the world. From the highest rank down (`moments.rs`):
  - a finished class quest (5.4),
  - a joke title (5.4.1),
  - the first kill of a rare or a boss (the echo of a later kill is not a big moment), or the first entry into a dungeon or a raid (3.3),
  - a second or later death to the same NPC: "The third death to the same murloc.",
  - a slap of an NPC, which it remembers (5.4.1), or a lasting buff or debuff of a quest (5.4),
  - a level up, or the first visit of a capital city (3.3),
  - the first visit of a zone.
- It speaks at most once for each batch, about the best moment. The moments on one line of the list have the same rank. Of two moments of one rank, the later one wins, because it holds the newer count. A flavor moment (5.4.1) speaks only when no big moment does.
- **The narrator stays quiet while a saga is written** (3.3). The bridge runs at most 2 model calls of the story program at once, so a question of the player always gets a call. The moments of the batch wait for the next batch.
- A batch that ends with a question gets no `batch_end` (5.5), so its moments wait for the next batch.
- It stays quiet while a saga is written, and when both model slots of the bridge are taken, so the player keeps a slot.
- It has a budget: at most 3 lines in one hour of game time. The budget lives in memory, so a restart of the story program starts it again. That costs at most 3 more lines once.
- A model writes each line through the bridge, with no tools. The line must be plain text, at most 300 characters, and hold no name from after the cutoff (5.9), except a name that the player wrote first (3.7). A line that breaks a rule gets no retry, and the player sees nothing.
- It remembers your history across sessions, because the world does.
- It speaks in the chat window now. A window of its own, and a voice (Gnomish Relay SPEC 13.3), come later.

#### 3.2.1 The voice

The prompts of the narrator, the chronicle, a talk, and a quest share one plan (built):

- **House rules** are the same for every call: the format, the lore cutoff (5.9), safety, and the rule that the input is data. The input goes between fence marks. The code removes each fence mark from the input text first, so an input cannot close its fence. A hidden mark goes too: the code drops invisible characters, and it removes each run of 3 angles of one direction, also wide or look-alike angles such as `＞` and `›`, with only spaces between them. Names from the game are input too: the name and the place of an NPC in a persona, and the places and the target of a `/lore` question, go between fence marks.
- **The persona goes first.** The persona of the narrator is a short text in `crates/story/data/narrator.txt`: its manner, what it does and never does, and the spoiler rule. It holds no lore.
- **An NPC never gets the persona of the narrator.** It gets a short persona of its own from the facts: its name, its place, and its trust in you as words ("You are wary of the player"), never as a number.
- **Golden samples.** Each prompt of the narrator or of a talk carries 2 or 3 short samples of the voice, in turn: 5 for a narrator line, 5 for a chapter, and 3 for an NPC reply. The samples are data in `crates/story/data/samples/`. A test checks that each sample passes each check.
- **The author's note goes last:** 2 or 3 lines on the tone and the format, because a model follows the end of a prompt best.
- **The checks** refuse an emoji, modern slang, a stock phrase such as "the sands of time" or an hourglass, and a copy of a long phrase of a sample. The banned words are data in `crates/story/data/banned_words.txt`. No banned word is a word that the facts use, such as "level".
- **Names in no fact.** The code logs each proper name of an answer that its prompt does not hold. It refuses nothing yet.
- **The size.** Each prompt fits a local model with a context of 2048 tokens, with room for the longest reply. A test measures the prompts of fixed test moments, at about 4 characters for each token.
- **The voice regression set.** Fixed test moments: a first dungeon, a world boss, a death, a level milestone, a new capital, a finished side quest, a quiet chapter, and an NPC talk. A live test, ignored by default, sends them to a real model through the same prompt code. It writes the answers to a file for review. It also runs the best of two (3.3) for the side quest chapter: two drafts and the judge. It runs the model as the bridge does: `claude -p` with no tools, no MCP servers, and no settings.
- **What the live runs found** (Claude, September 2026, four runs):
  - A chapter said "our hero" in almost every sentence. The note of a chapter now asks for "our hero" at most twice, and each chapter sample holds it at most twice.
  - A chapter listed the facts in order ("There was a task, and our hero finished it."). A quiet chapter padded itself out ("Nothing more of this chapter is known."). The note now asks for a story, not a list, and for two or three sentences when the facts are few.
  - A model copied the aphorism of a sample ("Some days are only a road" became "Some chapters are only a road"). That sample lost its aphorism, and the first sample lost "Nothing else of note happened".
  - Each footnote ended like the example of the prompt ("Nobody knows why" became "Nobody asked why"). The example is now a plain fact with a dry detail.
  - A narrator line brought in the hero sheet at every moment. The note now says to use it only when the moment touches it.
  - A push for "one concrete detail" made the narrator invent places and dropped the level number. The note now asks to name the place, foe, or number plainly, and to add nothing.
  - Still open: a line often repeats "for the first time" from the moment, and a line can name a place from the knowledge of the model ("Azshara" for Azuregos). The NPC talk was the best part in every run.
- **Best of two only for a chapter** (3.3). A narrator line, a talk, and a quest cost one call each.
- `/lore` keeps the voice of a historian (3.1), with the same house rules.

### 3.3 The chronicle

A chapter follows the progress of the character, not the clock. The narrator (3.2) writes each finished chapter as a short saga.

- It uses the real events of the chapter: the zones, the bosses, the deaths, the loot, and the quests.
- You read it in the game as a book, one page per chapter.
- The history of the world is the source, so the chronicle never contradicts itself.
- **Where a chapter begins** (built): the code decides, never a model.
  - The first chapter begins with the first play.
  - A later chapter begins at a milestone: the first visit of a zone, every tenth level, or the first kill of a rare or a boss.
  - A milestone after less than 45 minutes of play in the chapter joins that chapter. So two milestones close together make one chapter, not two thin ones.
  - Only play counts. A stretch of 30 minutes with no event ends a session, and the time away is no play. A pause alone never begins a chapter.
  - A finished class quest is a milestone too (5.4).
  - A dungeon, a raid, and a capital city are zones of their own, so the first entry into one is already the first visit of a zone.
- **The kind of a place** (built): after the zone of an instance, the addon sends `instance_entered` with "party" for a dungeon or "raid" (from `IsInInstance`). A battleground and an arena send nothing. The story program marks the zone, and the six capitals are known by name. The facts of a saga name the kind ("The Deadmines (a dungeon)"). A first dungeon or raid is a big moment for the narrator, as high as a first kill, and a first capital ranks above a new zone.
- **The saga** (built): after a batch, the story program asks a model for the saga, and the footnotes (5.4.1), of the oldest finished chapter that has none yet, one chapter at a time. The last chapter can still grow, so it waits for the next milestone. The facts of the prompt come from the chapter alone. The saga must be plain text in one paragraph, at most 600 characters, with no name from after the cutoff (5.9). A saga that fails keeps the plain list, and gets no retry in the same run. A saga is stored by the tick where its chapter begins. A change of these rules moves the beginnings, so an old saga can lose its chapter.
- **Best of two** (built): a chapter gets two drafts, one after the other. The second draft has the same facts and the next samples in turn, so the two drafts differ.
  - Each draft goes through the checks of a saga: the voice, the banned words, the cutoff, and no repeats.
  - When both drafts pass, a short judge call picks one. Its prompt holds the persona, the facts, and the two drafts. It answers `{"pick": 1}` or `{"pick": 2}`. A bad answer or a failed call picks draft 1.
  - When one draft passes, it wins with no judge. When none passes, the chapter keeps the plain list.
  - The second draft and the judge go out only when no other call is open.
  - **A question of the player never fails for a saga** (built). The bridge runs at most 2 calls of the story program at once, and fails a third one. So the story program opens at most 2 calls. A call of the player (a lore question, a talk, a task, or a draft) that finds both slots taken waits for the first one to end. The narrator never waits: it stays quiet.
  - So a chapter costs at most 3 calls. When the budget window is tight, it costs 1, as before: the first draft that passes wins. The window is tight after a failed call in the last 20 minutes, because the bridge refuses a call over its budget with a plain `model_failed`. It is also tight after 5 calls in the last 20 minutes: the bridge admits 10.
  - The drafts live in memory only. Only the final saga goes to the disk. A restart drops the drafts, and the chapter starts again with a first draft.
- **Chapter memory** (built): the prompt of a chapter also carries a short summary of each of the 3 chapters before it. The code writes each summary from the facts of its chapter. The model never writes one.
- **The hero sheet in a saga** (built): the sheet (3.7) goes only into the prompt of the first chapter, and of a chapter in which the player changed it. So the chapters do not all open with the same portrait. The entries that the player wrote in a chapter always go into its prompt.
- **No repeats** (built): a saga that holds 8 words in a row of an earlier saga is refused, and its chapter keeps the plain list.
- **Without a model** (built): a chapter lists what was new in it: the zones, the people, and the deeds. A chapter with nothing new gets no number. Each list of a chapter keeps at most 20 entries, so a chapter always fits on one page of the journal (5.5).

### 3.4 Personal side quests

An innkeeper tells you a rumor, and the rumor becomes a small quest line made for you.

- Each step uses a real game goal that the addon can check from game events: kill 8 of a mob, visit a place, collect an item, talk to an NPC. The format follows [PlayerMadeQuests](https://github.com/runeberry/PlayerMadeQuests), which already tracks these goals.
- **No overlap with the quests of the game.** A side quest never repeats, continues, or changes a quest of the game: one that exists now, or one that comes in a later phase. It never sends you to the goal of a game quest, and it never claims a deed of one. It lives only in your own story.
- A quest is plain text. You read it before you accept it.
- The rewards are story: a title in your journal, a line in your chronicle, and an NPC who trusts you more and tells you more lore later.
- Hourglass keeps the quests consistent. A quest cannot send you to an NPC who died in your story, or to a place that you never heard of.

**The first slice.** The combat log is closed and no loot event comes yet, so a step has one of three goals: `visit` a place, `meet` an NPC, or `kill` a count of one creature ("Kill 6 Duskbats").

- **Seen and met** (built). The addon sends each NPC that you hover or target once in a session (`npc_seen`): its name, `hostile` when you can attack it (`UnitCanAttack`) or `friendly`, and its creature type in English (`UnitCreatureType`, by its id: "beast", "humanoid"). The addon keys a session by the NPC id of `UnitGUID`, and never sends the GUID. It never sends a player or a pet, and it checks the name, the GUID, the reaction, and the type with `issecretvalue`. The world keeps `seen`, apart from `met`: seeing is not talking. The last sighting says whether the NPC is `hostile`, and a beast or a critter is an `animal` for good (5.1). A talk in the game (`npc_met`) also ends `hostile`: an NPC that talks to you is a friend now.

- **The offer** (built): you target an NPC that you met and type `/quest`. The event `quest_asked` has no reply of its own. At the end of the batch, the model call of the quest takes the place of the narrator call. The offer comes back as a notice of the batch answer: a line of Timeways itself, shown with the "Timeways:" prefix, never in the voice of the narrator (the optional `notice` of the relay protocol). Two cases move the line to the next answer of any kind: a batch of events, a question, or a journal page. A batch that ends with a question gets no `batch_end` (5.5), so its `/quest` goes to the model at the question. And the relay drops the answer of a batch of events after 60 seconds, so an offer that comes 55 seconds or more after the `batch_end` waits. The Tasks page shows the offer at once in both cases. Asking is meeting, as a talk is. A hostile NPC or a beast has no task: the line says that it has no task for you now, and asking does not meet it. With no model, or with an offer that breaks a rule, the line says that the NPC has no task for you now.
- **The check** (built, `quest.rs`): the code refuses an offer that breaks one of these rules.
  - The answer is JSON with a title (at most 60 characters), a text (at most 400), and 1 to 3 steps. The offer line fits in one notice (1000 bytes).
  - Each name is a string of the game, copied exactly, because progress matches it byte for byte.
  - A place is a zone or subzone that you visited. The addon names the zone of a text that you read by where you read it, so it adds no place.
  - An NPC to meet is one that you met or saw. It is not hostile, not an animal, not dead in your story, and not the giver. So a task never sends you to talk to a bat.
  - A creature to kill is one that you saw hostile, not dead in your story, and not the giver. A kill step asks for 1 to 10 kills.
  - Each step names a different target, so one event never does two steps.
  - **No target twice in a row:** no place, NPC, or creature of your newest task comes again in the next one, from the same giver or any other. The newest task is the last offer in the log, in any state.
  - **No overlap:** a subzone, NPC, or creature of a step is not in the title or the text of a game quest that you read. The title does not have the words of the title of such a quest, in any case. A zone is exempt, because most quest texts name their zone. The rule covers only the quests that you read.
- **The prompt lists only what the check allows** (built): the places that you can visit, the NPCs that you can meet, and the creatures that you can hunt, at most 20 of each. The targets in the zone of the giver come first, then the rest. Inside each group, the newest come first: the place of the newest first visit, and the NPC of the newest first meeting or sighting. One function of the code decides both the lists and the check, so they never disagree.
- **Limits** (built): each giver has at most one offer that waits, and a new offer of the giver ends the old one. A giver with an open quest waits for you to finish it. You hold at most 3 open quests. The limits hold when you ask, when the offer comes, and when you accept, because the world moves on while the model thinks. A refusal (when you ask or accept) comes back as a notice of its batch, or of the question that ends the batch, never as a narrator line.
- **Accept and progress** (built): the buttons of the Tasks page name their quest by its number, so you accept the offer that you read. `/quest accept` or `/quest decline` answers the newest offer. The story program checks each step against later `zone_entered`, `npc_met`, `talk_asked`, `npc_slapped`, and `npc_killed` events, in order. A step that holds when it becomes the next step is done at once, because the addon sends a zone only when it changes. The addon sends an NPC again only after 5 minutes, and it forgets the NPCs that you met when you accept a quest.
- **Kills** (built): the journal tells the addon the creature of the next step of each task in progress, when that step is a kill. The addon keeps, in memory only, the GUID of each unit of such a creature that you see and can attack. `PARTY_KILL` for one of these units sends `npc_killed` with the name, once for each unit. A visit or a meeting can make a kill step the next one. So after a batch of events, the addon asks for the journal again while a task has a later kill step, at most once a minute. A unit of the new creature that you already target or hover counts at once. The story program counts a kill only for the next step of an accepted task, and only up to its count, in the quest file. A kill before you accept counts nothing. A kill step keeps its creature by name. When the creature is friendly later, the addon counts no kill of it, so the step waits until you abandon the task. The Tasks page shows the count as the game does: "Duskbat slain: 2/6". At the end of a task, you get `quest_done`, and the giver trusts you 10 more. The model never picks this number. The finished quest is a deed, so it shows on the Deeds page and in the chronicle, and the saga gets it as a fact of the chapter.
- **Storage** (built): the quest file `c_<name>.quests.jsonl` holds the offers, the answers, the kills, and the steps done. The world holds the facts: the giver holds `quest_offered`, and you hold `quest_accepted` and `quest_done`. The quest thing is named "quest <number>: <title>", so it never merges with a title.

### 3.5 Talk to an NPC

You target an NPC and type `/talk <words>`. The agent plays that NPC. What the NPC tells you, and how much it trusts you, go into your world.

Built:

- **Talking is meeting.** The NPC enters your world with `met` before the model answers.
- **What the NPC knows:** its place, your level, your slaps (5.4.1), its trust in you, and up to 3 lore passages about it, under the spoiler limit (3.1).
- **The NPC proposes, and the code decides** (5.2). The model answers in JSON: `{"say": "...", "trust": n}`.
  - The words follow the rules of a narrator line, with at most 400 characters.
  - A change of trust outside -5 to 5 is dropped, and the words still show.
  - A valid change goes through Hourglass, inside the band of -100 to 100.
- **No retry.** With no model, or with an answer that breaks a rule, the NPC "looks at you and says nothing".
- The target counts only when it is an NPC: never a player, and never a pet (5.11).
- **Trust shows on the NPC, not in the book** (built). The tooltip of an NPC that you dealt with gets one line: "Timeways: Likes you. Slapped 2 times." An NPC that you only met gets none. When the feeling of an NPC changes band, the chat says so once: "Keeper Tessa now likes you." The bands: 50 and up trusts you, 10 and up likes you, -9 to 9 is neutral, -10 to -49 is wary of you, and below that distrusts you. The addon asks for the journal at login and after each talk, so the tooltips know the people before the book opens.
- **No People or Places page.** The chronicle names the people and places of each chapter, the map shows where you went, and the tooltip shows trust. The journal still carries the people and the places, for the map and the tooltips.

### 3.6 The journal

A window in the game, in the look of the Map and Quest Log of the game. `/journal` or `/timeways` opens it, and Escape closes it.

- **The frame:** a path and one row of tabs on top, the map on the left, the parchment on the right, and a bar of buttons at the bottom. The tab of the open section is marked. The buttons are the dark red buttons of the game.
- **The map** is the game's own map art of one zone (`C_Map`). A chapter or a task shows its zone when the game has a map of that name. Every other page shows the zone of the player. A pin marks where the player stands, on the map of the zone where the player is. With no map, the pane says so.
- **Where you have been** (built): the base art of a zone leaves out each part that the character never explored. The map draws each explored part over it, as the world map of the game does (`C_MapExplorationInfo.GetExploredMapTextures`). A part that the game shows only under the mouse stays hidden. A line at the bottom of the map names the subzones of the zone that you visited, in the order of the first visit. The game names no subzone before a visit, so the line names only visits, and the dark parts of the art show the rest.
- **Task pins** (built): the Tasks page shows the map where the giver of the open task stands, when the journal knows the position. Else it shows the zone of the giver. The giver gets the yellow mark of a quest giver: "!" for an offer, and "?" for a task that you took. Each step gets a pin with its number: a `visit` step at the position of its place, and a `meet` step at the position of its NPC. A done step fades. A step with no position, or with a position on another map, gets no pin.
- **A list and its page.** Tasks, Chronicle, and Hero have a list on the left. A click on a row opens it on the parchment. Over the map, the list floats in a dark box. The Hero list fills the left half on parchment, and covers the map.
- The other pages show their lines on the parchment, beside the map of the player.
- **Positions** (built): the addon sends where the player stands with `zone_entered` and `npc_met`: the map (`C_Map.GetBestMapForUnit`) and the point on it (`C_Map.GetPlayerMapPosition`), in whole thousandths. The journal map shows zones, so a city district or a cave goes up its parents (`parentMapID`) to its zone map, and the point is the one on the zone map. When the zone map knows no point, as in a dungeon, the position stays on the map where you stand. A hidden value, no map, a point off the map, or the point 0, 0 (the game gives it where it knows no position) sends no position. The place where you stand keeps the position of its first visit that had one, and an NPC keeps the position of its first meeting that had one. The zone around a subzone takes the same position when it has none yet, because you mostly stand in a subzone. The world holds them as `on_map`, `map_x`, and `map_y` (5.1), so they never move. A position off the map counts as none, and the event still counts. The journal carries each position with its place or its person.

The desktop sends the pages each time the book opens, because the world lives there (5.10). No model takes part in a page, and the sagas are stored words. Each page comes from the facts of the world, its history, and the files next to it (5.7).

| Section | What it holds | State |
|---|---|---|
| **Hero** | Your sheet and your own lore (3.7). The list holds each question of the sheet with its answer. The open question has an Edit button, and under it come your notes with Add a note and Remove. Previous and Next step through the questions, and the bar counts the answered ones. The first time that the book shows an empty hero in a session, it opens here. | Built |
| **Chronicle** | One chapter for each milestone (3.3), with the saga and the footnotes when a model wrote them. The list names each chapter by its first zone. A chapter shows its places, its people, and its deeds, with Previous chapter and Next chapter. The book opens on it, at the newest chapter. | Built |
| **Deeds** | Level milestones, first kills of rares and bosses, repeat kills (echoes, 5.13), your deaths, and your joke titles (5.4.1) | Built |
| **Knowledge** | What you read and heard (3.1.1): each book, each quest tale, and each story of an NPC, with the place and the date. A rumor from `/talk` shows as a rumor. | Built |
| **Nemesis** | Real players from world PvP only: the kill count on each side, the places, and the last time seen (4.1). Aliases only (5.11). | Later |
| **Tasks** | The personal side quests (3.4). The list groups the offers, the tasks in progress, and the done ones. The open task shows its steps, its state, and its rewards, with Accept and Decline for an offer, and Abandon for a task in progress. Then the tasks of players (4.7): the ones from players, Give a task, and the ones that you gave. They need no desktop, so they show while the journal loads. | Built |

### 3.7 The hero

Who your hero is, in your own words, as a player of a tabletop game writes before the first session. It is your hero's own story, never canon: `/lore` never reads it.

- **The sheet:** origin, background, goal, bond, flaw, and traits. Each field is optional, and holds at most 1000 characters. You change a field at any time, and an empty text clears it.
- **Your own lore:** entries that you add at any time, for example "A stranger at the inn knew my father's name." Each entry keeps its time and the place where you stood, and the NPC that you targeted when it is about one. You can remove your own entry.
- **Nothing is lost.** Each change is a new line in `c_<character id>.hero.jsonl` next to the history (5.7). A removal and an old text of a field stay in the file.
- **Your words are free.** No check reads the words of the player: a name from after the lore cutoff (5.9) is fine in your own story. A text has only these limits: an entry is not empty, no control character, at most 1000 characters, and at most 1200 bytes. The six fields go together on the first page of the journal, so a text also fits a sixth of a slot of the game: a text full of quotes is too long. An edit has no reply of its own, so a refused text leaves its reason for the next journal page, and the addon shows it once. The editor of the book stops at 1000 letters, and keeps a text over 1200 bytes open with the reason under it.
- **A later name in a model answer:** the narrator, a chapter, and an NPC can name something from after the cutoff when your own text names it first. The check of the cutoff reads your sheet and your entries as allowed words. It reads each text alone, so a name split across two texts ("Caverns" at the end of one, "of Time" at the start of the next) is not allowed.
- **Who reads it:**
  - The narrator (3.2) gets the sheet and the 5 newest entries, under the heading "the hero's own story, not canon", for a line and for a chapter (3.3). A prompt takes the first 300 characters of each text, so a long story keeps the prompt small.
  - The prompt of a chapter also gets the entries written during the chapter.
  - An NPC in `/talk` (3.5) gets only the entries about it or about its place, at most 5.
- **The journal** carries the sheet on its first page, and the entries as a list like the others.
- **In the game:** the Hero page of the book, or `/hero`, `/hero add <text>`, `/hero note <text about your target>`, and `/hero set <field> <text>`. Edit and Add open a writing page in the book: a box of several lines that scrolls, stops at 1000 characters, and counts them ("16 / 1000"), with Save and Cancel. Remove asks first in a dialog of the game.
- **An edit shows at once.** The book shows the new text, marked "Saving...", until the next journal comes. Each edit goes out with a journal request, so that journal comes soon. It shows what the desktop saved, or leaves out a refused edit and shows the reason.

## 4. The social level

The social level needs the same world and the same director as the solo level, plus sync between players. So it comes after the solo level.

### 4.1 Nemesis (world PvP)

- The enemy player who kills you becomes your nemesis. Addons cannot read the combat log in this client (open question 9), so the source of the killer is open.
- Your chronicle keeps the feud: the kill count on each side, the places, and your revenge.
- A grudge outlives each fight, and hourglass keeps it: `nemesis` is a fact that stays until you end it.
- When your nemesis is near, you get a revenge "hunt".
- The addon never sends a taunt by itself. You send any message yourself.

### 4.2 The guild saga

- Each boss kill, each first kill, the number of wipes, and the first player to die go into the history of the guild.
- The sources are `ENCOUNTER_END` and `BOSS_KILL`. Wipes and the first player to die need another source, because the combat log is closed (open question 9).
- The narrator writes each raid night as a saga, and every member with the addon reads the same book.

### 4.3 The raid herald

Before a pull, the raid leader gets a short battle speech from the history of the guild: "Third night on Ragnaros. Last week we fell at 5%." The leader sends it with one click, or does not.

### 4.4 The bounty board

Officers set bounties on enemy players or on rare mobs. The addon tracks the kills with `PARTY_KILL` (5.4), and the saga names the winner.

### 4.5 A shared guild world

- A weekly guild story, with guild quests that need everyone: "200 copper bars for the forge of the keep."
- Progress syncs through addon messages.
- Each shared world has one **keeper**: the bridge of one officer. The keeper accepts or refuses each proposal. The other members replay its history, so two agents never tell two stories.

### 4.6 A shared narrator

In a dungeon group, the narrator of each player with the addon tells the same big moments of the group. How the narrators avoid saying the same thing twice is open.

### 4.7 Player tasks

One player writes a task for another player who also has Timeways. The giver is the author, the game checks what it can, and the giver decides at the end. Built, in the addon: a task never goes to the desktop. Only "Help me write this" (below) asks the desktop, for a draft.

**Who can send.** A task goes only to a player in your party, your guild, or your friends list, who has Timeways and is online. The form asks with a `hello` to the group, the guild, and each friend online. Each addon that hears it from one of these players answers `here`. The receiver checks the same rule for every offer, and a player can block a giver. The giver's addon learns of the block and sends no more tasks.

**The task.** A title (at most 60 bytes), a text (at most 400), an optional promise (at most 100), and 1 to 5 steps. A letter such as "é" takes two bytes, so the form checks each text in bytes, as the wire does. A text that is too long, or an item that the form cannot read, stays in the box with the reason under it. A step is only what the game can check, and its name comes from the game, never from typing, except the name of an item:

| Step | The doer's addon sees it when | The giver's addon witnesses it when |
|---|---|---|
| Go to a place (where the giver stood) | the zone or subzone is the place, also at the accept | the giver stood in the place at that time, in a party with the doer |
| Talk to an NPC (the giver's target) | a talk window of that NPC opens | the giver's addon saw that NPC within 2 minutes, in a party with the doer |
| Defeat a creature or a player (the giver's target) | `PARTY_KILL` of a unit with that name, by the doer or a member of the doer's group, as many times as the count | the giver's addon saw as many kills of it while the doer was in its group |
| Find a player (the giver's target) | the doer targets that player at trade distance | only for the giver: the giver's addon saw the doer next to it |
| Bring an item to the giver ("10 Linen Cloth") | trades with the giver hand over the count | the giver's trades got the count |

The same foe added again raises its count. A kill by any member of the group counts, as quest credit does in the game. The last step is always the turn-in, face to face.

**The flow.** The giver sends the offer. The doer reads it in the Tasks section of the journal, and clicks Accept, Decline, or Block player. The doer's addon claims each step with its time and zone, and tells the giver. When every step is done, the doer clicks Turn in. That message carries every claim again, so a lost message costs nothing. The doer can click it again while no answer came. A step or a turn-in of a task that the giver finished or canceled gets that answer again, so a lost answer costs nothing either. The giver's addon shows the turn-in card: each step with its proof, and Complete task or Not yet. Complete task works only when the doer stands at trade distance (`CheckInteractDistance`). The doer can give up, and the giver can cancel an open task.

**Proof.** Nothing can be guaranteed, because the doer's addon runs on the doer's computer. So each step on the card has a level (`TaskProof.lua`, pure functions):

- **Witnessed:** the giver's addon saw it too.
- **Seen:** only the doer's addon recorded it.
- **Not confirmed:** the giver's addon was in a place to see it, and saw nothing. That is a step whose message came while the doer stood within about 28 yards of the giver (`CheckInteractDistance`, the follow distance). A doer farther away gives Seen, also in the same party and zone, because a kill reaches only players nearby. An item step is Witnessed or Not confirmed, because a trade is face to face.
- The time and the zone in a step message are the doer's word. So for a step message that comes live, the giver's addon uses its own clock and its own range check. Only a claim that got lost and comes again with the turn-in keeps the doer's time.
- The giver's addon judges each claim when it comes, and keeps that level: records that the store cuts later never change it. A witness that comes later still raises it to Witnessed.
- A party is never required. The giver's addon records each stretch of party time with a doer (`GROUP_ROSTER_UPDATE`), and each change of the zone or subzone where the giver was, with times, from the moment that it gives a task. A logout ends the stretch and records no place, because an addon that is offline sees nothing. The clocks of two computers differ, so "the same time" means within 2 minutes.

**The reward** is a promise, never mail. The giver hands it over in a normal trade. The giver's addon watches the trade window with the doer (`TRADE_SHOW`, `TRADE_ACCEPT_UPDATE`, the item and money events, and `TRADE_CLOSED`), and reads the items and money of each side at each change, because the window empties as it closes. A trade counts when both players accepted and the window closed, or when the game says "Trade complete." (`UI_INFO_MESSAGE` with `ERR_TRADE_COMPLETE`): the game can make the trade on the second accept before it tells of that accept. The line reads "Reward: promised", "Reward: paid in trade" after any trade in which the giver gave the doer money or an item, or "Reward: not paid" for a finished task with no such trade. A trade with the doer of a finished task whose reward is not paid still counts, because the giver pays after Complete task. Whether a trade that fails after both accepts (full bags) also closes the window is open: a test in the game settles it.

**The chronicle.** A finished task shows on the doer's page with one line for the Chronicle: "Corvin finished Trouble at Agamand Mills for Ada. They met face to face in Brill to turn it in." The line names two real players, so it stays in the addon: it never goes to the story program, and no model sees it (5.11).

**Help me write this** (built). A model turns the giver's idea into a title, a text, and steps, and the giver picks "Use this" or "Keep mine". Nothing changes until the giver picks, and every field stays editable after.

- The addon sends `draft_asked` with the idea, at most 255 bytes after the names are out. The idea loses the name of each player that the addon knows: the group, the whole guild roster, the friends list, the players of your tasks, and the players on the form. A name matches as a whole word, in any case, with or without its realm, also with letters such as "é" (`TaskNames.lua`). It becomes "my friend", and the giver's own name becomes `$N` (5.11).
- The story program asks a model with no tools, with the places and NPCs of the world: the places that you visited, the NPCs that you met and that live, and the foes that you saw hostile and that live, with the rares and bosses that you defeated. No player is in the prompt.
- The code checks the draft (`draft.rs`) before it goes back as `draft_answer`: JSON with a title of at most 60 bytes, a text of at most 400 bytes, and 1 to 5 steps, with the limits of the addon messages. A title or a text holds no `|`, no control character, no emoji, no banned word, and no name from after the cutoff (5.9). A step is `place` (a zone or subzone that you visited), `npc` (an NPC that you met and that lives), `kill` (a foe that you saw hostile and that lives, or a rare or a boss that you defeated), or `item`, and no step comes twice: "3 Rattlecage Soldier" and "2 Rattlecage Soldier" are the same step. A `kill` or an `item` can start with a count from 1 to 250: "3 Rattlecage Soldier", "10 Linen Cloth". A draft that breaks a rule, or a failed call, comes back as no draft, and the form says "Timeways couldn't turn that into a task."
- The addon checks the draft again with the rules of the wire, because the bridge doubles each `|`. It merges the same step twice into one with both counts. It puts the giver's name in place of `$N`, and drops a draft with any other `$`, because the game shows such a code as it is.

**The messages** go with `C_ChatInfo.SendAddonMessage` and the prefix `Timeways` (5.8), as whispers, and a `hello` to the group or the guild. A group of the group finder gets it on `INSTANCE_CHAT`. The receiver takes every type but `hello` only as a whisper.

- Each message is `1;<type>;<fields>`, with `%` and `;` escaped. The types are `hello`, `here`, `offer`, `accept`, `decline`, `block`, `cancel`, `step`, `turnin`, and `result`. The receiver checks the version, the type, the exact number of fields, each text against its limit, and each number against its range. A text with a control character or a `|` is refused, because a `|` starts a WoW escape such as a fake link.
- An offer comes only from party, guild, or friends. Every other type comes only from the other player of its task.
- A message goes in parts of at most 255 bytes: `<number>:<part>:<parts>:<text>`, at most 16 parts. The number starts at random after each load, so a part from before a reload never joins a new message. The receiver keeps at most 4 open messages for each sender and 32 senders, and drops a part after 60 seconds.
- Rate limits: the addon sends at most 8 parts in a burst and one each second after it. A peer gets 24 parts in a burst and one each 2 seconds after it: a peer that floods loses its own parts, and nobody else's. A giver has at most 3 offers waiting for you, you hold at most 20 open tasks, and you give at most 20 open tasks.
- A whisper waits while its player is offline, because a whisper to a player who is offline puts an error in the chat. The group, the target, the friends list, a message in the last minute, and the guild roster tell who is online. The addon asks the server for a new guild roster at most each 15 seconds, because the client does not keep it fresh. For a player who is in none of these, such as one who left your group, the whisper goes. If the game answers that the player is offline, the whisper tries again after 5 minutes, or at once when the player shows as online.
- A part that the game refuses waits with the parts after it, and goes again. A message that the game can never send, such as one to a group that you left, is dropped. Waiting messages live in memory.

**Storage.** The saved variables of each character, `TimewaysTasks`, hold the tasks, the blocked players, and the giver's records, each list cut to its newest 100, and the zones to their newest 300. Stretches of party time stay only for the doers of the tasks that the store keeps. Any addon can read them (5.10). So they hold only what the two players already share in the game: the task, the names, and times, never a key. Any addon can also write them, so the addon checks each task and record when it first reads them (`TaskSaved.lua`), and drops a broken one.

## 5. Technical design

### 5.1 The world of a character

Each character has one Hourglass world. A guild has one more world, held by its keeper.

**Entity types** (Hourglass has a closed list):

| Hourglass type | In Timeways |
|---|---|
| `Person` | You, each NPC that you met, each enemy player that you fought |
| `Place` | Each zone and subzone that you visited |
| `Thing` | A named item, a relic, a quest object |
| `Faction` | A faction of the game, your guild, a band of bandits from a rumor |

**The fact vocabulary** (a first draft; `FactVocabulary` holds the rules):

| Fact | Shape | Direction | Links | Meaning |
|---|---|---|---|---|
| `located_in` | flag | free | one place | Declared by Hourglass itself. Where an NPC lives, or where you are. |
| `met` | flag | up | person to person | You talked to this NPC. It never ends. |
| `seen` | flag | up | person to person | You hovered or targeted this NPC (3.4). Seeing is not meeting. It never ends. |
| `hostile` | flag | free | none | You can attack this NPC. The last sighting starts or ends it, and a talk in the game (`npc_met`) ends it. |
| `animal` | flag | up | none | A beast or a critter: no one to talk to. |
| `trusts` | number, -100 to 100 | free | person to person | How much an NPC trusts you. |
| `visited` | flag | up | person to place | You were in this place. It feeds the spoiler limit. |
| `knows_lore` | flag | up | person to thing or place | You heard this piece of lore. It feeds the spoiler limit. |
| `dead` | flag | up | none | Only for an NPC of your own story, never for a canon NPC (5.13). An `up` flag never ends, so the dead stay dead. |
| `defeated` | number, 0 to 1000 | up | person or faction to person | How often the killer killed the target. A kill is a deed of the killer. The target stays alive (5.13). |
| `nemesis` | number, 0 to 1000 | free | person to person | The kill count of a feud. Each side has its own value: `nemesis` on `P7` linked to you counts the kills of `P7`, and `nemesis` on you linked to `P7` counts yours. |
| `quest_offered`, `quest_accepted`, `quest_done` | flag | up | person to thing | A personal quest and its state. |
| `game_quest_taken`, `game_quest_done` | flag | up | person to thing | A quest of the game that you took, and that you turned in. The thing is named `game quest: <title>`. |
| `class_quest` | flag | up | none | On the thing of a game quest that only your class gets. |
| `dungeon`, `raid` | flag | up | none | On a zone that the game called an instance. |
| `marked_by` | flag | up | person to thing | A lasting buff or debuff that a quest of the game put on you. The thing is named `mark: <name>`. |
| `mark_of` | flag | up | thing to thing | The quest of the game that put a mark. |
| `level` | number, 1 to 60 | up | none | Your level. It only rises. |
| `deaths` | number, 0 to 1000 | up | none | Your deaths, with a known killer or not. A known killer also holds `defeated`. |
| `slapped` | number, 0 to 1000 | up | person to person | How often you slapped an NPC. It never ends. |
| `title` | flag | up | person to thing | A joke title of your journal, such as "Scourge of Squirrels". |
| `member_of` | flag | free | person to faction | Guild membership, and faction ties. |
| `leader_of` | flag | free | person to faction | A canon leader, for example Thrall and the Horde. Only the canon seed and game events change it (5.9). |
| `on_map` | number, 1 to 1000000 | free | none | The map of the game (`C_Map`) where a place began or where you met an NPC. |
| `map_x`, `map_y` | number, 0 to 1000 | free | none | The point on that map, in thousandths of its width and height from the top left. |

A count stops at 1000. A death, a kill, or a slap past it still lands: the NPC moves, and the slap still costs trust.

The vocabulary has a version. Hourglass migrates an old world to a new version (`migrate.rs`).

### 5.2 Two sources of change

1. **Game events are the truth.** The addon sends them, and the story module turns each one into Hourglass events: a new zone becomes `EntityCreated` and `visited`, a quest turn-in becomes `quest_done`, a level up becomes a `FactUpdate` of `level`, and a notable kill adds to `defeated` (5.13). They still go through `World::propose`, so a bug in the addon cannot break the world.
2. **The director proposes.** The agent gets a briefing and proposes events and text: "the innkeeper now trusts you (+10)", "a rumor about the Molsen farm". `World::propose` accepts or refuses each event. A refusal carries every reason at once (`Rejection`), so one retry fixes everything. After one failed retry, the proposal is dropped, and the text that depends on it is not shown.

### 5.3 The director loop

1. **Brief.** `World::brief(for_entity, budget)` gives the entities that matter and the tail of recent events. Timeways writes the words of the prompt; Hourglass writes none.
2. **Ask.** The prompt goes to the agent through Gnomish Relay. It holds the briefing, the vocabulary (`FactVocabulary::describe`), the feature (lore, narrator, chronicle, quest), and the rules of section 2.
3. **Answer.** The agent returns JSON: a list of proposed events, and the text to show.
4. **Check.** Each event goes through `World::propose`.
5. **Show.** The accepted text goes back to the game.

The spoiler limit is a property of step 1. The briefing holds only what your world holds, so the agent has nothing else to spoil. The prompt also tells it not to add lore from outside the briefing and the cited pages.

### 5.4 Game events that the addon watches

A first list. Each name goes through the API gate of Gnomish Relay (`scripts/wow-api.sh`) before use, because the Forever client can differ from retail.

| Moment | WoW events |
|---|---|
| New zone | `ZONE_CHANGED_NEW_AREA`, `ZONE_CHANGED` |
| Level up | `PLAYER_LEVEL_UP` |
| Quest of the game taken and done (built) | `QUEST_ACCEPTED` and `QUEST_TURNED_IN`, with the title from `C_QuestLog.GetInfo`. The log puts a quest of your class under a header with the name of the class (`UnitClass`), so the addon marks it as a class quest. The log is read at login too, so a quest taken before still counts. A finished class quest is a big moment for the narrator (3.2) and a chapter milestone (3.3). |
| Lasting buff or debuff of a quest (built) | `UNIT_AURA` for the player only, with its `addedAuras`. An aura counts only if it starts within 60 seconds after an event of a quest of the game (`QUEST_ACCEPTED`, `QUEST_WATCH_UPDATE`, `QUEST_TURNED_IN`), and it belongs to that quest. It must come from no player or pet, last 10 minutes or more (or have no end), and start out of combat. The state at login (`isFullUpdate`) and any hidden value never count. A short list of spell IDs drops Resurrection Sickness, the world buffs, and the Darkmoon fortunes. Each mark counts once. It is a deed, a fact of its chapter, and a moment for the narrator below a finished class quest. |
| Kill of a rare or a boss | `PARTY_KILL` for a unit that the addon saw as rare, rare elite, or world boss (`PLAYER_TARGET_CHANGED`, `UPDATE_MOUSEOVER_UNIT`, `NAME_PLATE_UNIT_ADDED`), and `ENCOUNTER_END` with `success` 1 |
| An NPC that you see (built) | `PLAYER_TARGET_CHANGED` and `UPDATE_MOUSEOVER_UNIT`, with `UnitCanAttack`, `UnitCreatureType`, and the NPC id of `UnitGUID`. Once for each NPC in a session (3.4). |
| Kill for a task (built) | `PARTY_KILL` for a unit of the creature of a kill step that comes next (3.4) |
| Your death | `PLAYER_DEAD`, and the killing blow from `C_DeathRecap.GetRecapEvents()` |
| Boss fight | `ENCOUNTER_START`, `ENCOUNTER_END` |
| Talk to an NPC | `GOSSIP_SHOW`, `QUEST_GREETING`, `QUEST_DETAIL`, `QUEST_PROGRESS`, `QUEST_COMPLETE` |
| The text that you read | The same events, and `ITEM_TEXT_READY` for a book (5.10) |
| Loot | `CHAT_MSG_LOOT` |
| Group and guild | `GROUP_ROSTER_UPDATE`, `GUILD_ROSTER_UPDATE` |
| Player tasks (4.7) | `CHAT_MSG_ADDON`, `TRADE_SHOW`, `TRADE_ACCEPT_UPDATE`, `TRADE_CLOSED`, and the events above for the steps |

The addon sends game events in batches with the next strip. Nothing needs to arrive at once.

**The combat log is closed to addons in this client.** `COMBAT_LOG_EVENT_UNFILTERED` fires, but only Blizzard code can read its payload (`C_CombatLogSecure` is secure-only). So the addon reads kills and deaths from the events above:

- It keeps, in memory only, the GUID of each rare, rare elite, and world boss that it sees. `PARTY_KILL` gives the GUID of the target of a killing blow of you or your group.
- A raid boss gives both `PARTY_KILL` and `ENCOUNTER_END`, so one name counts once in 2 minutes.
- The client can hide a value from addons ("secret values"). The addon checks each GUID and name with `issecretvalue`, and never compares or stores a hidden one.
- A death names its killer only when the killing blow of the recap has the GUID of an NPC (`Creature-` or `Vehicle-`), and the addon never saw the name on a player. With no GUID, the death names no killer. So the name of a real player never leaves the computer (5.11).

**What it watches, and what it never watches.** Timeways takes in chosen moments, not every action. A player makes thousands of actions per hour, a strip holds at most 3200 bytes, and a story needs meaning, not a damage log.

- **At once:** a boss kill, your death and the killer, a rare, a level up, a first visit to a zone.
- **Counted, then sent in a batch:** repeated kills ("23 Defias in Westfall"), and the flavor moments below.
- **Never:** each single cast, hit, or step; the text of chat and whispers; keys and clicks; your path; the private data of other players; your gold and bags, except notable loot.

The one name of another player that Timeways keeps is the enemy who kills you in world PvP. The nemesis feature (4.1) uses it only in your own world.

#### 5.4.1 Flavor moments

Small, silly moments are often the best part of a story. The addon collects them cheaply and the story uses them rarely.

| Moment | Source |
|---|---|
| A critter kill: a squirrel, a rabbit, a cow | Open: the combat log is closed (open question 9) |
| An emote of yours: `/dance` in Goldshire, `/slap` an NPC, `/kiss` a guard | A hook on `C_ChatInfo.PerformEmote` (`hooksecurefunc`, which changes nothing). The target counts only when it is the current target and an NPC, because a typed name can be a player's. Built: every emote. |
| A silly death: a fall, drowning, lava, a critter, a mob far below your level | `PLAYER_DEAD` and the death recap: its `environmentalType`, and its killer with the level that the addon saw. Built, but a critter needs the combat log. |
| An odd habit: the same mob 50 times, fishing up boots, a long AFK in a capital | Counts in the addon |

- **Counted locally.** A tally is tiny, for example `dance Goldshire 3`, and it goes out with the next batch. No moment costs a strip of its own.
- **Marked when it is funny:** a first time, a streak ("12 squirrels in a row"), an odd place or time (a dance in Goldshire at 3 AM), or a contrast (a level 60 that dies to a cow).
- **Used rarely.** The narrator picks one now and then, with a cooldown: "The fourth rabbit today." The chronicle gets footnotes: "On the fourth day, our hero danced in Goldshire. Nobody knows why." The journal gets joke titles: "Scourge of Squirrels", "Lord of the Goldshire Dance Floor".
- **With consequences.** A slap is an event in the world: the `trusts` value of the NPC drops, and a `slapped` fact starts. The innkeeper then remembers it. His rumors get shorter, and the narrator brings it up. Hourglass keeps the joke consistent for weeks. Built: each slap costs 10 trust, down to -100. The tooltip of the NPC shows the slaps and the trust in words (3.5), and a slap is a big moment for the narrator (3.2).

**Picking the moments.** Code scores each moment, and the model picks only among the best ones. The model never sees the whole pile, so the choice is predictable, testable, and free.

| Part | Points | Example |
|---|---|---|
| First time | +5 | The first dance, the first critter, the first fall |
| Rare for you | +1 to +4 | The rarer in your own history, the more. The 50th rabbit scores low. |
| A streak | +1 per step, at most +5 | 12 squirrels in a row |
| Contrast | +1 to +4, by the level gap | A level 60 killed by a cow |
| A famous place | +3 | A dance in Goldshire, a jump off the Stormwind wall. The places are a table of data. |
| A callback | +4 | The moment touches your world: an NPC that you slapped before, or your nemesis. The world answers this. |
| An odd hour | +2 | 3 AM |
| Told before | -3 for each telling in the last chapters | The same kind of joke again |

The caps are code, not prompt text:

- **The chronicle:** the top 5 moments of a session go to the model. It picks at most 3 footnotes for the chapter.
- **The narrator:** at most one flavor line in 20 minutes, and only for a score of 8 or more.
- **A cooldown for each kind:** no two rabbit jokes in one evening.

The score uses whole numbers only, like Hourglass, so a test can state each rule exactly.

**The numbers of the code** (built):

- **Rare for you:** 1 or 2 earlier moments of the kind give +4, 3 to 9 give +3, 10 to 24 give +2, 25 to 49 give +1, and more give 0. A kind is the emote, the cause of a fall, a humbling death, or a book that you read. A dance is a dance, wherever it happens.
- **Contrast:** +1 for each 10 levels between you and the NPC that killed you, at most +4. A gap of 10 or more makes the death a flavor moment ("humbled"). A book gives +3: a hero who reads is odd enough, so the first book reaches the narrator. A book counts once, for all its pages.
- **A famous place:** Goldshire and the 6 capital cities, also in a subzone of one, such as the Trade District.
- **A callback:** the NPC holds `trusts` or `defeated` about you, or you hold `slapped` or `defeated` about it. A plain meeting is no history.
- **An odd hour:** 2 to 5 in the local time of the player, which the addon sends.
- **Told before:** each telling of the kind in the last 72 hours of game time.
- **Flavor lines of the narrator** (built): a batch with no big moment gives its best flavor moment to the narrator, when it scores 8 or more, no flavor line came in the last 20 minutes of game time, and its kind was not told in the last 12 hours. A footnote of the chronicle tells its kind, but it is no flavor line. Game time is the newest time from the addon, because an emote or a book adds no event to the world. The line counts as told when the call goes out, whatever the model answers. The budget of 3 lines an hour covers flavor lines too.
- **Footnotes of the chronicle** (built): the prompt of a chapter gets the 5 best flavor moments of a finished chapter, numbered and in plain words. The moments of a chapter run until the next chapter begins, because an emote adds no event to the world. The narrator answers in JSON with its saga and at most 3 footnotes, each with the number of its moment. A footnote with no listed moment, a second one for the same moment, or one that breaks the text rules (at most 200 characters) is dropped alone. Each footnote counts as a telling of its kind.
- **Streaks:** not yet. A streak needs the kills of common mobs (open question 9).
- **No votes.** Timeways asks the player for no rating of a joke. The scoring and the cooldowns decide alone.
- The moments and their tellings live in `c_<character id>.flavor.jsonl` next to the history (5.7).

**Joke titles** (built). A title is a rule over the flavor moments and the world. When a rule holds, the title lands in the world as a `title` fact, so it stays for good. It shows as a deed in the journal, and it is the best big moment of the narrator (3.2):

| Title | Rule |
|---|---|
| Lord of the Goldshire Dance Floor | 3 dances in Goldshire |
| Dance Machine | 25 dances anywhere |
| Friend of Gravity | 3 deaths to a fall |
| Student of the Deep | 3 deaths to drowning |
| Lava Enthusiast | 2 deaths to lava or fire |
| The Humbled | 1 death to an NPC 10 or more levels below you |
| Slap Happy | 5 slaps, of any NPCs |
| Bookworm | 10 books |

**Who scores.** A function of the Timeways story module scores the moments, in the bridge. No model takes part. The scoring is not part of Hourglass: Hourglass holds no words of any game, and the scoring is full of WoW (places, critters, emotes). The scoring asks Hourglass two questions:

- For a callback: is this entity in the world, and does it hold a fact about you? For example `slapped`.
- For "told before": did the history of the chronicle already tell this kind of moment in the last chapters?

The work divides in three:

1. **The addon notices.** It counts and sends the moments.
2. **The scoring code decides** which moments are worth a line.
3. **The model writes** the words, only for the moments that the code gave it.

An example. A cow kills you in Goldshire at 3 AM. You are level 60, and no critter killed you before. The score: first time +5, contrast +4, famous place +3, odd hour +2, so 14. The 30th rabbit of the same evening scores 1. At the end of the session, the code sorts the moments and gives the top 5 to the model. The chapter then says: "On the ninth night, a cow in Goldshire ended the career of our hero. It was not recorded as a battle."

Hourglass plans a generic salience ranking for its briefing. If that ranking takes weights from the caller, Timeways can move its weights into it later.

### 5.5 The transport

Timeways uses the transport of Gnomish Relay, with its own key and its own slots (5.12):

- **Out:** game events and questions go in strips signed with the Timeways key. The frame format and the records do not change: Timeways uses its own values in the chat, flags, and text fields. The size limit of a strip (3200 bytes) is enough for a batch of events.
- **Batches:** after the lines of each batch, the bridge sends `batch_end` with the message id. The story program answers `events_seen`, with a narrator line or `null` (3.2), and an optional `notice`: a line of Timeways itself, shown with the "Timeways:" prefix, such as a quest offer (3.4). It answers at once when the batch has no big moment. The bridge waits at most 60 s, so a slow story program never blocks the player.
- **No game event is lost when the desktop program is closed.** The transport takes every message that fits, and gives up on it after 270 s with no bridge. So the outbox keeps the events of a batch until its done reply. When the transport gives up, the events go back to the front of the outbox, and the player sees no error: they did not send them by hand. One batch of events is on its way at a time, so a closed desktop program costs few strips. A done reply sends the next batch at once. A question goes at once, and its error shows.
- **Every request gets one answer.** The bridge stops a story program that leaves a `lore_asked`, `talk_asked`, or `journal_asked` with no answer. So a request that fails gets an empty answer of its type: a lore answer with no passages, a talk with no words, or a journal with no pages. The error goes to stderr. The addon keeps the book that it shows when a journal has no pages.
- **Sizes.** A reply must fit two limits of the bridge: 24576 bytes of JSON, and 32 KB in the slot of the game after the Lua escape. In the slot, a `|` takes 2 bytes, and a quote, a backslash, and each byte outside ASCII take 4. The journal pages and the passages of an answer count both (`reply_size.rs`). A page past the end gets the last page.
- **Time.** The addon and the story program share the clock of one computer. The story program refuses an event more than one day after its own clock, because one such event freezes the world: Hourglass refuses every event older than its last one. An event before the last event counts as the time of the last event.
- **In:** story text comes back through the Timeways slots. A long text, such as a chronicle chapter, goes into a file of its own, like `Restore.lua` and `Live.lua`, with its own proved writer and size bound. A new file adds new statements to the proofs. It changes no approved statement.

### 5.6 The model

**Timeways uses the model that the player already has, and recommends it.** It installs nothing by itself.

1. **The recommended model: the agent that the player already uses**, for example Claude, Codex, or Gemini through Gnomish Relay. The writing is the best, and most players who have the relay already have one of these agents with a login.
2. **A local server that the player already runs**, such as Ollama (`localhost:11434`) or LM Studio (`localhost:1234`). Setup finds it, like it finds agents.
3. **An optional local model**, only on request. Setup offers it when it finds no model, and it shows the download size first (a small model is 2 to 5 GB). The trade-offs are clear before the player says yes:
   - On the graphics card, the model takes memory from WoW and can lower the frame rate.
   - On the processor, it does not hurt the game, but a small model writes about 10 to 20 words per second. That is fine for a narrator line, and slow for a chronicle chapter.
   - A small model invents more. The lore pack (5.10) and the checks (5.9) matter even more with it.

**With no model at all, the addon still works.** The features that need no model stay on: the chronicle as a list of the real events, the nemesis counts, the guild boss log, and the lore passages of the pack shown as they are, with their sources. A model makes them better, but it is not required.

**Other rules:**

- Lore answers need no web access: the passages come from the lore pack (5.10).
- **A budget** limits the use: a number of calls per hour, and a length per answer. The narrator and the chronicle use the fewest calls. A chapter of the chronicle costs at most 3 calls, and 1 when the window of the bridge is tight (3.3). The budget matters most for a subscription agent, because its calls count against the player's plan.
- A story call needs no coding tools. **The story program never starts a model itself.** It asks the bridge for a model call over the app protocol, and the bridge runs the model with no tools and returns only text (Gnomish Relay SPEC 9.7, decision 10):
  - Claude runs with `--tools ""`, no MCP servers, no user or project settings, in an empty temp folder, and behind the `PreToolUse` gate that denies every tool.
  - A local server is called through `curl` on `127.0.0.1` or `[::1]` only, with no redirects and no proxy. Its answer is hostile text, like an agent reply.
- **The bridge enforces the budget**, with the proved rate limiter of the relay (S14). A hostile addon that drives Timeways cannot spend the player's plan faster than that.
- Timeways runs no model server of its own. A paid model service for an addon is a gray zone of Blizzard's add-on policy, and a server costs money for every call.

### 5.7 Storage

- The history of each world is a file in the data folder of the story program: `worlds/r_<realm id>/c_<character id>.jsonl`. The prefixes keep a name such as "Con" or "Aux" from naming a Windows device. The bridge gives the folder as the second argument, `<data>/timeways/story/`, and the sandbox lets the story program write only there (5.12).
- Realm and character names come from the game, with spaces, apostrophes, and non-ASCII letters. They map to safe ids: ASCII letters and digits stay, and every other byte becomes `_` and two hex digits. So two names never share an id, and no id holds a `/`, a `.`, or a space.
- The file has one JSON line for each Hourglass event, and it only grows. The story program writes the new events after each game event, also after a refusal, because the events before a refusal landed.
- A failed write puts the file back to its last good length, and the next write tries the same events again.
- A failed `character_entered` leaves no character active, so the events of one character never land in the world of another.
- The state is not stored. `World::replay` builds it from the history when a character enters.
- **A crash in the middle of a write** leaves a broken last line. The replay stops at the first line that does not read or that has the wrong position, and cuts the file there. New events then follow the good part.
- **Whose world:** every batch from the addon starts with a `character_entered` line with the realm and the name. So the story program knows the world of each batch, also after it restarts. The addon holds its events until the login names the character.
- The sagas of the chronicle (3.3) are words, not facts, so they live in a file of their own next to the history: `c_<character id>.chronicle.jsonl`, one line for each chapter, keyed by the tick that began the chapter. The same rules hold for a broken last line and a failed write.
- Undo is cheap: cut the history and replay (`World::rewind`).

### 5.8 Sync between players

- The addon sends hidden addon messages to the party, the raid, or the guild with `C_ChatInfo.SendAddonMessage`, with the prefix `Timeways`.
- A message is at most 255 bytes, and the rate is limited. So a message carries one compact event, never story text. Each player writes the long text on their own computer.
- The server of the game names the sender of each addon message. A member trusts only the events of the keeper of a shared world.
- The history is append-only, so sync is simple: a member asks for the events after the last one it has, and replays them.

### 5.9 The lore cutoff: only WoW Forever

WoW Forever is set in the first year of World of Warcraft, **25 ADP**: after the Forsaken campaign of Warcraft III: Reforged, and **before Molten Core** ([warcraft.wiki.gg: World of Warcraft: Forever](https://warcraft.wiki.gg/wiki/World_of_Warcraft:_Forever)).

- Thrall is Warchief of the Horde, and Bolvar Fordragon is regent of Stormwind.
- The raid stories of vanilla have not happened, and they can happen differently or never: Ragnaros is alive under Blackrock, Onyxia is a secret, the Scarab Wall is closed, and there is no Naxxramas over the Plaguelands. A lore answer treats them as rumors or dangers of the present, never as history.
- Outland, Northrend, and all later expansions have not happened.
- **Forever has its own timeline.** Blizzard adds new storylines and content that differ from the original history: more than 1,000 new quests, 9 new dungeons, and the raids Barrow Deeps and Hyjal Summit. This content is canon, and no older source knows it.
- The cutoff moves as Forever releases content. The [Wowhead Forever roadmap](https://www.wowhead.com/forever/guide/content-release-roadmap) shows the plan.

Models know all of WoW's lore up to today, and they leak it. A line in the prompt is not enough, so four layers hold the cutoff:

1. **The game text is canon.** The addon collects the text of the Forever client itself: quest text, NPC gossip, books, and item text. This text is always exactly Forever's lore, also when Forever adds content of its own. It grows as you play, and it also feeds the spoiler limit.
2. **Sources with a cutoff.** The sources are, in order: the game text, the Forever pages of warcraft.wiki.gg and Wowhead, and Blizzard's Forever news. A Classic page counts only for events before Molten Core. A page about a later raid, a later patch, or a later expansion is refused.
3. **Canon is read-only.** The canon characters, places, and factions go into the world with their facts as of Forever, for example `leader_of` Thrall and the Horde. Only game events change them. The director can change only your own story: the NPCs of your rumors, and your quests. The story module refuses a proposal that touches a canon entity before `World::propose` sees it. So "Varian Wrynn returns" can never become true.
4. **A check on every answer.** Before an answer shows, the story module checks it against a list of names and events past the cutoff: for example the defeat of Ragnaros, the opening of the Scarab Wall, Naxxramas over the Plaguelands, Shattrath, the fall of the Lich King, the Cataclysm, and Pandaria. The check reads words in any case, and the last word of a name also counts at the start of a longer word: "Pandarian" counts as Pandaria. For `/lore`, a hit means one retry with the reason, and a second hit drops the answer. The retry holds the first answer and its reasons as fenced data, because the model wrote them. Only `/lore` retries: a narrator line, a chapter, a talk, or a quest offer that fails the checks gets no retry, and shows nothing. A narrator line, a chapter, and a talk can name what the player's own text of the hero names (3.7): the player wrote it first. The player's own text gets no check of the cutoff. Names that already exist in the lore of 25 ADP, such as Ragnaros, Arthas, Illidan, and Deathwing, stay allowed with their story up to that year only.

The list of later names is data in the repo, with a test for each entry. When Forever moves forward in the story, the cutoff moves with one change to that list and to the canon seed.

### 5.10 Lore data: the lore pack and the lore of each player

A web request for each question is slow, depends on one website, and sends whole pages into the prompt. So the lore is a **pack**: one file that the player builds once on their own computer. Each question is a local search. The project never ships Blizzard text or wiki text, only the list of pages.

**The sources of the pack:**

1. **The game files of the Forever build.** The client holds little text (checked on build 1.60.1.70009, 2026-09-26). The server sends the dialogue, the quest text, the books, and the NPC names, so only the text that the player saw (below) holds them.
   - `BroadcastText` has 12 rows, all from one cinematic. `Creature` holds only companion pets. `PageText` and `QuestObjective` are not in the client.
   - The useful tables: `AreaTable` (the zones, with their parent zones and continents), `AreaPOI`, `Map`, `TaxiNodes`, the descriptions of `Faction`, and the flavor text of `ItemSparse`.
   - No client table links a text to an NPC. A text links to a place only through an ID, or through the whole name of a zone in its words.
   - The client holds zones before they open, for example Mount Hyjal. So a list of zone phases, with a test for each entry, sets the cutoff.
2. **Forever-era wiki pages**, from the public database dump of Wowpedia, cut into short passages, each with its source. The text is CC BY-SA, so the pack names its sources and keeps that license.
   - **A dump, never a fetch.** The terms of wiki.gg forbid crawling and scraping, and `robots.txt` blocks `/api.php`. The build reads a local dump file.
   - **The infoboxes give the links.** The raw wikitext of a dump holds each infobox call, for example `{{Npcbox}}` with its location. The Cargo tables of the wiki hold only a few of these fields.

**The pack:**

- One SQLite file with a full-text index (FTS5). The expected size is a few tens of MB.
- A passage has: its text, its source, its phase tag, and links to the zones, NPCs, quests, and items that it is about.
- The player builds it from the dump (below), and builds it again when Forever releases a new phase.
- **The cutoff is built in.** The pack holds only the passages up to the current phase of Forever. A Molten Core passage is not in the file before Molten Core opens, so no model can see it. Layers 3 and 4 of 5.9 still apply to the text of the model.

**The builder** (built): `timeways-pack` writes the pack. It refuses a passage with no link, and it never writes over a pack that exists.

- **From a dump:** `timeways-pack from-dump <dump> <pack>` reads the MediaWiki XML export of the wiki, as a `.7z` archive or unpacked. It streams the file and keeps only the listed pages. It reads the dump at most twice: once for the index page, the wiki pages, and every redirect, and once for the books and the targets of redirects.
- **The list is data:** `crates/story/data/pack_sources.toml` holds the pages, and the repo holds no lore text.
  - The index page "History of Warcraft" and its chapters I to V. Each `* [[Page]]` line of a chapter is a book. The builder takes the `content=` argument of the `{{Book}}` call of the page, and no other argument. A template with a longer name, such as `{{Bookshelf}}`, is no book. A copy from a website, with "(site)" in its title, comes only when the page has no other copy. Each book passage is common.
  - Wiki pages, each with its kept sections, and its places, its NPCs, or `common`. A page with none of them is refused when the list is read, before the dump.
  - Later terms: regular expressions for the names of later expansions and of their people and places. A paragraph of a wiki page that matches one goes out.
- **A title** gets an upper case first letter, as in MediaWiki: `[[night elf]]` is the page "Night elf".
- **A redirect** is followed one step. Two titles that lead to one book give its passages once.
- **Plain text:** references, comments, HTML tags, templates, tables, pictures, and bold and italic marks go. A link keeps its label. Broken markup leaves no marks.
- **A passage** is one line of plain text with at least 80 characters. A list line, a table line, or an indented line is no passage. The source is `the book "<title>"` or `the wiki page "<title>"`.
- **The limits of the bridge:** a passage has at most 4096 bytes of text, and a source of at most 512 bytes with no control character. A longer paragraph becomes several passages, each cut after a sentence. A line of passages past a limit is refused with its number, and no pack is written.
- **The report** gives the number of passages of each page, and names each missing chapter and each missing page. A missing page is skipped. A dump without the index page, or with broken XML, is an error, and no pack is written.
- **The same dump gives the same pack**, in the order of the list.
- **From lines:** `timeways-pack <passages.jsonl> <pack>` reads passages as JSON lines, each with its text, source, places, NPCs, and `common`. It is for tests and for passages by hand.

**Common knowledge** (built): a passage marked `common` passes the spoiler limit with no visit. It holds what everyone knows in 25 ADP, such as the History of Warcraft books of the game. A common passage with a place or an NPC still waits for them.

**A question:**

1. The story module searches the pack for the question and the context (zone, target, quest).
2. It keeps only passages whose links are in the world of the player: a zone that they visited, an NPC that they met, a quest that they did. This is the spoiler limit.
3. The best 5 to 10 passages go into the prompt with their sources. A passage past a limit of the bridge, from an old pack or from a seen text that grew, is cut to its first piece or left out, so every answer reaches the game.
4. The model answers only from them, and names the sources.

**The lore of each player**, in the data folder of the bridge:

- **The world:** the Hourglass history, one append-only file per character (5.7).
- **The text that the player saw** (built): the addon sends the game text of each quest, gossip window, and book as the player reads it. It covers the text that the server sends and the client files do not hold.
  - The text goes on one line: each run of control characters, such as the line breaks of a quest, becomes one space. The bridge drops a line of the addon with a control character.
  - The name of your character becomes `$N` as a whole word, in any case of ASCII and Latin-1 letters. A character past ASCII, such as the quote marks of "«Ada»", counts as no letter, so a name next to it becomes a mark too.
  - The name of the character becomes `$N` in the addon, so no model sees it (5.11). A letter that a player wrote has a creator, and never goes out.
  - A file for each character keeps each text once (`c_<name>.learned.jsonl`, which also keeps the rumors (3.1.1)). An index in memory searches it, and is built again at each start.
  - The player read the text, so it passes the spoiler limit. A search uses this text first, and the pack fills the rest (3.1.1).
  - The addon cuts a text at 2000 bytes.
- **Not in the saved variables.** Any addon can read the saved variables of another addon, so they hold only window state. Player tasks are the one exception (4.7): they live between two players, not in a world.

### 5.11 Player names: the alias table

The name of a real player never goes to a model, local or cloud. The model does not need it. It needs the role of the person and what happened.

1. **The alias table.** A local table of the bridge maps each player to an ID: `Grimtusk-Stormrage` becomes `P7`. An ID is never reused, so a player keeps the same ID in every chapter. The table never leaves the computer.
2. **The card.** The prompt gives the model the ID with a card: `{P7}: Undead Rogue, level 60, your nemesis, killed you 7 times, you won once, last seen in Stranglethorn.` The card comes from the world.
3. **The model writes the ID** in braces: "Once again, `{P7}` came from the shadows."
4. **The check.** Each `{P…}` in the text of the model must be in the table. An unknown ID means one retry. The model cannot invent a player.
5. **The swap back**, before the text shows, depends on who reads it:
   - On your own screen, the real name. You saw it in the game.
   - In anything shared with the guild, the real name only for a member who has the addon and allows it. Every other player shows as their card: "the Undead Rogue", "a brave Dwarf Priest".
   - A member can turn on "keep me out of the saga", and then shows as their card too.

**In Hourglass**, each player is a `Person` entity with the ID as its name. So the history holds `P7`, never the real name. When a guild world syncs its history between members, no real names travel with it. Each member swaps the IDs with their own table.

Canon NPCs, such as Thrall or the innkeeper of Goldshire, keep their real names. They are part of the lore, not people.

**Your own words go as you typed them.** A `/lore` question or `/talk` words reach the model as the player wrote them. If you type the name of another player there, it goes with them: the choice is yours, like a message that you send yourself (rule 5). A swap of known player names for aliases in typed text comes with the alias table.

**Pets count as players.** A player chose the name of a hunter pet, so the addon treats every unit that a player controls (`UnitPlayerControlled`) as a player: it never sends its name as a target, a foe, or a killer.

**Names have a size limit.** The story program refuses a name that is empty, longer than 96 bytes, or holds a control character. No game name comes close, and the limit keeps each page of the journal inside one reply.

### 5.12 Two addons, one desktop program

Timeways and Gnomish Relay are two separate addons, each with its own listing on CurseForge and Wago. Each works alone. One desktop program, the Gnomish Relay bridge, serves both, because only one program can own the Screenshots folder and the slot files (Gnomish Relay SPEC 8.4).

**A key for each addon.**

**The relay side of this section is Gnomish Relay SPEC 9.7.** It is the approved plan, and it wins where the two differ.

- The relay key stays `strip.key`. Setup makes `timeways.key` next to it, in the config folder of the bridge, with mode 0600, and writes the key into `Timeways_Key`, a key addon of its own outside the Timeways folder (relay SPEC 7.3.2). `KeyHandoff.lua` takes the key from it, so a CurseForge update keeps the key. The classifier of the relay denies both keys to every agent. The bridge refuses to start if the two keys are the same.
- The bridge checks the tag of each strip under both keys. One key verifies: that app. None: refused. Both: refused as ambiguous. A new statement, S29, proves this choice. A frame in the saved variables of one app counts only under that app's key.
- **Proved statements change.** Each app sets its own Lua globals in its slot files (`Timeways_SlotData` and more), so one app never overwrites what the other is about to read. S9, S18, and S20 of the relay are restated over the app (approved).
- The Timeways lane parses only the transport flags. A coding flag such as `perm=` or `level=` in a Timeways record does nothing, and a non-empty `cwd` is refused.
- **What the key split protects.** It stops a bug or a hacked story program from reaching the agents. It does not stop a hostile addon that loads first from reading either key.
- **A strip signed with the Timeways key reaches only the story program**, never a coding agent. The bridge enforces this: the story route has no access to the agents. So a Timeways bug, a hacked Timeways update, or a hostile addon that drives Timeways gets only story powers: the model budget, false game facts in the world, and fake story text. It gets no path to commands.
- A player with only Timeways has no coding config: setup asks no folder question and sets up no coding agent. The config has only a `[story]` section for the model.

**The copies in Timeways** (built): `addon/Timeways` holds `Sha256.lua`, `Codec.lua`, `Saved.lua`, `Health.lua`, `Strip.lua`, `Slots.lua`, `Messages.lua`, and `KeyHandoff.lua` from the relay, and CI compares each one with the pinned relay commit. `Link.lua` is the seam: it sends each batch in the one chat `story`, gives each done reply to the JSON handlers, and shows each error reply as plain text.

**No public send function.** Each addon carries its own private copy of the Lua transport: `Codec.lua`, `Sha256.lua`, `Strip.lua`, and the slot poll. A shared library addon is refused: its key would pass through a global function, and a hostile addon could hook it. One source folder of the transport, with its tests, lives in the Gnomish Relay repo. The packaging of each addon copies it, with a version pin.

**Slots for each addon.** Timeways gets its own set: `Timeways_S0001` to `Timeways_S1000`, with `## Dependencies: Timeways` and `## Group: Timeways`, so the AddOns list folds them under Timeways. The two addons never use up each other's slot loads.

**One strip at a time.** Both addons draw their strip in the same corner, so they take turns. A shared global busy value holds the time when the current strip ends. Each addon checks and sets it in one handler, and WoW Lua runs on one thread. A hostile addon can hold the value to block strips, which it can do today anyway. After 30 s of "busy", each addon shows "Screenshots blocked by another addon".

**The bridge keeps each app apart:** a replay store, a state file, a rate limit, a slot window, a saved-variables file to watch, a reload inbox, tokens, and restore for each app. Timeways has no restore bundle: its state lives on the desktop, and the addon rebuilds from there.

**The story program.** The bridge starts `timeways-story` when the Timeways key exists, the same way it starts an ACP agent. They talk over stdin and stdout, with JSON lines:

- The bridge sends the decoded Timeways records.
- The story program sends back the story text. The bridge writes every file that the game reads, with the proved writers.
- The story program reads hostile text, so it runs in the sandbox of the relay (SPEC 6.6.4): it writes only `<data>/timeways/`, has no network, and cannot read the protected paths. On Windows there is no sandbox yet: Timeways runs, with a one-time warning.
- The bridge starts it from a path in the config, with no shell and a short list of environment variables. `restart` and `update` restart it too, and a crash restarts it after a delay.
- The bridge writes no file into the Timeways addon folder. It writes the key addon `Timeways_Key` and the slots next to it, and deletes an old `Key.lua` in the Timeways folder.

This keeps the releases apart: Timeways ships `timeways-story` on its own schedule, and Gnomish Relay does not depend on Hourglass. Timeways tests the story program alone, with a fake bridge.

**What gets extracted:**

1. **Hourglass**: done on 2026-09-24, into the public repo `rusty-hourglass`. Timeways pins one commit as a git dependency. A crates.io release comes when the API is stable.
2. **The Lua transport**, as one source folder in the Gnomish Relay repo, copied into each addon.
3. **No Rust from the bridge.** Gnomish Relay gets a small "app protocol" for the story program in its SPEC section 9, next to ACP.

**Versions.** Each addon sends its version in the hello, and in every strip (`ver=`). Timeways is version 1 (`ns.App.version`), and the bridge takes 1 to 1. The bridge keeps a supported range for each app. A version out of range gets one reply: "Timeways: update the addon", or "update the desktop program".

### 5.13 Deaths and resets

In WoW, the dead come back. A mob respawns, a rare returns, and a raid boss is back after the weekly reset. So in Timeways, a kill is a deed of the killer. It does not change the target.

**The rules:**

- A kill adds to `defeated`, from the killer to the target. `defeated` only rises and never ends, so the first kill stays in the history for good.
- A canon NPC never gets `dead`. The game is the truth (rule 4), and the game brings the NPC back.
- Only an NPC of your own story gets `dead`, for example the bandit leader of a rumor. Nothing in the game brings it back, so it stays dead.
- Your own deaths are deeds too. A mob that kills you adds to `defeated` from the mob to you. Between two players, `nemesis` holds the count (4.1), not `defeated`.

**Tiers.** The weight in the story matches the target:

| Target | What the story keeps |
|---|---|
| A common mob | A count only, for flavor: "23 Defias in Westfall" (5.4.1). No entity in the world. |
| A rare or a quest boss | `defeated`. When it comes back, the story treats it as a rival: "Hogger again. He does not learn." |
| A dungeon or raid boss | `defeated`. The first kill is legend, and each later kill is an echo (below). |
| An NPC of your own story | `dead`. It stays dead. |
| You | `defeated` from your killer, and trips to the spirit healer for the narrator to joke about. |

**Echoes.** The Bronze Dragonflight guards the timeways, and the name of the addon comes from them. A reset is an echo in the timeways:

- **The first kill is the true kill.** The chronicle tells it as legend: "On the ninth night, Ragnaros fell."
- **After a reset, the boss is an echo.** The world forgets the kill, but your timeway remembers it.
- **A later kill is about mastery,** not death: fewer wipes, a faster kill, a new player at the front.

The guild world keeps `defeated` from the guild to each boss. So the saga gets an arc for each boss: "Week 1: 14 wipes. Week 6: Ragnaros fell before the tank's flask ran out."

**The lore cutoff.** A kill that you did is a deed of your story, not a lore claim. So the check of layer 4 (5.9) accepts "you defeated Ragnaros" when the world holds `defeated` from you or your guild to Ragnaros. A lore answer still treats Ragnaros as alive, because canon did not change. Both statements are true.

**The strength of the echo lore is open** (9.8). The echo idea is a setting of the player, not a fixed voice. The possible levels are:

- **Off:** kills are counts and deeds, with no echo text.
- **Light:** the narrator and the chronicle mention echoes now and then.
- **Strong:** a bronze dragon voice tells each reset. This voice is an invented character next to canon characters such as Anachronos.

## 6. Build order

1. **Lore on demand** (3.1): the world of a character, the spoiler limit, the lore cutoff (5.9), a model, and one window.
2. **The narrator** (3.2).
3. **The chronicle** (3.3).
4. **Personal side quests** (3.4), and **talk to an NPC** (3.5).
5. **Nemesis** (4.1): the first social feature. It needs no sync, because the feud lives in your own world.
6. **The guild saga, the herald, and the bounty board** (4.2 to 4.4): sync, and the keeper.
7. **The shared guild world** (4.5), and **a shared narrator** (4.6).

## 7. Risks

| Risk | What we do |
|---|---|
| The story gets old | The narrator talks little. Quests change the world for real. The chronicle shows your choices. |
| Cost to the player | The player's own model, with a budget for every feature. The features that need no model stay on without one. |
| Invented lore | Answers come from cited wiki pages, and "legend says" marks the rest. Hourglass refuses changes that break the history. |
| Lore from later expansions | The four layers of 5.9: game text as canon, sources with a cutoff, read-only canon, and a check on every answer. |
| Few players have the setup | Timeways is its own addon, installed for the story. The one-line install of Gnomish Relay sets up the desktop part. The addon works with no model, and gets better with one. |
| Part of the community dislikes AI content | The content is private to your screen, and nothing reaches other players without your click. |
| Blizzard's rules | Flavor only: no automation and no advantage. |

## 8. Name

"Timeways" had no match on CurseForge, Wago, or WoWInterface in a search on 2026-09-24. "Hourglass" is taken by an old cooldown addon, and "Chronicle", "Chronicles", and "Loremaster" are taken or crowded. Check the name again before a release.

## 9. Open questions

1. **Hourglass stability.** Its spec says that no consumer calls it yet. Timeways pins one commit until the API is stable.
2. **Decided: model calls go through the bridge, with no tools** (5.6, and Gnomish Relay SPEC 9.7). Was: a plain model backend for Gnomish Relay. A story call needs no coding tools. Add a `model` kind next to `acp` and `echo`, for Ollama and LM Studio? How does a coding agent run with no tools for a story call?
3. **Decided: the wiki dump, built on the player's computer** (2026-09-30). Wowpedia publishes a database dump (`https://s3.amazonaws.com/wikia_xml_dumps/w/wo/wowpedia_pages_current.xml.7z`). Setup downloads it and runs `timeways-pack from-dump`, so the pack exists only on the player's computer. The project ships only the list of pages (`crates/story/data/pack_sources.toml`), never wiki text or Blizzard text. Many of the chosen pages copy the in-game History of Warcraft books word for word, and Blizzard owns that text, so it never goes into a download of ours.
   - The dump changes over time, so nothing pins its checksum. The same dump always gives the same pack.
   - The tests of the lore code use invented passages only.
   - Later: short summaries in our own words, which the project can ship.
4. **The game files** (5.10). The client holds zones and a little text, and no dialogue. Still open:
   - The tool. The Forever build is on product `wow_classic_beta` now, and its code at launch is unknown. The files come from the public CDN (TACTTool with DBC2CSV, and the WoWDBDefs layouts), or from wago.tools after its owners allow it.
   - The license. The Blizzard EULA forbids data mining. The project ships no text from the game files. Decide before a release that ships names or links from them.
   - Does the server send gossip as `BroadcastText` rows into the cache of the client? A test in the game settles it.
5. **The API of the Forever client.** Check each event in 5.4 with the API gate.
6. **The canon seed.** Which canon characters, places, and factions go into every world at the start, and with which facts? The Forever client data (for example its database tables for the Forever build) is the best source.
7. **Decided: two addons** (5.12). Still open: do 2000 slot folders make the game start slower, and does a `## Group` start folded in the AddOns list? Measure both in the game.
8. **The strength of the echo lore** (5.13). Off, light, or strong, and which level is the default? Does a strong level need a named bronze dragon, and how does it stay inside the lore cutoff?
9. **The closed combat log** (5.4). Addons in this client cannot read the combat log. Kills of rares and bosses and your deaths work without it. These features still need a source:
   - Nemesis (4.1): the death recap names the killer, but its documentation lists no GUID, so the addon cannot tell a player from an NPC with the same name for sure. The addon reads `sourceGUID` when the recap gives it. If the recap gives none, no death names a killer.
   - Critter kills and common mob counts (5.4.1): `UNIT_DIED` gives a GUID, but a GUID can be secret, and the range of the event is not documented. A kill step of a task (3.4) counts only the units of its creature that the addon saw, from `PARTY_KILL`.
   - Wipes and the first player to die in a raid (4.2).
   - A test in the game settles what `UNIT_DIED`, `PARTY_KILL`, and the recap really give, and when values are secret.
