//! The texts of the tales and the history of each zone (docs/plans/chapters.md 6 and 10),
//! through the story program with a fake model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::journal::Journal;
use timeways_story::pack::Pack;
use timeways_story::places::InstanceKind;
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const HOUR: u64 = 3600;

const TALE: &str = r#"{"tale": "The Defias dug a fleet out of the rock below Moonbrook. Edwin VanCleef fell on its deck."}"#;
const HISTORY: &str =
    r#"{"history": "Westfall remembers the burned farms and the militia of Sentinel Hill."}"#;

fn fresh(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("tales-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn started(name: &str) -> Story {
    let folder = fresh(name);
    let pack = folder.join("pack.sqlite");
    Pack::write(&pack, &[]).unwrap();
    let mut story = Story::new(Pack::open(&pack).unwrap(), Store::Folder(folder));
    let character = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    story.handle(character).unwrap();
    story
}

fn enter(story: &mut Story, at: u64, zone: &str, subzone: Option<&str>) {
    let input = Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: subzone.map(str::to_string),
        spot: None,
        hour: None,
        taxi: None,
    };
    story.handle(input).unwrap();
}

fn meet(story: &mut Story, at: u64, name: &str) {
    let input = Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
        spot: None,
    };
    story.handle(input).unwrap();
}

fn defeat(story: &mut Story, at: u64, name: &str) {
    let input = Input::NpcDefeated {
        at: Tick(at),
        name: name.to_string(),
        kind: None,
    };
    story.handle(input).unwrap();
}

/// A run of the Deadmines from `at`, with a kill of each boss.
fn run(story: &mut Story, at: u64, bosses: &[&str]) {
    enter(story, at, "The Deadmines", None);
    let entered = Input::InstanceEntered {
        at: Tick(at),
        zone: "The Deadmines".to_string(),
        kind: InstanceKind::Dungeon,
    };
    story.handle(entered).unwrap();
    for (n, boss) in bosses.iter().enumerate() {
        defeat(story, at + 60 * (n as u64 + 1), boss);
    }
}

/// The prompt kinds of the fake model.
fn kind(prompt: &str) -> &'static str {
    if prompt.contains("Write the tale of") {
        "tale"
    } else if prompt.contains("Write the history of") {
        "history"
    } else if prompt.contains("Write who this character has become") {
        "summary"
    } else if prompt.contains(r#"{"saga":"#) {
        "saga"
    } else {
        "other"
    }
}

fn calls(outputs: Vec<Output>) -> Vec<(CallId, String)> {
    outputs
        .into_iter()
        .filter_map(|output| match output {
            Output::ModelCall { call, prompt } => Some((call, prompt)),
            _ => None,
        })
        .collect()
}

/// Ends a batch and answers every call that follows, with `answer` for each kind, or with
/// nothing. Returns the prompts of the calls, in order.
fn end_batch(
    story: &mut Story,
    batch: u64,
    answer: &dyn Fn(&str) -> Option<String>,
) -> Vec<String> {
    let mut open = calls(
        story
            .handle(Input::BatchEnd {
                id: MessageId(batch),
            })
            .unwrap(),
    );
    let mut prompts = Vec::new();
    while let Some((call, prompt)) = open.pop() {
        prompts.push(prompt.clone());
        // A refusal, not a failure: a failed call makes the pace window tight, and then the
        // tales and the summary wait.
        let text = answer(kind(&prompt)).unwrap_or_default();
        let outputs = story.handle(Input::ModelAnswered { call, text });
        open.extend(calls(outputs.unwrap()));
    }
    prompts
}

/// Batches end, half an hour of play apart, so the pace window stays calm. Returns the
/// prompts of all of them.
fn settle(story: &mut Story, first: u64, answer: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let mut prompts = Vec::new();
    for batch in first..first + 8 {
        let later = Input::HourChanged {
            at: Tick(20 * HOUR + batch * 1800),
            hour: 12,
        };
        story.handle(later).unwrap();
        prompts.extend(end_batch(story, batch, answer));
    }
    prompts
}

fn journal(story: &mut Story) -> Journal {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(70),
            page: 0,
        })
        .unwrap();
    match outputs.into_iter().next() {
        Some(Output::Journal { page, .. }) => page.journal,
        other => panic!("expected a journal, got {other:?}"),
    }
}

fn tale_answer(kind: &str) -> Option<String> {
    (kind == "tale").then(|| TALE.to_string())
}

#[test]
fn a_run_with_something_new_asks_for_its_tale_once_the_run_closes() {
    let mut story = started("run-closes");
    enter(&mut story, HOUR, "Westfall", None);
    run(&mut story, 2 * HOUR, &["Edwin VanCleef"]);
    let before = settle(&mut story, 1, &tale_answer);

    enter(&mut story, 3 * HOUR, "Westfall", None);
    let after = settle(&mut story, 20, &tale_answer);

    assert!(!before.iter().any(|prompt| kind(prompt) == "tale"));
    let tales: Vec<&String> = after
        .iter()
        .filter(|prompt| kind(prompt) == "tale")
        .collect();
    assert_eq!(tales.len(), 1);
    assert!(
        tales[0].contains("The Deadmines (a dungeon)"),
        "{}",
        tales[0]
    );
    assert!(
        tales[0].contains("Defeated Edwin VanCleef, a first kill"),
        "{}",
        tales[0]
    );
    let journal = journal(&mut story);
    assert_eq!(journal.tales[0].runs, 1);
    assert!(
        journal.tales[0]
            .text
            .as_deref()
            .is_some_and(|text| text.contains("VanCleef"))
    );
}

#[test]
fn a_second_run_with_nothing_new_costs_no_call_and_changes_only_its_count() {
    let mut story = started("second-run");
    run(&mut story, HOUR, &["Edwin VanCleef"]);
    enter(&mut story, 2 * HOUR, "Westfall", None);
    settle(&mut story, 1, &tale_answer);

    run(&mut story, 3 * HOUR, &["Edwin VanCleef"]);
    enter(&mut story, 4 * HOUR, "Westfall", None);
    let prompts = settle(&mut story, 20, &tale_answer);

    assert!(!prompts.iter().any(|prompt| kind(prompt) == "tale"));
    let journal = journal(&mut story);
    assert_eq!(journal.tales[0].runs, 2);
    assert_eq!(
        journal.tales[0].again,
        ["Defeated Edwin VanCleef again, 2 times."]
    );
}

#[test]
fn a_refused_tale_keeps_the_text_before() {
    let mut story = started("tale-refused");
    run(&mut story, HOUR, &["Edwin VanCleef"]);
    enter(&mut story, 2 * HOUR, "Westfall", None);
    settle(&mut story, 1, &tale_answer);

    run(&mut story, 3 * HOUR, &["Cookie"]);
    enter(&mut story, 4 * HOUR, "Westfall", None);
    let arrival =
        |kind: &str| (kind == "tale").then(|| r#"{"tale": "$N went into the mine."}"#.to_string());
    let prompts = settle(&mut story, 20, &arrival);

    let tale = prompts
        .iter()
        .find(|prompt| kind(prompt) == "tale")
        .expect("a tale call");
    assert!(
        tale.contains("The newest run:\n<<<\n- Defeated Cookie, a first kill"),
        "{tale}"
    );
    assert!(tale.contains("Edwin VanCleef fell on its deck"), "{tale}");
    let journal = journal(&mut story);
    assert!(
        journal.tales[0]
            .text
            .as_deref()
            .is_some_and(|text| text.contains("VanCleef fell"))
    );
}

fn learn_player(story: &mut Story, name: &str) {
    let in_sight = Input::PlayerDescribed {
        at: Tick(HOUR / 2),
        name: name.to_string(),
        race: None,
        class: None,
    };
    story.handle(in_sight).unwrap();
}

fn tale_text(text: &'static str) -> impl Fn(&str) -> Option<String> {
    move |kind: &str| (kind == "tale").then(|| serde_json::json!({ "tale": text }).to_string())
}

#[test]
fn a_tale_that_names_a_player_by_id_shows_the_name() {
    let mut story = started("tale-with-id");
    learn_player(&mut story, "Ada");
    run(&mut story, HOUR, &["Edwin VanCleef"]);
    enter(&mut story, 2 * HOUR, "Westfall", None);
    let with_id =
        tale_text("The Defias dug a fleet out of the rock. {P1} saw Edwin VanCleef fall.");

    settle(&mut story, 1, &with_id);

    assert_eq!(
        journal(&mut story).tales[0].text.as_deref(),
        Some("The Defias dug a fleet out of the rock. Ada saw Edwin VanCleef fall.")
    );
}

#[test]
fn a_tale_that_names_an_unknown_player_is_refused() {
    let mut story = started("tale-with-unknown-id");
    run(&mut story, HOUR, &["Edwin VanCleef"]);
    enter(&mut story, 2 * HOUR, "Westfall", None);
    let with_id =
        tale_text("The Defias dug a fleet out of the rock. {P1} saw Edwin VanCleef fall.");

    settle(&mut story, 1, &with_id);

    assert_eq!(journal(&mut story).tales[0].text, None);
}

/// A chapter in Westfall: a meeting and 14 camps on foot. Then a meeting in Duskwood
/// closes it.
fn a_chapter_in_westfall(story: &mut Story) {
    enter(story, HOUR, "Westfall", None);
    meet(story, HOUR, "Gryan Stoutmantle");
    for n in 1..=14 {
        enter(story, HOUR + n * 60, "Westfall", Some(&format!("Camp {n}")));
    }
    enter(story, 5 * HOUR, "Duskwood", None);
    meet(story, 5 * HOUR, "Madame Eva");
}

#[test]
fn the_history_of_a_zone_comes_after_the_summary_of_its_chapter() {
    let mut story = started("history");
    a_chapter_in_westfall(&mut story);
    let answer = |kind: &str| match kind {
        "history" => Some(HISTORY.to_string()),
        "summary" => Some(r#"{"summary": "Westfall knows the face of $N."}"#.to_string()),
        _ => None,
    };

    let prompts = settle(&mut story, 1, &answer);

    let kinds: Vec<&str> = prompts
        .iter()
        .map(|prompt| kind(prompt))
        .filter(|kind| *kind != "other")
        .collect();
    let summary = kinds
        .iter()
        .position(|kind| *kind == "summary")
        .expect("a summary");
    let history = kinds
        .iter()
        .position(|kind| *kind == "history")
        .expect("a history");
    assert!(summary < history, "{kinds:?}");
    let journal = journal(&mut story);
    assert_eq!(journal.histories.len(), 1);
    assert_eq!(journal.histories[0].zone, "Westfall");
}

#[test]
fn a_refused_history_gets_one_retry_with_its_reasons() {
    let mut story = started("history-retry");
    a_chapter_in_westfall(&mut story);
    let answer = |kind: &str| match kind {
        "history" => Some(r#"{"history": "$N came to Westfall."}"#.to_string()),
        _ => None,
    };

    let prompts = settle(&mut story, 1, &answer);

    let histories: Vec<&String> = prompts
        .iter()
        .filter(|prompt| kind(prompt) == "history")
        .collect();
    assert_eq!(histories.len(), 2);
    assert!(
        histories[1].contains("It only tells that the hero came"),
        "{}",
        histories[1]
    );
    assert!(journal(&mut story).histories.is_empty());
}
