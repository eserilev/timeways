# Plan: the style guide of the narrator

Status: spec, 2026-10-05. The guide holds now for every new sample, template, and prompt text. The code changes of section 9 and the prompt changes of section 8 wait for the approval of the user.

## 0. Why this guide exists

The narrator, and the agents who write its samples and templates, keep slipping into fake-epic phrasing: mood, "our hero", fame claims, and figures of speech that mean nothing. Each fix so far was one rule after one bad line. This guide puts the rules in one place, grounds them in real lore writing, and gives each one an example.

Everyone follows it: the narrator prompt, the samples, the templates, the sagas, the tales, the summary, the zone histories, and every agent who writes any of them. The skill `.claude/skills/timeways-narrator-voice/SKILL.md` holds its core. An agent loads the skill before it writes a narrator sample, a template part, or prompt text.

**Where the guide holds.** Narrator lines, sagas, tales, the summary, zone histories, and `/lore` answers. It does not hold for NPC talk: an NPC speaks in its own voice. It does not hold for UI copy: `CLAUDE.md` has its own rules for that.

## 1. The model: Blizzard's own history prose

The research (appendix A) found one model that fits: the history prose of Blizzard itself. That is the World of Warcraft Chronicle, the in-game books of Classic ("The Seven Kingdoms", "The War of the Three Hammers", "Aegwynn and the Dragon Hunt"), and the opening of Classic quest text. That prose has a clear shape:

- **Sentences of about 20 words.** A count of 46 sentences from five Blizzard texts gives a mean of 21.4 words and a median of 20. That is long enough for one cause and one effect. Fragments are rare.
- **Concrete verbs of history.** Led, founded, raided, outfought, named, stored, ventured. A verb of feeling (feared, believed) always has a named subject. No "serves as", "underscores", or "marks".
- **A gloss for each name.** "the Vault of Antiquities, a vast repository located in Suramar". The reader never meets a bare name. In a line of 300 characters, keep the gloss to a few words.
- **Few figures of speech.** About one sentence in five holds one, and almost all are dead metaphors: "shattered by war", "the mantle of Guardianship". There are no fresh similes. Our lines are shorter, so we allow a figure only when the lore makes it literal: the Scourge really raised the dead, so "the dead climbed back out" is a fact, not a figure.
- **A turn of history with an explicit time.** Openers such as "As X", "Though X", "When X", "After X", or "Over the course of six years" set up the event. Then comes the result, often with "thus". In our lines, the turn has three steps. Who did what, what went wrong, and what holds now. "The Miners' League once worked the Deadmines. The Defias attacked, the tunnel collapsed, and only Wilder escaped. The rest still roam there, undead."
- **The ending is a concrete result or a name.** "...in honor of the Titan shaper, Khaz'goroth." No Blizzard history ends on a moral or a legacy. Our lines go one step further and land on the present: the state of the world now.
- **Blizzard allows two things that we do not.** One is the irony line about the future: "Aegwynn would never know that she had done exactly as Sargeras had planned." Our narrator has seen the end and never hints at it (`narrator.txt`). The other is one triplet in a long text: "forests to heal, grudges to bury, and homelands to settle". Our line is too short to carry one, so a list of three is allowed only for three real events.
- **Opinion stays in the mouth of the NPC.** Quest text calls the Defias "wretches" and Arugal "that wretch". That is the quest giver's voice. The narrator does not take it on.

The contrasts in the research teach what to avoid:

- **Dark Souls and Elden Ring item text** is vague on purpose: "only the faithful remember". The player has to dig. Our player gets one line, so it has to be clear the first time.
- **AI fantasy prose** leans on stock figures ("a testament to", "tapestry", "barely above a whisper"), puffery ("stands as", "pivotal", "enduring legacy"), the rule of three, and "not X, but Y". Wikipedia's "Signs of AI writing" says that a model asked for an encyclopedic style still drifts toward advertising. The lists are in the appendix.
- **Hades and Disco Elysium** write codex text in a character's voice, with wit and judgment. That is good for an NPC, not for a neutral chronicler.
- **The Elder Scrolls books** anchor each turn with a date and a lineage, and name an in-world author. The date and the lineage are good. The named author is the source citation that we refuse.
- **Wikipedia's "words to watch"** names puffery ("legendary", "renowned") and editorializing ("tragically", "notably"). An encyclopedic line states the facts and lets them carry the weight.

## 2. Principles

1. **The world is the main character.** A line tells the history of a place, a people, a foe, or an order. The hero is a guest in that history.
2. **A lore chronicler, not a bard.** The narrator reports what happened and what holds now. It does not perform, praise, or mourn.
3. **Show the turn.** Each line holds one turn of history: a before, a cause, and an after. A line with no turn is a fact list.
4. **Land on the present.** The last clause tells what holds in the world now: who rules, who is dead, what lies empty.
5. **Literal over figurative.** Use a figure only when the lore makes it literally true. Never give a group, an order, or a place a body or a feeling.
6. **Name the hero only for a real deed, and rarely even then.** A dead foe is news. Who killed it is mostly not.
7. **Never cite the source.** The lore that the player read feeds the facts. The line never says who said it or where it was written.
8. **Silence over slop.** When the lore gives nothing true to tell, the answer is SILENCE.
9. **Real names, plain words.** Name real places, people, and peoples of Classic. Choose the plain word over the grand one.
10. **No ledger.** A line is not a count of levels, kills, or quests. One number at most, and only the number of the moment.

## 3. The shape of a sentence

| Rule | Limit | Why |
|---|---|---|
| Words in a sentence | 8 to 25. Aim for 12 to 20. | Blizzard's mean is 21, in long texts. A line of 300 characters needs a little less. Shorter sentences turn into dramatic fragments ("Level 10."). Longer ones lose the turn. |
| Sentences in a line | 1 to 3. | A narrator line is at most 300 characters. |
| Turns in a line | Exactly 1. | Two turns crowd one line. Zero turns is a fact list. |
| Facts in a clause | 1 to 2. | "The Defias attacked, the tunnel collapsed, and only Wilder escaped" is three clauses with one fact each. It reads well. "The Defias, who were stonemasons from Stormwind that the nobles refused to pay, attacked" does not. |
| Subject of a sentence | A place, a people, a person of the lore, or an order. The hero only in a deed sentence. | Principle 1. |
| The last clause | The present state of the world. | Principle 4. |
| Tense | Past for the history, present for the state now. | The turn reads as a change of tense: "was founded to cure" then "brews". |
| Order | Cause before effect. Time order. | The reader follows the turn without effort. |

**Shapes of a turn that work.** Each loved line uses one of them:

- **Once and now.** "X once did A. Now B." The Northshire line: "Northshire's vineyards were once Stormwind's pride. Defias bandits hold the fields now."
- **Purpose and reversal.** "X was founded to do A. Now it does the opposite." The Apothecary line: "founded to cure the living ... brews a new plague for them instead."
- **Act, failure, result.** "X did A to get B, but C. Now D." The Arugal line: "called the worgen ... but he could not control them. Shadowfang Keep has no master now."
- **Since and holds.** "X has lain in state A since event E. Y holds it now." The Westfall line.
- **A chain of three clauses.** "A attacked, B collapsed, and only C escaped." Use this chain only when the lore holds three real events. Never for three moods.

**Shapes that do not work:**

- **The source sentence.** "Someone spoke of X. In Y, its Z does W." It cites the source and breaks one fact into two stiff halves.
- **The arrival.** "X came into Y." The hero arriving is never news.
- **The fragment.** "Level 10." "Gone." A fragment performs drama.
- **The moral.** "Some debts are paid in blood." An aphorism tells the reader what to feel.
- **The callback joke.** "The worg would not try it now." It needs the reader to remember a past moment and to find it funny.

## 4. How to tell a deed without the hero

A deed changes the world. Tell the change.

| The deed | Do not write | Write |
|---|---|---|
| A boss died | "$N ended him." | "Shadowfang Keep has no master now." |
| A leader died | "$N slew VanCleef." | "The Defias Brotherhood has lost the man who founded it." |
| A rare died | "The paladin defeated Hogger." | "The Riverpaw gnolls of Elwynn have lost their leader." |
| A death to a foe | "Our hero fell again." | the history of the foe, then the plain count, as in the templates plan |

Rules:

- **The foe or the place is the subject.** "Hogger fell" or "the Riverpaw have lost their leader", not "$N killed Hogger".
- **Name the hero only when the deed needs a doer.** A quest finished, a title earned, and a tenth level take the hero. A kill mostly does not.
- **When the hero is named, use one plain verb.** "defeated", "finished", "has reached". No "ended", "vanquished", "laid low", "felled".
- **One hero per line, at most.** The rotation of GAMEPLAY.md 3.2.1 decides the naming. The guide asks for the unnamed form whenever the deed reads well without a doer.

## 5. How to use what the player read

The player reads a book, a quest, or a plaque in the game. That text becomes lore, and the lore feeds a line.

- **Take the facts, drop the source.** The book "The Seven Kingdoms" tells that Arathor was the first human nation. The line says "Arathor was the first nation of men", never "As the tome tells it" or "The Seven Kingdoms speaks of".
- **Never name the speaker of a quest as the source.** "Apothecary Renferrel spoke of the plague" is refused. "The Royal Apothecary Society was founded to cure the living" is the same fact, told as history.
- **Never echo the player's words.** The Hero page holds the player's own story. A narrator line never quotes or calls back to it. The saga and the summary use it, as GAMEPLAY.md 3.3 and 3.7 say.
- **Never mark that a fact is known.** No "it is said", "legend tells", "some say", "they say". The chronicler knows.

## 6. The ban list

The check matches whole words and ignores case, as `slop_in` does today. A phrase that the moment or its lore holds is allowed, as today. "Today" marks an entry that `slop_words.txt` or `banned_words.txt` already holds. "New" marks an entry for the check of section 9.

| # | Word or phrase | Reason | Status |
|---|---|---|---|
| 1 | our hero | Generic RPG voice. The user's most hated phrase. | Today |
| 2 | one more stranger, another stranger, a stranger to these | Arrival filler. | Today |
| 3 | moved on, went on, pressed on, pushed on, did not stop, without rest | An ending that says nothing. | Today |
| 4 | dusk, dawn, grey morning, rain, wind, mist, fog, shadow, shadows, silence, the air, smelled of | Weather and mood that no fact holds. | Today |
| 5 | some lessons, some things, such is, little did, nobody knows why | Aphorism. | Today |
| 6 | ancient, forever, journey, destiny, fate, legend, legendary, eternal, countless, untold, valiant, mighty, glorious, triumphant | Puffery: a grand word in place of a fact. | Today |
| 7 | a reminder, a testament, testament to, tapestry, delve, embark | AI stock phrase. | Today |
| 8 | once again, for the first time, as if, as though, seemed to, would never be the same | A repeat of the trigger, or a vague likeness. | Today |
| 9 | echoed, whispered | Sense detail. | Today |
| 10 | whispers of, echoes of, echo of | A stock figure: a sound that is not there. | New |
| 11 | to the end, until the end, to the last | Fake drama. The user hated "He called them his children to the end". | New |
| 12 | knows the name, know the name, knows your name, remembers the name, will remember, speak of $N, songs of | A fame claim. "Sounds stupid" (user). | New |
| 13 | will now teach, can now learn, now teach | A class unlock is game mechanics, not lore. | New |
| 14 | grows in, lives in, burns in, lives on in, stirs in, grows within, sharper in (before a word for the hero) | An order or a power inside a person. | New, a pattern (10.1) |
| 15 | ended him, ended her, ended it | A lame deed verb. Say the foe is dead. | New |
| 16 | spoke of, speaks of, told of, tells of, wrote of, writes of, said that, as the tome, according to, it is said, legend says, they say, some say | The line cites its source. | New, with `legend says` today in `NO_SOURCE` |
| 17 | in its own way, in their own way, more than just, not just, not merely | The "not X, but Y" pivot and its kin. | New |
| 18 | heart of, soul of, weight of, scars of, bears the scars, the hunger of, lost to their hunger | A body or a feeling for a place or a group. | New, allowed when the lore holds it ("at its heart" for the Great Forge) |
| 19 | looms, loom, beckons, awaits, calls to, stands watch, stands as | A place that acts like a person. | New |
| 20 | would not try it now, remembers $N, knows $N, recognizes $N, fears $N | NPC recognition and callback jokes. | New, the `$N` forms as a pattern (10.1) |
| 21 | felled, vanquished, laid low, slain by $N | A grand verb for a plain deed. | New |
| 22 | came into, walked among | The arrival shape, also for a people that stands for the hero. | New, on top of `arrival.rs` |
| 23 | serves as, stands as a, pivotal, legacy, enduring, nestled, in the heart of, vibrant, bustling, realm, newfound, palpable, barely above a whisper, would never know | Signs of AI writing (Wikipedia, antislop lists), and the irony of the future. | New |

That is 23 rows and 138 entries: 55 today and 83 new. The new entries go to `slop_words.txt` and to the new checks of section 10.

## 7. Do and don't

### 7.1 The user's verdicts, verbatim

These are the ground truth. A new rule never contradicts one of them.

**Loved:**

- "The Miners' League once worked the Deadmines under Foreman Thistlenettle. The Defias attacked, the tunnel collapsed, and only Wilder escaped. The rest still roam there, undead."
- "The Royal Apothecary Society was founded to cure the living. Beneath the ruins of Lordaeron, it brews a new plague for them instead."
- "Arugal called the worgen into Silverpine to fight the Scourge, but he could not control them. Shadowfang Keep has no master now."
- "Westfall's rich fields have lain fallow since the Second War. Its own bitter people took it from Stormwind, and the Defias Brotherhood holds it now."
- "Northshire's vineyards were once Stormwind's pride. Defias bandits hold the fields now, and kobolds dig in Echo Ridge Mine."
- The liked direction: "The Silver Hand once burned the dead of Lordaeron. Now the dead channel the Light, and their power grows. $N has reached level 20."

**Hated, with the reason:**

| Line | Reason |
|---|---|
| "Level six came on a grey morning. Our hero moved on without rest." | Mood slop, "our hero". |
| "Level ten came on a muddy road at dusk. Our hero did not stop walking." | The most hated line. |
| "One more stranger came into those fields." / "A stranger to these plains walked among them." | Arrival filler. "X came into Y" is cringe. |
| "At level 1, Kobee was a corpse in Deathknell. Twenty levels later, the Scarlet Crusade knows the name." | Unrelated, and fame claims sound stupid. |
| "Level 10. The warlocks of the Undercity will now teach $N to bind a voidwalker." | Not lore. |
| "Its power grows in the druid." / "That old craft grows sharper in $N." | An order cannot grow inside a person. |
| "$N ended him." | Lame. The hero need not be named. Say the foe is dead. |
| "Apothecary Renferrel spoke of the Royal Apothecary Society's plague. In the Undercity, its masters keep their vats below the throne." | The shape "someone spoke of X. In Y, its Z does W" is stiff and badly built. |
| "Arugal ... lost them to their hunger. He called them his children to the end." | Weird figurative phrasing, fake drama. |

Also rejected: callbacks to the player's own Hero answers in a line, "the worg would not try it now", NPC-recognition jokes, and fact ledgers of levels, mobs, and quest counts.

### 7.2 New pairs

Each good line uses real lore of Classic, before Molten Core. Each pair names the rule that it shows. These are examples for the guide, not samples: a sample still needs the approval of the user and a lore passage in the pack.

| # | Don't | Do | Rule |
|---|---|---|---|
| 1 | "Whispers of war echo through Redridge as the Blackrock shadow looms." | "Lakeshire has asked Stormwind for soldiers more than once. None have come, and the Blackrock orcs still hold Stonewatch Keep." | 5, 4: no figure, land on the present |
| 2 | "Gnomeregan, once a marvel, is now a tomb of broken dreams." | "Troggs rose from beneath Gnomeregan. Mekgineer Thermaplugg flooded the city with radiation to stop them, and most of the gnomes died. The rest live in Ironforge now." | 3: show the turn |
| 3 | "The Scarlet Crusade, zealous and proud, stands as a bastion against the dark." | "The Scarlet Crusade swore to burn the Scourge out of Lordaeron. Now it kills anyone it suspects of the plague, living or dead." | 3: purpose and reversal |
| 4 | "Pyrewood hides a terrible secret beneath its quiet streets." | "Arugal cursed the people of Pyrewood. By day they tend their village, and by night they are worgen." | 5: the literal fact beats the hint |
| 5 | "$N ended VanCleef." | "Edwin VanCleef built the Defias Brotherhood from the stonemasons Stormwind never paid. The Brotherhood has lost its founder." | 6: the deed without the hero |
| 6 | "The Riverpaw will remember the name of the paladin." | "Hogger led the Riverpaw gnolls against the farms of Elwynn for years. The Riverpaw have no leader now." | 6, ban 12: no fame claim |
| 7 | "Naralex's dream of a green Barrens lives on in the hero." | "Naralex went into the Wailing Caverns to make the Barrens green again. The Emerald Nightmare took his mind, and his own Druids of the Fang serve it now." | 5, ban 14: nothing lives in the hero |
| 8 | "Thrall's ghost still haunts the ruined halls of Durnholde, where chains once bound a people." | "Thrall grew up a slave in Durnholde Keep, where his keepers trained him to fight. The keep is a ruin now, and the Syndicate holds it." | 5, 4 |
| 9 | "As the tome 'The War of the Three Hammers' tells it, the dwarves fought bitterly." | "The War of the Three Hammers left the Bronzebeards on the throne of Ironforge. The Dark Irons fled south, and the Wildhammers went to the Hinterlands." | 7: no source |
| 10 | "Magistrate Solomon spoke of the Blackrock threat. In Redridge, its orcs raid the farms." | "The Blackrock orcs raid the farms of Redridge from Stonewatch Keep. Lakeshire holds its bridge with its own guards." | 7, the source shape |
| 11 | "Ashenvale mourns, its ancient trees weeping for the fallen demigod." | "Grom Hellscream killed the demigod Cenarius in Ashenvale. The Warsong Clan still cuts its trees for Orgrimmar, and the night elves fight them for every grove." | 5: no feeling for a place |
| 12 | "Level 20. The Forsaken warlock's power is a testament to the dark arts." | "The Shadow Council taught the first orcs to bargain with demons. The warlocks of the Horde grow stronger. $N has reached level 20." | 10, ban 7 |
| 13 | "Duskwood: a forest of shadow, sorrow, and secrets." | "Duskwood was part of Elwynn Forest until dark magic from Karazhan turned its trees. The Night Watch of Darkshire holds the town against the dead." | Triplet. 3: show the turn |
| 14 | "The Burning Blade lurks beneath Orgrimmar, a dark heart within the Horde." | "The Burning Blade cult hides in Ragefire Chasm, beneath Orgrimmar itself." | 5, ban 18 |
| 15 | "Not just a prison, the Stockade is a symbol of Stormwind's fall." | "The prisoners of the Stockade rose up and took it from their guards. Stormwind holds the gate, and the riot goes on inside." | Ban 17: no "not just" |
| 16 | "Stranglethorn's jungle hungers, its ruins whispering of glories long past." | "The Gurubashi trolls once ruled all of Stranglethorn. Their empire broke apart, and its tribes fight among its ruins now." | 5, ban 10 |
| 17 | "One more stranger arrived at the Crossroads, where caravans gather." | "The Crossroads stands where the roads of the Barrens meet. Orc grunts hold its walls against the centaur, and the caravans of the Horde stop there." | 1: no arrival |
| 18 | "The quilboar of Razorfen are savage and cruel, a blight upon the land." | "The quilboar believe the demigod Agamaggan died in the Barrens. The great thorns of Razorfen grew from his blood, and they guard them as holy ground." | 9: the people's own belief over a judgment |
| 19 | "The dead Twilight's Hammer cultists hunger for a god they cannot reach." | "Blackfathom Deeps was a temple to Elune before the sea took it. The Twilight's Hammer worships Aku'mai there now." | 4: land on the present |
| 20 | "Teldrassil's majestic boughs cradle the night elves in eternal starlight." | "The night elves planted Teldrassil to win back their immortality. The dragons never blessed the tree, and the Gnarlpine furbolgs of the island have turned corrupt." | Ban 6, 3 |
| 21 | "Level 30. $N has killed 412 foes and finished 87 quests." | (SILENCE, or the level line of `level-lines.md`) | 10: no ledger |
| 22 | "The dwarves of Ironforge greet $N as one of their own." | "Ironforge's Mountaineers hold the passes of Khaz Modan. The dwarves grow stronger. $N has reached level 30." | Ban 20: no recognition |

## 8. The prompt

### 8.1 The compact guide

The narrator prompt fits a local model with 2048 tokens (GAMEPLAY.md 3.2.1). The persona in `narrator.txt` holds the manner today in 6 lines. The compact guide replaces that manner block, at about the same size. The text, for approval:

```text
Your manner:
- Tell one turn of history: what was, what changed it, what holds now. End on the present.
- The subject is a place, a people, a foe, or an order. Name the hero only for a deed, and rarely.
- Plain past tense for history, present for now. Sentences of 12 to 25 words.
- Literal words only. A place or an order has no body, no feeling, and no voice. Nothing grows inside the hero.
- Never cite a source, never claim fame, never tell an arrival, never count levels or kills.
- No weather, no mood, no "our hero", no "whispers", "echoes", or "to the end".
```

This adds about 30 tokens over the manner block of today. The size test of GAMEPLAY.md 3.2.1 measures it.

### 8.2 The author's note

The place note and the deed note of `narrator.rs` add one line each, because a model weighs the end of a prompt most:

- Place: "End on what holds in the place now."
- Deed: "When the deed reads well without the hero, say what changed and leave the hero out."

### 8.3 The samples

The samples teach the voice more than the rules do. The guide applies to them:

- Each sample passes every rule of sections 3 to 6.
- The two deed samples that end on a contrast with the hero ("The paladin did.", "The rogue settled the account instead.") break principle 6 in spirit. Replace them with lines that land on the world, such as pair 5 and pair 6 of 7.2. The user approves the new lines first.
- The deed samples mix the shapes of a turn in section 3, so a model does not learn one shape.

### 8.4 Sagas, tales, the summary, zone histories, `/lore`

These prompts take the same compact manner block. A saga and a summary are paragraphs, so the line limit of section 3 becomes a limit of sentences, not of the whole text: each sentence still holds 8 to 25 words and at most 2 facts in a clause.

## 9. The templates plan

`docs/plans/narrator-templates.md` stays as it is. These are the changes that it needs, for the main session to make after the user approves this guide:

1. **The `lore` field follows this guide.** Section 2.1 adds: one turn, ends on the present, no hero, no source. The prompt task of 8.2 adds: "End on what holds now."
2. **Prefer unnamed deeds.** In 3.5, on a named turn, the pick still prefers a part with no hero when the lore ends on the present. Or simpler: raise the share of unnamed lines in the rotation for kills.
3. **Drop parts that break the guide:**
   - `v.ranks` ("counts one more mage in its ranks"): a fame claim. It answers open question 6: drop it.
   - `d.leads_u` ("leads, two to none"): a joke and a ledger.
   - `i.hand_u` ("found a new hand"), `i.wearer_u` ("has a new wearer"), `mt.rider_u` ("has a rider"): an object that acts like a person.
   - `em.runs`, `em.swift` ("runs under $N"): a stiff figure. Use `mt.rides`.
   - `k.end_u` ("That was the end of {foe}"): close to "ended him". Keep `k.fallen_u`.
4. **Add consequence parts for a kill**, with no hero: "{Foe} is dead." and "{Group} has lost its leader." The second needs a `leads` fact from the lore, as `there` needs the zone.
5. **Check every template part against the ban list** in `every_template_part_passes_the_line_checks`. It does that already through `checked_line`, once the new entries are in `slop_words.txt`.
6. **Assembled examples to rewrite**: 13 ("leads, two to none"), 16 ("runs under $N"), 17 ("found a new hand"), 18 ("is in the hand of the Bookworm").

## 10. What a check can enforce

### 10.1 Deterministic checks

These are exact. Each one goes into `line_check.rs` or `slop_words.txt`. Every pattern ignores case and matches whole words, as `contains_phrase` does. `HERO` stands for `$N`, "hero", "stranger", "newcomer", and the race, class, and title words of the naming, as `arrival.rs` builds them.

| Rule | Pattern | Where |
|---|---|---|
| Ban list, rows 10 to 13, 15 to 19, 21 to 23 | the phrases of section 6 | `slop_words.txt` |
| Nothing inside the hero | `(grows?\|grew\|lives?\|lived\|burns?\|burned\|stirs?\|stirred\|rises?\|rose\|sharper\|stronger)( \w+){0,2} (in\|within\|inside\|through) HERO` | new `inside_hero_in`, beside `arrival_in` |
| NPC recognition | `(knows\|remembers\|recognizes\|fears\|greets\|honors) HERO` | the same new check |
| A source sentence | a sentence that starts `In <Capital word>( <word>){0,3}, its ` after a sentence with `spoke of` or `told of` | new `source_shape_in` |
| A fragment | a sentence of fewer than 4 words | new `fragment_in` |
| A long sentence | a sentence of more than 30 words (a margin over 25, for names of several words) | new `long_sentence_in` |
| The "not X, but Y" pivot | `not [^.!?]{3,60} but`, from the antislop regexes | new, in `slop_in` |
| Too many sentences | more than 3 sentences in a narrator line | new, with `TooManySentences` of the templates plan |
| A ledger | more than one number in a line | new, beside `NewNumber` |
| A level opener | a line that starts with `Level \d+` | new |
| A Hero-page callback | a run of 3 words in a row from the player's Hero answers, in a narrator line | new, for the saga and the summary check only when the narrator line holds no sheet |

A sentence ends at `.`, `!`, or `?` followed by a space or the end. The tests: one unit test for each pattern with a hated line, and one that each loved line passes.

### 10.2 What only the guide can enforce

These need judgment. The prompt, the samples, the skill, and the review of the user carry them:

- The line holds one turn, and the turn is true to the lore.
- The line lands on the present.
- A figure of speech that the ban list does not name.
- A triplet of moods, or of grand nouns.
- The fit of the line to its moment: "Kobee was a corpse in Deathknell" next to the Scarlet Crusade is unrelated.
- Whether a deed reads better without the hero.
- Whether silence is better than the line.

## 11. Tests

- `every_loved_line_passes_every_check`
- `every_hated_line_fails_a_check`, for each hated line that a pattern of 10.1 can catch. The rest are in a list in the test, with the reason that no pattern catches it.
- `an_order_never_grows_inside_the_hero`
- `a_line_never_cites_its_source`
- `a_line_never_claims_fame`
- `a_fragment_is_refused`
- `a_ledger_is_refused`
- `every_sample_follows_the_guide` (the samples pass the new checks)

## Appendix A. Research sources

Fetch notes: most pages came through a summarizer. warcraft.wiki.gg, Wowpedia, UESP, and the Hades wiki block direct downloads. Quotes marked [snippet] come from a search summary only. Check a quote on its page before you put it in a sample.

### A.1 Blizzard lore prose

| Source | Example | Lesson |
|---|---|---|
| [Arathor and the Troll Wars](https://warcraft.wiki.gg/wiki/Arathor_and_the_Troll_Wars) (Classic book) | "After every victory, the Arathi offered peace and equality to the conquered people; thus, they won the loyalty of those they had beaten." | Cause and effect in one sentence. Time spans as plain numbers. |
| [Ironforge - the Awakening of the Dwarves](https://warcraft.wiki.gg/wiki/Ironforge_-_the_Awakening_of_the_Dwarves) (Classic book) | "They named their land Khaz Modan, or 'Mountain of Khaz', in honor of the Titan shaper, Khaz'goroth." | End on a named thing, not on a mood. |
| [Aegwynn and the Dragon Hunt](https://warcraft.wiki.gg/wiki/Aegwynn_and_the_Dragon_Hunt) (Classic book) | "Aegwynn would never know that she had done exactly as Sargeras had planned." | Blizzard's one irony line about the future. Our narrator never uses it. |
| [The Seven Kingdoms](https://warcraft.wiki.gg/wiki/The_Seven_Kingdoms) (Classic book) | [snippet] "King Thoradin's vision of a unified humanity had faded at last." | An era ends in one short sentence that names what faded. |
| [Old Hatreds - The Colonization of Kalimdor](https://warcraft.wiki.gg/wiki/Old_Hatreds_-_The_Colonization_of_Kalimdor) (WoW manual, History of Warcraft) | "Thrall led the orcs to the continent of Kalimdor, where they founded a new homeland with the help of their tauren brethren." | One "Though X, Y" opener sets up the turn, then plain agent-verb-object facts. |
| [Chronicle Vol. 1, "The Pillars of Creation"](https://www.mmorpg.com/world-of-warcraft/previews/chronicle-volume-1-excerpt-the-pillars-of-creation-2000105345) | "Most of these relics were stored in the Vault of Antiquities, a vast repository located in Suramar." | Gloss each proper name in a few words. |
| [Quest: The People's Militia](https://warcraft.wiki.gg/wiki/Quest:The_People%27s_Militia) | "The People's Militia has but one goal: To defend the lands of Westfall and return peace to our surroundings." | Quest text is the NPC's voice: first person, present tense, exact place names. |
| [Quest: The Defias Brotherhood](https://warcraft.wiki.gg/wiki/The_Defias_Brotherhood_(quest)) | "The band of wretches responsible for driving the good people of Westfall from the land call themselves The Defias Brotherhood." | Opinion belongs to the NPC. The narrator does not take it on. |
| [Quest: Arugal Must Die](https://warcraft.wiki.gg/wiki/Arugal_Must_Die!) | "Silverpine Forest is finally free from the vice of that wretch Arugal." | The outcome names the place and its new state. The world changes, not the hero. |
| [Loading screen tips](https://warcraft.wiki.gg/wiki/Loading_screen_tips) | Mostly game tips. | Not a model for the voice. |

### A.2 Comparison craft

| Source | Example | Lesson |
|---|---|---|
| [The Wolf Queen, v1](https://www.imperial-library.info/content/wolf-queen-v1) (Elder Scrolls) | "3E 63: In the autumntide of the year, Prince Pelagius, son of Prince Uriel, ... came to the High Rock city-state of Camlorn to pay court" | A date and a lineage anchor each turn. The named in-world author is a source citation, which we refuse. |
| [Elden Ring: Remembrance of the Blasphemous](https://eldenring.wiki.fextralife.com/Remembrance+of+the+Blasphemous) | "Rykard took the form of a giant serpent that he might devour, grow, and live eternally." | Vague on purpose, archaic, no time. We give who, when, and what changed. |
| [Dark Souls: Lordvessel](https://darksouls.wiki.fextralife.com/Lordvessel) | "Lordvessel bestowed upon the chosen Undead who is destined to succeed Lord Gwyn." | A fragment with no subject. We write whole sentences. |
| [Hades: Tartarus](https://hades.fandom.com/wiki/Tartarus) | [snippet] "The lowest reaches of the Underworld are reserved not just for the Master's own abode, but for ..." | A codex in a character's voice, with "not just X, but Y". Good for an NPC, wrong for the chronicler. |
| [Disco Elysium: Encyclopedia](https://discoelysium.wiki.gg/wiki/Encyclopedia) | "Your mangled brain would like you to know there is a boxer called Contact Mike." | An encyclopedic voice with a self-aware frame. Our narrator has no frame. |

Pillars of Eternity was not fetched.

### A.3 Writing advice and slop markers

| Source | Lesson |
|---|---|
| [Wikipedia: Words to watch](https://en.wikipedia.org/wiki/Wikipedia:Manual_of_Style/Words_to_watch) | "Instead of making subjective proclamations about a subject's importance, use facts and attribution to demonstrate it." Show weight through a result, not an adjective. |
| [Wikipedia: Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing) | Puffery ("stands as", "a testament", "pivotal", "enduring legacy"), "not X, but Y", and the rule of three. A model drifts toward advertising even when asked for an encyclopedic style. |
| [sam-paech/antislop-sampler](https://github.com/sam-paech/antislop-sampler) | Lists of slop phrases from fiction models ("tapestry", "a dance of", "barely above a whisper", "little did he know", "realm") and the regex `not [^.!?]{3,60} but`. |
| [ossa-ma, AI Writing Tropes](https://gist.github.com/ossa-ma/f3baa9d25154c33095e22272c631f5a1) | "It's not X, it's Y" is the most common tell. "A single tricolon is elegant; three back-to-back tricolons are a pattern recognition failure." |
| [B. Zedan, item descriptions](https://bzedan.com/blog/building-a-game-writing-portfolio-pww-assignment-2-5-item-descriptions/) | Decide what a line is for, its tone, and its length before you write it. |
| [Smith and Worch, What Happened Here? (GDC 2010)](https://gdcvault.com/play/1012647/What-Happened-Here-Environmental) | A player reads "a discernible chain of events". One line states one link of the chain, and the world holds the rest. |

### A.4 The measure of Blizzard prose

From 46 sentences of the Arathor, Ironforge, Aegwynn, Old Hatreds, and Chronicle texts:

- Words in a sentence: mean 21.4, median 20, from 3 to 41. About one sentence in five has 30 words or more.
- Verbs: concrete past-tense actions with a named subject. Few abstract verbs, each with a named subject.
- Figures: about one sentence in five, almost all dead metaphors.
- Epithets: one judging adjective for a person is common ("cunning Arathi"). Events get no judging adjective.
- Endings: a concrete result or a name. No moral and no legacy.
