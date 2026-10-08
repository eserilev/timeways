#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::{EventId, Tick};
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use timeways_story::input::{Input, MessageId};
use timeways_story::narrator::Budget;
use timeways_story::pace::Pace;
use timeways_story::pack::Pack;
use timeways_story::ratings::RatedLine;
use timeways_story::store::{
    CallEnd, CharacterKey, Database, Line, NewCall, Outcome, PROMPTS_KEPT, Store, StoreError,
    Table, name_of_safe_id, safe_id,
};
use timeways_story::story::{Output, Story, StoryError};

/// The one output of an input, or none.
fn one(outputs: Vec<Output>) -> Option<Output> {
    assert!(outputs.len() <= 1, "{outputs:?}");
    outputs.into_iter().next()
}

fn fresh_folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("store-{name}"));
    let _ = fs::remove_dir_all(&folder);
    let _ = fs::remove_file(folder.with_extension("sqlite"));
    folder
}

/// Each test has its own folder, so parallel tests never write one pack at once.
fn empty_pack(folder: &Path) -> Pack {
    let path = folder.with_extension("sqlite");
    if !path.exists() {
        Pack::write(&path, &[]).unwrap();
    }
    Pack::open(&path).unwrap()
}

fn story(folder: &Path, name: &str) -> Story {
    let mut story = Story::new(empty_pack(folder), Store::Folder(folder.to_path_buf()));
    let character = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: name.to_string(),
    };
    one(story.handle(character).unwrap());
    story
}

fn enter(story: &mut Story, at: u64, zone: &str) {
    let input = Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: None,
        spot: None,
        hour: None,
        taxi: None,
    };
    one(story.handle(input).unwrap());
}

fn places(story: &mut Story) -> Vec<String> {
    let output = one(story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap());
    let Some(Output::Journal { page, .. }) = output else {
        panic!("expected a journal, got {output:?}");
    };
    page.journal
        .places
        .into_iter()
        .map(|place| place.name)
        .collect()
}

fn world_file(folder: &Path, name: &str) -> PathBuf {
    folder
        .join("worlds")
        .join("r_Stormrage")
        .join(format!("c_{name}.sqlite"))
}

fn bodies(folder: &Path, table: &str) -> Vec<String> {
    let connection = Connection::open(world_file(folder, "Ada")).unwrap();
    let select = format!("SELECT body FROM {table} ORDER BY position");
    let mut statement = connection.prepare(&select).unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Adds a row at the end of a table, as another program can.
fn insert(folder: &Path, table: &str, body: &str) {
    let connection = Connection::open(world_file(folder, "Ada")).unwrap();
    let insert =
        format!("INSERT INTO {table} (position, body) VALUES ((SELECT count(*) FROM {table}), ?1)");
    connection.execute(&insert, params![body]).unwrap();
}

#[test]
fn a_safe_id_keeps_letters_and_digits() {
    assert_eq!(safe_id("Grimtusk42"), "Grimtusk42");
}

#[test]
fn a_safe_id_encodes_every_other_byte() {
    assert_eq!(
        safe_id("Quel'Thalas é/.._"),
        "Quel_27Thalas_20_C3_A9_2F_2E_2E_5F"
    );
}

#[test]
fn two_names_never_share_a_safe_id() {
    assert_ne!(safe_id("a_20"), safe_id("a "));
    assert_ne!(safe_id("A"), safe_id("_41"));
}

#[test]
fn a_key_refuses_an_empty_a_long_or_a_control_name() {
    assert!(CharacterKey::new("Stormrage", "").is_err());
    assert!(CharacterKey::new("", "Ada").is_err());
    assert!(CharacterKey::new("Stormrage", &"a".repeat(49)).is_err());
    assert!(CharacterKey::new(&"r".repeat(65), "Ada").is_err());
    assert!(CharacterKey::new("Stormrage", "A\nda").is_err());
    assert!(CharacterKey::new(&"r".repeat(64), &"a".repeat(48)).is_ok());
}

#[test]
fn the_world_survives_a_restart() {
    let folder = fresh_folder("restart");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    enter(&mut first, 2, "Westfall");
    drop(first);

    let mut second = story(&folder, "Ada");

    assert_eq!(places(&mut second), ["Elwynn Forest", "Westfall"]);
}

#[test]
fn events_after_a_restart_follow_the_old_ones() {
    let folder = fresh_folder("after-restart");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);
    let mut second = story(&folder, "Ada");
    enter(&mut second, 2, "Westfall");
    drop(second);

    let mut third = story(&folder, "Ada");

    assert_eq!(places(&mut third), ["Elwynn Forest", "Westfall"]);
}

#[test]
fn two_characters_never_share_a_world() {
    let folder = fresh_folder("two-characters");
    let mut ada = story(&folder, "Ada");
    enter(&mut ada, 1, "Elwynn Forest");
    drop(ada);

    let mut bren = story(&folder, "Bren");

    assert!(places(&mut bren).is_empty());
}

#[test]
fn switching_characters_in_one_run_switches_worlds() {
    let folder = fresh_folder("switch");
    let mut story = story(&folder, "Ada");
    enter(&mut story, 1, "Elwynn Forest");
    let bren = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: "Bren".to_string(),
    };
    one(story.handle(bren).unwrap());
    enter(&mut story, 2, "Durotar");

    let ada = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: "Ada".to_string(),
    };
    one(story.handle(ada).unwrap());

    assert_eq!(places(&mut story), ["Elwynn Forest"]);
}

#[test]
fn a_row_that_does_not_read_is_cut_and_the_rest_stays() {
    let folder = fresh_folder("damaged");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);
    insert(&folder, "events", r#"{"id":99,"tick":"#);

    let mut second = story(&folder, "Ada");
    enter(&mut second, 2, "Westfall");
    drop(second);

    let mut third = story(&folder, "Ada");
    assert_eq!(places(&mut third), ["Elwynn Forest", "Westfall"]);
    assert!(!bodies(&folder, "events").concat().contains("\"id\":99"));
}

#[test]
fn an_event_with_the_wrong_position_ends_the_history() {
    let folder = fresh_folder("wrong-position");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);
    let events = bodies(&folder, "events");
    let connection = Connection::open(world_file(&folder, "Ada")).unwrap();
    let swap = "UPDATE events SET body = ?1 WHERE position = ?2";
    connection.execute(swap, params![events[2], 1]).unwrap();
    connection.execute(swap, params![events[1], 2]).unwrap();
    drop(connection);

    let mut second = story(&folder, "Ada");

    assert!(places(&mut second).is_empty());
    assert_eq!(bodies(&folder, "events"), [events[0].clone()]);
}

#[test]
fn a_body_that_is_not_text_ends_its_table() {
    let folder = fresh_folder("blob-body");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);
    let connection = Connection::open(world_file(&folder, "Ada")).unwrap();
    let blob = "INSERT INTO events (position, body) VALUES ((SELECT count(*) FROM events), ?1)";
    connection.execute(blob, params![vec![0xC3_u8]]).unwrap();
    drop(connection);

    let mut second = story(&folder, "Ada");

    assert_eq!(places(&mut second), ["Elwynn Forest"]);
}

#[test]
fn a_history_that_another_program_wrote_is_refused() {
    let folder = fresh_folder("foreign");
    let file = world_file(&folder, "Ada");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    Store::Folder(folder.clone())
        .open(&CharacterKey::new("Stormrage", "Ada").unwrap())
        .unwrap();
    let foreign = r#"{"id":0,"tick":0,"kind":{"EntityCreated":{"id":0,"entity_type":"Place","name":"Nowhere"}}}"#;
    insert(&folder, "events", foreign);
    let mut story = Story::new(empty_pack(&folder), Store::Folder(folder.clone()));

    let result = story.handle(Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: "Ada".to_string(),
    });

    assert!(matches!(
        result,
        Err(StoryError::Store(StoreError::Foreign { .. }))
    ));
}

#[test]
fn a_kill_past_the_cap_still_moves_the_foe_and_is_saved() {
    let folder = fresh_folder("past-the-cap");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    for _ in 0..1000 {
        first
            .handle(Input::NpcDefeated {
                at: Tick(2),
                name: "Hogger".to_string(),
                kind: None,
            })
            .unwrap();
    }
    enter(&mut first, 3, "Westfall");
    let rows_before = bodies(&folder, "events").len();

    // Hogger moves to Westfall, and the count stays at 1000.
    let kill = first.handle(Input::NpcDefeated {
        at: Tick(4),
        name: "Hogger".to_string(),
        kind: None,
    });

    let rows_after = bodies(&folder, "events").len();
    assert!(kill.is_ok());
    assert_eq!(rows_after, rows_before + 1);
}

#[test]
fn a_name_like_a_windows_device_still_names_a_plain_file() {
    let folder = fresh_folder("device-name");
    let mut con = story(&folder, "Con");

    enter(&mut con, 1, "Elwynn Forest");

    assert!(world_file(&folder, "Con").is_file());
}

#[test]
fn a_refused_character_switch_leaves_no_character_active() {
    let folder = fresh_folder("refused-switch");
    let mut story = story(&folder, "Ada");
    let bad = Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: String::new(),
    };
    assert!(story.handle(bad).is_err());

    let result = story.handle(Input::ZoneEntered {
        at: Tick(1),
        zone: "Westfall".to_string(),
        subzone: None,
        spot: None,
        hour: None,
        taxi: None,
    });

    assert!(matches!(result, Err(StoryError::NoCharacter)));
    drop(story);
    assert!(places(&mut self::story(&folder, "Ada")).is_empty());
}

/// Another program that holds the write lock makes the save fail.
#[test]
fn a_failed_save_loses_its_line_and_the_next_line_saves() {
    let folder = fresh_folder("failed-save");
    let mut story = story(&folder, "Ada");
    enter(&mut story, 1, "Elwynn Forest");
    let other = Connection::open(world_file(&folder, "Ada")).unwrap();
    other.execute_batch("BEGIN IMMEDIATE").unwrap();

    let refused = story.handle(Input::ZoneEntered {
        at: Tick(2),
        zone: "Westfall".to_string(),
        subzone: None,
        spot: None,
        hour: None,
        taxi: None,
    });
    other.execute_batch("ROLLBACK").unwrap();
    let after_the_failure = places(&mut story);
    enter(&mut story, 3, "Duskwood");
    drop(story);

    assert!(matches!(refused, Err(StoryError::Store(_))));
    assert_eq!(after_the_failure, ["Elwynn Forest"]);
    let mut reloaded = self::story(&folder, "Ada");
    assert_eq!(places(&mut reloaded), ["Elwynn Forest", "Duskwood"]);
}

/// Ends a batch, and lets each model call of it fail, so no narrator call keeps the saga
/// waiting.
fn fail_each_call(story: &mut Story, batch: u64) {
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

#[test]
fn the_saga_survives_a_restart() {
    let folder = fresh_folder("saga-restart");
    let mut first = story(&folder, "Ada");
    // A meeting and 14 new camps on foot: the least weight of a chapter. Then a meeting in
    // a new zone: the step with weight that begins the second chapter.
    let entered = |at: u64, zone: &str, subzone: Option<String>| Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone,
        spot: None,
        hour: None,
        taxi: None,
    };
    let met = |at: u64, name: &str| Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
        spot: None,
    };
    let mut inputs = vec![
        entered(3600, "Westfall", None),
        met(3600, "Gryan Stoutmantle"),
    ];
    inputs.extend((1..=14).map(|n| entered(3600 + n * 60, "Westfall", Some(format!("Camp {n}")))));
    inputs.push(entered(5 * 3600, "Duskwood", None));
    for (batch, input) in inputs.into_iter().enumerate() {
        first.handle(input).unwrap();
        fail_each_call(&mut first, 90 + batch as u64);
    }
    first.handle(met(5 * 3600, "Madame Eva")).unwrap();
    let outputs = first.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    let text = r#"{"saga": "$N rode west to the farms of Westfall."}"#.to_string();
    first.handle(Input::ModelAnswered { call, text }).unwrap();
    drop(first);

    let mut second = story(&folder, "Ada");
    let output = one(second
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap());

    let Some(Output::Journal { page, .. }) = output else {
        panic!("expected a journal, got {output:?}");
    };
    assert_eq!(
        page.journal.chapters[0].prose.as_deref(),
        Some("$N rode west to the farms of Westfall.")
    );
    let second_batch = second.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();
    assert_eq!(second_batch.len(), 1);
}

#[test]
fn flavor_moments_survive_a_restart_and_keep_counting_toward_a_title() {
    let folder = fresh_folder("flavor-restart");
    let mut first = story(&folder, "Ada");
    let dance = |at| Input::EmoteDone {
        at: Tick(at),
        emote: "dance".to_string(),
        target: None,
        hour: None,
    };
    first
        .handle(Input::ZoneEntered {
            at: Tick(1),
            zone: "Elwynn Forest".to_string(),
            subzone: Some("Goldshire".to_string()),
            spot: None,
            hour: None,
            taxi: None,
        })
        .unwrap();
    first.handle(dance(2)).unwrap();
    first.handle(dance(3)).unwrap();
    drop(first);

    let mut second = story(&folder, "Ada");
    second.handle(dance(4)).unwrap();

    let output = one(second
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap());
    let Some(Output::Journal { page, .. }) = output else {
        panic!("expected a journal, got {output:?}");
    };
    let text = serde_json::to_string(&page.journal.deeds).unwrap();
    assert!(text.contains("Lord of the Goldshire Dance Floor"), "{text}");
}

#[test]
fn the_story_of_the_hero_survives_a_restart() {
    let folder = fresh_folder("hero-restart");
    let mut first = story(&folder, "Ada");
    first
        .handle(Input::HeroSet {
            at: Tick(1),
            field: "goal".to_string(),
            text: "Find my brother.".to_string(),
        })
        .unwrap();
    first
        .handle(Input::HeroAdded {
            at: Tick(2),
            text: "An oath.".to_string(),
            npc: None,
        })
        .unwrap();
    drop(first);

    let mut second = story(&folder, "Ada");
    second
        .handle(Input::HeroAdded {
            at: Tick(3),
            text: "A second oath.".to_string(),
            npc: None,
        })
        .unwrap();

    let output = one(second
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap());
    let Some(Output::Journal { page, .. }) = output else {
        panic!("expected a journal, got {output:?}");
    };
    let numbers: Vec<u64> = page
        .journal
        .hero
        .entries
        .iter()
        .map(|entry| entry.number)
        .collect();
    assert_eq!(page.journal.hero.sheet[0].text, "Find my brother.");
    assert_eq!(numbers, [1, 2]);
}

#[test]
fn a_file_that_is_not_a_database_is_an_error_and_stays() {
    let folder = fresh_folder("not-a-database");
    let file = world_file(&folder, "Ada");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "not a database at all, just some words").unwrap();

    let opened = Store::Folder(folder).open(&CharacterKey::new("Stormrage", "Ada").unwrap());

    assert!(matches!(opened, Err(StoreError::Sqlite { .. })));
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "not a database at all, just some words"
    );
}

/// A history that the program may not open is an error, never an empty world that then
/// takes the place of the real one.
#[cfg(unix)]
#[test]
fn a_history_that_cannot_be_opened_is_an_error_and_not_an_empty_world() {
    use std::os::unix::fs::PermissionsExt;
    let folder = fresh_folder("unreadable");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);
    let path = world_file(&folder, "Ada");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    let opened = Store::Folder(folder).open(&CharacterKey::new("Stormrage", "Ada").unwrap());

    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(opened, Err(StoreError::Sqlite { .. })));
}

#[test]
fn a_new_character_opens_with_empty_tables() {
    let folder = fresh_folder("new-files");
    let store = Store::Folder(folder);

    let opened = store
        .open(&CharacterKey::new("Stormrage", "Ada").unwrap())
        .unwrap();

    assert_eq!(opened.saved_events, 0);
    assert!(opened.prose.is_empty());
    assert_eq!(opened.prose.len(), 0);
}

#[test]
fn the_words_of_a_saga_come_back_after_a_restart() {
    let folder = fresh_folder("prose-back");
    let key = CharacterKey::new("Stormrage", "Ada").unwrap();
    let mut first = Store::Folder(folder.clone()).open(&key).unwrap();
    let written = timeways_story::store::Written {
        text: "$N came.".to_string(),
        footnotes: Vec::new(),
    };
    let span = timeways_story::store::SagaSpan {
        rule: 1,
        first: EventId(5),
        last: EventId(9),
    };
    first.prose.add(span, written).unwrap();
    let line = Line {
        rows: vec![(Table::Chapters, first.prose.take_unsaved())],
        ..Line::default()
    };
    first.database.save(&line).unwrap();

    let again = Store::Folder(folder).open(&key).unwrap();

    assert!(!again.prose.is_empty());
    assert_eq!(again.prose.len(), 1);
}

#[test]
fn a_history_with_events_opens_as_not_empty() {
    let folder = fresh_folder("not-empty");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);

    let opened = Store::Folder(folder)
        .open(&CharacterKey::new("Stormrage", "Ada").unwrap())
        .unwrap();

    assert!(opened.saved_events > 0);
}

/// A call whose line and parent call are both gone once failed the repair of its links.
/// A fuzzer found it.
#[test]
fn a_call_with_two_broken_links_opens_and_loses_both() {
    let folder = fresh_folder("two-broken-links");
    let mut first = story(&folder, "Ada");
    enter(&mut first, 1, "Elwynn Forest");
    drop(first);
    let connection = Connection::open(world_file(&folder, "Ada")).unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    connection
        .execute(
            "INSERT INTO calls (position, kind, input, call, pack, result)
             VALUES (5, 'talk', 77, 88, '', 'open')",
            [],
        )
        .unwrap();
    drop(connection);

    let opened =
        Store::Folder(folder.clone()).open(&CharacterKey::new("Stormrage", "Ada").unwrap());

    assert!(opened.is_ok(), "{:?}", opened.err());
    let links: (Option<i64>, Option<i64>) = Connection::open(world_file(&folder, "Ada"))
        .unwrap()
        .query_row(
            "SELECT input, call FROM calls WHERE position = 5",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(links, (None, None));
}

/// The narrator line of a batch, if the batch asks for one. Its call fails, and the line
/// still counts against the budget.
fn narrator_asked(story: &mut Story, batch: u64) -> bool {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    let mut asked = false;
    for output in outputs {
        if let Output::ModelCall { call, .. } = output {
            story.handle(Input::ModelFailed { call }).unwrap();
            asked = true;
        }
    }
    asked
}

#[test]
fn a_restart_keeps_the_budget_of_the_narrator() {
    let folder = fresh_folder("budget-restart");
    let mut first = story(&folder, "Ada");
    let mut asked = Vec::new();
    for (minute, zone) in [(1, "Elwynn Forest"), (2, "Westfall"), (3, "Duskwood")] {
        enter(&mut first, minute * 60, zone);
        asked.push(narrator_asked(&mut first, minute));
    }
    drop(first);
    let mut second = story(&folder, "Ada");

    enter(&mut second, 4 * 60, "Redridge Mountains");

    assert_eq!(asked, [true, true, true]);
    assert!(!narrator_asked(&mut second, 4));
}

#[test]
fn the_pace_of_the_model_comes_back_after_a_restart() {
    let folder = fresh_folder("pace-restart");
    let store = Store::Folder(folder.clone());
    let mut pace = Pace::default();
    pace.failed(Tick(1_000));
    store.open_shared().unwrap().save("pace", &pace).unwrap();

    let again: Pace = store.open_shared().unwrap().load("pace").unwrap();

    assert!(again.is_tight(Tick(1_060)));
}

#[test]
fn a_shared_value_that_does_not_read_comes_back_as_its_default() {
    let folder = fresh_folder("shared-bad");
    let store = Store::Folder(folder.clone());
    drop(store.open_shared().unwrap());
    Connection::open(folder.join("timeways.sqlite"))
        .unwrap()
        .execute(
            "INSERT INTO state (name, body) VALUES ('pace', 'not json')",
            [],
        )
        .unwrap();

    let pace: Pace = store.open_shared().unwrap().load("pace").unwrap();

    assert!(!pace.is_tight(Tick(1_000)));
}

#[test]
fn a_shared_value_that_is_not_text_comes_back_as_its_default() {
    let folder = fresh_folder("shared-blob");
    let store = Store::Folder(folder.clone());
    drop(store.open_shared().unwrap());
    let connection = Connection::open(folder.join("timeways.sqlite")).unwrap();
    connection
        .execute(
            "INSERT INTO state (name, body) VALUES ('pace', x'7b7d'), ('budget', CAST(x'ff' AS TEXT))",
            [],
        )
        .unwrap();

    let mut shared = store.open_shared().unwrap();
    let pace: Pace = shared.load("pace").unwrap();
    let budget: Budget = shared.load("budget").unwrap();

    assert!(!pace.is_tight(Tick(1_000)));
    assert_eq!(
        serde_json::to_string(&budget).unwrap(),
        serde_json::to_string(&Budget::default()).unwrap()
    );
}

fn shared_file(folder: &Path) -> PathBuf {
    fs::create_dir_all(folder).unwrap();
    folder.join("timeways.sqlite")
}

fn entered(story: &mut Story, name: &str) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::CharacterEntered {
        realm: "Stormrage".to_string(),
        name: name.to_string(),
    })
}

#[test]
fn a_shared_file_with_no_state_table_gets_one() {
    let folder = fresh_folder("shared-no-state");
    Connection::open(shared_file(&folder))
        .unwrap()
        .execute_batch("PRAGMA user_version = 1")
        .unwrap();
    let mut story = Story::new(empty_pack(&folder), Store::Folder(folder.clone()));

    let entered = entered(&mut story, "Ada");

    assert!(entered.is_ok(), "{entered:?}");
    assert!(story.take_notes().is_empty());
}

#[test]
fn a_shared_file_of_another_version_locks_no_character_out() {
    let folder = fresh_folder("shared-other-version");
    Connection::open(shared_file(&folder))
        .unwrap()
        .execute_batch("CREATE TABLE later (x); PRAGMA user_version = 99")
        .unwrap();
    let mut story = Story::new(empty_pack(&folder), Store::Folder(folder.clone()));

    let entered = entered(&mut story, "Ada");

    assert!(entered.is_ok(), "{entered:?}");
    assert_eq!(story.take_notes().len(), 1);
}

#[test]
fn a_shared_file_that_is_no_database_locks_no_character_out() {
    let folder = fresh_folder("shared-not-sqlite");
    fs::write(shared_file(&folder), "not a database, only text").unwrap();
    let mut story = Story::new(empty_pack(&folder), Store::Folder(folder.clone()));

    let entered = entered(&mut story, "Ada");

    assert!(entered.is_ok(), "{entered:?}");
    assert_eq!(story.take_notes().len(), 1);
}

#[test]
fn a_shared_value_with_no_body_comes_back_as_its_default() {
    let folder = fresh_folder("shared-null-body");
    Connection::open(shared_file(&folder))
        .unwrap()
        .execute_batch(
            "CREATE TABLE state (name TEXT PRIMARY KEY, body TEXT);
             INSERT INTO state (name, body) VALUES ('pace', NULL);
             PRAGMA user_version = 1",
        )
        .unwrap();

    let pace = Store::Folder(folder)
        .open_shared()
        .unwrap()
        .load::<Pace>("pace");

    assert!(pace.is_ok_and(|pace| !pace.is_tight(Tick(1_000))));
}

#[test]
fn the_shared_file_keeps_a_wal_journal() {
    let folder = fresh_folder("shared-wal");
    let shared = Store::Folder(folder.clone()).open_shared().unwrap();

    let mode: String = Connection::open(shared_file(&folder))
        .unwrap()
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();

    drop(shared);
    assert_eq!(mode, "wal");
}

#[test]
fn a_failed_save_of_the_shared_values_keeps_the_line() {
    let folder = fresh_folder("shared-save-fails");
    let mut story = story(&folder, "Ada");
    enter(&mut story, 60, "Elwynn Forest");
    let other = Connection::open(shared_file(&folder)).unwrap();
    other.execute_batch("BEGIN IMMEDIATE").unwrap();

    let outputs = story.handle(Input::BatchEnd { id: MessageId(1) });

    other.execute_batch("ROLLBACK").unwrap();
    assert!(outputs.is_ok_and(|outputs| {
        outputs
            .iter()
            .any(|output| matches!(output, Output::ModelCall { .. }))
    }));
    assert_eq!(story.take_notes().len(), 1);
}

#[test]
fn a_safe_id_gives_back_its_name() {
    for name in ["Kobee", "Classic Beta PvP 2", "Zoë", "a_b"] {
        assert_eq!(name_of_safe_id(&safe_id(name)).as_deref(), Some(name));
    }
    assert_eq!(name_of_safe_id("bad_Z"), None);
    assert_eq!(name_of_safe_id("cut_4"), None);
}

#[test]
fn an_id_that_safe_id_never_makes_gives_no_name() {
    for id in ["_+1", "_2e", "_41", "a-b", "a b", "é"] {
        assert_eq!(name_of_safe_id(id), None, "{id}");
    }
}

/// An accepted narrator call with the main part of its shape.
fn narrator_line(position: u64, shape: Option<&str>, outcome: Outcome) -> Line {
    Line {
        calls: vec![NewCall {
            position,
            kind: "narrator",
            pack: "test".to_string(),
            prompt: "prompt".to_string(),
            reads: Vec::new(),
        }],
        ended: vec![CallEnd {
            position,
            answer: Some("answer".to_string()),
            outcome,
            shape: shape.map(str::to_string),
        }],
        ..Line::default()
    }
}

#[test]
fn the_shape_of_a_line_is_stored_with_its_call() {
    let mut database = Database::in_memory().unwrap();
    database
        .save(&narrator_line(0, Some("k.fell"), Outcome::Accepted))
        .unwrap();
    database
        .save(&narrator_line(1, Some("k.dead_u"), Outcome::Refused))
        .unwrap();
    database
        .save(&narrator_line(2, Some("v.stronger"), Outcome::Accepted))
        .unwrap();
    database
        .save(&narrator_line(3, None, Outcome::Accepted))
        .unwrap();

    let newest = database.newest_shapes(8).unwrap();

    assert_eq!(newest, ["v.stronger", "k.fell"]);
    assert_eq!(database.newest_shapes(1).unwrap(), ["v.stronger"]);
}

#[test]
fn a_world_of_version_8_takes_the_shape_column_and_keeps_its_rows() {
    let folder = fresh_folder("version-8");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    {
        let database = Database::open(&path).unwrap();
        drop(database);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "DROP TABLE told_lore; ALTER TABLE calls DROP COLUMN shape; \
                 PRAGMA user_version = 8; \
                 INSERT INTO calls (position, kind, pack, result) VALUES (0, 'narrator', 'p', 'accepted');",
            )
            .unwrap();
    }

    let mut database = Database::open(&path).unwrap();
    database
        .save(&narrator_line(1, Some("k.fell"), Outcome::Accepted))
        .unwrap();

    assert_eq!(database.newest_shapes(8).unwrap(), ["k.fell"]);
    assert!(database.call(0).unwrap().is_some());
    assert_eq!(user_version(&path), 13);
}

fn user_version(path: &Path) -> i64 {
    Connection::open(path)
        .unwrap()
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap()
}

/// A crash between the new column and the new version once left such a file.
#[test]
fn a_world_of_version_8_that_has_the_shape_column_opens_at_the_newest_version() {
    let folder = fresh_folder("version-8-with-shape");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch("DROP TABLE told_lore; PRAGMA user_version = 8")
        .unwrap();

    let opened = Database::open(&path);

    assert!(opened.is_ok(), "{:?}", opened.err());
    assert_eq!(user_version(&path), 13);
}

/// Version 12 only added the table `past` (GAMEPLAY.md 3.3, the prologue).
#[test]
fn a_world_of_version_11_takes_the_past_table_and_keeps_its_rows() {
    let folder = fresh_folder("version-11-past");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "DROP TABLE past; PRAGMA user_version = 11; \
             INSERT INTO calls (position, kind, pack, result) VALUES (0, 'narrator', 'p', 'accepted');",
        )
        .unwrap();

    let database = Database::open(&path).unwrap();

    assert!(database.call(0).unwrap().is_some());
    assert_eq!(user_version(&path), 13);
    let connection = Connection::open(&path).unwrap();
    assert!(connection.prepare("SELECT body FROM past").is_ok());
}

/// Version 13 added the table `narrator_lines`, and the optional `reason` of a rating
/// (GAMEPLAY.md 3.2.2). An old rating reads with no reason.
#[test]
fn a_world_of_version_12_takes_the_narrator_lines_table_and_keeps_its_ratings() {
    let folder = fresh_folder("version-12-lines");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            r#"DROP TABLE narrator_lines; PRAGMA user_version = 12;
             INSERT INTO ratings (position, body) VALUES (0, '{"at":1,"rated":"summary","rating":"down","moment":"summary","text":"An oath.","faults":[]}');"#,
        )
        .unwrap();

    drop(Database::open(&path).unwrap());

    assert_eq!(user_version(&path), 13);
    let connection = Connection::open(&path).unwrap();
    assert!(
        connection
            .prepare("SELECT body FROM narrator_lines")
            .is_ok()
    );
    let body: String = connection
        .query_row("SELECT body FROM ratings", [], |row| row.get(0))
        .unwrap();
    let rating: RatedLine = serde_json::from_str(&body).unwrap();
    assert_eq!(rating.reason, None);
}

/// The column and the version change in one transaction, so a failed upgrade changes
/// neither.
#[test]
fn a_failed_upgrade_leaves_the_world_at_version_8() {
    let folder = fresh_folder("version-8-fails");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    let other = Connection::open(&path).unwrap();
    other
        .execute_batch(
            "DROP TABLE told_lore; ALTER TABLE calls DROP COLUMN shape; \
             PRAGMA user_version = 8; BEGIN IMMEDIATE",
        )
        .unwrap();

    let opened = Database::open(&path);

    other.execute_batch("ROLLBACK").unwrap();
    assert!(opened.is_err());
    assert_eq!(user_version(&path), 8);
    assert!(other.prepare("SELECT shape FROM calls").is_err());
}

fn narrator_call(position: u64, prompt: &str, outcome: Outcome) -> Line {
    let mut line = narrator_line(position, None, outcome);
    line.calls[0].prompt = prompt.to_string();
    line
}

const TOLD_PROMPT: &str = "The moment:\n<<<\nThe player entered the dungeon.\n>>>\n\n\
                           The lore:\n<<<\nVanCleef built the Brotherhood.\n>>>\n\nAnswer.";

const TOLD_LORE: &str = "<<<\nVanCleef built the Brotherhood.\n>>>";

#[test]
fn the_lore_of_an_accepted_narrator_call_stays_told_after_its_prompt_ages_out() {
    let mut database = Database::in_memory().unwrap();
    database
        .save(&narrator_call(0, TOLD_PROMPT, Outcome::Accepted))
        .unwrap();
    for position in 1..=PROMPTS_KEPT {
        database
            .save(&narrator_call(position, "prompt", Outcome::Failed))
            .unwrap();
    }

    let told = database.told_lore().unwrap();

    assert_eq!(database.call(0).unwrap().unwrap().prompt, None);
    assert_eq!(told, [TOLD_LORE]);
}

#[test]
fn the_lore_of_a_refused_narrator_call_is_not_told() {
    let mut database = Database::in_memory().unwrap();
    database
        .save(&narrator_call(0, TOLD_PROMPT, Outcome::Refused))
        .unwrap();

    assert!(database.told_lore().unwrap().is_empty());
}

#[test]
fn a_world_of_version_9_keeps_the_lore_of_its_kept_prompts_as_told() {
    let folder = fresh_folder("version-9");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    {
        let mut database = Database::open(&path).unwrap();
        database
            .save(&narrator_call(0, TOLD_PROMPT, Outcome::Accepted))
            .unwrap();
        database
            .save(&narrator_call(1, "no lore", Outcome::Accepted))
            .unwrap();
        drop(database);
        Connection::open(&path)
            .unwrap()
            .execute_batch("DROP TABLE told_lore; PRAGMA user_version = 9")
            .unwrap();
    }

    let database = Database::open(&path).unwrap();

    assert_eq!(database.told_lore().unwrap(), [TOLD_LORE]);
    assert_eq!(user_version(&path), 13);
}

/// The table and the version change in one transaction, so a failed upgrade changes
/// neither.
#[test]
fn a_failed_upgrade_leaves_the_world_at_version_9() {
    let folder = fresh_folder("version-9-fails");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    let other = Connection::open(&path).unwrap();
    other
        .execute_batch("DROP TABLE told_lore; PRAGMA user_version = 9; BEGIN IMMEDIATE")
        .unwrap();

    let opened = Database::open(&path);

    other.execute_batch("ROLLBACK").unwrap();
    assert!(opened.is_err());
    assert_eq!(user_version(&path), 9);
    assert!(other.prepare("SELECT lore FROM told_lore").is_err());
}

#[test]
fn told_lore_of_a_call_that_is_gone_goes_with_the_broken_links() {
    let folder = fresh_folder("told-lore-orphan");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    let other = Connection::open(&path).unwrap();
    other
        .execute_batch(
            "PRAGMA foreign_keys = OFF; INSERT INTO told_lore (call, lore) VALUES (99, 'lore')",
        )
        .unwrap();

    let database = Database::open(&path).unwrap();
    database.drop_broken_links().unwrap();

    let left: i64 = other
        .query_row("SELECT count(*) FROM told_lore", [], |row| row.get(0))
        .unwrap();
    assert_eq!(left, 0);
}

#[test]
fn a_world_of_version_10_takes_the_ratings_table_and_keeps_its_rows() {
    let folder = fresh_folder("version-11");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("world.sqlite");
    drop(Database::open(&path).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "DROP TABLE ratings; PRAGMA user_version = 10; \
             INSERT INTO calls (position, kind, pack, result) VALUES (0, 'narrator', 'p', 'accepted');",
        )
        .unwrap();

    let database = Database::open(&path).unwrap();

    assert!(database.call(0).unwrap().is_some());
    assert_eq!(user_version(&path), 13);
    let connection = Connection::open(&path).unwrap();
    assert!(connection.prepare("SELECT body FROM ratings").is_ok());
}
