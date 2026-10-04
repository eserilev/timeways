# Plan: the narrator frame

Status: spec only, 2026-10-03. Nothing is built, and nothing gets built from this text alone.

**This feature needs design iteration before any build.** The user approved a direction, not a design. Before any code, we make mockups, show them to the user, and change them over several rounds. The sizes, the place on the screen, the timing, and the camera in this plan are first guesses for the first mockup. The build plan (section 14) applies only after the user approves one design. When a part is built, its rules move into `GAMEPLAY.md` 3.2 (section 15), and this plan marks the part as done.

This plan shares the look of the talk window (`docs/plans/talk-window.md` 5.1): the backdrop, the parchment, the fonts, the ink, and the portrait box. This plan does not repeat that spec. It only says where the narrator frame differs.

## 1. Goal

Today a narrator line goes to the chat (`Narrator.lua`), where it gets lost between the chat of other players and the loot lines. The narrator speaks at most 3 times in an hour (3.2), so each line counts.

The narrator frame shows the line in a small frame on the screen, the way the game shows a quest NPC that talks:

- a 3D portrait of a bronze dragon, in the game's own model frame;
- the text, which appears at reading pace;
- a fade after a few seconds.

The line also goes to the chat, as today, so the player can scroll back.

## 2. What does not change

- **The narrator itself.** The story program writes the same lines, with the same budget (3 lines in one hour), the same moments, and the same checks (3.2, 3.2.1). The frame only changes how a line shows. It never asks for a line, and it never makes the narrator speak more.
- **The protocol.** A reply carries `narrator` as today (`Core.lua`, `ns.OnReply`). No new line goes to the desktop. No relay change, so the Gnomish Relay session gets no message for this plan.
- **The chat line.** `Narrator.Say` still prints `Narrator: <line>` at once, also in combat, also with the frame on.
- **The rule of the name.** The narrator is never named (3.2). The frame has no title, no name plate, and no tooltip. No text says "Anachronos", "bronze", or "dragon". Only the model shows.

## 3. The portrait: the model of Anachronos

The user chose the model of Anachronos, the bronze dragon who guards the Caverns of Time in Tanaris (Classic NPC 15192). No art ships with the addon: the model is in the game files on each computer.

### 3.1 The display IDs

| Use | NPC | Display ID | Model file | Skin |
|---|---|---|---|---|
| First choice | Anachronos, 15192 | **2719** | `creature/dragon/dragon.m2` (file 123459, model data 203) | `creature/dragon/dragonskin1bronze.blp` (file 123472) |
| Fallback 1 | Chronalis, 8197 | **6370** | `creature/drake/drake.m2` (file 123636, model data 571), scale 1.3 | `creature/drake/drakeskinbronze1.blp` (file 123649) |
| Fallback 2 | Tick, 8198 | **8318** | the same drake model (571), scale 1.15 | the same bronze drake skin |

Sources, checked on 2026-10-03:

- Wowhead Classic, [NPC 15192](https://www.wowhead.com/classic/npc=15192/anachronos): the "View in 3D" button carries `data-mv-display-id="2719"`.
- Wowhead Classic, [NPC 8197](https://www.wowhead.com/classic/npc=8197) (Chronalis): display 6370. [NPC 8198](https://www.wowhead.com/classic/npc=8198) (Tick, an elite of level 52 in Tanaris): display 8318.
- wago.tools, `CreatureDisplayInfo` of the product `wow_classic_era`: row 2719 has `ModelID` 203 and the skin file 123472. `CreatureModelData` row 203 has `FileDataID` 123459. Rows 6370 and 8318 have `ModelID` 571.
- wago.tools file search: 123459 is `creature/dragon/dragon.m2`, 123636 is `creature/drake/drake.m2`, 123472 is `dragonskin1bronze.blp`, and 123649 is `drakeskinbronze1.blp`.

Display 2719 is a bronze dragon skin on the common dragon model. A player who knows Anachronos can see the likeness. Nothing in the frame confirms it.

### 3.2 How to confirm the display in WoW: Forever (1.60.1.70009)

The API dump of `target/wow-api` (KethoDoc) holds functions and events, not game data. So the API gate cannot confirm a display ID. A person checks it in the game once, before the build:

1. Log in on the Forever client, out of combat.
2. Type this line in the chat:

   ```
   /run local m=CreateFrame("PlayerModel",nil,UIParent) m:SetSize(200,200) m:SetPoint("CENTER") m:SetDisplayInfo(2719) C_Timer.After(2,function() print(m:GetDisplayInfo(), m:GetModelFileID()) end)
   ```

3. Look at the box in the middle of the screen. A bronze dragon must show.
4. Read the chat. It must print `2719 123459`.
5. Do steps 2 to 4 again with 6370 and 8318. Each must print its display ID and `123636`.
6. Type `/reload` to remove the test boxes.

The build also checks at run time (3.3), so a later client patch that drops a display cannot break the frame.

### 3.3 The fallback chain

The frame tries each step in order. It moves to the next step when the model file is missing (`GetModelFileID()` returns `nil` or 0) at `OnModelLoaded`, or when no `OnModelLoaded` fires within 2 seconds.

1. Display 2719 (Anachronos).
2. Display 6370 (Chronalis).
3. Display 8318 (Tick).
4. A static portrait: a 2D texture from display 2719 with `SetPortraitTextureFromCreatureDisplayID`. This function is in the client dump (`GlobalAPI.lua`) but not in `addon/tests/api.lua`. It goes through the gate first (section 10).
5. No portrait. The text takes the full width of the frame.

The frame remembers the first step that works for the session, so it does the chain once.

## 4. The frame

### 4.1 Layout

```
+--------------------------------------------------------+
| +----------+                                           |
| |          |  Our hero falls for the third time to the |
| | (bronze  |  same murloc. The murloc does not know    |
| |  dragon, |  it is a legend now.                      |
| |   3D)    |                                           |
| +----------+                                       ... |
+--------------------------------------------------------+
```

- **Size:** 440 wide, the width of the talk window. The height follows the text: at least 112, at most 168. A line of 300 characters (the limit of 3.2) fits in about 7 lines of `QuestFont` at a text width of 316.
- **Portrait:** a `PlayerModel` of 88 by 88 at the left, the size of the talk window portrait, on the portrait background of the talk window.
- **Text:** `QuestFont`, text ink, on parchment, left of the portrait edge plus 12. No heading and no name.
- **The "..." mark:** a small faded mark at the bottom right while the text still appears. It tells the player that a click shows all. It goes away when the full text shows.
- **No buttons.** No close button, no "Okay". A click and Escape close it (4.4).

### 4.2 Place, moving, and scale

- **Place:** `BOTTOM` of `UIParent`, x 0, y 240: above the action bars and the cast bar of the default UI. This is a first guess for the mockup (question 1).
- **Movable:** yes. A drag with the left button moves it. The frame has a name (`TimewaysNarratorFrame`), so the client keeps the place in its layout cache, as it does for the journal. A drag never counts as a click.
- **Clamped:** `SetClampedToScreen(true)`, so a drag never loses it off screen.
- **Scale:** the frame is a child of `UIParent`, so it follows the UI scale of the game. No scale setting of its own in the first build.
- **Strata:** `LOW`, not top-level. Every window of the game and of Timeways shows above it. It never covers the map, the bags, or the talk window.
- **No managed position.** The frame does not join `AlertFrame`, Edit Mode, or the managed frames of `UIParent`. So it never moves another frame, and no other frame moves it.

### 4.3 Clicks

The frame takes the mouse only inside its own box, and only while it shows.

- While it is hidden, it is `Hide()`, not alpha 0. A frame at alpha 0 still takes clicks. That is a trap.
- When the fade starts (4.6), the frame calls `EnableMouse(false)`. A click on a fading frame goes to the world.
- No layer covers the screen. Clicks outside the 440-wide box always reach the game.

### 4.4 Close

| Action | Text still appears | Full text shows |
|---|---|---|
| Left click (no drag) | shows the full text at once | closes the frame |
| Right click | closes the frame | closes the frame |
| Escape | closes the frame | closes the frame |

- Right click closes at once, as the talking head of the game does.
- Escape works through `UISpecialFrames`. An Escape also closes the other open frames of that list, such as the journal. The game does this for every frame of the list.
- A close drops the line from the frame. The chat still has it.
- **A trap:** the hide of combat (section 7) also fires `OnHide`, as Escape does. The frame keeps a state, `Showing`, `Away`, or `Closed`, as the talk window does (talk-window.md 5.3). Only a hide in `Showing` is a close.

## 5. The model framing

| Setting | First value | API |
|---|---|---|
| Camera | the portrait camera of the model, zoomed in on the head | `SetPortraitZoom(1)` |
| Distance | a small step back, so the horns fit | `SetCamDistanceScale(1.1)` |
| Facing | turned a little toward the text on the right | `SetRotation(0.35)` (radians) |
| Idle | stand | `SetAnimation(0)` |
| Speak | the talk emote, while the text appears | `SetAnimation(60)` when `HasAnimation(60)` is true |
| Keep | keep the model while hidden, so it does not load again | `SetKeepModelOnHide(true)` |

- The values are first guesses. The mockup round tunes them in the game (question 3).
- Animation 0 is Stand, and 60 is EmoteTalk, in `AnimationData` (wago.tools). Nobody knows yet if the old dragon model has EmoteTalk. With no animation 60, the dragon only stands. The frame never plays a roar, an attack, or a flight animation: they are loud and they look like combat.
- When the full text shows, the model goes back to Stand.
- The game's talking head (`TalkingHeadUI.lua` in the client) uses a `PlayerModel` with `SetDisplayInfo` and `SetAnimation` in the same way. That code is the reference.
- A 2D fallback (3.3, step 4) has no camera and no animation.

## 6. The text, the fade, and the sound

### 6.1 Reveal

- The frame sets the full text at once, then shows it from left to right with `FontString:SetAlphaGradient(start, length)`. The quest frame of the game uses the same call for its text (`QuestFrame.lua`, `QUEST_DESCRIPTION_GRADIENT_LENGTH` = 30).
- The text never reflows while it appears, because the full text is set from the start. The code never cuts a string, so it never cuts a UTF-8 character or a `|` escape in half.
- **Speed:** 30 characters each second. Adults read about 20 to 25 characters each second, so the text stays a little ahead of the eye. The quest frame of the game uses 70 (`QUEST_DESCRIPTION_GRADIENT_CPS`), which is too fast to feel like speech. A line of 300 characters takes 10 seconds.
- **Gradient length:** 30 characters, as the game.
- **Click to show all:** a left click sets the gradient past the end (4.4).

### 6.2 Hold and fade

| Phase | Time |
|---|---|
| Fade in | 0.25 s, frame alpha from 0 to 1 |
| Reveal | the length of the line / 30 characters each second |
| Hold | 5 s + 1 s for each 40 characters, at most 12 s |
| Fade out | 1 s, frame alpha from 1 to 0, then `Hide()` |

- A line of 80 characters: about 2.7 s of reveal and 7 s of hold. A line of 300 characters: 10 s of reveal and 12 s of hold.
- **The mouse over the frame stops the clock** of the hold (`IsMouseOver`). The hold starts again when the mouse leaves. So a player who reads slowly can keep the line.
- One `OnUpdate` script drives the reveal, the hold, and the fade with the elapsed time. One clock is simpler to test than many timers. The script runs only while the frame shows.

### 6.3 Sound

- **No sound in the first build.** The narrator speaks at a big moment, which often has a sound of its own (a level up, a quest done). A second sound adds noise.
- **Never a voice.** No text-to-speech and no recorded voice for now. A voice comes later (Gnomish Relay SPEC 13.3), with its own plan.
- The mockup round can try one soft cue of the game, such as `SOUNDKIT.IG_QUEST_LIST_OPEN`, on the sound effects channel (question 5).

## 7. Combat

Story after the action (rule 3). The frame never shows in combat.

- **A line in combat waits.** When a line comes while `InCombatLockdown()` is true, the chat line prints at once, and the frame line waits in the queue (section 8).
- **After combat:** at `PLAYER_REGEN_ENABLED`, the frame waits 3 seconds, then shows the first waiting line, if the player is still out of combat. The 3 seconds stop a line from showing between two pulls that come close together.
- **A stale line drops.** A line that waited more than 5 minutes drops from the queue. The moment is over, and the chat has the line. A dungeon pull is rarely longer than 5 minutes, so most lines still show.
- **Combat while a line shows:** at `PLAYER_REGEN_DISABLED`, the frame hides at once (state `Away`). When the full text did not show yet, the line goes back to the front of the queue, with its first arrival time. When the full text showed, the line is done.
- Why hold and not drop: the lines are rare (3 in one hour), and each one is about the player's own history. Why a limit of 5 minutes: a line about a kill that shows 20 minutes later reads like a bug.

## 8. Two lines close together

The narrator speaks at most once for each batch (3.2), and at most 3 times in one hour. But two lines can still come close together: a reply to `/lore` carries a line, and an `events_seen` reply comes right after it. And the lines of a long fight wait together.

- **One line at a time.** The frame never shows two lines, and never stacks two frames.
- **A queue of at most 3 lines,** oldest first. A fourth line drops the oldest waiting line from the queue (not from the chat).
- **A new line while one shows:** the line on the frame ends its reveal, then holds for 3 seconds only (not the full hold), then fades. Then the next line fades in.
- **A same line twice:** when a line equals the line on the frame or a waiting line, it does not go into the queue again.
- **The talk window:** while the talk window is open (talk-window.md), a narrator line waits in the queue. Two speakers on the screen at once compete for the eye. The 5-minute limit applies.
- **A loading screen:** the frame hides, and the line on it is done.

## 9. The setting

A player can turn the frame off. Then the narrator line goes to the chat only, as today.

- **Where:** the game's options, under AddOns, in a Timeways page with the `Settings` API of the client (`Settings.RegisterVerticalLayoutCategory`, `Settings.RegisterAddOnSetting`, `Settings.CreateCheckbox`, `Settings.RegisterAddOnCategory`). Timeways has no options page today, so this adds the first one. These names are in the client UI (`Blizzard_Settings_Shared`) but not in `addon/tests/api.lua`. They go through the gate first (section 10). Question 6 has the alternatives.
- **The saved value:** `TimewaysDB.narratorShows`, `"screen"` (the default) or `"chat"`. A string with two values, not a `bool` (CLAUDE.md: enums in place of bool flags). An unknown value reads as `"screen"`.
- **A change while a line shows:** a switch to `"chat"` hides the frame and clears the queue.

### 9.1 UI copy

The copy follows the UI copy rules of `CLAUDE.md`.

| Where | Copy |
|---|---|
| Checkbox label | Show the narrator on screen |
| Checkbox tooltip | Narrator lines pop up near the bottom of your screen. They always show in chat too. |
| Options page title | Timeways |
| The frame | (the line only: no title, no name, no button text) |
| The chat line (today) | Narrator: <line> |

Bad and good:

| Bad | Good | Why |
|---|---|---|
| Enable narrator frame | Show the narrator on screen | "Frame" is the mechanism. Say what the player gets. |
| Route narrator output to chat only | (the checkbox off) | "Route" and "output" are internal words. |
| Anachronos says: | (no name) | The narrator is never named (3.2). |
| The Keeper of Time speaks... | (no name, no title) | The UI never performs the story voice. |
| Click to continue | (the "..." mark only) | A click shows all, and a player tries a click anyway. |
| Lines are held during combat and shown afterward. | (no tooltip line) | The player sees it work. The rule needs no words. |

## 10. API needs and the gate

Every name that the frame uses goes through the API gate of Gnomish Relay (`wow-api.sh`, README), which checks it against the Forever client. Names that `addon/tests/api.lua` and `api-signatures.lua` have today:

| Need | Name | Where in `api.lua` |
|---|---|---|
| The frame | `CreateFrame`, `UIParent`, `UISpecialFrames` | globals |
| The 3D portrait | `PlayerModel`: `SetDisplayInfo`, `GetDisplayInfo`, `SetPortraitZoom`, `SetCamDistanceScale`, `SetRotation`, `SetAnimation`, `HasAnimation`, `SetKeepModelOnHide` | widgets, `PlayerModel` |
| The model check | `Model`: `GetModelFileID`, `ClearModel` | widgets, `Model` |
| The reveal | `FontString`: `SetAlphaGradient`, `SetText` | widgets, `FontString` |
| The fade and the clicks | `Frame`: `SetAlpha`, `EnableMouse`, `SetMovable`, `SetClampedToScreen`, `SetFrameStrata`, `SetScript`, `IsMouseOver` | widgets |
| Combat | `InCombatLockdown`, the event `PLAYER_REGEN_ENABLED` | globals, `api-signatures.lua` events |
| Timers | `C_Timer.After`, `GetTime` | globals |
| A sound later | `PlaySound`, `SOUNDKIT` | globals |

New names. Each one is in the dump of the client (`target/wow-api`, build 1.60.1.70009), and each one goes into the gate files before use:

| Name | Kind | In the client dump |
|---|---|---|
| `PLAYER_REGEN_DISABLED` | event | `Events.lua` |
| `LOADING_SCREEN_ENABLED` | event | `Events.lua` (the talking head of the game uses it) |
| `SetPortraitTextureFromCreatureDisplayID` | global, only for fallback step 4 | `GlobalAPI.lua` |
| `Settings.RegisterVerticalLayoutCategory`, `Settings.RegisterAddOnSetting`, `Settings.CreateCheckbox`, `Settings.RegisterAddOnCategory` | globals of the client UI | `Blizzard_Settings_Shared` |

Not in the list on purpose:

- `CinematicModel` and `ModelScene` are in `api.lua` too. `PlayerModel` is enough: the game's own talking head uses it, and it is the simplest of the three.
- `SetCreature(15192)` loads a model from an NPC ID, but it needs the NPC in the cache of the client. A player who never met Anachronos has no cache entry. `SetDisplayInfo` needs only the game files.
- `UIFrameFadeOut` is a FrameXML helper, not in `api.lua`. The `OnUpdate` clock (6.2) does the fade with `SetAlpha`.
- `C_TalkingHead` and the game's `TalkingHeadFrame`: the server feeds them. An addon cannot give them a line (alternative E).

## 11. Tests

### 11.1 The Lua harness (`crates/addon-tests`)

The fake game (`addon/tests/wow.lua`) gets:

- a `PlayerModel` widget that records the display, the zoom, the rotation, and the animation, and a test switch for "this display has no model file". The talk window plan adds `PlayerModel` too (talk-window.md 8). The two plans share one fake.
- `SetAlphaGradient` on a font string, which records its start.
- `wow.Advance(seconds)`: it moves `wow.now`, calls the `OnUpdate` script of each shown frame with the elapsed time, and runs each due `C_Timer.After`.
- `wow.combat` exists already. The tests fire `PLAYER_REGEN_DISABLED` and `PLAYER_REGEN_ENABLED`.

A new test file `crates/addon-tests/tests/narrator_frame.rs`. The names are sentences:

- `a_narrator_line_shows_on_the_frame_and_in_the_chat`
- `the_frame_shows_no_name_and_no_title`
- `the_portrait_uses_the_display_of_anachronos`
- `a_missing_display_falls_back_to_the_bronze_drake`
- `with_no_display_the_text_takes_the_full_width`
- `the_text_appears_at_thirty_characters_each_second`
- `a_click_while_the_text_appears_shows_all_of_it`
- `a_click_on_the_full_text_closes_the_frame`
- `a_right_click_closes_the_frame_at_once`
- `escape_closes_the_frame`
- `a_drag_moves_the_frame_and_does_not_close_it`
- `the_frame_fades_after_its_hold`
- `the_hold_grows_with_the_line_up_to_twelve_seconds`
- `the_mouse_over_the_frame_stops_the_fade`
- `a_fading_frame_lets_clicks_through`
- `a_hidden_frame_is_hidden_and_not_only_clear`
- `a_line_in_combat_waits_until_three_seconds_after_combat`
- `a_line_that_waited_five_minutes_drops`
- `combat_hides_the_frame_and_an_unread_line_comes_back`
- `hiding_for_combat_is_not_a_close`
- `two_lines_show_one_after_the_other`
- `the_queue_keeps_at_most_three_lines`
- `the_same_line_twice_shows_once`
- `a_line_waits_while_the_talk_window_is_open`
- `with_chat_only_the_frame_never_shows`
- `an_unknown_setting_reads_as_screen`
- `a_line_with_a_pipe_shows_the_pipe_as_text`
- `the_speak_animation_plays_only_when_the_model_has_it`

Tests of today that stay: the chat tests of `addon.rs` and `link.rs` (`Narrator|r: ...`) keep their lines. The chat line does not change.

### 11.2 Fuzz

`fuzz/fuzz_targets/replies.rs` already sends random replies to `ns.OnReply`, which calls `Narrator.Say`. With the frame, a random `narrator` value also reaches the frame. The target then also runs `wow.Advance` with random steps and random combat events. The frame never raises a Lua error. New seeds in `fuzz/seeds/replies/`: a line of 300 characters, a line of many UTF-8 characters, a line of only `|` marks, and four lines in one reply.

### 11.3 In the game

The harness cannot check what a player sees. A person checks these in the game, on the Forever client, before the frame ships:

1. The display IDs show a bronze dragon (3.2).
2. The camera: the head fits the box, the horns are not cut, and the dragon faces the text.
3. The animations: does the dragon model have animation 60? Does Stand look alive (it breathes)?
4. The reveal looks smooth, and 30 characters each second feels right.
5. The place: the frame covers no action bar, no cast bar, no quest tracker, and no raid frame, at UI scale 0.64, 1.0, and the largest scale, at 1920x1080 and at 1280x720.
6. A drag moves it, and the place stays after `/reload` and after a new login.
7. Clicks next to the frame reach the world: a click on a mob, a loot window under it.
8. Escape closes it, and a second Escape opens the game menu.
9. A pull while a line shows hides it at once, and the line comes back after combat.
10. The frame hides with the UI (Alt+Z) and in a cinematic.
11. The cost: the frame rate with the model on screen, on a low-end computer.

## 12. Design alternatives

### A. A parchment card with the 3D model (this plan)

The look of the talk window and the lore book: the rock backdrop, the parchment, `QuestFont`, and a `PlayerModel` portrait.

- For: one look for all Timeways windows. The parts exist (`LoreBook.lua`, the talk window). Classic in feel.
- Against: a parchment card is heavier than the game's talking head. It reads as a window, not as a voice.

### B. The look of the game's talking head

The client has `TalkingHeadUI.xml` and its atlases (`TalkingHeads-TextBackground`, `TalkingHeads-PortraitBg`, `TalkingHeads-Alliance-PortraitFrame`, `TalkingHeads-Glow-Sheen`) in the dump of 1.60.1.70009. The frame copies the layout (570 by 155, the model 115 by 115) with these atlases.

- For: players know this frame. It reads as "someone speaks" at once.
- Against: it is the look of later expansions, not of Classic. The atlases are in the UI dump, but a person must confirm that their art is in the Forever client files. The frame of the game shows a name, and ours has none, so the layout has a hole.

### C. Text only, in the middle of the screen

A `MessageFrame` line in the middle of the screen, like a boss emote, with a fade. No model.

- For: the smallest build. No model cost. No clicks to handle.
- Against: a 300-character line is too long for a banner. It looks like a boss warning, which is the wrong feeling. No portrait, and the user asked for one.

### D. A static 2D portrait

The card of A, with a round 2D portrait from `SetPortraitTextureFromCreatureDisplayID` in the place of the 3D model.

- For: cheaper on a slow computer. No camera to tune. The look of the unit frames.
- Against: no motion, so it feels less alive. A new name for the gate. It stays as fallback step 4.

### E. The game's own talking head frame (rejected)

Write the line into `TalkingHeadFrame`.

- For: the real thing, with no frame of our own.
- Against: the server feeds it through `C_TalkingHead`, and an addon cannot give it a line. Writing into its regions taints the Blizzard code and fights it at each real talking head. Not possible to do well.

The first mockup round shows A and B side by side, with D as a variant of A.

## 13. Open design questions

1. **The place.** Bottom center above the action bars (this plan), top center under the minimap row, or next to the chat window? The zone text and the error lines of the game show at top center.
2. **The look.** Alternative A or B (section 12)?
3. **The camera.** The values of section 5 come from a guess. They need a round in the game.
4. **The speed.** 30 characters each second, or slower, closer to speech? Does the speed need a setting?
5. **A sound.** None (this plan), or one soft cue of the game?
6. **The setting.** A page in the game's AddOns options (this plan), a line in the journal, or a chat command such as `/timeways narrator`? The AddOns page is where players look, but it adds the `Settings` API to the gate.
7. **The bronze dragon and the secret.** The narrator never says what it serves (3.2). A bronze dragon face hints at the Bronze Dragonflight, and 5.13 links the name of the addon to them. Showing is not naming, and the user chose the model. Is the hint too strong for players who know the lore?
8. **The talk window.** Does a narrator line wait while the talk window is open (this plan), or show in a corner of the talk window?
9. **Other frames.** Does a line also wait while the lore book, the journal, or a quest window of the game is open?
10. **A way back.** A line that the player closed early is only in the chat. Does the journal need a "last lines" list?

## 14. Build plan, after approval

Each step is one commit with its tests, and each one ships.

1. **The gate.** Add the new names of section 10 to `api.lua` and `api-signatures.lua` with `wow-api.sh`. Do the in-game display check (3.2), and write the result into this plan.
2. **The fake.** `PlayerModel`, `SetAlphaGradient`, and `wow.Advance` in `addon/tests/wow.lua`. Share the `PlayerModel` fake with the talk window plan.
3. **The queue.** `NarratorQueue.lua`: the lines, the combat hold, the 5-minute limit, at most 3 lines, and no line twice. Pure logic with no frame, with its tests.
4. **The frame.** `NarratorFrame.lua`: the card, the text, the reveal, the hold, the fade, the clicks, Escape, and the drag. `Narrator.Say` prints to the chat and gives the line to the queue.
5. **The portrait.** The model, the camera, the animations, and the fallback chain.
6. **The setting.** `TimewaysDB.narratorShows` and the options page.
7. **The in-game checks** of 11.3. Fix what they find, each fix with a failing test first.
8. **`GAMEPLAY.md`.** Put in the text of section 15. This plan marks the steps as done.

The talk window and the narrator frame share the look. When the talk window comes first, step 4 reuses its backdrop and portrait helpers. When this frame comes first, the talk window reuses them.

## 15. GAMEPLAY.md text

In 3.2, replace the bullet "It speaks in the chat window now. A window of its own, and a voice (Gnomish Relay SPEC 13.3), come later." with:

> - **The narrator frame.** A line shows in a small frame near the bottom of the screen, the way the game shows a quest NPC that talks. The frame shows a 3D bronze dragon, the model of Anachronos (display 2719) from the game files. No text names the narrator. The line appears at 30 characters each second. It holds for 5 seconds and 1 more second for each 40 characters, at most 12 seconds, and then it fades. The mouse over the frame stops the fade.
> - **The line also goes to the chat,** at once, so the player can scroll back.
> - **A click shows the full line,** and a second click closes the frame. A right click or Escape closes it at once. A drag moves it. The frame takes no clicks outside its own box, and it moves no other frame.
> - **Never in combat** (rule 3). A line in combat waits until 3 seconds after combat ends. A line that waits more than 5 minutes drops from the frame. Combat hides a frame that shows. An unread line comes back after combat.
> - **One line at a time.** At most 3 lines wait, oldest first. A line waits while the talk window is open.
> - **A setting turns the frame off.** The lines then go to the chat only.
> - **The frame changes only how a line shows.** The budget and the moments of the narrator stay the same.
> - **A voice** (Gnomish Relay SPEC 13.3) comes later.

Add to 5.4 (the API list), after the gate sentence:

> The narrator frame uses `PlayerModel` with `SetDisplayInfo`, `FontString:SetAlphaGradient`, `InCombatLockdown`, and the events `PLAYER_REGEN_DISABLED`, `PLAYER_REGEN_ENABLED`, and `LOADING_SCREEN_ENABLED`.
