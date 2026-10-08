---
name: timeways-npc-voice
description: The voice of an NPC in Timeways talk, a person of Classic Azeroth who speaks to the player. Load before you write or review any NPC sample, talk prompt text, or check of NPC words. Holds the principles, the shape of a reply, the ban list, and the do/don't pairs.
---

# The Timeways NPC voice

The full guide is `docs/plans/npc-voice.md`. This skill is its core. If the two differ, the guide wins. The narrator has its own voice (`timeways-narrator-voice`). Do not mix them: an NPC is a character, not a chronicler.

## Principles

1. A person, not a narrator. The NPC has a job, a place, a problem, and an opinion. It speaks in the first person.
2. Answer first. The first sentence reacts to what the player said. A question gets its answer.
3. Concrete over mood. Name who, where, and what happened, with the names of the lore. Never "trouble lingers".
4. Short. 1 to 4 sentences, at most 40 words. A spoken sentence has at most 25 words, and most have 5 to 15.
5. One ask at most.
6. The people shows in its cares, not in its spelling. At most one word of dialect, and mostly none.
7. Plain words with a little color ("wretches", "the Light"). No old-timey words, no formal hedges.
8. Subtext over explanation. Never tell the player how they feel. Show the NPC's own feeling in what it says.
9. No made-up past and no made-up scene. Only the memories, the trust, and the lore of the prompt.
10. No pep talk at the end. End on a fact, an ask, or a judgment.
11. "I don't know" in character: "Darnassus? Couldn't tell you."

## The shape of a reply

1. The reaction: one short sentence. "Road's bad." / "You again." / "Work?"
2. The fact: one or two sentences of what the NPC saw, lost, or knows, with real names.
3. The want (optional): one ask, one fear, or one judgment.

End on a fact, an ask, or a judgment. After a question of the player, never end on a question. After a greeting, a question of at most 6 words is fine: "Yes, what do you want?" (Executor Zygand).

## Each people

| People | What it cares about |
|---|---|
| Humans of Stormwind | the militia, the law, the Light, the crown that does not help |
| Orcs | duty, honor, the Horde; blunt praise |
| Forsaken | the Dark Lady, contempt for the living, cold glee |
| Dwarves | the cold, the drink, the troggs, pride in their work |
| Gnomes | machines gone wrong, quick asks for help |
| Goblins | deals, profit, fast asides |
| Trolls (Darkspear) | the tribe, the spirits, voodoo; grave and short. Classic trolls almost never say "mon". |
| Tauren | the tribe, the Earth Mother, the plains; calm |
| Night elves | duty, balance, the groves; formal and measured |

## Ban list

The data is `crates/story/data/npc_slop.txt` (127 entries). A phrase that the prompt gave is allowed.

| Group | Examples | Reason |
|---|---|---|
| Stock address | greetings traveler, welcome adventurer, brave soul, weary traveler, hail hero | Generic RPG voice |
| Mood with no fact | dark times, trouble lingers, evil stirs, a darkness, mark my words | Mood in place of what happened |
| Habit filler | I've taken to, I find myself, I must say, I assure you, fear not | The NPC hedges, not the world |
| Pep talk | keep your wits about you, keep your eyes open, stay safe, safe travels, the road ahead | An ending that says nothing |
| Stock question | what brings you, what say you, how can I help, can I count on you | Hands the talk back |
| Old-timey | thee, thou, thy, hath, doth, verily, pray tell | Fake flourish |
| Made-up past or fame | been expecting you, your reputation, word of your deeds | A past that no memory holds |
| The player's feelings | you look like, in your eyes, your heart, I sense | Narrates the player |
| Lecture opener | as you know, legend has it, it is said | Exposition dump. "They say" stays, for a rumor. |
| Stage direction | leans in, chuckles, sighs, anything between `*` | Only spoken words |

Also refused by code: an opening "Ah", more than 60 words, more than 4 sentences, a sentence of more than 25 words, "you" + look/seem/must/are + a feeling ("you look tired", "you must be weary"), and more than 2 words of dialect ("Aye, lad, ye be wantin'").

## Real Classic lines (copy the craft, not the words)

- "Yes, Hogger has been a real pain for me and my men." (Marshal Dughan)
- "All the quilboars are our enemies, <name>. Some just prove to be more of a nuisance than others." (Thork)
- "Humans infest the land like mold on a rotting corpse." (Deathguard Simmer)
- "Like to share a drink with me, perhaps? Not much else to do in the cold." (Senir Whitebeard)
- "Hm... your report comes at a bad time." (Master Gadrin)

## Pairs (don't, then do)

- "Road? Trouble lingers, Blackrock orcs and gnolls still raid our shores. We need brave souls to drive them off." -> "Road's bad. Blackrock orcs hold Stonewatch Keep, and the gnolls come down to the lake at night. Stormwind sends no one."
- "Ah, adventurer! What brings you to Goldshire on this fine day?" -> "Room's two silver. Hogger's gnolls took the Stonefield farm last week, so the road east is yours to risk."
- "I sense a great weariness in you, traveler." -> "Sit if you like. The stew's cold, but the ale isn't."
- "Hrmm, aye, ye be lookin' fer work, lad?" -> "Work? Troggs are tunneling up from every crevice of the Loch."
- "Thou art welcome, child of the Horde. Pray tell, what dost thou seek?" -> "You stand in Thunder Bluff, so you are welcome. Speak plainly."
- "As you know, the Defias Brotherhood was founded by the stonemasons..." -> "The Defias? Stonemasons Stormwind never paid. Now they burn our farms."
- "I've been expecting you, hero." -> "Never seen you before. In Ratchet, that means you pay up front."
- "Dark times, friend. Keep your eyes open and your blade ready." -> "Gnolls took the bridge. If you head north, go by day."
- "*sighs heavily* My heart breaks for my lost wife." -> "My wife went into the Barrens and never came back. The Kolkar took her, I know it."

## Work in a talk

Work becomes a real quest (`WORK_RULE` in `talk.rs`). So a reply that offers work names only the trouble: no place, creature, count, or reward. "Saldean can't bring his harvest in. Something tears up his fields every night."

## Before you hand in a line

1. Does the first sentence answer or react to the player?
2. Is every fact concrete and in the prompt, with nothing made up?
3. Would a person of this people and this trade say it, in these words?
4. Is it 1 to 4 sentences and at most 40 words, with one ask at most?
5. Is it free of every banned phrase, and of a closing question after a question?
6. Run `npc_faults` on it: `cargo test -p timeways-story --test npc_voice`.
