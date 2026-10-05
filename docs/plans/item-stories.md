# Plan: which items get a narrator line

Status: a later idea, 2026-10-05. Nothing is built. The user wants it kept for later.

## The problem

The narrator speaks for a first epic item and for a big upgrade (`gear.rs`, `GAMEPLAY.md` 3.2). Most items have no story. Their line then glues unrelated place lore to the item: "Westfall has lain fallow since the Second War. There $N took up the Cruel Barb." Some items do have a good story, and the code cannot tell which.

## The rule

An item gets a narrator line only if one or more of these is true. Otherwise it only adds chapter weight (`docs/plans/chapters.md` 4).

1. **The lore pack has a page about the item.** Example: Thunderfury. The pack marks it with `about`.
2. **The item has flavor text.** This is the yellow quote on its tooltip. Blizzard writes it only for items with a story.
3. **The item came from a quest whose text the player read.** The quest text is its story. The `learned` rows hold it.
4. **The item dropped from a boss or a rare that the player just killed, and that foe has lore.** Example: "Cruel Barb, taken from VanCleef's own hand."

Each test is a fixed rule over facts, so a test can check it, and a model never decides it.

## What it needs

| Source | Needs |
|---|---|
| 1. A pack page | Item pages in `pack_sources.toml` (a new data choice for the user), and `about` on each. |
| 2. Flavor text | The tooltip text of the item, for example with `C_TooltipInfo.GetHyperlink`. Check it in the API gate and in the game. Flavor text is outside text: fence it in the prompt, and add a fuzz seed. |
| 3. A quest reward | A link from the item to the quest that gave it: the quest turn-in and the item equip close in time, or the reward list of the quest. |
| 4. A boss drop | A link from the item to the kill: the loot event (`CHAT_MSG_LOOT` or a loot API) soon after a kill of a foe with lore. |

## Tests

- `an_item_with_no_story_gets_no_line`
- `an_item_with_flavor_text_gets_a_line`
- `a_quest_reward_tells_its_quest`
- `a_boss_drop_tells_its_boss`
- A property test: any item with none of the four sources makes no narrator moment, and still adds its chapter weight.

## Open questions

- Until this is built, does a big upgrade with no story keep its line, or go silent?
- Does source 2 work in WoW Forever, and is the tooltip text the same in every client language?
