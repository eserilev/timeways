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
