//! Random play through the fake bridge, then every page of the journal. The real checks of
//! the relay take each answer, and each page fits a slot of the game.

#![no_main]

#[path = "common.rs"]
mod common;

use arbitrary::Arbitrary;
use fake_bridge::{FakeBridge, Model, Reply};
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use timeways_story::store::Store;
use timeways_story::story::Story;

#[derive(Arbitrary, Debug)]
enum Play {
    Zone {
        zone: String,
        subzone: Option<String>,
    },
    Meet(String),
    Defeat(String),
    Die {
        killer: Option<String>,
        cause: Option<String>,
        killer_level: Option<u8>,
    },
    Slap(String),
    Level(u8),
    Emote {
        emote: String,
        hour: Option<u8>,
    },
    SetHero {
        field: u8,
        text: String,
    },
    AddHero(String),
    RemoveHero(u64),
    Read {
        kind: u8,
        title: Option<String>,
        npc: Option<String>,
        text: String,
    },
    /// A talk, and the words of the model: a rumor.
    Talk {
        npc: String,
        say: String,
    },
    /// `/quest`, and the steps that the model proposes: `true` visits.
    Quest {
        npc: String,
        steps: Vec<(bool, String)>,
    },
    Accept,
    Decline,
    /// Seconds of game time before the next play, so sessions and chapters form.
    Wait(u16),
}

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

const FIELDS: [&str; 6] = ["origin", "background", "goal", "bond", "flaw", "traits"];
const KINDS: [&str; 3] = ["quest", "gossip", "book"];

fn input(play: &Play, at: u64) -> Option<Value> {
    Some(match play {
        Play::Zone { zone, subzone } => {
            json!({"type": "zone_entered", "at": at, "zone": zone, "subzone": subzone})
        }
        Play::Meet(name) => json!({"type": "npc_met", "at": at, "name": name}),
        Play::Defeat(name) => json!({"type": "npc_defeated", "at": at, "name": name}),
        Play::Die {
            killer,
            cause,
            killer_level,
        } => {
            json!({"type": "died", "at": at, "killer": killer, "cause": cause, "killer_level": killer_level})
        }
        Play::Slap(name) => json!({"type": "npc_slapped", "at": at, "name": name}),
        Play::Level(level) => json!({"type": "level_reached", "at": at, "level": level}),
        Play::Emote { emote, hour } => {
            json!({"type": "emote_done", "at": at, "emote": emote, "hour": hour})
        }
        Play::SetHero { field, text } => {
            let field = FIELDS[usize::from(*field) % FIELDS.len()];
            json!({"type": "hero_set", "at": at, "field": field, "text": text})
        }
        Play::AddHero(text) => json!({"type": "hero_added", "at": at, "text": text}),
        Play::RemoveHero(number) => json!({"type": "hero_removed", "at": at, "number": number}),
        Play::Read {
            kind,
            title,
            npc,
            text,
        } => {
            let kind = KINDS[usize::from(*kind) % KINDS.len()];
            json!({"type": "text_seen", "at": at, "kind": kind, "title": title, "npc": npc, "zone": "Testvale", "text": text})
        }
        Play::Talk { npc, .. } => {
            json!({"type": "talk_asked", "at": at, "npc": npc, "text": "any news"})
        }
        Play::Quest { npc, .. } => json!({"type": "quest_asked", "at": at, "npc": npc}),
        Play::Accept => json!({"type": "quest_accepted", "at": at}),
        Play::Decline => json!({"type": "quest_declined", "at": at}),
        Play::Wait(_) => return None,
    })
}

/// The words of the model for the call of this play, if the play makes one.
fn model_text(play: &Play) -> Option<String> {
    match play {
        Play::Talk { say, .. } => Some(json!({"say": say, "trust": 0}).to_string()),
        Play::Quest { steps, .. } => {
            let steps: Vec<Value> = steps
                .iter()
                .map(|(visit, name)| match visit {
                    true => json!({"goal": "visit", "place": name}),
                    false => json!({"goal": "meet", "npc": name}),
                })
                .collect();
            Some(json!({"title": "A Task", "text": "Go.", "steps": steps}).to_string())
        }
        _ => None,
    }
}

fuzz_target!(|plays: Vec<Play>| {
    let answers = Rc::new(RefCell::new(VecDeque::new()));
    let queue = Rc::clone(&answers);
    let model: Model = Box::new(move |_| queue.borrow_mut().pop_front());
    let story = Story::new(common::pack(), Store::Memory);
    let mut bridge = FakeBridge::new(story).with_model(model);
    let mut at = 1_000;
    for play in &plays {
        match play {
            Play::Wait(seconds) => at += u64::from(*seconds) * 60,
            _ => at += 1,
        }
        let Some(line) = input(play, at) else {
            continue;
        };
        answers.borrow_mut().extend(model_text(play));
        bridge.batch(&format!("{CHARACTER}\n{line}"));
    }
    let first = journal_page(&mut bridge, 0);
    let pages = first["pages"].as_u64().unwrap_or(0);
    for page in 1..pages {
        journal_page(&mut bridge, page);
    }
});

/// The bridge checks the page and each of its lists as it takes it.
fn journal_page(bridge: &mut FakeBridge, page: u64) -> Value {
    let asked = json!({"type": "journal_asked", "page": page});
    match bridge.batch(&format!("{CHARACTER}\n{asked}")) {
        Reply::Done(text) => serde_json::from_str(&text).unwrap(),
        Reply::Error(error) => panic!("a journal page failed: {error}"),
    }
}
