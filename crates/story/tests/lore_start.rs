//! The start of the story program while the desktop app builds the lore pack (GAMEPLAY.md
//! 5.10, relay SPEC 11.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use timeways_story::input::{Input, MessageId};
use timeways_story::lore_start::{BUILDING_MARK, LORE_READY, LoreStartError, open_lore};
use timeways_story::pack::{FORMAT_VERSION, Link, Origin, Pack, PackError, Passage};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

fn fresh(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("lore-start-{name}"));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

fn tower() -> Passage {
    Passage {
        text: "The tower of Elwynn fell long ago.".to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![Link::Place("Elwynn Forest".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: Vec::new(),
        setup_for: None,
    }
}

#[test]
fn a_missing_pack_starts_with_no_passages_and_leaves_the_mark() {
    let root = fresh("missing");
    let story_folder = root.join("story");

    let start = open_lore(&root.join("lore.sqlite"), Some(&story_folder)).unwrap();

    assert!(start.pack.search("tower", 5).unwrap().is_empty());
    assert_eq!(start.notice, None);
    assert!(story_folder.join(BUILDING_MARK).is_file());
    assert!(!root.join("lore.sqlite").exists());
}

/// A pack of `format`, as an older or a newer Timeways left it.
fn pack_of_format(root: &Path, format: i64) -> PathBuf {
    let pack = root.join("lore.sqlite");
    Pack::write(&pack, &[tower()]).unwrap();
    rusqlite::Connection::open(&pack)
        .unwrap()
        .pragma_update(None, "user_version", format)
        .unwrap();
    pack
}

#[test]
fn an_older_pack_starts_with_no_lore_and_asks_for_a_rebuild() {
    let root = fresh("older");
    let pack = pack_of_format(&root, FORMAT_VERSION - 3);

    let start = open_lore(&pack, Some(&root)).unwrap();

    assert!(start.pack.search("tower", 5).unwrap().is_empty());
    assert!(root.join(BUILDING_MARK).is_file());
    assert_eq!(
        start.log.as_deref(),
        Some(
            format!(
                "lore pack format {}, this program reads {FORMAT_VERSION}: \
                 starting with no lore until it is rebuilt",
                FORMAT_VERSION - 3
            )
            .as_str()
        )
    );
}

#[test]
fn a_newer_pack_starts_with_no_lore_and_asks_for_a_rebuild() {
    let root = fresh("newer");
    let pack = pack_of_format(&root, FORMAT_VERSION + 1);

    let start = open_lore(&pack, Some(&root)).unwrap();

    assert!(start.pack.search("tower", 5).unwrap().is_empty());
    assert!(root.join(BUILDING_MARK).is_file());
    assert!(start.log.is_some());
}

#[test]
fn a_pack_of_another_format_is_never_changed() {
    let root = fresh("unchanged");
    let pack = pack_of_format(&root, FORMAT_VERSION - 1);
    let before = fs::read(&pack).unwrap();

    open_lore(&pack, Some(&root)).unwrap();

    assert_eq!(fs::read(&pack).unwrap(), before);
}

#[test]
fn a_corrupt_pack_still_stops_the_start() {
    let root = fresh("corrupt");
    let pack = root.join("lore.sqlite");
    fs::write(
        &pack,
        b"this is no pack, only some bytes that sqlite cannot read",
    )
    .unwrap();

    let result = open_lore(&pack, Some(&root));

    assert!(matches!(
        result,
        Err(LoreStartError::Pack(PackError::Sqlite(_)))
    ));
    assert!(!root.join(BUILDING_MARK).exists());
}

#[test]
fn the_first_start_with_a_pack_says_that_the_lore_is_ready_once() {
    let root = fresh("ready");
    let pack = root.join("lore.sqlite");
    open_lore(&pack, Some(&root)).unwrap();
    Pack::write(&pack, &[tower()]).unwrap();

    let first = open_lore(&pack, Some(&root)).unwrap();
    let second = open_lore(&pack, Some(&root)).unwrap();

    assert_eq!(first.notice.as_deref(), Some(LORE_READY));
    assert_eq!(second.notice, None);
    assert!(!root.join(BUILDING_MARK).exists());
    assert_eq!(first.pack.search("tower", 5).unwrap().len(), 1);
}

#[test]
fn a_start_with_a_pack_and_no_mark_says_nothing() {
    let root = fresh("no-mark");
    let pack = root.join("lore.sqlite");
    Pack::write(&pack, &[tower()]).unwrap();

    let start = open_lore(&pack, Some(&root)).unwrap();

    assert_eq!(start.notice, None);
}

#[test]
fn a_run_with_no_data_folder_keeps_no_mark() {
    let root = fresh("no-folder");

    let start = open_lore(&root.join("lore.sqlite"), None).unwrap();

    assert_eq!(start.notice, None);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}

#[test]
fn lore_is_ready_rides_on_the_next_answer() {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_program_notice(LORE_READY.to_string());
    story
        .handle(Input::CharacterEntered {
            realm: "Stormrage".to_string(),
            name: "Ada".to_string(),
        })
        .unwrap();

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();

    let Some(Output::EventsSeen { notice, .. }) = outputs.first() else {
        panic!("expected events_seen, got {outputs:?}");
    };
    assert_eq!(notice.as_deref(), Some(LORE_READY));
}

#[test]
fn with_no_pack_a_lore_question_still_gets_an_answer() {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story
        .handle(Input::CharacterEntered {
            realm: "Stormrage".to_string(),
            name: "Ada".to_string(),
        })
        .unwrap();

    let outputs = story
        .handle(Input::LoreAsked {
            id: MessageId(7),
            question: "why did the tower fall?".to_string(),
            target: None,
        })
        .unwrap();

    assert!(
        matches!(outputs.as_slice(), [Output::LoreAnswer { .. }]),
        "{outputs:?}"
    );
}
