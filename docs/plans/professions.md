# Plan: professions in Timeways

Status: draft 1, 2026-10-09. A spec for review. Nothing is built. The user approved the idea in principle. `pack_sources.toml` does not change until the user approves the page list (section 4).

## 0. The idea, in short

- **Milestones.** The narrator speaks at skill 75, 150, 225, and 300 of each profession. The line tells the lore of the craft and its people, and the order of the craft grows stronger, as in a tenth level (`level-lines.md`). The line is faction-aware.
- **Famous firsts.** The first rare catch, the first craft of a notable item, a recipe from a dungeon or a world drop, the first epic craft, and a specialization (Gnomish or Goblin Engineering, for example). A first gets a line only with a real lore source (`item-stories.md`). Else it only adds weight.
- **Knowledge.** The trainers you met, the recipes you learned and where, the gathering spots and fishing spots of each zone, and the fish you caught, in the atlas.
- **The Chronicle.** Profession deeds go into their chapter. Only firsts weigh. A repeat weighs 0.
- **Fishy Fishy.** The fishing addon (`/home/eitan/Documents/Code/Personal/fishy-fishy/SPEC.md`) keeps a catch log. Timeways reads it when both addons are installed (section 7). Each addon works alone.

Sources: the WoW Forever client 1.60.1.70205, from `gnomish-relay/target/wow-api/` (the UI source `ui/`, commit `e3ecc27`, and the API dump `bir/`, commit `4149af6`). "Confirmed" in this spec means "found in that dump". "Game test" means that the dump has the name, but only a test in the game shows the behavior.

## 1. Events and APIs

### 1.1 What the client is

Forever runs the modern (mainline) UI code with the game type `camelot`. Its own profession book (`Blizzard_ProfessionsBook/Camelot/Blizzard_ProfessionsBook.lua`) reads the professions with `GetProfessions()` and `GetProfessionInfo(index)`, and `C_TradeSkillUI.GetProfessionInfoBySkillLineID`. Its crafting screen is `Blizzard_Professions` with a few `Camelot` files. So the modern trade-skill API is the API of this client. The Classic API (`GetTradeSkillLine`, `GetCraftInfo`, `CRAFT_SHOW`) does not exist.

### 1.2 The table

| Part | Events | Functions | In the dump | Notes |
|---|---|---|---|---|
| Professions and ranks | `SKILL_LINES_CHANGED`, `PLAYER_ENTERING_WORLD`; `CHAT_MSG_SKILL` only as a trigger | `GetProfessions()` (prof1, prof2, first aid, fishing, cooking, in the order of the Camelot book), `GetProfessionInfo(index)` (name, texture, rank, maxRank, numSpells, spellOffset, skillLine, rankModifier, specializationIndex, specializationOffset, skillLineName), `C_TradeSkillUI.GetProfessionInfoBySkillLineID` (`skillLevel`, `maxSkillLevel`) | Confirmed | Read the rank from the functions, never from the chat text. `CHAT_MSG_SKILL` is `SecretInChatMessagingLockdown`, so its text can be a secret value, and its words change with the client language. The skill line id is the key: 164 Blacksmithing, 165 Leatherworking, 171 Alchemy, 182 Herbalism, 186 Mining, 197 Tailoring, 202 Engineering, 333 Enchanting, 393 Skinning, 129 First Aid, 185 Cooking, 356 Fishing. |
| Rank fallback | | `C_SkillInfo.GetNumSkillLines`, `C_SkillInfo.GetSkillLineInfo(index)` (`skillID`, `rank`, `maxRank`, `skillLineCategoryID`, `isHeader`) | Confirmed, and built (`Past.lua` reads categories 9 and 11) | Use it when `GetProfessions` gives nil for a slot that the skill list has. Game test. |
| Rank title | | `maxRank` of `GetProfessionInfo`: 75 Apprentice, 150 Journeyman, 225 Expert, 300 Artisan | Confirmed | A skill of 150 needs a rank with a top of 150 or more, so "Journeyman" at 150 is true. |
| Trainers | `TRAINER_SHOW`, `TRAINER_CLOSED`, `TRAINER_UPDATE` | `UnitName("npc")`, `UnitGUID("npc")`, `GetNumTrainerServices`, `GetTrainerServiceInfo`, `GetTrainerServiceSkillLine`, `GetTrainerGreetingText` | Confirmed | A trainer of a profession is an NPC whose services name one skill line of section 1.2. A class trainer names none. |
| A recipe learned | `NEW_RECIPE_LEARNED` (recipeID, recipeLevel, baseRecipeID), `LEARNED_SPELL_IN_SKILL_LINE` (spellID, skillLineIndex), `CHAT_MSG_SYSTEM` with `ERR_LEARN_RECIPE_S` as a fallback trigger | `C_TradeSkillUI.GetRecipeInfo(recipeID)` (`name`, `learned`, `sourceType`, `hyperlink`), `C_TradeSkillUI.GetProfessionInfoByRecipeID`, `C_TradeSkillUI.GetRecipeOutputItemData`, `C_TradeSkillUI.GetRecipeItemLink` | Confirmed | Game test: does Forever fire `NEW_RECIPE_LEARNED` for a recipe from a trainer and for a recipe item? Which of the two events fires, and is `recipeID` a spell id? |
| Where a recipe came from | `TRAINER_SHOW` open; `MERCHANT_SHOW` open; `CHAT_MSG_LOOT` and `LOOT_OPENED` for a recipe item; `UNIT_SPELLCAST_SUCCEEDED` of the recipe item | `GetMerchantItemLink`, `C_MerchantFrame.GetItemInfo`, `C_Item.GetItemInfoInstant` (`classID` 9 is `Enum.ItemClass.Recipe`), `C_TradeSkillUI.GetRecipeSourceText` | Confirmed | The source is decided by a fixed rule (section 2.4), never by `GetRecipeSourceText`. That text is localized and shows only for a recipe that you do not know yet. |
| A craft | `TRADE_SKILL_ITEM_CRAFTED_RESULT` (`data`: `itemID`, `hyperlink`, `quantity`, `firstCraftReward`, `isEnchant`), `TRADE_SKILL_CRAFT_BEGIN`, `UPDATE_TRADESKILL_CAST_STOPPED`; `CHAT_MSG_LOOT` with `LOOT_ITEM_CREATED_SELF` as a fallback | `C_Item.GetItemInfo` (quality, item level), `C_TradeSkillUI.IsRecipeFirstCraft` | Confirmed | Forever loads the base `Blizzard_ProfessionsCraftingOutputLog.lua`, which listens to `TRADE_SKILL_ITEM_CRAFTED_RESULT`. Game test: does it fire for a Classic recipe? `firstCraftReward` and `IsRecipeFirstCraft` are Dragonflight features. Timeways keeps its own "first" in the world, so it does not need them. |
| An enchant | the same event with `isEnchant` true | | Confirmed | An enchant makes no item. It counts as a craft of the recipe, not of an item. |
| Specialization | `LEARNED_SPELL_IN_SKILL_LINE`, `SPELLS_CHANGED` | `C_SpellBook.IsSpellKnown(spellID)`, `C_Spell.GetSpellName`; `specializationIndex` of `GetProfessionInfo` | Confirmed (`IsPlayerSpell` and the global `GetSpellInfo` are **not** in the dump) | A Classic specialization is a spell: 20219 Gnomish Engineer, 20222 Goblin Engineer, 9787 Weaponsmith, 9788 Armorsmith, 17039 Master Swordsmith, 17040 Master Hammersmith, 17041 Master Axesmith, 10656 Dragonscale, 10658 Elemental, 10660 Tribal Leatherworking. Game test: are these the spell ids of Forever? `C_ProfSpecs` is the Dragonflight tree, not this. |
| Gathering | `UNIT_SPELLCAST_SUCCEEDED` (unitTarget, castGUID, spellID) of a gathering spell, then `LOOT_READY` or `LOOT_OPENED` (autoLoot, isFromItem); `CHAT_MSG_OPENING` (`OPEN_LOCK_SELF`, "You perform %s on %s.") only as a trigger | `GetNumLootItems`, `GetLootSlotLink`, `GetLootSlotInfo`, `GetLootSlotType`, `GetLootSourceInfo` (the GUID of the node or the corpse) | Confirmed | Herb Gathering, Mining, and Skinning are spells, so the spell of the cast says which profession it was. A `GameObject-` source is a herb or an ore node. A `Creature-` source after the Skinning spell is a skinned corpse. `CHAT_MSG_OPENING` is secret in a lockdown, so its words never count. |
| Fishing | `UNIT_SPELLCAST_CHANNEL_START` and `_STOP` of Fishing, then `LOOT_READY` | `IsFishingLoot()`, the loot functions above | Confirmed | The same detection as Fishy Fishy 5.1, and the same tests T4 and T5 in the game. `LootFrame.lua` of the client uses `IsFishingLoot()`. |
| Place | `ZONE_CHANGED*` (built) | `GetRealZoneText`, `GetSubZoneText`, `C_Map.GetBestMapForUnit`, `C_Map.GetPlayerMapPosition` (built, `Position.lua`) | Confirmed | The position rules of GAMEPLAY.md 3.6 hold. |
| Item data | `GET_ITEM_INFO_RECEIVED` (built) | `C_Item.GetItemInfo`, `C_Item.GetItemNameByID`, `C_Item.GetItemQualityByID`, `C_TooltipInfo.GetItemByID` (flavor text, `item-stories.md` source 2) | Confirmed (the global `GetItemInfo` is **not** in the dump) | The 60-second wait of `item_equipped` holds. |
| Secret values | | `issecretvalue` (built) | Confirmed | Every name, GUID, and number from these calls goes through `issecretvalue` first, as today. |

**Missing from the dump:** `GetTradeSkillLine`, `GetNumTradeSkills`, `GetTradeSkillInfo`, `GetCraftInfo`, `GetNumCrafts`, `CRAFT_SHOW`, `CRAFT_UPDATE`, `NEW_PROFESSION_LEARNED`, `LEARNED_SPELL_IN_TAB`, `IsPlayerSpell`, the global `GetSpellInfo`, and the global `GetItemInfo`. No event says "a rare fish bites" or "a node is near".

**The API gate.** The addon has none of these calls today except `C_SkillInfo`. `scripts/wow-api.sh` of Gnomish Relay writes `addon/tests/api.lua` again from the calls of the addon, so the build adds each new name to the gate and to the fake game of the tests.

### 1.3 Game tests before the build

| # | What we assume | Test |
|---|---|---|
| P1 | `GetProfessions` gives the indexes of first aid, fishing, and cooking in that order in Forever. | `/run print(GetProfessions())` with all three learned. |
| P2 | `SKILL_LINES_CHANGED` fires on each skill up. | Craft a recipe that gives a point. Print the event. |
| P3 | `NEW_RECIPE_LEARNED` or `LEARNED_SPELL_IN_SKILL_LINE` fires for a trainer recipe and for a recipe item. | Learn one of each. Print both events. |
| P4 | `TRADE_SKILL_ITEM_CRAFTED_RESULT` fires for a Classic craft, with `itemID` and `hyperlink`. | Craft one item and one enchant. |
| P5 | The specialization spell ids of section 1.2 are known spells of Forever. | `/run print(C_SpellBook.IsSpellKnown(20219, Enum.SpellBookSpellBank.Player))` on an engineer of each kind. |
| P6 | `GetLootSourceInfo` gives a `GameObject-` GUID for a node, and a `Creature-` GUID for a skinned corpse. | Gather one herb, one ore, one skin. Print the GUIDs. |
| P7 | The gathering spell ids of `UNIT_SPELLCAST_SUCCEEDED` are stable over ranks of skill. | Print the spell id at skill 1 and at skill 150. |
| P8 | Fishy Fishy T4 and T5 (shared). | As in the Fishy Fishy spec. |

A failed test changes only the source of one fact. The rules of sections 2 to 6 stay.

## 2. Input lines, facts, and moments

### 2.1 New input lines

Each line is a game event with no reply, so the bridge needs no new message. Each name goes into `Input` (`crates/story/src/input.rs`) and `Inputs.lua`. **The relay session hears of each new line before the build** (the standing rule of relay coordination).

| Line | Fields | When the addon sends it | Big moment? |
|---|---|---|---|
| `profession_ranked` | `at`, `skill_line` (a number), `rank`, `max_rank` | At login, one line for each profession. Then at `SKILL_LINES_CHANGED`, only for a profession whose rank crossed a multiple of 75, or whose `max_rank` changed. A rank that drops (an unlearned profession) goes as rank 0. | Yes, when it crosses a milestone |
| `specialized` | `at`, `skill_line`, `spell` (a number) | Once, when a spell of the list of section 1.2 becomes known. At login for a spell that is known already. | Yes |
| `trainer_seen` | `at`, `npc`, `skill_line`, `spot` | At `TRAINER_SHOW` for a trainer of a profession, once for each NPC in a session. It follows the `npc_met` of the gossip. | No |
| `recipe_learned` | `at`, `recipe` (the name), `skill_line`, `source` (`trainer`, `vendor`, `drop`, `quest`, or `unknown`), `from` (the NPC, for `trainer` and `vendor`; the foe, for `drop` when a kill of section 2.4 holds), `zone` | At each recipe learned. | Only for `drop` and `quest` |
| `item_crafted` | `at`, `item`, `skill_line`, `quality`, `level`, `enchant` (`yes` or absent) | The first craft of each item in a session. The addon never sends a second one in the session. | Only for quality 3 or more |
| `gathered` | `at`, `item`, `skill_line`, `node` (`herb`, `ore`, `skin`), `zone`, `subzone`, `spot` | The first gather of each item in each subzone in a session. | No (waits for the 10-minute flush) |
| `fish_caught` | `at`, `item`, `quality`, `zone`, `subzone`, `spot`, `pool` (a number, or absent) | The first catch of each item in each subzone in a session, and every catch of quality 3 or more. | Only for quality 3 or more |

| `past_fish` | `at`, `catches` (a list of `item`, `zone`, `subzone`, at most 48) | Once in a life, from Fishy Fishy (section 7.2). | No |

`fish_caught` has the field names of a Fishy Fishy record (section 7), so one reader in the addon serves both.

**Bounds.** An item name keeps the limits of `item_equipped` (`MAX_NAME_BYTES`). A skill line outside the list of 12 is dropped with no fault. A rank goes in the band 0 to 300 (`SKILL_RANKS`). A larger rank (a bug or a hostile addon) is refused, as a level over 60 is.

### 2.2 The facts: vocabulary version 11

`VERSION` goes from 10 to 11. Each name is a constant of `vocabulary.rs`. A profession, a recipe, a specialization, and an item are things. The story program maps a skill line id to the English name of its profession (`professions.rs`), so no client language changes a fact.

| Fact | Holder, target | Shape | What it says |
|---|---|---|---|
| `profession` | you, thing (the profession) | flag | You know this profession now. Unlearning ends it. |
| `skill_rank` | you, thing (the profession) | number in `SKILL_RANKS` (0 to 300), any direction | Your skill now. It can drop to 0. |
| `craft_milestone` | you, thing (the profession) | number in `MILESTONES` (75 to 300), up only | The highest milestone told or passed. It never goes down, also after an unlearn. |
| `specialized` | you, thing (the specialization, such as "Gnomish Engineering") | up flag | Once in a life for each specialization. |
| `trains` | person (the NPC), thing (the profession) | up flag | This NPC trains this profession. |
| `recipe_learned` | you, thing (`recipe: <name>`) | up flag | |
| `recipe_of` | thing (the recipe), thing (the profession) | up flag | |
| `recipe_from` | thing (the recipe), person or place | up flag | The trainer, the vendor, the foe, or the zone of the drop. |
| `recipe_source` | thing (the recipe) | number in `SOURCES` (0 unknown, 1 trainer, 2 vendor, 3 drop, 4 quest) | |
| `crafted` | you, thing (the item) | up flag | Your first craft of this item. The item holds `quality` (built). |
| `first_epic_craft` | you, thing (the item) | up flag | Once in a life. |
| `gathered` | you, thing (the item) | up flag | |
| `gathered_in` | thing (the item), place (the subzone) | up flag | Where it grows or lies. The place holds the spot (built). |
| `caught` | you, thing (the fish) | up flag | |
| `caught_in` | thing (the fish), place (the subzone) | up flag | |
| `rare_catch` | you, thing (the fish) | up flag | A catch of quality 3 or more, once for each fish. |

A recipe lives as a thing apart from items, as a mount does (`mounts.rs`): its name has the prefix `recipe: `. The prefix keeps "Arcanite Reaper" the item and "Arcanite Reaper" the recipe apart.

**What the story program does with a line:**

- `profession_ranked`: sets `profession` and `skill_rank`. Then the milestone rule of section 2.3 decides `craft_milestone`.
- `specialized`: the spell id maps to its name (`professions.rs`). An unknown spell id is dropped.
- `trainer_seen`: the NPC exists (`npc_met` makes it). It gets `trains`.
- `recipe_learned`: the recipe thing, `recipe_learned`, `recipe_of`, `recipe_source`, and `recipe_from` when `from` or the zone is known.
- `item_crafted`: `crafted` only if you never crafted it. `first_epic_craft` only for quality 4 or more, once in a life.
- `gathered` and `fish_caught`: the first of each item sets `gathered` or `caught`, the first of each item in each place sets `gathered_in` or `caught_in`, and a quality of 3 or more sets `rare_catch`. A repeat changes no fact, so it makes no event.

### 2.3 The milestone rule

A pure rule in `crates/rules/src/craft_milestones.rs`, in loop style for Aeneas:

```rust
/// The milestone to tell for one new rank of one profession, or none.
pub fn milestone(seen_before: bool, told: u16, rank: u16) -> Option<u16>
```

1. `MILESTONES` are 75, 150, 225, and 300.
2. **The first rank of a profession tells nothing.** It is the baseline: the rank that the character had before Timeways saw it, from the login or from the past (`past_read` carries the professions today). `told` takes the highest milestone at or below that rank.
3. Later, a rank that reaches a milestone above `told` tells **only the highest milestone it reached**, and `told` takes it. A jump from 70 to 160 tells 150, never 75 and 150.
4. A rank at or below `told` tells nothing. So an unlearn and a new start tell no milestone a second time.
5. Fishing and cooking and first aid follow the same rule. Their lore is thinner, so more of their milestones are silent (section 4).

### 2.4 Where a recipe came from

A pure rule, `recipe_source` in `crates/rules/src/craft_milestones.rs`, run in the addon logic as a table and mirrored in the story program. The first that holds wins:

1. A trainer window was open in the last 10 seconds: `trainer`, from that NPC.
2. A merchant window was open, and the recipe item was bought in the last 60 seconds: `vendor`, from that NPC.
3. The recipe item came from a loot window of a corpse in the last 10 minutes: `drop`. The foe is the source only when the story program knows the corpse as a rare or a boss that you killed (`npc_defeated`). Else the source is the zone (a world drop).
4. The recipe item came as a quest reward (`QUEST_TURNED_IN` in the last 60 seconds): `quest`.
5. Else `unknown`.

A trainer recipe and a vendor recipe are no famous first, because every player of the profession gets them.

### 2.5 New moments

| Moment | Fields | Subjects of its lore | Rank in `moments.rs` |
|---|---|---|---|
| `CraftMilestone` | `profession`, `rank`, `zone` | the craft groups of the pairing (section 5), then the craft pages | 2 for 75, 150, and 225, as a tenth level; 5 for 300 |
| `Specialized` | `specialization`, `profession` | the specialization, then its trainer | 5 |
| `RareRecipe` | `recipe`, `source`, `from`, `zone` | the item of the recipe, then `from` (a foe with lore) | 3 |
| `NotableCraft` | `item`, `profession` | the item alone | 3 |
| `FirstEpicCraft` | `item`, `profession` | the item alone | 5 |
| `RareCatch` | `fish`, `zone` | the fish alone | 3 |

- **The item-stories rule** (`item-stories.md`) holds for `RareRecipe`, `NotableCraft`, `FirstEpicCraft`, and `RareCatch`: a line only when the pack has a page about the item, the item has flavor text, a quest text that the player read tells of it, or it came from a boss with lore. Else the moment adds weight and stays silent. The rule of thin lore (`thin_lore::is_silent`) does the rest: a deed with no passage about its subject is silent.
- `NotableCraft` is a first craft of an item of rare quality or more, or of an item that has a pack page.
- `KINDS` of `moments.rs` gets the six names, so the scenario test and the smoke run check that some play makes each one.
- `is_deed` is true for all six. None is an arrival.
- `outside_names`: the item, the recipe, the fish, and the specialization. Their names come from the game, so they allow no banned word in the history (GAMEPLAY.md 3.2).
- A milestone, a specialization, and a first epic craft are big moments: the addon sends their lines within 5 seconds.

## 3. Weights and no-bloat keys

New rows of `docs/plans/chapters.md` 4, and new arms of `KeyKind` and `weight` (`crates/rules/src/weights.rs`):

| Event | Key | Weight | Track |
|---|---|---|---|
| A milestone at 75, 150, or 225 | `(CraftRank, profession, milestone)` | 2 | `World`, always, as a level |
| A milestone at 300 | `(CraftRank, profession, 300)` | 3 | `World`, always |
| A specialization | `(Specialization, spec)` | 3 | `World`, always |
| A first epic craft | `(EpicCraft, first)` | 3 | where you stood |
| A notable first craft | `(Craft, item)` | 1 | where you stood |
| A recipe from a drop or a quest | `(Recipe, recipe)` | 1 | where you stood |
| A rare catch | `(RareCatch, fish)` | 2 | where you stood |
| A recipe from a trainer or a vendor | none | 0 | |
| A plain first craft (below rare, no page) | none | 0 | |
| A gather, first or repeat | none | 0 | |
| A plain catch, first or repeat | none | 0 | |
| A trainer met | `(Talk, npc)` (built) | 1, through `npc_met` | |

**No bloat.**

- A key weighs only the first time (theorem 11 of `chapters.md`). Every new row is a first.
- A repeat makes no event at all: the world holds each fact once. A second craft, gather, or catch of one item changes no fact.
- Gathering and plain fishing weigh 0. A herbalist who picks 2,000 Peacebloom makes no chapter. Knowledge still shows the spots.
- The keys are bounded: 12 professions × 4 milestones, 10 specializations, one epic craft, and the notable items and rare fish that the game has. So profession weight in a life is bounded too (theorem 13 of `chapters.md`).
- `W_MAX` and `CAP_MAX` stay 7: no new row weighs more than 3.
- A milestone is never a break, as a mount is never a break. A tenth level stays the only level break.
- **Tally lines.** A repeat sends nothing, so no profession tally line exists. "Crafted Arcanite Reaper again" is no news.

**The rule epoch.** The new kinds never happen in an old log, so the weight of every old step stays the same, and no closed chapter moves. The fold needs no new rule number (`chapter_rules`). A property test checks it: the chapters of each scenario world are the same before and after the change.

## 4. Lore sources: a pack choice for the user

Checked in the Wowpedia dump of 2026-10-05. Nothing goes into `pack_sources.toml` until the user approves. "In pack" means the page is there today.

**The profession pages themselves are mostly game talk.** "Blacksmithing", "Alchemy", "Engineering", "Herbalism", "Mining", "Tailoring", "Enchanting", "Leatherworking", "Skinning", "Cooking", "First Aid", and "Fishing" open with "Official overview", training, and skill tables. Only "Engineering" has a lore section ("In Warcraft RPG", question 2). So the lore of a craft comes from the pages of its people and its orders.

| Profession | Pages | In dump | In pack | Notes |
|---|---|---|---|---|
| Blacksmithing | Great Forge | yes | no | The first part of Ironforge, the city grew around it, it never shuts down. Good lore. |
| | Mithril Order | yes | no | Mithrilsmiths of every faction, led by Galvan the Ancient. The second half is a quest guide. |
| | Thorium Brotherhood | yes | **yes** | Dark Iron smiths outside the Dark Iron clan, at Thorium Point. Its recipes come with patch 1.x reputation. |
| | Lokhtos Darkbargainer | yes | no | The Brotherhood's trader in Blackrock Depths. Molten Core materials: wait for Forever (decided, question 1). |
| | Dark Iron dwarf | yes | **yes** | |
| | Arcanite Reaper, Truesilver Champion | yes | no | Item pages. The reaper has a quote of the RPG about a rite of passage. |
| | Weaponsmithing, Armorsmithing, Hammersmith | yes | no | Game talk. Leave them out. |
| Engineering | Tinker Town | yes | no | The gnomish quarter of Ironforge, built by the exiles of Gnomeregan. |
| | Gnomeregan | yes | **yes** | |
| | Gelbin Mekkatorque | yes | no | Long. Later sections tell of Operation: Gnomeregan (after the cutoff). |
| | Tinkers' Union | yes | no | The goblin union of Undermine, with workshops in Gadgetzan. |
| | Gadgetzan, Steamwheedle Cartel | yes | **yes** | |
| | Gnome Engineering, Goblin Engineering | yes | no | Game talk about the quests. Their trainers (Tinkmaster Overspark, Nixx Sprocketspring) are better as NPC pages. |
| | Gnomish Battle Chicken, Goblin Rocket Boots, Gnomish Cloaking Device, Mithril Mechanical Dragonling | yes | no | Item pages, mostly game data. Keep only those with a quote of the game: no "In the RPG" (decided, question 2). |
| Alchemy | Royal Apothecary Society | yes | **yes** | Forsaken only. |
| | Alchemist | yes | no | A lore page of the craft: potions, transmutation, and goblin alchemists. |
| | Arcanite Bar, Flask of the Titans | yes | no | Item pages. |
| Herbalism | Cenarion Circle | yes | **yes** | Night elf and tauren druids. |
| | Herb | yes | no | A definition, no lore. Leave it out. |
| | Black Lotus, Arthas' Tears, Peacebloom, Earthroot | yes | no | Item pages. "Arthas' Tears" has a History section. Most are game data. |
| Mining | Miners' League | yes | no | Dwarven miners: Gol'Bolar Quarry, Silver Stream Mine, the Deadmines. Good lore. |
| | Explorers' League, Venture Company | yes | **yes** | |
| | Thorium, Truesilver, Elementium | yes | no | "Thorium" has a Background section. |
| Skinning, Leatherworking | Leatherworking specialization | yes | no | Game talk. |
| | Devilsaur | yes | no | A beast page with an "In the RPG". |
| | Timbermaw tribe | yes | **yes** | |
| | (no page) | | | Nothing tells of the leatherworkers of a people. Thin, so silent for now (decided, question 3). |
| Tailoring | Mooncloth | yes | no | Cloth made with the power of the moonwells. |
| | Moonglade | yes | **yes** | |
| | Felcloth, Robe of the Archmage | yes | no | Item pages. |
| Enchanting | Enchanter | yes | no | The lore of enchanters as a school of arcane magic. |
| | Kirin Tor | yes | no | Long, and much of it is after the cutoff. |
| Fishing | Nat Pagle | yes | no | The most famous fisherman. His biography is good. Later sections drop on the cutoff. |
| | Booty Bay | yes | **yes** | |
| | Old Ironjaw, Old Crafty | yes | no | The rare fish of Ironforge and Orgrimmar. Both are of uncommon quality, so a rare catch needs the page, not the quality (question 4). |
| | Stranglethorn Fishing Extravaganza | yes | no | A later patch: it waits for Forever (decided, question 1). |
| Cooking | Dirge Quikcleave | yes | no | NPC page, thin. |
| | Thunderbrew Distillery, Barleybrew | yes | no | Thin. |
| First Aid | Doctor Gregory Victor, Doctor Gustaf VanHowzen | yes | no | The Triage quest. NPC pages with quotes. |
| | Argent Dawn | yes | **yes** | |

**Missing from the dump:** "Ironforge Mountaineers", "Brotherhood of Thorium" (the page is "Thorium Brotherhood"), "Gnomish Inventor", "Mekgineer", "Gizlock Gizmotinker", "Hemet Nessingwary", "Ironforge Brewery".

**The link model.** As in `mount-lore.md` 3, a craft page gets no new link kind. A page gets a new optional list `crafts = ["Blacksmithing"]`. Its passages count as craft lore for those professions, and they are common knowledge, like a family page of a mount. A page in the pack today keeps its `places`, so the Great Forge stays a place page and also serves Blacksmithing.

**The pick is deterministic**, as in a tenth level: milestone 75 takes the first fitting passage of the craft pages, in page order, 150 the second, and so on. A passage that a narrator call of the character told before is skipped (`told_lore`). With no passage left, the milestone is silent.

## 5. Template parts for milestone lines

The milestone uses the template model of `narrator-templates.md`: the model writes the lore, the code writes the hero. A template is data, and the user approves every part.

### 5.1 Frames

| Frame | Order | For |
|---|---|---|
| `f.craft` | lore. [connective] group grow. coda. | a milestone |
| `f.craft_joined` | lore. [connective] group grow, and coda. | a milestone, named coda only |
| `f.deed` (built) | lore. [connective] deed. | a specialization, a recipe, a craft, a rare catch |

### 5.2 New slots

`craft` ("Blacksmithing"), `crafter` ("blacksmith"), `crafter_pl` ("blacksmiths"), `rank` (75 to 300), `rank_title` (Apprentice, Journeyman, Expert, Artisan), `spec` ("Goblin Engineering"), `recipe`, `fish`. The crafter words come from `professions.rs`. First Aid has no crafter word, so a part with `crafter` never fits it.

### 5.3 Groups of a craft (faction-aware)

Built as in `narrator-templates.md` 3.4, not listed for each pair.

| Id | Text | Number | Fits | Anchors |
|---|---|---|---|---|
| `pg.faction` | the {crafter_pl} of the {faction} | many | every pair | |
| `pg.capital` | the {crafter_pl} of {capital} | many | every pair: the capital of the race | the capital |
| `po.great_forge` | the smiths of the Great Forge (short: the smiths) | many | Blacksmithing, Alliance | Great Forge |
| `po.mithril` | the Mithril Order (short: the order) | one | Blacksmithing, any faction, milestone 150 or more | Mithril Order |
| `po.tinker_town` | the engineers of Tinker Town | many | Engineering, Alliance | Tinker Town, Gnomeregan |
| `po.gnomish` | the gnomish engineers | many | Gnomish Engineering, Alliance | gnome, Gnomeregan |
| `po.goblin` | the goblin engineers of Gadgetzan | many | Goblin Engineering, any faction | Gadgetzan, goblin |
| `po.apothecaries` | the Royal Apothecary Society (short: the Society) | one | Alchemy, Forsaken | Apothecary |
| `po.circle_herbs` | the herbalists of the Cenarion Circle | many | Herbalism, night elf or tauren | Cenarion Circle |
| `po.miners` | the Miners' League (short: the League) | one | Mining, dwarf | Miners' League |
| `ps.horde_gnomish` | the {people_pl} who build gnomish devices | many | Gnomish Engineering, Horde | gnomish |

- `po.goblin` fits any faction: the goblins are neutral, so a goblin engineer of the Alliance is no strange pairing.
- `ps.horde_gnomish` is the strange pairing of crafts: a Horde engineer who chose the gnomes. It never takes a gnomish group of the Alliance.
- A Horde smith, leatherworker, tailor, or cook gets only `pg.faction` and `pg.capital` for now: thin crafts stay silent (decided, question 3).
- A line never says that the hero fights their own faction. The check of `level-lines.md` holds.

### 5.4 Grow verbs

The five verbs of `narrator-templates.md` 9.4 serve the milestones as they are: `v.stronger`, `v.strength`, `v.now`, `v.little`, `v.grown`. No new verb.

### 5.5 Codas

| Id | Alone | Joined (after "and") |
|---|---|---|
| `cc.reached` | {Hero} has reached {rank} in {craft}. | {hero} has reached {rank} in {craft}. |
| `cc.title` | {Hero} is {a} {rank_title} {crafter} now. (tag: now) | {hero} is {a} {rank_title} {crafter} now. |
| `cc.makes_u` | That makes {rank} in {craft}. | (none) |
| `cc.none_u` | (nothing) | (none) |

`{a}` is "a" or "an" from the first letter of the next word, with the exception list of `narrator-templates.md` 3.7: "an Expert blacksmith", "an Artisan tailor".

### 5.6 Deeds of the famous firsts

| Id | Text | Moment | Needs |
|---|---|---|---|
| `pf.made` | {Hero} has made {a} {item}. | notable craft, epic craft | |
| `pf.signed` | {Hero} signed the pledge of {spec}. | specialization | Goblin Engineering only (the pledge is in its quest) |
| `pf.took_up` | {Hero} has taken up {spec}. | specialization | |
| `pf.learned` | {Hero} has learned to make {item}. | rare recipe | |
| `pf.from_foe` | {Foe} carried the plans, and {hero} has them now. | rare recipe | a drop from a known foe |
| `pf.caught` | {Hero} has caught {fish}. | rare catch | |
| `pf.out_u` | {Fish} is out of the water now. | rare catch | unnamed |
| `pf.in_hands` | {Fish} is out of the water now, in the hands of {hero}. | rare catch | |

The connectives of 9.2 (`c.now`, `c.there`, `c.in_zone`) fit as they do for items.

### 5.7 The slot JSON

| Moment | Closed fields | Check |
|---|---|---|
| Milestone | `group` | The lore names an anchor of the group. |
| Specialization, rare recipe, crafts, rare catch | `there` | `true` needs the lore to name the zone of the moment. |

## 6. Knowledge and the atlas

UI copy follows the UI copy rules of `CLAUDE.md`. The words below are drafts for review.

**A zone page** gets two lists, each with 3 rows and "Show all 9":

- **Gathered here.** Each item with its subzone and its kind: "Peacebloom · Brackwell Pumpkin Patch". From `gathered_in`.
- **Fish caught here.** Each fish with its subzone. With Fishy Fishy installed, each fish also shows its share: "Oily Blackmouth · 62%". From `caught_in`, and from the totals of Fishy Fishy (section 7).
- The counts in small boxes get "4 herbs", "2 ores", and "6 fish". Never a total with "of".

**The map of a zone** gets one pin for each subzone with a gather or a catch, at the spot of the place. The tooltip names what you found there: "Brackwell Pumpkin Patch", "Peacebloom, Silverleaf". A herb, an ore, and a fish pin use the minimap icons of the game when the atlas of the client has them (game test).

**A person page** of a trainer gets "Trains Blacksmithing" under the name, and "Taught you 12 recipes" in "Between you". From `trains` and `recipe_from`.

**A new page: Professions.** A button "Professions" on the world page of Knowledge opens it. It holds, for each profession: the name, the rank and its title ("Blacksmithing · 225 · Expert"), the specialization, the milestones with their chapter ("150 · Ch. 6"), and the recipes. A recipe row says where it came from:

- "Learned from Bengus Deepforge, Ironforge"
- "Bought from Jaquilina Dramet, Stranglethorn Vale"
- "Dropped by Overmaster Pyron, Searing Gorge"
- "A drop in Winterspring"
- "A quest reward"

The page lists at most 40 recipes for each profession, newest first, with "Show all 61". An empty page says "Learn a profession from a trainer, and it shows up here."

**The Chronicle.** A chapter page lists each profession deed in "In this chapter": "Reached 150 in Blacksmithing", "Took up Goblin Engineering", "Made Arcanite Reaper", "Learned to make Mooncloth", "Caught Old Ironjaw". Each item and place is a link to Knowledge. The deed mark of a chapter in the contents gets no new mark: a milestone is not a break.

**The journal budget.** The new lists go through the page budget of `journal.rs`, so a page still fits 1600 bytes for each string and the strip limits. A property test checks it, as for the lore of the atlas.

## 7. Sharing with Fishy Fishy

The two addons stay independent. Each works alone, and neither needs the other.

### 7.1 The contract

- **Fishy Fishy offers, Timeways reads.** Fishy Fishy defines one global table, `FishyFishyAPI`, with `version = 1` and three functions. Timeways never reads `FishyFishyDB` itself, so Fishy Fishy can change its storage.
  - `FishyFishyAPI.Zones()`: an iterator of `mapID, subzone, totals`. `totals` holds `casts`, `catches`, `misses`, and `items` (`itemID` to `count`).
  - `FishyFishyAPI.Catches(since)`: the records of the log with a `time` after `since`, oldest first.
  - `FishyFishyAPI.Version()`: the contract version.
- **One record**, the fields of Fishy Fishy 5.2, by the same names: `time`, `itemID`, `count`, `quality`, `mapID`, `zone`, `subzone`, `x`, `y` (thousandths, as `Position.lua` uses), `skill`, `pool`, and one new field, `character` ("Name-Realm"). The Fishy Fishy log is for the whole account (its question 4), so Timeways needs the name to keep only the catches of the character.
- **A version Timeways does not know** is ignored, with no error, and Knowledge shows only the own facts of Timeways.
- The load order does not matter: Timeways asks at `PLAYER_ENTERING_WORLD`, after both addons loaded, with `C_AddOns.IsAddOnLoaded("FishyFishy")` (confirmed, in the gate today).

### 7.2 What Timeways does with it

- **Live catches.** Timeways detects its own catches (section 1.2). It never turns a Fishy Fishy record into a `fish_caught` line while both run, so a catch never counts twice.
- **The past.** At the first login with both addons, Timeways reads the catches of the character from before its first event. They go as one `past_fish` line, once in a life: the fish and the subzones, at most 48 pairs, with no count. Like `past_read`, the line makes no moment and no weight, and the fold of the chapters never reads it. It only fills "Fish caught here". A rare fish from before Timeways stays silent.
- **The shares.** "Fish caught here" asks `Zones()` each time the journal opens, and adds the share of each fish to the rows. The share is never sent to the desktop. It lives only in the addon, so it costs no strip bytes.
- **No write.** Timeways never calls a Fishy Fishy function that changes anything, and never sets a key of its tables.

### 7.3 Asks for the Fishy Fishy spec

These go to the session that writes the Fishy Fishy spec. They do not change its design.

1. Add `FishyFishyAPI` with the three functions of 7.1, and a test that its records keep the field names.
2. Add `character` to each record.
3. Keep `x` and `y` in thousandths, or name the unit in the contract.
4. A version bump of `FishyFishyAPI` for each change of a record field.

## 8. Tests, fuzz seeds, and proofs

### 8.1 Unit and scenario tests (story program)

- `a_milestone_tells_at_seventy_five`
- `the_first_rank_of_a_profession_tells_nothing`
- `a_jump_tells_only_the_highest_milestone`
- `an_unlearned_profession_never_tells_a_milestone_again`
- `a_milestone_with_no_craft_passage_is_silent`
- `two_milestones_of_one_profession_never_use_one_passage`
- `a_forsaken_alchemist_gets_the_apothecaries`
- `a_horde_smith_never_gets_the_great_forge`
- `a_horde_gnomish_engineer_gets_the_strange_pairing`
- `a_specialization_counts_once`
- `a_trainer_recipe_is_no_famous_first`
- `a_drop_recipe_names_its_foe_only_after_the_kill`
- `a_craft_with_no_story_adds_weight_and_stays_silent`
- `the_first_epic_craft_counts_once_in_a_life`
- `a_rare_catch_counts_once_for_each_fish`
- `a_gather_adds_no_weight`
- `a_repeat_catch_makes_no_event`
- `an_unknown_skill_line_is_dropped`
- `a_rank_over_three_hundred_is_refused`
- `old_chapters_stay_the_same_with_the_new_kinds` (every scenario world)
- `every_profession_moment_comes_from_some_play` (the `KINDS` test of the scenarios)

### 8.2 Property tests (`crates/story/tests/properties.rs`)

- For any sequence of ranks of one profession, each milestone is told at most once, and only a milestone that some rank reached. The draw makes 0, 74, 75, 76, 149, 150, 224, 225, 299, and 300 likely, and drops to 0.
- For any play, the profession weight of a life is at most 12 × (2 + 2 + 2 + 3) + 10 × 3 + 3 + the notable keys.
- For any play of gathers and plain catches alone, no chapter closes and no tale opens.
- The Knowledge pages with any number of gathers, catches, and recipes keep the page budget.

### 8.3 Addon tests (Lua, the fake game)

- `the_rank_comes_from_the_function_not_the_chat`
- `a_secret_chat_line_is_never_read`
- `a_gather_goes_once_for_each_item_and_subzone_in_a_session`
- `a_skinned_corpse_counts_as_skinning`
- `a_trainer_recipe_names_its_trainer`
- `a_bought_recipe_names_its_vendor`
- `a_fishy_fishy_catch_never_counts_twice`
- `an_unknown_fishy_fishy_version_is_ignored`
- `timeways_works_without_fishy_fishy`
- `a_catch_of_another_character_is_left_out`

### 8.4 Fuzz

- New seeds in `fuzz/seeds/input/professions.txt`: one line of each new kind, a rank of 301, a rank of -1, an unknown skill line, an empty item name, a name at `MAX_NAME_BYTES` + 1, a `source` outside the list, and a `pool` that is not a number.
- New words in `fuzz/dicts/input.dict`: the line names, the field names, and the `source` and `node` values.
- The `play` target gets the new lines, so the fold of the chapters sees them in any order.
- Fishy Fishy records are read in Lua, not in Rust. The Lua tests cover a record with missing fields, wrong types, and a huge `count`.

### 8.5 Lean theorems (Aeneas only)

The pure rules live in `crates/rules` in loop style, so Aeneas extracts them. New file `lean/Timeways/CraftMilestones.lean`:

| Theorem | In plain words |
|---|---|
| `a_milestone_is_told_at_most_once` | Over any run of ranks of one profession, no milestone value comes out twice. |
| `a_milestone_needs_its_rank` | A milestone comes out only at a rank that reaches it. |
| `the_first_rank_tells_nothing` | The baseline rank never tells a milestone. |
| `a_jump_tells_the_highest_milestone` | A rank that passes two milestones tells only the higher one. |
| `told_never_goes_down` | The told milestone only grows, also when the rank drops to 0. |
| `a_trainer_recipe_is_never_rare` | `recipe_source` with a trainer window open gives `trainer`, so the moment is no famous first. |
| `milestone.spec`, `recipe_source.spec` | The rules never panic and always end. |

In `lean/Timeways/Chapters.lean`, the theorems of `chapters.md` 8 cover the new kinds when `KeyKind` grows. Two new statements:

| Theorem | In plain words |
|---|---|
| `a_gathering_repeat_adds_no_weight` | A step of a gather or a plain catch gains 0, first or repeat, under every rule. |
| `the_new_kinds_keep_w_max` | No weight of the new kinds passes 3, so `W_MAX` and `CAP_MAX` stay 7, and theorems 8 and 13 keep their constants. |

The milestone passage pick reuses the proved rule `instance_lore::next_passage`: no passage is told twice.

## 9. Dev mode and the smoke test

A standing rule: every feature ships with a trigger and a smoke step.

### 9.1 `/twdev` triggers (`DevTriggers.lua`)

Each one starts at the narrowest point after the game API, as the others do.

| Command | What it fakes |
|---|---|
| `skill <profession> <rank>` | `profession_ranked` with that rank. `skill Blacksmithing 150`. |
| `spec <name>` | `specialized` for a name of the list: `spec Goblin Engineering`. |
| `trainer <npc> / <profession>` | `npc_met`, then `trainer_seen`. |
| `recipe <name> [trainer <npc> \| vendor <npc> \| drop <foe> \| quest]` | `recipe_learned` with that source. |
| `craft <item> [rare \| epic]` | `item_crafted` with quality 2, 3, or 4. |
| `gather <item> [herb \| ore \| skin]` | `gathered` in the zone where you stand. |
| `fish <fish> [rare]` | `fish_caught` in the zone where you stand. |
| `fishy` | A fake `FishyFishyAPI` with 6 records of the character in two zones, and one of another character. It lives in memory only. |

### 9.2 The scenario

`crates/dev/scenarios/professions.jsonl`: a level 40 dwarf with Mining and Blacksmithing, a baseline at 70 and 140, then 75, a jump to 160, a trainer, a trainer recipe, a drop recipe from a known foe, a rare craft, an epic craft, 30 gathers in two zones, a rare catch, and an unlearn of Mining with a new start to 80. `timeways-dev seed` plays it. The scenario test checks each moment kind and the weights.

### 9.3 Smoke steps (`DevSmoke.lua`)

After the mount and item steps:

| Step | Runs | Expects | Waits for |
|---|---|---|---|
| `skill-baseline` | `skill Blacksmithing 140` | `profession_ranked`, no moment | |
| `skill-milestone` | `skill Blacksmithing 150` | `profession_ranked`, the moment `craft_milestone` | narrator |
| `skill-again` | `skill Blacksmithing 150` | no new fact | |
| `trainer` | `trainer Bengus Deepforge / Blacksmithing` | `trainer_seen`, the fact `trains` | |
| `recipe-trainer` | `recipe Golden Scale Bracers trainer Bengus Deepforge` | `recipe_learned`, no moment | |
| `recipe-drop` | `recipe Arcanite Reaper drop Overmaster Pyron` | `recipe_learned`, weight 1 | |
| `craft-epic` | `craft Arcanite Reaper epic` | `item_crafted`, the moment `first_epic_craft` | narrator |
| `spec` | `spec Goblin Engineering` | `specialized` | narrator |
| `gather` | `gather Peacebloom herb`, twice | one `gathered`, weight 0 | |
| `fish-rare` | `fish Old Ironjaw rare` | `fish_caught`, the moment `rare_catch` | |
| `fishy` | `fishy`, then open Knowledge | the share on "Fish caught here", no catch of the other character | |
| `professions-page` | open the Professions page | the rows of the recipes with their sources, no Lua error | |

The story program adds the facts, the weights, the moments, and the calls of each step to the smoke log. A step with a narrator line gets WAIT, not FAIL, when the budget of the narrator is full.

## 10. Migration notes

- **Vocabulary 10 to 11.** New fact names only. No old event changes, so `hourglass::migrate` has nothing to rewrite.
- **The SQLite store.** No new table. The new facts are events in `events`. Nothing is live, so a world of version 10 starts fresh. After the release on CurseForge, this change needs an upgrade step that only takes the new version number, because no row changes.
- **The chapter fold.** No new rule number (section 3). The test of section 8.1 proves that no closed chapter moves.
- **The past.** `past_read` carries the professions today. The baseline of section 2.3 reads them, so an existing character never gets the milestones it passed before Timeways. `past_fish` is a new line, kept like the table `past`: only the first one counts.
- **The pack.** `crafts` is a new optional list of a page. The pack format takes a new minor version, and an old pack reads with no craft lore: every milestone is silent until the player builds the pack again.
- **The templates.** New parts in `narrator_templates.toml`. The loader checks them, and `the_templates_load` fails CI first.
- **The relay.** New event lines with no reply. The bridge passes them as it passes every event line. The relay session hears of them before the build.

## 11. Open questions: decided

The user decided each question on 2026-10-09. The plan follows these answers. Nothing of it is built yet.

1. **The patch cutoff.** Decided: later-patch content waits for Forever. The Thorium Brotherhood recipes, Lokhtos and his Molten Core trades, and the Stranglethorn Fishing Extravaganza wait until Forever opens them, as Dire Maul does.
2. **"In the RPG" sections.** Decided: no "In the RPG" sections. A craft takes no lore from the RPG, the same rule as `mount-lore.md` 6.2.
3. **Thin crafts.** Decided: thin-lore crafts stay silent for now. Leatherworking, skinning, tailoring, cooking, and first aid get no milestone line until the pack has lore for their people. No passage is added by hand.
4. **What is a rare fish?** Decided: a rare catch is a fish of quality 3 or more, OR a fish with its own pack page. So Old Ironjaw and Old Crafty count by their pages.
5. **Gathering weight.** Decided: the first gather weighs 0, as the spec says. A herbalist's hours make no chapter.
6. **Fishing milestones.** Decided: fishing is a profession with milestones, and it has rare catches too.
7. **The Professions page.** Decided: a new page under Knowledge.
8. **Specialization words.** Decided: "signed the pledge" is fine. It is common knowledge in 25 ADP, not a spoiler.
9. **Fishy Fishy.** Decided: the asks of 7.3 and the interface of 7.1 are approved, and Timeways reads the old catches of the character as its past.

## 12. Example lines

Written under the voice skill (`timeways-narrator-voice`). Each one names its shape. The lore comes from the pages of section 4. The lore of a real line comes from the pack.

1. Milestone, dwarf, Blacksmithing 150. `f.craft`, `po.great_forge`, `v.stronger`, `cc.reached`, name.
   "The dwarves built the Great Forge before the rest of Ironforge, and its fires still burn at the center of the city. The smiths of the Great Forge grow stronger. $N has reached 150 in Blacksmithing."
2. Milestone, Forsaken, Alchemy 225. `f.craft_joined`, `po.apothecaries` (short), `v.strength`, `cc.reached`, name. "The Forsaken" names the people, and "its alchemists" names a group, so neither the race nor "the alchemist" names the hero (`docs/plans/narrator-style.md` 4.1).
   "Sylvanas created the Royal Apothecary Society to brew a new plague against the Scourge, and its alchemists still work in the Apothecarium of the Undercity. The Society gains strength, and $N has reached 225 in Alchemy."
3. Milestone, gnome, Engineering 300. `f.craft`, `c.now`, `po.tinker_town`, `v.stronger`, `cc.title`, class.
   "The dwarves took in the gnomes when troggs overran Gnomeregan, and the exiles built Tinker Town beside the Deeprun Tram. Now, the engineers of Tinker Town grow stronger. The mage is an Artisan engineer now."
4. Specialization, Goblin Engineering. `f.deed`, `pf.signed`, name.
   "The Tinkers' Union of Undermine keeps workshops in Gadgetzan, and its goblins build their gizmos and weapons against the gnomes and dwarves. $N signed the pledge of Goblin Engineering."
5. Rare catch, Old Ironjaw. `f.deed`, `pf.in_hands`, name.
   "The pools of the Forlorn Cavern and the Mystic Ward hold the rarest fish of Ironforge. Old Ironjaw is out of the water now, in the hands of $N."
6. First epic craft, Arcanite Reaper. `f.deed`, `pf.made`, name.
   "Warriors of Azeroth long took an arcanite reaper as the mark of a true warrior, and having one made was a rite of passage. $N has made an Arcanite Reaper."

These lines pass the checklist of the skill: one turn each, the world as the subject of the lore, the hero only in the deed or the coda, one number at most, no word of the ban list, and no word for the hero that also names a group of the line.

Line 6 takes its lore from the RPG quote on the item page. Question 2 refuses RPG lore, so line 6 needs other lore or stays silent. Question 8 allows line 4.
