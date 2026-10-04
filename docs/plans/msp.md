# Plan: MSP parity

Status: built, 2026-10-03. The user approved draft 1. This draft is the review of draft 1 against MSP and the code, and the final design. Every part of section 5 is built, and its rules are in `GAMEPLAY.md` 3.7 and 3.7.1.

The Mary Sue Protocol (MSP) lets roleplay addons share character profiles: Total RP 3 (TRP3), MyRolePlay (MRP), and XRP. With this plan, the Hero sheet of Timeways speaks MSP. A player with only Timeways can share a profile with them, and read theirs.

## 1. What MSP is today

The research read the source of each part, at these versions:

- **LibMSP v32** (2023-05-04), CC0 1.0. MRP 12.1.0.657 ships this exact file, and TRP3 and XRP ship it too.
- **Chomp v36 to v38**, an ISC-style license that requires its notice in every copy. LibMSP sends every message through Chomp.
- **MRP 12.1.0.657** (WoWInterface, 2026-08-10), GPLv3.

The old MSP of 2011 (prefix `MSP`, `\1` between commands, counters as versions, parts marked `MSP\1`, `MSP\2`, `MSP\3`) is dead. LibMSP left it in 2020, and drops a message in the old format. The MSP of today:

- **Prefix** `MSP2`, as whispers only.
- **Parts.** Chomp splits a message into parts of at most 255 bytes. Each part starts with a header of 12 hex digits: `%03X%03X%03X%03X`, for the flags, the session, the part number from 1, and the number of parts. The flags are always `00A` today (Chomp v16 and codec 2). A part with no header comes from an old client, and LibMSP drops it.
- **Logged and unlogged.** A reply with field data goes with `C_ChatInfo.SendAddonMessageLogged`, and arrives as `CHAT_MSG_ADDON_LOGGED`. Its text escapes `~`, `|`, `\`, and a line break as `~XX` (two hex digits). A request and a "not changed" reply go unlogged, with no escapes. **LibMSP keeps field data only from a logged message.** So without the logged API, no TRP3, MRP, or XRP player sees a single field from us.
- **Commands.** One message holds commands, separated by a backtick (`` ` ``). A command is `<action><field><version>[:<value>]`, with two capital letters as the field:
  - `?NA`: a request with no version. `?DE1A2B3C4D`: a request with the version that the asker has.
  - `NA1A2B3C4D:Mary Sue`: a field, its version, and its value. A bare `NA` is an empty field.
  - `!DE1A2B3C4D`: "not changed": the asker has this version already.
- **Versions** are the CRC32C of the value, as upper-case hex with no leading zeros. An empty value has no version.
- **The tooltip set** (`TT`): VP, VA, NA, NH, NI, NT, RA, CU, FR, FC, in this order. A request for any of these, or for RC, CO, IC, PX, PN, becomes a request for `TT`. The reply is each field as `NA:value` (no version) or bare `NA`, then `` `TT<version> ``, with the version of the text before it.
- **Limits of LibMSP.** It asks a player for a field again only after 30 seconds. A player who never answered gets a new probe only after 300 seconds. It ignores a second request for the same field within 5 seconds. ChatThrottleLib sends the parts at the lowest priority. The protocol sets no size limit on a field.
- **The shared global.** LibMSP keeps everything in the global `msp`: `msp.my` holds the player's own fields, and `msp.char` the fields of others. Each roleplay addon sets `msp_RPAddOn` to its name ("Total RP 3", "MyRolePlay", "XRP"), and refuses to run its MSP part when another one set it first.
- **Fields of the game.** LibMSP also sends VP (the protocol, `3`), VA (`Addon/Version`), and the game facts GU, GC, GR, GS, GF, and TR.

## 2. Review of draft 1

Each point names what draft 1 said, what was wrong or open, and what this draft decides.

1. **Embed LibMSP? No.** Its license allows it (CC0). But LibMSP needs Chomp, ChatThrottleLib, LibStub, and CallbackHandler, four libraries in the style of another project, with Battle.net code and some 20 client calls that Timeways does not use. It also creates the global `msp`, which tells every roleplay addon that an MSP addon is present. Timeways writes a small MSP of its own in two files: `MspWire.lua` (pure functions: parts, escapes, commands, versions) and `Msp.lua` (the channel). It never creates the global `msp`.
2. **Draft 1 did not know about the logged API.** Field data must go logged, or nobody keeps it. So Timeways uses two new client names: `C_ChatInfo.SendAddonMessageLogged` and the event `CHAT_MSG_ADDON_LOGGED`. The pinned client has both. The API gate lists every name that the addon uses, so `addon/tests/api.lua` and `api-signatures.lua` gain these names. The task said to keep `api.lua` unchanged. That is not possible together with a working MSP. The gate script writes the new files, and CI checks that they match.
3. **Game fields.** Timeways also sends GU, GC, GR, GS, and GF, as LibMSP does, so that a TRP3 or MRP profile of a Timeways player shows the right race and class. GR, GS, and GF need `UnitRace`, `UnitSex`, and `UnitFactionGroup`, three more names for the gate.
4. **Rule 6 of `GAMEPLAY.md`** says that only players with Timeways take part, and that nobody else gets a message. MSP talks to players of other addons. The switch is the player's own choice, as rule 5 allows for a message that you send yourself. So rule 6 gets one exception: with the switch on, Timeways answers MSP requests and asks for MSP profiles. With the switch off, Timeways sends no MSP message at all, and answers none. Draft 1 let Timeways read others with the switch off. That broke rule 6, so reading also needs the switch.
5. **Origin as HB (birthplace).** Draft 1 kept the limit of 1000 characters. Every roleplay addon shows HB on one short line. The question "Where is your character from?" asks for a place. So `origin` holds at most 200 characters. The question on screen does not change.
6. **The six roleplay fields need sizes,** and draft 1 gave none. MSP has no limits, so these are ours. They keep the editor, the journal, and a reply small:

   | Field | MSP | Characters | Bytes |
   |---|---|---|---|
   | origin | HB | 200 | 240 |
   | background | HI | 1000 | 1200 |
   | goal, bond, flaw, traits | none | 1000 | 1200 |
   | name | NA | 100 | 120 |
   | title | NT | 100 | 120 |
   | currently | CU | 200 | 240 |
   | appearance | DE | 1000 | 1200 |
   | age | AG | 100 | 120 |
   | motto | MO | 200 | 240 |

7. **The journal could not hold the new sheet.** Today the six fields go together on the first page, and each one fits a sixth of a slot. Twelve fields at these sizes do not fit one page. So the fields of the sheet now come in pages like the entries: each field is an item of its own, and a long sheet spreads over two pages. The check "a text also fits a sixth of a slot" goes away, because no field needs it now. The relay is not involved: a page already holds any item that fits.
8. **Stored names.** Nothing is live, so a rename needs no migration. But the six questions on screen use the words origin and background, and so do the prompts. The MSP code word is the only place that needs HB or HI. So the stored names stay, and one table in `Msp.lua` maps each Timeways field to its MSP field. The new fields get plain names: `name`, `title`, `currently`, `appearance`, `age`, `motto`. "Appearance" is the word on screen for DE, because "description" says nothing to a player.
9. **The model never sees `name` or `title`.** The name is the name of the character in the game unless you set one, and most roleplay names start with it. Rule 5 keeps the name of a real player from every model (5.11). So the portrait of the narrator leaves out `name` and `title`. The other ten fields go in, as the six do today.
10. **Goal, bond, flaw, and traits stay private.** MSP has no field for them. Putting them into HI or DE would change the text that the player wrote for those fields, and TRP3 shows each field in its own box. Notes and stories about you never go out either.
11. **Import: only the six roleplay fields.** Draft 1 imported "the profile". HB and HI of TRP3 would then write over the player's answers to the first two questions at each login, and fight each edit. So the import takes NA, NT, CU, DE, AG, and MO into the Roleplay Profile, and leaves the six questions to the player.
12. **An import must never loop.** An imported text that the desktop refuses would come back different in each journal, and the addon would send it again each time. So the import cleans each text first: it takes out color codes and TRP3 tags (`{...}`), makes a line break a space, and cuts the text to the limit of its field. And it sends each text at most once in a session.
13. **Read others: only the name and the title, in the tooltip.** The value: a player with only Timeways sees the roleplay name of a TRP3 or MRP player, as those players see each other. The cost is one line in the tooltip and one small request. The text comes from another player, so the line takes out every escape (`|`), and cuts the name and the title. The cache lives in memory, at most 200 players, and no prompt and no file ever gets it.
14. **The saved copy** of the own profile exists only while the switch is on, and turning the switch off clears it. It holds only the eight fields that MSP shares, so it holds nothing that a request would not give anyway.
15. **Rate limits.** The documents of the client do not say whether the game counts addon messages for each prefix or for all of them. So Timeways shares one budget between quests and MSP: 8 parts in a burst, and one each second after it. MSP takes at most half of it: 4 parts in a burst, and one each 2 seconds after it. A part of a quest waits for nobody. MSP also keeps the limits of LibMSP: 30 seconds per field, 300 seconds per player who never answered, and 5 seconds per request that came again. A sender gets 24 parts in a burst and one each 2 seconds, as in player quests.
16. **Battle.net.** Chomp sends to a Battle.net friend on another realm or faction through Battle.net. Timeways does not, because the gate lists no Battle.net API, and the friend's request comes in game anyway when you stand next to them. Open: a request that comes through Battle.net gets no answer.

## 3. The final design

### 3.1 The fields

`crates/story/src/hero.rs` holds 12 fields in the order of the page: the six questions, then `name`, `title`, `currently`, `appearance`, `age`, `motto`. Each field has a limit in characters and in bytes (table in point 6). An entry of your own lore keeps 1000 characters and 1200 bytes. `hero_set` takes any of the 12 fields. The journal sends each field of the sheet as an item of the list, so a sheet can fill more than one page.

### 3.2 The Hero page

- The list keeps "About your hero" with the six questions. Under them, one more item: **Roleplay Profile**, with a short state: "Not shared", "Shared", or "From MyRolePlay" (the name of the other addon).
- The section is folded by default. A click on Roleplay Profile opens it: its six fields show under it in the list, and the parchment shows the state and the switch. A click on a question folds it again.
- A field of the profile opens like a question: its question, its answer, and Edit. The name shows the name of the game until you set one.
- **The switch** is a button on the parchment: "Share" when it is off, and "Stop sharing" when it is on. The line above it says what other players see. The switch is off by default, and stays off for a new character.
- With another roleplay addon, the six fields show its text with "From MyRolePlay" (or its name), and have no Edit. The switch does not show: that addon shares the profile.

Copy (UI copy rules of `CLAUDE.md`):

| Where | Text |
|---|---|
| List item | Roleplay Profile |
| Detail, off | Not shared |
| Detail, on | Shared |
| Detail, another addon | From MyRolePlay |
| Help, off | Players with roleplay addons like Total RP 3 can see this profile if you share it. |
| Help, on | Players with roleplay addons like Total RP 3 can see this profile. |
| What goes out | They see your name, title, currently, appearance, age, motto, where you're from, and your background. Your other answers and your notes stay private. |
| Help, another addon | MyRolePlay shares your profile. Change it there. |
| Buttons | Share, Stop sharing, Edit |
| Questions | What's your character's name? / What title does your character go by? / What is your character doing right now? / What does your character look like? / How old is your character? / What's your character's motto? |

### 3.3 MSP in the addon

- `MspWire.lua`, pure functions: the CRC32C version, the parts with their header, the escapes of a logged message, a request, a reply, the tooltip block, and the parse of a message into commands. A part with no header, a broken header, more than 64 parts, or a broadcast flag is dropped.
- `Msp.lua`, the channel:
  - **Alone** means: no global `msp`. Timeways checks it at each event, so an addon that loads later still wins.
  - **Answer:** with the switch on and alone, a request from a player gets the fields from the saved copy. A field that the asker has already gets `!`. A field that Timeways does not share is empty.
  - **Ask:** with the switch on and alone, mousing over or targeting another player asks for `TT` once each 30 seconds. The name and the title go into the cache in memory, and the tooltip of that player shows them.
  - **Import:** with another roleplay addon, after each journal, the six fields of `msp.my` go to the desktop as hero edits when they differ from the sheet.
- The saved variables `TimewaysProfile` (each character) hold the switch and the copy of the eight shared fields. Any addon can write them, so the addon checks each value when it reads them.

### 3.4 Tests

- `crates/story/tests/hero.rs`: the limits of each field, the new fields, the portrait without the name and the title.
- `crates/story/tests/journal.rs`: a long sheet spreads over pages.
- `crates/story/tests/properties.rs`: any play of hero edits gives journal pages that each fit one reply.
- `crates/addon-tests/tests/msp.rs`: the wire (known CRC32C values, headers, escapes, parse), an exchange between two Timeways players with the two-player harness, the switch, the rate limits, the saved copy, the tooltip, and the import from a fake TRP3 `msp` table.
- `fuzz/fuzz_targets/msp_wire.rs`: random parts from four senders through `MspWire.lua`. A broken part never raises a Lua error, and a parsed command holds only a field of two capital letters.

## 4. The MyRolePlay check

The user asked for a check in CI that keeps Timeways compatible with MRP.

- **The pin.** MRP has no source repository for its current releases (the GitHub copies stop in 2017 or hold only translations). WoWInterface keeps each release as a file. CI downloads MRP 12.1.0.657 from `https://www.wowinterface.com/downloads/getfile.php?id=4990&aid=170341` and checks its SHA-256: `ff656b746c818f1c15d2e16bc0124361f711a80de408e8fc1d7ab3d335a53c92`. `scripts/fetch-mrp.sh` holds the pin, as `ci.yml` holds the relay commit. To move the pin, change the URL and the hash together.
- **The license.** MRP is GPLv3, so its source never goes into this repo. CI fetches it into `target/mrp` at the pin. The MSP code that MRP ships (LibMSP v32, Chomp v36, ChatThrottleLib, CallbackHandler, LibStub) runs only in the test.
- **The run.** The test loads the MSP libraries of MRP into a Lua 5.1 state of its own, with the fake WoW API of `addon/tests/wow.lua` and a small shim (`crates/addon-tests/tests/mrp/shim.lua`) for the client calls that only these libraries use. The shim lives with the test, so the API gate never reads it. Then it fills `msp.my` as `Profile.lua` of MRP does (`mrp:SetCurrentProfile`), with `VA` set to `MyRolePlay/12.1.0.657`, and calls `msp:Update()`. The UI of MRP does not load: it needs hundreds of frames and the art of the game, and it takes no part in MSP.
- **What it checks**, both ways, through the harness that carries the messages:
  - MRP asks Timeways for NA, NT, CU, DE, AG, MO, HB, and HI, and `msp.char` of MRP then holds the right value of each.
  - Timeways asks MRP for the same fields, and its cache holds the right value of each.
  - Versions: MRP asks again with the versions that it has, and gets `!` for an unchanged field and the new text for a changed one.
  - The import: with the MRP global and `msp_RPAddOn = "MyRolePlay"` in the same Lua state as Timeways, the journal sends MRP's six fields to the desktop.
- **The run in CI.** These tests need the download, so each one is `#[ignore]` with the reason. A job `mrp` in `ci.yml` runs `scripts/fetch-mrp.sh`, then `cargo test -p addon-tests --test msp_mrp -- --ignored`. When MRP changes its protocol, the move of the pin shows the break.
- **TRP3** ships the same LibMSP and Chomp, so the MRP check covers its wire too. Its own MSP module needs the whole of TRP3 to load, so a TRP3 check costs much more, and this plan leaves it out.

## 5. Build order

1. This plan.
2. The desktop: fields, limits, the journal pages, tests.
3. The addon: the fields and the Hero page.
4. The addon: `MspWire.lua`, its tests, its fuzz target.
5. The addon: `Msp.lua`, the switch, the saved copy, the tooltip, the import, tests.
6. The MRP check and its CI job.
7. `GAMEPLAY.md` and the README.

## 6. Open

- A request through Battle.net gets no answer (point 16).
- The fields that only TRP3 or MRP know (eye color, height, pronouns, and more) stay out. A later plan can add them.
- How TRP3 shows a long `HB` that a player wrote before this plan: no player has one, because nothing is live.
