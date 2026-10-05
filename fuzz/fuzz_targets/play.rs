//! Random play for the fuzz targets that run a session through the fake bridge: the plays,
//! and a model that answers each kind of call.

use arbitrary::{Arbitrary, Unstructured};
use fake_bridge::edges::{NAMES, WORDS};
use fake_bridge::{FakeBridge, Model, Reply};
use serde_json::{Value, json};
use timeways_story::hero::PROMPT_TEXT_CHARS;
use timeways_story::npc_memory::{MAX_MEMORIES, MAX_MEMORY_CHARS};
use timeways_story::store::Store;
use timeways_story::story::Story;

pub const CHARACTERS: [&str; 2] = [
    r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#,
    r#"{"type":"character_entered","realm":"Stormrage","name":"Bea"}"#,
];

pub const FIELDS: [&str; 6] = ["origin", "background", "goal", "bond", "flaw", "traits"];
pub const KINDS: [&str; 3] = ["quest", "gossip", "book"];
/// Creature types as the addon sends them, and one that it never sends.
const CREATURES: [&str; 4] = ["beast", "critter", "humanoid", "Not A Token"];
pub const EMOTES: [&str; 5] = ["dance", "kiss", "wave", "cheer", "flex"];
pub const CAUSES: [&str; 4] = ["falling", "drowning", "lava", "fire"];
/// The genres of a quest, and one that the check refuses.
const GENRES: [&str; 7] = [
    "errand", "hunt", "mystery", "rescue", "rivalry", "comic", "romance",
];
const TIMES: [&str; 5] = ["dawn", "noon", "dusk", "night", "teatime"];
/// Goods of the list of carry steps, and one that is not on it.
const GOODS: [&str; 3] = ["Linen Cloth", "Wool Cloth", "Gold Bar"];

/// Plain words that pass the checks of a model answer.
pub const PROSE: [&str; 4] = [
    "Our hero walks on.",
    "The Lost Lantern",
    "A dance in the rain.",
    "Nobody knows why.",
];

/// Text from a list most of the time, and any text now and then. A uniform draw almost
/// never makes a name that the story takes, or words that pass a check.
#[derive(Debug)]
pub struct Pick<const LIST: u8>(pub String);

pub type Name = Pick<0>;
pub type Prose = Pick<1>;
pub type Emote = Pick<2>;
pub type Cause = Pick<3>;

impl<'a, const LIST: u8> Arbitrary<'a> for Pick<LIST> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let list: &[&str] = match LIST {
            0 => &NAMES,
            1 if u.arbitrary()? => &WORDS,
            1 => &PROSE,
            2 => &EMOTES,
            _ => &CAUSES,
        };
        let text = if u.int_in_range(0..=3)? == 0 {
            u.arbitrary()?
        } else {
            (*u.choose(list)?).to_string()
        };
        Ok(Pick(text))
    }
}

fn text<const LIST: u8>(pick: &Option<Pick<LIST>>) -> Option<&str> {
    pick.as_ref().map(|pick| pick.0.as_str())
}

#[derive(Arbitrary, Debug)]
pub enum Play {
    Zone {
        zone: Name,
        subzone: Option<Name>,
        spot: Option<(u16, u16, u16)>,
    },
    Meet(Name, Option<(u16, u16, u16)>),
    /// A hover or a target, with the creature type by index.
    See {
        name: Name,
        hostile: bool,
        creature: Option<u8>,
    },
    /// A kill for the kill step of a task.
    Kill(Name),
    Defeat(Name),
    Die {
        killer: Option<Name>,
        cause: Option<Cause>,
        killer_level: Option<u8>,
    },
    Slap(Name),
    Level(u8),
    Emote {
        emote: Emote,
        hour: Option<u8>,
    },
    SetHero {
        field: u8,
        text: Prose,
    },
    AddHero(Prose),
    RemoveHero(u64),
    Read {
        kind: u8,
        title: Option<Prose>,
        npc: Option<Name>,
        text: Prose,
    },
    Talk(Name),
    Lore(Prose),
    Quest(Name),
    /// `/quest`, and the model call stays open until `Settle` or the next batch, so two
    /// calls overlap.
    AskLater(Name),
    /// The oldest open model call ends.
    Settle,
    Accept(Option<u8>),
    Decline(Option<u8>),
    /// The next batches come from the other character.
    Switch,
    /// Seconds of game time before the next play, so sessions and chapters form.
    Wait(u16),
    /// The clock of the computer jumps: back, far ahead, or to its end.
    Clock(i64),
    /// The local hour changed, while a time step is open or not.
    Hour(u8),
    /// The count of a good in the bags, at a meeting with the NPC.
    Items {
        npc: Name,
        good: u8,
        count: u32,
    },
}

/// The words of the model for one call. A call takes the next one, and the list repeats.
#[derive(Arbitrary, Debug)]
pub struct Words {
    fails: bool,
    saga: Prose,
    footnotes: Vec<(u8, Prose)>,
    line: Prose,
    trust: i8,
    title: Prose,
    text: Prose,
    /// Steps by goal, the index of a name that the prompt lists, and a count of kills.
    steps: Vec<(u8, u8, u8)>,
    /// A name that the prompt does not list, for a step that the check refuses.
    stranger: Option<Name>,
    /// The genre of a quest, by index.
    genre: u8,
    /// The first two steps of a quest go in an any-order set.
    set: bool,
    /// The NPC of a talk offers work, so a quest call follows (GAMEPLAY.md 3.5).
    work: bool,
}

/// The plays come last, so they take every byte that is left: a long input is a long play.
#[derive(Arbitrary, Debug)]
pub struct Run {
    words: Words,
    more_words: Vec<Words>,
    plays: Vec<Play>,
}

/// A position as the addon sends it. A number past 1000 is off the map, and counts as no
/// position.
fn spot_json((map, x, y): (u16, u16, u16)) -> Value {
    json!({"map": map, "x": x % 1100, "y": y % 1100})
}

fn input(play: &Play, at: u64) -> Option<Value> {
    Some(match play {
        Play::Zone {
            zone,
            subzone,
            spot,
        } => json!({"type": "zone_entered", "at": at, "zone": zone.0, "subzone": text(subzone),
            "spot": spot.map(spot_json)}),
        Play::Meet(name, spot) => {
            json!({"type": "npc_met", "at": at, "name": name.0, "spot": spot.map(spot_json)})
        }
        Play::See {
            name,
            hostile,
            creature,
        } => {
            let reaction = if *hostile { "hostile" } else { "friendly" };
            let creature = creature.map(|index| CREATURES[usize::from(index) % CREATURES.len()]);
            json!({"type": "npc_seen", "at": at, "name": name.0, "reaction": reaction,
                "creature": creature})
        }
        Play::Kill(name) => json!({"type": "npc_killed", "at": at, "name": name.0}),
        Play::Defeat(name) => json!({"type": "npc_defeated", "at": at, "name": name.0}),
        Play::Die {
            killer,
            cause,
            killer_level,
        } => json!({"type": "died", "at": at, "killer": text(killer), "cause": text(cause),
            "killer_level": killer_level}),
        Play::Slap(name) => json!({"type": "npc_slapped", "at": at, "name": name.0}),
        Play::Level(level) => json!({"type": "level_reached", "at": at, "level": level}),
        Play::Emote { emote, hour } => {
            json!({"type": "emote_done", "at": at, "emote": emote.0, "hour": hour})
        }
        Play::SetHero { field, text } => {
            let field = FIELDS[usize::from(*field) % FIELDS.len()];
            json!({"type": "hero_set", "at": at, "field": field, "text": text.0})
        }
        Play::AddHero(text) => json!({"type": "hero_added", "at": at, "text": text.0}),
        Play::RemoveHero(number) => json!({"type": "hero_removed", "at": at, "number": number}),
        Play::Read {
            kind,
            title,
            npc,
            text: words,
        } => {
            let kind = KINDS[usize::from(*kind) % KINDS.len()];
            json!({"type": "text_seen", "at": at, "kind": kind, "title": text(title),
                "npc": text(npc), "zone": "Testvale", "text": words.0})
        }
        Play::Talk(npc) => {
            json!({"type": "talk_asked", "at": at, "npc": npc.0, "text": "any news"})
        }
        Play::Lore(question) => json!({"type": "lore_asked", "at": at, "question": question.0}),
        Play::Quest(npc) | Play::AskLater(npc) => {
            json!({"type": "quest_asked", "at": at, "npc": npc.0})
        }
        Play::Accept(number) => json!({"type": "quest_accepted", "at": at, "number": number}),
        Play::Decline(number) => json!({"type": "quest_declined", "at": at, "number": number}),
        Play::Hour(hour) => json!({"type": "hour_changed", "at": at, "hour": hour}),
        Play::Items { npc, good, count } => {
            let item = GOODS[usize::from(*good) % GOODS.len()];
            json!({"type": "items_held", "at": at, "npc": npc.0, "item": item, "count": count})
        }
        Play::Wait(_) | Play::Switch | Play::Clock(_) | Play::Settle => return None,
    })
}

/// The names of the list under `heading` in a prompt.
fn listed<'a>(prompt: &'a str, heading: &str) -> Vec<&'a str> {
    let Some((_, after)) = prompt.split_once(heading) else {
        return Vec::new();
    };
    let lines = after.lines().skip(1);
    let items = lines.take_while(|line| line.starts_with("- "));
    items.map(|line| &line[2..]).collect()
}

/// Steps from the names that the prompt lists, so most quests pass the check.
fn quest(prompt: &str, words: &Words) -> Value {
    let places = listed(prompt, "Places that the player can visit:");
    let people = listed(prompt, "People that the player can meet:");
    let prey = listed(prompt, "Creatures that the player can hunt:");
    let mut steps: Vec<Value> = Vec::new();
    for (goal, index, count) in &words.steps {
        if let Some(step) = step(*goal, *index, *count, [&places, &people, &prey]) {
            steps.push(step);
        }
    }
    if let Some(stranger) = &words.stranger {
        steps.push(json!({"goal": "meet", "npc": stranger.0}));
    }
    if words.set && steps.len() >= 2 {
        let set: Vec<Value> = steps.drain(..2).collect();
        steps.insert(0, json!({"goal": "any_order", "steps": set}));
    }
    let genre = GENRES[usize::from(words.genre) % GENRES.len()];
    json!({"title": words.title.0, "genre": genre, "text": words.text.0, "steps": steps})
}

/// One step of a goal, by its number, on a name of its list.
fn step(goal: u8, index: u8, count: u8, [places, people, prey]: [&[&str]; 3]) -> Option<Value> {
    let pick = |names: &[&str]| -> Option<String> {
        let name = names.get(usize::from(index) % names.len().max(1))?;
        Some((*name).to_string())
    };
    Some(match goal % 8 {
        0 => json!({"goal": "visit", "place": pick(places)?}),
        1 => json!({"goal": "meet", "npc": pick(people)?}),
        2 => json!({"goal": "kill", "creature": pick(prey)?, "count": count % 12}),
        3 => json!({"goal": "talk", "npc": pick(people)?}),
        4 => json!({"goal": "wait", "days": count % 5}),
        5 => {
            let time = TIMES[usize::from(count) % TIMES.len()];
            json!({"goal": "visit_at", "place": pick(places)?, "time": time})
        }
        6 => {
            let item = GOODS[usize::from(count) % GOODS.len()];
            json!({"goal": "carry", "item": item, "count": count % 22, "npc": pick(people)?})
        }
        _ => json!({"goal": "slap", "npc": pick(people)?}),
    })
}

/// The answer of a model that knows which call it answers.
fn answer(prompt: &str, words: &Words) -> Option<String> {
    if words.fails {
        return None;
    }
    let footnote = |(moment, text): &(u8, Prose)| json!({"moment": moment % 6, "text": text.0});
    let footnotes: Vec<Value> = words.footnotes.iter().map(footnote).collect();
    Some(if prompt.contains("Write chapter") {
        json!({"saga": words.saga.0, "footnotes": footnotes}).to_string()
    } else if prompt.contains("Tell the moment") {
        words.line.0.clone()
    } else if prompt.contains("small task of your own") {
        assert_hook_block(prompt);
        quest(prompt, words).to_string()
    } else if prompt.contains("A player speaks to you") {
        assert_memory_block(prompt);
        assert_hook_block(prompt);
        json!({"say": words.text.0, "trust": words.trust, "work": words.work}).to_string()
    } else {
        format!("{} [1]", words.text.0)
    })
}

/// The memories of a talk (GAMEPLAY.md 3.5): one fence of at most `MAX_MEMORIES` lines,
/// each within its limit. The model does not know which character plays, so the property
/// `a_talk_prompt_holds_at_most_five_memories_and_never_the_name_of_the_character` checks
/// the name.
fn assert_memory_block(prompt: &str) {
    let Some((_, block)) = prompt.split_once("Each line is true:\n<<<\n") else {
        return;
    };
    let (block, after) = block
        .split_once("\n>>>")
        .expect("the fence of the memories closes");
    assert!(!after.contains("Each line is true:"), "two memory fences");
    let lines: Vec<&str> = block.lines().collect();
    assert!(lines.len() <= MAX_MEMORIES, "{lines:?}");
    for line in lines {
        let memory = line
            .strip_prefix("- ")
            .expect("a memory line starts with a dash");
        assert!(memory.chars().count() <= MAX_MEMORY_CHARS, "{line}");
    }
}

/// The hook of the hero sheet (GAMEPLAY.md 3.7): at most one, with at most
/// `PROMPT_TEXT_CHARS` characters in one fence.
fn assert_hook_block(prompt: &str) {
    let heading = "Something the player wrote about their hero, as ";
    let Some((_, block)) = prompt.split_once(heading) else {
        return;
    };
    assert!(!block.contains(heading), "two hooks");
    let (_, fenced) = block.split_once("<<<\n").expect("the hook has a fence");
    let (text, _) = fenced
        .split_once("\n>>>")
        .expect("the fence of the hook closes");
    assert!(text.chars().count() <= PROMPT_TEXT_CHARS, "{text}");
}

/// A session after its plays: the bridge, the character that played last, and the done
/// reply of each batch, in order, as the addon gets them.
pub struct Played {
    pub bridge: FakeBridge,
    pub character: &'static str,
    pub replies: Vec<String>,
}

/// A model that takes the next words for each call, and repeats the list.
fn model(first: Words, more: Vec<Words>) -> Model {
    let mut words = more;
    words.insert(0, first);
    let mut next = 0;
    Box::new(move |prompt| {
        let chosen = &words[next % words.len()];
        next += 1;
        answer(prompt, chosen)
    })
}

pub fn play(run: Run, story: Story) -> Played {
    let mut bridge = FakeBridge::new(story).with_model(model(run.words, run.more_words));
    let mut character = 0;
    let mut replies = Vec::new();
    let mut at: u64 = 1_000;
    for play in &run.plays {
        at = match play {
            Play::Wait(seconds) => at.saturating_add(u64::from(*seconds) * 60),
            Play::Clock(seconds) => at.saturating_add_signed(*seconds),
            _ => at.saturating_add(1),
        };
        let batch = input(play, at).map(|line| format!("{}\n{line}", CHARACTERS[character]));
        match (play, batch) {
            (Play::Switch, _) => character = 1 - character,
            (Play::Settle, _) => {
                bridge.settle_one();
            }
            (Play::AskLater(_), Some(batch)) => {
                bridge.send(&batch);
            }
            (_, Some(batch)) => {
                if let Reply::Done(reply) = bridge.batch(&batch) {
                    replies.push(reply);
                }
            }
            (_, None) => {}
        }
    }
    Played {
        bridge,
        character: CHARACTERS[character],
        replies,
    }
}

/// The bridge checks the page and each of its lists as it takes it.
pub fn journal_page(bridge: &mut FakeBridge, character: &str, page: u64) -> String {
    let asked = json!({"type": "journal_asked", "page": page});
    match bridge.batch(&format!("{character}\n{asked}")) {
        Reply::Done(text) => text,
        Reply::Error(error) => panic!("a journal page failed: {error}"),
    }
}

/// Every page of the journal, as the addon gets them.
pub fn journal_pages(bridge: &mut FakeBridge, character: &str) -> Vec<String> {
    let first = journal_page(bridge, character, 0);
    let value: Value = serde_json::from_str(&first).unwrap_or_default();
    let count = value["pages"].as_u64().unwrap_or(0);
    let mut pages = vec![first];
    pages.extend((1..count).map(|page| journal_page(bridge, character, page)));
    pages
}

/// The story of a fuzz run: a small pack and no files.
pub fn story(pack: timeways_story::pack::Pack) -> Story {
    Story::new(pack, Store::Memory)
}
