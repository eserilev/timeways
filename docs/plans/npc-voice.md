# Plan: the voice of an NPC

Status: draft 1, 2026-10-07. The prompt block of section 7 and the checks of section 6 are built. The samples of section 8 wait for the approval of the user. `npc_replies.txt` keeps its 3 old samples until then.

## 0. Why this guide exists

The user, 2026-10-07: "The way the NPC talks is kinda cringe, bruv."

The bench showed why. The local model wrote this talk line for Magistrate Solomon, to "Any news from the road?":

> "Road? Trouble lingers, Blackrock orcs and gnolls still raid our shores. We need brave souls to drive them off, but Stormwind's too busy with their own strife. Still, I've taken to standing watch myself, no one's supposed to be left to face the enemy alone."

It has mood in place of a fact ("trouble lingers"), a stock address ("brave souls"), a habit filler ("I've taken to"), and a moral at the end. Claude did better, but it made up scene detail ("a rider came in last night with an arm cut open") and ended on a pep talk ("Keep your eyes open and your blade ready.").

The narrator got its own guide (`narrator-style.md`). That guide does not hold for talk: an NPC is a character who speaks to the player, not a chronicler. This guide is for every NPC sample, the talk prompt, and every agent who writes them. The skill `.claude/skills/timeways-npc-voice/SKILL.md` holds its core.

**Where the guide holds.** The words of an NPC in `/talk`, and the NPC samples. The text of a quest that an NPC gives is in the voice of that NPC too, so the quest prompt can take this guide later. UI copy follows `CLAUDE.md`, and the narrator follows `narrator-style.md`.

## 1. The model: Classic quest text and gossip

The research (appendix A) read the quest text and gossip of Classic NPCs on Wowhead Classic. It found a clear shape:

- **Short.** A progress or a completion line has 1 to 3 sentences and 5 to 25 words: "You have done well, <name>." (Thork). "I understand. Thank you, <class>." (Mankrik). Gossip has 3 to 10 words: "What are you looking for?" (Orgrimmar Grunt). Only the opening text of a quest is longer, 60 to 120 words.
- **Plain syntax, a light touch of age.** The words are modern. A few words add color: "wretches", "insolent", "pledge". Gryan Stoutmantle: "Perhaps I did not make myself clear, pledge."
- **A reaction first, then one fact or one ask.** Marshal Dughan: "Hah! Well done! I was starting to think no one would take down that monster!"
- **Opinion is the voice.** The NPC judges the world in its own words. Deathguard Simmer: "Humans infest the land like mold on a rotting corpse." Thork: "All the quilboars are our enemies, <name>. Some just prove to be more of a nuisance than others."
- **About one image in a whole quest.** Metaphor is rare, and it is concrete: mold, a corpse, cattle.
- **Accent is very light.** Thrall, Master Gadrin, and Nimboya write standard English. Classic trolls almost never say "mon". That habit grew later. The dwarves of Loch Modan show no written brogue. Only a few comic NPCs speak pidgin (Witch Doctor Unbagwa).
- **Each people shows in what it cares about**, not in spelling:

| People | What it cares about | Example |
|---|---|---|
| Humans of Stormwind | the militia, the law, the Light, the crown that does not help | Gryan Stoutmantle: "Unfortunately, the price of peace is often blood." |
| Orcs | duty, honor, the might of the Horde, blunt praise | Thrall: "One thing I will not tolerate are traitors in our midst, <name>." |
| Forsaken | the Dark Lady, contempt for the living, cold glee | Executor Zygand: "Yes, what do you want?" / Faranell: "I'll need many hearts for this delightful experiment!" |
| Dwarves | the cold, the drink, the troggs, pride in their own work | Senir Whitebeard: "Like to share a drink with me, perhaps? Not much else to do in the cold." |
| Gnomes | machines gone wrong, quick asks for help | Tinkmaster Overspark: "Yes it's true. Techbot has gone rogue!" |
| Goblins | deals, profit, fast asides | Wharfmaster Dizzywig: "Speak up! Tell me, are you dropping off or picking up?" |
| Trolls (Darkspear) | the tribe, the spirits, voodoo, grave and short | Master Gadrin: "Hm... your report comes at a bad time." |
| Tauren | the tribe, the Earth Mother, the plains, calm | Innkeeper Adegwa: "As the wind on the plains, you are always welcome here." |
| Night elves | duty, balance, the groves, formal and measured | Conservator Ilthalaine: "There is still work to be done, <name>." |

Classic also says "What brings you to my village, <class>?" (Baine Bloodhoof) and "Welcome to my Inn, weary traveler." (an innkeeper). In Classic each one comes once, in context. A model puts them in every line. So this guide bans them for the model, as section 4 says.

The craft advice of game writers agrees (appendix A.2):

- **Be brief.** Cut everything that does not serve the line. Do not force lore on the player.
- **One goal for each line.** A bark does one job and gives no extra information.
- **Skip the small talk.** Go straight to the substance.
- **Each character has its own voice**: its own rhythm, its own words, its own interests. A town of people who all talk the same is the failure.
- **Subtext.** An NPC keeps back what it does not want to say. It does not explain its own feelings.

## 2. Principles

1. **A person, not a narrator.** The NPC has a job, a place, a problem, and an opinion. It talks about them in the first person.
2. **Answer first.** The first sentence reacts to what the player said. A question gets its answer. A greeting gets a greeting or a demand.
3. **Concrete over mood.** Name who, where, and what happened, with the names of the lore. Never "trouble lingers" or "dark times".
4. **Short.** 1 to 4 sentences, at most 40 words. A spoken sentence has at most 25 words. Most have 5 to 15.
5. **One ask at most.** An NPC wants one thing from the player, or nothing.
6. **The people shows in the cares, not in the spelling.** An orc speaks of honor and the Horde, a Forsaken of the Dark Lady and the living. At most one word of dialect, and mostly none.
7. **Plain words with a little color.** Everyday words. One word of the setting is fine ("wretches", "the Light"). No old-timey words ("thee", "hath"), no formal hedges ("I must say", "I assure you").
8. **Subtext over explanation.** The NPC never tells the player how the player feels, and seldom says how it feels itself. It shows it in what it says: a curt line for distrust, a joke for trust.
9. **No made-up past and no made-up scene.** The NPC speaks only of the memories, the trust, and the lore of the prompt. It invents no wound, no rider who came last night, and no fame of the player.
10. **No pep talk at the end.** The line ends on a fact, an ask, or a judgment. Never on "Keep your eyes open", "Stay safe", or a question that hands the talk back.
11. **"I don't know" in character.** "Darnassus? Couldn't tell you." Never a made-up answer (`npc-knowledge.md` 7.3).

## 3. The shape of a reply

| Part | Rule | Example |
|---|---|---|
| 1. The reaction | One short sentence that answers the player or reacts to them. Never "Ah". | "Road's bad." / "You again." / "Darnassus? Couldn't tell you." |
| 2. The fact | One or two sentences of what the NPC saw, lost, or knows, with real names. | "The Blackrock took Stonewatch Keep, and the gnolls hold the Redridge road." |
| 3. The want (optional) | One ask, one fear, or one judgment. | "Stormwind won't send a single soldier." |
| The end | A fact, an ask, or a judgment. | not "Will you help?", not "Stay safe." |

Limits:

| Rule | Limit | Why |
|---|---|---|
| Words in a reply | at most 40 asked, 60 checked | Classic progress lines have 5 to 25. A talk answers a typed line, so it gets a little more. |
| Sentences in a reply | 1 to 4 | Gossip is 1 or 2. 4 is room for reaction, fact, and want. |
| Words in a sentence | at most 25 | People speak in short sentences. Claude wrote a sentence of 35 words, and it read as a run-on. |
| Asks | at most 1 | Section 1, "one goal for each line". |
| Dialect words | at most 1 asked, 2 checked | Classic accent is very light. |
| A closing question | none after a question of the player. After a greeting, one of at most 6 words. | "Yes, what do you want?" is Classic. "What brings you to these lands, brave soul?" is slop. |

## 4. The ban list

The words live in `crates/story/data/npc_slop.txt`. The check matches whole words, in any case, within one clause. A phrase that the prompt gave is allowed: a line of the lore, or the words of the player. The general list of `banned_words.txt` (modern slang, "testament to", "delve") holds for an NPC too. The narrator list `slop_words.txt` does not: rain and dusk are fine in the mouth of a farmer.

| Group | Entries (examples) | Reason |
|---|---|---|
| Stock address | greetings traveler, welcome adventurer, brave soul, brave souls, weary traveler, kind stranger, hail hero | A generic RPG voice. The player is a person in front of the NPC. |
| Mood with no fact | dark times, troubled times, trouble lingers, trouble brews, evil stirs, a darkness, the winds of, mark my words, make no mistake | Mood in place of what happened. The bench line opened with "Trouble lingers". |
| A habit filler | i've taken to, i find myself, i must say, i must admit, i assure you, rest assured, fear not | The NPC talks about its own habits and hedges, not the world. The bench line had "I've taken to standing watch". |
| A pep talk at the end | keep your wits about you, keep your eyes open, keep your blade ready, watch your back, stay safe, safe travels, tread carefully, stay vigilant, the road ahead, your journey | An ending that says nothing. Claude ended on "Keep your eyes open and your blade ready." |
| A stock question | what brings you, what say you, how can i help, what can i do for you, can i count on you, if you dare, should you choose to | It hands the talk back to the player and says nothing. |
| Old-timey words | thee, thou, thy, thine, hath, doth, verily, forsooth, pray tell, methinks | Fake old-timey flourish. Classic text is modern. |
| Made-up past or fame | been expecting you, i knew you would come, your reputation, word of your deeds, tales of your, your legend | A past that no memory holds, and a fame claim. The narrator guide bans fame too. |
| The player's looks and feelings | you look like, you have the look of, in your eyes, your heart, your soul, i sense, i can tell you're | The NPC tells the player how they feel. A pattern of `npc_voice.rs` catches the rest (section 6). |
| A lecture opener | as you know, as you may know, legend has it, legend tells, it is said, since time immemorial | An exposition dump. An NPC answers, it does not read from a book. "They say" stays: it marks a rumor (`npc-knowledge.md`). |
| A stage direction | leans in, leans closer, lowers his voice, chuckles, sighs | The words of an NPC are only what it says. |

That is 10 groups and 127 entries. The data file holds all of them.

## 5. Do and don't

The "do" lines are examples for the guide, not samples. A sample needs the approval of the user (section 8).

| # | Don't | Do | Rule |
|---|---|---|---|
| 1 | "Road? Trouble lingers, Blackrock orcs and gnolls still raid our shores. We need brave souls to drive them off." | "Road's bad. Blackrock orcs hold Stonewatch Keep, and the gnolls come down to the lake at night. Stormwind sends no one." | 3, ban list |
| 2 | "Ah, adventurer! What brings you to Goldshire on this fine day?" | "Room's two silver. Hogger's gnolls took the Stonefield farm last week, so the road east is yours to risk." | 2, 10 |
| 3 | "I sense a great weariness in you, traveler. Rest your bones by the fire." | "Sit if you like. The stew's cold, but the ale isn't." | 8 |
| 4 | "Greetings, brave soul. The Dark Lady's work is never done in these troubled times." | "The Dark Lady wants the Scarlet Crusade gone from Tirisfal. Kill a few of them, and the Deathguard will learn your face." | 3, 6 |
| 5 | "Hrmm, aye, ye be lookin' fer work, lad? Och, the troggs be everywhere, ye ken." | "Work? Troggs are tunneling up from every crevice of the Loch. Mountaineer Cobbleflint wants every axe he can get." | 6: no caricature |
| 6 | "Thou art welcome, child of the Horde. Pray tell, what dost thou seek?" | "You stand in Thunder Bluff, so you are welcome. Speak plainly. The elders have no patience for riddles." | 7 |
| 7 | "As you know, the Defias Brotherhood was founded by the stonemasons who rebuilt Stormwind after the Second War, when the nobles refused to pay them..." | "The Defias? Stonemasons Stormwind never paid. Now they burn our farms." | 4, 5: no lecture |
| 8 | "I've been expecting you, hero. Word of your deeds has reached even Ratchet." | "Never seen you before. In Ratchet, that means you pay up front." | 9 |
| 9 | "Dark times, friend. Keep your eyes open and your blade ready." | "Gnolls took the bridge. If you head north, go by day." | 10 |
| 10 | "Ironforge is a city of dwarves, known for its Great Forge and its bustling halls of trade." (an Elwynn farmer) | "Ironforge? Never been past the mountains. Ask at the inn." | 11 |
| 11 | "*sighs heavily* My heart breaks for my lost wife." | "My wife went into the Barrens and never came back. The Kolkar took her, I know it." | 8, ban list |
| 12 | "You look tired. Will you help us?" | "The raptors at the Lushwater ate another caravan. The Crossroads needs every blade." | 5, 8, 10 |

Real Classic lines that show the same rules (sources in appendix A.1):

- Answer first, then one fact: "Yes, Hogger has been a real pain for me and my men." (Marshal Dughan)
- The people in the cares: "You are proving yourself to be quite an asset to The Dark Lady's army." (Apothecary Johaan)
- Short and curt for a cold NPC: "Yes, what do you want?" (Executor Zygand)
- Feeling shown in a plain fact: "The pain in my chest tells me that the worst has happened, but I have hope you will find her safe and sound." (Mankrik)
- A judgment as the end: "Some just prove to be more of a nuisance than others." (Thork)

## 6. What a check can enforce

### 6.1 Deterministic checks (built)

The checks live in `crates/story/src/npc_voice.rs` and run in `talk::checked_answer`, after the checks that held before: one line, at most 400 characters, no emoji, no banned word, no copy of a sample, no name after the cutoff, and no name that the prompt did not give (`grounding.rs`). An answer that fails any check gets one retry with the reasons (`talk::answer_or_reasons`), when a slot is free and the retry fits the budget. A second failure gives the line "looks at you and says nothing".

| Rule | Pattern | Fault |
|---|---|---|
| Ban list | each phrase of `npc_slop.txt`, whole words, in one clause, unless the prompt gave it | `Slop` |
| No "Ah" opener | the first word is "ah", "ahh", "ahhh", or "aah" | `StockOpener` |
| No closing question | the last sentence ends with `?`, and the player asked something, or the question has more than 6 words | `ClosingQuestion` |
| Words | more than 60 words | `TooManyWords` |
| Sentences | more than 4 sentences | `TooManySentences` |
| A long sentence | a sentence of more than 25 words | `LongSentence` |
| The player's feelings | "you", then a word of seeming (look, seem, sound, feel, must, are, 're), then a feeling (tired, weary, lost, troubled, brave, ...) within 3 words. A "you" after "if", "when", "are", or "do" asks or supposes, so it passes. | `PlayerFeeling` |
| A stage direction | a `*` in the words | `StageDirection` |
| A caricature | more than 2 words of dialect ("mon", "lad", "aye", "ye", "yer", ...) or words that drop their "g" ("nothin'") | `Caricature` |

The player asked something (`Asked::Question`) when the words hold a `?`, or open with a question word such as "any", "where", or "tell". Players often type with no question mark: "any news".

### 6.2 What only the guide can enforce

- The reply answers what the player said.
- The NPC sounds like its people and its trade.
- The facts are true to the lore of the prompt, and no scene detail is made up.
- Subtext: a feeling shows in the words, not in a name for it.
- One ask at most.
- An image, a moral, or a pep talk that the ban list does not name.

## 7. How it plugs into `talk.rs`

The talk prompt gets a compact manner block after the samples, before the words of the player. It costs about 150 tokens. "An NPC talk with everything" goes from 1658 to 1805 of its budget of 1823 tokens, and the budget test of `tests/voice.rs` measures it. A longer block needs a cut elsewhere first.

```text
Your manner:
- Talk like a person of your people and trade: everyday words, short sentences.
- Answer the player first. Then one thing you saw, want, or fear. One ask at most.
- Be concrete: who, where, what happened, with the names of the lore. No speeches.
- Show your people in what you care about, not in spelling.
- Never open with "Ah". Never call the player "adventurer" or "brave soul". Never say how the player feels.
- No "dark times", "trouble lingers", "I've taken to", or old words like "thee".
- When the player asked something, end on your answer, not a question.
```

The last note of the prompt changes from "Speak plainly, in your own voice, in at most 60 words." to "Answer first, in 1 to 4 short sentences and at most 40 words."

The persona (`talk::persona`) does not change, because the quest prompt shares it, and the quest offer is close to its budget.

The samples teach the voice more than the rules do. Section 8 holds the new samples for approval.

## 8. Draft samples, for the approval of the user

Each sample passes every check of section 6. They cover humans, orcs, Forsaken, dwarves, gnomes, goblins, trolls, tauren, and night elves, and the moods friendly, wary, hostile, a quest offer, and "I don't know". The quest offer follows `WORK_RULE`: it names the trouble, and no creature, count, or reward.

1. (human farmer, Westfall, a quest offer) "Saldean can't bring his harvest in. Something tears up his fields every night, and the Stormwind guard won't ride this far west anymore. If you can stop it, I'd owe you."
2. (orc grunt, the Crossroads, wary) "You're not one of Thork's scouts. Keep your hands where I can see them. The quilboar hit the walls twice this week, and I don't trust faces I don't know."
3. (Forsaken apothecary, Brill, cold) "The living always ask about the smell. It's the new plague, and it is working. The Dark Lady pays me for results, not for chatter."
4. (dwarf, Thelsamar, friendly) "Quiet night, and I'll take it. The troggs came up by the dam again, and two of Cobbleflint's lads have cracked heads. Sit down and have a stout. It's the one thing in Loch Modan that never lets me down."
5. (gnome, Tinker Town, friendly) "Gnomeregan? I left with my boots and one spanner. The troggs have the lower decks, and the radiation has the rest. Tinkmaster Overspark says we'll take it back, and most days I believe him."
6. (goblin, Ratchet, a question about a boat) "Information costs, pal. The boat to Booty Bay leaves when the captain sobers up. Who's on it and what's in the crates, that's another price."
7. (troll, Sen'jin Village, grave) "Zalazane still holds the Echo Isles. He made puppets of our own people, and Vol'jin led the rest of us to this shore. We will go back. Not yet."
8. (tauren, Mulgore, calm) "The Bael'dun dwarves dig at the edge of our land again. Cairne asks us to watch and wait, so we wait. The Earth Mother does not like what they pull out of her."
9. (night elf sentinel, Astranaar, wary) "The orcs cut another grove at the Warsong camp this morning. Astranaar will answer for it. You may use the road, but stay off the shrines."
10. (human innkeeper, Goldshire, "I don't know") "Darnassus? Couldn't tell you. I've never been across the sea, and the elves who stop here don't say much."
11. (human guard, Goldshire, hostile after slaps) "Touch me again and you'll spend the night in the Stockade. I mean it. Marshal Dughan has a long memory and a short temper."

## 9. Tests

`crates/story/tests/npc_voice.rs` holds a refused test and a near-miss test for each check:

- `every_npc_sample_passes_the_npc_checks`
- `the_bench_lines_that_the_user_found_cringe_are_refused`
- `a_ban_phrase_is_refused_and_a_phrase_of_the_prompt_is_allowed`, `a_ban_phrase_counts_only_within_one_clause`
- `a_stock_form_of_address_is_refused_and_a_plain_mention_passes`
- `an_answer_that_opens_with_ah_is_refused`, `a_word_that_only_starts_with_ah_opens_no_stock_reply`
- `an_answer_to_a_question_never_ends_on_a_question`, `a_question_inside_an_answer_passes`, `a_greeting_gets_a_short_question_back_and_never_a_long_one`
- `an_answer_over_the_word_limit_is_refused_and_one_at_the_limit_passes`, `an_answer_of_five_sentences_is_refused_and_one_of_four_passes`, `a_long_spoken_sentence_is_refused_and_one_at_the_limit_passes`
- `an_npc_never_tells_the_player_how_they_look_or_feel`, `a_you_that_asks_supposes_or_names_tells_no_feeling`
- `a_stage_direction_is_refused_and_plain_words_pass`
- `a_heavy_accent_is_refused_and_a_light_one_passes`
- `a_talk_prompt_carries_the_manner_of_an_npc`, `a_talk_answer_with_a_closing_question_to_a_question_is_dropped`

The fuzz target `answers` runs `talk::checked_answer`, so it runs the new checks too.

## Appendix A. Research sources

WebFetch gives the pages through a summarizer. Check a quote on its page before you put it in a sample.

### A.1 Classic quest text and gossip

| Source | Example |
|---|---|
| [The People's Militia](https://www.wowhead.com/classic/quest=12) | Gryan Stoutmantle: "The People's Militia has but one goal: To defend the lands of Westfall and return peace to our surroundings. Unfortunately, the price of peace is often blood." |
| [The Defias Brotherhood](https://www.wowhead.com/classic/quest=65) | Gryan Stoutmantle: "The band of wretches responsible for driving the good people of Westfall from the land call themselves The Defias Brotherhood." |
| [Wanted: Hogger](https://www.wowhead.com/classic/quest=176) | Marshal Dughan: "Yes, Hogger has been a real pain for me and my men." / "Hah! Well done! I was starting to think no one would take down that monster!" |
| [quest 5726](https://www.wowhead.com/classic/quest=5726) | Thrall: "One thing I will not tolerate are traitors in our midst, <name>." |
| [quest 871](https://www.wowhead.com/classic/quest=871) | Thork: "All the quilboars are our enemies, <name>. Some just prove to be more of a nuisance than others." |
| [Lost in Battle](https://www.wowhead.com/classic/quest=4921) | Mankrik: "I understand. Thank you, <class>." |
| [quest 365](https://www.wowhead.com/classic/quest=365) | Deathguard Simmer: "Humans infest the land like mold on a rotting corpse." Apothecary Johaan: "You are proving yourself to be quite an asset to The Dark Lady's army." |
| [quest 383](https://www.wowhead.com/classic/quest=383) | Executor Zygand: "Yes, what do you want?" |
| [quest 1113](https://www.wowhead.com/classic/quest=1113) | Master Apothecary Faranell: "I'll need many hearts for this delightful experiment!" |
| [quest 420](https://www.wowhead.com/classic/quest=420) | Senir Whitebeard: "Like to share a drink with me, perhaps? Not much else to do in the cold." |
| [quest 224](https://www.wowhead.com/classic/quest=224) | Mountaineer Cobbleflint: "Troggs are tunneling up from every crevice!" |
| [quest 2923](https://www.wowhead.com/classic/quest=2923) | Tinkmaster Overspark: "Yes it's true. Techbot has gone rogue! Please, can you help me?" |
| [quest 1111](https://www.wowhead.com/classic/quest=1111) | Wharfmaster Dizzywig: "Speak up! Tell me, are you dropping off or picking up?" |
| [quest 805](https://www.wowhead.com/classic/quest=805) | Master Gadrin: "Hm... your report comes at a bad time." |
| [quest 349](https://www.wowhead.com/classic/quest=349) | Witch Doctor Unbagwa: "Witch Doctor Unbagwa like Gorilla Fangs!" The one strong dialect found. |
| [quest 763](https://www.wowhead.com/classic/quest=763) | Baine Bloodhoof: "What brings you to my village, <class>?" |
| [quest 456](https://www.wowhead.com/classic/quest=456) | Conservator Ilthalaine: "There is still work to be done, <name>." |
| [Orgrimmar Grunt](https://warcraft.wiki.gg/wiki/Orgrimmar_Grunt) | Gossip: "What are you looking for?" |
| [Innkeeper Adegwa](https://warcraft.wiki.gg/wiki/Innkeeper_Adegwa) | "As the wind on the plains, you are always welcome here." |
| [Sen'jin Village (quest)](https://warcraft.wiki.gg/wiki/Sen'jin_Village_(quest)) | "Ya come highly recommended, mon." This is Cataclysm, not Classic. Classic trolls almost never say "mon". |

### A.2 Craft of game dialogue

| Source | Lesson |
|---|---|
| [8 key principles of writing effective game dialogue](https://www.gamedeveloper.com/game-platforms/8-key-principles-of-writing-effective-game-dialogue) | Concision is the most important quality. Do not force lore on the player. No flowery or archaic speech. |
| [Adding life to worlds with dialogue barks](https://www.gamedeveloper.com/design/adding-life-to-worlds-with-dialogue-barks) | One goal for each bark, no information overload. Each NPC has its own personality, or the town talks the same. |
| [Worldbuilding with NPC dialogue](https://www.gamedeveloper.com/design/worldbuilding-with-npc-dialogue-a-beginner-s-guide) | Skip the small talk. Leave some mystery. |
| [Chris Avellone on characterization](https://forums.obsidian.net/blogs/entry/168-project-eternity-and-characterization/) | Give each character its cadence, its slang, and its interests. Carry the theme without hammering it home. |
| [Emily Short, conversation](https://emshort.blog/how-to-play/writing-if/my-articles/conversation/) | Subtext: an NPC holds back what it does not want to discuss. |

### A.3 Slop markers

| Source | Lesson |
|---|---|
| [sam-paech/antislop-sampler](https://github.com/sam-paech/antislop-sampler) | "testament to", "tapestry", "barely above a whisper", "shivers down", "eyes glinted", "palpable". |
| [Wikipedia: Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing) | Puffery, "serves as", "not just X, but Y". |
| [ossa-ma, AI writing tropes](https://gist.github.com/ossa-ma/f3baa9d25154c33095e22272c631f5a1) | The rule of three, repeated openings, "magic" adverbs. |
| [Character.AI overused phrases](https://www.roborhythms.com/character-ai-overused-phrases/) | Roleplay tics: "a pang of", "smirk", "chuckle". |
| [LLM NPCs drift to one style (arXiv 2402.18659)](https://arxiv.org/pdf/2402.18659) | Different NPCs fall into the same phrases and lose their voice. |

No source names "Ah, adventurer", "brave soul", "these are dark times", or a closing question. These come from the bench of this project and from the user.
