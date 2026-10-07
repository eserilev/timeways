# Plan: the share card

Status: a spec, 2026-10-07, item 5 of the product review. Nothing is built. It needs no server.

## 1. Goal

A player copies one chapter or one tale of the Chronicle as a card, and shows it to friends. The card comes in two forms:

1. **Plain text**, for Discord or any chat outside the game.
2. **A frame in the game**, made for a screenshot.

The card is the small form of "Share a chapter" (`docs/plans/online.md` 23). That plan writes a full page on the desktop. The card stays in the game, and the addon builds it from the journal that it already holds.

## 2. What a card shows

All parts come from the `Chapter` or the `Tale` of the journal (`crates/story/src/journal.rs`). No model call makes a card.

| Part | From | Limit |
|---|---|---|
| The title | `title`, or the player's title of the edit | 60 letters |
| The kind and the number | "Chapter 8", or "Dungeon", "Raid", "Battleground" | one line |
| The character line | the level range, the race, and the class: "Level 18 to 24 · Human Paladin" | one line |
| The story | the shown text: the saga, the tale text, or the player's edit (`docs/plans/chapters.md` 11) | 600 characters |
| In this chapter | the 3 best deeds of the entry, as the journal words them | 3 lines |
| The credit line | only when the story quotes a lore passage (section 5) | one line |
| The mark | "Timeways" | one word |

An entry with no saga shows "In this chapter" with up to 6 deeds, and no story. A tale adds its run count: "3 runs".

The best deeds, in order: a class quest, a first kill of a boss or a rare, a revenge, a first epic item or mount, a first dungeon or raid, a title, a tenth level. The order is the rank of the narrator moments (`moments.rs`), so it needs no new rule.

## 3. Plain text, for Discord

The format is Markdown that Discord shows well, and that reads well as plain text too:

```text
**Westfall** · Chapter 3
Level 12 to 18 · Human Paladin

> Westfall's rich fields have lain fallow since the Second War. The Defias Brotherhood holds them now, and its foreman in Moonbrook is dead.

In this chapter:
- Defeated Foreman Thistlenettle
- Finished the quest "The Defias Brotherhood"
- Reached level 18

Timeways · lore from Wowpedia, CC BY-SA
```

- At most 1000 characters, so the card fits one message of Discord (2000) with room for a comment. The story is cut after its last whole sentence that fits.
- `$N` becomes the name of your own character, as on your screen.
- No line break inside the story. The quote mark `>` opens it once.

**The copy.** The game has no clipboard for an addon. So "Copy" opens a small box with the text selected, and the line "Press Ctrl+C to copy.". This is the box of the game that other addons use for a link. Escape closes it.

## 4. The frame in the game

A frame of its own, in the look of the journal, made for a screenshot. Mockup in words, about 420 by 560 pixels:

```text
┌──────────────────────────────────────────┐
│                 TIMEWAYS                 │  small bronze mark
│                                          │
│              Westfall                    │  title, large
│      Chapter 3 · Level 12 to 18          │  one small line
│          Human Paladin                   │
│ ──────────────────────────────────────── │
│  Westfall's rich fields have lain        │  the story, in the
│  fallow since the Second War. The        │  book font of the
│  Defias Brotherhood holds them now...    │  journal
│ ──────────────────────────────────────── │
│  In this chapter                         │  heading
│   • Defeated Foreman Thistlenettle       │
│   • Finished "The Defias Brotherhood"    │
│   • Reached level 18                     │
│                                          │
│  Lore from Wowpedia, CC BY-SA            │  credit, small grey
└──────────────────────────────────────────┘
```

- **The buttons** sit outside the frame, under it, so a screenshot holds no button: "Copy text" and "Close". The frame hides the other windows of Timeways while it shows.
- **Where it opens:** "Share" on the page of a chapter or a tale in the Chronicle. The place of the button waits for the design of `docs/plans/readable-chronicle.md`.
- **Size.** The frame keeps one size. A long story gets a smaller font, never a scroll bar, because a screenshot can't scroll.

## 5. Privacy and credit

- **Other players.** A real name of another player never goes on a card by default. Each player of the text shows as their card of the alias table: "an Undead Rogue" (`GAMEPLAY.md` 5.11, rule 8). The player can keep a name on purpose: the Share step lists each player of the entry, each one off until the player picks it, as in `online.md` 23.1. A player story (4.8) never goes on a card.
- **Your own name.** The card shows the name of your character, because you share it yourself. An option "Hide my name" shows the race and the class in its place.
- **Nothing leaves by itself.** The addon sends nothing for a card. The player copies the text or takes the screenshot.
- **The credit line.** Lore passages are CC BY-SA (5.10). When the story of the card holds a quoted run of 8 words or more from a lore passage of its prompt, the card adds "Lore from Wowpedia, CC BY-SA". The story program knows the passages of each call (`reads`), so the journal can mark an entry that needs the line. With no mark, the line stays off. The game text of a quest is no wiki text, and needs no line.

## 6. Ties to other plans

- **Wrapped** (`docs/plans/wrapped.md`). A card of Wrapped uses the same frame and the same plain text format, with its own parts. The "Share" of Wrapped is this card.
- **Timeways online** (`docs/plans/online.md` 23). Later, "Share online" on the same panel publishes the card as a page. The card is the head of that page. The page adds the map and the screenshots. The `ChapterShare` object of format 1 gets the three best deeds and the credit mark, so the card and the page agree.

## 7. Tests, when it is built

- `a_card_never_shows_another_players_name_unless_picked`
- `the_plain_text_card_fits_one_discord_message`
- `the_credit_line_shows_only_when_lore_is_quoted`
- `an_entry_with_no_saga_shows_its_deeds`
- The addon tests: the frame holds no button, and Copy selects the whole text.

## 8. Open questions

1. Does a card of the summary (the title page) come too?
2. Does "Hide my name" start on or off?
3. The size of the frame on a small screen.
