//! Any play through the real checks of the bridge (relay SPEC.md 9.8). Every batch gets one
//! answer that the bridge takes, and the world stays open for the next event.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::edges::{CLOCK_STEPS, NAMES, PAGES, WORDS};
use fake_bridge::{FakeBridge, Reply};
use proptest::prelude::*;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::Story;

/// A time of the real clock, in 2026.
const START: u64 = 1_790_000_000;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

/// What a model says: nothing, a talk, a quest, or plain words.
const MODEL_ANSWERS: [Option<&str>; 4] = [
    None,
    Some(r#"{"say": "Well met.", "trust": 3}"#),
    Some(
        r#"{"title": "A Task", "text": "Go.", "steps": [{"goal": "visit", "place": "Goldshire"}, {"goal": "meet", "npc": "Hogger"}]}"#,
    ),
    Some("Our hero walks on."),
];

#[derive(Clone, Debug)]
enum Play {
    Zone(String, Option<String>),
    Meet(String),
    Slap(String),
    Defeat(String),
    Die(Option<String>),
    Level(u8),
    Emote(String),
    Read(String, String),
    Talk(String, String),
    Lore(String),
    Journal(u32),
    Quest(String),
    Accept,
    Decline,
    HeroAdd(String),
    Clock(i64),
}

fn name() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => prop::sample::select(NAMES.to_vec()).prop_map(String::from),
        1 => "[A-Za-z' ]{1,12}",
        1 => any::<String>(),
    ]
}

fn words() -> impl Strategy<Value = String> {
    prop_oneof![
        2 => prop::sample::select(WORDS.to_vec()).prop_map(String::from),
        1 => any::<String>(),
        1 => "[a-z ]{250,260}",
    ]
}

fn play() -> impl Strategy<Value = Play> {
    prop_oneof![
        (name(), prop::option::of(name())).prop_map(|(zone, sub)| Play::Zone(zone, sub)),
        name().prop_map(Play::Meet),
        name().prop_map(Play::Slap),
        name().prop_map(Play::Defeat),
        prop::option::of(name()).prop_map(Play::Die),
        any::<u8>().prop_map(Play::Level),
        "[a-z]{1,8}".prop_map(Play::Emote),
        (name(), words()).prop_map(|(title, text)| Play::Read(title, text)),
        (name(), words()).prop_map(|(npc, text)| Play::Talk(npc, text)),
        words().prop_map(Play::Lore),
        prop::sample::select(PAGES.to_vec()).prop_map(Play::Journal),
        name().prop_map(Play::Quest),
        Just(Play::Accept),
        Just(Play::Decline),
        words().prop_map(Play::HeroAdd),
        prop::sample::select(CLOCK_STEPS.to_vec()).prop_map(Play::Clock),
    ]
}

/// The line of the addon for this play, as `Inputs.lua` writes it.
fn addon_line(play: &Play, at: u64) -> Option<Value> {
    Some(match play {
        Play::Zone(zone, subzone) => {
            json!({"type": "zone_entered", "at": at, "zone": zone, "subzone": subzone})
        }
        Play::Meet(name) => json!({"type": "npc_met", "at": at, "name": name}),
        Play::Slap(name) => json!({"type": "npc_slapped", "at": at, "name": name}),
        Play::Defeat(name) => json!({"type": "npc_defeated", "at": at, "name": name}),
        Play::Die(killer) => json!({"type": "died", "at": at, "killer": killer}),
        Play::Level(level) => json!({"type": "level_reached", "at": at, "level": level}),
        Play::Emote(emote) => json!({"type": "emote_done", "at": at, "emote": emote, "hour": 3}),
        Play::Read(title, text) => json!({"type": "text_seen", "at": at, "kind": "book",
            "title": title, "zone": "Goldshire", "text": text}),
        Play::Talk(npc, text) => json!({"type": "talk_asked", "at": at, "npc": npc, "text": text}),
        Play::Lore(question) => json!({"type": "lore_asked", "at": at, "question": question}),
        Play::Journal(page) => json!({"type": "journal_asked", "page": page}),
        Play::Quest(npc) => json!({"type": "quest_asked", "at": at, "npc": npc}),
        Play::Accept => json!({"type": "quest_accepted", "at": at}),
        Play::Decline => json!({"type": "quest_declined", "at": at}),
        Play::HeroAdd(text) => json!({"type": "hero_added", "at": at, "text": text}),
        Play::Clock(_) => return None,
    })
}

fn story() -> Story {
    static CASE: AtomicUsize = AtomicUsize::new(0);
    let case = CASE.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("through-{case}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

fn bridge(answers: Vec<usize>) -> FakeBridge {
    let mut next = answers.into_iter().cycle();
    let model = Box::new(move |_: &str| MODEL_ANSWERS[next.next()?].map(String::from));
    FakeBridge::new(story()).with_model(model)
}

fn batch(bridge: &mut FakeBridge, line: &Value) -> Reply {
    bridge.batch(&format!("{CHARACTER}\n{line}"))
}

/// Moves the clock of the addon. It never goes below 0 or past `u64::MAX`.
fn step(clock: u64, seconds: i64) -> u64 {
    clock.saturating_add_signed(seconds)
}

fn place_names(reply: &Reply) -> Vec<String> {
    let Reply::Done(text) = reply else {
        panic!("expected a journal, got {reply:?}");
    };
    let page: Value = serde_json::from_str(text).unwrap();
    let places = page["places"].as_array().cloned().unwrap_or_default();
    places
        .iter()
        .filter_map(|place| place["name"].as_str().map(String::from))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn every_batch_gets_one_answer_that_the_bridge_takes_and_the_world_stays_open(
        plays in prop::collection::vec(play(), 0..40),
        answers in prop::collection::vec(0..MODEL_ANSWERS.len(), 1..4),
    ) {
        let mut bridge = bridge(answers);
        let mut clock = START;
        for play in &plays {
            clock = match play {
                Play::Clock(seconds) => step(clock, *seconds),
                _ => step(clock, 1),
            };
            if let Some(line) = addon_line(play, clock) {
                prop_assert!(matches!(batch(&mut bridge, &line), Reply::Done(_)));
            }
        }

        let now = START + 3600 * 24;
        let zone = json!({"type": "zone_entered", "at": now, "zone": "Newvale", "subzone": null});
        batch(&mut bridge, &zone);
        let journal = batch(&mut bridge, &json!({"type": "journal_asked", "page": 0}));
        prop_assert!(place_names(&journal).contains(&"Newvale".to_string()));
    }
}

/// A game event that adds an entry to the journal.
fn entry() -> impl Strategy<Value = Play> {
    prop_oneof![
        (name(), prop::option::of(name())).prop_map(|(zone, sub)| Play::Zone(zone, sub)),
        name().prop_map(Play::Meet),
        name().prop_map(Play::Defeat),
        (name(), words()).prop_map(|(title, text)| Play::Read(title, text)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn every_page_of_a_long_journal_fits_the_game(
        plays in prop::collection::vec(entry(), 300..700),
    ) {
        let mut bridge = bridge(vec![0]);
        let mut clock = START;
        for play in &plays {
            clock += 1;
            if let Some(line) = addon_line(play, clock) {
                batch(&mut bridge, &line);
            }
        }

        let first = batch(&mut bridge, &json!({"type": "journal_asked", "page": 0}));
        let Reply::Done(text) = first else {
            panic!("expected a journal, got {first:?}");
        };
        let pages = serde_json::from_str::<Value>(&text).unwrap()["pages"].as_u64().unwrap();
        for page in 1..pages {
            let reply = batch(&mut bridge, &json!({"type": "journal_asked", "page": page}));
            prop_assert!(matches!(reply, Reply::Done(_)));
        }
    }
}
