#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::Page;
use timeways_story::pack::Pack;
use timeways_story::store::{CharacterKey, Store, StoreError, Table, safe_id};
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
    let Some(Output::Journal {
        page: Page { journal, .. },
        ..
    }) = output
    else {
        panic!("expected a journal, got {output:?}");
    };
    journal.places.into_iter().map(|place| place.name).collect()
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

fn insert(folder: &Path, table: &str, body: &str) {
    let connection = Connection::open(world_file(folder, "Ada")).unwrap();
    let insert = format!("INSERT INTO {table} (body) VALUES (?1)");
    connection.execute(&insert, params![body]).unwrap();
}

/// The file of a build before SQLite, next to where the database goes.
fn old_file(folder: &Path, suffix: &str) -> PathBuf {
    let realm = folder.join("worlds").join("r_Stormrage");
    fs::create_dir_all(&realm).unwrap();
    realm.join(format!("c_Ada{suffix}"))
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
    connection.execute(swap, params![events[2], 2]).unwrap();
    connection.execute(swap, params![events[1], 3]).unwrap();
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
    let blob = "INSERT INTO events (body) VALUES (?1)";
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
            })
            .unwrap();
    }
    enter(&mut first, 3, "Westfall");
    let rows_before = bodies(&folder, "events").len();

    // Hogger moves to Westfall, and the count stays at 1000.
    let kill = first.handle(Input::NpcDefeated {
        at: Tick(4),
        name: "Hogger".to_string(),
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
    });

    assert!(matches!(result, Err(StoryError::NoCharacter)));
    drop(story);
    assert!(places(&mut self::story(&folder, "Ada")).is_empty());
}

/// A folder that takes no new file takes no SQLite journal, so the save fails.
#[cfg(unix)]
#[test]
fn a_failed_save_loses_its_line_and_the_next_line_saves() {
    use std::os::unix::fs::PermissionsExt;
    let folder = fresh_folder("failed-save");
    let mut story = story(&folder, "Ada");
    enter(&mut story, 1, "Elwynn Forest");
    let realm = world_file(&folder, "Ada").parent().unwrap().to_path_buf();
    fs::set_permissions(&realm, fs::Permissions::from_mode(0o555)).unwrap();

    let refused = story.handle(Input::ZoneEntered {
        at: Tick(2),
        zone: "Westfall".to_string(),
        subzone: None,
        spot: None,
    });
    fs::set_permissions(&realm, fs::Permissions::from_mode(0o755)).unwrap();
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
    // 50 minutes of play, then a new zone: the milestone of a second chapter.
    let zones = [
        (3600, "Westfall", None),
        (3600 + 25 * 60, "Westfall", Some("Moonbrook")),
        (3600 + 50 * 60, "Westfall", Some("Sentinel Hill")),
        (5 * 3600, "Duskwood", None),
    ];
    for (batch, (at, zone, subzone)) in zones.into_iter().enumerate() {
        let entered = Input::ZoneEntered {
            at: Tick(at),
            zone: zone.to_string(),
            subzone: subzone.map(str::to_string),
            spot: None,
        };
        first.handle(entered).unwrap();
        fail_each_call(&mut first, 90 + batch as u64);
    }
    let outputs = first.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    let text = r#"{"saga": "Our hero rode west."}"#.to_string();
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
        Some("Our hero rode west.")
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
fn an_old_world_moves_into_the_database_at_its_first_open() {
    let folder = fresh_folder("import");
    let mut memory = story(&folder, "Ada");
    enter(&mut memory, 1, "Elwynn Forest");
    enter(&mut memory, 2, "Westfall");
    drop(memory);
    let events = bodies(&folder, "events");
    fs::remove_file(world_file(&folder, "Ada")).unwrap();
    fs::write(old_file(&folder, ".jsonl"), events.join("\n") + "\n").unwrap();

    let mut story = story(&folder, "Ada");

    assert_eq!(places(&mut story), ["Elwynn Forest", "Westfall"]);
    assert!(old_file(&folder, ".jsonl").is_file());
}

#[test]
fn an_old_line_cut_inside_a_character_is_left_out_of_the_import() {
    let folder = fresh_folder("import-torn");
    let mut memory = story(&folder, "Ada");
    enter(&mut memory, 10, "Elwynn Forest");
    drop(memory);
    let events = bodies(&folder, "events");
    fs::remove_file(world_file(&folder, "Ada")).unwrap();
    let mut bytes = (events.join("\n") + "\n").into_bytes();
    bytes.extend_from_slice(b"{\"id\":99,\"text\":\"Caf\xC3");
    fs::write(old_file(&folder, ".jsonl"), bytes).unwrap();

    let mut story = story(&folder, "Ada");

    assert_eq!(places(&mut story), ["Elwynn Forest"]);
    assert_eq!(bodies(&folder, "events"), events);
}

#[test]
fn every_old_file_moves_into_its_table() {
    let folder = fresh_folder("import-tables");
    let saga = r#"{"began":5,"text":"Our hero came."}"#;
    fs::write(old_file(&folder, ".chronicle.jsonl"), format!("{saga}\n")).unwrap();

    let opened = Store::Folder(folder.clone())
        .open(&CharacterKey::new("Stormrage", "Ada").unwrap())
        .unwrap();

    assert_eq!(opened.prose.get(Tick(5)).unwrap().text, "Our hero came.");
    assert_eq!(bodies(&folder, "chapters"), [saga]);
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

    assert!(opened.database.is_some());
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
        text: "Our hero came.".to_string(),
        footnotes: Vec::new(),
    };
    first.prose.add(Tick(5), written).unwrap();
    let rows = vec![(Table::Chapters, first.prose.take_unsaved())];
    first.database.unwrap().save(&rows).unwrap();

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
