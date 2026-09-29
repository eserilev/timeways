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

/// What a model says: nothing, a talk, two quests, or plain words.
const MODEL_ANSWERS: [Option<&str>; 5] = [
    None,
    Some(r#"{"say": "Well met.", "trust": 3}"#),
    Some(
        r#"{"title": "A Task", "text": "Go.", "steps": [{"goal": "visit", "place": "Goldshire"}, {"goal": "meet", "npc": "Hogger"}]}"#,
    ),
    Some(
        r#"{"title": "Hogger Twice", "text": "Go.", "steps": [{"goal": "meet", "npc": "Hogger"}, {"goal": "meet", "npc": "Hogger"}]}"#,
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
    /// `/quest`, and the model call stays open until `Settle`.
    AskLater(String),
    Settle,
    Accept(Option<u64>),
    Decline(Option<u64>),
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
        prop::option::of(1u64..5).prop_map(Play::Accept),
        prop::option::of(1u64..5).prop_map(Play::Decline),
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
        Play::Quest(npc) | Play::AskLater(npc) => {
            json!({"type": "quest_asked", "at": at, "npc": npc})
        }
        Play::Accept(number) => json!({"type": "quest_accepted", "at": at, "number": number}),
        Play::Decline(number) => json!({"type": "quest_declined", "at": at, "number": number}),
        Play::HeroAdd(text) => json!({"type": "hero_added", "at": at, "text": text}),
        Play::Clock(_) | Play::Settle => return None,
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

fn list(reply: &Reply, name: &str) -> Vec<Value> {
    let Reply::Done(text) = reply else {
        panic!("expected a journal, got {reply:?}");
    };
    let page: Value = serde_json::from_str(text).unwrap();
    page[name].as_array().cloned().unwrap_or_default()
}

fn place_names(reply: &Reply) -> Vec<String> {
    let places = list(reply, "places");
    places
        .iter()
        .filter_map(|place| place["name"].as_str().map(String::from))
        .collect()
}

/// The givers of the quests with this status, one entry for each quest.
fn givers(quests: &[Value], status: &str) -> Vec<String> {
    let with_status = quests.iter().filter(|quest| quest["status"] == status);
    with_status
        .filter_map(|quest| quest["giver"].as_str().map(String::from))
        .collect()
}

fn has_repeats(names: &[String]) -> bool {
    let mut sorted = names.to_vec();
    sorted.sort();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
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
        let quests = list(&journal, "quests");
        let open = givers(&quests, "accepted");
        prop_assert!(open.len() <= 3, "{quests:?}");
        prop_assert!(!has_repeats(&open), "{quests:?}");
        prop_assert!(!has_repeats(&givers(&quests, "offered")), "{quests:?}");
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

const GIVERS: [&str; 5] = [
    "Keeper Tessa",
    "Innkeeper Pell",
    "Guard Rolf",
    "Smith Hana",
    "Hogger",
];

/// Play around quests: few givers, and the places and NPCs of the steps of `QUESTS`.
fn quest_play() -> impl Strategy<Value = Play> {
    let giver = || prop::sample::select(GIVERS.to_vec()).prop_map(String::from);
    prop_oneof![
        2 => giver().prop_map(Play::Quest),
        2 => giver().prop_map(Play::AskLater),
        1 => Just(Play::Settle),
        2 => prop::option::of(1u64..8).prop_map(Play::Accept),
        1 => prop::option::of(1u64..8).prop_map(Play::Decline),
        1 => Just(Play::Zone("Goldshire".to_string(), None)),
        1 => Just(Play::Zone("Elwynn Forest".to_string(), Some("Goldshire".to_string()))),
        1 => giver().prop_map(Play::Meet),
        1 => giver().prop_map(|npc| Play::Talk(npc, "any news?".to_string())),
    ]
}

/// Quests whose steps name the places and NPCs of `quest_play`, with other titles.
fn quest_answer(n: usize) -> String {
    let steps = [
        r#"[{"goal": "visit", "place": "Goldshire"}]"#,
        r#"[{"goal": "meet", "npc": "Hogger"}, {"goal": "visit", "place": "Elwynn Forest"}]"#,
        r#"[{"goal": "meet", "npc": "Guard Rolf"}, {"goal": "meet", "npc": "Smith Hana"}]"#,
    ];
    format!(
        r#"{{"title": "Task {n}", "text": "Go.", "steps": {}}}"#,
        steps[n % steps.len()]
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn the_quest_limits_hold_for_any_play(plays in prop::collection::vec(quest_play(), 1..60)) {
        let mut next = 0;
        let model = Box::new(move |_: &str| {
            next += 1;
            Some(quest_answer(next))
        });
        let mut bridge = FakeBridge::new(story()).with_model(model);
        let mut clock = START;
        for play in &plays {
            clock += 60;
            match (play, addon_line(play, clock)) {
                (Play::Settle, _) => bridge.settle(),
                (Play::AskLater(_), Some(line)) => {
                    bridge.send(&format!("{CHARACTER}\n{line}"));
                }
                (_, Some(line)) => {
                    batch(&mut bridge, &line);
                }
                (_, None) => {}
            }

            // A journal needs no model call, so its reply comes at once, and the open calls
            // stay open.
            let asked = json!({"type": "journal_asked", "page": 0});
            let id = bridge.send(&format!("{CHARACTER}\n{asked}"));
            let journal = bridge.reply(id);
            let quests = list(&journal, "quests");
            let open = givers(&quests, "accepted");
            prop_assert!(open.len() <= 3, "{quests:?}");
            prop_assert!(!has_repeats(&open), "{quests:?}");
            prop_assert!(!has_repeats(&givers(&quests, "offered")), "{quests:?}");
        }
    }
}

/// The bard, the narrator, and a question at once. The relay runs 2 model calls of the
/// story program, so the story program keeps one slot for the player.
#[test]
fn a_question_gets_a_model_call_while_the_bard_writes() {
    let pack = story_with_lore();
    let model = Box::new(|_: &str| Some("Our hero walks on.".to_string()));
    let mut bridge = FakeBridge::new(pack).with_model(model);
    let event = |at: u64, zone: &str| json!({"type": "zone_entered", "at": at, "zone": zone, "subzone": null});

    // Two sessions, and each narrator call ends.
    bridge.batch(&format!("{CHARACTER}\n{}", event(START, "Testvale")));
    bridge.batch(&format!(
        "{CHARACTER}\n{}",
        event(START + 7200, "Goldshire")
    ));
    // A quiet batch starts the saga of the first session, and the bard is slow.
    bridge.send(&format!("{CHARACTER}\n{}", event(START + 7250, "Testvale")));
    // A new zone wants the narrator, and then the player asks.
    bridge.send(&format!(
        "{CHARACTER}\n{}",
        event(START + 7300, "Elwynn Forest")
    ));
    let question = json!({"type": "lore_asked", "at": START + 7301, "question": "the tower"});
    bridge.send(&format!("{CHARACTER}\n{question}"));

    assert_eq!(bridge.refused_calls(), 0);
}

fn story_with_lore() -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("through-lore.sqlite");
    let _ = std::fs::remove_file(&path);
    let tower = timeways_story::pack::Passage {
        text: "The tower of Testvale fell.".to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![timeways_story::pack::Link::Place("Testvale".to_string())],
        origin: timeways_story::pack::Origin::Pack,
    };
    Pack::write(&path, &[tower]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}
