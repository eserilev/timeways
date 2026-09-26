//! Random play through the input path of the bridge, then every page of the journal. Each
//! page keeps the limits of the bridge, and the pages hold every entry once.

#![no_main]

#[path = "common.rs"]
mod common;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};
use timeways_story::serve;
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
    /// Seconds of game time before the next play, so sessions and chapters form.
    Wait(u16),
}

const FIELDS: [&str; 6] = ["origin", "background", "goal", "bond", "flaw", "traits"];

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
        Play::Wait(_) => return None,
    })
}

fn send(story: &mut Story, value: &Value) -> Vec<String> {
    serve::line(story, value.to_string().into_bytes()).unwrap_or_default()
}

fuzz_target!(|plays: Vec<Play>| {
    let mut story = Story::new(common::pack(), Store::Memory);
    send(
        &mut story,
        &json!({"type": "character_entered", "realm": "Stormrage", "name": "Ada"}),
    );
    let mut at = 1_000;
    for (batch, play) in plays.iter().enumerate() {
        match play {
            Play::Wait(seconds) => at += u64::from(*seconds) * 60,
            _ => at += 1,
        }
        if let Some(value) = input(play, at) {
            send(&mut story, &value);
        }
        for line in send(&mut story, &json!({"type": "batch_end", "id": batch + 1})) {
            common::check_output(&line);
        }
    }
    let first = send(
        &mut story,
        &json!({"type": "journal_asked", "id": 1, "page": 0}),
    );
    let first: Value = serde_json::from_str(&first[0]).unwrap();
    let pages = first["pages"].as_u64().unwrap();
    for page in 0..pages {
        let lines = send(
            &mut story,
            &json!({"type": "journal_asked", "id": 1, "page": page}),
        );
        common::check_output(&lines[0]);
        let value: Value = serde_json::from_str(&lines[0]).unwrap();
        for list in ["chapters", "places", "people", "deeds"] {
            assert!(value[list].as_array().unwrap().len() <= 200);
        }
        assert!(value["hero"]["entries"].as_array().unwrap().len() <= 200);
    }
});
