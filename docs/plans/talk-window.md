# Plan: the talk window

Status: draft 1, 2026-10-03. The user approved the design. When a part is built, its rules move into `GAMEPLAY.md` (section 11 of this plan), and this plan marks the part as done.

A smaller first version is built (2026-10-04, `GAMEPLAY.md` 3.5): the window with the name of the NPC, "Thinking...", a text box after the first answer, Goodbye and Escape, and the hide in combat (`TalkWindow.lua`). Past talks live in the saved variables (`TalkHistory.lua`). Work in a talk becomes a real quest with a quest card. The protocol does not change: each reply is a normal `talk_asked`, and the NPC memory of the prompt stays as it was. Not built yet: the conversation in memory (4.1 to 4.3), the options and their keys, the portrait, the daily cap of talk trust, and the two pools (7).

This plan builds on `docs/plans/npc-memory.md` (NPCs remember you in `/talk`). That plan owns the memory block of the prompt. This plan owns the conversation: the window, the turns, the options, and the limits.

## 1. Goal

`/talk` on the target opens a conversation window, as the gossip window of the game does:

- the portrait of the NPC, and its line;
- 3 short replies to click;
- a text box for your own reply;
- a Goodbye button that is always there and costs no model call.

One model call for each turn gives the line of the NPC, the change of trust, and the 3 replies, in one JSON answer. The story program keeps the open conversation in memory, so each turn sees the earlier turns. The memories of npc-memory.md go into the first turn.

The model call limits go up. Today the bridge admits 10 calls in 20 minutes for all of Timeways, and a talk of 5 turns takes half of that. Section 7 gives the new limits.

## 2. What changes, in short

| Part | Change | Owner |
|---|---|---|
| `talk_asked` | new optional `conversation` | relay check, Timeways |
| `talk_answer` | new optional `conversation`, `options`, `ended` | relay check, Timeways |
| `model_call` | new optional `pool`: `story` or `talk` | relay, Timeways |
| `model_failed` | new optional `reason` | relay, Timeways |
| `hello` of the bridge | new optional window lengths of the two pools | relay, Timeways |
| Story program protocol | `PROTOCOL` goes from 1 to 2 | Timeways; the bridge accepts 1 and 2 |
| Bridge budget | one budget for each pool, and a window for each model kind | relay |
| Open calls | 2 for the story pool, 1 for the talk pool | relay, Timeways |
| Story program | `conversation.rs`, options in `talk.rs`, a daily cap of talk trust for each NPC | Timeways |
| Addon | `TalkWindow.lua`, and `Talk.lua` sends to it | Timeways |

Goodbye sends no line. Section 3.3 says why.

## 3. Protocol

### 3.1 Addon to story program: `talk_asked`

```json
{"type":"talk_asked","at":1790000000,"npc":"Innkeeper Farley","text":"Any work for me?","conversation":7}
```

- `conversation` is optional. With none, the turn starts a new conversation. The story program picks the number, and the answer carries it (3.2). The addon sends it with each later turn of that window.
- `text` is what the player says: the typed words, or the words of the clicked reply. The limit stays 255 bytes. A reply option has at most 240 bytes (4.4), so a clicked option always fits.
- `/talk` alone still sends "Hello.", so the NPC speaks first.
- **Relay change:** `talk_asked` has an exact shape (`deny_unknown_fields`). The bridge adds `conversation`: optional, an integer from 1 to 4294967295.

### 3.2 Story program to addon: `talk_answer`

```json
{"type":"talk_answer","id":12,"npc":"Innkeeper Farley","text":"Work? The mill has rats, and nobody wants them.","conversation":7,"options":["Rats? I can handle rats.","Who owns the mill?","What else is new?"],"ended":false}
```

| Field | Rule |
|---|---|
| `conversation` | Optional. The number of the conversation, also when `text` is `null`. |
| `options` | Optional. A list of 0 to 3 strings. Each one is 1 to 240 bytes, on one line, with no control character. The story program sends no `|` (4.4). The bridge doubles every `|` anyway (S10). |
| `ended` | Optional, default `false`. `true` when the conversation is over: the NPC said its last line (4.3), or the conversation ended while the model worked. |

- The done reply of the bridge carries the three fields to the game, after `text`.
- **Relay change:** `talk_answer` has an exact shape (`deny_unknown_fields`). The bridge adds the three fields with these checks.
- `notice` of a talk answer now carries a line for the window too: the budget and the missing model (5.5). The addon shows it in the window, not in the chat, when the window waits for that answer.

### 3.3 Goodbye sends nothing

Goodbye, the close button, and Escape close the window. No line goes to the desktop. Why:

- The outbox sends game events every 10 minutes (`FLUSH_SECONDS` = 600). A goodbye line comes late anyway.
- The story program ends a conversation on its own (4.2). It needs no notice.
- One line fewer means one shape fewer to check in the bridge, the story program, and the fuzzers.

### 3.4 Story program and bridge

| Line | Change |
|---|---|
| `model_call` | Optional `pool`: `"story"` (default) or `"talk"`. A conversation turn is `talk`. Every other call is `story`: narrator, saga drafts and judge, quest, task draft, and `/lore`. |
| `model_failed` | Optional `reason`: `"no_model"`, `"busy"` (too many open calls), `"budget"`, `"timeout"`, or `"failed"`. |
| `hello` of the bridge | Optional `story_window_seconds` and `talk_window_seconds`: the window of each budget (7.2). Each pool admits 10 calls in its window. |
| `hello` of the story program | `protocol` is 2. |

**Relay change:** all four. A missing `pool` is `story`, and a missing `reason` reads as `failed`, so the bridge and the story program each work with an older other side, except for the protocol check.

### 3.5 Versions and order

The bridge compares the protocol of the story program. Protocol 2 means: the story program sends the new fields, and the bridge must take them.

1. The relay ships first. Its bridge accepts story protocol 1 and 2, takes the new optional fields, and runs the two pools.
2. Timeways bumps its pin of `app-protocol` (`crates/fake-bridge/Cargo.toml`) and ships protocol 2.

A player with an old bridge and a new story program gets "Update the desktop app: run gnomish-relay update." (relay SPEC 9.8, Versions). A new bridge with an old story program works as today.

An old addon with a new story program works too. It ignores the new fields of `talk_answer`, and it shows the words in the chat. It sends no `conversation`, so each `/talk` is a conversation of one turn.

### 3.6 The message for the relay session

Send this list to the Gnomish Relay session:

1. `talk_asked` (addon line): add optional `conversation`, an integer from 1 to 4294967295.
2. `talk_answer` (story line): add optional `conversation` (the same range), optional `options` (an array of at most 3 strings, each 1 to 240 bytes, one line, no control character), and optional `ended` (a boolean). Put the three in the done reply after `text`, with every `|` doubled (S10).
3. `model_call`: add optional `pool`, `"story"` or `"talk"`, default `"story"`.
4. Two pools in `model.rs`: each pool has its own S14 limiter (10 calls in 60 steps) and its own cap of open calls: 2 for `story`, 1 for `talk`. A call over the cap of its pool, or over the budget of its pool, gets `model_failed` at once. One pool never takes a slot or a call of the other. So the total is 20 calls in a window pair and 3 open calls.
5. The window of each pool depends on the kind of model, when the config gives no window (7.2): local model, 1 minute for each pool; Claude, 10 minutes for `story` and 5 minutes for `talk`; a hosted model (later), the Claude windows and a daily cap in money.
6. Config: `[story] budget_window_minutes` stays, as the override for the `story` pool. Add `[story] talk_window_minutes` (1 to 1440), the override for the `talk` pool. Setup writes neither as a live line, so the default of the model kind applies. Decide what happens to a config that has the old live line `budget_window_minutes = 20` from setup: the plan proposes that `update` comments it out once.
7. `model_failed`: add optional `reason`: `"no_model"`, `"busy"`, `"budget"`, `"timeout"`, or `"failed"`.
8. `hello` of the bridge: add `story_window_seconds` and `talk_window_seconds`, the windows in force.
9. Accept story protocol 1 and 2.
10. SPEC 9.8 says "At most 2 model calls ... (one of the narrator, one of the bard)". Replace it with the pools of item 4.
11. The fake story program and the end-to-end test: a talk of two turns with a conversation number and options.

No new addon line needs the relay: Goodbye sends nothing (3.3).

## 4. Story program

### 4.1 The conversation, in memory

A new module `crates/story/src/conversation.rs` holds the state and its rules. It holds no store code.

```rust
//! An open talk with one NPC (GAMEPLAY.md 3.5): its turns, its end, and its trust. It
//! lives in memory only, so a restart ends it.

/// The turns of one conversation that ask the model. The last one ends it in character.
pub const MAX_TURNS: usize = 8;

/// The earlier turns that a prompt holds in full. More breaks the context of a small model.
pub const TURNS_IN_PROMPT: usize = 4;

/// A turn after this pause starts a new conversation.
pub const IDLE_SECONDS: u64 = 10 * 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConversationId(pub u64);

pub struct Turn {
    pub words: String,
    /// None when no model answered, or the answer broke a rule.
    pub say: Option<String>,
}

pub struct Conversation {
    pub id: ConversationId,
    pub key: CharacterKey,
    pub npc: String,
    pub turns: Vec<Turn>,
    /// The call rows of the earlier turns. A later turn reads them (5.14).
    pub calls: Vec<u64>,
    pub last_at: Tick,
    /// True after the last turn. The options are not kept: the addon sends the words of
    /// the reply that the player clicked.
    pub ended: bool,
}
```

Functions, one job each:

| Function | Job |
|---|---|
| `Conversation::continues(&self, key, npc, at) -> bool` | True when a turn with this number belongs here: same character, same NPC, and less than `IDLE_SECONDS` since `last_at`. A full conversation still continues, so its number cannot buy a ninth turn. |
| `Conversation::is_last_turn(&self) -> bool` | True when the next turn is turn `MAX_TURNS`. |
| `Conversation::is_full(&self) -> bool` | True when `MAX_TURNS` turns asked the model. |
| `Conversation::recent(&self) -> &[Turn]` | The last `TURNS_IN_PROMPT` turns. |

`Story` gets `conversation: Option<Conversation>` and `next_conversation: ConversationId`. The number goes into the shared values of `timeways.sqlite`, next to `budget` and `pace`, so a number is never used twice for a character, also after a restart.

### 4.2 When a conversation starts and ends

`fn talk` takes the turn:

1. With a `conversation` that `continues`: the turn joins it.
2. With no `conversation`, or one that does not continue: a new conversation starts. The old one ends. So a restart, an unknown number, another NPC, or a pause of 10 minutes gives a new conversation, and the answer names the new number. The addon takes the new number (5.6).
3. A turn that joins a conversation that `is_full` asks no model. The answer has `text: null`, no options, and `ended: true`. The addon never sends it (its window shows only Goodbye), so only a bug or a hostile addon gets here. A turn with no number still starts a new conversation: the daily cap of talk trust (4.5) and the talk pool (7.2) bound that.

A conversation also ends when:

- `character_entered` names another character;
- its turn `MAX_TURNS` gets its answer (it stays in memory as full, for rule 3);
- a new conversation starts.

There is one conversation at a time, because the addon has one window.

An answer that comes after its conversation ended still counts: its rumor and its trust land as today. The answer has `ended: true` and no options. The addon shows it in the chat (5.6).

### 4.3 The prompt

`talk::prompt` takes the turns. The parts, in order:

1. The persona and the house rules, as today.
2. What the NPC knows, as today. **Only the first turn** gets the memory block of npc-memory.md (its section 5). A later turn has the earlier turns, and those hold any memory that the NPC used. So the prompt has room for the turns.
3. The golden samples, as today.
4. **The talk so far**, when the conversation has earlier turns: the last `TURNS_IN_PROMPT` turns, oldest first, in one fence (`house::fenced`). The player's words are hostile input, so the fence rule of 3.2.1 applies.

   ```
   The talk so far, oldest first:
   <<<
   Player: Hello.
   You: Evening. Rain again, and the roads are mud.
   Player: Any work for me?
   You: (you said nothing)
   >>>
   ```

   A turn with no answer shows "(you said nothing)", so the model sees the gap and does not invent a line.
5. The words of this turn, fenced, as today.
6. The author's note and the JSON shape, last:

   ```
   Remember: you are the person of the name above. Speak plainly, in your own voice, in at most 60 words.
   Then give 3 short things that the player can say next, in the player's voice: each at most 10 words, each different, with no names and no goodbye. Example: ["What happened at the mill?", "Can I help?", "Who runs this place?"]
   Reply with JSON only: {"say": "<your answer>", "trust": <a whole number from -5 to 5>, "options": ["<reply 1>", "<reply 2>", "<reply 3>"]}
   ```

   The last turn (`is_last_turn`) gets another note: "This is your last answer in this talk. End the talk as this person would: you have work to do, or somewhere to be. Reply with JSON only: {"say": "<your answer>", "trust": <...>, "options": []}".

**Size.** Each prompt fits a context of 2048 tokens (3.2.1).

| Part | Tokens, at most |
|---|---|
| The talk prompt of `tests/voice.rs` today | 457 |
| The memory block, first turn only (npc-memory.md, 7) | 365 |
| The options rule and example | 60 |
| 4 earlier turns: 255 bytes of words and 400 characters of say each, with labels | 4 x 175 = 700 |

The first turn takes at most 457 + 365 + 60 = 882 tokens. A later turn takes at most 457 + 60 + 700 = 1217 tokens. The reply grows by 3 options of 60 characters and their marks (about 50 tokens), so `tokens::Call::Talk` leaves 1823 - 50 = 1773 tokens for the prompt. Both fit.

### 4.4 The answer and its checks

```rust
#[derive(Deserialize)]
struct Reply {
    say: String,
    trust: i64,
    #[serde(default)]
    options: serde_json::Value,
}

pub struct Answer {
    pub say: String,
    pub trust_change: i64,
    pub options: Vec<String>,
}
```

- `say` and `trust` keep their checks (3.5 of `GAMEPLAY.md`). A bad `say` still means "looks at you and says nothing".
- `options` is a `Value`, so a wrong type drops the options and keeps the words.
- `fn checked_options(options: &Value, facts: &str, own_name: &str) -> Vec<String>` keeps each option that passes, in order, up to 3:
  - a string, trimmed, 1 to `MAX_OPTION_CHARS` = 60 characters and at most `MAX_OPTION_BYTES` = 240 bytes;
  - no control character and no `|`. A `|` starts an escape in the game, and an option goes back to the desktop as the player's words;
  - passes `check::in_voice`: no emoji, no banned word;
  - no name after the cutoff (`names_after_cutoff`);
  - not the name of the character (`check::mentions(option, own_name)`);
  - no name of no fact: `check::names_in_no_fact(option, facts)` is empty. `facts` is the prompt **without the player's words** of this turn and of the earlier turns. So a name that only the player typed, such as another player's name, never comes back in an option (5.11);
  - not the same words as an option before it, in any case.
- The last turn keeps no options, whatever the model sent.
- An answer with fewer than 3 good options shows the ones that passed. With none, the window shows the text box and Goodbye only.

Why 60 characters and 240 bytes: a reply that you click is short, like a gossip option of the game. 240 bytes is 60 characters of 4 bytes, and it fits the 255 bytes of `talk_asked`.

### 4.5 Trust

- Each turn proposes a change of -5 to 5, as today. A change outside is dropped, and the words still show.
- **New: a daily cap for each NPC.** Talk changes the trust of one NPC at most 10 up and 10 down in any 24 hours of game time. The code clamps each change: `applied = clamp(sum + change, -10, 10) - sum`, where `sum` is the talk trust of that NPC in the last 24 hours. A quest (+10) and a slap do not count, and the cap never limits them.
- Why: 8 turns of +5 give +40, and a new conversation starts the count again. With the cap, talk alone never moves an NPC from neutral (0) past "likes you" (10) in one day. A finished quest still matters more.
- `sum` comes from the world, so it holds across a restart: `why::talk_trust_since(active, npc, since) -> Result<i64, StoreError>` adds the trust events of the NPC after `since` whose cause is a talk (`cause_of`, the code of "Why the trust changed").
- The trust change lands only for the character that talked, and never before the last event, as today.

### 4.6 Rows and reads (5.14)

| Row | Each turn |
|---|---|
| `inputs` | the `talk_asked` line, root Player, as today |
| `calls` | one row of kind `talk`, with its prompt and its answer |
| `learned` | the words of the NPC as a rumor, as today |
| Hourglass | the trust event, when the change is not 0 |

A turn reads what a talk reads today, and also **the call rows of the earlier turns of its conversation** (`Conversation::calls`). The saga judge reads the earlier calls of its round in the same way. So a trust change of turn 6 rests on the words of turns 1 to 5, and their proof flows down to the inputs.

The first turn also reads the rows behind its memories (npc-memory.md, 6). A later turn reads them through the call of the first turn.

**What is stored.** No new table and no migration.

- Each turn is a row in `inputs` and in `calls`, and its words are a rumor. The conversation itself is not stored: its number and its turns live in memory.
- `Rumor` gets `#[serde(default)] conversation: Option<ConversationId>`. An old rumor reads as `None`.
- The Knowledge page shows **one rumor for each conversation**: the newest one. A rumor with no conversation shows alone, as today. So a talk of 8 turns adds one line to Knowledge, not 8.
- **For npc-memory.md:** `answers_heard` then takes the newest rumor of each of the newest `REMEMBERED_ANSWERS` conversations, not the newest rumors. Tell the npc-memory session.

### 4.7 Calls and pools

- `Pending::Talk` gets `conversation: ConversationId`.
- `MAX_OPEN_CALLS` splits into `MAX_OPEN_STORY_CALLS` = 2 and `MAX_OPEN_TALK_CALLS` = 1, with one queue for each pool. `fn pool(&self) -> Pool` on `Pending` gives `Pool::Talk` for `Talk` and `Pool::Story` for every other kind.
- `Output::ModelCall` gets `pool: Pool`.
- `has_free_slot` and `send_queued` take a pool. A talk turn never waits for a saga, and a saga, a narrator line, a quest, or a `/lore` question never waits for a talk turn.
- A second turn while the first one runs waits in the talk queue. The addon never sends one, because its window waits (5.4).
- `Pace` counts only calls of the story pool, as `opened` and `failed`. A failure of a talk turn says nothing about the budget of the sagas.
- `Pace` takes its window from the `hello` of the bridge (`story_window_seconds`). With none, it keeps 20 minutes. `BUSY_CALLS` stays 5: half of the 10 calls of a window.
- `fn failed` reads `reason`. For a talk turn, `budget` gives the notice "AI limit reached. Try again in a few minutes.", and `no_model` gives "Talking needs an AI model. To add one, run gnomish-relay setup --timeways on your computer." Other reasons give no notice, and the NPC says nothing.

## 5. Addon

### 5.1 The window

A new file `addon/Timeways/TalkWindow.lua`, in the look of the lore book (`LoreBook.lua`): the rock background, the dialog border, `HIGH` strata, movable, and in `UISpecialFrames`.

```
+---------------------------------------------------+
|                 Innkeeper Farley              [x] |
| +--------+  You: Any work for me?                 |
| |portrait|  Work? The mill has rats, and nobody   |
| |        |  wants them.                           |
| +--------+                                        |
|  (o) 1. Rats? I can handle rats.                  |
|  (o) 2. Who owns the mill?                        |
|  (o) 3. What else is new?                         |
| [ Say something...                    ] [ Say ]   |
|                                     [ Goodbye ]   |
+---------------------------------------------------+
```

- **Size:** 440 wide. The height follows the text, from 260 to 420. A line of 400 characters fits in about 8 lines at this width, so the NPC line needs no scroll.
- **Place:** left of the center (`CENTER`, -260, 40), so the NPC stays in view. The player can drag it.
- **Title:** the name of the NPC, `GameFontNormal`.
- **Portrait:** a `PlayerModel` frame of 88 by 88, set once at open with `SetUnit("target")` and `SetPortraitZoom(1)`. It keeps the NPC when the target changes. `CanSetUnit`, `SetUnit`, and `SetPortraitZoom` are methods of `PlayerModel` in `addon/tests/api.lua`, the API of the Forever client. When `CanSetUnit("target")` is false, the portrait box hides, and the text takes its place. The 2D `SetPortraitTexture` is not in `api.lua`: use it only after the API gate (`wow-api.sh`) confirms it in the client.
- **Your words:** "You: <words>", `QuestFontNormalSmall`, faded ink, above the NPC line. Only the newest turn shows.
- **The NPC line:** `QuestFont`, text ink, on parchment.
- **Options:** up to 3 buttons, full width. Each shows "1. ", "2. ", or "3. " and the words, with the gossip bubble icon (`Interface\GossipFrame\GossipGossipIcon`) and the quest highlight on hover. The words go through `ns.Plain`.
- **The text box:** `InputBoxTemplate`, `SetMaxBytes(255)`, with the faded hint "Say something..." while it is empty. It has no focus at open.
- **Say:** a button right of the box. Off while the box is empty.
- **Goodbye:** bottom right, always on, also while the NPC thinks.
- The close button `[x]` and Escape do what Goodbye does.

### 5.2 Keyboard

The frame takes the keyboard only while it shows (`EnableKeyboard(true)`), and only out of combat (5.3).

| Key | Box has no focus | Box has focus |
|---|---|---|
| 1, 2, 3 | picks that option, when it shows; else goes to the game | types |
| Enter | gives the box focus | sends the words (Say) |
| Escape | closes the window (Goodbye) | clears the focus; a second Escape closes |
| any other key | goes to the game (`SetPropagateKeyboardInput(true)`) | types |

So while the window shows and the box has no focus, 1 to 3 pick options and do not use the action bar. The window is never open in combat, so this never costs a spell in a fight. `EnableKeyboard` and `SetPropagateKeyboardInput` are in `api.lua`.

### 5.3 Combat

Story after the action (rule 3).

- `/talk` in combat opens no window and sends nothing. The chat says "You can't talk while in combat."
- When combat starts (`PLAYER_REGEN_DISABLED`), an open window hides and lets go of the keyboard. The conversation stays open.
- When combat ends (`PLAYER_REGEN_ENABLED`), the window shows again with what came in the meantime.
- An answer in combat waits in the window. It does not go to the chat.
- **A trap:** the hide of combat fires `OnHide`, as Escape does. The window keeps a state, `Open`, `Away` (combat), or `Closed`, and only a hide in `Open` is a Goodbye.

### 5.4 Thinking, a closed window, and a timeout

- **Thinking.** After a turn goes out, the NPC line shows "Thinking..." in faded ink. The options hide. The box keeps the words and the Say button is off. Goodbye stays on.
- **The answer.** The line shows, then the options. With `ended: true`, the options and the box hide, and only Goodbye shows.
- **No words** (`text: null` with no notice): "Innkeeper Farley looks at you and says nothing.", as in the chat today. The box shows again with the words of the turn, so the player can say them again or change them.
- **A notice** of the answer (budget, no model) shows in the place of the NPC line, in faded ink. It does not go to the chat.
- **No answer.** An error reply of the bridge for the batch (`Outbox.Add(input, failed)`, as `/lore` does), or no reply after 150 seconds (the bridge timeout of 120 seconds and the transport), shows "No answer came back." and a Retry button. Retry sends the same words again. The box keeps them.
- **A closed window mid-turn.** Goodbye while the NPC thinks closes the window at once. The answer still comes: its words go to the chat as today ("Innkeeper Farley says: ..."), with no options. The trust lands.

### 5.5 Which answer goes where

`Core.lua` keeps `talk_answer = ns.Talk.Show`. `Talk.Show` gives the answer to `TalkWindow.Receive` when the window is open or away and the answer belongs to it:

- the window waits for the first answer of a new conversation, and the NPC matches; or
- `conversation` equals the number of the window.

Every other talk answer goes to the chat, as today. `Talk.Show` returns whether the window took the notice, so `ns.OnReply` does not show it twice.

### 5.6 The conversation number

- The window starts with no number. The first answer gives it.
- When an answer gives another number than the window holds, the story program started a new conversation (4.2): after a restart or a pause of 10 minutes. The window takes the new number. The player sees no change.
- `/talk` on another NPC while a window is open closes the old window (Goodbye) and opens a new one. `/talk` on the same NPC with words sends them as a turn of the open window. `/talk` alone on the same NPC only shows the window.

## 6. UI copy

The copy follows the UI copy rules of `CLAUDE.md`. Every line that a player reads:

| Where | Copy |
|---|---|
| Window title | Innkeeper Farley (the name of the NPC) |
| Your last words | You: Any work for me? |
| While the model works | Thinking... |
| An answer that broke a rule, or no model answer | Innkeeper Farley looks at you and says nothing. |
| Option buttons | 1. Rats? I can handle rats. |
| Text box hint | Say something... |
| Send button | Say |
| Close button | Goodbye |
| No reply came | No answer came back. |
| Retry button | Retry |
| Over the budget (notice) | AI limit reached. Try again in a few minutes. |
| No model (notice) | Talking needs an AI model. To add one, run gnomish-relay setup --timeways on your computer. |
| `/talk` in combat (chat) | You can't talk while in combat. |
| `/talk` with no target (chat, today) | Target someone to talk to first. |
| `/talk` to a foe (chat, today) | Duskbat won't talk to you. |
| An answer after Goodbye (chat, today) | Innkeeper Farley says: Work? The mill has rats. |

Why "Say": it is the word of the game for speaking aloud (`/say`). "Send" reads like a message app. Why "Goodbye": it is the button of the gossip window of the game.

Bad and good:

| Bad | Good | Why |
|---|---|---|
| Waiting for the model... | Thinking... | "Model" is an internal word. A status is a few words. |
| Send reply | Say | The game word, one verb. |
| End conversation | Goodbye | The word of the gossip window. |
| Option 1: Who owns the mill? | 1. Who owns the mill? | The number is the key to press. "Option" adds nothing. |
| The NPC has reached its turn limit. | (the NPC's own last line, then only Goodbye) | The talk ends in character, not in an error. |
| Talk budget exceeded (pool: talk). | AI limit reached. Try again in a few minutes. | "Budget" and "pool" are internal. Say what to do next. |
| Request timed out after 120 s. Press Retry to resend. | No answer came back. | The Retry button next to it already says the fix. |
| Type your reply here, then press Enter to send it. | Say something... | Robot form language. The hint only invites. |
| Thou mayest speak thy mind, traveler. | Say something... | Fake old-timey flourish. The UI never performs a voice. |
| The conversation window cannot open in combat. | You can't talk while in combat. | The way the game says it: "You can't do that while in combat." |

The model writes the options, and a player reads them, so the prompt gives what `CLAUDE.md` asks for: where the text shows (a reply that the player clicks), the length (10 words), the tone (the player's voice, plain), and an example (4.3).

## 7. Limits

### 7.1 Every limit that touches a conversation

| Limit | Today | Why it exists | New value or rule | Who |
|---|---|---|---|---|
| Bridge budget | 10 calls in 20 minutes, for all calls of Timeways (relay 9.7, decision 10) | A hostile addon cannot spend the player's plan faster than this. S14 proves it. | Two pools, each 10 calls in its window. The window depends on the model kind (7.2). | relay |
| Open calls in the bridge | 2 (relay 9.8) | Bounds the threads, the `claude` processes, and the load of a local model. | `story` 2, `talk` 1. | relay |
| `MAX_OPEN_CALLS` in `story.rs` | 2 | Matches the bridge, so the bridge never fails a call of the player for a saga (3.3). | `MAX_OPEN_STORY_CALLS` = 2, `MAX_OPEN_TALK_CALLS` = 1, with a queue for each. | Timeways |
| `Pace` (`pace.rs`) | 5 calls in 20 minutes make the window tight | Keeps the best of two from eating the budget. | Counts the story pool only. Its window comes from `hello` (`story_window_seconds`). `BUSY_CALLS` stays 5. | Timeways, after the relay sends the window |
| Narrator budget (`narrator.rs`, `Budget`) | 3 lines in one hour | Rule 7: talk less, mean more. It is a design limit, not a cost. | No change. A talk turn never counts against it. A talk no longer takes a slot of the story pool, so a talk never makes the narrator quiet. | none |
| Turns of one conversation | none (one turn) | A talk of many turns costs many calls. | `MAX_TURNS` = 8. Turn 8 ends in character (4.3). | Timeways |
| Earlier turns in a prompt | none | A prompt fits a context of 2048 tokens. | `TURNS_IN_PROMPT` = 4 (4.3). | Timeways |
| Trust of one turn | -5 to 5 | The NPC proposes, the code decides. | No change. | Timeways |
| Trust from talk for one NPC | none | Flattery must not buy trust. | At most 10 up and 10 down in 24 hours (4.5). | Timeways |
| Idle conversation | none | The memory of the story program stays small. | Ends after 10 minutes with no turn. | Timeways |
| `talk_asked` `text` | 255 bytes (relay 9.8; the chat box of the game) | One chat line. | No change. The text box has `SetMaxBytes(255)`. | none |
| `talk_answer` `text` | 400 characters, 1600 bytes | About 70 words, and the slot limit. | No change. | none |
| Options | none | | At most 3, each 60 characters and 240 bytes (4.4). The longest answer line is then about 1600 + 3 x 240 + the notice, far below 24576 bytes, and below 32 KB after the Lua escape. | relay check, Timeways |
| Model answer | 16 KiB (relay) | Bounds the read. | No change. The JSON of a turn is under 1 KiB. | none |
| Prompt | 256 KiB (relay) | Bounds the line. | No change. The 2048 tokens of a local model are the tighter limit. | none |
| Reply timeout | 120 seconds (`[story] timeout_seconds`) | A hang of the story program ends. | No change. The window waits 150 seconds. | none |
| Model timeout | 60 seconds (`[story] model_timeout_seconds`) | A stuck model ends. | No change. With 3 open calls on one local model, a turn can wait behind a saga. A saga of 600 characters takes 10 to 20 seconds on a small model, so 60 seconds holds. | none |
| Messages of the lane | 10 messages in a minute (relay S14) | Strip spam. | No change. A fast talk sends a turn every 15 seconds or more: 4 in a minute, with room for the game events. | none |

### 7.2 The windows of the two pools

The proof of S14 fixes 10 calls in 60 steps. The config picks the length of a step through the window (relay 9.7, decision 10). So each pool keeps the proved limiter, and only the window changes. Two independent limiters need no new theorem: each one holds alone, and the total is their sum.

| Model kind | `story` window | `talk` window | At most, in one hour | Why |
|---|---|---|---|---|
| A local model (Ollama, LM Studio) | 1 minute | 1 minute | about 600 + 600 | It costs no money. The limit only stops a loop: a bug or a hostile addon that calls without end. A small model writes one answer in 5 to 20 seconds, so a player never reaches it. |
| Claude through the player's login (subscription) | 10 minutes | 5 minutes | about 60 + 120 | Each call counts against the player's plan, so the cap stays real. It is 6 times the cap of today. |
| A hosted model that the player pays for (later, relay 11.6) | 10 minutes | 5 minutes | about 60 + 120, and a cap in money | Every call costs money. The bridge also stops at `[story] daily_cost_cap_usd`, default 1 USD a day, which the player can change. The business plan sets the final default. |

How the numbers come out, for one busy hour of play:

- Story pool: 3 narrator lines, 2 chapters of 3 calls, 2 quests, 5 `/lore` questions, and 1 task draft: about 17 calls. The Claude window of 10 minutes allows 60.
- Talk pool: 3 conversations of 6 turns: 18 calls. The Claude window of 5 minutes allows 120. One full conversation of 8 turns always fits in one window of 10 calls, so a talk never hits the budget halfway, unless the player starts a second talk in the same 5 minutes.
- A hostile addon at full rate spends at most 180 Claude calls in an hour, against 30 today. Each prompt is under 2048 tokens.

The player can set `budget_window_minutes` and `talk_window_minutes` in `[story]` to tighten or loosen each pool.

## 8. Tests

Each test is a sentence. Each one reads arrange, act, assert.

### 8.1 Story crate

`crates/story/tests/conversation.rs` (new):

- `a_turn_with_no_number_starts_a_conversation`
- `a_turn_with_the_number_joins_its_conversation`
- `a_turn_for_another_npc_starts_a_new_conversation`
- `a_turn_ten_minutes_after_the_last_starts_a_new_conversation`
- `a_turn_just_under_ten_minutes_joins_the_conversation`
- `another_character_ends_the_conversation`
- `the_prompt_holds_the_last_four_turns_oldest_first`
- `a_turn_with_no_answer_shows_as_silence_in_the_prompt`
- `the_eighth_turn_is_the_last`
- `a_full_conversation_asks_no_model_and_ends`

`crates/story/tests/talk.rs`:

- `an_answer_keeps_three_good_options_in_order`
- `a_fourth_option_is_dropped`
- `an_option_over_sixty_characters_or_240_bytes_is_dropped`
- `an_option_with_a_pipe_or_a_control_character_is_dropped`
- `an_option_with_the_name_of_the_character_is_dropped`
- `an_option_with_a_name_that_only_the_player_typed_is_dropped`
- `an_option_with_a_name_after_the_cutoff_is_dropped`
- `an_option_with_a_banned_word_or_an_emoji_is_dropped`
- `the_same_option_twice_shows_once`
- `options_that_are_not_a_list_leave_the_words`
- `an_answer_with_no_options_still_shows_its_words`
- `the_last_turn_keeps_no_options`
- `the_prompt_of_the_last_turn_asks_the_npc_to_end_the_talk`
- `a_player_line_cannot_close_the_fence_of_the_talk_so_far`
- `the_memories_go_only_into_the_first_turn`

`crates/story/tests/story.rs`:

- `the_answer_of_a_first_turn_names_its_conversation`
- `a_later_turn_carries_the_earlier_turns_in_its_prompt`
- `each_turn_keeps_its_input_its_call_and_its_rumor`
- `a_turn_reads_the_calls_of_the_earlier_turns`
- `a_restart_forgets_the_conversation_and_the_next_turn_starts_a_new_one`
- `conversation_numbers_never_repeat_after_a_restart`
- `an_answer_after_a_new_conversation_still_lands_its_trust_and_ends`
- `talk_moves_the_trust_of_one_npc_at_most_ten_in_a_day`
- `the_cap_of_talk_trust_opens_again_after_a_day`
- `a_quest_and_a_slap_do_not_count_against_the_cap_of_talk_trust`
- `a_talk_turn_goes_to_the_talk_pool`
- `a_talk_turn_never_waits_for_a_saga`
- `a_saga_never_waits_for_a_talk_turn`
- `the_narrator_speaks_while_a_talk_turn_runs`
- `a_second_turn_waits_while_the_first_one_runs`
- `pace_counts_only_calls_of_the_story_pool`
- `pace_takes_its_window_from_the_hello_of_the_bridge`
- `a_talk_over_the_budget_gets_a_notice`
- `a_talk_with_no_model_gets_a_notice`
- `the_knowledge_page_shows_one_rumor_for_each_conversation`

`crates/story/tests/input.rs`:

- `talk_asked_reads_with_and_without_a_conversation`
- `hello_reads_with_and_without_the_windows_of_the_bridge`
- `model_failed_reads_with_and_without_a_reason`

`crates/story/tests/voice.rs`: a new prompt `npc_talk_late_turn()` with 4 earlier turns at full length (words of 255 bytes, says of 400 characters). `every_prompt_fits_a_local_model_with_a_context_of_2048_tokens` then covers it. `each_budget_leaves_room_for_the_longest_reply` covers the bigger reply.

`crates/story/tests/through_the_bridge.rs`: a talk of 3 turns through the fake bridge, with options. The fake bridge (`crates/fake-bridge`) runs the two pools of 3.6, item 4, and takes the pinned `app-protocol` with the new fields.

### 8.2 Property tests (`crates/story/tests/properties.rs`)

A new strategy `talk_play()`: `play()`, and also talks to "Innkeeper Farley" with a conversation number that is right, wrong, or missing; `Play::Wait` of exactly 599, 600, and 601 seconds, and of 24 hours; trust answers of -6, -5, 0, 5, and 6 made likely; and a restart now and then.

- `a_conversation_never_asks_the_model_more_than_eight_times`
- `talk_never_moves_the_trust_of_one_npc_more_than_ten_in_a_day`
- `every_option_that_shows_fits_its_limits_and_names_no_typed_name`
- `every_talk_call_reads_the_calls_of_the_earlier_turns_of_its_conversation`
- `the_open_calls_never_pass_two_story_and_one_talk`

### 8.3 Addon tests (`crates/addon-tests/tests/talk.rs`)

The fake game (`addon/tests/wow.lua`) gets a `PlayerModel` frame, `EnableKeyboard`, `SetPropagateKeyboardInput`, `OnKeyDown`, `SetFocus`, `SetMaxBytes`, and a key press helper. `wow.combat` exists already.

- `talk_opens_the_window_and_sends_the_first_words`
- `talk_alone_says_hello_in_the_window`
- `the_window_shows_thinking_until_the_answer_comes`
- `the_answer_shows_the_line_and_three_options`
- `clicking_an_option_sends_its_words_in_the_same_conversation`
- `pressing_two_picks_the_second_option`
- `a_number_with_no_option_goes_to_the_game`
- `a_number_types_in_the_box_when_it_has_focus`
- `other_keys_go_to_the_game`
- `enter_gives_the_box_focus_and_then_sends_the_words`
- `an_empty_box_sends_nothing`
- `goodbye_closes_the_window_and_sends_nothing`
- `escape_closes_the_window`
- `escape_in_the_box_clears_the_focus_first`
- `the_last_answer_shows_only_goodbye`
- `an_answer_after_goodbye_shows_in_the_chat`
- `an_answer_of_another_conversation_goes_to_the_chat`
- `a_new_number_from_the_desktop_replaces_the_number_of_the_window`
- `no_words_from_the_npc_keep_the_words_of_the_player_in_the_box`
- `an_error_reply_shows_retry_and_keeps_the_words`
- `no_reply_in_150_seconds_shows_retry`
- `retry_sends_the_same_words_again`
- `a_notice_of_a_talk_shows_in_the_window_and_not_in_the_chat`
- `talk_in_combat_opens_no_window_and_sends_nothing`
- `combat_hides_the_window_and_the_end_of_combat_shows_it_again`
- `hiding_for_combat_is_not_a_goodbye`
- `an_answer_in_combat_waits_in_the_window`
- `talk_to_another_npc_closes_the_open_window`
- `the_portrait_shows_the_target_at_open`
- `with_no_unit_to_show_the_window_has_no_portrait`
- `options_show_a_pipe_as_text`
- `an_option_that_is_not_a_string_is_skipped`
- `the_box_takes_at_most_255_bytes`

Changes to tests of today: `the_npc_says_its_answer_in_the_chat` and `with_no_model_the_npc_says_nothing` keep their chat lines for an answer with no open window.

### 8.4 Fuzz

- `fuzz/fuzz_targets/answers.rs`: `talk::checked_answer` with random `options`, also not a list, with non-strings, and with more than 3. Each option that passes keeps its limits: at most 3, each 1 to 60 characters and at most 240 bytes, no control character, no `|`, `in_voice`, no "Ada" as a word, and no two the same. New seeds in `fuzz/seeds/answers/`: `talk_options.txt`, `talk_options_wrong_type.txt`, `talk_option_pipe.txt`.
- `fuzz/fuzz_targets/input.rs`: `talk_asked` with a random `conversation` (missing, 0, the largest, a string), `hello` with random windows, and `model_failed` with a random `reason`. Each request still gets exactly one answer. New seeds in `fuzz/seeds/input/`: `talk_turns.txt`, `hello_windows.txt`.
- `fuzz/fuzz_targets/play.rs` and `pages.rs`: talks of many turns with restarts. The model of the fuzzer checks each talk prompt: at most 4 earlier turns, one fence for the talk so far. It also checks the open calls of each pool.
- `fuzz/fuzz_targets/replies.rs` (the addon reply handler): `talk_answer` with random `options`, `conversation`, and `ended`. The window never raises a Lua error.

## 9. Build order

Each step is one commit with its tests, and each one ships.

1. `talk.rs`: `checked_options` and the talk so far in the prompt, as pure functions. Nothing calls them yet.
2. `conversation.rs`: the state and its rules, with `tests/conversation.rs`. Nothing calls it yet.
3. `why::talk_trust_since` and the daily cap of talk trust in `talk_answered`. This holds for the talk of today too.
4. **Wait for the relay** (3.6). Then bump the pin of `app-protocol`, set `PROTOCOL` to 2, and wire the conversation: `talk_asked.conversation`, the new fields of `talk_answer`, `Pending::Talk::conversation`, the reads, and the rumor number. The addon of today still shows the words in the chat.
5. The pools: `model_call.pool`, the split of `MAX_OPEN_CALLS`, `Pace` from `hello`, and the notices of `model_failed.reason`.
6. The window: `TalkWindow.lua` with the portrait, the line, the options, the box, Goodbye, thinking, retry, and the routing of `Talk.Show`.
7. The keyboard and combat rules of the window.
8. The Knowledge page: one rumor for each conversation. Tell the npc-memory session about `answers_heard` (4.6).
9. The `GAMEPLAY.md` text of section 11. This plan marks the steps as done.

After step 4 and step 5, tell the Gnomish Relay session what changed in the story lines.

npc-memory.md can come before or after this plan. When it comes first, step 4 moves its memory block to the first turn only (4.3).

## 10. Open questions

1. **Distance.** The gossip window of the game closes when you walk away. This window stays open. Close it when `CheckInteractDistance("target", 3)` is false for 5 seconds? That needs the NPC as the target.
2. **The keys 1 to 3.** While the window shows and the box has no focus, they pick options and do not reach the action bar. Is that right out of combat, or do players want a modifier, such as Alt+1?
3. **The default of the money cap** for a hosted model: 1 USD a day is a placeholder until the business plan.
4. **Old config lines.** A config that setup wrote with a live `budget_window_minutes = 20` keeps the slow window of today. The relay session decides how `update` handles it.

## 11. GAMEPLAY.md text

Replace the bullets of 3.5 from "The NPC proposes, and the code decides" to "No retry" with:

> - **The talk window.** `/talk` on an NPC opens a window, as the gossip window of the game does. It shows the portrait of the NPC, its line, 3 short replies to click, a text box for your own words, and a Goodbye button. Goodbye costs no model call and sends nothing. Keys 1 to 3 pick a reply. Enter sends your words. Escape closes the window.
> - **One call for each turn.** The model answers in JSON: `{"say": "...", "trust": n, "options": ["...", "...", "..."]}`.
>   - The words follow the rules of a narrator line, with at most 400 characters.
>   - A change of trust outside -5 to 5 is dropped, and the words still show.
>   - Talk changes the trust of one NPC at most 10 up and 10 down in 24 hours. A quest and a slap do not count.
>   - A valid change goes through Hourglass, inside the band of -100 to 100.
>   - Each reply has at most 60 characters, and there are at most 3. The code drops a reply with a `|`, a control character, a banned word, a name after the cutoff, the name of your character, or a name that no fact holds. So a name that only you typed never comes back as a reply.
> - **The conversation lives in memory.** Each turn sees the last 4 turns. The memories of the NPC go into the first turn. A conversation has at most 8 turns. In the eighth, the NPC ends the talk in its own words, and only Goodbye stays. A pause of 10 minutes, another NPC, another character, or a restart of the story program ends a conversation. The next turn then starts a new one.
> - **Each turn is a row.** It keeps its input, its call, and its words as a rumor. The Knowledge page shows the newest rumor of each conversation.
> - **No retry.** With no model, or with an answer that breaks a rule, the NPC "looks at you and says nothing". Your words stay in the text box.
> - **Story after the action.** In combat, `/talk` opens no window. A window that is open hides in combat and comes back after it.

In 3.2, replace "The bridge runs at most 2 model calls of the story program at once, so a question of the player always gets a call." and the bullet "It stays quiet while a saga is written, and when both model slots of the bridge are taken, so the player keeps a slot." with:

> - It stays quiet while a saga is written, and when both slots of the story pool are taken, so the player keeps a slot. A talk turn uses the talk pool (5.6), so a talk never makes the narrator quiet.

In 3.2, replace "The budget lives in memory, so a restart of the story program starts it again. That costs at most 3 more lines once." with:

> The budget is saved with the shared values, so a restart keeps it.

In 3.3, replace the bullet "A question of the player never fails for a saga" and the bullet after it with:

> - **A question of the player never fails for a saga** (built). The bridge has two pools of calls (5.6). The story pool runs at most 2 calls at once, and fails a third one. So the story program opens at most 2 calls of this pool. A call of the player (a lore question, a task, or a draft) that finds both slots taken waits for the first one to end. The narrator never waits: it stays quiet. A talk turn uses the talk pool, so it never waits for a saga, and a saga never waits for it.
> - So a chapter costs at most 3 calls. When the budget window is tight, it costs 1: the first draft that passes wins. The window is tight after a failed call of the story pool in the last window, or after 5 calls of the story pool in the last window. The bridge admits 10 in each window, and it says the length of the window when it starts.

In 5.6, replace the bullet "A budget limits the use" with:

> - **A budget** limits the use. The bridge has two pools, each with 10 calls in a window: the story pool (the narrator, the chronicle, quests, drafts, and `/lore`) and the talk pool (the turns of a talk). One pool never takes a call or a slot of the other. The window depends on the model:
>   - a local model: 1 minute for each pool. It costs no money, so the limit only stops a loop;
>   - Claude through your login: 10 minutes for the story pool and 5 minutes for the talk pool. Each call counts against your plan;
>   - a hosted model that you pay for (later): the windows of Claude, and a daily cap in money that you set.
>
>   A chapter of the chronicle costs at most 3 calls, and 1 when the story pool is tight (3.3). A talk of 8 turns fits in one window of the talk pool.

In 5.11, after "Your own words go as you typed them", add:

> A reply that the model offers in the talk window (3.5) never holds a name that only you typed. The code drops it.

In 5.14, change the Talk row of the table:

> | Talk | the events behind the NPC, the hero entries about it, the `learned` rows of its passages, the rows behind each memory of the first turn (3.5), and the calls of the earlier turns of its conversation |
