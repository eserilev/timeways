//! Random play through the fake bridge, with a model that answers each kind of call, then
//! every page of the journal. The real checks of the relay take each answer, and each page
//! fits a slot of the game.

#![no_main]

#[path = "common.rs"]
mod common;

use arbitrary::{Arbitrary, Unstructured};
use fake_bridge::edges::{NAMES, WORDS};
use fake_bridge::{FakeBridge, Model, Reply};
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTERS: [&str; 2] = [
    r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#,
    r#"{"type":"character_entered","realm":"Stormrage","name":"Bea"}"#,
];

const FIELDS: [&str; 6] = ["origin", "background", "goal", "bond", "flaw", "traits"];
const KINDS: [&str; 3] = ["quest", "gossip", "book"];
const EMOTES: [&str; 5] = ["dance", "kiss", "wave", "cheer", "flex"];
const CAUSES: [&str; 4] = ["falling", "drowning", "lava", "fire"];

/// Plain words that pass the checks of a model answer.
const PROSE: [&str; 4] = [
    "Our hero walks on.",
    "The Lost Lantern",
    "A dance in the rain.",
    "Nobody knows why.",
];

/// Text from a list most of the time, and any text now and then. A uniform draw almost
/// never makes a name that the story takes, or words that pass a check.
#[derive(Debug)]
struct Pick<const LIST: u8>(String);

type Name = Pick<0>;
type Prose = Pick<1>;
type Emote = Pick<2>;
type Cause = Pick<3>;

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
enum Play {
    Zone {
        zone: Name,
        subzone: Option<Name>,
    },
    Meet(Name),
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
}

/// The words of the model for one call. A call takes the next one, and the list repeats.
#[derive(Arbitrary, Debug)]
struct Words {
    fails: bool,
    saga: Prose,
    footnotes: Vec<(u8, Prose)>,
    line: Prose,
    trust: i8,
    title: Prose,
    text: Prose,
    /// Steps by the index of a name that the prompt lists: `true` visits a place.
    steps: Vec<(bool, u8)>,
    /// A name that the prompt does not list, for a step that the check refuses.
    stranger: Option<Name>,
}

/// The plays come last, so they take every byte that is left: a long input is a long play.
#[derive(Arbitrary, Debug)]
struct Run {
    words: Words,
    more_words: Vec<Words>,
    plays: Vec<Play>,
}

fn input(play: &Play, at: u64) -> Option<Value> {
    Some(match play {
        Play::Zone { zone, subzone } => {
            json!({"type": "zone_entered", "at": at, "zone": zone.0, "subzone": text(subzone)})
        }
        Play::Meet(name) => json!({"type": "npc_met", "at": at, "name": name.0}),
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
    let mut steps: Vec<Value> = Vec::new();
    for (visit, index) in &words.steps {
        let names = if *visit { &places } else { &people };
        let Some(name) = names.get(usize::from(*index) % names.len().max(1)) else {
            continue;
        };
        steps.push(match visit {
            true => json!({"goal": "visit", "place": name}),
            false => json!({"goal": "meet", "npc": name}),
        });
    }
    if let Some(stranger) = &words.stranger {
        steps.push(json!({"goal": "meet", "npc": stranger.0}));
    }
    json!({"title": words.title.0, "text": words.text.0, "steps": steps})
}

/// The answer of a model that knows which call it answers.
fn answer(prompt: &str, words: &Words) -> Option<String> {
    if words.fails {
        return None;
    }
    let footnote = |(moment, text): &(u8, Prose)| json!({"moment": moment % 6, "text": text.0});
    let footnotes: Vec<Value> = words.footnotes.iter().map(footnote).collect();
    Some(if prompt.contains("You are a bard of Azeroth") {
        json!({"saga": words.saga.0, "footnotes": footnotes}).to_string()
    } else if prompt.contains("You are the narrator") {
        words.line.0.clone()
    } else if prompt.contains("small task of your own") {
        quest(prompt, words).to_string()
    } else if prompt.contains("A player speaks to you") {
        json!({"say": words.text.0, "trust": words.trust}).to_string()
    } else {
        format!("{} [1]", words.text.0)
    })
}

fuzz_target!(|run: Run| {
    let mut words = run.more_words;
    words.insert(0, run.words);
    let mut next = 0;
    let model: Model = Box::new(move |prompt| {
        let chosen = &words[next % words.len()];
        next += 1;
        answer(prompt, chosen)
    });
    let story = Story::new(common::pack(), Store::Memory);
    let mut bridge = FakeBridge::new(story).with_model(model);
    let mut character = 0;
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
                bridge.batch(&batch);
            }
            (_, None) => {}
        }
    }

    // After any play, an event at the time of the clock still lands in the world.
    let zone = json!({"type": "zone_entered", "at": now(), "zone": "Newvale"});
    bridge.batch(&format!("{}\n{zone}", CHARACTERS[character]));
    let journal = whole_journal(&mut bridge, CHARACTERS[character]);
    let places = journal["places"].as_array().cloned().unwrap_or_default();
    assert!(
        places.iter().any(|place| place["name"] == "Newvale"),
        "an event after the play did not land"
    );
    assert_quest_limits(&journal["quests"]);
});

fn now() -> u64 {
    let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since_epoch.map_or(0, |elapsed| elapsed.as_secs())
}

/// Every page, with the lists of each page joined in order.
fn whole_journal(bridge: &mut FakeBridge, character: &str) -> Value {
    let first = journal_page(bridge, character, 0);
    let pages = first["pages"].as_u64().unwrap_or(0);
    let mut lists: serde_json::Map<String, Value> = serde_json::Map::new();
    for page in (0..pages.max(1)).map(|page| journal_page(bridge, character, page)) {
        for name in ["places", "quests"] {
            let items = page[name].as_array().cloned().unwrap_or_default();
            let joined = lists.entry(name).or_insert_with(|| json!([]));
            joined
                .as_array_mut()
                .into_iter()
                .for_each(|list| list.extend(items.clone()));
        }
    }
    Value::Object(lists)
}

/// At most 3 open quests, at most one open quest and one waiting offer for each giver
/// (GAMEPLAY.md 3.4).
fn assert_quest_limits(quests: &Value) {
    let quests = quests.as_array().cloned().unwrap_or_default();
    for status in ["accepted", "offered"] {
        let mut givers: Vec<&str> = quests
            .iter()
            .filter(|quest| quest["status"] == status)
            .filter_map(|quest| quest["giver"].as_str())
            .collect();
        if status == "accepted" {
            assert!(givers.len() <= 3, "{} open quests", givers.len());
        }
        givers.sort_unstable();
        assert!(
            givers.windows(2).all(|pair| pair[0] != pair[1]),
            "two {status} quests of one giver"
        );
    }
}

/// The bridge checks the page and each of its lists as it takes it.
fn journal_page(bridge: &mut FakeBridge, character: &str, page: u64) -> Value {
    let asked = json!({"type": "journal_asked", "page": page});
    match bridge.batch(&format!("{character}\n{asked}")) {
        Reply::Done(text) => serde_json::from_str(&text).unwrap(),
        Reply::Error(error) => panic!("a journal page failed: {error}"),
    }
}
