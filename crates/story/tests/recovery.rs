#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What a failed save leaves behind (GAMEPLAY.md 5.7): the line is gone, and the next
//! line works as before.

use hourglass::Tick;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::pack::Pack;
use timeways_story::store::Store;
use timeways_story::story::{Output, Story, StoryError};

fn fresh_folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("recovery-{name}"));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

fn story(folder: &Path) -> Story {
    let pack = folder.join("pack.sqlite");
    Pack::write(&pack, &[]).unwrap();
    let mut story = Story::new(
        Pack::open(&pack).unwrap(),
        Store::Folder(folder.to_path_buf()),
    );
    story.handle(entered("Ada")).unwrap();
    story
}

fn entered(name: &str) -> Input {
    Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: name.to_string(),
    }
}

fn world_file(folder: &Path) -> PathBuf {
    folder
        .join("worlds")
        .join("r_Stormrage")
        .join("c_Ada.sqlite")
}

/// Another program holds the write lock of the world, so each save fails.
fn locked(folder: &Path) -> Connection {
    let other = Connection::open(world_file(folder)).unwrap();
    other.execute_batch("BEGIN IMMEDIATE").unwrap();
    other
}

fn unlock(other: &Connection) {
    other.execute_batch("ROLLBACK").unwrap();
}

fn talk_asked(npc: &str) -> Input {
    Input::TalkAsked {
        id: MessageId(8),
        at: Tick(50),
        npc: npc.to_string(),
        text: "any news?".to_string(),
    }
}

fn call_of(outputs: &[Output]) -> Option<CallId> {
    outputs.iter().find_map(|output| match output {
        Output::ModelCall { call, .. } => Some(*call),
        _ => None,
    })
}

fn talk(story: &mut Story, npc: &str) -> Option<CallId> {
    call_of(&story.handle(talk_asked(npc)).unwrap())
}

fn answered(call: CallId) -> Input {
    Input::ModelAnswered {
        call,
        text: r#"{"say": "Nothing but rain.", "trust": 3}"#.to_string(),
    }
}

#[test]
fn a_talk_after_failed_saves_still_reaches_the_model() {
    let folder = fresh_folder("talk-after-failed-saves");
    let mut story = story(&folder);
    let other = locked(&folder);
    for _ in 0..2 {
        let refused = story.handle(talk_asked("Innkeeper Farley"));
        assert!(matches!(refused, Err(StoryError::Store(_))));
    }
    unlock(&other);

    let call = talk(&mut story, "Innkeeper Farley");

    assert!(call.is_some());
}

#[test]
fn a_call_that_waits_still_goes_out_after_a_failed_answer() {
    let folder = fresh_folder("queued-after-failed-answer");
    let mut story = story(&folder);
    let first = talk(&mut story, "Innkeeper Farley").unwrap();
    let second = talk(&mut story, "Marshal Dughan").unwrap();
    assert_eq!(talk(&mut story, "Guard Thomas"), None);
    let other = locked(&folder);
    assert!(story.handle(answered(first)).is_err());
    unlock(&other);

    let outputs = story.handle(answered(second)).unwrap();

    assert!(call_of(&outputs).is_some(), "{outputs:?}");
}

const HOUR: u64 = 3600;

fn handled(story: &mut Story, input: Input) -> Vec<Output> {
    story.handle(input).unwrap()
}

fn enter(story: &mut Story, at: u64, zone: &str, subzone: Option<&str>) {
    handled(
        story,
        Input::ZoneEntered {
            at: Tick(at),
            zone: zone.to_string(),
            subzone: subzone.map(str::to_string),
            spot: None,
            hour: None,
        },
    );
}

fn meet(story: &mut Story, at: u64, name: &str) {
    handled(
        story,
        Input::NpcMet {
            at: Tick(at),
            name: name.to_string(),
            spot: None,
        },
    );
}

/// Two chapters: the first one is finished. Its narrator line fails long before the
/// saga, so the pace leaves room for two drafts.
fn two_sessions(story: &mut Story) {
    meet(story, HOUR, "Gryan Stoutmantle");
    enter(story, HOUR, "Westfall", None);
    enter(story, HOUR + 25 * 60, "Westfall", Some("Camp One"));
    enter(story, HOUR + 50 * 60, "Westfall", Some("Camp Two"));
    enter(story, 5 * HOUR, "Duskwood", None);
    for output in handled(story, Input::BatchEnd { id: MessageId(91) }) {
        if let Output::ModelCall { call, .. } = output {
            handled(story, Input::ModelFailed { call });
        }
    }
    meet(story, 6 * HOUR, "Salma Saldean");
}

/// The reads of each call that read a call, as (call, read call).
fn calls_read(folder: &Path) -> Vec<(i64, i64)> {
    let connection = Connection::open(world_file(folder)).unwrap();
    let mut statement = connection
        .prepare("SELECT call, row FROM reads WHERE tab = 'calls'")
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn a_saga_after_a_failed_save_reads_only_earlier_calls() {
    let folder = fresh_folder("saga-after-failed-save");
    let mut story = story(&folder);
    two_sessions(&mut story);
    let outputs = handled(&mut story, Input::BatchEnd { id: MessageId(3) });
    let draft = call_of(&outputs).expect("a saga draft");
    let other = locked(&folder);
    let text = r#"{"saga": "Our hero rode into the golden fields of Westfall."}"#;
    let failed = story.handle(Input::ModelAnswered {
        call: draft,
        text: text.to_string(),
    });
    assert!(failed.is_err());
    unlock(&other);

    let outputs = handled(&mut story, Input::BatchEnd { id: MessageId(4) });
    drop(story);

    assert!(call_of(&outputs).is_some(), "{outputs:?}");
    for (call, read) in calls_read(&folder) {
        assert!(read < call, "call {call} reads call {read}");
    }
}
