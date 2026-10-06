# Plan: Timeways Wrapped

Status: a high-level idea, 2026-10-06. Nothing is built. The user wants it for later.

## The idea

At the end of each year, every character gets a recap of their year, like Spotify Wrapped. It is a short run of cards. The player clicks through them, and can share them.

## When

- It opens once from the last week of December. A chat line says it is ready.
- The player can open it again from the Chronicle at any time.
- A character with too little play in the year gets no Wrapped (a minimum to set in play tests).
- Later: a Wrapped for a character's whole life, at level 60 or on request.

## The cards

Each card holds one fact or one small set of facts. Counts are allowed here, because Wrapped is a recap, not a narrator line.

| Card | What it shows | Source today |
|---|---|---|
| Your year | Levels gained, from what to what, and the zone where the year began and ended | Level deeds, chapters |
| Your chapters | The number of chapters, and the one with the most weight, with its title | `docs/plans/chapters.md` |
| Your turning points | The firsts of the year: first mount, first epic item, first dungeon, first raid, first capital | Turning points, firsts |
| Your nemesis | The foe that killed you most, and if you took revenge | Death records, revenge |
| Your tales | The dungeon or raid you ran most, its run count, and your first clears | Tales and their visits |
| Your places | The zone where you spent the most chapters, and the number of zones you saw | Chapters, `zones_played` |
| Your people | The NPC you talked to most, and the NPC who trusts you most | Talks, trust |
| Your stories | The stories that other players told about you, and the ones you told | `docs/plans/hero-stories.md` |
| Your reading | The books and lore you found, as a count and one title | `learned` rows |
| The close | One narrated line about the year, in the narrator's voice | One model call |

Each card is left out when its facts are empty, so no card says "0".

## Rules

- **Only real facts.** Every number and name comes from the world of the character. A model writes only the closing line.
- **The closing line** follows the narrator voice skill (`.claude/skills/timeways-narrator-voice`) and the checks of the narrator. It is one call for each Wrapped, and its text is kept, so the Wrapped reads the same each time.
- **Deterministic.** The same world gives the same cards. The facts come from a pure function over the year's events, so it can live in `crates/rules` and have Lean proofs: for example, every count on a card equals the count of its events in that year.
- **Private by default.** Real player names show only on the player's own screen (`GAMEPLAY.md` 5.11). A shared Wrapped shows the alias cards of other players, never their names.
- **UI copy rules** of `CLAUDE.md` hold for every card label.

## Sharing

- In the game: a chat line, and a "Share" that posts one short line to the group or the guild.
- Later, with Timeways online (`docs/plans/online.md`): a link to a web page of the cards, and a picture to post.

## Open questions

1. Which year: the calendar year, or a year from the character's creation?
2. The minimum play for a Wrapped.
3. Which cards ship first. A first cut: Your year, Your nemesis, Your tales, Your turning points, The close.
4. The look of a card in the addon: the journal frame, or a frame of its own.
5. Does a guild get a Wrapped too, with the guild's shared saga?
