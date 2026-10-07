# Plan: chapters and tales of the Chronicle

Status: built, 2026-10-05, in the order of section 14. Section 16.3 lists what the build changed, and what is not built. The long first draft is in git history (`6e8b857`). A review fixed it after `778226b` (section 15).

## 1. Goal

The Chronicle is one book in time order. It holds two kinds of entries:

- **Chapters** tell the open world: quests, travel, levels, and the PvP outside battlegrounds.
- **Tales** tell one instance each: a dungeon, a raid, or a battleground.

The code decides every chapter and every tale. A model only writes their text.

Four rules hold for any play:

1. A flight, a walk, or a hearthstone makes no entry.
2. A long grind in one zone still breaks into chapters.
3. The same thing done again never makes a new entry.
4. A closed entry never changes when play goes on.

Lean proves the part of each rule that the fold decides (section 8). The walk from events to steps is outside Lean, so property tests cover it.

## 2. Today, and why it fails

Today a chapter starts at a milestone: the first visit of a zone, every tenth level, or a first kill of a rare or a boss. A milestone after less than 45 minutes of play joins the chapter (`GAMEPLAY.md` 3.3).

| Problem | Cause |
|---|---|
| A flight across 4 zones makes thin chapters. | A first visit counts, also from the air. |
| A long grind never breaks. | Quests and deaths are no milestones. |
| A raid night cuts the leveling story in two. | An instance is one more zone. |
| A slow player and a fast player get other chapters for the same deeds. | The cut reads the clock of play. |
| A rule change orphans old sagas. | A saga is keyed by the tick of its chapter. |

The new rule drops `sessions`, `chapter_starts`, and the 45 and 30 minutes.

## 3. Steps, keys, and tracks

The story program walks the history once, oldest first. Each event becomes one **step**. A step has:

- `key`: what the event is about, or none. Example: `(Kill, Hogger)`, `(Level, 20)`.
- `zone`: the zone where you stood.
- `track`: `World`, or `Instance(zone)` when the zone is a dungeon, a raid, or a battleground.
- `mark`: a break, or none (section 5).
- `at`: the time, in seconds.

The walk reads only the events up to the step. It never looks ahead. So the first `LOCATED_IN` of a new dungeon is a `World` step. The game names the instance one event later, with `instance_entered`.

**The track is where you stood, with one exception.** A level up is always on the `World` track, also inside an instance. A level belongs to the leveling story, not to a place.

A dungeon or raid is known from `instance_entered` (built). **A battleground sends nothing today.** It needs `instance_entered` with `"pvp"` (section 9). An arena stays no instance: it makes no tale.

**Ids are small numbers.** The walk gives each key, foe, zone, and instance a dense id: the first new one gets 0, the next 1, and so on. The fold then keeps each record in a `Vec` at the index of its id. It needs no set, no map, and no `Vec::insert`, which Aeneas handles poorly.

## 4. Weight

A step adds its weight **only the first time its key happens**. A repeat adds 0. The table is the same on both tracks. Only the entry that gets the gain differs.

| Event | Key | Weight |
|---|---|---|
| A game quest turned in | `(GameQuest, quest)` | 1 |
| A Timeways side quest finished | `(SideQuest, quest)` | 2 |
| A class quest turned in | `(ClassQuest, quest)` | 3 |
| A new subzone, on foot | `(Subzone, place)` | 1 |
| A level up, except the first level at the first login | `(Level, n)` | 2 |
| A first talk with an NPC | `(Talk, npc)` | 1 |
| A first kill of a rare or a dungeon boss | `(Kill, foe)` | 3, +2 revenge |
| A first kill of a raid boss or a world boss | `(RaidKill, foe)` | 5, +2 revenge |
| A death to a foe you never beat | `(DeathBy, foe)` | 2, then 1, then 0 |
| A death to a foe you beat | `(DeathBy, foe)` | 0 |
| A death with no known killer | `(DeathIn, zone)` | 2, then 1, then 0 |
| A lasting mark, a joke title | `(Mark, …)`, `(Title, …)` | 1 |
| The first mount (built: `first_mount`) | `(Mount, first)` | 3 |
| The first epic mount (built: `first_epic_mount`) | `(EpicMount, first)` | 5 |
| The first epic item (built: `first_epic_item`) | `(EpicItem, first)` | 3 |
| A big upgrade (built: `upgraded`, value = slot, item `quality`) | `(Upgrade, slot, quality)` | 1 |
| A new PvP rank | `(PvpRank, n)` | 2 |
| The first entry into a dungeon | `(Dungeon, zone)` | 3 |
| The first entry into a raid | `(Raid, zone)` | 5 |
| The first entry into a battleground | `(Battleground, zone)` | 3 |
| The first win in a battleground | `(BgWin, zone)` | 3 |
| A flight, a walk, a new zone, a sighting, trust | none | 0 |

**Subzones.** Until the taxi flag exists (section 9), a subzone gets a key only after a step with another key in the same zone, in the same stay. `VISITED` is an up flag. So a subzone first seen from the air never counts, also later on foot.

**Deaths and revenge.** Once you beat a foe, dying to it matters less.

- A foe is **beaten** after any kill of it by you, on either track, in any rule epoch.
- A death to a beaten foe weighs 0.
- The first kill of a foe that killed you before adds 2 for revenge.
- One record per foe serves both tracks. Die to a boss in a dungeon, kill it later in the open world: the revenge goes to the chapter.
- The saga reads the real count from the world: "Defeated Hogger, who had killed them 3 times." The fold counts only to 2, because a third death weighs 0.

A kill counts only for a rare, a rare elite, a world boss, or a boss of an encounter. A normal mob or an elite is never beaten. So deaths to it decay, and it gives no revenge.

**Mounts and gear.** The world holds each of these facts at most once in a life, and `upgraded` once for each slot and quality (`GAMEPLAY.md` 3.2). So the key of each is new at its only event, and a repeat makes no event at all. A first ride on an epic mount makes two events, `(Mount, first)` and `(EpicMount, first)`: two steps, so no step weighs more than 5. A mount or an item is never a break. A mount is on the `World` track, also inside an instance, as a level is: it belongs to the leveling story. An item follows the track of where you stood.

**Caps.** `W_MAX`, the largest weight of one step, is 7. The rows of mounts and gear weigh at most 5, so `W_MAX` and `CAP_MAX` stay 7, and theorems 8 and 13 keep their constants. The **cap** of a key is the most it can add in all: its weight, 3 for a death key, and the weight + 2 for a kill. `CAP_MAX`, the largest cap, is 7. The fold keeps the gain of each key so far, so the cap is an invariant of the state.

## 5. Chapters

### 5.1 Breaks

A break is a natural pause.

| Break | Source | Today? |
|---|---|---|
| Settling in a new zone | The first `World` step with weight in a zone | Yes |
| A return to a zone | A `World` step with weight in a zone that had none in the last 2 chapters | Yes |
| Every tenth level | The `LEVEL` update | Yes |
| Leaving a capital | The `LOCATED_IN` that leaves it | Yes |
| 8 hours away | The gap between two events | Yes |
| Leaving an inn | A new line `rest_changed` | No (section 9) |

Leaving an instance is **no** chapter break. It ends a visit (section 6).

### 5.2 The rule

1. A break waits until the next `World` step with weight. A break on an instance step waits too.
2. At that step, the chapter closes if its weight is **MIN** (15) or more. Else the break is spent.
3. A chapter also closes once its weight reaches **MAX** (40), with no break.
4. Only `World` steps add weight to a chapter. An instance step adds nothing and closes nothing.
5. A rule change (section 12) closes the open chapter, even below MIN.

A chapter is a range of steps: `first` to `end`. The ranges cover the whole log, also the instance steps between world steps. Its key is the `EventId` of its first event, not a tick: one line can make several events at one tick.

**A raid night** leaves the open chapter open. It adds no weight, closes nothing, and makes no break. After the raid, play goes on in the same chapter. Only a pending break from before the raid, a tenth level, or 8 hours away can close it, at the next `World` step with weight.

**Titles.** The title is the zone where the chapter opened, or "Return to Tirisfal Glades". After a close at MAX, it is "Tirisfal Glades, continued". When two breaks wait at one cut, the name takes the first of: return, new zone, level, capital, inn, away.

## 6. Tales

A tale is one entry for one instance. Each instance has **at most one tale** in the whole life of the character. A tale is a list of **visits**.

- **A visit opens** at the first step on the `Instance` track of its zone.
- **A visit closes** at the first step that comes `RUN_GAP` (30 minutes) or more after you left, or at the first step in another instance, or at 8 hours away. A step back in the same instance before that resumes the visit. So a wipe and a corpse run stay one run.
- **The tale opens** with its first visit. The first entry always has weight.
- **Its weight** is the sum of the gains of its visits. It only grows.
- **Its count of runs** is its number of closed visits. "Molten Core, 7 runs."
- **Its text** is written when a visit with gain closes. A visit with no gain changes only the count, and costs no model call.

| Instance | What gives a tale weight |
|---|---|
| Dungeon | The first entry (3), first boss kills, first deaths, quests done inside |
| Raid | The first entry (5), first raid boss kills, first deaths |
| Battleground | The first entry (3), the first win (3), a new PvP rank earned inside |

A battleground win and a PvP rank need new events (section 9). A PvP rank earned outside a battleground is `World` weight.

**What is stable.** A closed visit never changes: its range of steps, its gain, and its tale. A tale itself grows: new visits add runs and weight. Its text changes only after a closed visit with gain. Each text row names the visit that it covers. So a text never loses its source.

**The text.** One model call, no second draft, no judge. Its prompt holds the facts of the tale, its text before, and the new visit. The check is the saga check: one paragraph, at most 600 characters, no slop, never "our hero". A refused answer or a failed call keeps the text before. A tale with no text shows its plain list. The calls wait behind the sagas, before the summary.

**In the book.** A tale follows the chapter whose range holds its first step. That place never moves. The contents list marks a tale as a dungeon, a raid, or a battleground, and can filter by kind. A tale gets one page: its text, its bosses, its tally lines, and its count of runs. Each list keeps at most 20 entries, as a chapter does (3.3). The atlas page of the instance shows its tale in place of a zone history.

## 7. No bloat

**Entries grow with what is new, never with time played.**

- A repeat has no weight. So it never meets a break, never reaches MAX, opens no tale, and calls no model.
- A repeat still shows. Its chapter or its tale gets one tally line: "Defeated Ragnaros again, 6 times." The list `again` keeps at most 20 lines.
- A long endgame of repeats keeps one chapter open. That is correct.

**The bound.** Every closed chapter weighs at least MIN, except one per rule change. So:

> closed chapters × MIN ≤ world gain + rule changes × MIN
>
> world gain + instance gain ≤ distinct keys × CAP_MAX
>
> tales ≤ distinct instances
>
> tale texts ≤ closed visits with gain ≤ instance gain

## 8. The proofs

The fold lives in `crates/rules/src/chapters.rs`, in loop style: index loops, `push`, and writes to `v[i]`. Aeneas extracts it to Lean, and `lean/Timeways/Chapters.lean` proves these laws.

- `advance(state, steps)` folds steps onto a state. `chapters(steps)` is `advance(start(), steps)`.
- The fold also gives the **gain of each step**, as a `Vec<u16>`. The journal needs it to leave repeats out, and the proofs get simpler.
- `cutsOf` and `advanceOf` are the Lean functions of the fold. A reachable state is `runM ss`: the fold of one log from `start`. A law that holds from any reachable state lets the story program fold one line at a time.
- A rule change is a step: `Rule(n)`. The walk puts it at the `from` of each row of `chapter_rules`. The constants of each rule are a `match` on `n`. An old rule is never edited and never deleted.
- A key is **spent** when it can gain nothing more: a seen key of any kind but a death, or a death key whose foe is beaten or whose deaths reached 2.

| # | Theorem | In plain words |
|---|---|---|
| 1 | `every_step_is_in_one_chapter` | The chapter ranges cover the steps in order, with no gap, no overlap, and none empty. |
| 2 | `every_instance_step_is_in_one_visit` | Each `Instance` step is in exactly one visit, and that visit is in the tale of its zone. |
| 3 | `a_closed_chapter_never_changes` | The closed chapters of a log are a prefix of the closed chapters of the log with more steps. |
| 4 | `a_closed_visit_never_changes` | The closed visits of a log are a prefix of the closed visits of the log with more steps. |
| 5 | `a_tale_changes_only_with_a_visit` | A tale's weight and runs only grow, and its weight grows only by the gain of a closed visit. |
| 6 | `a_closed_chapter_has_min_weight` | Every closed chapter weighs MIN or more, or a rule step closed it. |
| 7 | `a_rule_change_closes_at_most_one_chapter` | A `Rule` step closes at most one chapter, and no other step closes one below MIN. |
| 8 | `no_chapter_passes_max_and_one_step` | No chapter weighs more than MAX − 1 + W_MAX of its rule. |
| 9 | `the_weight_of_a_chapter_is_the_sum_of_its_gains` | A chapter's weight is the sum of the gains of its `World` steps. |
| 10 | `a_step_with_no_gain_closes_nothing` | A step with gain 0 closes no chapter, unless it is a `Rule` step. |
| 11 | `a_repeat_never_adds_weight` | A step with a spent key gains 0, and the key stays spent. |
| 12 | `repeats_alone_never_make_an_entry` | Steps with spent keys or no key close no chapter, add no gain to a visit, and add a tale only for an instance that has none (16.2). |
| 13 | `entries_grow_only_with_what_is_new` | The bounds of section 7. |
| 14 | `an_instance_step_never_adds_world_weight` | An `Instance` step adds 0 to a chapter. |
| 15 | `an_instance_has_at_most_one_tale` | No two tales share an instance. |
| 16 | `a_death_to_a_beaten_foe_weighs_nothing` | Once a foe is beaten, every later death to it gains 0, on both tracks and under every rule. |
| 17 | `deaths_to_one_foe_weigh_at_most_three` | The deaths to one foe gain at most the sum of the death weights of the rule. 100 deaths to one mob never make a chapter. |
| 17b | `deaths_with_no_killer_weigh_at_most_three` | The deaths with no known killer of one key, so of one zone, gain 3 at most in all. One such death weighs 2, then 1, then 0 (`a_death_with_no_killer_weighs_two_then_one_then_nothing`). |
| 18 | `revenge_counts_once` | Only the first kill of a foe that killed you adds revenge. |
| 18b | `revenge_needs_an_earlier_death` | A kill adds revenge only when an earlier step of the log is a death to its foe, and then it gains its weight plus 2. |
| 19 | `advance.spec`, `chapters.spec` | The fold never panics, never overflows, and always ends. |

Notes:

- Theorem 16 alone says it all. "Never more than a death to an unbeaten foe" follows, because nothing gains less than 0.
- 10, 11, and 12 hold from any fold (16.2): a close needs a gain or a rule step, so no pending close can fire on a step with no gain.
- The weights of a chapter are `u16`, and theorem 8 bounds them. The weight of a tale is a `u32` with `saturating_add`: a damaged log can hold many quests inside one instance.
- Determinism needs no theorem. `cutsOf` is a function, so the same log always gives the same entries.
- The walk reads Hourglass, so Lean does not see it. Its laws are property tests: "the steps of a prefix are a prefix of the steps", and "the ids are dense, in order of first use".

## 9. New events

Checked against the API of the WoW Forever client 1.60.1 (`target/wow-api`, the source of the API gate of Gnomish Relay).

| Event | Why | The API | Available? |
|---|---|---|---|
| `instance_entered` with `"pvp"` | Battleground tales | `IsInInstance()` gives `"pvp"`. Add `"pvp"` to `INSTANCE_KINDS`, a new kind `Battleground`, and a new fact `battleground`. | Yes |
| `bg_won` | A battleground win | `UPDATE_BATTLEFIELD_STATUS`, then `GetBattlefieldWinner()` (0 Horde, 1 Alliance, nil while the match runs) against `UnitFactionGroup("player")`. A fallback: `PVP_MATCH_COMPLETE` and `C_PvP.GetActiveMatchWinner()`. | Yes, with a test in the game |
| `pvp_rank` | A new PvP rank | **`UnitPVPRank` and `GetPVPRankInfo` do not exist in this client.** The rank is the renown of faction 2800, as the client's own `PVPRankFrame.lua` reads it: `MAJOR_FACTION_RENOWN_LEVEL_CHANGED`, then `C_MajorFactions.GetMajorFactionProgressionInfo(2800).renownLevel`. | Yes, but the faction id needs a test in the game |
| `rest_changed` | The inn break | `PLAYER_UPDATE_RESTING` and `IsResting()`, and a new fact `resting` | Yes |
| `taxi` on `zone_entered` | A subzone seen from the air never counts | `UnitOnTaxi("player")`. `PLAYER_CONTROL_LOST` and `PLAYER_CONTROL_GAINED` show the start and the end. | Yes |
| `kind` on `npc_defeated` | A world boss weighs as a raid boss | `UnitClassification` gives `"worldboss"`. The addon knows it already (`Foes.lua`), but the line drops it. | Yes |

**Not detectable:** the killer of a death in PvP. The addon never names a player (5.11), so a PvP death is `(DeathIn, zone)`. The nemesis of 4.1 waits for a source of the killer. When it comes, a nemesis kill gets its own key and joins the `World` track.

**Open questions for the game:** does a login at an inn fire `PLAYER_UPDATE_RESTING`? Does a rank drop at the end of a season? A rank key counts once in a life, so a new season adds nothing until a new top rank.

Each new line gets a fuzz seed for the `input` target.

## 10. Zone history

Each open-world zone page in the atlas gets "Your history here": one narrated paragraph about what you did there, over all visits. An instance shows its tale instead.

- After a chapter closes, its zone with the most new `World` weight (5 or more) gets a rewrite. One model call.
- The call waits behind sagas, tales, and the summary. A chapter costs at most 5 calls.
- The check is the saga check: at most 400 characters, no slop, never "our hero". It must not copy 8 words in a row from a saga. A refused answer gets one retry, then the old text stays.
- The page also lists every chapter that had weight in the zone.

## 11. Player edits

It is the player's story. A player can change what a chapter, a tale, or the summary **says**, never which events it holds.

**What a player can do:**

- **Edit** the text of a chapter or a tale: add paragraphs after the narrator's text, or rewrite it.
- **Rename** a chapter or a tale.
- **Edit the summary** on the title page. It is one paragraph, because it can go out as the History of the Roleplay Profile.
- **Restore** the narrator's text and title.

**What stays fixed:** the range of events, the weight, the count of runs, and the "In this chapter" list. The game saw them. The proofs of section 8 and the atlas depend on them.

**The row.** An edit is a new row of `entry_edits`, never a change of an old row: `{entry, title, text, at}`.

- `entry` is `Chapter(EventId)`, `Tale(EventId)`, or `Summary`, with the first `EventId` of the chapter or the tale. The kind is part of the key, because a chapter and a tale can start at one event.
- `title` is the player's title, or none for the title of the code.
- `text` is one of three:
  - `Keep(paragraphs)`: the narrator's text stays, and the player's paragraphs follow it.
  - `Replace(paragraphs)`: the player's paragraphs stand in place of the narrator's text.
  - `Narrator`: the narrator's text alone.
- **Restore** is a row with no title and `Narrator`.
- The newest row of an entry, by row id, stands. A row equal to the standing one is not written.

**The editor.** One Edit button opens one box with the title and the shown text. On Save, the addon sends `Keep` when the box starts with the narrator's paragraphs unchanged, with only the paragraphs after them. Else it sends `Replace`. An entry with no narrator text yet, such as the open chapter or a chapter whose saga still waits, always gives `Keep`. So the saga that comes later shows above the player's paragraphs.

**What the narrator text is.** The newest model text of the entry: the saga of a chapter, the newest text of a tale, or the newest summary. With none, the page shows the plain list.

**Limits.** The rules of a story (`hero-stories.md` 3.1): a title of 60 letters and 72 bytes; 1 to 20 paragraphs of the player, with no control character and no `|`, and at most 1000 letters and 1200 bytes together. The narrator's text does not count. A summary has no title and takes only `Replace` with 1 paragraph: the limit of Background, so it can stand as the History.

**The line.** `entry_edited`, with no reply, as a hero edit:

```json
{"type":"entry_edited","at":1790000000,"entry":{"kind":"chapter","first":4211},"title":"The Long Night","text":"keep","paragraphs":["..."]}
```

- `text` is `"keep"`, `"replace"`, or `"narrator"`. With `"narrator"`, `paragraphs` is empty.
- An `EventId` stays below 2^53, so Lua reads it exactly.
- The addon marks player names as in a story (5.11): `{Name}`, and `$N` for your own name. The story program swaps them to IDs before it writes the row (`aliases::with_ids`). So the row holds no known name.
- The longest line without names is about 2650 bytes, under the strip room of about 2730. Marks add 2 bytes for each name. So `Outbox.Fits` gates Save. A text that does not fit stays in the box with "Too long to save. Shorten it a little."
- The line goes out with a journal request. The page shows "Saving..." until the journal comes. A refused edit leaves its reason for the next journal, as a hero edit does.
- The edit is no event. It writes its input and its row, and nothing in the history. So the walk never sees it.

**Proof.** An edit rests on its input: the root is `Player`. A call that reads an edit rests on `Player` too.

**What the narrator does with an edit:**

- **Who reads it.** A saga reads the standing edit of its own chapter. A tale text reads the edit of its tale. The summary reads the edits of the chapters in its prompt. The zone history reads no edit. Each call adds the edit rows to its reads (5.14).
- **How.** The prompt holds the player's paragraphs and title under "The player's telling, not canon", fenced with `house::fenced` as all outside text, and cut to the first 600 characters. Its rule: tell the deeds of the facts, do not repeat the telling, and do not contradict it.
- **The facts stay the facts.** The deeds come from the chapter. An edit adds no fact, and the narrator's own text before is the text of the model, never the edit.
- **Checks.** A model text that holds 8 words in a row of an edit in its prompt is refused, as for a saga. An edit gives no allowed word to the check of the cutoff (3.7). So a later name in an edit never reaches a model text.
- An edit makes no model call. The narrator keeps writing under `Replace`, so Restore shows its newest text. That changes no bound of section 7.

**In the journal:** each edit is its own item, as a story is. `journal::pages` puts it on any page with room. So a full chapter and a full edit never have to share one page. The chapter page shows Edit, and Restore when edited. Restore asks first: "Restore the narrator's version? Yours is removed." The contents list shows the player's title and a small "Edited".

**Sharing:** an edited summary goes out as the History of the Roleplay Profile as the player wrote it, with the real names of the alias table. The player saw each name before Save. A summary that the player did not edit shows each other player as their card (5.11, rule 8).

**Rule epochs.** A rule step closes the open chapter and keeps its first `EventId` (section 13), and a `from` is never before the newest event. So the key of an edit always names a chapter or a tale. An edit whose key names none stays in the table and shows nowhere.

**Growth.** The addon keeps no edit: the outbox is in memory, and the journal brings the edits back. A row is at most about 1.4 KB, and a no-op edit writes none. So 10,000 saves are about 14 MB. Old rows stay, as hero rows do.

**Proofs** (section 8 gets these). `shown(narrator_rows, edit_rows)` is a small pure function in `crates/rules`. It works over row ids and the three kinds of `text`, never over strings, so Aeneas extracts it.

| # | Theorem | In plain words |
|---|---|---|
| 20 | `the_newest_edit_decides` | `shown` of the whole lists equals `shown` of only the newest row of each. After a restore, the shown text is the newest narrator text, also one that came after the edit. |
| 21 | `a_model_text_never_hides_player_words` | A new narrator row never changes the shown title, never removes the shown player paragraphs, and changes nothing under `Replace`. |

**Edits never move an entry.** This is no Lean theorem: the fold has no input for an edit, and an edit is no event. The property test `edits_never_move_an_entry` runs the same plays with and without random edits, and compares every range, gain, visit, and count of runs.

## 12. Tests

- **Unit tests** in `crates/rules` and `crates/story`, named as sentences. Examples: `a_flight_across_four_zones_makes_no_chapter`, `a_raid_night_does_not_cut_the_open_chapter`, `a_second_dungeon_run_with_nothing_new_changes_only_its_count`, `a_wipe_and_a_corpse_run_stay_one_run`, `a_level_inside_a_dungeon_counts_for_the_chapter`, `revenge_outside_an_instance_counts_for_the_chapter`.
- **Property tests** in `crates/story/tests/properties.rs`. Make the edges likely: chapter weights at MIN−1, MIN, MAX, and MAX+1, gaps at `RUN_GAP` − 1 and `RUN_GAP`, runs of 10,000 zero-weight steps, and logs made only of breaks. They also cover:
  - 1000 weekly raid clears give the same chapters, the same visits with gain, and the same tale weights as 1;
  - 100 deaths to one mob weigh at most 3;
  - folding one line at a time gives the same entries as folding the whole log;
  - the walk gives dense ids, and the steps of a prefix are a prefix of the steps.
- **Edits:** `an_edit_keeps_the_chapter_list`, `restore_shows_the_newest_narrator_text`, `a_saga_after_a_replace_does_not_show`, `a_saga_after_a_note_on_the_open_chapter_shows_above_it`, `a_tale_rewrite_keeps_the_players_paragraphs`, `an_edit_that_does_not_fit_the_strip_stays_in_the_box`, `a_model_text_that_copies_an_edit_is_refused`, `a_later_name_in_an_edit_is_not_allowed_in_a_saga`, and `the_largest_chapter_and_its_largest_edit_fit_the_journal`.
- **Edit properties:** `edits_never_move_an_entry` (section 11). `no_prompt_holds_a_known_player_name` gets a new play: an edit of marked words, as `StoryAccept` has. Without it, the test never sees an edit.
- **Fuzz:** seeds for each new line, the `entry_edits` rows, the `entry_edited` line, the `tales` and `zone_histories` rows, and the model answers of a tale and a zone history.

## 13. Migration

A new rule, or new values of MIN and MAX, re-cut the log. That orphans old sagas.

**Rule epochs fix it.** A row table `chapter_rules` holds `{rule, from: EventId}` for each change. The walk puts a `Rule` step at each `from`. The fold keeps its whole state across the step: seen keys, foe records, settled zones, tales, and visits. Only the open chapter closes, even below MIN. Theorems 6 and 7 name this exception. A `from` is the next event after the change, never an older one, so the key of an edit or a saga never loses its entry. Old chapters and their sagas never move.

Nothing is live, so no world needs a migration before the first release (5.7). After it, each change needs one:

| Schema change | What a migration needs |
|---|---|
| Saga rows keyed by first `EventId`, with `rule`, `first`, `last` | Map each old tick to its first event under rule 1, then add the epoch rows. |
| `summaries.after` becomes an `EventId` | The same map. |
| New tables `chapter_rules`, `tales`, `zone_histories` | Create them. `chapter_rules` gets `{rule: 1, from: 0}`. A `tales` row is `{instance, visit: EventId, text}`. |
| Old instance visits get tales | The fold finds them. They show the plain list. Only a visit that closes after the upgrade calls a model, so no backlog of calls. |
| New table `entry_edits` | Create it, empty. |
| New facts `resting`, `battleground` | A vocabulary version. No old event changes. |
| New fields `taxi`, `kind` on old lines | None. Both are optional. |

## 14. Build order

1. The fold, the weight table, tracks, visits, tales, and rule steps in `crates/rules`, with unit tests.
2. The Lean proofs of section 8.
3. The walk in the story program, dense ids, chapter keys as `EventId`, and the fold state kept in memory. Measure the open of a world of 50,000 events.
4. Rule epochs.
5. Titles, tally lines, and tales in the journal and the addon.
6. Property tests and fuzz seeds.
7. Tale texts and the zone history.
8. The new events: battleground entry, win, and rank first, then rest, taxi, and the kind of a kill.
9. Player edits: `entry_edits`, `shown` and its proofs, the `entry_edited` line, the prompts and their checks, and Edit and Restore in the addon.

## 15. Open questions

Proposed answers wait for the user.

| Question | Proposed |
|---|---|
| Start with MIN 15, MAX 40, deaths 2 then 1, revenge 2, 8 hours away, `RUN_GAP` 30 minutes? | Yes, and tune in play tests. |
| Is one open chapter for weeks of endgame repeats right? | Yes. |
| Does a first talk with an NPC weigh 1? | Yes (in the table). |
| Does a return need MIN in the chapter before? | Yes, or quick visits bloat the book. |
| Build a list of each dungeon's last boss for "cleared"? | Later. Boss kills stand in for it. |
| Add the taxi flag? | Yes. |
| Does a battleground tale count wins after the first? | No. Only the first win and new ranks. |
| Does a level up count for the chapter, also inside an instance? | Yes. |
| One call for a tale text, with no second draft? | Yes. A tale can change many times. |
| Does an edited summary go to the shared History? | Yes (user, 2026-10-05). |
| Can a player edit the zone history too? | Later, with the same rows and `entry` `Zone(zone)`. |
| Undo after Restore? | Not now. The old rows stay, so an Undo can come later. |
| Cap the edit rows of one entry? | No. A row is small, and hero rows have no cap either. |
| Does an edited summary share other players by their real names? | Yes: the player saw each name before Save (section 11). |

## 16. Review notes

- **Theorem 2 was false for tales.** A tale grows, so it is never closed. Now a closed *visit* is stable, a tale only grows, and its text names its visit (theorems 4, 5).
- **A raid night still cut the chapter.** "Leaving an instance" was a break. It is not one now. Section 5.2 says what a raid night does.
- **Theorem 1 mixed ranges and tracks.** Chapter ranges now cover all steps. Each instance step is in one visit (theorems 1, 2).
- **A wipe made a new run.** A corpse run leaves the instance. `RUN_GAP` keeps it one run.
- **`UnitPVPRank` does not exist in WoW Forever.** The rank is renown of faction 2800. A rank is earned anywhere, so it goes on the track where you stand.
- **Rule epochs broke theorems 6 and 16.** The epoch carried only seen keys. Now a rule change is a step of the one fold, and the whole state carries over.
- **Theorem 12 had a vacuous half.** It now says "every later death to a beaten foe gains 0, on both tracks".
- **Provability.** Dense ids and `Vec` by index replace sets and maps. The fold returns the gain of each step. A tale weight is a saturating `u32`.
- **The bound** now counts the short chapters of rule changes, and bounds tale texts by instance gain.
- **Gaps filled:** tale text and its calls, its page size, migration of old visits, world bosses, levels inside instances, revenge across tracks, PvP deaths, and arenas.
- **Restored from the long draft:** the subzone stay rule, keys as `EventId`, the title order, the first-level rule, `Reachable`, and the cap of 20 tally lines.
- **The saga count of deaths** comes from the world. The fold counts only to 2.

### 16.1 Review of player edits

- **"Add paragraphs" froze the narrator.** A note on the open chapter was the newest row, so the saga that came at its close never showed, and each tale rewrite hid too. Now `Keep` puts the player's paragraphs after the newest narrator text, and only `Replace` stands in its place.
- **Theorem 20 was no Lean theorem.** "The fold never reads edits" is a fact of its input type. It is now a property test, and the edit is no event, so the walk cannot see it.
- **`shown_text` over strings** does not extract well with Aeneas. `shown` now works over row ids and the kind of `text`. Theorem 21 says which narrator text shows after a restore: the newest, also one that came after the edit.
- **The key of an entry** was a bare `EventId`. A chapter and a tale can start at one event, and the summary has none. `entry` is now tagged.
- **The summary edit was not in the list,** but the sharing rule needed it. It is now one paragraph with `Replace` only, to fit History.
- **The limits** left out 1000 letters and 72 title bytes. They now name the story rules.
- **Names.** The addon marks names as in a story, and the story program stores IDs. The property test of known names gets an edit play.
- **Prompts.** The edit goes through `house::fenced` and a cut of 600 characters. A model text that copies 8 words of an edit is refused. An edit gives no allowed word to the cutoff check.
- **The wire.** The line has a shape, fits the strip without names, and `Outbox.Fits` gates Save. Each edit is its own journal item, so the largest chapter still fits a page.
- **Gaps closed:** the reads of each call, the proof root, rule epochs, Restore asks first, no-op rows, and growth. The row `after` is gone: nothing read it.
- **The proofs of 20 and 21** (`lean/Timeways/EntryEdits.lean`). Theorem 20 is three theorems: `the_newest_edit_decides`, `a_restore_shows_the_newest_narrator_text`, and `a_restore_shows_a_later_narrator_text`. Theorem 21 holds for any new narrator row, not only a newer one, because the title and the player row never read the narrator rows. Of two edits with the same row id, the later one in the list stands (`pickEdit_tie`).

### 16.2 Review of the proofs

Theorems 1 to 19 are proved in `lean/Timeways/Chapters.lean`, about a pure model (`ChaptersModel.lean`) that `ChaptersBridge.lean` proves equal to the Rust fold. Each axiom is pinned in `Axioms.lean`. These statements changed, and each keeps its intent:

- **Theorem 12 was false as written.** "Open no tale" fails for a step with no key into an instance that has no tale: the fold opens its tale, so that theorem 2 holds for every step. Only a damaged walk makes such a step, because the first step in an instance is `instance_entered` with its new key. Now: the steps close no chapter, gain 0, add no gain to a visit, and add a tale only for an instance that has none (the instances of the tales grow as a prefix, with no instance twice). It holds from any fold, not only a reachable one.
- **"Add no gain to a visit"** is stated as: every new gain is 0, and the sum of the gains of the visits does not grow.
- **Theorem 13 has six parts.** The count of closed chapters that no rule step closed, times `MIN`, is at most the gain in the open world. The chapters that rule steps closed are no more than the rule steps. The gain of all steps is at most 7 times the keys, and the keys are no more than the distinct key ids of the steps. The tales are no more than the distinct instances. The closed visits with gain are no more than the gain in instances. The tale texts are rows of the story program, so the proof bounds the closed visits with gain, which bound the texts. `MIN` is the `MIN` of rule 1, the only rule so far.
- **Theorem 17 is per foe.** The deaths count on the foe record, not on the key, so two keys of one foe from a damaged walk still weigh 3 at most. A death to a foe out of range weighs 0.
- **Theorem 18** says that the steps of a log add revenge for one foe once at most.
- **Theorems 17b and 18b** are in `lean/Timeways/ChaptersDeaths.lean`. Theorem 18b reads the death from the foe record of the fold, and `a_death_in_the_fold_is_a_step_of_the_log` ties that record to a death step of the log. The +2 holds in every fold of a log, because a key that the fold has not seen has no gain yet.
- **The decay of deaths with no known killer is per step, not per count of the log.** A step whose key id is out of range gains 0 and counts nothing. So in a damaged walk the second such death of a key can weigh 2. The law of one step holds from any fold: at 0 deaths of the key the death weighs 2, at 1 it weighs 1, and after that 0, when the key has room under its cap. The walk gives each zone one key, so the record of the key holds only these deaths, and the room always holds. The bound of 3 holds for any log.
- **Theorems 5, 7, 10, 11, 14, and 16** hold from any fold, so they need no `Reachable`. Theorem 5 needs room for one more closed visit, as every fold of a log has.
- **Theorem 19** needs room: no vector of the fold that a step grows is longer than its steps, and the steps fit a `usize`. A fold of one log from the start always has it.
- **Theorem 2** names the visit by its tale index: the tale at that index has the instance of the step.

### 16.3 Review of the build

What the build changed from the text above, and why:

- **A zone gets its record at its first step** (`note_zone`), with weight or not. The walk gives dense zone ids at every step, so a zone that only gave a flight still takes its id. Without this, the next zone settled under a wrong id and made no break. A unit test and the property tests check it.
- **A move is a step of the place where you go.** The game writes the events of a move (the new place, the visit, the move itself) before you stand there. So the walk puts each such event in its new place. Else a step out of an instance, 30 minutes later, opened a run of the instance that you left.
- **The founding of the character is in no chapter.** It is no play.
- **A chapter names the zones where it gained weight**, not the zones of its first visits. So a flight over a zone names no place of the chapter, and the chapter after it names its own zone.
- **A tale and a zone history use the samples of a chapter.** The user approved those. Samples of their own wait for the user.
- **A rank and a level at the first login weigh nothing.** The addon sends rank 0 at login, so the first rank that you earn counts.
- **A flight marks no place as visited.** With `"taxi": "yes"`, the story program moves you, and the visit waits until you come on foot.
- **An edit can change the title alone:** a narrator text with a title.
- **A bug found in the addon:** an empty Lua table goes out as a JSON object, so a restore with `"paragraphs": {}` broke the line. A restore now sends no paragraphs.

Not built:

- **The filter of the contents by kind** (section 6). The contents mark each tale with its kind.
- **The Knowledge atlas.** The journal carries "Your history here" of each zone (`histories`), and nothing shows it yet. The atlas page of an instance shows its tale there, once the atlas exists.
- **Narrator lines** for a battleground won and a new rank.
