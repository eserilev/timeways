//! Player stories through the story program (GAMEPLAY.md 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::Pack;
use timeways_story::store::{
    CallEnd, CharacterKey, Line, NewCall, Node, Outcome, Root, Store, Table,
};
use timeways_story::stories::{MAX_STORY_BYTES, PlayerStory};
use timeways_story::story::{Output, Story, StoryError};

const TOLD: &str = "$N held the bridge alone while we ran.";

fn fresh_folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("stories-{name}"));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

fn story(folder: &Path) -> Story {
    let pack = folder.join("pack.sqlite");
    if !pack.exists() {
        Pack::write(&pack, &[]).unwrap();
    }
    let mut story = Story::new(
        Pack::open(&pack).unwrap(),
        Store::Folder(folder.to_path_buf()),
    );
    story
        .handle(Input::CharacterEntered {
            realm: "Stormrage".to_string(),
            name: "Ada".to_string(),
        })
        .unwrap();
    story
}

fn accept(story: &mut Story, number: u64, text: &str) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::StoryAccepted {
        at: Tick(10),
        number,
        text: text.to_string(),
    })
}

fn remove(story: &mut Story, number: u64) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::StoryRemoved {
        at: Tick(20),
        number,
    })
}

fn stories(story: &mut Story) -> Vec<PlayerStory> {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();
    let Some(Output::Journal { page, .. }) = outputs.into_iter().next() else {
        panic!("expected a journal");
    };
    page.journal.stories
}

fn key() -> CharacterKey {
    CharacterKey::new("Stormrage", "Ada").unwrap()
}

#[test]
fn an_accepted_story_shows_in_the_journal() {
    let folder = fresh_folder("accepted");
    let mut story = story(&folder);

    accept(&mut story, 1, TOLD).unwrap();

    let told = PlayerStory {
        number: 1,
        text: TOLD.to_string(),
        at: Tick(10),
        used: false,
    };
    assert_eq!(stories(&mut story), [told]);
}

#[test]
fn an_accepted_story_rests_on_shared_proof() {
    let folder = fresh_folder("shared-proof");
    let mut story = story(&folder);

    accept(&mut story, 1, TOLD).unwrap();
    drop(story);

    let database = Store::Folder(folder).open(&key()).unwrap().database;
    let proof = database.proof_of(Node::Row(Table::Stories, 0)).unwrap();
    assert_eq!(proof, BTreeSet::from([Root::Shared]));
}

#[test]
fn a_story_with_the_same_number_is_refused() {
    let folder = fresh_folder("same-number");
    let mut story = story(&folder);
    accept(&mut story, 1, TOLD).unwrap();
    remove(&mut story, 1).unwrap();

    let again = accept(&mut story, 1, "Another tale.");

    assert!(matches!(again, Err(StoryError::StoryTaken(1))));
    assert!(stories(&mut story).is_empty());
}

#[test]
fn an_empty_long_or_odd_story_is_refused() {
    let folder = fresh_folder("bad-text");
    let mut story = story(&folder);

    for text in [
        "   ".to_string(),
        "a".repeat(MAX_STORY_BYTES + 1),
        "a line\nand another".to_string(),
        "a fake |Hlink|h".to_string(),
    ] {
        let refused = accept(&mut story, 1, &text);
        assert!(matches!(refused, Err(StoryError::BadStory)), "{text:?}");
    }
    assert!(accept(&mut story, 1, &"a".repeat(MAX_STORY_BYTES)).is_ok());
}

#[test]
fn a_removed_story_leaves_the_journal() {
    let folder = fresh_folder("removed");
    let mut story = story(&folder);
    accept(&mut story, 1, TOLD).unwrap();
    accept(&mut story, 2, "A second tale.").unwrap();

    remove(&mut story, 1).unwrap();

    let numbers: Vec<u64> = stories(&mut story).iter().map(|told| told.number).collect();
    assert_eq!(numbers, [2]);
    assert!(matches!(remove(&mut story, 1), Err(StoryError::NoStory(1))));
}

/// A call that read the story, as a later rule of the reads can make one.
fn a_call_reads_the_story(folder: &Path, outcome: Outcome) {
    let mut database = Store::Folder(folder.to_path_buf())
        .open(&key())
        .unwrap()
        .database;
    let line = Line {
        calls: vec![NewCall {
            position: 0,
            kind: "saga",
            pack: "test".to_string(),
            prompt: "prompt".to_string(),
            reads: vec![Node::Row(Table::Stories, 0)],
        }],
        ended: vec![CallEnd {
            position: 0,
            answer: Some("answer".to_string()),
            outcome,
        }],
        ..Line::default()
    };
    database.save(&line).unwrap();
}

#[test]
fn a_story_that_a_call_used_cannot_be_removed() {
    let folder = fresh_folder("used");
    let mut first = story(&folder);
    accept(&mut first, 1, TOLD).unwrap();
    drop(first);
    a_call_reads_the_story(&folder, Outcome::Accepted);
    let mut second = story(&folder);

    let refused = remove(&mut second, 1);

    assert!(matches!(refused, Err(StoryError::StoryInUse(1))));
    assert!(stories(&mut second)[0].used);
}

#[test]
fn a_story_that_a_refused_call_read_can_be_removed() {
    let folder = fresh_folder("refused-reader");
    let mut first = story(&folder);
    accept(&mut first, 1, TOLD).unwrap();
    drop(first);
    a_call_reads_the_story(&folder, Outcome::Refused);
    let mut second = story(&folder);

    assert!(!stories(&mut second)[0].used);
    assert!(remove(&mut second, 1).is_ok());
}

#[test]
fn a_refused_story_keeps_its_input_and_adds_no_row() {
    let folder = fresh_folder("refused-row");
    let mut story = story(&folder);

    let _ = accept(&mut story, 1, "");
    drop(story);

    let connection = rusqlite::Connection::open(
        folder
            .join("worlds")
            .join("r_Stormrage")
            .join("c_Ada.sqlite"),
    )
    .unwrap();
    let count =
        |select: &str| -> i64 { connection.query_row(select, [], |row| row.get(0)).unwrap() };
    assert_eq!(
        count("SELECT count(*) FROM inputs WHERE kind = 'story_accepted'"),
        1
    );
    assert_eq!(count("SELECT count(*) FROM stories"), 0);
}

fn stored_bodies(folder: &Path, table: &str) -> Vec<String> {
    let path = folder.join("worlds/r_Stormrage/c_Ada.sqlite");
    let connection = rusqlite::Connection::open(path).unwrap();
    let mut statement = connection
        .prepare(&format!("SELECT body FROM {table} ORDER BY position"))
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn a_story_keeps_an_id_for_each_player_and_the_journal_shows_the_name() {
    let folder = fresh_folder("ids");
    let mut story = story(&folder);

    accept(
        &mut story,
        1,
        "{Corvin} and $N held the bridge. CORVIN-Stormrage ran.",
    )
    .unwrap();

    assert_eq!(
        stories(&mut story)[0].text,
        "Corvin and $N held the bridge. Corvin ran."
    );
    drop(story);
    let body = &stored_bodies(&folder, "stories")[0];
    assert!(
        body.contains("{P1} and $N held the bridge. {P1} ran."),
        "{body}"
    );
    assert!(!body.contains("Corvin"), "{body}");
}

#[test]
fn a_player_keeps_the_same_id_after_a_reopen() {
    let folder = fresh_folder("same-id");
    let mut story = story(&folder);
    accept(&mut story, 1, "{Corvin} and {Bob} came.").unwrap();
    drop(story);

    let mut story = self::story(&folder);
    accept(&mut story, 2, "{Bob} came back.").unwrap();
    drop(story);

    let bodies = stored_bodies(&folder, "stories");
    assert!(bodies[1].contains("{P2} came back."), "{}", bodies[1]);
    assert_eq!(stored_bodies(&folder, "aliases").len(), 2);
}

#[test]
fn a_refused_story_gives_no_player_an_id() {
    let folder = fresh_folder("refused-ids");
    let mut story = story(&folder);

    let refused = accept(&mut story, 1, "{Corvin} said |cff0000|r");
    drop(story);

    assert!(matches!(refused, Err(StoryError::BadStory)));
    assert!(stored_bodies(&folder, "aliases").is_empty());
}

#[test]
fn the_limit_of_a_story_holds_for_the_text_without_the_marks() {
    let folder = fresh_folder("limit-marks");
    let mut story = story(&folder);
    let words = "{Al} ".repeat(MAX_STORY_BYTES / 3);

    accept(&mut story, 1, words.trim_end()).unwrap();

    assert_eq!(stories(&mut story).len(), 1);
}

#[test]
fn a_player_described_with_no_name_is_refused() {
    let folder = fresh_folder("bad-player");
    let mut story = story(&folder);

    let refused = story.handle(Input::PlayerDescribed {
        at: Tick(5),
        name: "P7".to_string(),
        race: None,
        class: None,
    });

    assert!(matches!(refused, Err(StoryError::BadName)));
}

#[test]
fn a_row_that_names_a_player_twice_ends_the_table_at_the_open() {
    let folder = fresh_folder("twice");
    let mut story = story(&folder);
    accept(&mut story, 1, "{Corvin} came.").unwrap();
    drop(story);
    let path = folder.join("worlds/r_Stormrage/c_Ada.sqlite");
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .execute(
            "INSERT INTO aliases (position, body) VALUES (1, '{\"name\":\"CORVIN\"}')",
            [],
        )
        .unwrap();
    drop(connection);

    let mut story = self::story(&folder);
    accept(&mut story, 2, "{Bob} came.").unwrap();
    drop(story);

    let rows = stored_bodies(&folder, "aliases");
    assert_eq!(rows.len(), 2);
    assert!(rows[1].contains("Bob"), "{}", rows[1]);
}
