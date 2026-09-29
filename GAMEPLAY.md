# Timeways

**Azeroth remembers you.**

Timeways is a World of Warcraft: Forever addon. It adds a story layer to the game: lore on demand, a narrator that remembers you, a chronicle of your adventure, personal side quests, and, with a guild, a shared saga and world PvP feuds.

An AI model writes the words. A rules engine decides what is true. The game itself supplies the facts.

Status: early build. The story program keeps the world of each character in a file, and serves `/lore`, `/talk`, the journal, the narrator, and the bard. The addon talks to it through the shared transport of Gnomish Relay. Nobody has run the parts together in the game yet, and no real lore pack exists yet.

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
- **A voice in the world.** The answer comes from a local historian or a bard.
- **A follow-up question** continues the same conversation.

This slice tests the whole chain with one question and one answer: the addon, the relay, the world, the spoiler limit, and the agent.

#### 3.1.1 Emergent lore, on a seeded floor

Most lore comes from play. You learn what your character read or heard: quests, gossip, and books (5.10). Two players know different things, and what you know is part of your story.

- **What you read comes first.** A question uses your own text first. The lore pack only fills the gaps. (Built)
- **The answer says where you learned it:** "You read in *The Kingdom of Stormwind* that…". (Built)
- **A Learned page** in the journal lists what you read and heard (3.6).
- **The narrator notices** a first book about a place, or a first story from an NPC, as a small moment.
- **Only game text is canon.** The words of an NPC in `/talk` come from a model. They go into your journal as a rumor, never as a fact, and `/lore` never cites them.

**The seed is the floor.** Some lore is common knowledge in 25 ADP: the kingdoms, the factions, the big names, and the zones. The seed holds it, so `/lore` is not empty on the first day. It stays small and general. The deep lore of the pack stays behind the spoiler limit (3.1), and opens as you visit places and meet people. The zones come from the game files (5.10), and the rest is a short list that a person checks (open question 6).

### 3.2 The narrator

No invented companion rides along. A narrator tells the big moments as they happen, in one short line in the voice of the chronicle, about "our hero". The chronicle speaks after a session. The narrator is the only voice while you play.

- It reacts to big moments. The story program finds them in the events that each batch adds to the world:
  - a joke title (5.4.1),
  - the first kill of a rare or a boss (the echo of a later kill is not a big moment),
  - a second or later death to the same NPC: "Third time this murloc got our hero,"
  - a slap of an NPC, which it remembers (5.4.1),
  - a level up,
  - the first visit of a zone.
- It speaks at most once for each batch, about the best moment, in the order above. Of two moments of one kind, the later one wins, because it holds the newer count.
- A batch that ends with a question gets no `batch_end` (5.5), so its moments wait for the next batch.
- It has a budget: at most 3 lines in one hour of game time. The budget lives in memory, so a restart of the story program starts it again. That costs at most 3 more lines once.
- A model writes each line through the bridge, with no tools. The line must be plain text, at most 300 characters, and hold no name from after the cutoff (5.9). A line that breaks a rule gets no retry: the narrator stays silent.
- It remembers your history across sessions, because the world does.
- It speaks in the chat window now. A window of its own, and a voice (Gnomish Relay SPEC 13.3), come later.

### 3.3 The chronicle

After each play session, the agent writes the session as a short saga in the voice of a bard.

- It uses the real events of the session: the zones, the bosses, the deaths, the loot, and the quests.
- You read it in the game as a book, one chapter per session.
- The history of the world is the source, so the chronicle never contradicts itself.
- **The bard** (built): after a batch, the story program asks a model for the saga, and the footnotes (5.4.1), of the oldest finished chapter that has none yet, one chapter at a time. The last chapter can still grow, so it waits for the next session. The facts of the prompt come from the chapter alone. The saga must be plain text in one paragraph, at most 600 characters, with no name from after the cutoff (5.9). A saga that fails keeps the plain list, and gets no second call in the same run.
- **Without a model** (built): a chapter lists what was new in the session: the zones, the people, and the deeds. A session ends after 30 minutes with no event. A session with nothing new gets no chapter. Each list of a chapter keeps at most 20 entries, so a chapter always fits on one page of the journal (5.5).

### 3.4 Personal side quests

An innkeeper tells you a rumor, and the rumor becomes a small quest line made for you.

- Each step uses a real game goal that the addon can check from game events: kill 8 of a mob, visit a place, collect an item, talk to an NPC. The format follows [PlayerMadeQuests](https://github.com/runeberry/PlayerMadeQuests), which already tracks these goals.
- **No overlap with the quests of the game.** A side quest never repeats, continues, or changes a quest of the game: one that exists now, or one that comes in a later phase. It never sends you to the goal of a game quest, and it never claims a deed of one. It lives only in your own story.
- A quest is plain text. You read it before you accept it.
- The rewards are story: a title in your journal, a line in your chronicle, and an NPC who trusts you more and tells you more lore later.
- Hourglass keeps the quests consistent. A quest cannot send you to an NPC who died in your story, or to a place that you never heard of.

**The first slice.** The combat log is closed and no loot event comes yet, so a step has one of two goals: `visit` a place, or `meet` an NPC.

- **The offer** (built): you target an NPC that you met and type `/quest`. The event `quest_asked` has no reply of its own. At the end of the batch, the model call of the quest takes the place of the narrator call. The offer comes back as the narrator line, so the relay needs no change. Asking is meeting, as a talk is. With no model, or with an offer that breaks a rule, the line says that the NPC has no task for you now.
- **The check** (built, `quest.rs`): the code refuses an offer that breaks one of these rules.
  - The answer is JSON with a title (at most 60 characters), a text (at most 400), and 1 to 3 steps. The offer line fits in one narrator line (1000 bytes).
  - Each name is a string of the game, copied exactly, because progress matches it byte for byte.
  - A place is a zone or subzone that you visited, or a zone that a text that you read names.
  - An NPC is one that you met, is not dead in your story, and is not the giver.
  - **No overlap:** a subzone or NPC of a step is not in the title or the text of a game quest that you read. The title is not the title of such a quest. A zone is exempt, because most quest texts name their zone. The rule covers only the quests that you read.
- **Limits** (built): at most one offer waits, and a new offer ends the old one. A giver with an open quest waits for you to finish it. You hold at most 3 open quests.
- **Accept and progress** (built): `/quest accept`, the buttons of the Quests page, or `/quest decline` answers the offer that waits. The story program checks each step against later `zone_entered`, `npc_met`, `talk_asked`, and `npc_slapped` events, in order. A step that holds when it becomes the next step is done at once, because the addon sends a zone only when it changes. The addon also sends each NPC once in a session, so it forgets the NPCs that you met when you accept a quest. At the end, you get `quest_done`, and the giver trusts you 10 more. The model never picks this number.
- **Storage** (built): the quest file `c_<name>.quests.jsonl` holds the offers, the answers, and the steps done. The world holds the facts: the giver holds `quest_offered`, and you hold `quest_accepted` and `quest_done`. The quest thing is named "quest <number>: <title>", so it never merges with a title.

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

### 3.6 The journal

A book in the game, in the look of the classic quest frame: the quest dialog art, the parchment, the book icon of the quest log, and dark brown ink. `/journal` or `/timeways` opens it.

The desktop sends the pages each time the book opens, because the world lives there (5.10). No model takes part in a page, and the sagas of the bard are stored words. Each page comes from the facts of the world, its history, and the files next to it (5.7).

| Section | What it holds | State |
|---|---|---|
| **Hero** | Your sheet and your own lore (3.7), each field with an Edit button, and your entries with Add and Remove. The first time that the book shows an empty hero in a session, it opens here. | Built |
| **Chronicle** | One chapter for each play session (3.3), with the saga and the footnotes of the bard when a model wrote them. The book opens on it. | Built |
| **Places** | Each zone, with the date of the first visit, and its subzones under it | Built |
| **People** | Each NPC that you met, with the place, the date, your slaps, and its trust in you | Built |
| **Deeds** | Level milestones, first kills of rares and bosses, repeat kills (echoes, 5.13), your deaths, and your joke titles (5.4.1) | Built |
| **Learned** | What you read and heard (3.1.1): each book, each quest tale, and each story of an NPC, with the place and the date. A rumor from `/talk` shows as a rumor. | Built |
| **Nemesis** | Real players from world PvP only: the kill count on each side, the places, and the last time seen (4.1). Aliases only (5.11). | Later |
| **Quests** | The personal side quests (3.4): each offer with Accept and Decline, then each quest with its steps and its state | Built |

### 3.7 The hero

Who your hero is, in your own words, as a player of a tabletop game writes before the first session. It is your hero's own story, never canon: `/lore` never reads it.

- **The sheet:** origin, background, goal, bond, flaw, and traits. Each field is optional, and holds at most 300 characters. You change a field at any time, and an empty text clears it.
- **Your own lore:** entries that you add at any time, for example "A stranger at the inn knew my father's name." Each entry keeps its time and the place where you stood, and the NPC that you targeted when it is about one. You can remove your own entry.
- **Nothing is lost.** Each change is a new line in `c_<character id>.hero.jsonl` next to the history (5.7). A removal and an old text of a field stay in the file.
- **The same rules as other text:** no control character, and no name from after the lore cutoff (5.9). An edit has no reply of its own, so a refused text leaves its reason for the next journal page, and the addon shows it once.
- **Who reads it:**
  - The narrator (3.2) and the bard (3.3) get the sheet and the 5 newest entries, under the heading "the hero's own story, not canon".
  - The bard also gets the entries written during its chapter.
  - An NPC in `/talk` (3.5) gets only the entries about it or about its place, at most 5.
- **The journal** carries the sheet on its first page, and the entries as a list like the others.
- **In the game:** the Hero page of the book, with the dialog of the game for each text, or `/hero`, `/hero add <text>`, `/hero note <text about your target>`, and `/hero set <field> <text>`. Each edit goes out with a journal request, so the page shows the result, or the reason for a refusal, at once.

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
- A bard writes each raid night as a saga, and every member with the addon reads the same book.

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
| `trusts` | number, -100 to 100 | free | person to person | How much an NPC trusts you. |
| `visited` | flag | up | person to place | You were in this place. It feeds the spoiler limit. |
| `knows_lore` | flag | up | person to thing or place | You heard this piece of lore. It feeds the spoiler limit. |
| `dead` | flag | up | none | Only for an NPC of your own story, never for a canon NPC (5.13). An `up` flag never ends, so the dead stay dead. |
| `defeated` | number, 0 to 1000 | up | person or faction to person | How often the killer killed the target. A kill is a deed of the killer. The target stays alive (5.13). |
| `nemesis` | number, 0 to 1000 | free | person to person | The kill count of a feud. Each side has its own value: `nemesis` on `P7` linked to you counts the kills of `P7`, and `nemesis` on you linked to `P7` counts yours. |
| `quest_offered`, `quest_accepted`, `quest_done` | flag | up | person to thing | A personal quest and its state. |
| `level` | number, 1 to 60 | up | none | Your level. It only rises. |
| `slapped` | number, 0 to 1000 | up | person to person | How often you slapped an NPC. It never ends. |
| `title` | flag | up | person to thing | A joke title of your journal, such as "Scourge of Squirrels". |
| `member_of` | flag | free | person to faction | Guild membership, and faction ties. |
| `leader_of` | flag | free | person to faction | A canon leader, for example Thrall and the Horde. Only the canon seed and game events change it (5.9). |

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
| Quest done | `QUEST_TURNED_IN` |
| Kill of a rare or a boss | `PARTY_KILL` for a unit that the addon saw as rare, rare elite, or world boss (`PLAYER_TARGET_CHANGED`, `UPDATE_MOUSEOVER_UNIT`, `NAME_PLATE_UNIT_ADDED`), and `ENCOUNTER_END` with `success` 1 |
| Your death | `PLAYER_DEAD`, and the killing blow from `C_DeathRecap.GetRecapEvents()` |
| Boss fight | `ENCOUNTER_START`, `ENCOUNTER_END` |
| Talk to an NPC | `GOSSIP_SHOW`, `QUEST_GREETING`, `QUEST_DETAIL`, `QUEST_PROGRESS`, `QUEST_COMPLETE` |
| The text that you read | The same events, and `ITEM_TEXT_READY` for a book (5.10) |
| Loot | `CHAT_MSG_LOOT` |
| Group and guild | `GROUP_ROSTER_UPDATE`, `GUILD_ROSTER_UPDATE` |

The addon sends game events in batches with the next strip. Nothing needs to arrive at once.

**The combat log is closed to addons in this client.** `COMBAT_LOG_EVENT_UNFILTERED` fires, but only Blizzard code can read its payload (`C_CombatLogSecure` is secure-only). So the addon reads kills and deaths from the events above:

- It keeps, in memory only, the GUID of each rare, rare elite, and world boss that it sees. `PARTY_KILL` gives the GUID of the target of a killing blow of you or your group.
- A raid boss gives both `PARTY_KILL` and `ENCOUNTER_END`, so one name counts once in 2 minutes.
- The client can hide a value from addons ("secret values"). The addon checks each GUID and name with `issecretvalue`, and never compares or stores a hidden one.
- A death names its killer only when the addon saw that name on an NPC and never on a player. So the name of a real player never leaves the computer (5.11).

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
- **Used rarely.** The narrator picks one now and then, with a cooldown: "The fourth rabbit today. The rabbits of Elwynn begin to talk." The chronicle gets footnotes: "On the fourth day, our hero danced in Goldshire. Nobody knows why." The journal gets joke titles: "Scourge of Squirrels", "Lord of the Goldshire Dance Floor".
- **With consequences.** A slap is an event in the world: the `trusts` value of the NPC drops, and a `slapped` fact starts. The innkeeper then remembers it. His rumors get shorter, and the narrator brings it up. Hourglass keeps the joke consistent for weeks. Built: each slap costs 10 trust, down to -100. The People page of the journal shows the slaps and the trust in words, and a slap is a big moment for the narrator (3.2).

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

- **Rare for you:** 1 or 2 earlier moments of the kind give +4, 3 to 9 give +3, 10 to 24 give +2, 25 to 49 give +1, and more give 0. A kind is the emote, the cause of a fall, or a humbling death. A dance is a dance, wherever it happens.
- **Contrast:** +1 for each 10 levels between you and the NPC that killed you, at most +4. A gap of 10 or more makes the death a flavor moment ("humbled").
- **A famous place:** Goldshire and the 6 capital cities.
- **A callback:** the NPC holds `trusts` or `defeated` about you, or you hold `slapped` or `defeated` about it. A plain meeting is no history.
- **An odd hour:** 2 to 5 in the local time of the player, which the addon sends.
- **Told before:** each telling of the kind in the last 72 hours of game time.
- **Flavor lines of the narrator** (built): a batch with no big moment gives its best flavor moment to the narrator, when it scores 8 or more, no flavor line came in the last 20 minutes of game time, and its kind was not told in the last 12 hours. The line counts as told when the call goes out, whatever the model answers. The budget of 3 lines an hour covers flavor lines too.
- **Footnotes of the chronicle** (built): the bard gets the 5 best flavor moments of a finished chapter, numbered and in plain words. The moments of a chapter run until the next chapter begins, because an emote adds no event to the world. The bard answers in JSON with its saga and at most 3 footnotes, each with the number of its moment. A footnote with no listed moment, a second one for the same moment, or one that breaks the text rules (at most 200 characters) is dropped alone. Each footnote counts as a telling of its kind.
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

**Who scores.** A function of the Timeways story module scores the moments, in the bridge. No model takes part. The scoring is not part of Hourglass: Hourglass holds no words of any game, and the scoring is full of WoW (places, critters, emotes). The scoring asks Hourglass two questions:

- For a callback: is this entity in the world, and does it hold a fact about you? For example `slapped`.
- For "told before": did the history of the chronicle already tell this kind of moment in the last chapters?

The work divides in three:

1. **The addon notices.** It counts and sends the moments.
2. **The scoring code decides** which moments are worth a line.
3. **The model writes** the words, only for the moments that the code gave it.

An example. A cow kills you in Goldshire at 3 AM. You are level 60, and no critter killed you before. The score: first time +5, contrast +4, famous place +3, odd hour +2, so 14. The 30th rabbit of the same evening scores 1. At the end of the session, the code sorts the moments and gives the top 5 to the model. The chapter then says: "On the ninth night, a cow in Goldshire ended the career of our hero. The bards do not sing of it."

Hourglass plans a generic salience ranking for its briefing. If that ranking takes weights from the caller, Timeways can move its weights into it later.

### 5.5 The transport

Timeways uses the transport of Gnomish Relay, with its own key and its own slots (5.12):

- **Out:** game events and questions go in strips signed with the Timeways key. The frame format and the records do not change: Timeways uses its own values in the chat, flags, and text fields. The size limit of a strip (3200 bytes) is enough for a batch of events.
- **Batches:** after the lines of each batch, the bridge sends `batch_end` with the message id. The story program answers `events_seen`, with a narrator line or `null` (3.2). It answers at once when the batch has no big moment. The bridge waits at most 60 s, so a slow story program never blocks the player.
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
- **A budget** limits the use: a number of calls per hour, and a length per answer. The narrator and the chronicle use the fewest calls. The budget matters most for a subscription agent, because its calls count against the player's plan.
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
- The sagas of the bard (3.3) are words, not facts, so they live in a file of their own next to the history: `c_<character id>.chronicle.jsonl`, one line for each chapter, keyed by the tick that began the chapter. The same rules hold for a broken last line and a failed write.
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
4. **A check on every answer.** Before an answer shows, the story module checks it against a list of names and events past the cutoff: for example the defeat of Ragnaros, the opening of the Scarab Wall, Naxxramas over the Plaguelands, Shattrath, the fall of the Lich King, the Cataclysm, and Pandaria. A hit means one retry with the reason. A second hit drops the answer. Names that already exist in the lore of 25 ADP, such as Ragnaros, Arthas, Illidan, and Deathwing, stay allowed with their story up to that year only.

The list of later names is data in the repo, with a test for each entry. When Forever moves forward in the story, the cutoff moves with one change to that list and to the canon seed.

### 5.10 Lore data: the lore pack and the lore of each player

A web request for each question is slow, depends on one website, and sends whole pages into the prompt. So the lore is a **pack**: one file that we build once and ship. Each question is a local search.

**The sources of the pack**, built in CI:

1. **The game files of the Forever build.** The client holds little text (checked on build 1.60.1.70009, 2026-09-26). The server sends the dialogue, the quest text, the books, and the NPC names, so only the text that the player saw (below) holds them.
   - `BroadcastText` has 12 rows, all from one cinematic. `Creature` holds only companion pets. `PageText` and `QuestObjective` are not in the client.
   - The useful tables: `AreaTable` (the zones, with their parent zones and continents), `AreaPOI`, `Map`, `TaxiNodes`, the descriptions of `Faction`, and the flavor text of `ItemSparse`.
   - No client table links a text to an NPC. A text links to a place only through an ID, or through the whole name of a zone in its words.
   - The client holds zones before they open, for example Mount Hyjal. So a list of zone phases, with a test for each entry, sets the cutoff.
2. **Forever-era wiki pages**, from a database dump of warcraft.wiki.gg, cut into short passages, each with its source link. The text is CC BY-SA, so the pack names its sources and keeps that license.
   - **A dump, never a fetch.** The terms of wiki.gg forbid crawling and scraping, and `robots.txt` blocks `/api.php`. The build reads a local dump file.
   - **The infoboxes give the links.** The raw wikitext of a dump holds each infobox call, for example `{{Npcbox}}` with its location. The Cargo tables of the wiki hold only a few of these fields.

**The pack:**

- One SQLite file with a full-text index (FTS5). The expected size is a few tens of MB.
- A passage has: its text, its source, its phase tag, and links to the zones, NPCs, quests, and items that it is about.
- It downloads with the release of the bridge, and it updates when Forever releases a new phase.
- **The cutoff is built in.** The pack holds only the passages up to the current phase of Forever. A Molten Core passage is not in the file before Molten Core opens, so no model can see it. Layers 3 and 4 of 5.9 still apply to the text of the model.

**The builder** (built): `timeways-pack` reads passages as JSON lines, each with its text, source, places, and NPCs, and writes the pack. It refuses a passage with no link, and it never writes over a pack that exists. The dump pipeline feeds it later.

**A question:**

1. The story module searches the pack for the question and the context (zone, target, quest).
2. It keeps only passages whose links are in the world of the player: a zone that they visited, an NPC that they met, a quest that they did. This is the spoiler limit.
3. The best 5 to 10 passages go into the prompt with their sources.
4. The model answers only from them, and names the sources.

**The lore of each player**, in the data folder of the bridge:

- **The world:** the Hourglass history, one append-only file per character (5.7).
- **The text that the player saw** (built): the addon sends the game text of each quest, gossip window, and book as the player reads it. It covers the text that the server sends and the client files do not hold.
  - The name of the character becomes `$N` in the addon, so no model sees it (5.11). A letter that a player wrote has a creator, and never goes out.
  - A file for each character keeps each text once (`c_<name>.learned.jsonl`, which also keeps the rumors (3.1.1)). An index in memory searches it, and is built again at each start.
  - The player read the text, so it passes the spoiler limit. A search uses this text first, and the pack fills the rest (3.1.1).
  - The addon cuts a text at 2000 bytes.
- **Not in the saved variables.** Any addon can read the saved variables of another addon, so they hold only window state.

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

- The relay key stays `strip.key`. Setup makes `timeways.key` next to it, in the config folder of the bridge, with mode 0600, and writes a `Key.lua` into the Timeways addon folder. The classifier of the relay denies both keys to every agent. The bridge refuses to start if the two keys are the same.
- The bridge checks the tag of each strip under both keys. One key verifies: that app. None: refused. Both: refused as ambiguous. A new statement, S29, proves this choice. A frame in the saved variables of one app counts only under that app's key.
- **Proved statements change.** Each app sets its own Lua globals in its slot files (`Timeways_SlotData` and more), so one app never overwrites what the other is about to read. S9, S18, and S20 of the relay are restated over the app (approved).
- The Timeways lane parses only the transport flags. A coding flag such as `perm=` or `level=` in a Timeways record does nothing, and a non-empty `cwd` is refused.
- **What the key split protects.** It stops a bug or a hacked story program from reaching the agents. It does not stop a hostile addon that loads first from reading either key.
- **A strip signed with the Timeways key reaches only the story program**, never a coding agent. The bridge enforces this: the story route has no access to the agents. So a Timeways bug, a hacked Timeways update, or a hostile addon that drives Timeways gets only story powers: the model budget, false game facts in the world, and fake story text. It gets no path to commands.
- A player with only Timeways has no coding config: setup asks no folder question and sets up no coding agent. The config has only a `[story]` section for the model.

**The copies in Timeways** (built): `addon/Timeways` holds `Sha256.lua`, `Codec.lua`, `Saved.lua`, `Health.lua`, `Strip.lua`, `Slots.lua`, and `Messages.lua` from the relay, and CI compares each one with the pinned relay commit. `Link.lua` is the seam: it sends each batch in the one chat `story`, gives each done reply to the JSON handlers, and shows each error reply as plain text.

**No public send function.** Each addon carries its own private copy of the Lua transport: `Codec.lua`, `Sha256.lua`, `Strip.lua`, and the slot poll. A shared library addon is refused: its key would pass through a global function, and a hostile addon could hook it. One source folder of the transport, with its tests, lives in the Gnomish Relay repo. The packaging of each addon copies it, with a version pin.

**Slots for each addon.** Timeways gets its own set: `Timeways_S0001` to `Timeways_S1000`, with `## Dependencies: Timeways` and `## Group: Timeways`, so the AddOns list folds them under Timeways. The two addons never use up each other's slot loads.

**One strip at a time.** Both addons draw their strip in the same corner, so they take turns. A shared global busy value holds the time when the current strip ends. Each addon checks and sets it in one handler, and WoW Lua runs on one thread. A hostile addon can hold the value to block strips, which it can do today anyway. After 30 s of "busy", each addon shows "Screenshots blocked by another addon".

**The bridge keeps each app apart:** a replay store, a state file, a rate limit, a slot window, a saved-variables file to watch, a reload inbox, tokens, and restore for each app. Timeways has no restore bundle: its state lives on the desktop, and the addon rebuilds from there.

**The story program.** The bridge starts `timeways-story` when the Timeways key exists, the same way it starts an ACP agent. They talk over stdin and stdout, with JSON lines:

- The bridge sends the decoded Timeways records.
- The story program sends back the story text. The bridge writes every file that the game reads, with the proved writers.
- The story program reads hostile text, so it runs in the sandbox of the relay (SPEC 6.6.4): it writes only `<data>/timeways/`, has no network, and cannot read the protected paths. On Windows there is no sandbox yet: Timeways runs, with a one-time warning.
- The bridge starts it from a path in the config, with no shell and a short list of environment variables. `restart` and `update` restart it too, and a crash restarts it after a delay.
- The bridge writes only `Key.lua` into the Timeways addon folder, never other Timeways files.

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
3. **The wiki.** warcraft.wiki.gg text is CC BY-SA. The pack keeps the source of each passage and the license. Answers summarize and cite, and they do not copy long passages.
   - **No dump is available yet** (checked on 2026-09-25). `Special:Statistics` shows none. A request goes through `Special:Contact` on the wiki, or through the wiki.gg service desk. wiki.gg allows one request every 7 days.
   - Until the dump arrives, the tests of the lore code use invented passages only.
4. **The game files** (5.10). The client holds zones and a little text, and no dialogue. Still open:
   - The tool. The Forever build is on product `wow_classic_beta` now, and its code at launch is unknown. The files come from the public CDN (TACTTool with DBC2CSV, and the WoWDBDefs layouts), or from wago.tools after its owners allow it.
   - The license. The Blizzard EULA forbids data mining. Names and links in the pack carry less risk than long text. Decide before the first release that ships Blizzard text.
   - Does the server send gossip as `BroadcastText` rows into the cache of the client? A test in the game settles it.
5. **The API of the Forever client.** Check each event in 5.4 with the API gate.
6. **The canon seed.** Which canon characters, places, and factions go into every world at the start, and with which facts? The Forever client data (for example its database tables for the Forever build) is the best source.
7. **Decided: two addons** (5.12). Still open: do 2000 slot folders make the game start slower, and does a `## Group` start folded in the AddOns list? Measure both in the game.
8. **The strength of the echo lore** (5.13). Off, light, or strong, and which level is the default? Does a strong level need a named bronze dragon, and how does it stay inside the lore cutoff?
9. **The closed combat log** (5.4). Addons in this client cannot read the combat log. Kills of rares and bosses and your deaths work without it. These features still need a source:
   - Nemesis (4.1): the death recap names the killer, but gives no GUID, so the addon cannot tell a player from an NPC with the same name for sure.
   - Critter kills and common mob counts (5.4.1): `UNIT_DIED` gives a GUID, but a GUID can be secret, and the range of the event is not documented.
   - Wipes and the first player to die in a raid (4.2).
   - A test in the game settles what `UNIT_DIED`, `PARTY_KILL`, and the recap really give, and when values are secret.
