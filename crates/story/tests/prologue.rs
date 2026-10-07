//! The prologue of the Chronicle (GAMEPLAY.md 3.3), through the story program with a fake
//! model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::journal::{Chapter, OpenedBy};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::past::{Gear, Past, Standing};
use timeways_story::race_class::{Class, Race};
use timeways_story::store::Store;
use timeways_story::story::{Output, PastWanted, Story};

const PROLOGUE: &str = r#"{"prologue": "The Defias Brotherhood took the farms of Westfall from Stormwind, and the People's Militia still holds Sentinel Hill against it. $N finished the work of the militia there."}"#;

const WESTFALL: &str = "Westfall was the breadbasket of Stormwind until the Defias Brotherhood \
took its farms, and the People's Militia holds Sentinel Hill against them now.";

fn fresh(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("prologue-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn westfall() -> Passage {
    Passage {
        text: WESTFALL.to_string(),
        source: "the wiki page \"Westfall\"".to_string(),
        links: vec![Link::Place("Westfall".to_string())],
        origin: Origin::Pack,
        about: Some("Westfall".to_string()),
        depends_on: Vec::new(),
        setup_for: None,
    }
}

fn started(folder: &Path) -> Story {
    let pack = folder.join("pack.sqlite");
    if !pack.exists() {
        Pack::write(&pack, &[westfall()]).unwrap();
    }
    let mut story = Story::new(
        Pack::open(&pack).unwrap(),
        Store::Folder(folder.to_path_buf()),
    );
    let character = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    story.handle(character).unwrap();
    story
}

/// The first login: the race, the class, the level, and the zone.
fn log_in(story: &mut Story, level: u8) {
    let describe = Input::CharacterDescribed {
        at: Tick(10),
        race: Race::Human,
        class: Class::Paladin,
    };
    story.handle(describe).unwrap();
    story
        .handle(Input::LevelReached {
            at: Tick(10),
            level,
        })
        .unwrap();
    enter(story, 11, "Westfall", None);
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

fn long_past(at: u64) -> Past {
    Past {
        at: Tick(at),
        level: 35,
        zone: Some("Westfall".to_string()),
        played: Some(400_000),
        quests: 212,
        quest_titles: vec!["The Defias Brotherhood".to_string()],
        zones: vec!["Elwynn Forest".to_string(), "Westfall".to_string()],
        factions: vec![Standing {
            name: "Stormwind".to_string(),
            standing: 6,
        }],
        professions: Vec::new(),
        mounts: vec!["Brown Horse".to_string()],
        gear: Gear { rare: 3, epic: 0 },
    }
}

fn short_past(at: u64) -> Past {
    Past {
        level: 1,
        quests: 0,
        quest_titles: Vec::new(),
        zones: Vec::new(),
        ..long_past(at)
    }
}

fn read_past(story: &mut Story, past: Past) {
    story.handle(Input::PastRead(past)).unwrap();
}

fn calls(outputs: &[Output]) -> Vec<(CallId, String)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            Output::ModelCall { call, prompt } => Some((*call, prompt.clone())),
            _ => None,
        })
        .collect()
}

fn notices(outputs: &[Output]) -> Vec<String> {
    let notice = |output: &Output| match output {
        Output::EventsSeen { notice, .. } | Output::Journal { notice, .. } => notice.clone(),
        _ => None,
    };
    outputs.iter().filter_map(notice).collect()
}

fn is_prologue(prompt: &str) -> bool {
    prompt.contains("Write the prologue of the chronicle")
}

fn is_summary(prompt: &str) -> bool {
    prompt.contains("Write who this character has become")
}

/// What a batch and the calls after it brought.
#[derive(Default)]
struct Batch {
    prologues: Vec<String>,
    summaries: Vec<String>,
    notices: Vec<String>,
}

/// Ends a batch. Each call that follows gets `answer` when it is the prologue, and an
/// answer that no check takes when it is not, so no failed call makes the pace tight.
fn end_batch(story: &mut Story, batch: u64, answer: &str) -> Batch {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    let mut seen = Batch {
        notices: notices(&outputs),
        ..Batch::default()
    };
    let mut open = calls(&outputs);
    while let Some((call, prompt)) = open.pop() {
        let text = if is_prologue(&prompt) {
            answer.to_string()
        } else {
            "not an answer".to_string()
        };
        if is_prologue(&prompt) {
            seen.prologues.push(prompt);
        } else if is_summary(&prompt) {
            seen.summaries.push(prompt);
        }
        let outputs = story.handle(Input::ModelAnswered { call, text }).unwrap();
        seen.notices.extend(notices(&outputs));
        open.extend(calls(&outputs));
    }
    seen
}

fn journal(story: &mut Story) -> (Vec<Chapter>, Option<PastWanted>) {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(900),
            page: 0,
        })
        .unwrap();
    match outputs.into_iter().next() {
        Some(Output::Journal { page, past, .. }) => (page.journal.chapters, past),
        other => panic!("no journal: {other:?}"),
    }
}

#[test]
fn a_long_past_at_the_first_login_gets_one_prologue_call() {
    let folder = fresh("first-login");
    let mut story = started(&folder);
    log_in(&mut story, 35);
    read_past(&mut story, long_past(12));

    let prompts = end_batch(&mut story, 1, PROLOGUE).prologues;

    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("The hero: a human paladin, level 35."));
    assert!(prompts[0].contains("breadbasket of Stormwind"));
    assert!(prompts[0].contains("Stormwind (honored)"));
    assert!(!prompts[0].contains("212"));
}

#[test]
fn the_prologue_is_chapter_0_of_the_chronicle() {
    let folder = fresh("chapter-0");
    let mut story = started(&folder);
    log_in(&mut story, 35);
    read_past(&mut story, long_past(12));
    let batch = end_batch(&mut story, 1, PROLOGUE);

    let (chapters, _) = journal(&mut story);

    let prologue = &chapters[0];
    assert_eq!(prologue.number, 0);
    assert_eq!(prologue.opened_by, OpenedBy::Prologue);
    assert_eq!(prologue.title.as_deref(), Some("Prologue"));
    assert_eq!(prologue.levels, Some([35, 35]));
    assert!(
        prologue
            .prose
            .as_deref()
            .unwrap()
            .starts_with("The Defias Brotherhood took")
    );
    assert_eq!(prologue.zones, ["Westfall", "Elwynn Forest"]);
    assert_eq!(batch.notices, ["Your Chronicle has a prologue now."]);
}

#[test]
fn a_new_character_gets_no_prologue() {
    let folder = fresh("new-character");
    let mut story = started(&folder);
    log_in(&mut story, 1);
    read_past(&mut story, short_past(12));

    let prompts: Vec<String> = (1..=3)
        .flat_map(|batch| end_batch(&mut story, batch, PROLOGUE).prologues)
        .collect();

    assert!(prompts.is_empty());
}

#[test]
fn a_later_long_past_never_brings_a_prologue() {
    let folder = fresh("later-past");
    let mut story = started(&folder);
    log_in(&mut story, 1);
    read_past(&mut story, short_past(12));
    end_batch(&mut story, 1, PROLOGUE);

    read_past(&mut story, long_past(5000));
    let prompts = end_batch(&mut story, 2, PROLOGUE).prologues;

    assert!(prompts.is_empty());
}

#[test]
fn the_journal_asks_for_the_past_until_one_came() {
    let folder = fresh("wanted");
    let mut story = started(&folder);
    log_in(&mut story, 1);

    let (_, before) = journal(&mut story);
    read_past(&mut story, short_past(12));
    let (_, after) = journal(&mut story);

    assert_eq!(before, Some(PastWanted::Wanted));
    assert_eq!(after, None);
}

#[test]
fn a_written_prologue_is_never_asked_for_again() {
    let folder = fresh("once");
    let mut story = started(&folder);
    log_in(&mut story, 35);
    read_past(&mut story, long_past(12));
    end_batch(&mut story, 1, PROLOGUE);
    drop(story);

    let mut story = started(&folder);
    read_past(&mut story, long_past(5000));
    let prompts = end_batch(&mut story, 2, PROLOGUE).prologues;

    assert!(prompts.is_empty());
}

const REFUSED: &str = r#"{"prologue": "Our hero walked."}"#;

#[test]
fn a_refused_prologue_gets_a_new_try_at_a_later_batch() {
    let folder = fresh("refused");
    let mut story = started(&folder);
    log_in(&mut story, 35);
    read_past(&mut story, long_past(12));
    end_batch(&mut story, 1, REFUSED);

    let next = end_batch(&mut story, 2, PROLOGUE).prologues;

    assert_eq!(next.len(), 1);
    assert_eq!(journal(&mut story).0[0].number, 0);
}

#[test]
fn a_prologue_gets_three_tries_in_a_life_also_across_a_restart() {
    let folder = fresh("three-tries");
    let mut story = started(&folder);
    log_in(&mut story, 35);
    read_past(&mut story, long_past(12));
    end_batch(&mut story, 1, REFUSED);
    end_batch(&mut story, 2, REFUSED);
    drop(story);
    let mut story = started(&folder);
    let third = end_batch(&mut story, 3, REFUSED).prologues;

    let fourth = end_batch(&mut story, 4, PROLOGUE).prologues;

    assert_eq!(third.len(), 1);
    assert!(fourth.is_empty());
}

#[test]
fn a_world_with_a_closed_chapter_gets_no_prologue() {
    let folder = fresh("played");
    let mut story = started(&folder);
    log_in(&mut story, 9);
    for n in 1..=20 {
        enter(
            &mut story,
            100 + n * 60,
            "Westfall",
            Some(&format!("Camp {n}")),
        );
        story
            .handle(Input::LevelReached {
                at: Tick(100 + n * 60),
                level: 9 + u8::try_from(n).unwrap(),
            })
            .unwrap();
    }
    enter(&mut story, 5000, "Duskwood", Some("Darkshire"));
    story
        .handle(Input::LevelReached {
            at: Tick(5001),
            level: 30,
        })
        .unwrap();
    assert!(journal(&mut story).0.len() >= 2, "no closed chapter yet");

    read_past(&mut story, long_past(6000));
    let prompts = end_batch(&mut story, 1, PROLOGUE).prologues;

    assert!(prompts.is_empty());
}

#[test]
fn the_summary_tells_the_prologue() {
    let folder = fresh("summary");
    let mut story = started(&folder);
    log_in(&mut story, 35);
    read_past(&mut story, long_past(12));

    let summaries: Vec<String> = (1..=2)
        .flat_map(|batch| end_batch(&mut story, batch, PROLOGUE).summaries)
        .collect();

    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].contains("The Defias Brotherhood took the farms"));
}

#[test]
fn a_past_at_level_0_is_refused() {
    let folder = fresh("bad");
    let mut story = started(&folder);
    let bad = Past {
        level: 0,
        ..long_past(12)
    };

    assert!(story.handle(Input::PastRead(bad)).is_err());
}

#[test]
fn the_past_line_reads_from_the_addon() {
    let line = r#"{"type":"past_read","at":12,"level":35,"zone":"Westfall","quests":212,"zones":["Elwynn Forest","Westfall"],"factions":[{"name":"Stormwind","standing":6}],"gear":{"rare":3}}"#;

    let input: Input = serde_json::from_str(line).unwrap();

    let Input::PastRead(past) = input else {
        panic!("not a past");
    };
    assert_eq!(past.quests, 212);
    assert_eq!(past.gear, Gear { rare: 3, epic: 0 });
    assert!(past.mounts.is_empty());
}
