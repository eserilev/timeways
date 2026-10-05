# Plan: Hero, Stories, and the Chronicle title page

Status: built, 2026-10-04, except steps 8 and 9 (section 8). The user approved the design mockup "Hero and Stories". This plan reviews that mockup against the code, fixes what does not work, and turns it into steps. Draft 2 added five decisions of the user (section 1.4). Draft 3 adapts the plan to the alias table (`docs/plans/aliases.md`), which landed after draft 2: stories now carry `{Name}` marks, not "my friend" (section 1.5). The rules of each built step are in `GAMEPLAY.md` 3.3, 3.6, 3.7, 3.7.1, 4.7, and 4.8.

The mockup covers five parts:

1. Hero, Your Story: answer cards and Your Notes.
2. Hero, Roleplay Profile: the MSP fields, Origin and Background shared as Birthplace and History, and the "What Others See" preview.
3. A new Stories tab: waiting and accepted stories, a reader page, Block player, and a badge.
4. The `/story` writing scroll: a title and a body with line breaks.
5. The Chronicle title page: a summary of the character that a model writes when a chapter ends.

This plan builds on `GAMEPLAY.md` 3.3, 3.7, 3.7.1, 4.7, and 4.8, and on `docs/plans/msp.md` and `docs/plans/logged-messages.md`. It does not repeat them.

## 1. Review of the mockup

Each point names what the mockup says, what is wrong or missing, and what this plan decides. The first five points break the feature. The rest are gaps.

### 1.1 Problems that break the feature

1. **The bridge drops a line break.** The mockup sends `story_accepted` with `\n` in the text, and the journal carries it back. Relay SPEC 9.8 forbids both: an addon line may hold no control character in any string ("A line break is a control character"), and a journal string may hold no control character. The bridge drops such a `story_accepted` line, so the story is lost after the author heard "accepted". **Decision:** the body is a list of paragraphs. Each paragraph is one string with no control character. The addon line carries `paragraphs` as a JSON array, and the journal does the same. This needs no change in the relay. Between two addons, a paragraph break travels as the escape `%0A`, because the logged channel takes no control character either.

2. **Name scrubbing makes a story longer.** (Draft 3: the alias table fixed this, see 1.5.) On Accept, the addon puts "my friend" in place of each known player name (`TaskNames.WithoutNames`). A name of 2 letters becomes 9 bytes. "Al, Al, Al" grows by a factor of more than 3. Today, a 400-byte story that names a short name many times grows past `MAX_STORY_BYTES`. The desktop refuses it, and the story is lost: the author already heard "accepted", and the reader's addon already removed it. With a 1200-byte body, the line can also pass the room of one strip, and `Outbox` drops a line that never fits, with no error. **Decision:** the desktop limits are larger than the wire limits (section 3.3). Before Accept removes a story, the addon checks the scrubbed story against the desktop limits and against `Outbox.Fits`. If either check fails, the story stays waiting, and the page says why. This bug exists today, so step 1 starts with its failing test.

3. **A C1 control character passes the addon and fails the bridge.** `TaskWire.IsCleanText` refuses `%c`, which in Lua 5.1 is only bytes 0 to 31 and 127. Rust `char::is_control`, which the bridge and `stories::checked_text` use, also refuses U+0080 to U+009F. So a story with U+0085 passes the wire and the Accept button, and then the bridge drops its line. `MspProfile.lua` already handles this range. **Decision:** `IsCleanText` refuses `\194[\128-\159]` too. This also protects the other texts that reach the desktop.

4. **The mockup sends a new type `story2` next to `story`.** An older addon drops an unknown type with no answer, so the author of a `story2` sees "Sent to Morvane." and nothing arrives. Two story types also mean two readers and two code paths. Timeways has release candidates only (`v0.1.0-rc.1` to `rc.3`), and nothing is live. **Decision:** `story` changes in place to `id`, `title`, `text`. There is no `story2`. A quick `/story <words>` sends an empty title. Both clients of the in-game test run the same build. After the first real release, a new field takes a new type name, because an older addon drops it in silence. If the user counts the release candidates as live, use the fallback in section 8.

5. **The summary reads accepted stories.** The mockup's summary reads "the Hero answers, chapters, deeds, standing with towns, and accepted stories". Three problems:
   - `GAMEPLAY.md` 4.8 says that no story reaches a model before the alias table (5.11) exists, and a property test checks it. Scrubbing takes out only the names that the addon knows. A name that it never saw stays in the text.
   - A story that an accepted call read is "used", and Remove goes away for good. So after the next chapter, every accepted story would be stuck.
   - Standing with towns is not built (`docs/plans/standing.md` is a draft).

   **Decision:** the summary reads the hero sheet, the sagas, the deeds, the level, the race, and the class. It reads no story and no standing. Stories join the summary with the alias table, and standing joins it when standing is built. The example text of the mockup ("he gave his last bread to a stranger in Brill") comes from a story, so it changes too (section 4.5).

### 1.2 Gaps

6. **The size math of the mockup is off.** It says "with every byte escaped, a story is about 3.8 KB", "a full one is 6 parts, about 4 seconds", and "16 parts hold 3904 bytes".
   - The letter limit binds before the byte limit. Only `%`, `;`, `\`, and a paragraph break take an escape, and each is one byte and one letter. With at most 1000 letters and 1200 bytes, the worst body is 934 escaped letters and 66 letters of 4 bytes: 934 × 3 + 66 × 4 = 3066 bytes. The worst title (60 letters, 72 bytes) is 56 × 3 + 4 × 4 = 184 bytes. With `1;story;`, an id of 16 bytes, and two separators, the worst message is 3276 bytes.
   - A part holds 244 bytes of text, but a cut never splits a letter, so a part can hold only 241. 16 parts then hold 3856 bytes, not 3904. The worst story takes 14 parts and fits.
   - A full story of plain ASCII is about 1090 bytes: 5 parts. The budget has a burst of 8, so it goes at once when nothing else waits. A worst story of 14 parts sends 8 at once and the rest in 6 seconds.

7. **A burst of stories can lose parts at the receiver.** A sender sends one part each second after its burst. The receiver refills a sender's allowance at one part each 2 seconds (`PeerAllowance`, 24 in a burst). Three worst stories in a row are 42 parts, and the last ones are dropped. Such a message never completes, and nobody hears of it. **Decision:** one story at a time between a sender and a receiver (1.4, decision 2). So at most one story to a player is ever in flight.

8. **The author never learns of a full box or a block.** The receiver drops a story in silence when the box is full or when the author is blocked. The author still sees "Sent to Morvane." And Block player in the mockup "uses the same list as quests", but the only `block` message of the wire names a quest id. The author's addon finds no quest, and never learns of the block. **Decision:** the author asks before writing and before sending (1.4, decision 3). The answer names a full box, a story that still waits, and a block.

9. **The target may not have Timeways.** `PlayerStories.Tell` checks only that the target is in your group. A story to a player with no Timeways goes nowhere, and the author sees "Sent". **Decision:** the question of decision 3 (1.4) also finds a player with no Timeways: no answer comes.

10. **Two meanings of `/story`.** The chat line of the mockup says "Type /story to read it". But `/story` with a target opens the scroll. The reader of a new story is often in a group with its author, and may have the author targeted. Then `/story` opens a scroll to write back, not the story. **Decision:** `/stories` opens the Stories tab. `/story` writes.

11. **"The same limit as a Hero note" is not the same rule.** A note holds no line break: the editor turns one into a space (`GAMEPLAY.md` 3.7). A story keeps its paragraph breaks. The limits in letters and bytes are the same, so the scroll can share the count code. The rules of the text are its own.

12. **The title has no byte limit in the mockup.** It says "60 characters". **Decision:** 60 letters and 72 bytes, with the 1.2 rule of the hero fields.

13. **Saved variables grow.**
    - `told` keeps the newest 100 stories that you told, each with its text. At 1200 bytes, that is about 120 KB that any addon can read. **Decision:** a told story keeps its title, not its text. The author needs only the title, for the chat line of the answer.
    - `authors` maps the number of each accepted story to its author, and is never cut. **Decision:** the newest 500 numbers stay.
    - Drafts: see 1.4, decision 4. At most 10, each at most 72 + 1200 bytes: about 13 KB.
    - `waiting` holds at most 20 stories of about 1.3 KB each: 26 KB. That is fine.

### 1.5 What the alias table changed (draft 3)

The alias table landed after draft 2. It changes these points:

- **Point 2 is gone.** Accept marks each known name as `{Name}`, never "my friend". The desktop measures a story without its marks, and a mark never makes a name longer than the player typed it. So the desktop limits are the limits of the wire: a title of 60 letters and 72 bytes, and a body of 1000 letters and 1200 bytes, with one letter and one byte for each break. `StoryText.FitsDesktop` and the larger desktop limits of 3.3 (240, 2400, 1600) are not needed. A test of each side holds the numbers equal (`the_story_limits_of_the_desktop_match_the_addon`). The test `a_story_full_of_names_at_the_limit_still_fits_the_desktop` shows it.
- **`Outbox.Fits` stays the gate before Accept.** Marks add 2 bytes to each name, and the JSON escape doubles `"` and `\`. The longest story without names fits one strip (`the_longest_story_without_names_fits_one_strip`). A story whose line does not fit stays waiting with the reason (`a_story_whose_marks_make_it_too_long_for_the_strip_stays_waiting`, with a small fake strip).
- **Point 14 is gone.** The desktop keeps IDs, and the journal gives the names back. So an accepted story shows every name, and your own name in place of `$N`.
- **The world version.** The alias table took version 5. The new shape of a story is version 6, and the table `summaries` is version 7 (3.6).

### 1.4 Decisions of the user, 2026-10-04

1. **The name is not editable.** The Roleplay Profile has no Name field. MSP `NA` is always the name of the character in the game. The code that goes away:
   - `Hero.lua`: `name` leaves `Hero.PROFILE`, `Hero.LABELS`, `Hero.HINTS`, and `Hero.LIMITS`. The profile has five fields: title, currently, appearance, age, and motto.
   - `MspProfile.lua`: `name = "NA"` leaves `MspProfile.CODES`. So the saved copy holds no `NA`, and `Checked` drops one that another addon wrote. `MspProfile.Fields()` sets `fields.NA = UnitName("player")` always, in place of `fields.NA or UnitName("player")`. The import from Total RP 3 or MyRolePlay no longer takes `NA`, because it walks `Hero.PROFILE`.
   - `crates/story/src/hero.rs`: `"name"` leaves `FIELDS` (11 fields), `limit_of`, and `NOT_IN_PROMPTS`. A `hero_set` of `name` is refused as an unknown field. The world version moves to 5 in step 3 anyway, so an old `name` row never comes back.
   - `Msp.lua` keeps `NA` in `KEPT`, because the tooltip still shows the roleplay name of other players.
   - The tests of the name field go: the desktop limit test of `name`, the import of `NA` in `msp.rs`, and the check of NA in `msp_mrp.rs`, which now expects the name in the game.
   - `GAMEPLAY.md` 3.7 and 3.7.1, and `docs/plans/msp.md` 3.1 and 3.2, lose the name field in step 6c.
2. **One story at a time between a sender and a receiver.** If I sent you a story, I can't send you another until you Accept or Decline it. This replaces "3 for each author", and Decline all goes. The box still holds at most 20 stories in all. The receiver enforces it, because a changed addon can skip the check of the sender.
3. **Check before writing.** When `/story` opens on a target, the scroll asks the target first, with a new small question: `story_ask`, answered by `story_room`. The scroll opens with the answer as its status line, and Send stays off until the answer is "open". This is cleaner than not opening, because you can still write and save a draft. Send asks again, and sends only on "open".
4. **Drafts: save for later.** A story can be saved and finished later, also when the target is offline or out of your group. The scroll has a Save button, and Close also saves a draft that changed, so no text is lost. The Stories tab has a Drafts group with Continue and Delete. Sending a draft still needs the target in your group and the answer "open".
5. **The summary.** It stays as section 3.5 says, until the user decides. The summary reads stories once the alias table (`GAMEPLAY.md` 5.11) exists.

14. **Accepted stories show "my friend".** (Draft 3: gone with the alias table, see 1.5. An accepted story shows every name.) The mockup shows an accepted story with every name in place. The desktop keeps the scrubbed copy, and the journal brings that copy back. So the page shows "my friend" for other players, and your own name for `$N`. **Decision:** keep it. One copy of each story, and no real name on the desktop. The waiting copy, before Accept, still shows every name.

15. **Another roleplay addon.** With Total RP 3, MyRolePlay, or XRP, Timeways sends no MSP message. That addon shares its own Birthplace and History. The mockup still shows "Also shared, from Your Story", the "Shared" tag on Origin and Background, and the preview. **Decision:** with another roleplay addon, those three hide. The page shows "Total RP 3 shares your profile. Change it there." and the six imported fields, read only, as today.

16. **The preview reads the wrong copy.** The build table of the mockup fills the preview from `MspProfile.Fields()`. That copy exists only while the profile is shared (`GAMEPLAY.md` 3.7.1). **Decision:** the preview reads the sheet of the journal, and the unsaved edits, so it also shows before you share.

17. **API names that the gate does not list.**
    - `PanelTemplates_*`, for the sub-tabs of Hero. **Decision:** build the sub-tabs from the tab code of the journal's own top row, so no new name comes in.
    - `IsControlKeyDown`, for Ctrl+Enter. **Decision:** add it to the gate with `scripts/wow-api.sh`. The in-game test checks that Enter in a multi-line box inserts a line break when the box has an `OnEnterPressed` handler.
    - `EditBox:HighlightText`, `GetCursorPosition`, `SetMaxLetters`, `SetMaxBytes`, `PlayerModel:SetUnit`, and `FontString:SetMaxLines` are already in the gate.

18. **The summary has no schedule.** "One model call when a chapter ends" does not say when, against which slots, or what happens with a backlog. The bridge runs at most 2 calls, a chapter already costs up to 3, and the pace window admits 10 calls in 20 minutes. **Decision:** section 3.5.

19. **The summary needs approved samples.** The narrator's voice uses samples, and the user approves new narrator lines before they ship (the narrator-voice decision of 2026-10-04). **Decision:** step 9 writes the samples of the summary and shows them to the user first.

20. **"he" in the example summary.** The model does not know the sex of the character. **Decision:** the summary follows the saga rule: `$N` at most twice, else "they", the race or class, or no name.

21. **"Your story uses this one, so it stays" never shows in this plan.** No call reads a story (point 5), so no story is used. The page keeps the code for it, and the state waits for the alias table.

22. **The reader needs to scroll.** A story of 1000 letters does not fit the parchment of the journal at the font size of the mockup. **Decision:** the reader is a scroll frame, as the editor is.

### 1.3 The relay

**This plan needs no change in the relay protocol, and no message to the relay session.** The reasons:

- `story_accepted` is a line with no reply. The bridge checks only its envelope: one object, at most 4 KiB, depth at most 4, at most 64 keys, and no control character in a string. A list of paragraphs passes all of them (depth 2). The strip room (about 2730 bytes) is lower than 4 KiB, and `Outbox.Fits` gates it.
- A new field of the journal needs no change: the journal is bounded JSON. Each paragraph and the summary are strings of at most 1600 bytes with no control character. `stories[i].paragraphs[j]` is at depth 4, under the limit of 6. A list holds at most 20 paragraphs, under the limit of 200.
- The summary is a `model_call` with a prompt far below 256 KiB. Its answer is a `model_answered`. The story program keeps its own rule of at most 2 open calls.

The mockup's design, with `\n` in the strings, would need a change to two checks of the relay: the addon lines and the journal strings. This plan avoids it.

## 2. Goal

- The Hero tab shows all six answers at once, as cards, and the Roleplay Profile as its own sub-tab with a preview of what other players see.
- Stories get their own tab, with a reader page, so a story can be long, have a title, and have paragraphs.
- The scroll lets a player write a story of up to 1000 letters, with a title and paragraphs.
- The Chronicle opens with a short summary of who the character has become, in the narrator's voice.

## 3. The design

### 3.1 The data model

**A story** has a title and a body.

| Part | Rule |
|---|---|
| Title | Optional. At most 60 letters and 72 bytes. No control character, no `\|`, valid UTF-8, no refused sign (`TaskWire.IsCleanText`). |
| Body | 1 to 20 paragraphs. Each paragraph is not empty, has no space at its start or end, and is clean text. Together, with one break between two paragraphs, at most 1000 letters and 1200 bytes. A break counts as one letter and one byte. |

`StoryText.lua` (new, pure functions) owns these rules in the addon:

- `StoryText.Body(typed)`: the body of a typed text. It turns `\r\n` and `\r` into `\n`, trims each line, and joins the lines that are not empty with one `\n`. So a blank line and a single line break both give one paragraph break.
- `StoryText.Paragraphs(body)`: the list of paragraphs.
- `StoryText.Problem(title, body)`: the reason why a story can't go, or nil.
- `StoryText.FitsDesktop(title, paragraphs)`: true when the scrubbed story is inside the limits of section 3.3. (Draft 3: not built, see 1.5. `StoryText.IsTitle`, `StoryText.IsBody`, and `StoryText.BadSign` came in its place.)

**In the addon, `TimewaysStories`** (each character):

| Key | Holds | Bound |
|---|---|---|
| `waiting` | `{ id, author, title, text, at }`, oldest first. `text` is the body, with `\n` between paragraphs. | 1 for each author, 20 in all |
| `told` | `{ to, title, at, status }`, by story id. No text. | newest 100 |
| `authors` | the author of each accepted story, by its number | newest 500 numbers |
| `drafts` | `{ to, title, text, at }`, one for each player | 10 in all. A draft holds at most 60 letters and 72 bytes of title, and 1000 letters and 1200 bytes of body: about 13 KB for all of them. |
| `nextNumber` | the next number of an accepted story | |

`Clean` checks each entry when it first reads the table, as today. A waiting body must pass the body rule, and a draft must be clean text with `\n` only as its control character. A broken entry goes. If the table holds more than one waiting story of an author, or more than 10 drafts, `Clean` keeps the oldest story and the newest drafts.

A told story with the status `sent` blocks a new story to that player. The answer of the receiver is the truth: when it says "open", a `sent` story of this author no longer waits there, and it gets the status `lost`. So an answer that got lost never blocks the author for good.

**On the desktop**, a row of the `stories` table:

```rust
pub enum StoryChange {
    Accepted { number: u64, at: Tick, title: Option<String>, paragraphs: Vec<String> },
    Removed { number: u64, at: Tick },
}
```

`PlayerStory` in the journal gets `title: Option<String>` and `paragraphs: Vec<String>` in place of `text`.

**The summary** is a row of a new table `summaries`:

```rust
pub struct Summary { pub after: Tick, pub text: String }
```

`after` is the tick where the chapter began, whose saga came just before the summary. The newest row stands. The table only grows, as the other row tables do.

### 3.2 The wire

Every message stays `1;<type>;<fields>`. The changes:

| Type | Fields | Channel | Change |
|---|---|---|---|
| `story` | `id`, `title`, `text` | logged | Was `id`, `text`. `title` may be empty. |
| `story_accept` | `id` | normal | none |
| `story_decline` | `id` | normal | none |
| `story_ask` | none | normal | New. "Can I send you a story?" |
| `story_room` | `room` | normal | New. The answer to `story_ask`, and to a story that the box did not take. `room` is `open`, `full`, `waiting`, or `blocked`. |

- **Escapes.** `Escape` adds `\n` to the bytes that take an escape: `%`, `;`, `\`, and `\n`. A paragraph break travels as `%0A`.
- **A new reader, `Body`.** It unescapes the field and checks the body rule of 3.1: the only control character is `\n`, never two in a row, never at the start or the end, at most 19 of them, and at most 1000 letters and 1200 bytes. Every other reader keeps `IsCleanText`, so a `\n` in a title, a quest, or a zone is still refused.
- **C1 controls.** `IsCleanText` refuses `\194[\128-\159]` (point 3).
- **The room of a sender**, in order: `blocked` for a blocked sender, `waiting` when a story of that sender waits, `full` when 20 wait, and else `open`.
- **On `story_ask`:** a sender outside your group gets no answer. Every other sender gets `story_room`.
- **On `story`:** a sender outside your group gets nothing back. The same id from the same author again is dropped. A room other than `open` answers `story_room` with it, and drops the story. Else the story waits. So the box never holds two stories of one author, even from a changed addon.
- **The author, on `story_room`:** the scroll shows the room. `blocked` adds the player to `refusedBy`, as for a quest block. `open` gives a told story with the status `sent` to that player the status `lost`. A `story_room` other than `open` that comes with no open question refuses the last story sent to that player: that story gets the status of the room, and the chat says so.
- **No answer to `story_ask` in 5 seconds** means the target has no Timeways.
- A blocked author now gets an answer, `blocked`, where a call of a player quest gets none. The author can't send anyway, and the scroll says why.
- **The limits of the parts do not change:** at most 16 parts of 255 bytes. The worst story takes 14 (point 6).

### 3.3 The desktop

**The input line.** `story_accepted` becomes:

```json
{"type":"story_accepted","at":1790000000,"number":7,"title":"The Fire at Solliden Farmstead","paragraphs":["We went into Solliden...","She stood in the doorway..."]}
```

`title` is optional, and an empty one counts as none.

**The checks** (`stories::is_good_story`). Draft 3: the limits are the limits of the wire (1.5): `MAX_TITLE_CHARS` 60, `MAX_TITLE_BYTES` 72, `MAX_BODY_CHARS` 1000, `MAX_BODY_BYTES` 1200, and `MAX_PARAGRAPHS` 20. Draft 2 said, before the alias table:

| Constant | Value | Why |
|---|---|---|
| `MAX_STORY_TITLE_BYTES` | 240 | A title of 72 bytes, with room for names that grow |
| `MAX_PARAGRAPHS` | 20 | The wire rule. The journal takes at most 200 items in a list. |
| `MAX_PARAGRAPH_BYTES` | 1600 | One string of the journal (relay SPEC 9.8) |
| `MAX_STORY_BYTES` | 2400 | The body of 1200 bytes, twice, for names that grow |

A title or a paragraph that holds a control character (`char::is_control`, so C1 too) or a `|` refuses the story. So does an empty paragraph, a paragraph with a space at its start or end, more than 20 paragraphs, or no paragraph. `StoryText.FitsDesktop` in the addon uses the same numbers, and a test of each side holds them equal.

**The fit in one strip.** Without names that grow, the longest story fits one strip: about 2580 of about 2730 bytes, with the longest character line. The JSON escape doubles `"` and `\`, and the letter limit binds first: 934 × 2 + 66 × 4 bytes in the body. Names that grow can pass the room, so `Outbox.Fits` is the last gate before Accept.

**The schema.** The `stories` rows change their shape, and a new table comes. `store/database.rs` moves `VERSION` from 4 to 5. A world of version 4 is refused, never changed, as `GAMEPLAY.md` 5.7 says. Nothing is live, so there is no migration.

**The journal.** Each story is an item with its title and paragraphs. A page holds as many items as fit (`journal::pages`), so a long shelf spreads over pages.

### 3.4 The story program: calls and reads

- **No read rule reads `stories`** (point 5). The property test `no_prompt_holds_the_text_of_a_story` also runs the summary call.
- **A new call kind `summary`.** Its read rule (`story/reads.rs`, and the table of `GAMEPLAY.md` 5.14): the hero rows of the six questions, the `chapters` rows whose sagas its prompt holds, the events behind each deed of its prompt and behind the level, and the `summaries` row before it.

### 3.5 The summary

The summary reads stories once the alias table (`GAMEPLAY.md` 5.11) exists. Until then, it reads none (point 5).

**When.** After the saga round of a chapter ends, with a saga or with none, the summary of that chapter is due. The call opens only when all of these hold:

- no other call is open;
- the pace window is not tight (`GAMEPLAY.md` 3.3);
- no older chapter waits for its saga. The sagas go first.

If a condition fails, the summary waits for a later batch. With a backlog, only the newest finished chapter gets a summary: an older summary would be replaced at once. A restart forgets a due summary that never opened, and the next saga round makes it due again.

**Cost.** One call for each chapter. With the 3 calls of a saga, a chapter costs at most 4. A chapter in a tight window costs 1 for the saga and nothing for the summary.

**The prompt** (`summary.rs`), in this order, with `Call::Summary` in `tokens.rs`:

1. The persona of the narrator and the house rules.
2. The task: "Write who this character has become, in one paragraph of at most 80 words."
3. The race, the class, and the level.
4. The six answers of the sheet, as "the hero's own story, not canon", fenced, each cut to 300 characters. Never the Roleplay Profile: the name and the title can name a real player.
5. The summary before it, fenced, so the new one grows from the old one.
6. The sagas of the newest chapters, fenced, newest first. A chapter with no saga gives its plain list. `tokens::largest_fit` keeps as many as fit the budget, at most 3.
7. The deeds of note: first kills of bosses and rares, and class quests, newest first, at most 10.
8. The samples of the summary voice (step 9).
9. The note: `$N` at most twice, else "they", the race or the class, or no name. Never "our hero". No markdown. Reply with JSON: `{"summary": "<the paragraph>"}`.

**The budget.** The context is 2048 tokens. The reply is at most 600 characters and the JSON: about 175 tokens. The template takes 100. So the prompt has about 1770 tokens. The estimate: the persona and the house rules about 365, the task and the note about 150, the sheet at most 450, the summary before about 150, the deeds about 150, and the samples about 150. That leaves about 350 tokens for the sagas, so `largest_fit` keeps about 2 sagas. The property test `the_summary_prompt_fits_its_budget` checks every case.

**The check** (`summary::checked_summary`), with no retry, as for a chapter:

- `json_object`, then `voice_text` with at most 600 characters and 1600 bytes: one line, no control character, no emoji, no banned word, no name past the cutoff except what the player's own text names.
- `slop_in` against the facts of its prompt.
- `$N` at most twice, and no "our hero".
- It copies no 8 words in a row of a sample.

A refused answer or a failed call keeps the summary before it. A model answer is outside text, so `checked_summary` joins the fuzz target `answers`.

**Without a model**, there is no summary. The title page shows the character line and the chapters.

### 3.6 Migrations

The user decided on 2026-10-04: from now on, each change of the schema says what a migration from the old version needs. Migrations are not built before Timeways launches on CurseForge, but they will be required then. A world of another version is still refused (`GAMEPLAY.md` 5.7).

| Version | Change | What a migration from the version before needs |
|---|---|---|
| (none, step 6c) | The field `name` of the hero leaves the sheet. | Nothing. A row `{"field":"name",...}` of the `hero` table still reads, and the sheet leaves it out. A migration can keep it. |
| 6 (step 3) | A row of `stories` holds `title` and `paragraphs` in place of `text`. | Read each `accepted` row of version 5, and write it again as `{"line":"accepted","number":...,"at":...,"title":null,"paragraphs":[<text>]}`. The text keeps its IDs. A text of version 5 holds no line break, so it is one paragraph. A `removed` row stays as it is. The positions stay, so the reads of a call still point at the same story. Then set `user_version` to 6. |
| 7 (step 10) | A new row table `summaries`, one row `{"after": <tick>, "text": "..."}` for each summary. | Create the table `summaries` with the columns of a row table, empty, and set `user_version` to 7. No other row changes. The first summary comes after the next saga round. |
| Vocabulary 9 (database still 7) | Five new fact names: `first_mount`, `first_epic_mount`, `first_epic_item`, `upgraded` (a number: the slot), and `quality` (a number on the item). Two new input lines: `mount_ridden` and `item_equipped`. | Nothing. No event of vocabulary 8 uses a new name, and no declared name changed, so a world of version 8 replays as it is. The mounts and items from before the version are not in the world. A migration cannot make them, because no line of the `inputs` table holds them. |

## 4. The UI, page by page

The copy below is final. It follows the UI copy rules of `CLAUDE.md`.

### 4.1 The tabs

`Journal.SECTIONS` becomes `hero`, `chapters`, `stories`, `deeds`, `learned`, `quests`. The tabs read: Hero, Chronicle, Stories, Deeds, Knowledge, Quests.

- **The badge** on Stories counts the waiting stories. With none, it hides.
- The waiting stories live in the addon. So the Stories tab shows them while the journal loads, as the Quests tab shows player quests. The accepted part shows "Loading..." until the journal comes.

### 4.2 Hero: Your Story

Two sub-tabs under the Hero tab: **Your Story** and **Roleplay Profile**. They use the tab code of the top row (point 17).

| Where | Copy |
|---|---|
| Line under the sub-tabs | Shapes your chapters and what NPCs say to you. |
| Card labels | Origin, Background, Goal, Bond, Flaw, Traits |
| Empty card | the question (as today, `Hero.HINTS`) and [Answer] |
| Filled card | the answer, at most 3 lines, and [Edit] |
| Tag on Origin and Background | Shared (only while the profile is shared and no other roleplay addon is present) |
| Edit in a card | the question, the box, "74 / 1000", [Cancel] [Save] |
| Bar | 4 of 6 answered |
| Notes heading | Your Notes [Add a note] |
| Notes, empty | A grudge, a promise, a secret. |
| A note | the text, then "About Novice Elreth · Deathknell · 03 Oct" [Remove] |

- After Save, the next empty card opens for its answer.
- **Step 6a** opens today's editor for Answer and Edit. **Step 6b** edits inside the card. A box that grows moves the cards below it.
- `Hero.lua` keeps the limits and the "Saving..." state of today.

### 4.3 Hero: Roleplay Profile

| State | Line on top | Button |
|---|---|---|
| Not shared | Only you see this. | [Share] |
| Shared | Players with roleplay addons like Total RP 3 see this. | [Stop sharing] |
| Another roleplay addon | Total RP 3 shares your profile. Change it there. | none |

| Where | Copy |
|---|---|
| Field labels | Title, Age, Motto, Currently, Appearance |
| Empty field | the question and [Add] |
| Filled field | the text and [Edit] |
| Block | Also shared, from Your Story [Edit in Your Story] |
| In the block | Origin: the text, "as Birthplace". Background: the text, "as History". Empty: "Not answered yet". |
| Preview heading | What Others See |
| Preview parts | Tooltip, Profile, Description, History |

- With another roleplay addon, the fields show its text, read only, with no Edit. The block "Also shared" and the preview hide (point 15).
- The preview reads the sheet of the journal and the unsaved edits (point 16). It cuts Description and History to 3 lines with "…".
- The tooltip line of the preview uses the level, race, and class of the game: "Level 9 Undead Paladin (Player)".
- The preview names the character by the name in the game, with the title after it: "Kobee, Oathkeeper of the Fallen Watch". There is no Name field (1.4, decision 1).

### 4.4 Stories

The list is on the left on parchment (`side = "sheet"`). The reader is on the right.

| Where | Copy |
|---|---|
| Empty | No stories yet. |
| Empty, second line | To tell one, target someone in your group and type /story. |
| Heading | Waiting 5 |
| Heading | Accepted 2 |
| Row | the title, the author, the day ("Today", "3 Oct") |
| Row with no title | A story from Brokka (in faded ink) |
| Reader tag | New (on a waiting story) |
| Byline, waiting | By Kobee · Today |
| Byline, accepted | By Ashka · Accepted 2 Oct |
| Byline, author unknown | By a friend · Accepted 2 Oct |
| Under a waiting story | To report abuse, open Support in the game menu. |
| Buttons, waiting | [Block player] [Decline] [Accept] |
| Button, accepted | [Remove] |
| Used story (later, point 21) | Your story uses this one, so it stays. |
| At 20 waiting | Answer some stories to get new ones. |
| Bar | 5 waiting |
| Accept fails (point 2) | This story is too long to keep. Decline it, or ask Kobee for a shorter one. |
| Block popup | Block Kobee? You won't get quests or stories from them anymore. [Block] [Cancel] |
| Remove popup | Remove this story? [Remove] [Cancel] (as today) |
| Heading | Drafts 2 |
| Draft row | the title, or "A story about Morvane" in faded ink, then "To Morvane · Today" |
| Draft buttons | [Continue] [Delete] |
| Delete popup | Delete this draft? [Delete] [Cancel] |
| Blocked players, in Quests | They can't send you quests or stories |

- The reader shows each paragraph as its own block, with a gap between them, in a scroll frame (point 22). `$N` shows as your name.
- Waiting stories come first, newest first. Then the accepted ones, newest first. Then the drafts, newest first.
- **Continue** opens the scroll with the draft, for its player, also when that player is not your target. The scroll then asks the player, as in 4.5.
- Accept and Decline open the next waiting story.
- **Block player** asks first. It adds the author to the blocked list of quests, and removes the waiting story of that author. It sends nothing: the next question of the author gets `blocked`.
- The Hero page loses its "Stories About You" part.

**Chat:**

| When | Copy |
|---|---|
| A story came, with a title | Kobee told a story about you: The Fire at Solliden Farmstead. Type /stories to read it. |
| A story came, no title | Kobee told a story about you. Type /stories to read it. |
| You accepted | Story accepted. It's part of your story now. |
| You declined | Story declined. |
| The author hears | Morvane accepted your story. / Morvane declined your story. |
| The author hears, the box did not take it | Morvane's story box is full. / Morvane hasn't answered your last story yet. / Morvane doesn't take stories from you. |

**Commands:** `/stories` opens the Stories tab. `/story accept` and `/story decline` answer the newest waiting story, as today.

### 4.5 The writing scroll

`/story` with a target in your group opens the scroll for that player, with the draft for that player when there is one. The scroll asks the player at once (`story_ask`), and the status line shows the answer. Continue on a draft opens it the same way. `/story <words>` still sends a quick story with no title, from the chat, after the same question.

| Where | Copy |
|---|---|
| Window title | Tell a Story |
| Header | A story about / Morvane / Level 11 Undead Mage |
| Title box, empty | Title |
| Body box, empty | What happened with Morvane? |
| Count, near the limit | 87 left |
| At the limit | No room left |
| Footer | Like chat, Blizzard can read what you send. |
| Buttons | [Close] [Save] [Send] |
| Saved | Saved. Find it under Drafts in Stories. |
| Drafts full | You have 10 drafts. Send or delete one first. |
| Close with drafts full | Discard this story? [Discard] [Cancel] |
| Sending | Sending... |
| Sent | Sent to Morvane. They'll decide if it's part of their story. |
| Checking | Checking... |
| Room: open | (no line, and Send is on) |
| Room: full | Morvane's story box is full. |
| Room: waiting | Morvane hasn't answered your last story yet. |
| Room: blocked | Morvane doesn't take stories from you. |
| No answer | Morvane needs Timeways to get stories. |
| Not in your group | Invite Morvane to your group to send it. |
| Left the group | Morvane left your group. Invite them back to send it. |
| The `\|` sign | Stories can't hold the \| sign. Take it out and try again. |
| Other signs | Some of these characters can't be sent. Take them out and try again. |
| Chat, no target | Target a player in your group first. |

- **Title:** one line, at most 60 letters and 72 bytes. Enter or Tab goes to the body.
- **Body:** Enter starts a new paragraph. A blank line also counts as one break, so two Enters give the same story as one. Ctrl+Enter sends.
- The count hides until 100 letters are left.
- Save keeps the draft and closes the scroll. Close or Escape also saves a draft that changed. A sent story deletes its draft.
- One draft for each player, 10 in all. Save never drops an older draft: at 10, it says so, and the text stays.
- On an error, the text stays, and the bad sign is selected. A typed `|` reads as `||` in the box, so the position comes from the text before the unescape.
- Send stays off until the answer is "open". Send asks again, and sends only on "open". The status line follows each answer.
- The header of a draft for a player who is not your target shows the name only, with no portrait and no level.
- The portrait is `PlayerModel:SetUnit("target")`.

### 4.6 Chronicle: the title page

The first row of the Chronicle list is the title page: the name of the character and "Who you've become". The book still opens at the newest chapter, as `GAMEPLAY.md` 3.6 says.

| Where | Copy |
|---|---|
| Row | Kobee / Who you've become |
| Page header | Kobee / Level 12 Undead Paladin |
| Before the first summary | Fills in when your first chapter ends. |
| With no model, after a chapter | (the header and the chapters only) |

The summary shows as one paragraph with your name in place of `$N`. An example in the voice, from the sheet and the sagas only:

> Deathknell has buried its dead twice, and $N, once a squire of the Silver Hand, was among those who climbed back out. The paladin keeps an oath the Light may no longer hear. Brill has come to trust the Forsaken who carries it, and the Scarlet recruits at Solliden know that face.

Later, as an option: share the summary as the History of the profile.

## 5. What changed from the mockup, and why

| Mockup | This plan | Why |
|---|---|---|
| A new type `story2`; `story` stays | `story` changes in place to `id`, `title`, `text` | Nothing is live. One type means one reader. An older addon drops either in silence. |
| `\n` in `story_accepted` and the journal | A list of paragraphs | The bridge forbids control characters in both (1.1, point 1). |
| Enter: new line. Two Enters: new paragraph. | Enter and a blank line both make one paragraph break | No line break inside a paragraph can cross the bridge. |
| `MAX_STORY_BYTES` 400 to 1200 | Title 240, body 2400, paragraph 1600, 20 paragraphs | Names that leave the text make it longer (point 2). |
| Accept always removes the story | Accept checks the desktop limits and the strip first | A story that can't go stays waiting, with the reason. |
| Same limit as a Hero note | Same letters and bytes, its own text rule | A note holds no line break. |
| Title "60 characters" | 60 letters and 72 bytes | Every text of the desktop has a byte limit. |
| "About 3.8 KB", "6 parts, about 4 seconds" | At most 3276 bytes in 14 parts. A full ASCII story is 5 parts and goes at once. | The letter limit binds first, and a part never splits a letter. |
| 3 stories waiting from one author, and Decline all | One story at a time between two players. No Decline all. | The user's decision. It also keeps a burst of stories from losing parts at the receiver. |
| Block uses the quest list | Also the room `blocked` in `story_room` | Else the author never learns of a block. |
| The scroll checks only the group | `story_ask` and `story_room` when the scroll opens, and again on Send | The user's decision: know before you write. |
| Close keeps the draft | Save, Close saves too, and a Drafts group with Continue and Delete. 10 drafts. | The user's decision: finish a story later, also for a player who is offline. |
| A Name field in the profile | No Name field. MSP `NA` is the name in the game. | The user's decision. |
| "Type /story to read it" | "Type /stories to read it" | `/story` with a target opens the scroll. |
| Any group member | Only a member who answered `story_ask` | Else a story to a player with no Timeways goes nowhere. |
| Accepted stories with every name | Every name (draft 3) | The alias table keeps IDs, and the journal gives the names back (1.5). |
| "Also shared" and the preview always | Hidden with another roleplay addon | That addon shares its own fields. |
| Preview from `MspProfile.Fields()` | Preview from the sheet | The shared copy exists only while sharing. |
| `PanelTemplates` sub-tabs | The journal's own tab code | `PanelTemplates_*` is not in the API gate. |
| The summary reads stories and standing | Neither, for now. It reads stories once the alias table exists. | No story reaches a model before the alias table. Standing is not built. |
| "One model call when a chapter ends" | After the saga round, with no open call and no tight window, newest chapter only | The slots and the pace window are shared with sagas and questions. |
| Example summary with "he" and a story | `$N`, "they", or the class, from the sheet and the sagas only | The model does not know the sex, and stories stay out. |
| "In use" state | Kept in code, never shown yet | No call reads a story yet. |
| Told stories keep their text | Only the title | Saved variables stay small. |

## 6. Tests

### 6.1 Unit tests of the story program

`crates/story/tests/stories.rs`:

- `a_story_with_a_title_and_three_paragraphs_is_kept`
- `a_story_with_no_title_is_kept`
- `an_empty_paragraph_refuses_the_story`
- `a_paragraph_with_a_space_at_its_end_refuses_the_story`
- `a_story_with_no_paragraph_is_refused`
- `a_story_with_twenty_paragraphs_is_kept_and_twenty_one_are_refused`
- `a_paragraph_with_a_line_break_refuses_the_story`
- `a_c1_control_character_refuses_the_story`
- `a_title_with_a_bar_or_a_control_character_is_refused`
- `a_body_at_the_byte_limit_is_kept_and_one_byte_more_is_refused`
- `a_title_at_the_byte_limit_is_kept_and_one_byte_more_is_refused`
- `the_journal_carries_the_title_and_the_paragraphs`
- `a_long_shelf_of_stories_spreads_over_pages`
- `the_story_limits_of_the_desktop_match_the_addon` (reads `StoryText.lua`)
- `a_world_of_version_four_is_refused`

`crates/story/tests/bridge.rs`:

- `a_story_accepted_line_with_paragraphs_passes_the_bridge`
- `a_story_accepted_line_with_a_line_break_is_dropped_by_the_bridge` (it records why the body is a list)
- `the_longest_story_without_names_fits_one_strip`

`crates/story/tests/summary.rs` (new):

- `no_summary_before_the_first_chapter_ends`
- `the_summary_call_opens_after_the_saga_round_of_its_chapter`
- `the_summary_waits_while_another_call_is_open`
- `the_summary_waits_while_the_window_is_tight`
- `the_summary_waits_while_an_older_chapter_has_no_saga`
- `only_the_newest_finished_chapter_gets_a_summary`
- `a_refused_summary_keeps_the_one_before_it`
- `a_failed_call_keeps_the_one_before_it`
- `a_summary_over_six_hundred_characters_is_refused`
- `a_summary_that_names_the_hero_three_times_is_refused`
- `a_summary_that_says_our_hero_is_refused`
- `the_summary_prompt_holds_no_story_and_no_profile_field`
- `the_summary_reads_the_sheet_the_sagas_and_the_summary_before_it`
- `the_journal_carries_the_newest_summary`

`crates/rules` (unit tests next to the Lean theorems, section 7):

- `a_number_comes_once`
- `a_removed_story_never_comes_back`
- `a_used_story_cannot_be_removed`

### 6.2 Lua tests

The addon tests run the real Lua in `crates/addon-tests`, with the fake API of `addon/tests/wow.lua`.

`crates/addon-tests/tests/story_text.rs` (new):

- `a_blank_line_and_a_line_break_give_the_same_story`
- `a_carriage_return_becomes_a_paragraph_break`
- `spaces_at_the_ends_of_a_paragraph_go`
- `a_body_of_twenty_one_paragraphs_has_a_problem`
- `a_body_of_a_thousand_letters_fits_and_one_more_does_not`
- `a_typed_bar_is_found_at_its_place`
- `a_c1_control_character_is_not_clean_text`

`crates/addon-tests/tests/player_stories.rs` (existing, extended):

- `a_story_with_a_title_and_paragraphs_crosses_the_wire_unchanged`
- `a_paragraph_break_travels_as_an_escape`
- `a_story_with_two_breaks_in_a_row_is_dropped`
- `a_line_break_in_a_title_is_dropped`
- `a_story_of_the_old_shape_is_dropped`
- `the_longest_story_with_every_letter_escaped_fits_sixteen_parts`
- `a_second_story_from_one_author_gets_the_room_waiting`
- `a_twenty_first_story_gets_the_room_full`
- `a_story_from_a_blocked_author_gets_the_room_blocked`
- `story_ask_gets_open_when_nothing_of_yours_waits`
- `story_ask_from_outside_the_group_gets_no_answer`
- `the_author_hears_why_the_box_did_not_take_the_story`
- `the_answer_open_marks_a_sent_story_as_lost`
- `the_author_can_send_again_after_accept_or_decline`
- `a_story_from_a_player_outside_the_group_gets_no_answer`
- `block_player_removes_the_waiting_story_of_that_author`
- `a_blocked_author_learns_it_from_the_room`
- `accept_sends_the_title_and_the_paragraphs_without_names`
- `names_leave_the_title_too`
- `a_story_whose_names_grow_past_the_strip_stays_waiting` (the failing test of today's bug)
- `a_story_whose_names_grow_past_the_desktop_limit_stays_waiting`
- `a_told_story_keeps_no_text`
- `a_saved_waiting_story_with_a_broken_body_is_dropped`
- `a_draft_survives_a_reload`
- `save_at_ten_drafts_keeps_the_text_and_says_why`
- `a_sent_story_deletes_its_draft`
- `a_saved_file_with_two_waiting_stories_of_one_author_keeps_the_oldest`
- `authors_keep_the_newest_five_hundred_numbers`

`crates/addon-tests/tests/stories_page.rs` (new):

- `the_badge_counts_the_waiting_stories`
- `the_badge_hides_with_no_waiting_story`
- `waiting_stories_show_while_the_journal_loads`
- `a_story_with_no_title_shows_who_told_it`
- `drafts_show_with_continue_and_delete`
- `continue_opens_the_scroll_for_the_player_of_the_draft`
- `delete_asks_first`
- `accept_opens_the_next_waiting_story`
- `at_twenty_waiting_the_list_asks_for_answers`
- `the_reader_puts_your_name_in_place_of_the_hero_mark`
- `stories_slash_command_opens_the_tab`
- `the_hero_page_has_no_stories_part`

`crates/addon-tests/tests/story_scroll.rs` (new):

- `the_scroll_does_not_open_without_a_target_in_your_group`
- `the_scroll_asks_the_target_when_it_opens`
- `send_stays_off_until_the_room_is_open`
- `the_status_line_names_a_full_box_a_waiting_story_and_a_block`
- `the_scroll_says_when_the_target_has_no_timeways`
- `send_asks_again_and_sends_only_on_open`
- `close_saves_a_changed_draft`
- `a_draft_for_a_player_out_of_the_group_can_be_written_but_not_sent`
- `an_error_keeps_the_text_and_selects_the_bad_sign`
- `the_count_shows_only_when_a_hundred_letters_are_left`
- `a_target_who_left_the_group_keeps_the_text_with_the_reason`

`crates/addon-tests/tests/hero.rs` (extended):

- `every_question_shows_as_a_card`
- `save_opens_the_next_empty_card`
- `shared_shows_on_origin_and_background_only_while_sharing_alone`
- `another_roleplay_addon_hides_also_shared_and_the_preview`
- `the_preview_reads_the_sheet_before_you_share`

`crates/addon-tests/tests/journal.rs` (extended):

- `the_title_page_is_the_first_row_of_the_chronicle`
- `the_title_page_puts_your_name_in_place_of_the_hero_mark`
- `before_the_first_summary_the_title_page_says_when_it_fills_in`

### 6.3 Property tests

In `crates/story/tests/properties.rs`. Each number draws its edges on purpose, with `prop_oneof!`: 0, 1, the limit, the limit plus one, and a uniform draw. Each text draws its edges too: empty, all `"`, all `\`, all letters of 4 bytes, and many paragraphs.

- `any_play_of_story_lines_keeps_each_number_standing_at_most_once`. The numbers are 0, 1, `u64::MAX`, and small ones that repeat.
- `every_accepted_story_fits_one_page_of_the_journal`. Paragraph counts 0, 1, 20, and 21. Bytes at 0, the limit, and the limit plus one. Text of all `"` (the JSON escape) and of all 4-byte letters (the slot escape).
- `a_story_line_that_the_bridge_passes_lands_or_is_refused_with_its_reason`.
- `no_prompt_holds_the_text_of_a_story` (existing), now with summary calls in the play.
- `the_summary_prompt_fits_its_budget`, for any sheet (each answer empty or at its limit), any number of chapters (0, 1, 3, 50), and sagas at 600 characters.

In `crates/addon-tests/tests/player_stories.rs`, with proptest over the real Lua, as `task_names.rs` does:

- `the_waiting_box_never_holds_two_stories_of_one_author_or_more_than_twenty`, for any order of stories from 1 to 30 authors, also from a changed addon that skips `story_ask`, with blocks, accepts, and declines between them. Author counts draw 1, 20, 21, and 30 on purpose.
- `a_sender_never_has_two_stories_waiting_with_one_receiver`. Two real addons in the two-player harness, with any order of asks, sends, accepts, declines, reloads of either side, and lost messages. After each step: the receiver holds at most one story of the sender, and the sender holds at most one told story with the status `sent` to the receiver.
- `the_drafts_never_pass_ten_and_save_never_drops_one`.
- `encode_then_decode_gives_any_clean_story_back`. Edges: all `%`, all `;`, all `\`, 19 breaks, 1000 letters, 1001 letters, and 1200 bytes.
- `every_story_that_the_scroll_takes_fits_sixteen_parts`.

### 6.4 Fuzz targets

- **New: `fuzz/fuzz_targets/story_accepted.rs`.** An arbitrary title, arbitrary paragraphs, and a number build a `story_accepted` line. The line goes through `fake_bridge::dropped_lines_of`, then into the story program, then a journal request. It checks: no panic; a line that the bridge passes is kept or refused with `BadStory`; each journal page passes `fake_bridge::game_reply`; a kept story has at most 20 paragraphs, each at most 1600 bytes with no control character. Dictionary `fuzz/dicts/story_accepted.dict`: `"paragraphs"`, `"title"`, `\n`, `\u0085`, `|`, `"`, `\\`.
- **Seeds in `fuzz/seeds/story_accepted/`:** `title_and_paragraphs.json`, `no_title.json`, `twenty_paragraphs.json`, `twenty_one_paragraphs.json`, `all_quotes.json`, `line_break_in_a_paragraph.json`, `c1_control.json`, `empty_paragraph.json`, `max_number.json`.
- **`task_wire`:** new seeds with `%0A`, `%0A%0A`, `%0D`, a title with `%0A`, the bytes of U+0085, and `story_room` with an unknown room.
- **`task_play`:** `story_ask` and `story_room` join the types, with a room of any text. Corvin's `/story` words gain line breaks, and the scroll sends with a title.
- **`answers`:** `summary::checked_summary` joins the checked answers. A summary that passes keeps its limits and holds no control character.

### 6.5 Steps in the game

Built: `TESTING.md` sections 13, 21, and 22 hold these steps now, in their final words. The text below is draft 2.

**13. Stories about each other.** Two characters with Timeways in one party.

1. On the first character, target the second and type `/story`. The scroll opens with the second character's portrait. Type a title and three paragraphs, with a blank line between two of them. Click Send. The chat says "Sent to ... They'll decide if it's part of their story."
2. On the second character, the chat says "... told a story about you: <title>. Type /stories to read it." The Stories tab has a badge with 1.
3. Type `/stories`. The story shows on a page, with its title, "By ... · Today", and three paragraphs. The line "To report abuse, open Support in the game menu." shows under it.
4. Click Accept. The first character's chat says "... accepted your story." On the second character, the story moves under Accepted, with "By ... · Accepted <day>".
5. Tell a story, and before the second character answers, type `/story` again. The scroll opens with "... hasn't answered your last story yet.", and Send is off. Decline it on the second character. Send turns on.
6. Write a story, click Save, and leave the party. Open Stories: the draft shows under Drafts. Click Continue: "Invite ... to your group to send it." Invite the player again: Send turns on.
7. Type a `|` in the body, and click Send. The error shows, the `|` is selected, and the text stays.
8. Press Enter in the body. A new paragraph starts, and nothing is sent. Press Ctrl+Enter. The story is sent.
9. Close the scroll with text in it. `/reload`. Open it again for the same player. The text is still there.
10. Click Block player on a waiting story, then Block. The first character types `/story`: the scroll opens with "... doesn't take stories from you." The Quests tab lists the player under Blocked players.
11. Target a player in your group who has no Timeways, and type `/story`. After a few seconds, the scroll says "... needs Timeways to get stories."
12. Tell a story that names the first character. Accept it the next day, after the author left the party. In the world file, run `sqlite3 c_<name>.sqlite "SELECT body FROM stories"`: the name shows as "my friend".

**20. The Hero tab.**

1. Open `/hero`. The six answers show as cards. Click Answer on an empty card, type, and Save. The next empty card opens.
2. Open Roleplay Profile. The line says "Only you see this." There is no Name field. The preview under What Others See shows your name in the game and your fields.
3. Click Share. Origin and Background in Your Story show "Shared".
4. If you use Total RP 3: the page says "Total RP 3 shares your profile. Change it there." "Also shared" and the preview do not show.

**21. The Chronicle title page.** This test needs a model.

1. Before your first chapter ends, open the Chronicle and click the first row. It says "Fills in when your first chapter ends."
2. Play until a chapter ends and its saga shows. Open the first row again. A paragraph shows with your name in it, and never "our hero".

## 7. Formal proofs

The Lean proofs read `crates/rules` through Charon and Aeneas, so a theorem speaks about the code that runs. A rule that runs in Lua has no Rust that runs. A Lean proof of a Rust copy of a Lua rule proves the copy, not the addon, and the two can drift. So the Lua rules get property tests over the real Lua (6.3), and only rules of the story program go to Lean.

### 7.1 Into `crates/rules`: the story shelf

A new module `crates/rules/src/story_shelf.rs`, in the loop style of `quest_log.rs` (no closure, no iterator adapter, no `?` on an `Option`). `stories.rs` and `story/stories.rs` call it for `standing`, `is_taken`, and the removal rule. No other code of the story program decides these.

```rust
pub enum ShelfLine { Accepted { number: u64 }, Removed { number: u64 } }

/// True when the line may land on the shelf. `used` holds the numbers that an accepted call read.
#[cfg_attr(charon, verify::start_from)]
pub fn lands(lines: &[ShelfLine], line: &ShelfLine, used: &[u64]) -> bool

/// The numbers that stand, oldest first.
#[cfg_attr(charon, verify::start_from)]
pub fn standing(lines: &[ShelfLine]) -> Vec<u64>
```

`Landed lines` means: each line landed against the lines before it. The story program only appends a line that lands, so every shelf in a world is `Landed`.

| Theorem | In plain words | Lean sketch | Worth it |
|---|---|---|---|
| `a_number_stands_at_most_once` | No number stands twice. | `theorem a_number_stands_at_most_once (ls : List ShelfLine) (h : Landed ls) : standing ls ⦃ ns => ns.val.Nodup ⦄` | Yes. The journal keys a story by its number, and the addon maps a number to one author. |
| `a_removed_story_never_comes_back` | After a removal, the number never stands again, whatever lands after. | `theorem a_removed_story_never_comes_back (ls more : List ShelfLine) (n : U64) (h : Landed (ls ++ more)) (hr : .Removed n ∈ ls) : standing (ls ++ more) ⦃ ns => n ∉ ns.val ⦄` | Yes. "A number comes once" is a rule of `GAMEPLAY.md` 4.8, and it holds for any order of lines. |
| `a_used_story_always_stands` | A story that a call used stands after any line. | `theorem a_used_story_always_stands (ls : List ShelfLine) (l : ShelfLine) (used : Slice U64) (n : U64) (hu : n ∈ used.val) (hs : n ∈ standingSpec ls) : lands ls l used ⦃ ok => ok → n ∈ standingSpec (ls ++ [l]) ⦄` | Yes. It is the rule "accepted in use can't be removed", stated over any line, not over one test. |
| `lands.spec` | `lands` and `standing` never panic and always end. | `theorem lands.spec ... : lands ls l used ⦃ _ => True ⦄` | Yes, and it comes free with the other proofs. |

Decision 2 of 1.4 changes no theorem here: the shelf holds accepted stories, and the one-story rule lives in the addon (7.2).

The `used` list comes from the database (`uses_of`). No proof reads that glue. The tests of `story/stories.rs` cover it, as `lean/README.md` says for the other glue.

**The property test** (`crates/story/tests/properties.rs`): `the_shelf_keeps_each_number_once`. Any play of accepts, uses, and removals, with numbers from a small range, so that a number often comes again. The Lean theorems cover `lands` and `standing` for any length. The test covers the glue of `stories.rs` around them, which no proof reads.

### 7.2 Not in Lean

| Candidate | Where it runs | Decision |
|---|---|---|
| The waiting box: 1 for each author, 20 in all | Lua (`PlayerStories.lua`) | Property test over the real Lua (6.3). A Lean proof would prove a Rust copy. |
| A sender never has two stories waiting with one receiver | Lua, on two computers | The two-player property test of 6.3, with lost messages and reloads. It is a rule of two addons and a lossy channel, so the real Lua on both sides is the only honest model. Lean sees neither. |
| The drafts never pass 10 | Lua | Property test over the real Lua. |
| The escape round trip | Lua (`TaskWire.lua`) | Property test with every escaped byte, and the fuzz target `task_wire`. |
| The story text check (bytes, paragraphs, control characters) | Rust (`stories::checked_story`) | A property test and the fuzz target `story_accepted`. The check is a few lines over `str`. Aeneas models `str` and `char` poorly, and a proof costs far more than it finds. |
| The summary sees only accepted stories | Rust | The rule is now stronger: the summary reads no story. The property test `no_prompt_holds_the_text_of_a_story` checks every prompt of any play. A theorem would need the whole prompt builder in `crates/rules`, with strings and the world. |
| The wire size bound (the worst story fits 16 parts) | Lua | A unit test of the worst case, and a property test over the real Lua. The bound is arithmetic over three numbers, so the worst case is known exactly. |

## 8. Build order

Each step is one commit or a few, with its tests, and with its rules moved into `GAMEPLAY.md` and the README.

1. **Done. The fix of today's bugs.** The failing test `a_c1_control_character_is_not_clean_text` came first, then `IsCleanText` refuses U+0080 to U+009F. The bug of names that grow was already fixed by the alias table (1.5), so its test `a_story_full_of_names_at_the_limit_still_fits_the_desktop` is a regression test, not a failing one. The flaky tests of the review tool (`narrator_review.rs`) got a fix too: a model that never reads the prompt broke the pipe.
2. **Done. The shelf in `crates/rules`.** `story_shelf.rs` with `lands` and `standing`, the extraction, `lean/Timeways/StoryShelf.lean` with the theorems of 7.1, `lean/README.md`, and the property test `the_shelf_keeps_each_number_once`.
3. **Done. The desktop story.** The title, the paragraphs, the limits of the wire (1.5), `VERSION` 6, the journal, the tests of 6.1, the property tests `a_story_line_that_the_bridge_passes_lands_or_is_refused_with_its_reason` and `every_accepted_story_fits_one_page_of_the_journal` (in `through_the_bridge.rs`), and the fuzz target `story_accepted` with its seeds and dictionary.
4. **Done. The wire.** `StoryText.lua`, `StorySaved.lua`, `StoryDrafts.lua`, `story` with a title (`story_title` and `body` in `TaskWire`), the `%0A` escape, `story_ask` and `story_room`, one story for each author, the saved bounds of point 13, the Lua tests and properties, and the seeds of `task_wire` and `task_play`. A send marks an older told story to the same player as `lost`, because a story goes only after "open": so the sender never waits on two.
5. **Done. The Stories tab.** `JournalStories.lua`: the list, the reader, the badge, Block player, the Drafts group, `/stories`, and the new chat lines. The Hero page lost "Stories About You".
6. **Done. Hero cards.** `JournalHero.lua` builds the page and `JournalCards.lua` draws the left half: the sub-tabs with the button code of the top row, the cards two by two, the profile page with no Name field, "Also shared", and the preview. 6b edits inside the card: the box grows with its text, and the cards below it move (`a_box_that_grows_moves_the_cards_below_it`). Add a note keeps the writing page of the book.
7. **Done. The writing scroll.** `StoryScroll.lua`, Save and the drafts, the question when it opens and on Send, the errors with the selection, Ctrl+Enter, and `IsControlKeyDown` in the API gate and in `wow.yml`.
8. **Not done. Scroll rods.** Finding the wooden art needs the game client, and no test can see a texture. The scroll uses the parchment of the journal with no rods.
9. **Done. The summary samples.** The user approved the 6 lines below on 2026-10-05. They are in `crates/story/data/samples/summaries.txt`, and the prompt holds 2 of them in turn: the count of summaries so far picks them. Sample 1 shared "Deathknell has buried its dead" with a narrator sample, so the narrator sample became "Deathknell has twice buried its dead."
10. **Done. The summary.** The table `summaries` (`VERSION` 7), the call kind `summary`, the schedule, the prompt (`summary.rs`), `Call::Summary`, the check, the read rule, the journal field, the tests of `crates/story/tests/summary.rs`, the property test of the budget, the fuzz of the answer, the title page in the addon, and the steps in `TESTING.md` (21 and 22).

### The samples of step 9, for the user

Each sample comes from a sheet and the sagas only, and keeps the rules of the summary: one paragraph, at most 80 words, `$N` at most twice, never "our hero", no weather, no feeling, and nothing after the cutoff.

**Changed 2026-10-05, and approved by the user the same day.** The user said that the world is the main character, and that a clause where the hero only came somewhere is cringe (GAMEPLAY.md 3.2). Samples 2 to 6 had such a clause ("has walked the Barrens", "has since stood at Sentinel Hill", "crossed to Darkshore", "has since gone down into Gnomeregan", "has walked through the library"). Each one became a place that knows the deed. Sample 1 had none, and stays.

1. Deathknell has buried its dead twice, and $N, once a squire of the Silver Hand, was among those who climbed back out. The paladin keeps an oath the Light may no longer hear. Brill has come to trust the Forsaken who carries it, and the Scarlet recruits at Solliden know that face.
2. Razor Hill raised $N as it raises every orc child, with an axe and a watch post. In the Wailing Caverns, Mutanus the Devourer no longer stirs in the deep, and the Crossroads know which hunter to thank. A brother lost at Northwatch is the reason the hunter keeps going south.
3. Northshire Abbey trained $N to hold a sword before the Defias burned the fields of Westfall. Sentinel Hill counts the warrior among its militia, and Edwin VanCleef lies dead in the Deadmines below Moonbrook. A temper that the warrior owns to has not cost a fight yet.
4. The night elves of Shadowglen gave $N a duty to the trees of Teldrassil. Darkshore lost the furbolgs of the Blackwood to madness, and Auberdine has seen the druid fight it. The druid still searches the ruins of Ameth'Aran for a teacher who never came home.
5. Kharanos keeps the forge where $N first beat iron into a hammer. Gnomeregan has felt that hammer, and the troggs of Loch Modan know the dwarf who held the line at Thelsamar. An oath to an old clan still decides each road.
6. The Undercity sent $N out with a staff and a grudge against the Scarlet Crusade. Arcanist Doan's library in the Scarlet Monastery fell to the mage, and Southshore, in the hills of Hillsbrad, watches the Forsaken with good reason. Of the life before death, only the name of a sister remains.

### Deviations from this plan, and why

- **The desktop limits** are the limits of the wire (1.5).
- **`StoryText.FitsDesktop`** is gone (1.5). `StoryText.Problem` gives the reason, and `StoryText.BadSign` the place of the bad sign.
- **A draft keeps the text as the box shows it**, so a typed `|` stays `||`, and blank lines stay. Its check takes a `|` and twice the bytes, so a draft with a bad sign survives a reload.
- **Enter in the body inserts the break itself.** A blank line makes the same story, so the key works whether or not the game adds a break too (9, the first check in the game).
- **The selection of the bad sign counts bytes** (9, the second check in the game).
- **The summary waits while any finished chapter waits for its saga**, not only an older one. The rounds take the oldest chapter first, so this is the same rule, said more simply.
- **The badge** is a small red square with the count, from `SetColorTexture`. No atlas of a round badge is in the API gate.
- **With another roleplay addon**, the right half of the profile says "Other players see the profile of Total RP 3.", so the page is never empty.
- **The Chronicle with no chapter** opens on the title page, with "Fills in when your first chapter ends." The old line "Your story hasn't started yet." is gone.
- **The Remove popup of a story** shows its title, or its first paragraph.

### Fallback for point 4

If the release candidates count as live, keep `story` as it is for a story with no title and one paragraph of at most 400 bytes, and send every other story as `story2` (`id`, `title`, `text`). The receiver reads both. The scroll then needs to know whether the target reads `story2`. An older addon gives no answer to `story_ask`, so the scroll would read it as "no Timeways". That is why this plan prefers the change in place.

## 9. Open

- An in-game check: does a multi-line `EditBox` with an `OnEnterPressed` handler still insert a line break on Enter?
- An in-game check: do `HighlightText` and `GetCursorPosition` count bytes or letters in this client?
- The throttle of the logged channel in numbers (open in `docs/plans/logged-messages.md`). A story of 14 parts tests it.
- Stories in the summary and the narrator. The alias table exists now, so a story keeps IDs and could go into a prompt. The user decides (1.4, decision 5).
- The samples of the summary (step 9).
- The art of the scroll rods (step 8).
- Standing in the summary, after `docs/plans/standing.md` is built.
- Share the summary as the History of the profile, with "Changes when a chapter ends."
