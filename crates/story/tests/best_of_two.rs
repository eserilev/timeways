//! The best of two for the saga of a chapter (GAMEPLAY.md 3.3), through the story program
//! with a fake model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::journal::Chapter;
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const HOUR: u64 = 3600;

fn fresh(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("best-of-two-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// The story program on `folder`, as after a start, with the character entered.
fn started(folder: &Path, files: Store) -> Story {
    let pack = folder.join("pack.sqlite");
    if !pack.exists() {
        Pack::write(&pack, &[]).unwrap();
    }
    let mut story = Story::new(Pack::open(&pack).unwrap(), files);
    let character = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    story.handle(character).unwrap();
    story
}

fn in_memory(name: &str) -> Story {
    started(&fresh(name), Store::Memory)
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

/// 14 new camps on foot after a meeting in the zone: with the meeting, the weight of a
/// chapter (docs/plans/chapters.md 4). A camp is a subzone, so the facts of the chapter
/// stay short.
fn walk_camps(story: &mut Story, at: u64, zone: &str) {
    for n in 1..=14 {
        enter(story, at + n * 60, zone, Some(&format!("Camp {n}")));
    }
}

/// Ends a batch, and lets each model call of it fail.
fn close_batch(story: &mut Story, batch: u64) {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    for output in outputs {
        if let Output::ModelCall { call, .. } = output {
            story.handle(Input::ModelFailed { call }).unwrap();
        }
    }
}

/// A finished chapter in Westfall, and a new one in Duskwood. The last failed call is an
/// hour before the next batch, so the budget window is calm.
fn two_calm_sessions(story: &mut Story) {
    enter(story, HOUR, "Westfall", None);
    meet(story, HOUR, "Gryan Stoutmantle");
    walk_camps(story, HOUR, "Westfall");
    enter(story, 5 * HOUR, "Duskwood", None);
    close_batch(story, 91);
    meet(story, 6 * HOUR, "Salma Saldean");
}

/// The first draft call of the saga, from the end of the next batch.
fn first_draft(story: &mut Story, batch: u64) -> (CallId, String) {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    match outputs.as_slice() {
        [_, Output::ModelCall { call, prompt }] => (*call, prompt.clone()),
        _ => panic!("expected a saga call, got {outputs:?}"),
    }
}

/// The next call of the saga after an input, if one goes out.
fn next_call(story: &mut Story, input: Input) -> Option<(CallId, String)> {
    let outputs = story.handle(input).unwrap();
    match outputs.as_slice() {
        [] => None,
        [Output::ModelCall { call, prompt }] => Some((*call, prompt.clone())),
        _ => panic!("expected at most one call, got {outputs:?}"),
    }
}

fn answered(story: &mut Story, call: CallId, text: &str) -> Option<(CallId, String)> {
    let text = text.to_string();
    next_call(story, Input::ModelAnswered { call, text })
}

fn talk_to_salma(at: u64) -> Input {
    Input::TalkAsked {
        id: MessageId(8),
        at: Tick(at),
        npc: "Salma Saldean".to_string(),
        text: "any news?".to_string(),
    }
}

fn saga(text: &str) -> String {
    serde_json::json!({ "saga": text }).to_string()
}

fn chapters(story: &mut Story) -> Vec<Chapter> {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();
    match outputs.into_iter().next() {
        Some(Output::Journal { page, .. }) => page.journal.chapters,
        other => panic!("expected a journal, got {other:?}"),
    }
}

const FIRST: &str = "$N kept to the long west road and helped a farmer.";
const SECOND: &str = "$N took the west road. A farmer needed help, and got it.";

/// Two drafts that pass, and the call of the judge.
fn up_to_the_judge(story: &mut Story) -> (CallId, String) {
    two_calm_sessions(story);
    let (first, _) = first_draft(story, 3);
    let (second, _) = answered(story, first, &saga(FIRST)).unwrap();
    answered(story, second, &saga(SECOND)).unwrap()
}

#[test]
fn a_calm_window_asks_a_second_draft_with_other_samples() {
    let mut story = in_memory("second-draft");
    two_calm_sessions(&mut story);
    let (call, first_prompt) = first_draft(&mut story, 3);

    let (_, second_prompt) = answered(&mut story, call, &saga(FIRST)).unwrap();

    assert!(second_prompt.contains("Write chapter 1"), "{second_prompt}");
    assert_ne!(first_prompt, second_prompt);
    assert_eq!(chapters(&mut story)[0].prose, None);
}

#[test]
fn the_judge_sees_the_facts_and_both_drafts() {
    let mut story = in_memory("judge-prompt");

    let (_, prompt) = up_to_the_judge(&mut story);

    assert!(
        prompt.contains("The facts of chapter 1:\n<<<\n- Traveled to: Westfall."),
        "{prompt}"
    );
    assert!(
        prompt.contains(&format!("Draft 1:\n<<<\n{FIRST}\n>>>")),
        "{prompt}"
    );
    assert!(
        prompt.contains(&format!("Draft 2:\n<<<\n{SECOND}\n>>>")),
        "{prompt}"
    );
    assert!(
        prompt.ends_with("{\"pick\": 1} or {\"pick\": 2}"),
        "{prompt}"
    );
}

#[test]
fn the_pick_of_the_judge_becomes_the_saga() {
    let mut story = in_memory("judge-picks");
    let (judge, _) = up_to_the_judge(&mut story);

    let after = answered(&mut story, judge, r#"{"pick": 2}"#);

    assert_eq!(after, None);
    assert_eq!(chapters(&mut story)[0].prose.as_deref(), Some(SECOND));
}

#[test]
fn a_judge_answer_that_is_no_pick_keeps_the_first_draft() {
    let mut story = in_memory("judge-rambles");
    let (judge, _) = up_to_the_judge(&mut story);

    answered(&mut story, judge, "Both are fine, honestly.");

    assert_eq!(chapters(&mut story)[0].prose.as_deref(), Some(FIRST));
}

#[test]
fn a_failed_judge_keeps_the_first_draft() {
    let mut story = in_memory("judge-fails");
    let (judge, _) = up_to_the_judge(&mut story);

    next_call(&mut story, Input::ModelFailed { call: judge });

    assert_eq!(chapters(&mut story)[0].prose.as_deref(), Some(FIRST));
}

#[test]
fn a_draft_that_breaks_a_rule_loses_with_no_judge() {
    let mut story = in_memory("one-passes");
    two_calm_sessions(&mut story);
    let (first, _) = first_draft(&mut story, 3);
    let (second, _) = answered(&mut story, first, "Here is your saga!").unwrap();

    let after = answered(&mut story, second, &saga(SECOND));

    assert_eq!(after, None);
    assert_eq!(chapters(&mut story)[0].prose.as_deref(), Some(SECOND));
}

#[test]
fn two_drafts_that_break_rules_leave_the_plain_list() {
    let mut story = in_memory("none-passes");
    two_calm_sessions(&mut story);
    let (first, _) = first_draft(&mut story, 3);
    let (second, _) = answered(&mut story, first, "no json").unwrap();

    let after = answered(&mut story, second, &saga("$N 🗡 rode west."));

    assert_eq!(after, None);
    assert_eq!(chapters(&mut story)[0].prose, None);
}

#[test]
fn a_recent_failure_leaves_the_saga_with_one_draft() {
    let mut story = in_memory("tight");
    two_calm_sessions(&mut story);
    // A talk 5 minutes before the batch fails, as a call over the budget does.
    let (talk, _) = next_call(&mut story, talk_to_salma(6 * HOUR + 60)).unwrap();
    story.handle(Input::ModelFailed { call: talk }).unwrap();
    meet(&mut story, 6 * HOUR + 300, "Tobias Mistmantle");
    let (first, _) = first_draft(&mut story, 3);

    let after = answered(&mut story, first, &saga(FIRST));

    assert_eq!(after, None);
    assert_eq!(chapters(&mut story)[0].prose.as_deref(), Some(FIRST));
}

#[test]
fn the_second_draft_waits_while_another_model_call_is_open() {
    let mut story = in_memory("waits");
    two_calm_sessions(&mut story);
    let (first, _) = first_draft(&mut story, 3);
    let (talk, _) = next_call(&mut story, talk_to_salma(6 * HOUR + 1)).unwrap();

    let while_open = answered(&mut story, first, &saga(FIRST));
    let words = serde_json::json!({ "say": "The road is quiet.", "trust": 0 }).to_string();
    story
        .handle(Input::ModelAnswered {
            call: talk,
            text: words,
        })
        .unwrap();
    let after = story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();

    assert_eq!(while_open, None);
    let [_, Output::ModelCall { prompt, .. }] = after.as_slice() else {
        panic!("expected the second draft, got {after:?}");
    };
    assert!(prompt.contains("Write chapter 1"), "{prompt}");
}

#[test]
fn a_restart_between_the_drafts_asks_the_chapter_again_and_keeps_one_saga() {
    let folder = fresh("restart");
    let mut before = started(&folder, Store::Folder(folder.clone()));
    two_calm_sessions(&mut before);
    let (first, _) = first_draft(&mut before, 3);
    answered(&mut before, first, &saga(FIRST)).unwrap();
    drop(before);

    let mut after = started(&folder, Store::Folder(folder.clone()));
    let unwritten = chapters(&mut after)[0].prose.clone();
    // The pace of the calls before the restart stays, so the next round waits for a calm
    // window to get its extra calls.
    meet(&mut after, 7 * HOUR, "Tobias Mistmantle");
    let (first, prompt) = first_draft(&mut after, 3);
    let (second, _) = answered(&mut after, first, &saga(FIRST)).unwrap();
    let (judge, _) = answered(&mut after, second, &saga(SECOND)).unwrap();
    answered(&mut after, judge, r#"{"pick": 1}"#);
    let written = chapters(&mut after)[0].prose.clone();
    drop(after);
    let mut again = started(&folder, Store::Folder(folder.clone()));
    let asked_again = again.handle(Input::BatchEnd { id: MessageId(5) }).unwrap();

    assert_eq!(unwritten, None);
    assert!(prompt.contains("Write chapter 1"), "{prompt}");
    assert_eq!(written.as_deref(), Some(FIRST));
    assert_eq!(chapters(&mut again)[0].prose.as_deref(), Some(FIRST));
    assert_eq!(asked_again.len(), 1, "{asked_again:?}");
}

#[test]
fn a_draft_for_another_character_is_dropped() {
    let mut story = in_memory("switch");
    two_calm_sessions(&mut story);
    let (first, _) = first_draft(&mut story, 3);
    let bren = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Bren".to_string(),
    };
    story.handle(bren).unwrap();

    let after = answered(&mut story, first, &saga(FIRST));

    assert_eq!(after, None);
}
