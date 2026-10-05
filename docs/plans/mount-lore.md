# Plan: mount lore in the lore pack

Status: draft 1, 2026-10-05. A spec for review. Nothing is built, and `pack_sources.toml` does not change until the user approves the list.

## 1. Problem

The narrator has a first mount and a first epic mount (GAMEPLAY.md 3.2, `mounts.rs`). The subject of the moment is the place of the people, such as "Stormwind City". The pack has no page about mounts. So the lore is the history of the capital, and the line glues an unrelated history to the deed: "King Barathen Wrynn scattered the gnoll packs… $N now rode a chestnut mare."

The fix: pages about the mount families, and a subject for the family before the subject of the people.

## 2. Candidate pages

Checked in the Wowpedia dump of 2026-10-05. "Paras" is the count of lines of 80 or more characters in the kept sections, before the later terms and the game terms drop some. "RPG" is the section "In the RPG". No page of the pack keeps it today (question 2).

| Family | Page | In dump | Keep | Cutoff risks | Paras |
|---|---|---|---|---|---|
| Ram | Ram | yes | Background (RPG) | lead names Northrend; Brewfest line | 3 (+2) |
| | Amberstill Ranch | yes | lead | Legion section | 1, game talk |
| | Veron Amberstill | yes | Quotes | none | 1 ("Barak Tor'ol mountain ram") |
| | Ironforge Mountaineers | **no** | | "Ironforge Mountaineer" is a guard page with no lore | 0 |
| Saber | Nightsaber | yes | lead (RPG) | Hearthstone gallery only | 4 (+1) |
| | Frostsaber | yes | lead (RPG) | none | 3 (+2) |
| | Saber cat | yes | lead (RPG) | none | 1 (+1) |
| | Rivern Frostwind | yes | Quotes | lead drops on "Trainers" | 2 |
| | Frostsaber Rock, Wintersaber Trainers | yes | lead | both drop on "Trainers" / "players" | 0 |
| | Lelanai, Jartsam | yes | Quotes | leads tell of the burning of Teldrassil | 1 |
| | Winterspring Frostsaber | stub | | | 0 |
| Skeletal horse | Skeletal horse | yes | lead, History | Naxxramas line | 4 |
| | Zachariah Post | yes | Quotes | lead: "Battle for Lordaeron", "Gates of Orgrimmar" (no later term catches them) | 1 |
| Wolf | Wolf | yes | "Wolves and the Horde" | Goldrinn/Lo'Gosh (later lore, no term); blood elves and Hellscream lines drop | 3 |
| | Dire wolf | yes | lead | Outland line drops | 1 |
| | Frostwolf clan | yes | lead, History, Revitalization of the Horde, Culture | Alterac Valley section is patch 1.5; Cataclysm, Dragonflight sections | 8 |
| | Ogunaro Wolfrunner | yes | Quotes | none | 1, thin |
| Kodo | Kodo | yes | lead, Background, Kodos of the Horde | Black War Kodo line (patch 2.0.1); Pandaria line drops | 5 |
| | Harb Clawhoof | yes | Quotes | none | 2 ("Golden Plains of Mulgore") |
| Raptor | Raptor | yes | Relation with trollkind | Gonk (later lore, no term); Zandalari line drops; skip "The Gurubashi" (Zul'Gurub, patch 1.7) | 3 |
| | Zjolnir | yes | Quotes | none | 1 |
| Mechanostrider | Mechanostrider | yes | lead, Gnomes, History | none in these sections | 3 |
| | Milli Featherwhistle | yes | Quotes | Binjy's quote is Operation: Gnomeregan, skip Binjy | 1 |
| Horse | Horse | yes | lead | first line names Gilneas and worgen, so it drops; deathchargers | 2 |
| | Eastvale Logging Camp | yes | (already a place) | not about horses | 0 |
| | Katie Hunter | yes | Quotes | none | 1 ("a companion for the ages") |

**PvP and class epics.** Each one comes after the launch, so each stays out until Forever opens its patch (question 1):

| Mount | Page | Patch | Paras |
|---|---|---|---|
| Frostwolf Howler | Horn of the Frostwolf Howler | 1.5 (Alterac Valley) | 1 |
| Stormpike Battle Charger | Stormpike Battle Charger | 1.5 | 1 (Mount Journal) |
| Black War mounts | Black War Steed Bridle, Black War Ram, Black War Kodo, … | 1.4 (honor ranks) | 0 or 1 each; journal lines under 80 characters |
| Paladin charger | Summon Charger, Lord Grayson Shadowbreaker, Judgment and Redemption | 1.4 (needs Dire Maul, 1.3) | 1 + 5 |
| Warlock dreadsteed | Dreadsteed, Lord Hel'nurath, Xoroth, Mor'zul Bloodbringer | 1.4 (needs Dire Maul) | 3 |

The launch class mounts are safe now: **Warhorse** ("Description", 1 para) and **Felsteed** ("Origin", 2 paras: the wild horses of Desolace that the Shadow Council bound).

**Good:** sabers, kodos, mechanostriders, skeletal horses, wolves (with Frostwolf clan). **Thin:** rams (one section), raptors (one section), horses (two lines after the filters). **Missing:** Ironforge Mountaineers, Winterspring Frostsaber, any page for the Stormwind horse breeders. The keepers give one gossip line each. Their leads are game talk.

## 3. Link model

**Choice: `about` the mount family, linked as `common`. No new link kind.**

- `pack_sources.toml` gets an optional `about` on a page: `about = "ram"`. It overrides the derived subject (5.10). The pack format stays the same, because `about` is a column since format 2.
- The family pages are `common = true`. Everyone in 25 ADP knows that dwarves ride rams. A keeper page links its NPC, so its quote waits until the player meets the keeper.
- `mounts.rs` maps a word to a family and a people: `("ram", "ram", "Ironforge")`. The class words go in too: "warhorse" and "charger" to `warhorse`, "felsteed" and "dreadsteed" to `felsteed`. Today a dwarf paladin's Warhorse falls back to Ironforge and gets ram lore, and an orc's Felsteed gets wolf lore.
- `Moment::subjects()` for a mount gives `[family, people]`. `lore_of_moment` already tries each subject in turn.
- **The pick is deterministic.** The first mount takes the first passage of the family in page order. The first epic mount takes the second, else the first. So the two lines of one life never tell the same passage. No random draw, no hash.

**Rejected: a `mount` link kind.** A link is a spoiler gate (5.10). A mount family is no spoiler, so the gate buys nothing. It costs a format change, a new world fact, and new reader code.

**Rejected: link to the capital.** The own page of the capital wins over any linked passage (3.2). That is the bug of section 1.

## 4. Class mounts: the quest-chain link

The charger and the dreadsteed are long chains. The page of Lord Grayson Shadowbreaker tells the whole chain step by step, so it spoils it.

- The chain pages take `about` = the title of the last quest: "Judgment and Redemption", "Dreadsteed of Xoroth". `Moment::ClassQuestDone { title }` has that title as its subject, so its own page wins. The mount moment gets the same passages through the family `charger` or `dreadsteed`.
- The gate: a new `quests = [...]` list on a page, read as a link that passes only after `GAME_QUEST_DONE` for that quest. This is the one case where a new link kind pays: the text is a spoiler until the chain ends.
- Until then, link the chain pages to their quest giver NPC. That gate is weaker: it opens at the first talk.
- All of this waits for patch 1.4 content in Forever.

## 5. Sample lines

- Ram, first mount, naming by race: "The Ironforge ram was the first breed that the dwarves of Dun Morogh tamed, a beast as fearless as its masters. The dwarf rides one now."
- Skeletal horse, first mount, by name: "The plague took Lordaeron's horses with their riders, and the apothecaries of the Undercity raised them both. One of them carries $N."
- Mechanostrider, first epic mount, no name: "Gelbin Mekkatorque built the first mechanostrider in Sector 17, so that gnomes could keep pace with human chargers. The swiftest of them answers to a new rider."

## 6. Open questions

1. **The patch cutoff.** Alterac Valley (1.5), honor ranks (1.4), and the class chains (1.4) are later patches. Do they wait for Forever to open them, like Dire Maul? Does Forever change any of them in its own timeline?
2. **"In the RPG" sections.** The Warcraft RPG gives the best detail (battle rams, Sentinel nightsabers, the Warhorse). It is old, but not game canon. Keep it for mounts only, or never?
3. **Keeper quotes.** They are game gossip, but they talk of "exalted". Keep them, or let the seen text bring them when the player talks to the keeper?
4. **New later terms.** "Goldrinn", "Lo'Gosh", "Gonk", "Brewfest", "Battle for Lordaeron", "Black War", "Zul'Gurub". Add them for all pages, or drop the lines by hand?
5. **Common or gated.** With `common`, `/lore` explains kodos to a human who never saw one. Is that fine, or link the family to the zone of its keeper?
6. **Thin families.** Rams, raptors, and horses have little text. Add a History of Warcraft passage by hand (`timeways-pack` from lines), or accept the capital as the fallback?
