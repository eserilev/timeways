//! The summary of the character on the title page of the Chronicle
//! (docs/plans/hero-stories.md 3.5), through the story program with a fake model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const HOUR: u64 = 3600;

const SUMMARY: &str =
    r#"{"summary": "Westfall knows the face of $N, who came to the camps and stayed."}"#;
const NEXT_SUMMARY: &str =
    r#"{"summary": "The paladin walked from Westfall into the gloom of Duskwood."}"#;

/// Batches end until a summary call comes, at most this many.
const MOST_BATCHES: u64 = 6;

fn fresh(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("summary-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn started(folder: &Path) -> Story {
    let pack = folder.join("pack.sqlite");
    if !pack.exists() {
        Pack::write(&pack, &[]).unwrap();
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

fn calls(outputs: Vec<Output>) -> Vec<(CallId, String)> {
    outputs
        .into_iter()
        .filter_map(|output| match output {
            Output::ModelCall { call, prompt } => Some((call, prompt)),
            _ => None,
        })
        .collect()
}

fn is_summary(prompt: &str) -> bool {
    prompt.contains("Write who this character has become")
}

fn is_narrator(prompt: &str) -> bool {
    prompt.contains("Tell the moment below")
}

/// The fake model: silence for the narrator, a draft that breaks the rules for a saga, and
/// `summary` for a summary. None fails the call.
fn reply(prompt: &str, summary: Option<&str>) -> Option<String> {
    if is_summary(prompt) {
        return summary.map(str::to_string);
    }
    if is_narrator(prompt) {
        return Some(String::new());
    }
    Some("not an answer".to_string())
}

/// Ends a batch, and answers each call that follows from it. Returns the prompts of the
/// summary calls.
fn end_batch(story: &mut Story, batch: u64, summary: Option<&str>) -> Vec<String> {
    let mut open = calls(
        story
            .handle(Input::BatchEnd {
                id: MessageId(batch),
            })
            .unwrap(),
    );
    let mut summaries = Vec::new();
    while let Some((call, prompt)) = open.pop() {
        if is_summary(&prompt) {
            summaries.push(prompt.clone());
        }
        let outputs = match reply(&prompt, summary) {
            Some(text) => story.handle(Input::ModelAnswered { call, text }),
            None => story.handle(Input::ModelFailed { call }),
        };
        open.extend(calls(outputs.unwrap()));
    }
    summaries
}

/// Batches end until a summary call comes. Returns its prompt, or None.
fn until_summary(story: &mut Story, first_batch: u64, summary: Option<&str>) -> Option<String> {
    (first_batch..first_batch + MOST_BATCHES)
        .find_map(|batch| end_batch(story, batch, summary).into_iter().next())
}

/// 14 new camps on foot after a meeting in the zone: with the meeting, the least weight of
/// a chapter (docs/plans/chapters.md 4). A camp is a subzone, so the facts stay short.
fn walk_camps(story: &mut Story, at: u64, zone: &str) {
    for n in 1..=14 {
        enter(story, at + n * 60, zone, Some(&format!("Camp {n}")));
    }
}

/// A finished chapter in Westfall, and a new one in Duskwood, in a calm window.
fn one_finished_chapter(story: &mut Story) {
    enter(story, HOUR, "Westfall", None);
    meet(story, HOUR, "Gryan Stoutmantle");
    walk_camps(story, HOUR, "Westfall");
    enter(story, 5 * HOUR, "Duskwood", None);
    meet(story, 5 * HOUR, "Madame Eva");
}

/// Play in Duskwood long enough to fill a chapter, and a new zone hours later, so the
/// chapter in Duskwood is finished too.
fn second_finished_chapter(story: &mut Story) {
    walk_camps(story, 5 * HOUR, "Duskwood");
    enter(story, 9 * HOUR, "Redridge Mountains", None);
    meet(story, 9 * HOUR, "Marshal Dughan");
}

/// The summary that the journal carries.
fn journal_summary(story: &mut Story) -> Option<String> {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(70),
            page: 0,
        })
        .unwrap();
    match outputs.into_iter().next() {
        Some(Output::Journal { page, .. }) => page.journal.summary.map(String::from),
        other => panic!("expected a journal, got {other:?}"),
    }
}

/// A talk with Salma, whose call stays open until the test answers it.
fn open_talk(story: &mut Story) -> CallId {
    meet(story, 5 * HOUR, "Salma Saldean");
    let talk = Input::TalkAsked {
        id: MessageId(8),
        at: Tick(5 * HOUR + 60),
        npc: "Salma Saldean".to_string(),
        text: "any news".to_string(),
    };
    calls(story.handle(talk).unwrap())
        .into_iter()
        .find(|(_, prompt)| !is_narrator(prompt))
        .expect("a talk call")
        .0
}

/// The first summary of the character, from the chapter in Westfall.
fn first_summary(story: &mut Story) {
    one_finished_chapter(story);
    until_summary(story, 90, Some(SUMMARY)).expect("a summary call");
}

#[test]
fn no_summary_before_the_first_chapter_ends() {
    let mut story = started(&fresh("none"));
    enter(&mut story, HOUR, "Westfall", None);

    let prompt = until_summary(&mut story, 90, Some(SUMMARY));

    assert_eq!(prompt, None);
    assert_eq!(journal_summary(&mut story), None);
}

#[test]
fn the_summary_call_opens_after_the_saga_round_of_its_chapter() {
    let mut story = started(&fresh("after-round"));
    one_finished_chapter(&mut story);

    let prompt = until_summary(&mut story, 90, Some(SUMMARY)).expect("a summary call");

    assert!(
        prompt.contains("Chapter 1: traveled to Westfall"),
        "{prompt}"
    );
    assert_eq!(
        journal_summary(&mut story).as_deref(),
        Some("Westfall knows the face of $N, who came to the camps and stayed.")
    );
}

#[test]
fn the_summary_waits_while_another_call_is_open() {
    let mut story = started(&fresh("busy"));
    one_finished_chapter(&mut story);
    let talk = open_talk(&mut story);

    let while_open = until_summary(&mut story, 90, Some(SUMMARY));
    let text = r#"{"say": "Well met.", "trust": 0}"#.to_string();
    story
        .handle(Input::ModelAnswered { call: talk, text })
        .unwrap();
    enter(&mut story, 7 * HOUR, "Duskwood", Some("Darkshire"));
    let after = until_summary(&mut story, 100, Some(SUMMARY));

    assert_eq!(while_open, None);
    assert!(after.is_some());
}

#[test]
fn the_summary_waits_while_the_window_is_tight() {
    let mut story = started(&fresh("tight"));
    one_finished_chapter(&mut story);
    let talk = open_talk(&mut story);
    story.handle(Input::ModelFailed { call: talk }).unwrap();

    let tight = until_summary(&mut story, 90, Some(SUMMARY));
    enter(&mut story, 6 * HOUR, "Duskwood", Some("Darkshire"));
    let calm = until_summary(&mut story, 100, Some(SUMMARY));

    assert_eq!(tight, None);
    assert!(calm.is_some());
}

/// Two chapters finish before any saga: the sagas go first, and only the newest chapter
/// gets a summary.
#[test]
fn only_the_newest_finished_chapter_gets_a_summary_after_every_saga() {
    let mut story = started(&fresh("newest"));
    one_finished_chapter(&mut story);
    second_finished_chapter(&mut story);

    let mut summaries: Vec<String> = (90..90 + MOST_BATCHES)
        .flat_map(|batch| end_batch(&mut story, batch, Some(SUMMARY)))
        .collect();
    // Five calls in one minute make the window tight, so the summary waits for later play.
    enter(
        &mut story,
        10 * HOUR,
        "Redridge Mountains",
        Some("Lakeshire"),
    );
    summaries.extend(
        (100..100 + MOST_BATCHES).flat_map(|batch| end_batch(&mut story, batch, Some(SUMMARY))),
    );

    assert_eq!(summaries.len(), 1, "{summaries:?}");
    let newest = summaries[0].split("newest first:\n<<<\n- ").nth(1).unwrap();
    assert!(
        newest.starts_with("Chapter 2: traveled to Duskwood"),
        "{newest}"
    );
}

#[test]
fn a_refused_summary_keeps_the_one_before_it() {
    let mut story = started(&fresh("refused"));
    first_summary(&mut story);
    second_finished_chapter(&mut story);

    until_summary(
        &mut story,
        100,
        Some(r#"{"summary": "Our hero moved on."}"#),
    )
    .expect("a summary call");

    assert!(journal_summary(&mut story).unwrap().starts_with("Westfall"));
}

#[test]
fn a_failed_call_keeps_the_one_before_it() {
    let mut story = started(&fresh("failed"));
    first_summary(&mut story);
    second_finished_chapter(&mut story);

    until_summary(&mut story, 100, None).expect("a summary call");

    assert!(journal_summary(&mut story).unwrap().starts_with("Westfall"));
}

#[test]
fn a_new_summary_grows_from_the_one_before_it() {
    let mut story = started(&fresh("grows"));
    first_summary(&mut story);
    second_finished_chapter(&mut story);

    let prompt = until_summary(&mut story, 100, Some(NEXT_SUMMARY)).expect("a summary call");

    assert!(prompt.contains("Westfall knows the face of $N"), "{prompt}");
    assert!(
        journal_summary(&mut story)
            .unwrap()
            .starts_with("The paladin")
    );
}

#[test]
fn the_summary_prompt_holds_no_story_and_no_profile_field() {
    let mut story = started(&fresh("private"));
    let story_line = Input::StoryAccepted {
        at: Tick(HOUR / 2),
        number: 1,
        title: Some("Zqtitle".to_string()),
        paragraphs: vec!["Zqstory held the bridge.".to_string()],
    };
    story.handle(story_line).unwrap();
    for (field, text) in [("title", "Zqprofile"), ("goal", "Find my brother.")] {
        let set = Input::HeroSet {
            at: Tick(HOUR / 2),
            field: field.to_string(),
            text: text.to_string(),
        };
        story.handle(set).unwrap();
    }
    one_finished_chapter(&mut story);

    let prompt = until_summary(&mut story, 90, Some(SUMMARY)).expect("a summary call");

    assert!(prompt.contains("Find my brother."), "{prompt}");
    for private in ["Zqtitle", "Zqstory", "Zqprofile"] {
        assert!(!prompt.contains(private), "{private} in {prompt}");
    }
}

#[test]
fn the_summary_reads_the_sheet_the_chapters_and_the_summary_before_it() {
    let folder = fresh("reads");
    let mut story = started(&folder);
    let set = Input::HeroSet {
        at: Tick(HOUR / 2),
        field: "goal".to_string(),
        text: "Find my brother.".to_string(),
    };
    story.handle(set).unwrap();
    first_summary(&mut story);
    second_finished_chapter(&mut story);
    until_summary(&mut story, 100, Some(NEXT_SUMMARY)).expect("a summary call");
    drop(story);

    let connection =
        rusqlite::Connection::open(folder.join("worlds/r_Testrealm/c_Tester.sqlite")).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT tab FROM reads WHERE call =
                 (SELECT max(position) FROM calls WHERE kind = 'summary') ORDER BY tab",
        )
        .unwrap();
    let tables: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tables, ["events", "hero", "summaries"]);
}

#[test]
fn the_journal_carries_the_newest_summary() {
    let mut story = started(&fresh("journal"));
    first_summary(&mut story);

    let line = story
        .handle(Input::JournalAsked {
            id: MessageId(70),
            page: 0,
        })
        .unwrap();

    let line = serde_json::to_string(&line[0]).unwrap();
    assert!(
        line.contains(r#""summary":"Westfall knows the face of $N"#),
        "{line}"
    );
}
