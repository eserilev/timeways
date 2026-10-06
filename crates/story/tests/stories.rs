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
use timeways_story::stories::{
    MAX_BODY_BYTES, MAX_BODY_CHARS, MAX_PARAGRAPHS, MAX_TITLE_BYTES, PlayerStory,
};
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

fn accept_told(
    story: &mut Story,
    number: u64,
    title: Option<&str>,
    paragraphs: &[&str],
) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::StoryAccepted {
        at: Tick(10),
        number,
        title: title.map(str::to_string),
        paragraphs: paragraphs.iter().map(|text| (*text).to_string()).collect(),
    })
}

/// A quick story: one paragraph and no title.
fn accept(story: &mut Story, number: u64, text: &str) -> Result<Vec<Output>, StoryError> {
    accept_told(story, number, None, &[text])
}

fn is_bad(result: &Result<Vec<Output>, StoryError>) -> bool {
    matches!(result, Err(StoryError::BadStory))
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
        title: None,
        paragraphs: vec![TOLD.to_string()],
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
        String::new(),
        "a".repeat(MAX_BODY_BYTES + 1),
        "a line\nand another".to_string(),
        "a fake |Hlink|h".to_string(),
    ] {
        assert!(is_bad(&accept(&mut story, 1, &text)), "{text:?}");
    }
    assert!(accept(&mut story, 1, &"a".repeat(MAX_BODY_CHARS)).is_ok());
}

#[test]
fn a_story_with_a_title_and_three_paragraphs_is_kept() {
    let folder = fresh_folder("title-and-three");
    let mut story = story(&folder);

    accept_told(
        &mut story,
        1,
        Some("The Fire at Solliden"),
        &[
            "We went in.",
            "She stood in the doorway.",
            "The barn burned.",
        ],
    )
    .unwrap();

    let told = &stories(&mut story)[0];
    assert_eq!(told.title.as_deref(), Some("The Fire at Solliden"));
    assert_eq!(told.paragraphs.len(), 3);
}

#[test]
fn a_story_with_no_title_is_kept() {
    let folder = fresh_folder("no-title");
    let mut story = story(&folder);

    accept_told(&mut story, 1, Some(""), &[TOLD]).unwrap();
    accept_told(&mut story, 2, None, &[TOLD]).unwrap();

    let titles: Vec<Option<String>> = stories(&mut story)
        .into_iter()
        .map(|told| told.title)
        .collect();
    assert_eq!(titles, [None, None]);
}

#[test]
fn an_empty_paragraph_refuses_the_story() {
    let folder = fresh_folder("empty-paragraph");
    let mut story = story(&folder);

    let refused = accept_told(&mut story, 1, None, &["We went in.", "", "We left."]);

    assert!(is_bad(&refused));
}

#[test]
fn a_paragraph_with_a_space_at_its_end_refuses_the_story() {
    let folder = fresh_folder("space-paragraph");
    let mut story = story(&folder);

    for paragraph in ["We went in. ", " We went in."] {
        let refused = accept_told(&mut story, 1, None, &[paragraph]);
        assert!(is_bad(&refused), "{paragraph:?}");
    }
}

#[test]
fn a_story_with_no_paragraph_is_refused() {
    let folder = fresh_folder("no-paragraph");
    let mut story = story(&folder);

    let refused = accept_told(&mut story, 1, Some("A Title"), &[]);

    assert!(is_bad(&refused));
}

#[test]
fn a_story_with_twenty_paragraphs_is_kept_and_twenty_one_are_refused() {
    let folder = fresh_folder("twenty");
    let mut story = story(&folder);
    let many = ["Short."; MAX_PARAGRAPHS + 1];

    let refused = accept_told(&mut story, 1, None, &many);
    let kept = accept_told(&mut story, 2, None, &many[..MAX_PARAGRAPHS]);

    assert!(is_bad(&refused));
    assert!(kept.is_ok());
}

#[test]
fn a_paragraph_with_a_line_break_refuses_the_story() {
    let folder = fresh_folder("break");
    let mut story = story(&folder);

    let refused = accept_told(&mut story, 1, None, &["We went in.\nWe left."]);

    assert!(is_bad(&refused));
}

#[test]
fn a_c1_control_character_refuses_the_story() {
    let folder = fresh_folder("c1");
    let mut story = story(&folder);

    let refused = accept_told(&mut story, 1, None, &["We went\u{85} in."]);

    assert!(is_bad(&refused));
}

#[test]
fn a_title_with_a_bar_or_a_control_character_is_refused() {
    let folder = fresh_folder("bad-title");
    let mut story = story(&folder);

    for title in ["A |cffff0000red|r tale", "A\ttale", "A\u{9f}tale"] {
        let refused = accept_told(&mut story, 1, Some(title), &[TOLD]);
        assert!(is_bad(&refused), "{title:?}");
    }
}

#[test]
fn a_body_at_the_byte_limit_is_kept_and_one_byte_more_is_refused() {
    let folder = fresh_folder("body-bytes");
    let mut story = story(&folder);
    // 399 letters of two bytes in two paragraphs, a break, and plain letters up to the limit.
    let first = "é".repeat(200);
    let second = format!(
        "{}{}",
        "é".repeat(199),
        "a".repeat(MAX_BODY_BYTES - 400 - 1 - 398)
    );

    let kept = accept_told(&mut story, 1, None, &[&first, &second]);
    let refused = accept_told(&mut story, 2, None, &[&first, &format!("{second}a")]);

    assert!(kept.is_ok(), "{kept:?}");
    assert!(is_bad(&refused));
}

#[test]
fn a_body_at_the_letter_limit_is_kept_and_one_letter_more_is_refused() {
    let folder = fresh_folder("body-letters");
    let mut story = story(&folder);
    let half = "a".repeat(MAX_BODY_CHARS / 2);
    let rest = "a".repeat(MAX_BODY_CHARS / 2 - 1);

    let kept = accept_told(&mut story, 1, None, &[&half, &rest]);
    let refused = accept_told(&mut story, 2, None, &[&half, &half]);

    assert!(kept.is_ok());
    assert!(is_bad(&refused));
}

#[test]
fn a_title_at_the_byte_limit_is_kept_and_one_byte_more_is_refused() {
    let folder = fresh_folder("title-bytes");
    let mut story = story(&folder);
    let title = "é".repeat(MAX_TITLE_BYTES / 2);

    let kept = accept_told(&mut story, 1, Some(&title), &[TOLD]);
    let refused = accept_told(&mut story, 2, Some(&format!("{title}a")), &[TOLD]);

    assert!(kept.is_ok());
    assert!(is_bad(&refused));
}

#[test]
fn names_leave_the_title_too() {
    let folder = fresh_folder("title-names");
    let mut story = story(&folder);

    accept_told(
        &mut story,
        1,
        Some("{Corvin} at the Bridge"),
        &["We held it."],
    )
    .unwrap();

    assert_eq!(
        stories(&mut story)[0].title.as_deref(),
        Some("Corvin at the Bridge")
    );
    drop(story);
    let body = &stored_bodies(&folder, "stories")[0];
    assert!(body.contains("{P1} at the Bridge"), "{body}");
}

#[test]
fn the_journal_carries_the_title_and_the_paragraphs() {
    let folder = fresh_folder("journal-shape");
    let mut story = story(&folder);
    accept_told(&mut story, 1, Some("A Tale"), &["One.", "Two."]).unwrap();

    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();

    let line = serde_json::to_string(&outputs[0]).unwrap();
    assert!(
        line.contains(r#""title":"A Tale","paragraphs":["One.","Two."]"#),
        "{line}"
    );
}

#[test]
fn a_long_shelf_of_stories_spreads_over_pages() {
    let folder = fresh_folder("long-shelf");
    let mut story = story(&folder);
    let paragraph = "a".repeat(MAX_BODY_CHARS / 2 - 1);
    for number in 0..40 {
        accept_told(&mut story, number, None, &[&paragraph, &paragraph]).unwrap();
    }

    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();

    let Some(Output::Journal { page, .. }) = outputs.into_iter().next() else {
        panic!("expected a journal");
    };
    assert!(page.pages > 1, "{}", page.pages);
    assert!(page.journal.stories.len() < 40);
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

#[test]
fn a_damaged_shelf_that_accepts_one_number_twice_loads_it_once() {
    let folder = fresh_folder("damaged-shelf");
    let mut first = story(&folder);
    accept(&mut first, 1, TOLD).unwrap();
    drop(first);
    let world = folder.join("worlds/r_Stormrage/c_Ada.sqlite");
    let copy =
        "INSERT INTO stories (position, body) SELECT 1, body FROM stories WHERE position = 0";
    rusqlite::Connection::open(world)
        .unwrap()
        .execute(copy, [])
        .unwrap();

    let mut second = story(&folder);

    let numbers: Vec<u64> = stories(&mut second)
        .iter()
        .map(|told| told.number)
        .collect();
    assert_eq!(numbers, [1]);
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
            shape: None,
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
        stories(&mut story)[0].paragraphs,
        ["Corvin and $N held the bridge. Corvin ran."]
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

    assert!(is_bad(&refused));
    assert!(stored_bodies(&folder, "aliases").is_empty());
}

#[test]
fn the_limit_of_a_story_holds_for_the_text_without_the_marks() {
    let folder = fresh_folder("limit-marks");
    let mut story = story(&folder);
    let words = "{Al} ".repeat(MAX_BODY_CHARS / 3);
    assert!(words.trim_end().len() > MAX_BODY_BYTES);

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

/// Version 6 changed the shape of a story. A world of version 5 holds stories of one
/// text, so it is refused and left as it is (GAMEPLAY.md 5.7).
#[test]
fn a_world_of_version_five_is_refused() {
    let folder = fresh_folder("version-five");
    let file = folder.join("worlds/r_Stormrage/c_Ada.sqlite");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    rusqlite::Connection::open(&file)
        .unwrap()
        .execute_batch("CREATE TABLE stories (position INTEGER PRIMARY KEY, body TEXT); PRAGMA user_version = 5;")
        .unwrap();

    let opened = Store::Folder(folder).open(&key());

    assert!(matches!(
        opened,
        Err(timeways_story::store::StoreError::OtherVersion { version: 5, .. })
    ));
}
