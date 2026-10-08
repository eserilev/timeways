//! The ratings of the player (GAMEPLAY.md 3.2.2): a rating names the exact text, the desktop
//! keeps the text that it showed, and an export holds no name of a real player.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::{FakeBridge, Model, Reply};
use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_rules::aliases::Alias;
use timeways_story::aliases::alias_of;
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::race_class::{Class, Race};
use timeways_story::ratings::{Rated, RatedLine, Rating, Reason, export, shareable};
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
        text:
            "King Barathen Wrynn scattered the gnolls of Elwynn, and his line rules Stormwind City."
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

/// A rating of the player, with no key and no reason.
fn rating(rated: Rated, rating: Rating) -> Input {
    Input::LineRated {
        at: Tick(10),
        rated,
        first: None,
        line: None,
        rating,
        reason: None,
    }
}

fn of_line(line: u64, rating: Rating, reason: Option<Reason>) -> Input {
    Input::LineRated {
        at: Tick(10),
        rated: Rated::Narrator,
        first: None,
        line: Some(line),
        rating,
        reason,
    }
}

fn of_first(rated: Rated, first: u64, rating: Rating) -> Input {
    Input::LineRated {
        at: Tick(10),
        rated,
        first: Some(first),
        line: None,
        rating,
        reason: None,
    }
}

fn rate(story: &mut Story, line: Input) {
    assert!(story.handle(line).unwrap().is_empty());
}

/// The rows of the accepted narrator calls, oldest first: the IDs of the lines that showed.
fn shown_lines(folder: &Path) -> Vec<u64> {
    let key = CharacterKey::new(REALM, NAME).unwrap();
    let connection = rusqlite::Connection::open(folder.join(key.relative_path())).unwrap();
    let mut statement = connection
        .prepare(
            "SELECT position FROM calls WHERE kind = 'narrator' AND result = 'accepted' \
             ORDER BY position",
        )
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, i64>(0))
        .unwrap()
        .map(|position| u64::try_from(position.unwrap()).unwrap())
        .collect()
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

/// Level 30 a few seconds after level 20: a second narrator call in the same minute.
fn level_thirty(story: &mut Story) -> CallId {
    for (at, level) in [(20, 29), (21, 30)] {
        story
            .handle(Input::LevelReached {
                at: Tick(at),
                level,
            })
            .unwrap();
    }
    let output = story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();
    match output.as_slice() {
        [Output::ModelCall { call, .. }] => *call,
        other => panic!("expected a narrator call, got {other:?}"),
    }
}

/// A second true sentence of the same lore, so the two lines differ.
const SECOND_ANSWER: &str = "{\"lore\": \"The line of King Barathen Wrynn rules Stormwind City, and the gnolls of Elwynn remember how he scattered them.\", \"group\": \"g.people\"}";

#[test]
fn a_rating_of_a_narrator_line_keeps_the_text_and_the_moment_of_its_call() {
    let folder = folder("narrator");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, HISTORY_ANSWER);
    let line = shown_lines(&folder)[0];

    rate(&mut story, of_line(line, Rating::Up, None));

    let rows = ratings_of(&folder);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].rating, Rating::Up);
    assert_eq!(rows[0].key, Some(line));
    assert_eq!(rows[0].moment, "level_up");
    assert!(
        rows[0].text.starts_with("King Barathen Wrynn"),
        "{}",
        rows[0].text
    );
    assert!(rows[0].faults.is_empty());
}

#[test]
fn a_narrator_reply_carries_the_id_of_its_line() {
    let folder = folder("reply-id");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);

    let output = answered(&mut story, call, HISTORY_ANSWER);

    let line = shown_lines(&folder)[0];
    match output.as_slice() {
        [Output::EventsSeen { narrator_id, .. }] => assert_eq!(*narrator_id, Some(line)),
        other => panic!("expected a narrator line, got {other:?}"),
    }
}

/// The bridge passes the ID on to the game, so the [Rate] link can name the line.
#[test]
fn the_id_of_a_narrator_line_reaches_the_game_through_the_bridge() {
    let folder = folder("reply-id-bridge");
    let story = paladin(&folder);
    let model: Model = Box::new(|_| Some(HISTORY_ANSWER.to_string()));
    let mut bridge = FakeBridge::new(story).with_model(model);
    let character = format!(r#"{{"type":"character_entered","realm":"{REALM}","name":"{NAME}"}}"#);

    let reply = bridge.batch(&format!(
        "{character}\n{}\n{}",
        r#"{"type":"level_reached","at":2,"level":19}"#,
        r#"{"type":"level_reached","at":3,"level":20}"#
    ));

    let Reply::Done(text) = reply else {
        panic!("{reply:?}")
    };
    let line = shown_lines(&folder)[0];
    assert!(text.contains(&format!(r#""narrator_id":{line}"#)), "{text}");
}

#[test]
fn a_quiet_reply_carries_no_line_id() {
    let folder = folder("quiet-id");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);

    let output = answered(&mut story, call, "SILENCE");

    match output.as_slice() {
        [Output::EventsSeen { narrator_id, .. }] => assert_eq!(*narrator_id, None),
        other => panic!("expected a quiet reply, got {other:?}"),
    }
}

#[test]
fn a_rating_names_the_older_of_two_lines_of_one_minute() {
    let folder = folder("two-lines");
    let mut story = paladin(&folder);
    let first = level_twenty(&mut story);
    answered(&mut story, first, HISTORY_ANSWER);
    let second = level_thirty(&mut story);
    answered(&mut story, second, SECOND_ANSWER);
    let lines = shown_lines(&folder);
    assert_eq!(lines.len(), 2, "{lines:?}");

    rate(&mut story, of_line(lines[0], Rating::Down, None));

    let rows = ratings_of(&folder);
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0].text.starts_with("King Barathen Wrynn"),
        "{}",
        rows[0].text
    );
}

#[test]
fn a_rating_of_a_line_still_finds_it_after_a_restart() {
    let folder = folder("restart");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, HISTORY_ANSWER);
    let line = shown_lines(&folder)[0];
    drop(story);
    let mut story = paladin(&folder);

    rate(&mut story, of_line(line, Rating::Up, None));

    assert_eq!(ratings_of(&folder).len(), 1);
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
    let line = shown_lines(&folder)[0];

    rate(&mut story, of_line(line, Rating::Down, None));

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

    rate(&mut story, rating(Rated::Narrator, Rating::Up));
    rate(&mut story, of_line(0, Rating::Up, None));
    rate(&mut story, of_first(Rated::Chapter, 0, Rating::Up));
    rate(&mut story, of_first(Rated::Tale, 0, Rating::Down));
    rate(&mut story, rating(Rated::Summary, Rating::Up));

    assert!(ratings_of(&folder).is_empty());
}

#[test]
fn a_silent_line_has_no_line_to_rate() {
    let folder = folder("silent");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, "SILENCE");

    rate(&mut story, of_line(call.0, Rating::Up, None));

    assert!(shown_lines(&folder).is_empty());
    assert!(ratings_of(&folder).is_empty());
}

#[test]
fn a_dislike_keeps_its_reason_and_the_export_holds_it() {
    let folder = folder("reason");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, HISTORY_ANSWER);
    let line = shown_lines(&folder)[0];

    rate(
        &mut story,
        of_line(line, Rating::Down, Some(Reason::TooLong)),
    );

    let rows = ratings_of(&folder);
    assert_eq!(rows[0].reason, Some(Reason::TooLong));
    let mut log = RowLog::default();
    log.add(rows[0].clone()).unwrap();
    let exported = export(&log, &[], NAME, "none");
    let value = serde_json::to_value(&exported).unwrap();
    assert_eq!(value[0]["reason"], "too_long");
}

#[test]
fn a_like_keeps_no_reason() {
    let folder = folder("like-reason");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, HISTORY_ANSWER);
    let line = shown_lines(&folder)[0];

    rate(&mut story, of_line(line, Rating::Up, Some(Reason::Boring)));

    assert_eq!(ratings_of(&folder)[0].reason, None);
}

/// The reasons of a lore answer are for the lore book only.
#[test]
fn a_reason_of_a_lore_answer_on_a_narrator_line_keeps_no_reason() {
    let folder = folder("lore-reason");
    let mut story = paladin(&folder);
    let call = level_twenty(&mut story);
    answered(&mut story, call, HISTORY_ANSWER);
    let line = shown_lines(&folder)[0];

    rate(
        &mut story,
        of_line(line, Rating::Down, Some(Reason::Spoiler)),
    );

    let rows = ratings_of(&folder);
    assert_eq!(rows[0].rating, Rating::Down);
    assert_eq!(rows[0].reason, None);
}

#[test]
fn a_rating_line_holds_no_text_of_the_addon() {
    let line = r#"{"type":"line_rated","at":1,"rated":"narrator","line":7,"rating":"down","reason":"made_up_name"}"#;
    let input: Input = serde_json::from_str(line).unwrap();

    assert_eq!(
        input,
        Input::LineRated {
            at: Tick(1),
            rated: Rated::Narrator,
            first: None,
            line: Some(7),
            rating: Rating::Down,
            reason: Some(Reason::MadeUpName),
        }
    );
}

#[test]
fn a_rating_line_with_an_unknown_reason_is_refused() {
    let line = r#"{"type":"line_rated","at":1,"rated":"narrator","line":7,"rating":"down","reason":"rude"}"#;

    assert!(serde_json::from_str::<Input>(line).is_err());
}

fn row(rated: Rated, key: Option<u64>, rating: Rating, text: &str) -> RatedLine {
    RatedLine {
        at: Tick(1),
        rated,
        key,
        rating,
        reason: None,
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
