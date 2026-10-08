# Plan: game names, the present of a line, and wrong deed tags

Status: design review, 2026-10-07. Built the same day: the game names of section 1, the tag fixes of section 3, and the outcome tag of every end. Section 2 is built apart (narrator-style.md). The user decides the open questions of section 5. The facts come from the pack of 2026-10-07 (format 4, 1865 passages), the wiki dump of the build, and the Claude review run of 22 moments of that day.

The plan answers three proposals:

1. A table from wiki names to game names (section 1).
2. A required `now` slot in the slot JSON (section 2).
3. The wrong setup tag of Moira Thaurissan, and the precision of both detectors (section 3).

## 1. Game names

### 1.1 The problem

The pack names people as the wiki does. The world names them as the game does. Every match between the two is an exact string compare: `spoiler::Names::id`, `Character::holds_about`, and `Pack::about`. So a wiki name never meets its game name.

The game name comes from the addon (`Foes.lua`): `UnitName` for a rare, and the encounter name of `ENCOUNTER_END` for a boss. A dungeon boss is elite, not rare, so only its encounter name counts.

### 1.2 The count

The game names below come from the dump: the infobox of the page, the infobox of its "(Classic)" page, its redirects, and the labels of links to it on 24,020 quest pages. A person checks each row in the game (question 1).

| What | In the pack | Wiki name differs | Rows |
|---|---|---|---|
| Foe tags (16 outcome, 15 setup) | 31 tags, 21 names | 4 names, 6 tags | Sicco Thermaplugg (Mekgineer Thermaplugg): 1 outcome, 1 setup. Sally Whitemane (High Inquisitor Whitemane): 2 setups. Aku'mai the Devourer (Aku'mai): 1 setup. Moira Thaurissan (Princess Moira Bronzebeard): 1 setup, a wrong tag (3.1). |
| Quest tags | 7 | 0 | The builder takes the `name` of the Questbox, which is the game title. |
| NPC links | 22 names | 0 | `pack_sources.toml` holds them by hand, in game form: "King Magni Bronzebeard", "Master Mathias Shaw". |
| Known bosses | 24 pages | 6 | Sicco Thermaplugg, Sally Whitemane, Moira Thaurissan, Renault Mograine (Scarlet Commander Mograine), Dal'rend Blackhand (Warchief Rend Blackhand), Dagran Thaurissan (Emperor Dagran Thaurissan). |
| Subjects of a person page (`about`) | 49 names | 14 names, 133 passages | The 6 bosses above, and Magni Bronzebeard, Jaina Proudmoore, Sylvanas Windrunner, Gelbin Mekkatorque, Mathias Shaw, Fandral Staghelm, Hamuul Runetotem, Remulos. |
| Place subjects | 157 places | 1 known | "Stormwind Stockade" is the subject of 8 passages. The game says "The Stockade". |

In all, 15 people have a wiki name that is not their game name. The table has about 15 rows.

**What breaks today:**

- The outcome passage of Sicco Thermaplugg never unlocks.
- 4 setup passages never go stale: Sicco Thermaplugg, Sally Whitemane (2), and Aku'mai. After the kill, the entry still tells "Gelbin ordered adventurers to kill Sicco."
- The own page of 6 bosses (34 passages) is never the lore of their kill. The kill of "High Inquisitor Whitemane" looks for `about = "High Inquisitor Whitemane"` and finds nothing. The thin lore rule can then silence the kill.
- Inside the builder, `known_bosses` holds page titles, and `with_known_bosses` and `instance_of` compare infobox names. For "Dagran Thaurissan" the two differ. His tag works only because his infobox says "Boss".

**Not a name problem, same symptom.** 5 of the 16 outcome foe tags name a foe that is neither rare nor a boss, by its infobox: Mor'Ladim (elite), Hitah'ya the Keeper, Yarrog Baneshadow, Demon Spirit, and Tyranis Malem. The addon sends no `npc_defeated` for them. So 6 of 16 outcome foe tags never unlock today. Question 4 asks what to do with these 5.

### 1.3 Can the dump build the table alone?

No. The dump gives good candidates, but each source fails somewhere:

| Source | Finds | Fails |
|---|---|---|
| Infobox `name` of the page | "Emperor Dagran Thaurissan" | Sally, Sicco, Moira: the lore name. |
| Infobox `name` of the "(Classic)" page | High Inquisitor Whitemane, Mekgineer Thermaplugg, Princess Moira Bronzebeard | "Rend Blackhand": the game says "Warchief Rend Blackhand". Most pages have no "(Classic)" page. |
| Redirects | "High Inquisitor Whitemane", "Master Mathias Shaw" | Noise: "Green Jesus" (Thrall), "Twilight Father" (Benedictus, a later name), "Captured Orc" and "Warsong Warrior" (Eitrigg). A generic name such as "Warsong Warrior" is a real mob. With that row, its kill counts as a kill of Eitrigg. |
| Link labels on quest pages | "Warchief Rend Blackhand", "Arch Druid Fandral Staghelm" | Nicknames and pronouns: "my father", "Their leader". |
| None of them | | Aku'mai: the pack tag comes from the page "Aku'mai the Devourer". The game name is on another page, "Aku'mai", with no redirect between them. "Renault Mograine" and "Scarlet Commander Mograine" are two pages too. |

**Recommendation: a hand-checked data file, not a table in the pack.** The file is `crates/story/data/game_names.toml`. It holds names only, no lore text, as `docs/plans/npc-knowledge.md` does for factions. The NPC links already work this way, and all 22 are right.

```toml
# The game names of people whose wiki name differs (docs/plans/lore-names-and-now.md).
# A person checks each row in the game.
[[person]]
wiki = "Sally Whitemane"
game = ["High Inquisitor Whitemane"]

[[person]]
wiki = "Sicco Thermaplugg"
game = ["Mekgineer Thermaplugg"]
```

- `game` is a list: a boss can have an encounter name and a unit name.
- The story crate reads the file with `include_str!`, as it reads the templates. A fix needs no new pack.
- A candidate tool writes the table of 1.2 again from a dump: `timeways-pack game-names <dump>`. It prints candidates for each person page of the list. It writes no file. A person picks the rows.
- The builder report gets one more line: each foe of a tag, and each person subject, with no row and no game evidence. So a new page in the list shows its gap at once.

Why not in the pack: the builder runs unattended on the computer of the player. A wrong row unlocks a spoiler, and no person sees it there. CI has no dump, so no test reads a table that only the dump makes.

### 1.4 The match: exact aliases, never a normalization

A row says that two exact names are one person. Nothing else matches.

A normalization, such as "strip a title" or "match the surname", fails on the pack itself:

- "Thaurissan": Emperor Dagran Thaurissan, Moira Thaurissan, and Sorcerer-Thane Thaurissan. This is the bug of section 3.
- "Bronzebeard": King Magni Bronzebeard and Princess Moira Bronzebeard. With a surname match, a meeting with Moira in Blackrock Depths counts as a meeting with Magni, and opens the passages of Magni.
- "Fordring": Tirion Fordring and Taelan Fordring. "Mograine": Renault Mograine and Highlord Mograine. "Blackhand": Rend and his father.
- A title word is not always a title: "Herod" has no title, and "Lord Cobrahn" keeps "Lord" in the game.

The candidate tool can use a normalization to propose rows. The runtime never does.

### 1.5 Where it applies

Every name goes through one function, `game_names::person(name) -> &str`. It gives the wiki name of a row for a game name, and the name itself otherwise. Both sides of each compare go through it:

| Compare | File | Today |
|---|---|---|
| Kill facts against outcome tags | `spoiler::outcome_allowed` | broken for 1 tag |
| Kill facts against setup tags | `spoiler::setup_allowed` | broken for 4 tags |
| Met facts against NPC links (the spoiler limit) | `Character::knows_all` | works, by hand. The function keeps it safe when a link changes. |
| The subject of a moment against `about` | `narrator_lore::lore_about`, `own_page` | broken for 14 people |
| The subjects of the thin lore rule | `narrator_lore::is_thin` | broken for the same 14 |
| Known bosses against infobox names | `pack_sources::known_bosses`, `setup_passages::instance_of` | broken for Dagran Thaurissan |
| The text of a passage that names the subject | `check::mentions` | no change. A passage that says "Whitemane" alone does not name "High Inquisitor Whitemane", and the row does not change that. |

The rules crate keeps reading ids. `spoiler::Names::id` gives one id to all names of one row.

### 1.6 Proofs and tests

The rules crate gets a small module `game_names.rs` over ids: `person(rows: &[(u32, u32)], id: u32) -> u32`, an index loop. Lean proves (`lean/Timeways/GameNames.lean`):

| Theorem | In plain words |
|---|---|
| `a_kill_under_either_name_unlocks_the_outcome` | For a row (g, w): a passage tagged w passes `outcome_usable` after a kill of g, and after a kill of w. |
| `a_kill_under_either_name_makes_the_setup_stale` | The same for `setup_usable`, inverted. |
| `a_kill_of_another_person_unlocks_nothing` | When `person` gives two names two ids, the kill of one never unlocks the tag of the other. |
| `person_is_idempotent` | `person(person(x)) = person(x)`. |

The theorems assume that the rows are a function: no game name in two rows, and no game name that is also the wiki name of a row. A unit test checks that on the data file: `the_game_names_are_a_function`.

Tests in the story crate:

- `a_kill_under_the_game_name_unlocks_a_passage_tagged_with_the_wiki_name` and the setup twin.
- `a_kill_of_moira_never_unlocks_a_tag_of_emperor_thaurissan` and `meeting_moira_bronzebeard_never_meets_magni`: the surname edges.
- `the_kill_of_high_inquisitor_whitemane_takes_the_page_of_sally_whitemane`.
- `no_row_holds_a_bare_surname`: a game name of one word needs a wiki name of one word.
- A property test in `properties.rs`, `a_kill_under_any_name_of_a_row_unlocks_its_tags`: random rows, random tags, random kills. The kills come from the names of the rows most of the time, and from a shared surname some of the time.

## 2. The present of a line

### 2.1 What the review run shows

The proposal says that lines slip into trivia because nothing forces "now". The run of 2026-10-07 shows a different cause. Of 21 answers:

- 19 already end on a present clause. 11 of them end on "holds" or "still holds".
- 2 end in the past: the Shaw line and the Stilwell line. In both, the passage held no present fact at all. The Shaw passage is a part of the page of Edwin VanCleef that names no place.
- Of the 19 present endings, 10 restate a present fact of the passage. 4 are a weak inference ("the thorn vines form Razorfen Kraul now"). **5 are invented:**
  - "Mutanus the Devourer still holds the Wailing Caverns." The setup prompt asks for this shape: "..., and <the foe> still holds <the place>." (`narrator.rs`).
  - "Archaedas still holds Uldaman." The same shape. The lore says that he guards the Discs.
  - "...and they hold the Deadmines now." The lore was a quote of VanCleef, with no present state.
  - "...and that grievance against the kingdom stands." A filler present.
  - "Mr. Smite ... still holds that post in the Deadmines now." The lore is in the past tense.

So the ask for a present already pushes the model to invent one. A required `now` slot pushes harder. The real faults are the pick of the passage and the lack of a check on the present clause.

### 2.2 Why not a required `now` slot

- **It invents.** With a passage in the past tense, the model must write a present state. It writes "X still holds Y". About half of the passages (47%) hold no present sentence: 39% of the place pages, 52% of the other pages, and 68% of the books, by a crude count of present verbs.
- **The present of a wiki page is not the present of 25 ADP.** The wiki writes "today" as of the live game. The later terms remove the worst of it, and the rest leaks: "Moira ... rules Ironforge".
- **It fights the templates.** For every deed, the code already writes the present: "Edwin VanCleef has fallen.", "The Riverpaw have no leader now.", "The order grows stronger." A model `now` before it doubles the present. It can contradict it: "VanCleef still holds the Deadmines. Edwin VanCleef has fallen."
- **It costs every prompt.** One more field, one more line of meaning, and a `now` in every sample. The 26 samples change. It is small, about 30 tokens, but it buys nothing for deeds.
- **The check is the same work.** A `now` slot still needs the check of 2.3 to stop the invention. With the check, the slot adds only the pressure.

### 2.3 Recommendation

Four parts. Each one stands alone.

**A. The code writes the present when the game knows it.** Deeds and tenth levels do this today. Setups do not: the prompt asks the model for "<the foe> still holds <the place>". The gate already proves the fact: a setup shows only while the player has not defeated its foe. So a setup gets a code coda, as a kill does: a frame `f.setup`, the lore, then a part such as "{Foe} is still alive." The user approves the words (question 2). The prompt drops its forced shape.

**B. A place moment never takes pure biography.** A new zone, a first entry, and a later entry take a passage of a person page only when its shown text names a place: the place of the moment, or any place link of the pack. Of the 4 later entries of the Deadmines in the run, this drops the Shaw passage and the quote of VanCleef, and keeps "VanCleef took the gold mines of Westfall" and Mr. Smite. A later entry also prefers a passage with a present sentence. The pick of `instance_lore::next_passage` reads the candidates in the order it gets, so the theorem `an_instance_passage_is_told_at_most_once` holds for any order.

**C. A present claim needs a present source.** A new check of the lore, `unsourced_present_in`, beside `lore_faults`:

1. A clause is present when its verb is in the present tense: a closed list ("is", "are", "has", "holds", "rules", "remains", "lies", "lives", "guards", "leads"), any verb after "still" or before "now", and a verb in "-s" after a name.
2. A present clause needs a source in the lore of the prompt that shares one word that tells something with it, besides the names of the moment: "Defias" for "the Defias Brotherhood holds it now". A source is a present sentence, or a past sentence that starts a state with no end, from a closed list: "began to", "took over", "has since", "fell into the hands of", "was taken over by". So the loved line "The rest still roam there, undead." passes on "began to roam".
3. A present clause never names a foe that the player defeated: "VanCleef still holds the Deadmines" after the kill is refused. The world knows the kill, so this is a fact of the game.
4. A sentence inside quote marks in the lore is no source.

The reason for the retry: "The lore does not say that this holds now. End on a fact that the lore holds, or answer SILENCE."

On the run, part A takes the 6 setup endings away from the model, Mutanus and Archaedas among them. Of the 13 present endings that stay with the model, C refuses 5: the 3 invented ones ("they hold the Deadmines now", "that grievance stands", Mr. Smite), and the 2 weak endings of Razorfen Kraul, whose lore holds only "the vines sprouted". It passes the other 8.

**D. Principle 4 for an arrival with no present in its lore.** Today the guide says that every line ends on the present. With B and C, an arrival whose lore holds no present gets either an ending on the last event of the turn, or silence. This is a decision of the user (question 3). My advice: allow the last event. "Wilder alone escaped, and the rest of the company died in the tunnel." is a good line with no "now".

### 2.4 How it fits the rest

| Moment | Present today | With the plan |
|---|---|---|
| Kill, death, revenge, slap, quest, mount, item | code part | no change. C refuses a lore that tells the foe as alive. |
| Tenth level | code part ("grows stronger") | no change |
| First entry with a setup | model, forced shape | code coda (A) |
| New zone, capital, first entry with no setup | model | model, with check C, and the pick of B |
| Later entry | model | model, with B and C |
| Flavor moment | free text | no change |

The templates plan (`narrator-templates.md` 3.3) gets one frame, `f.setup`, and its coda parts. The rotation keys on the coda, as on a deed.

### 2.5 Cost

- **Prompt:** less than today. The setup prompt loses its forced shape. The check adds one reason to a retry, about 20 tokens.
- **Silence:** more. A later entry with only past biography left is silent. An invented present gets one retry and then silence. On the run, 5 lines in 21 go to a retry, and the Shaw line goes before the call. This follows principle 8: silence over slop.
- **Code:** one frame and two to four parts, one check of about 80 lines, and one filter in `narrator_lore.rs`.

### 2.6 Risks

- **The tense check is a heuristic.** A present verb outside the list passes unchecked. A plural noun after a name ("Stormwind guards") reads as a verb, and a start of a state outside the list reads as no source. A false refusal costs one retry, and at worst a silence.
- **The wiki present leaks.** C accepts a present clause from a present sentence of the wiki, and that sentence can be later than 25 ADP. The later terms of the pack stay the guard.
- **B drops good lore.** A passage of a boss page that names no place, such as the oath of VanCleef, never tells a place moment. It still tells the kill of that boss.

### 2.7 Tests

- `a_setup_entry_ends_on_the_code_coda` and `a_setup_coda_names_the_tagged_foe`.
- `a_place_moment_never_takes_a_person_passage_that_names_no_place` (the Shaw passage).
- `a_later_entry_prefers_a_passage_with_a_present_sentence`.
- `an_invented_present_is_refused`: the 3 endings of 2.1 that C catches, verbatim.
- `a_present_from_the_lore_passes`: the 10 grounded endings of the run, verbatim.
- `a_defeated_foe_is_never_alive_in_a_line`.
- `every_loved_line_passes_the_present_check`, with its lore.
- A property test, `no_present_clause_without_a_present_source`: random lore and random answers. The edges: a lore with no present sentence, a present sentence inside quotes, a present clause that names only the moment, and a foe in the defeated list.
- Lean: a theorem in `NarratorShapes.lean` that every shape of `f.setup` ends on its coda, as the deed shapes do.

## 3. Wrong deed tags

### 3.1 Moira Thaurissan

The passage, on the page "Moira Thaurissan": "King Magni, upset that Moira was with an arch-enemy of his family, sent a team to kill Emperor Thaurissan and retrieve the presumably ensorcelled Moira back to Ironforge." The tag is Moira. It is Emperor Dagran Thaurissan.

Four causes add up (`setup_passages.rs`, `outcome_passages.rs`):

1. **The page is a candidate first.** Her page has places only, so `known_bosses` makes her a foe. For Classic that is right: "Princess Moira Bronzebeard" fights beside the Emperor, with the faction "Boss" on her tactics page. Her modern infobox says Alliance. A fix by the modern infobox is a trap: the infobox of Sally Whitemane is friendly to both sides too.
2. **The line links to no one.** "Emperor Thaurissan" is plain text. So the list of foes holds only the page itself, and the page wins by default.
3. **The span of a commission runs to the end of the sentence.** `Span::After(to)` takes every word after "to". "...and retrieve ... Moira" is inside it, so the object of "retrieve" counts as the foe. "retrieve", "rescue", "free", "recover", and "find" never name a foe.
4. **A latent bug in `name_at`.** The start of a name takes every word of the name, and ignores the rule of a title of another name. So "Emperor Thaurissan" moves the start of "Moira Thaurissan" to "Thaurissan". It did not decide this tag, because Dagran was not in the list. With both in the list, it can.

**A class, not a one-off.** Cause 3 hits any hostage, item, or ally after a second verb, on any page whose own person is a candidate. In this pack it hit 1 tag of 15 setup foe tags.

**The fix:**

- A deed verb binds the name of a foe only up to the next "and" with a verb, or the next comma before a verb. A foe binds to a hostile verb: "kill", "slay", "destroy", "defeat", "stop", "end", "eliminate", "assassinate", "hunt", "confront", "attack", "raid", "assault", "punish", "weaken", "overthrow", "purge", "conquer", and "protect" or "guard" for the ward of a foe (Aku'mai). "retrieve", "rescue", "free", "recover", "find", and "investigate" bind no foe.
- With that rule, this sentence names no foe of its list on the page of Moira, and the other rules find no deed: the passage sets up nothing. That is right, because the line links no page of the Emperor. The same sentence on the page of Dagran Thaurissan still sets up his defeat.
- `name_at` applies the title rule to the start too.
- Tests: `a_hostage_after_and_retrieve_is_no_foe` (this sentence), `the_start_of_a_name_skips_a_title_of_another_name`, and a fuzz seed with this sentence in `fuzz/seeds/`.

### 3.2 Every tag, checked against its text

I read each of the 46 tags against its passage and the wikitext of its line.

**Setups (16): 3 wrong, 2 leak an end.**

| Tag | Verdict | Why |
|---|---|---|
| Blackrock Depths, Emperor Dagran Thaurissan, "The Emperor kidnapped ... Princess Moira" | wrong | No setup. The cue is the intent "Thaurissan ... wanted to free his people". An intent of a foe toward his own people is no hook. The passage is the love story of Moira. |
| Blackrock Depths, Moira Thaurissan | wrong | 3.1. |
| The Temple of Atal'Hakkar, quest "Into The Temple of Atal'Hakkar" | wrong | The commission is Thrall's ("sent a group of orcs to investigate"). Its own reference is "Pool of Tears (Horde)", which continues, so the rule took the last chain end of the paragraph: an Alliance quest of another sentence. A Horde player never goes stale on it. |
| Scarlet Monastery, Sally Whitemane, page "Sally Whitemane" | leaks | It ends "Although in time the other leaders succumbed, Sally survived." That tells the death of Mograine, Herod, and Doan before the kills. "succumbed" is no end word. |
| Scarlet Monastery, Sally Whitemane, page "Scarlet Crusade" | leaks | "after Commander Mograine's mysterious death" tells his death. The end words hold "death of", not "X's death". |
| The other 11 | right | VanCleef (3), Aku'mai, Bazil Thredd, Scarlet Commander Mograine, Archaedas, Mutanus, Sicco Thermaplugg, Herod, and "King Magni Bronzebeard sent a team to kill Emperor Thaurissan". |

**Outcomes (30): 5 wrong or partial, 1 false outcome.**

| Tag | Verdict | Why |
|---|---|---|
| Jintha'Alor, Gahz'rilla | wrong | The deed is the Ancient Egg: "secured by hapless adventurers", with the reference "The Ancient Egg". That quest continues, so rule 2 skipped it, and rule 4 took the first linked foe, from a sentence with no deed. The passage also tells "This egg would eventually be used to rebirth the Blood God", which is later than Molten Core. |
| Booty Bay, quest "Keep An Eye Out" | wrong | The deed is the plot of the Bloodsail, with the references "The Bloodsail Buccaneers (3)" and "Bloodsail Orders (Classic)". The first has a number, so it counts as no quest. Rule 2 took the reference of the sentence before. It is unresolved in truth. |
| Theramore Isle, quest "Confirming the Suspicion" | wrong | That reference is the death of the Hyal family. The deed "brought the Grimtotem to justice" cites "Raze Direhorn Post!". |
| Stormwind Stockade, quest "Crime and Punishment" | partial | Two deeds: Targorr, killed for Guard Berton ("What Comes Around..."), and Dextren Ward. One tag gates both. |
| Argent Dawn, Balnazzar | partial | It also tells "The champions who accomplished such a task", the defeat of Baron Rivendare. The tag gates only Balnazzar. |
| Blackrock clan, unresolved | false outcome | "Horde adventurers ... discovered Blackrock documents" is a clue, not an end. "discovered" counts as a result. The passage is a setup for Warchief Rend Blackhand in Blackrock Spire. Unresolved is safe, but the setup is lost. |
| Westfall, Defias Brotherhood, Stromgarde Keep (unresolved) | missed | Resolvable: Edwin VanCleef (twice) and Galen Trollbane. Safe. |
| The other 21 | right | The tag fits the text. Mor'Ladim, Hitah'ya, Yarrog, Demon Spirit, and Tyranis Malem are right, but never unlock (1.2). |

**Later lore in the pack**, found on the way: "During the war in Draenor, Koristrasza..." (Razorfen Downs), the "clever facsimile of Thermaplugg" (Sicco Thermaplugg), and "rebirth the Blood God" (Jintha'Alor). Each one needs a later term.

### 3.3 The classes of fault

| Class | Tags | Fix |
|---|---|---|
| The reference of another sentence: rule 2 takes the last chain end of the paragraph | Temple, Booty Bay, Theramore | Take the references of the deed sentence only, the `<ref>` right after it. |
| A fallback to a foe of another sentence: rule 4 | Gahz'rilla | Drop rule 4. A deed with no foe in its sentence and no quest is unresolved. That is safe. |
| A second deed in one passage | Stockade, Argent Dawn | Two deed sentences with two resolutions make the passage unresolved, or tag it with both. Both is a change of format and of the gate: "all of these". |
| The object of a second verb read as the foe | Moira | 3.1. |
| An intent that is no hook | Dagran, love story | An intent cue needs the foe as the subject, and an object that is not his own people or kin. Simpler: drop "wanted", "hopes", and "hoping" from the intents, and keep "sought" and "planned". |
| An end word missing | Sally (2): "succumbed", "X's death" | Add "succumbed", "perished", "fell", and a possessive before "death". |
| A clue read as a result | Blackrock clan | "discovered" and "uncovered" count as a result only with a person as the object. Else they are no end. |

Each fix gets a named test with the sentence of its tag, and the sentence goes to `fuzz/seeds/`. The builder report then shows each tag again, so a person can check the 46 by eye.

## 4. Order of the build

1. The fixes of section 3 and their tests. They change tags, so they come first.
2. The game names (section 1): the data file, `game_names::person`, the rules module and its theorems, and the candidate tool.
3. Part B and part C of section 2, with their tests.
4. Part A of section 2, after the user approves the coda parts.

## 5. Open questions for the user

1. **The rows.** Approve the 15 rows of 1.2. Two need a check in the game: the encounter name of Aku'mai, and of the High Inquisitor Whitemane fight. `/twdev` can print the name of the last `npc_defeated`.
2. **The coda of a setup.** "{Foe} is still alive." is the safe form. "{Foe} still holds {instance}." reads better, but overclaims for Archaedas and Mutanus. Which words?
3. **An arrival with no present in its lore.** End on the last event of the turn, or answer silence?
4. **The 5 foes that are not rare.** Mor'Ladim, Hitah'ya, Yarrog, the Demon Spirit, and Tyranis Malem never give a kill. Gate their passages on the quest of the deed instead, or let the addon send the kill of any named foe of a quest?
5. **Two deeds in one passage.** Tag both, and show the passage after both deeds, or leave the passage unresolved?
