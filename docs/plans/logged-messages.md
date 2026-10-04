# Plan: reportable player text

Status: draft 1, 2026-10-03. When a part is built, its rules move into `GAMEPLAY.md` 4.7 and 4.8, and this plan marks the part as done.

## 1. Goal

A player writes text for another player: a quest offer and a story. A harasser can use this text to abuse a player. Blizzard support reads a reported chat message, but it can't read a normal addon message. So the text that a player wrote goes on the logged addon channel, `C_ChatInfo.SendAddonMessageLogged`. Blizzard logs that channel, and support reads a reported message there like a whisper.

Total RP 3 made the same change after harassment: [Reporting RP profiles](https://github.com/Total-RP/Total-RP-3/wiki/Reporting-RP-profiles-that-infringe-on-Blizzard's-Terms-of-Service).

## 2. The API in WoW Forever 1.60.1.70009

The client has the function and the event. The API documentation of the pinned client (`ChatInfoDocumentation.lua`, wow-ui-source `bd2470ae`) lists both. The resources of BlizzardInterfaceResources list both too.

- `C_ChatInfo.SendAddonMessageLogged(prefix, message, chatType, target)`. The arguments are the same as for `SendAddonMessage`. The documentation says: "Intended for plain text payloads; logged and throttled."
- The result is `Enum.SendAddonMessageResult`, as for `SendAddonMessage`, but the documentation marks it as nilable. The addon reads `nil` as `Success`, because a client without a result gives no reason to wait.
- The receiver gets `CHAT_MSG_ADDON_LOGGED`, with the same payload as `CHAT_MSG_ADDON`: prefix, text, channel, sender, and more. The prefix is the same registered prefix.
- Channels: the same chat types as `SendAddonMessage`. Timeways uses `WHISPER`, `PARTY`, `RAID`, `INSTANCE_CHAT`, and `GUILD`. Chomp (the library of Total RP 3 and MSP) sends logged whispers and logged group messages.
- Size: at most 255 bytes, as for `SendAddonMessage`.
- Throttle: the documentation says "throttled", with no numbers. The addon uses one rate limit for both channels (4.7), so the logged channel never sends more than the normal one did.

### 2.1 The bytes that the logged channel refuses

The normal channel takes every byte but NUL. The logged channel takes only plain text. Blizzard does not list the rules. Chomp checks them before each logged message (`Chomp.CheckLoggedContents` and `EncodeQuotedPrintable`), from tests in the game:

| Refused | Why it matters to Timeways |
|---|---|
| Control characters: bytes 0 to 31, and 127 | The wire refuses them already. |
| `\|` | The wire refuses it already, because it starts a WoW escape. |
| `\` | A player can type it. The wire did not escape it. |
| Text that is not valid UTF-8 | A cut between parts can split a letter such as "é". Text from another addon can be broken. |
| U+FFFE and U+FFFF | Not letters. |
| 卍 and 卐 (U+534D, U+5350) | Blizzard refuses these signs. |

## 3. Design

### 3.1 Which types go logged

A type goes logged when it carries text that a player wrote, or text that the receiver shows and that is the sender's word.

| Type | Channel | Why |
|---|---|---|
| `offer` | logged | The title, the text, the reward, and each step target are typed by the giver. An `other` step is a line as the giver wrote it. |
| `story` | logged | The whole story is typed by the author. |
| `step` | logged | The zone is the doer's word, and the giver's turn-in card shows it. A changed addon can put any text there. |
| `turnin` | logged | Each claim carries a zone, as `step` does. |
| `hello`, `here`, `accept`, `decline`, `block`, `cancel`, `result`, `story_accept`, `story_decline` | normal | No free text: only a type, an id, or a verdict. |

One function decides the channel for a type: `TaskWire.Log(type)`. The send path and the receive path both ask it.

### 3.2 The receiver

- The addon listens to `CHAT_MSG_ADDON` and `CHAT_MSG_ADDON_LOGGED`. Both events go to one function, `TaskChannel.Received`, with the channel that the part came on.
- Each channel has its own collector of parts. So a message never joins parts from both channels: a sender can't put half of a text on the normal channel.
- After the decode, a type that goes logged and that came on the normal channel is dropped. Otherwise a changed addon could skip the log.
- A type that goes normal is taken on both channels. The log does no harm to it.
- Every other check of 4.7 stays: the prefix, the sender, the rate limit of the peer, the parts, the wire, and who can send which type.

### 3.3 The wire

- `\` gets an escape, as `%` and `;` do: `%5C`. The receiver unescapes any `%XX`, so an older addon reads the new escape.
- A text of a peer must be valid UTF-8, with no U+FFFE, no U+FFFF, and no refused sign. `TaskWire.IsCleanText` checks this, so the form, the saved files, and the receiver all use the same rule.
- The form takes the refused signs out of a typed text, as it takes out `|`.
- `TaskChunks.Split` cuts a message only between two letters, never inside one. A part can then hold fewer than 244 bytes of text, so the count of parts comes after the cut. The limit of 16 parts stays.
- The escape `%XX` uses only ASCII. A cut inside an escape is harmless, because the receiver joins the parts before it unescapes.

### 3.4 UI copy

- On the quest form, under "Send to": "Like chat, Blizzard can read what you send."
- In the usage line of `/story`: the same fact, short.
- On a quest that you got, and when `/story` shows a story: "To report abuse, open Customer Support in the game menu."

### 3.5 Privacy

Rule 5 of section 2 still holds. A logged message goes to the same player as before, through the same Blizzard servers. Only Blizzard support can read it, and only after a report. No model sees it.

## 4. Tests

- An offer goes on the logged channel. A story goes on the logged channel. A step and a turn-in go on the logged channel.
- A control message stays on the normal channel.
- An offer that comes on the normal channel is dropped. A story that comes on the normal channel is dropped.
- The parts of one message on two channels never join.
- An offer with every byte that a text can hold comes through the logged channel as it went in.
- A cut never splits a letter.
- The fake `SendAddonMessageLogged` refuses each byte of 2.1, so a test fails when the addon sends one.
- The two-player harness carries each part to the event of its channel, so every test of player quests and stories runs over the real channels.
- The fuzz target `task_play` hears raw parts and messages on both channels, and checks that a logged type never comes in on the normal channel.

## 5. Open

- An in-game test with two clients: an offer and a story arrive on `CHAT_MSG_ADDON_LOGGED`, in a whisper, a party, and a guild.
- An in-game test of the byte rules of 2.1 in WoW Forever. The rules come from Chomp, which tests them on retail and Classic.
- The throttle of the logged channel in numbers. If it is lower than the normal one, the rate limit of 4.7 goes down.
