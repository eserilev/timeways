//! Any play through the real checks of the bridge (relay SPEC.md 9.8). Every batch gets one
//! answer that the bridge takes, and the world stays open for the next event.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::edges::{CLOCK_STEPS, NAMES, PAGES, WORDS};
use fake_bridge::{FakeBridge, Reply};
use proptest::prelude::*;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::stories::is_good_story;
use timeways_story::story::Story;

/// A time of the real clock, in 2026.
const START: u64 = 1_790_000_000;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

/// What a model says: nothing, a talk, three quests, or plain words.
const MODEL_ANSWERS: [Option<&str>; 6] = [
    None,
    Some(r#"{"say": "Well met.", "trust": 3}"#),
    Some(
        r#"{"title": "A Task", "genre": "errand", "text": "I need you to go.", "steps": [{"goal": "visit", "place": "Goldshire"}, {"goal": "meet", "npc": "Hogger"}]}"#,
    ),
    Some(
        r#"{"title": "Hogger Twice", "genre": "errand", "text": "I need you to go.", "steps": [{"goal": "meet", "npc": "Hogger"}, {"goal": "meet", "npc": "Hogger"}]}"#,
    ),
    Some(
        r#"{"title": "The Hunt", "genre": "errand", "text": "I need you to go.", "steps": [{"goal": "kill", "creature": "Hogger", "count": 2}]}"#,
    ),
    Some("$N walks on."),
];

#[derive(Clone, Debug)]
enum Play {
    Zone(String, Option<String>, Value),
    Meet(String, Value),
    /// A sighting: `true` for a hostile NPC.
    See(String, bool),
    Kill(String),
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

/// A position as the addon sends it: at the ends of the map, none, or broken.
fn spot() -> impl Strategy<Value = Value> {
    let number = || prop_oneof![Just(0i64), Just(1000), Just(1001), Just(-1), any::<i64>()];
    prop_oneof![
        Just(Value::Null),
        (number(), number(), number()).prop_map(|(map, x, y)| json!({"map": map, "x": x, "y": y})),
        (1i64..2000, 0i64..=1000, 0i64..=1000)
            .prop_map(|(map, x, y)| json!({"map": map, "x": x, "y": y})),
        Just(json!({"map": 1420, "x": 0.5, "y": "top"})),
    ]
}

fn play() -> impl Strategy<Value = Play> {
    prop_oneof![
        (name(), prop::option::of(name()), spot())
            .prop_map(|(zone, sub, spot)| Play::Zone(zone, sub, spot)),
        (name(), spot()).prop_map(|(name, spot)| Play::Meet(name, spot)),
        (name(), any::<bool>()).prop_map(|(name, hostile)| Play::See(name, hostile)),
        name().prop_map(Play::Kill),
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
        Play::Zone(zone, subzone, spot) => {
            json!({"type": "zone_entered", "at": at, "zone": zone, "subzone": subzone, "spot": spot})
        }
        Play::Meet(name, spot) => json!({"type": "npc_met", "at": at, "name": name, "spot": spot}),
        Play::See(name, hostile) => {
            let reaction = if *hostile { "hostile" } else { "friendly" };
            json!({"type": "npc_seen", "at": at, "name": name, "reaction": reaction,
                "creature": "beast"})
        }
        Play::Kill(name) => json!({"type": "npc_killed", "at": at, "name": name}),
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
        (name(), prop::option::of(name()), spot())
            .prop_map(|(zone, sub, spot)| Play::Zone(zone, sub, spot)),
        (name(), spot()).prop_map(|(name, spot)| Play::Meet(name, spot)),
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
        1 => Just(Play::Zone("Goldshire".to_string(), None, Value::Null)),
        1 => Just(Play::Zone("Elwynn Forest".to_string(), Some("Goldshire".to_string()), Value::Null)),
        1 => giver().prop_map(|npc| Play::Meet(npc, Value::Null)),
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
        r#"{{"title": "Task {n}", "genre": "errand", "text": "I need you to go.", "steps": {}}}"#,
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

/// A bridge whose model answers every call with plain words, and the prompts that it got.
fn saga_bridge(name: &str) -> (FakeBridge, Rc<RefCell<Vec<String>>>) {
    let prompts = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&prompts);
    let model = Box::new(move |prompt: &str| {
        seen.borrow_mut().push(prompt.to_string());
        Some("$N walks on.".to_string())
    });
    (
        FakeBridge::new(story_with_lore(name)).with_model(model),
        prompts,
    )
}

/// A pack from before the builder cut long passages still gives an answer that the bridge
/// takes. Else the batch waits, and the bridge stops the story program as hung.
#[test]
fn a_passage_over_the_limit_of_the_bridge_still_gets_an_answer() {
    let long = "The tower of Testvale fell. ".repeat(200);
    let mut bridge = FakeBridge::new(story_with_passage("through-long.sqlite", &long));
    let zone = json!({"type": "zone_entered", "at": START, "zone": "Testvale", "subzone": null});
    bridge.batch(&format!("{CHARACTER}\n{zone}"));

    let question = json!({"type": "lore_asked", "at": START + 1, "question": "the tower"});
    let reply = bridge.batch(&format!("{CHARACTER}\n{question}"));

    assert!(
        matches!(&reply, Reply::Done(text) if text.contains("Testvale fell")),
        "{reply:?}"
    );
}

/// The addon puts the name back in place of each `$N` of an answer.
#[test]
fn a_seen_text_full_of_the_mark_of_the_name_still_gets_an_answer() {
    let story = story_with_passage("through-seen.sqlite", "The tower of Testvale fell.");
    let mut bridge = FakeBridge::new(story);
    let text = "$N ".repeat(660);
    let seen = json!({"type": "text_seen", "at": START, "kind": "gossip", "npc": "Keeper Tessa",
        "zone": "Testvale", "text": text});
    bridge.batch(&format!("{CHARACTER}\n{seen}"));

    let question = json!({"type": "lore_asked", "at": START + 1, "question": "Tessa"});
    let reply = bridge.batch(&format!("{CHARACTER}\n{question}"));

    assert!(
        matches!(&reply, Reply::Done(text) if text.contains("$N $N")),
        "{reply:?}"
    );
}

/// Two sessions finish the first chapter. The saga of that chapter starts after a quiet
/// batch, and its call stays open.
fn start_saga(bridge: &mut FakeBridge) {
    let zone = |at: u64, zone: &str, subzone: Option<&str>| json!({"type": "zone_entered", "at": at, "zone": zone, "subzone": subzone});
    let met = |at: u64, name: &str| json!({"type": "npc_met", "at": at, "name": name});
    // A meeting and 14 new camps on foot: the least weight of a chapter.
    let mut first_session = vec![
        zone(START, "Testvale", None),
        met(START + 60, "Keeper Tessa"),
    ];
    for n in 1..=14 {
        first_session.push(zone(
            START + 60 + n * 60,
            "Testvale",
            Some(&format!("Camp {n}")),
        ));
    }
    first_session.push(zone(START + 5 * 3600, "Duskwood", None));
    for line in &first_session {
        batch(bridge, line);
    }
    bridge.send(&format!(
        "{CHARACTER}\n{}",
        met(START + 5 * 3600 + 60, "Salma")
    ));
}

fn is_saga(prompt: &str) -> bool {
    prompt.contains(r#"{"saga":"#)
}

/// The narrator stays quiet while a saga is written, so the question gets the other slot.
#[test]
fn a_question_gets_a_model_call_while_a_saga_is_written() {
    let (mut bridge, prompts) = saga_bridge("saga-question");
    start_saga(&mut bridge);

    let zone =
        json!({"type": "zone_entered", "at": START + 5 * 3600 + 120, "zone": "Elwynn Forest"});
    bridge.send(&format!("{CHARACTER}\n{zone}"));
    let question =
        json!({"type": "lore_asked", "at": START + 5 * 3600 + 121, "question": "the tower"});
    let lore = bridge.send(&format!("{CHARACTER}\n{question}"));
    bridge.settle();

    assert_eq!(bridge.refused_calls(), 0);
    assert!(matches!(bridge.reply(lore), Reply::Done(_)));
    assert!(prompts.borrow().iter().any(|prompt| is_saga(prompt)));
}

/// A saga takes one slot. A task request and then a question take the other one in turn,
/// so the bridge fails neither of them.
#[test]
fn a_task_request_and_a_question_both_get_a_call_while_a_saga_is_written() {
    let (mut bridge, prompts) = saga_bridge("saga-task");
    start_saga(&mut bridge);

    let asked = json!({"type": "quest_asked", "at": START + 5 * 3600 + 120, "npc": "Keeper Tessa"});
    let task = bridge.send(&format!("{CHARACTER}\n{asked}"));
    let question =
        json!({"type": "lore_asked", "at": START + 5 * 3600 + 121, "question": "the tower"});
    let lore = bridge.send(&format!("{CHARACTER}\n{question}"));
    bridge.settle();

    assert_eq!(bridge.refused_calls(), 0);
    assert!(matches!(bridge.reply(task), Reply::Done(_)));
    assert!(matches!(bridge.reply(lore), Reply::Done(_)));
    assert!(prompts.borrow().iter().any(|prompt| is_saga(prompt)));
}

/// The relay sends no `batch_end` for a batch that ends with a question, so the refusal
/// goes on the journal.
#[test]
fn a_refused_accept_in_a_batch_with_a_journal_request_shows_on_the_journal() {
    let mut next = 0;
    let model = Box::new(move |_: &str| {
        next += 1;
        // Four quests in a row, each with its own shape and new targets.
        let step = [
            r#"{"goal": "visit", "place": "Mill Pond"}"#,
            r#"{"goal": "meet", "npc": "Farmer Bram"}"#,
            r#"{"goal": "visit", "place": "Old Tower"}, {"goal": "visit", "place": "Mill Pond"}"#,
            r#"{"goal": "talk", "npc": "Farmer Bram"}"#,
        ][(next - 1) % 4];
        Some(format!(
            r#"{{"title": "Task {next}", "genre": "errand", "text": "I need you to go.", "steps": [{step}]}}"#
        ))
    });
    let mut bridge = FakeBridge::new(story()).with_model(model);
    let pond =
        json!({"type": "zone_entered", "at": START, "zone": "Testvale", "subzone": "Mill Pond"});
    let bram = json!({"type": "npc_met", "at": START + 1, "name": "Farmer Bram"});
    let tower = json!({"type": "zone_entered", "at": START + 2, "zone": "Testvale", "subzone": "Old Tower"});
    for line in [pond, bram, tower] {
        batch(&mut bridge, &line);
    }
    for (n, giver) in (0u64..).zip(["Keeper Tessa", "Innkeeper Pell", "Guard Rolf", "Smith Hana"]) {
        batch(
            &mut bridge,
            &json!({"type": "npc_met", "at": START + 10 + n, "name": giver}),
        );
        batch(
            &mut bridge,
            &json!({"type": "quest_asked", "at": START + 20 + n, "npc": giver}),
        );
    }
    for n in 1..=3 {
        batch(
            &mut bridge,
            &json!({"type": "quest_accepted", "at": START + 30 + n, "number": n}),
        );
    }

    let accepted = json!({"type": "quest_accepted", "at": START + 40, "number": 4});
    let asked = json!({"type": "journal_asked", "page": 0});
    let reply = bridge.batch(&format!("{CHARACTER}\n{accepted}\n{asked}"));

    let Reply::Done(text) = reply else {
        panic!("expected a journal, got {reply:?}");
    };
    let page: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        page["notice"],
        "You already have 3 quests. Finish one first."
    );
}

/// The relay sends no `batch_end` for a batch that ends with a question, so the offer comes
/// with the next answer, here a second journal that already lists it.
#[test]
fn a_task_asked_in_a_batch_with_a_journal_request_comes_with_the_next_answer() {
    let model = Box::new(|_: &str| {
        Some(r#"{"title": "The Lost Lantern", "genre": "errand", "text": "I lost it. Find it.", "steps": [{"goal": "visit", "place": "Mill Pond"}]}"#.to_string())
    });
    let mut bridge = FakeBridge::new(story()).with_model(model);
    let pond =
        json!({"type": "zone_entered", "at": START, "zone": "Testvale", "subzone": "Mill Pond"});
    let tessa = json!({"type": "npc_met", "at": START + 1, "name": "Keeper Tessa"});
    let tower = json!({"type": "zone_entered", "at": START + 2, "zone": "Testvale", "subzone": "Old Tower"});
    for line in [pond, tessa, tower] {
        batch(&mut bridge, &line);
    }

    let asked = json!({"type": "quest_asked", "at": START + 10, "npc": "Keeper Tessa"});
    let journal = json!({"type": "journal_asked", "page": 0});
    bridge.batch(&format!("{CHARACTER}\n{asked}\n{journal}"));
    let next = batch(&mut bridge, &journal);

    let Reply::Done(text) = &next else {
        panic!("expected a journal, got {next:?}");
    };
    let page: Value = serde_json::from_str(text).unwrap();
    assert_eq!(
        page["notice"],
        "Keeper Tessa has a quest for you: The Lost Lantern. I lost it. Find it. Type /quest accept."
    );
    assert_eq!(givers(&list(&next, "quests"), "offered"), ["Keeper Tessa"]);
}

fn story_with_lore(name: &str) -> Story {
    story_with_passage(
        &format!("through-lore-{name}.sqlite"),
        "The tower of Testvale fell.",
    )
}

fn story_with_passage(file: &str, text: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(file);
    let _ = std::fs::remove_file(&path);
    let tower = timeways_story::pack::Passage {
        text: text.to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![timeways_story::pack::Link::Place("Testvale".to_string())],
        origin: timeways_story::pack::Origin::Pack,
        about: None,
        depends_on: Vec::new(),
        setup_for: None,
    };
    Pack::write(&path, &[tower]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

fn story_line(number: u64, title: Option<&str>, paragraphs: &[String]) -> Value {
    json!({"type": "story_accepted", "at": START, "number": number, "title": title,
           "paragraphs": paragraphs})
}

/// The numbers of the stories that the journal shows, from every page.
fn shelf_numbers(bridge: &mut FakeBridge) -> Vec<u64> {
    let mut numbers = Vec::new();
    let mut page = 0;
    loop {
        let reply = batch(bridge, &json!({"type": "journal_asked", "page": page}));
        let Reply::Done(text) = &reply else {
            panic!("expected a journal, got {reply:?}");
        };
        let value: Value = serde_json::from_str(text).unwrap();
        let stories = list(&reply, "stories");
        numbers.extend(stories.iter().filter_map(|story| story["number"].as_u64()));
        page += 1;
        if page >= value["pages"].as_u64().unwrap() {
            return numbers;
        }
    }
}

#[test]
fn a_story_accepted_line_with_paragraphs_passes_the_bridge() {
    let mut bridge = bridge(vec![0]);
    let paragraphs = ["We went in.".to_string(), "We came out.".to_string()];
    let line = story_line(7, Some("The Barn"), &paragraphs);

    let text = format!("{CHARACTER}\n{line}");

    assert_eq!(fake_bridge::dropped_lines_of(&text), Some(0));
    bridge.batch(&text);
    assert_eq!(shelf_numbers(&mut bridge), [7]);
}

/// The reason why the body is a list: the bridge drops a line with a line break in a
/// string (relay SPEC 9.8).
#[test]
fn a_story_accepted_line_with_a_line_break_is_dropped_by_the_bridge() {
    let line = story_line(7, None, &["We went in.\nWe came out.".to_string()]);

    let dropped = fake_bridge::dropped_lines_of(&format!("{CHARACTER}\n{line}"));

    assert_eq!(dropped, Some(1));
}

/// A text at the edges of what the journal escapes: quotes and backslashes double in
/// JSON, and a letter of 4 bytes is the most bytes for one letter.
fn story_text() -> impl Strategy<Value = String> {
    let edge = |unit: &'static str| {
        prop_oneof![
            Just(0usize),
            Just(1),
            Just(250),
            Just(500),
            Just(1000),
            0usize..1100
        ]
        .prop_map(move |count| unit.repeat(count))
    };
    prop_oneof![
        edge("\""),
        edge("\\"),
        edge("\u{1F409}"),
        edge("a"),
        "[a-z ]{1,80}",
        Just(" leading".to_string()),
        Just("broken\nline".to_string()),
        Just("c1\u{85}".to_string()),
    ]
}

fn paragraph_count() -> impl Strategy<Value = usize> {
    prop_oneof![Just(0usize), Just(1), Just(20), Just(21), 0usize..22]
}

fn told_story() -> impl Strategy<Value = (Option<String>, Vec<String>)> {
    let title = prop::option::of(prop_oneof![story_text(), "[A-Za-z ]{0,61}"]);
    let paragraphs = (paragraph_count(), story_text()).prop_flat_map(|(count, first)| {
        prop::collection::vec(prop_oneof![Just(first), story_text()], count)
    });
    (title, paragraphs)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// GAMEPLAY.md 4.8: a story that the bridge passes lands, or the story program refuses
    /// it for its shape. Either way the journal keeps fitting the game.
    #[test]
    fn a_story_line_that_the_bridge_passes_lands_or_is_refused_with_its_reason(
        stories in prop::collection::vec(told_story(), 1..6),
    ) {
        let mut bridge = bridge(vec![0]);
        let mut expected = Vec::new();
        for (number, (title, paragraphs)) in (0u64..).zip(&stories) {
            let line = story_line(number, title.as_deref(), paragraphs);
            let text = format!("{CHARACTER}\n{line}");
            if fake_bridge::dropped_lines_of(&text) != Some(0) {
                continue;
            }
            bridge.batch(&text);
            let title = title.as_deref().filter(|title| !title.is_empty());
            if is_good_story(title, paragraphs) {
                expected.push(number);
            }
        }

        prop_assert_eq!(shelf_numbers(&mut bridge), expected);
    }

    /// Each accepted story fits a page of the journal, at the edges of its limits.
    #[test]
    fn every_accepted_story_fits_one_page_of_the_journal(
        (title, paragraphs) in told_story(),
    ) {
        let mut bridge = bridge(vec![0]);
        let line = story_line(1, title.as_deref(), &paragraphs);

        bridge.batch(&format!("{CHARACTER}\n{line}"));

        let reply = batch(&mut bridge, &json!({"type": "journal_asked", "page": 0}));
        prop_assert!(matches!(reply, Reply::Done(_)), "{:?}", reply);
    }
}
