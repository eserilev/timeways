//! The ratings of the player (GAMEPLAY.md 3.2.2): the desktop keeps the text that it showed,
//! and an export holds no name of a real player.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_rules::aliases::Alias;
use timeways_story::aliases::alias_of;
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::race_class::{Class, Race};
use timeways_story::ratings::{Rated, RatedLine, Rating, export, shareable};
use timeways_story::store::{CharacterKey, RowLog, Store};
use timeways_story::story::{Output, Story};

const REALM: &str = "Testrealm";
const NAME: &str = "Tester";

/// A sentence of history about the people of a human hero, and its answer.
const HISTORY_ANSWER: &str = "{\"lore\": \"King Barathen Wrynn scattered the gnoll packs of Elwynn, and his line still rules Stormwind City now.\", \"group\": \"g.people\"}";

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ratings-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    folder
}

/// A human paladin with the lore of Stormwind, so a tenth level has lore to tell.
fn paladin(folder: &Path) -> Story {
    let pack_path = folder.with_extension("pack.sqlite");
    let _ = std::fs::remove_file(&pack_path);
    let stormwind = Passage {
        text: "King Barathen Wrynn scattered the gnolls, and his line rules Stormwind City."
            .to_string(),
        source: "the wiki page \"Stormwind City\"".to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: Some("Stormwind City".to_string()),
        depends_on: Vec::new(),
        setup_for: None,
    };
    Pack::write(&pack_path, &[stormwind]).unwrap();
    let pack = Pack::open(&pack_path).unwrap();
    let mut story = Story::new(pack, Store::Folder(folder.to_path_buf()));
    let lines = [
        Input::CharacterEntered {
            realm: REALM.to_string(),
            name: NAME.to_string(),
        },
        Input::CharacterDescribed {
            at: Tick(1),
            race: Race::Human,
            class: Class::Paladin,
        },
    ];
    for line in lines {
        story.handle(line).unwrap();
    }
    story
}

/// Level 20 at the end of a batch: the narrator call, with its id.
fn level_twenty(story: &mut Story) -> CallId {
    for (at, level) in [(2, 19), (3, 20)] {
        story
            .handle(Input::LevelReached {
                at: Tick(at),
                level,
            })
            .unwrap();
    }
    let output = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    match output.as_slice() {
        [Output::ModelCall { call, .. }] => *call,
        other => panic!("expected a narrator call, got {other:?}"),
    }
}

fn answered(story: &mut Story, call: CallId, text: &str) -> Vec<Output> {
    story
        .handle(Input::ModelAnswered {
            call,
            text: text.to_string(),
        })
        .unwrap()
}

fn rate(story: &mut Story, rated: Rated, first: Option<u64>, rating: Rating) {
    let line = Input::LineRated {
        at: Tick(10),
        rated,
        first,
        rating,
    };
    assert!(story.handle(line).unwrap().is_empty());
}

/// The rows of `ratings` in the world file.
fn ratings_of(folder: &Path) -> Vec<RatedLine> {
    let key = CharacterKey::new(REALM, NAME).unwrap();
    let connection = rusqlite::Connection::open(folder.join(key.relative_path())).unwrap();
    let mut statement = connection
        .prepare("SELECT body FROM ratings ORDER BY position")
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|body| serde_json::from_str(&body.unwrap()).unwrap())
        .collect()
}

#[test]
fn a_rating_of_the_newest_narrator_line_keeps_its_text_and_its_moment() {
    let folder = folder("narrator");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, HISTORY_ANSWER);

    rate(&mut story, Rated::Narrator, None, Rating::Up);

    let rows = ratings_of(&folder);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rating, Rating::Up);
    assert_eq!(rows[0].moment, "level_up");
    assert!(
        rows[0].text.starts_with("King Barathen Wrynn"),
        "{}",
        rows[0].text
    );
    assert!(rows[0].faults.is_empty());
}

#[test]
fn a_rating_of_a_retried_line_keeps_why_the_first_answer_was_refused() {
    let folder = folder("retried");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    let first = "{\"lore\": \"On the 23rd day King Barathen Wrynn scattered the gnolls of Elwynn, and his line rules Stormwind.\", \"group\": \"g.people\"}";
    let retry = match answered(&mut story, call, first).as_slice() {
        [Output::ModelCall { call, .. }] => *call,
        other => panic!("expected a retry, got {other:?}"),
    };
    answered(&mut story, retry, HISTORY_ANSWER);

    rate(&mut story, Rated::Narrator, None, Rating::Down);

    let rows = ratings_of(&folder);
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0]
            .faults
            .iter()
            .any(|fault| fault.contains("The number 23 is not in the moment.")),
        "{:?}",
        rows[0].faults
    );
}

#[test]
fn a_rating_with_no_line_shown_writes_no_row() {
    let folder = folder("nothing-shown");
    let mut story = paladin(&folder);

    rate(&mut story, Rated::Narrator, None, Rating::Up);
    rate(&mut story, Rated::Chapter, Some(0), Rating::Up);
    rate(&mut story, Rated::Tale, Some(0), Rating::Down);
    rate(&mut story, Rated::Summary, None, Rating::Up);

    assert!(ratings_of(&folder).is_empty());
}

#[test]
fn a_silent_line_leaves_the_line_before_it_to_rate() {
    let folder = folder("silent");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, "SILENCE");

    rate(&mut story, Rated::Narrator, None, Rating::Up);

    assert!(ratings_of(&folder).is_empty());
}

#[test]
fn a_rating_line_holds_no_text_of_the_addon() {
    let line = r#"{"type":"line_rated","at":1,"rated":"narrator","rating":"up"}"#;
    let input: Input = serde_json::from_str(line).unwrap();

    assert_eq!(
        input,
        Input::LineRated {
            at: Tick(1),
            rated: Rated::Narrator,
            first: None,
            rating: Rating::Up,
        }
    );
}

fn row(rated: Rated, key: Option<u64>, rating: Rating, text: &str) -> RatedLine {
    RatedLine {
        at: Tick(1),
        rated,
        key,
        rating,
        moment: "chapter".to_string(),
        text: text.to_string(),
        faults: Vec::new(),
    }
}

fn table(names: &[&str]) -> Vec<Alias> {
    names.iter().filter_map(|name| alias_of(name)).collect()
}

#[test]
fn an_export_keeps_the_newest_rating_of_each_text() {
    let mut log = RowLog::default();
    log.add(row(Rated::Chapter, Some(4), Rating::Up, "First."))
        .unwrap();
    log.add(row(Rated::Tale, Some(4), Rating::Up, "A tale."))
        .unwrap();
    log.add(row(Rated::Chapter, Some(4), Rating::Down, "First."))
        .unwrap();

    let exported = export(&log, &[], NAME, "llama3.2:3b");

    assert_eq!(exported.len(), 2);
    assert_eq!(exported[0].rated, Rated::Tale);
    assert_eq!(exported[1].rating, Rating::Down);
    assert_eq!(exported[1].model, "llama3.2:3b");
}

#[test]
fn an_export_swaps_each_known_name_for_its_id_and_the_own_name_for_the_mark() {
    let players = table(&["Corvin", "Ada"]);

    let text = shareable(
        "Tester and corvin held the bridge, and Ada-Stormrage watched {P1} and $N.",
        &players,
        NAME,
    );

    assert_eq!(
        text,
        "$N and {P1} held the bridge, and {P2} watched {P1} and $N."
    );
}

#[test]
fn a_name_inside_a_longer_word_stays() {
    let text = shareable("Adamant stood at the gate.", &table(&["Ada"]), NAME);

    assert_eq!(text, "Adamant stood at the gate.");
}
