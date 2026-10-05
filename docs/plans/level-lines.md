# Plan: the narrator line of a tenth level

Status: direction set by the user, 2026-10-05. Nothing is built.

## The idea

A tenth level (10, 20, 30...) is about the character, but in the voice of the world. The line tells how the character's **order or people grows stronger** as a group. The level number is optional: the line can leave it out, or close with it as one plain fact.

Examples of the voice (the lore of a real line comes from the pack):

- Human paladin: "The Silver Hand was founded to carry the Light into war. The order grows stronger. $N has reached level 30."
- Forsaken paladin: "The Silver Hand once burned the dead of Lordaeron. Now the dead channel the Light, and their power grows. $N has reached level 20."
- Orc warlock: "The Shadow Council first taught the orcs to bargain with demons. The warlocks of the Horde grow stronger. $N has reached level 10."
- Tauren druid: "The Cenarion Circle keeps the balance of Kalimdor, and its druids grow stronger. The druid has reached level 20."

## The rule

1. **Who grows stronger:** the character's class order, as it fits their race and faction, or the character's people.
2. **Race and class together:** WoW Forever allows any race in any class. A combination that the lore finds strange is a story: the Forsaken paladin is "the dead channel the Light". The line never pretends that a Horde paladin fights the Horde.
3. **The level:** left out, or one short plain fact at the end. Never a feeling about the character ("has grown stronger" alone, "felt the Light stir").
4. **Literal, never inside a person.** The order or the people grows stronger as a group. It never grows, lives, or burns *in* a person: "its power grows in the druid" and "the craft grows sharper in $N" are refused. An order cannot be inside someone.
5. **No repeats:** each tenth level of one character takes a different passage.
6. **The lore comes from the pack**, never from the model's memory. With no fitting passage, the line is silence.

Forbidden here (rejected by the user): fame claims ("the Scarlet Crusade knows the name"), class unlocks ("will now teach you"), callbacks to the Hero page, "the worg would not try it now", and jokes about an NPC who knows you.

## What it needs

| Part | Needs |
|---|---|
| Class order lore | Pages such as the Silver Hand, the Shadow Council, the Cenarion Circle, the Kirin Tor, the Earthen Ring, and the Ravenholdt rogues, in `pack_sources.toml`. This is a new pack choice for the user, as with `docs/plans/mount-lore.md`. Each page gets `about` set to its class. |
| Race lore | The race and capital pages are in the pack today. |
| The pairing | A table from (race, class) to the order, and a flag for a combination that the lore finds strange. The table lives in the story program, with a test for every pair of WoW Forever. |
| The prompt | The level moment gets the task "tell how the order or people grows stronger through the hero", the pairing note, and the passage. The level number is optional. |
| The check | The arrival and slop checks stay. Add a check that refuses a line where the hero fights their own faction, and a check that refuses a group that grows, lives, or burns in the hero ("grows in $N", "lives on in the druid"). |

## Tests

- `a_tenth_level_tells_its_order_growing`
- `a_strange_pairing_gets_its_tension_note`
- `a_tenth_level_with_no_passage_is_silent`
- `two_tenth_levels_never_use_one_passage`
- `a_horde_paladin_never_fights_the_horde` (the check)
- `an_order_never_grows_inside_the_hero` (the check)
