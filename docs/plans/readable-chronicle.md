# Plan: a readable chronicle and Knowledge by place

Status: spec only, 2026-10-03. Nothing is built, and nothing gets built from this text alone.

**This feature needs a lot of design work before any build.** The user approved a direction, not a design. Before any code, we make mockups, show them to the user, and change them over several rounds. We also test a prototype with players (section 7). The build plans (section 11 and section 12.8) apply only after the user approves one design. Section 12, Knowledge by place, is in the same state: a spec only, with design work before any build. When a part is built, its rules move into `GAMEPLAY.md` 3.3 and 3.6, and this plan marks the part as done.

## 1. Goal

The Chronicle tab (GAMEPLAY.md 3.6) gets three new tools:

- **A timeline of chapters.** Big moments get a mark: the first dungeon, a rare killed, a level milestone, a choice quest, a death.
- **Filters.** Deaths only, quests only, one zone, chapters with a written story.
- **Map links in both directions.** A chapter shows where it happened on the journal map. A place on the map lists its chapters.

The chronicle keeps its rules. The code still decides where a chapter begins (3.3). No model takes part in a page.

A second part, Knowledge by place (section 12), uses the same map links for what you read and heard.

## 2. The problem

The chronicle gets one chapter for each milestone (3.3). Each first visit of a zone, each tenth level, and each first kill of a rare or a boss can start a chapter. After a month of play, a character has 50 to 100 chapters. The list names each chapter only by its number and its first zone: "Chapter 37: Duskwood". Players get real history, but they cannot find anything in it. Dwarf Fortress has the same complaint about its legends mode.

Concrete player problems:

1. **"When did I first die?"** Deaths hide in the "What you did" part of each chapter. The player opens chapters one by one.
2. **"What happened in Westfall?"** A chapter is named by its first new zone. A chapter that went back to Westfall has another name. Nothing lists all the Westfall chapters.
3. **"Where was that?"** A chapter page shows the map of its first zone only. It shows no pin for the people you met or the place where you died.
4. **"Which chapters are worth reading?"** A chapter with a written story and a chapter with a plain list look the same in the list.
5. **"How far have I come?"** The list has no shape. Level 10, the first dungeon, and level 30 look like every other row.
6. **"Show a friend my best moments."** The player cannot pull out the highlights.
7. **Deeds get cut.** Each list of a chapter keeps at most 20 entries (3.3). A death after the 20th deed of a busy chapter shows only as "And 3 more." A filter on the chapter lists alone misses it.

## 3. What the journal reply carries today

The desktop sends the whole journal each time the book opens. It cuts the journal into pages, and each page fits in one reply (`crates/story/src/journal.rs`, `pages`). A page line holds at most 24,576 bytes (`MAX_LINE`), and a list on one page holds at most 200 items. The addon joins the pages. So every chapter, place, person, and deed is in the addon when the book shows. A filter needs no new request.

| List | Fields today | Limit |
|---|---|---|
| `chapters` | `number`, `began`, `ended`, `zones` (first visits of zones only), `people` (first meetings), `deeds`, `left_out`, `prose` (the story, 600 characters at most), `footnotes` (3 at most, 200 characters each) | 20 entries in each list of a chapter |
| `places` | `name`, `kind` (zone, dungeon, raid, capital), `within` (the zone of a subzone), `first_visit`, `spot` (map id, x, y of the first visit with a position) | 200 items for each page |
| `people` | `name`, `place`, `first_met`, `trust`, `slapped`, `spot`, `trust_why` | 200 items for each page |
| `deeds` | One of: `level`, `defeated` (with `times`), `titled`, `quest_done`, `game_quest_done`, `class_quest_done`, `quest_marked`, `died` (with `killer`). Each has `at` and `place` (the place name where you stood, often a subzone). | 200 items for each page |

Gaps:

- **No mark of a big moment on a chapter.** The addon can rebuild some marks from the full `deeds` and `places` lists by tick. That puts game rules into Lua, and the narrator already ranks big moments in Rust (3.2).
- **No rare or boss split.** `defeated` does not say if the foe was a rare or a boss.
- **No choice quests.** They come with `docs/plans/standing.md`, which is not built.
- **No list of the zones where a chapter took place.** `zones` holds only first visits. A chapter that went back to Westfall does not name Westfall.
- **No position of a death.** The addon sends a position only with `zone_entered` and `npc_met` (3.6). A deed carries a place name, not a spot.
- **No reason why a chapter began.** The code knows the milestone, but the reply does not carry it.

## 4. What each view needs

| View | Data it needs | Carried today? | New bounded field |
|---|---|---|---|
| Timeline with marks | The big moments of each chapter | Partly, through `deeds`, but cut at 20 | `marks`: a list of `Mark`, at most one entry for each kind of mark (about 10) |
| The milestone that opened a chapter | The milestone kind and its name | No | `opened_by`: one `Mark`, or none for chapter 1 |
| Filter: deaths | The deaths of each chapter, also past the 20 cap | Partly | `counts.deaths`, a whole number |
| Filter: quests | The quests of each chapter, also past the 20 cap | Partly | `counts.quests`, a whole number |
| Filter: one zone | Every zone where the chapter took place | No | `played_in`: zone names, at most 20, like `zones` |
| Filter: written story | `prose` | Yes | None |
| Chapter to map | The spots of its zones and people | Yes, by a join on the names in `places` and `people` | None. Death pins need `spot` on `died` (a new event field and a new fact). |
| Map to chapters | For each zone, its chapters | Yes, from `played_in` once it exists | None |

All new fields are bounded: an enum with a fixed number of kinds, a whole number, or a list capped at 20 names. So a chapter still fits on one page. The budget: about 10 marks at about 40 bytes, 2 counts, and 20 names of at most `MAX_NAME_BYTES`. That is under 2 KB for each chapter. 100 chapters add about one page to the reply.

A sketch of the new fields, for discussion only:

```rust
pub struct Chapter {
    // ...the fields of today...
    pub opened_by: Option<Mark>,
    pub marks: Vec<Mark>,       // one for each kind at most
    pub played_in: Vec<String>, // at most CHAPTER_LIST
    pub counts: Counts,         // deaths, quests
}

pub enum Mark {
    FirstDungeon { place: String },
    FirstRaid { place: String },
    FirstCapital { place: String },
    FirstKill { foe: String },  // a rare or a boss
    Level { level: i64 },       // every tenth level
    ClassQuest { title: String },
    Title { title: String },
    Died { times: u32 },
    Choice { title: String },   // after standing.md
}
```

## 5. Alternative designs

The frame stays as it is: tabs on top, the map on the left, the parchment on the right, buttons at the bottom (3.6). Every design keeps Previous chapter and Next chapter. The sketches show the journal at its real proportions, without detail.

### 5.1 Design A: marked list

The list over the map stays. Each row gets small icons for its marks on the right side, where the list already keeps room for a mark (`MARK_WIDTH`). Headers split the list by level band ("Levels 1 to 9", "Levels 10 to 19"). A Filter button above the list opens a menu with checkboxes, as in the Collections window of the game.

```
+-- Chronicle ------------------------------------------------------------+
| [Filter v]               |                                              |
| Levels 20 to 29          |  Chapter 37                                  |
|  Ch. 41  Duskwood   [x]  |  Duskwood                                    |
|  Ch. 40  Redridge  [30]  |  12 Oct 2026.                                |
|  Ch. 39  Deadmines [D]   |  The fog came in early that night...         |
|  Ch. 38  Westfall        |                                              |
| Levels 10 to 19          |  Places you visited                          |
|  Ch. 37  Duskwood  [*]   |  Duskwood and Darkshire.                     |
|      ( map behind )      |                                              |
+--------------------------+----------------------------------------------+
|                 [Previous chapter]  Chapter 37 of 41  [Next chapter]    |
+-------------------------------------------------------------------------+
  [x] a death  [30] a level milestone  [D] a first dungeon  [*] a story
```

- **Good:** the smallest change. It uses the list, the rows, and the float box that exist. The player learns nothing new. The filter menu is a known WoW pattern.
- **Bad:** it is still a list. 100 rows still scroll. The level bands help with "how far", but not with "when". The float box is 250 wide, so a row has room for two or three icons at most.

### 5.2 Design B: timeline strip under the map

A thin strip runs across the bottom of the map pane, over the band of visited subzones. It shows one notch for each chapter, oldest on the left. A big moment gets a taller notch with its icon. A hover shows a tooltip: "Chapter 39: The Deadmines. First dungeon." A click opens the chapter. A filter dims the notches that do not match, so the shape of the whole story stays. A level ruler sits under the notches.

```
+-- Chronicle ------------------------------------------------------------+
|                                     |  Chapter 39                       |
|          ( map of the zone,         |  The Deadmines                    |
|            pins of the chapter )    |  9 Oct 2026.                      |
|                                     |  ...                              |
|  |  |  ||  D| | x |  30 |||  |  ^   |                                   |
|  1     10        20       30     40 |                                   |
+-------------------------------------+-----------------------------------+
| [All] [Deaths] [Quests] [Has a story] [Zone v]                          |
|                 [Previous chapter]  Chapter 39 of 41  [Next chapter]    |
+-------------------------------------------------------------------------+
```

- **Good:** the whole story is visible at once. "How far have I come" gets a real answer. A filter shows a pattern ("I died a lot in Duskwood"). The map stays free of a list box.
- **Bad:** a new widget, so more code and more tests. At 580 wide, 100 chapters give about 5 pixels for each notch. After 200 chapters, the strip needs a zoom or a scroll. A thin notch is hard to hit with the mouse. The strip takes room from the visited subzones line, which then moves.

### 5.3 Design C: map first

The map pane is the index. On the zone map, each chapter that took place in the zone gets a pin with its number, at the first-visit spot of its place. A zone dropdown above the map picks the zone, and lists only zones with chapters, with a count: "Westfall (4)". A click on a pin opens the chapter. A small list under the dropdown repeats the chapters of the zone, for the players who do not want to aim at pins. Marks show on the pins.

```
+-- Chronicle ------------------------------------------------------------+
| [Westfall (4) v]                    |  Chapter 22                       |
|        (22)                         |  Westfall                         |
|               (15)                  |  ...                              |
|     (9)            (31)             |                                   |
|    ( Westfall map art )             |                                   |
| Chapters here: 9, 15, 22, 31        |                                   |
+-------------------------------------+-----------------------------------+
|            [Timeline]  [Previous chapter] 22 of 41 [Next chapter]       |
+-------------------------------------------------------------------------+
```

- **Good:** "What happened in Westfall?" takes one click. It matches how players remember: by place. It uses the real map, which the user asked for.
- **Bad:** it is weak for "when". Pins of several chapters pile up on one spot, because a place keeps the spot of its first visit only. A dungeon has no point on its zone map, so its pins sit on the dungeon map. It needs Design A or B beside it for the time view, so it is the most work.

### 5.4 Design D: highlights first

The list shows only chapters with a mark, plus the newest chapter. Quiet chapters fold into one row: "4 more chapters". A click unfolds them. A toggle at the top switches between Highlights and All. Filters are a short row of toggle buttons.

```
+-- Chronicle ------------------------------------------------------------+
| [Highlights] [All]       |  Chapter 39                                  |
|  Ch. 41  Duskwood   [x]  |  The Deadmines                               |
|  Ch. 40  Level 30        |  First dungeon.                              |
|  Ch. 39  Deadmines [D]   |  ...                                         |
|    4 more chapters       |                                              |
|  Ch. 34  Level 20        |                                              |
|      ( map behind )      |                                              |
+--------------------------+----------------------------------------------+
| [Deaths] [Quests] [Has a story]  [Previous chapter] 39 of 41 [Next...]  |
+-------------------------------------------------------------------------+
```

- **Good:** a month of play shrinks to 10 or 15 rows. Problem 6 ("show a friend") is solved by default. It needs the marks, but no new widget.
- **Bad:** it hides chapters, and a player can feel the quiet chapters are lost. The choice of what is "big" becomes a design decision with real weight. A fold row is a new kind of row in `JournalList`.

### 5.5 Filters: three ways to show them

| Way | Looks like | Good | Bad |
|---|---|---|---|
| Filter button with a menu | The Filter button of the Collections window | Known to WoW players. Takes one button of room. Holds the zone list too. | Hidden: the player does not see a filter is on, unless the button says so ("Filter (2)"). |
| Toggle buttons in the bottom bar | The dark red buttons of the bar | Always visible. One click. | The bar holds Previous chapter and Next chapter. It has room for three or four short labels at most. |
| Tabs inside the Chronicle | Small tabs above the list | Very clear. | A second row of tabs under the main tabs looks busy. One filter at a time only. |

The zone filter has many values, so it needs a menu in each design.

### 5.6 Combinations

The designs mix. Likely pairs to show the user: A with the Filter button (the cheapest), B with toggle buttons (the most new), C with A (place first, list as backup). The user picks after the mockups.

## 6. Trade-offs in short

| | A: marked list | B: timeline strip | C: map first | D: highlights |
|---|---|---|---|---|
| "When did I first die?" | Filter, then top row | Filter, then the first lit notch | Weak | Filter, then top row |
| "What happened in Westfall?" | Zone filter | Zone filter | One click | Zone filter |
| "How far have I come?" | Level headers | Strong | Weak | Medium |
| Works at 200 chapters | Scrolls | Needs zoom | Yes | Yes |
| New widgets | None | Strip, notches | Chapter pins, dropdown | Fold row |
| Build size | Small | Medium | Large | Small to medium |

## 7. What to prototype first, and how to test it

**Step 1: mockups, not code.** Build HTML mockups of A, B, C, and D in one artifact canvas, in the style of the approved journal design (memory note: journal redesign). Fill them with a fake month of play: 60 chapters, 12 zones, 2 dungeons, 9 deaths, 3 written stories in 4 chapters. Generate the data from a test world in `crates/story/tests`, so the numbers are real shapes, not guesses. The mockup map is a drawing, because no Blizzard art goes in an artifact. The addon uses the real map.

**Step 2: rounds with the user.** Show the four designs. Expect at least three rounds: pick a direction, fix the layout, fix the copy. Each round changes the mockup only.

**Step 3: a throwaway prototype in the game.** After the user picks a design, build the cheapest part of it in Lua on a branch, from the data of today (the full `deeds` and `places` lists, joined by tick). No desktop change. This tests the feel, the frame size, and the mouse targets, with the real map. The branch does not merge. The real build computes marks in Rust (section 9).

**Step 4: test with players.** Use 3 to 5 players with at least two weeks of play. Ask each one to think aloud, with no help, on their own character:

1. Find the chapter where you first died.
2. Find everything that happened in one zone that you pick.
3. Find the chapter where you reached level 20.
4. Show me the chapter you like best.
5. Read one story chapter.

Measure the time and the clicks for each task, on the chronicle of today and on the prototype. Goal: tasks 1 to 3 in under 20 seconds and under 4 clicks. Watch for misclicks on small targets, and for players who do not find the filter. Ask one question at the end: "What did you expect to find and didn't?"

## 8. Open design questions for the user

1. Which design, or which mix, do you want to see in more detail?
2. What counts as a big moment? The list in section 4 has 9 kinds. Is a joke title big? Is every death big, or only the first one and the funny ones (5.4.1)?
3. Does a chapter keep its title "Chapter 37: Duskwood", or does the milestone name it ("Chapter 39: First dungeon")?
4. What do players call a chapter with a written story? "Saga" is our word. Options: "Has a story", "Written", or an icon with no word.
5. Can filters combine (deaths in Westfall), or is one filter on at a time?
6. Highlights (Design D): is hiding the quiet chapters fine, or must every chapter stay in view?
7. Deaths on the map: do we add a position to each death (a new event field from `PLAYER_DEAD`, and a new fact)? Or do we pin the death at the spot of its subzone, which is close but not exact?
8. The rare or boss split: is "a rare killed" its own mark, apart from a boss? That needs a new fact.
9. Map to chapters: is a pin on the zone map enough, or do you want a continent view that shows the zones with chapters? A continent view is new map code.
10. Does the book still open on the newest chapter, or on the timeline?

## 9. Rules that apply to the copy

The UI copy rules of `CLAUDE.md` apply. In short:

- No internal words. Never "milestone", "tick", "fact", "saga", "batch", "mark", "slot", or "bard".
- Title case only where WoW uses it: the tab ("Chronicle"), headings ("Chapter 37"), and the window title. Lines and buttons use sentence case.
- Buttons: one or two words, starting with a verb where it fits: "Filter", "Show all", "Clear".
- One name for each thing: "chapter", "quest", "death". A player quest is a "quest" too.
- Key fact first. A tooltip is a few words: "First dungeon", "Level 30", "Died 3 times".
- An empty filter says what is missing in plain words, without blame.

Copy to start the rounds with (not final):

| Where | Bad | Good |
|---|---|---|
| Filter menu | Deed type: death | Deaths |
| Filter menu | Chapters with prose | Has a story |
| Filter menu | Zone filter | Zone |
| Tooltip on a notch | Milestone: level 30 reached | Level 30 |
| Tooltip on a notch | Instance (party) first entered | First dungeon: The Deadmines |
| Empty filter | No chapters match the current filter criteria. | No deaths yet. |
| Fold row (D) | 4 chapters collapsed | 4 more chapters |
| Map pin tooltip | Chapters linked to this place: 9, 15 | Chapters 9 and 15 |
| Filter button with a filter on | Filter [active] | Filter (2) |

## 10. Constraints of the WoW UI

- **Frame size.** The journal is 980 by 520 UI units, so it fits a screen of 1024 by 768 (`JournalFrame.lua`). The map pane is 580 wide. The body is about 400 tall. The parchment is about 370 wide. The float list is 250 wide. A new design fits in this frame. It does not grow the window.
- **Mouse targets.** A row or a notch is at least 12 units wide and 16 tall. Below that, players miss it. This limits Design B to about 45 notches without zoom.
- **The map pane.** It shows the real art of one zone (`C_Map`) with explored parts and pins (`MapPane.lua`). A dungeon has its own map, and its pins stay on it. A place keeps the spot of its first visit only. A continent view is new code: art of the continent, and a zone under the mouse from `C_Map.GetMapInfoAtPosition`. The visited subzones line uses the bottom 22 units of the pane.
- **Combat.** The journal opens only when the player asks. It never opens or redraws itself in combat. It touches no protected frame and calls no protected function, so it adds no taint. Filters and the timeline run in plain Lua on data that is already loaded. A filter change redraws the list only, and the map art redraws only when the zone changes.
- **Menus.** The menu API of the game differs between clients, and the old `UIDropDownMenu` spreads taint when an addon uses it. The prototype checks which menu API this client has, before a design depends on a menu.
- **Load size.** Every open sends the whole journal. The new fields add about one page for each 100 chapters (section 4). The paging of today handles it.
- **Escape closes the journal.** A filter menu closes first, then the journal, as in the game.

## 11. Rough build plan, after the user approves a design

This order assumes a mix of A and B. It changes with the design.

**Desktop (`crates/story`):**

1. `Mark` and `opened_by`: compute them in `journal.rs` from the deeds and places of the chapter range, before the 20 cap. Reuse the big-moment ranking of the narrator (3.2) where it fits.
2. `counts`: deaths and quests of the chapter range, before the cap.
3. `played_in`: the zones of each `located_in` change in the chapter range, capped at 20, with the rest in `left_out`.
4. Optional, after open questions 7 and 8: a death spot and the rare or boss split. Each is a new fact in `vocabulary.rs` and a new field of an event of the addon. The new field gets a fuzz seed in `fuzz/`.

**Addon (`addon/Timeways`):**

1. Read the new fields in `Journal.lua`, with the same type checks as today.
2. The filter state and the filter logic, in a new file `ChronicleFilter.lua`, one idea in one file.
3. The marks on the rows (`JournalList.lua`), or the strip, in a new file `Timeline.lua`.
4. Chapter pins on the map (`MapPane.lua`): the spots of the zones and people of the chapter, with the existing pin code.
5. Map to chapters: a tooltip on a zone pin, or the zone menu.

**Tests:**

- Named tests in `crates/story/tests/journal.rs`, one for each rule: `a_first_dungeon_marks_its_chapter`, `a_death_past_the_cap_still_counts`, `a_return_to_a_zone_lists_it_in_played_in`, `the_first_chapter_has_no_opening_milestone`.
- Property tests in `crates/story/tests/properties.rs`: every mark has its deed in the range of its chapter; the counts are never below the deeds that show; a chapter with every list full still fits on one page. Make the edge of 20 entries likely in the generator.
- Addon tests in `crates/addon-tests/tests/journal.rs`: the deaths filter shows only chapters with a death; an empty filter shows its line; a click on a mark opens its chapter; a chapter shows the pins of its places; the journal does not redraw in combat.
- The checks of `CLAUDE.md` before each commit.

**Docs:** move the built rules into `GAMEPLAY.md` 3.3 and 3.6, and mark the parts done here.

## 12. Knowledge by place

Status: spec only, like the rest of this plan. The user approved the direction on 2026-10-03. It needs mockups and rounds with the user before any build.

The user, after play on 2026-10-04: "knowledge needs to be more tangible". They want cards of facts for each place or person in place of a list of texts. Since then, talk answers are no rumors on the Knowledge page: they live in the talk window (GAMEPLAY.md 3.5).

### 12.1 Goal

- **A place on the map shows everything you know about it.** A click on a place of the journal map lists the texts you read there (books, quest texts, gossip), the rumors you heard there, the people you met there, and the chapters set there. This is the map-to-chapters link of section 1, with more in it.
- **The Knowledge tab groups by zone:** "Elwynn Forest: 12 texts, 3 rumors".
- **A "read" mark on a tooltip.** When the mouse is over a book in the world or in your bags, the tooltip says that you already read it, as the addon Book Archivist does.
- **Completion only for sets that a player can finish.** A count such as "3 of 12" shows only for a set from a trusted list, for example the History of Warcraft books (GAMEPLAY.md 5.10). Never for a set that we cannot stand behind: texts of the other faction, removed quests, or "all gossip". SWTOR's Codex made this mistake: it showed counts that players were not able to complete, and players felt cheated.

### 12.2 The player problems

1. **"What do I know about Duskwood?"** The Knowledge page is one long list by date. The player scrolls and reads each row.
2. **"Did I read this book already?"** A bookshelf in a city holds several books. The player opens each one to find out.
3. **"Who told me about the Defias?"** The rumor is in the list, but nothing ties it to the map.
4. **"Is there more to find here?"** For most texts, nobody can answer this honestly. For a small trusted set, we can.

### 12.3 What the journal reply carries today

| Data | Where | Fields | Has a place? |
|---|---|---|---|
| A text you read (book, quest, gossip) | `learned` list, from `Read` in `learned.rs` | `kind`, `title`, `npc`, `place`, `at`, `excerpt` (240 characters) | Yes: `place` is the zone where you read it (`GetRealZoneText` in `Seen.lua`). A text read again in another place is the same text (`SeenIndex`), so it keeps the first zone. |
| A rumor from `/talk` | `learned` list, from `Rumor` | `kind = rumor`, `npc`, `at`, `excerpt` | **No.** `place` is always none. The NPC has a place in `people`, so a join by name gives it. |
| A person you met | `people` list | `name`, `place`, `spot` | Yes, often a subzone. `places.within` gives its zone. |
| A chapter | `chapters` list | `zones` (first visits only) | Only first visits. `played_in` (section 4) fixes this. |
| A place | `places` list | `name`, `kind`, `within`, `spot` | Yes. |
| A read book, in the addon | `Seen.lua` keeps a `sent` table for one session only | the kind, title, NPC, and text | Not saved. The tooltip mark needs a saved list. |
| A trusted set | The lore pack on the player's computer holds the History of Warcraft chapters I to V (5.10) | the books of each chapter | Not in the journal reply. |

So the place view and the zone groups work mostly from data the addon has today, by joins on names. Two gaps are real:

- **A rumor has no place.** Option 1: the join through the NPC in `people`. Option 2: a new `place` on `Rumor`, from the zone of the `/talk` input. Option 2 is exact, and a bounded name.
- **A place of a subzone.** A text keeps its zone, but a person keeps a subzone. The view groups by zone, and uses `within` for the join.

New bounded fields, if the design needs them:

| Field | Where | Bound |
|---|---|---|
| `place` on a rumor | `Rumor` and the `learned` entry | One name of at most `MAX_NAME_BYTES` |
| `sets`: the progress of each trusted set | A new short list of the journal | One item for each set in the trusted list (fewer than 10). Each item: a name, a found count, a total. |
| A saved list of read books in the addon | The saved variables of the character | Book titles only, at most 2000 titles. Not in the reply. |

The `learned` list grows with every gossip text. After a month it holds hundreds of entries. The paging of today handles it, but the reply gets longer at each open. This is a risk to watch, and not a new problem of this part.

### 12.4 The "read" mark on a tooltip

What the addon has:

- `Seen.Book` reads a book with `ItemTextGetItem()` (its title) and `ItemTextGetText()`. A letter of a player has a creator, and stays out.
- `Trust.lua` adds lines to the tooltip of an NPC with `TooltipDataProcessor.AddTooltipPostCall(Enum.TooltipDataType.Unit, ...)`. `addon/tests/api.lua` lists `TooltipDataProcessor`, `Enum.TooltipDataType.Unit`, and `GameTooltip`.

What it needs:

- **A book in your bags or a link:** `Enum.TooltipDataType.Item`. It is not in `addon/tests/api.lua` today. The prototype checks that this client has it.
- **A book on a shelf or a lectern in the world:** a game object. The tooltip of a game object is `Enum.TooltipDataType.Object` in clients that have it. The tooltip data gives the name on the first line. Some clients give no id for an object. So the match is by the title: the title of the tooltip against the titles of the books you read. Fallback: `GameTooltip:HookScript("OnShow", ...)` and the first line of the tooltip.
- **A saved list of read books.** The desktop has the texts, but a tooltip cannot wait for the desktop. So the addon keeps the titles of the books it read in its saved variables, per character. Each new book adds its title when `Seen.Book` sends it.
- **The trap of two books with one title.** Some objects in the world share a name ("Old Book", "Journal"). For a title on a list of generic names, the mark shows nothing rather than a wrong "read".
- **A quest giver or a gossip NPC:** the tooltip of a unit already carries the trust line. A "You heard their story" line is possible there with no new API. This is an open question (12.7).
- **Combat:** a tooltip post call adds one line from a table that is in memory. It does no work that grows with the list, and it does nothing in combat that it does not do out of combat.

Copy for the line on the tooltip: "Read" or "Already read", in the faded color of the game, as the game shows "Already known" on a recipe. "Already known" is the closest WoW word. The rounds decide.

### 12.5 Completion: only for trusted sets

Rules:

- A set shows a count only when it comes from a trusted list that a person checked. The first set: the History of Warcraft books, chapters I to V (5.10). Each chapter is a set.
- A set is trusted only when every book in it is in the game of this client, and a player of either faction can read it. The check happens once, by a person, before the set goes on the list.
- No count shows for quests, gossip, NPCs, or zones. Their totals depend on faction, class, phase, and removed content, so no total is honest.
- A set that we stop trusting goes off the list. The count disappears. It never shows a wrong total.
- The total comes from the trusted list, not from the lore pack. The pack lives on the player's computer, and its contents depend on the dump. The list ships with Timeways as titles only, with no Blizzard text.
- A finished set is a moment: a line in the journal, and a big moment for the narrator. This is open question 12.7.5.

Copy: "History of Warcraft: 3 of 5" on the Knowledge page. A finished set: "You've read all of History of Warcraft." No count anywhere else.

### 12.6 Alternative designs

#### 12.6.1 Design K1: place card on the parchment

A click on the map (on a pin of a place, or on the zone itself) opens a place card on the parchment. The card has sections: What you read, What you heard, People, Chapters. Each row opens its own page (the text in Knowledge, the chapter in Chronicle).

```
+-- Knowledge ------------------------------------------------------------+
|                                     |  Duskwood                         |
|      ( Duskwood map,                |  What you read (4)                |
|        pins of places with          |   - The Tale of Stalvan           |
|        something to read )          |   - Wanted: Stitches              |
|                                     |  What you heard (2)               |
|                                     |   - Only a rumor. Abercrombie...  |
|  Visited: Darkshire, Raven Hill     |  People (6)  Chapters (3)         |
+-------------------------------------+-----------------------------------+
|                         [Back]                                          |
+-------------------------------------------------------------------------+
```

- **Good:** one place, everything you know. It uses the parchment as a page, as every page does today.
- **Bad:** the map needs a hit target for a place. A pin exists only for places with a spot. A click on the zone art needs `C_Map.GetMapInfoAtPosition` and new code.

#### 12.6.2 Design K2: Knowledge list grouped by zone

The Knowledge list groups by zone with a header and counts. A header folds. A click on a header shows that zone on the map and the place card of K1 on the parchment.

```
+-- Knowledge ------------------------------------------------------------+
| Elwynn Forest   12 texts, 3 rumors  |  Elwynn Forest                    |
|   - A Threat Within                 |  12 texts, 3 rumors, 9 people     |
|   - The Stolen Tome                 |  ...                              |
| Westfall         7 texts, 1 rumor   |                                   |
| Stormwind City  20 texts            |                                   |
|   History of Warcraft: 3 of 5       |                                   |
|        ( map behind )               |                                   |
+-------------------------------------+-----------------------------------+
```

- **Good:** works without the map, and a zone answers "what do I know about it" in one click. Smallest step from the page of today.
- **Bad:** the list loses its order by date. A player who wants "what did I read last night" needs a sort switch.

#### 12.6.3 Design K3: map pins for knowledge

On the map of a zone, each place where you read or heard something gets a small book or speech pin. A hover lists the titles. A click opens the text. A toggle above the map switches pins on or off: chapters, knowledge, both.

```
+-- Knowledge ------------------------------------------------------------+
| [Chapters] [Knowledge]              |                                   |
|        [b]  (b = something read)    |  The Tale of Stalvan              |
|              [r] (r = a rumor)      |  Read in Duskwood, 12 Oct 2026.   |
|     [b]                             |  ...                              |
|    ( Duskwood map art )             |                                   |
+-------------------------------------+-----------------------------------+
```

- **Good:** the map becomes the index, as players remember places.
- **Bad:** a text has a zone, not a spot. A pin needs a spot: the spot of the NPC who gave it (from `people`), or a new position sent with each seen text. A book on a shelf has no NPC, so it has no spot today. Many pins pile up in a capital city.

### 12.7 Open design questions

1. K1, K2, K3, or a mix? K2 with K1 as its page is the smallest. K3 needs positions.
2. Does a rumor get its own place (a new field), or the place of its NPC?
3. Does a seen text get a position, so it can have a pin on the map (K3)? That is a new field on the `seen` input, and its fuzz seeds.
4. The tooltip line: "Read", "Already read", or "Already known"? And for NPCs: a line "You heard their story"?
5. A finished set: only a line on the Knowledge page, or also a big moment for the narrator and a chapter mark?
6. Which sets are on the trusted list after History of Warcraft? Each needs a person to check it.
7. Does the place card show texts of a subzone under its zone, or the subzone as its own card?
8. Knowledge sort: by zone only, or a switch between "By place" and "By date"?

### 12.8 Rough build plan, after the user approves a design

**Desktop (`crates/story`):**

1. `place` on `Rumor`, from the zone of the `/talk` input. A migration fills old rumors with none.
2. The trusted set list as data in `crates/story/data`, titles only. A function that counts the books of each set that the player read, and a `sets` list in the journal, with at most one item for each set.
3. Optional, after open question 3: a position on a seen text.

**Addon (`addon/Timeways`):**

1. The zone groups and the counts in the Knowledge page (`Journal.lua`), from the `place` of each entry and `within` of `places`.
2. The place card, as a page of its own, in a new file `PlaceCard.lua`.
3. The saved list of read book titles, in a new file `ReadBooks.lua`, filled from `Seen.Book`.
4. The tooltip post calls for items and objects, after the prototype confirms the APIs in this client. Add the APIs to `addon/tests/api.lua`.

**Tests:**

- Rust, named: `a_rumor_keeps_the_zone_where_you_heard_it`, `a_set_counts_only_books_on_its_list`, `a_set_off_the_trusted_list_shows_no_count`, `a_book_read_twice_counts_once`.
- Property test: the found count of a set is never above its total.
- Addon tests in `crates/addon-tests/tests`: a read book gets the mark on its tooltip; an unread book gets none; a generic title gets none; the Knowledge page groups by zone with the right counts; a click on a zone header opens the place card; the read list survives a reload.
- A fuzz target or seed for any new field that the desktop reads from the addon (a rumor place, a text position).

**Docs:** move the built rules into `GAMEPLAY.md` 3.1.1, 3.6, and 5.10.
