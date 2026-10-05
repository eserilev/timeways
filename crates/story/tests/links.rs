#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The proof, the source, and the uses of rows (docs/plans/links.md).

use hourglass::Tick;
use rusqlite::{Connection, params};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::store::graph::weakest;
use timeways_story::store::{
    CallEnd, CharacterKey, Database, Line, NewCall, Node, Outcome, PROMPTS_KEPT, Root, Store, Table,
};
use timeways_story::story::why::{TrustCause, TrustWhy};
use timeways_story::story::{Output, Story, StoryError};

const HOUR: u64 = 3600;

fn fresh_folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("links-{name}"));
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

fn key() -> CharacterKey {
    CharacterKey::new("Stormrage", "Ada").unwrap()
}

fn world_file(folder: &Path) -> PathBuf {
    folder
        .join("worlds")
        .join("r_Stormrage")
        .join("c_Ada.sqlite")
}

/// The database of the world, as a second reader sees it.
fn database(folder: &Path) -> Database {
    Store::Folder(folder.to_path_buf())
        .open(&key())
        .unwrap()
        .database
}

fn sql(folder: &Path) -> Connection {
    Connection::open(world_file(folder)).unwrap()
}

fn count(folder: &Path, select: &str) -> i64 {
    sql(folder).query_row(select, [], |row| row.get(0)).unwrap()
}

fn positions(folder: &Path, select: &str) -> Vec<u64> {
    let connection = sql(folder);
    let mut statement = connection.prepare(select).unwrap();
    statement
        .query_map([], |row| row.get::<_, i64>(0))
        .unwrap()
        .map(|row| u64::try_from(row.unwrap()).unwrap())
        .collect()
}

fn enter(story: &mut Story, at: u64, zone: &str, subzone: Option<&str>) {
    story
        .handle(Input::ZoneEntered {
            at: Tick(at),
            zone: zone.to_string(),
            subzone: subzone.map(str::to_string),
            spot: None,
            hour: None,
        })
        .unwrap();
}

fn meet(story: &mut Story, at: u64, name: &str) {
    story
        .handle(Input::NpcMet {
            at: Tick(at),
            name: name.to_string(),
            spot: None,
        })
        .unwrap();
}

fn talk(story: &mut Story, npc: &str) -> CallId {
    let outputs = story
        .handle(Input::TalkAsked {
            id: MessageId(8),
            at: Tick(50),
            npc: npc.to_string(),
            text: "any news?".to_string(),
        })
        .unwrap();
    call_of(&outputs)
}

fn call_of(outputs: &[Output]) -> CallId {
    outputs
        .iter()
        .find_map(|output| match output {
            Output::ModelCall { call, .. } => Some(*call),
            _ => None,
        })
        .expect("a model call")
}

fn answer(story: &mut Story, call: CallId, text: &str) -> Vec<Output> {
    story
        .handle(Input::ModelAnswered {
            call,
            text: text.to_string(),
        })
        .unwrap()
}

/// The newest event that a model call made.
fn newest_event_of_a_call(folder: &Path) -> u64 {
    positions(
        folder,
        "SELECT position FROM events WHERE call IS NOT NULL ORDER BY position DESC LIMIT 1",
    )[0]
}

#[test]
fn every_kept_line_leaves_an_input_row() {
    let folder = fresh_folder("inputs");
    let mut story = story(&folder);

    enter(&mut story, 1, "Elwynn Forest", None);
    meet(&mut story, 2, "Marshal Dughan");
    drop(story);

    let kinds = sql(&folder)
        .prepare("SELECT kind FROM inputs ORDER BY position")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(kinds, ["character_entered", "zone_entered", "npc_met"]);
}

#[test]
fn a_journal_request_leaves_no_input_row() {
    let folder = fresh_folder("journal-no-input");
    let mut story = story(&folder);

    story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();
    drop(story);

    assert_eq!(count(&folder, "SELECT count(*) FROM inputs"), 1);
}

#[test]
fn a_refused_line_keeps_its_input_row() {
    let folder = fresh_folder("refused-input");
    let mut story = story(&folder);

    let refused = story.handle(Input::TalkAsked {
        id: MessageId(8),
        at: Tick(5),
        npc: String::new(),
        text: "hi".to_string(),
    });
    drop(story);

    assert!(refused.is_err());
    assert_eq!(
        count(
            &folder,
            "SELECT count(*) FROM inputs WHERE kind = 'talk_asked'"
        ),
        1
    );
}

#[test]
fn each_event_rests_on_the_line_that_made_it() {
    let folder = fresh_folder("evidence");
    let mut story = story(&folder);

    enter(&mut story, 1, "Elwynn Forest", None);
    drop(story);

    let zone_line = positions(
        &folder,
        "SELECT position FROM inputs WHERE kind = 'zone_entered'",
    )[0];
    let from_zone = positions(
        &folder,
        &format!("SELECT position FROM events WHERE input = {zone_line}"),
    );
    assert!(!from_zone.is_empty());
    assert_eq!(
        count(
            &folder,
            "SELECT count(*) FROM events WHERE input IS NULL AND call IS NULL"
        ),
        0
    );
}

#[test]
fn an_event_keeps_its_position_after_a_cut() {
    let folder = fresh_folder("positions");
    let mut first = story(&folder);
    enter(&mut first, 1, "Elwynn Forest", None);
    drop(first);
    let kept = count(&folder, "SELECT count(*) FROM events");
    sql(&folder)
        .execute(
            "INSERT INTO events (position, body) VALUES (?1, 'broken')",
            params![kept],
        )
        .unwrap();

    let mut second = story(&folder);
    enter(&mut second, 2, "Westfall", None);
    drop(second);

    let events = positions(&folder, "SELECT position FROM events ORDER BY position");
    let ids = sql(&folder)
        .prepare("SELECT body FROM events ORDER BY position")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|body| {
            let event: serde_json::Value = serde_json::from_str(&body.unwrap()).unwrap();
            event["id"].as_u64().unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(events, ids);
    assert_eq!(events, (0..events.len() as u64).collect::<Vec<_>>());
}

#[test]
fn a_failed_save_keeps_no_input_call_or_read() {
    let folder = fresh_folder("failed-save");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", None);
    let inputs_before = count(&folder, "SELECT count(*) FROM inputs");
    let other = sql(&folder);
    other.execute_batch("BEGIN IMMEDIATE").unwrap();

    let refused = story.handle(Input::TalkAsked {
        id: MessageId(8),
        at: Tick(50),
        npc: "Innkeeper Farley".to_string(),
        text: "any news?".to_string(),
    });
    other.execute_batch("ROLLBACK").unwrap();
    drop(story);

    assert!(matches!(refused, Err(StoryError::Store(_))));
    assert_eq!(count(&folder, "SELECT count(*) FROM inputs"), inputs_before);
    assert_eq!(count(&folder, "SELECT count(*) FROM calls"), 0);
    assert_eq!(count(&folder, "SELECT count(*) FROM reads"), 0);
}

#[test]
fn a_call_keeps_its_prompt_its_answer_and_how_it_ended() {
    let folder = fresh_folder("call-row");
    let mut story = story(&folder);
    let call = talk(&mut story, "Innkeeper Farley");

    answer(
        &mut story,
        call,
        r#"{"say": "Nothing but rain.", "trust": 3}"#,
    );
    drop(story);

    let record = database(&folder).call(0).unwrap().unwrap();
    assert_eq!(record.kind, "talk");
    assert!(record.prompt.unwrap().contains("Innkeeper Farley"));
    assert_eq!(
        record.answer.as_deref(),
        Some(r#"{"say": "Nothing but rain.", "trust": 3}"#)
    );
    assert_eq!(record.result, "accepted");
}

#[test]
fn a_failed_call_ends_as_failed() {
    let folder = fresh_folder("call-failed");
    let mut story = story(&folder);
    let call = talk(&mut story, "Innkeeper Farley");

    story.handle(Input::ModelFailed { call }).unwrap();
    drop(story);

    let record = database(&folder).call(0).unwrap().unwrap();
    assert_eq!(record.answer, None);
    assert_eq!(record.result, "failed");
}

#[test]
fn a_trust_change_from_talk_rests_on_the_words_of_the_player() {
    let folder = fresh_folder("trust-proof");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 2, "Innkeeper Farley");
    let call = talk(&mut story, "Innkeeper Farley");

    answer(
        &mut story,
        call,
        r#"{"say": "Nothing but rain.", "trust": 3}"#,
    );
    drop(story);

    let trust = Node::Row(Table::Events, newest_event_of_a_call(&folder));
    let proof = database(&folder).proof_of(trust).unwrap();
    assert_eq!(proof, BTreeSet::from([Root::Player, Root::Game]));
    assert_eq!(weakest(&proof), Root::Player);
}

#[test]
fn a_talk_reads_the_npc_and_its_source_is_the_call() {
    let folder = fresh_folder("talk-reads");
    let mut story = story(&folder);
    meet(&mut story, 2, "Innkeeper Farley");
    let met = positions(&folder, "SELECT position FROM events WHERE input = 1");
    let call = talk(&mut story, "Innkeeper Farley");

    answer(
        &mut story,
        call,
        r#"{"say": "Nothing but rain.", "trust": 3}"#,
    );
    drop(story);

    let database = database(&folder);
    let trust = Node::Row(Table::Events, newest_event_of_a_call(&folder));
    let source = database.source_of(trust).unwrap().unwrap();
    assert_eq!(source.call, 0);
    for event in met {
        assert!(
            database
                .uses_of(Node::Row(Table::Events, event))
                .unwrap()
                .contains(&0),
            "event {event} is not read"
        );
    }
}

#[test]
fn a_refused_call_uses_nothing() {
    let folder = fresh_folder("refused-call");
    let mut story = story(&folder);
    meet(&mut story, 2, "Innkeeper Farley");
    let call = talk(&mut story, "Innkeeper Farley");

    answer(&mut story, call, "not json at all");
    drop(story);

    let database = database(&folder);
    assert_eq!(database.call(0).unwrap().unwrap().result, "refused");
    let read = database.reads_of(0).unwrap();
    assert!(!read.is_empty());
    for node in read {
        assert!(database.uses_of(node).unwrap().is_empty(), "{node:?}");
    }
}

#[test]
fn a_hero_entry_removed_during_a_call_still_counts_as_read() {
    let folder = fresh_folder("withdrawn");
    let mut story = story(&folder);
    meet(&mut story, 2, "Innkeeper Farley");
    story
        .handle(Input::HeroAdded {
            at: Tick(3),
            text: "Farley owes me gold.".to_string(),
            npc: Some("Innkeeper Farley".to_string()),
        })
        .unwrap();
    let call = talk(&mut story, "Innkeeper Farley");
    story
        .handle(Input::HeroRemoved {
            at: Tick(60),
            number: 1,
        })
        .unwrap();

    answer(
        &mut story,
        call,
        r#"{"say": "Ah, about that gold.", "trust": 0}"#,
    );
    drop(story);

    let uses = database(&folder)
        .uses_of(Node::Row(Table::Hero, 0))
        .unwrap();
    assert_eq!(uses, [0]);
}

#[test]
fn an_answer_for_another_character_writes_nothing_here() {
    let folder = fresh_folder("other-character");
    let mut story = story(&folder);
    let call = talk(&mut story, "Innkeeper Farley");
    story
        .handle(Input::CharacterEntered {
            realm: "Stormrage".to_string(),
            name: "Bren".to_string(),
        })
        .unwrap();

    answer(
        &mut story,
        call,
        r#"{"say": "Nothing but rain.", "trust": 3}"#,
    );
    drop(story);

    let bren = folder
        .join("worlds")
        .join("r_Stormrage")
        .join("c_Bren.sqlite");
    let bren = Connection::open(bren).unwrap();
    let calls: i64 = bren
        .query_row("SELECT count(*) FROM calls", [], |row| row.get(0))
        .unwrap();
    let made: i64 = bren
        .query_row(
            "SELECT count(*) FROM events WHERE call IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((calls, made), (0, 0));
    assert_eq!(database(&folder).call(0).unwrap().unwrap().result, "open");
}

#[test]
fn an_answer_after_a_relog_rests_on_its_call() {
    let folder = fresh_folder("answer-after-relog");
    let mut story = story(&folder);
    meet(&mut story, 2, "Innkeeper Farley");
    let call = talk(&mut story, "Innkeeper Farley");
    for name in ["Bren", "Ada"] {
        story
            .handle(Input::CharacterEntered {
                realm: "Stormrage".to_string(),
                name: name.to_string(),
            })
            .unwrap();
    }

    answer(
        &mut story,
        call,
        r#"{"say": "Nothing but rain.", "trust": 3}"#,
    );
    drop(story);

    let made_by_calls = positions(
        &folder,
        "SELECT position FROM events WHERE call IS NOT NULL",
    );
    assert_eq!(made_by_calls.len(), 1);
    assert_eq!(
        database(&folder).call(0).unwrap().unwrap().result,
        "accepted"
    );
}

#[test]
fn a_game_event_has_game_proof_alone() {
    let folder = fresh_folder("game-proof");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", None);
    story
        .handle(Input::NpcDefeated {
            at: Tick(2),
            name: "Hogger".to_string(),
        })
        .unwrap();
    drop(story);

    let database = database(&folder);
    let events = positions(&folder, "SELECT position FROM events ORDER BY position");
    for event in events {
        let proof = database.proof_of(Node::Row(Table::Events, event)).unwrap();
        assert_eq!(proof, BTreeSet::from([Root::Game]), "event {event}");
    }
}

#[test]
fn a_row_whose_line_is_gone_is_lost() {
    let folder = fresh_folder("lost");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", None);
    drop(story);
    let connection = sql(&folder);
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    connection
        .execute("DELETE FROM inputs WHERE position = 1", [])
        .unwrap();
    drop(connection);

    let database = database(&folder);
    let orphan = positions(
        &folder,
        "SELECT position FROM events WHERE input IS NULL AND call IS NULL",
    );

    assert!(!orphan.is_empty());
    let proof = database
        .proof_of(Node::Row(Table::Events, orphan[0]))
        .unwrap();
    assert_eq!(weakest(&proof), Root::Lost);
}

#[test]
fn a_narrator_line_reads_the_events_of_its_batch() {
    let folder = fresh_folder("narrator-reads");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", None);
    let batch_events = positions(&folder, "SELECT position FROM events WHERE input = 1");

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let call = call_of(&outputs);
    answer(
        &mut story,
        call,
        "The forest knew your name before you did.",
    );
    drop(story);

    let database = database(&folder);
    let read = database.reads_of(0).unwrap();
    assert_eq!(database.call(0).unwrap().unwrap().kind, "narrator");
    for event in batch_events {
        assert!(read.contains(&Node::Row(Table::Events, event)), "{event}");
    }
}

/// Two chapters: the first one is finished. The narrator lines of the zones get no
/// answer.
fn two_sessions(story: &mut Story) {
    meet(story, HOUR, "Gryan Stoutmantle");
    enter(story, HOUR, "Westfall", None);
    enter(story, HOUR + 25 * 60, "Westfall", Some("Camp One"));
    enter(story, HOUR + 50 * 60, "Westfall", Some("Camp Two"));
    enter(story, 5 * HOUR, "Duskwood", None);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(91) }).unwrap();
    for output in outputs {
        if let Output::ModelCall { call, .. } = output {
            story.handle(Input::ModelFailed { call }).unwrap();
        }
    }
    meet(story, 5 * HOUR, "Salma Saldean");
}

#[test]
fn a_saga_reads_its_chapter_and_rests_on_its_call() {
    let folder = fresh_folder("saga-reads");
    let mut story = story(&folder);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let call = call_of(&outputs[1..]);

    answer(
        &mut story,
        call,
        r#"{"saga": "$N rode into the golden fields of Westfall."}"#,
    );
    drop(story);

    let database = database(&folder);
    let saga_call = positions(&folder, "SELECT position FROM calls WHERE kind = 'saga'")[0];
    let read = database.reads_of(saga_call).unwrap();
    let gryan = positions(
        &folder,
        "SELECT position FROM events WHERE body LIKE '%Gryan Stoutmantle%'",
    );
    let salma = positions(
        &folder,
        "SELECT position FROM events WHERE body LIKE '%Salma Saldean%'",
    );
    assert!(read.contains(&Node::Row(Table::Events, gryan[0])));
    assert!(!read.contains(&Node::Row(Table::Events, salma[0])));
    let source = database.source_of(Node::Row(Table::Chapters, 0)).unwrap();
    assert!(source.is_some_and(|source| source.call >= saga_call));
}

#[test]
fn a_talk_reads_the_rows_behind_its_memories() {
    let folder = fresh_folder("talk-reads-memories");
    let mut story = story(&folder);
    let first = talk(&mut story, "Innkeeper Farley");
    answer(
        &mut story,
        first,
        r#"{"say": "The gnolls grow bold.", "trust": 0}"#,
    );

    let second = talk(&mut story, "Innkeeper Farley");
    answer(&mut story, second, r#"{"say": "Hm.", "trust": 0}"#);
    drop(story);

    let reads = database(&folder).reads_of(1).unwrap();
    assert!(reads.contains(&Node::Row(Table::Learned, 0)), "{reads:?}");
}

fn set_goal(story: &mut Story, text: &str) {
    story
        .handle(Input::HeroSet {
            at: Tick(40),
            field: "goal".to_string(),
            text: text.to_string(),
        })
        .unwrap();
}

#[test]
fn a_hook_reads_the_hero_row_that_wrote_its_text() {
    let folder = fresh_folder("hook-reads");
    let mut story = story(&folder);
    set_goal(&mut story, "Find my father.");
    set_goal(&mut story, "Avenge my father.");
    for _ in 0..3 {
        let call = talk(&mut story, "Innkeeper Farley");
        answer(&mut story, call, r#"{"say": "Hm.", "trust": 0}"#);
    }
    drop(story);

    let database = database(&folder);
    let hooked = database.reads_of(2).unwrap();
    assert!(hooked.contains(&Node::Row(Table::Hero, 1)), "{hooked:?}");
    assert!(!hooked.contains(&Node::Row(Table::Hero, 0)), "{hooked:?}");
    let before = database.reads_of(1).unwrap();
    assert!(!before.contains(&Node::Row(Table::Hero, 1)), "{before:?}");
}

#[test]
fn the_count_of_calls_survives_a_restart() {
    let folder = fresh_folder("hook-restart");
    let mut before = story(&folder);
    set_goal(&mut before, "Find my father.");
    for _ in 0..2 {
        let call = talk(&mut before, "Innkeeper Farley");
        answer(&mut before, call, r#"{"say": "Hm.", "trust": 0}"#);
    }
    drop(before);
    let mut after = story(&folder);

    let outputs = after
        .handle(Input::TalkAsked {
            id: MessageId(8),
            at: Tick(60),
            npc: "Innkeeper Farley".to_string(),
            text: "any news?".to_string(),
        })
        .unwrap();

    let Some(Output::ModelCall { prompt, .. }) = outputs.first() else {
        panic!("expected a model call, got {outputs:?}");
    };
    assert!(prompt.contains("<<<\nFind my father.\n>>>"), "{prompt}");
}

#[test]
fn a_quest_offer_reads_its_giver_and_rests_on_its_call() {
    let folder = fresh_folder("quest-reads");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 2, "Marshal Dughan");
    story
        .handle(Input::QuestAsked {
            at: Tick(3),
            npc: "Marshal Dughan".to_string(),
        })
        .unwrap();
    let outputs = story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();
    let call = call_of(&outputs);
    let text = r#"{"title": "A Walk", "genre": "errand", "text": "I need you in Goldshire.", "steps": [{"goal": "visit", "place": "Goldshire"}]}"#;

    answer(&mut story, call, text);
    drop(story);

    let database = database(&folder);
    assert_eq!(database.call(0).unwrap().unwrap().result, "accepted");
    let dughan = positions(
        &folder,
        "SELECT position FROM events WHERE body LIKE '%Marshal Dughan%' ORDER BY position LIMIT 1",
    )[0];
    assert!(
        database
            .reads_of(0)
            .unwrap()
            .contains(&Node::Row(Table::Events, dughan))
    );
    let source = database.source_of(Node::Row(Table::Quests, 0)).unwrap();
    assert_eq!(source.map(|source| source.call), Some(0));
}

#[test]
fn a_quest_from_a_talk_reads_the_talk_and_its_offer_rests_on_the_quest_call() {
    let folder = fresh_folder("talk-quest-call-reads");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 2, "Marshal Dughan");
    let talk_call = talk(&mut story, "Marshal Dughan");
    let work = r#"{"say": "I could use a hand.", "trust": 0, "work": true}"#;
    let quest_call = call_of(&answer(&mut story, talk_call, work));
    let text = r#"{"title": "A Walk", "genre": "errand", "text": "I need you in Goldshire.", "steps": [{"goal": "visit", "place": "Goldshire"}]}"#;

    answer(&mut story, quest_call, text);
    drop(story);

    let database = database(&folder);
    assert!(database.reads_of(1).unwrap().contains(&Node::Call(0)));
    let source = database.source_of(Node::Row(Table::Quests, 0)).unwrap();
    assert_eq!(source.map(|source| source.call), Some(1));
}

#[test]
fn a_talk_to_the_npc_of_a_talk_step_reads_the_rows_of_its_quest() {
    let folder = fresh_folder("talk-quest-reads");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 2, "Marshal Dughan");
    meet(&mut story, 3, "Farmer Saldean");
    story
        .handle(Input::QuestAsked {
            at: Tick(4),
            npc: "Marshal Dughan".to_string(),
        })
        .unwrap();
    let outputs = story.handle(Input::BatchEnd { id: MessageId(5) }).unwrap();
    let text = r#"{"title": "A Word", "genre": "errand", "text": "I need word from Saldean.", "steps": [{"goal": "talk", "npc": "Farmer Saldean"}]}"#;
    answer(&mut story, call_of(&outputs), text);
    story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();

    talk(&mut story, "Farmer Saldean");
    drop(story);

    let read = database(&folder).reads_of(1).unwrap();
    for row in 0..3 {
        assert!(
            read.contains(&Node::Row(Table::Quests, row)),
            "{row}: {read:?}"
        );
    }
}

#[test]
fn the_offer_prompt_reads_the_rows_of_the_recent_quests() {
    let folder = fresh_folder("quest-recent-reads");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 2, "Marshal Dughan");
    let ask = |story: &mut Story, at: u64| {
        story
            .handle(Input::QuestAsked {
                at: Tick(at),
                npc: "Marshal Dughan".to_string(),
            })
            .unwrap();
        call_of(&story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap())
    };
    let first = ask(&mut story, 3);
    let text = r#"{"title": "A Walk", "genre": "errand", "text": "I need you in Goldshire.", "steps": [{"goal": "visit", "place": "Goldshire"}]}"#;
    answer(&mut story, first, text);

    ask(&mut story, 5);
    drop(story);

    let read = database(&folder).reads_of(1).unwrap();
    assert!(read.contains(&Node::Row(Table::Quests, 0)), "{read:?}");
}

#[test]
fn the_retry_call_rests_on_the_first_call() {
    let folder = fresh_folder("quest-retry");
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 2, "Marshal Dughan");
    story
        .handle(Input::QuestAsked {
            at: Tick(3),
            npc: "Marshal Dughan".to_string(),
        })
        .unwrap();
    let outputs = story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();
    let first = call_of(&outputs);

    let retry = call_of(&answer(&mut story, first, "no quest here"));
    let text = r#"{"title": "A Walk", "genre": "errand", "text": "I need you in Goldshire.", "steps": [{"goal": "visit", "place": "Goldshire"}]}"#;
    answer(&mut story, retry, text);
    drop(story);

    let database = database(&folder);
    let (first, retry) = (
        database.call(0).unwrap().unwrap(),
        database.call(1).unwrap().unwrap(),
    );
    assert_eq!(
        (first.kind.as_str(), first.result.as_str()),
        ("quest", "refused")
    );
    assert_eq!(
        (retry.kind.as_str(), retry.result.as_str()),
        ("quest_retry", "accepted")
    );
    let read = database.reads_of(1).unwrap();
    assert!(read.contains(&Node::Call(0)), "{read:?}");
    assert!(
        database
            .reads_of(0)
            .unwrap()
            .iter()
            .all(|node| read.contains(node))
    );
    let source = database.source_of(Node::Call(1)).unwrap();
    assert_eq!(source.map(|source| source.call), Some(0));
}

fn call_line(position: u64) -> Line {
    Line {
        calls: vec![NewCall {
            position,
            kind: "lore",
            pack: "test".to_string(),
            prompt: format!("prompt {position}"),
            reads: Vec::new(),
        }],
        ended: vec![CallEnd {
            position,
            answer: None,
            outcome: Outcome::Failed,
        }],
        ..Line::default()
    }
}

#[test]
fn only_the_newest_prompts_are_kept() {
    let folder = fresh_folder("prompts-kept");
    let mut database = Store::Folder(folder.clone()).open(&key()).unwrap().database;

    for position in 0..=PROMPTS_KEPT {
        database.save(&call_line(position)).unwrap();
    }

    assert_eq!(database.call(0).unwrap().unwrap().prompt, None);
    assert_eq!(
        database.call(1).unwrap().unwrap().prompt.as_deref(),
        Some("prompt 1")
    );
    assert_eq!(
        count(
            &folder,
            "SELECT count(*) FROM calls WHERE prompt IS NOT NULL"
        ),
        i64::try_from(PROMPTS_KEPT).unwrap()
    );
}

#[test]
fn a_line_of_many_calls_clears_each_prompt_that_aged_out() {
    let folder = fresh_folder("prompts-kept-many");
    let mut database = Store::Folder(folder.clone()).open(&key()).unwrap().database;
    for position in 0..PROMPTS_KEPT {
        database.save(&call_line(position)).unwrap();
    }
    let newest = PROMPTS_KEPT..PROMPTS_KEPT + 3;
    let mut line = call_line(newest.start);
    line.calls = newest
        .flat_map(|position| call_line(position).calls)
        .collect();

    database.save(&line).unwrap();

    let cleared = positions(
        &folder,
        "SELECT position FROM calls WHERE prompt IS NULL ORDER BY position",
    );
    assert_eq!(cleared, [0, 1, 2]);
}

#[test]
fn the_count_of_calls_of_a_kind_reads_an_index() {
    let folder = fresh_folder("calls-kind-index");
    drop(database(&folder));

    let plan: String = sql(&folder)
        .query_row(
            "EXPLAIN QUERY PLAN SELECT count(*) FROM calls WHERE kind IN ('talk', 'quest')",
            [],
            |row| row.get(3),
        )
        .unwrap();

    assert!(plan.contains("calls_of_a_kind"), "{plan}");
}

#[test]
fn a_file_of_another_version_is_refused_and_left_as_it_is() {
    let folder = fresh_folder("other-version");
    let file = world_file(&folder);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let connection = Connection::open(&file).unwrap();
    connection
        .execute_batch("CREATE TABLE events (position INTEGER PRIMARY KEY, body TEXT); PRAGMA user_version = 1;")
        .unwrap();
    drop(connection);

    let opened = Store::Folder(folder.clone()).open(&key());

    assert!(matches!(
        opened,
        Err(timeways_story::store::StoreError::OtherVersion { version: 1, .. })
    ));
    let version: i64 = sql(&folder)
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 1);
}

/// The story program writes links back in time only. A fuzzer found a cycle that another
/// program can write.
#[test]
fn a_call_that_rests_on_itself_is_lost() {
    let folder = fresh_folder("cycle");
    let mut story = story(&folder);
    let call = talk(&mut story, "Innkeeper Farley");
    answer(&mut story, call, "not json at all");
    drop(story);
    let connection = sql(&folder);
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    connection
        .execute(
            "UPDATE calls SET input = NULL, call = 0 WHERE position = 0",
            [],
        )
        .unwrap();
    connection.execute("DELETE FROM reads", []).unwrap();
    drop(connection);

    let proof = database(&folder).proof_of(Node::Call(0)).unwrap();

    assert_eq!(proof, BTreeSet::from([Root::Lost]));
}

fn trust_why_of(story: &mut Story, npc: &str) -> Option<TrustWhy> {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();
    let Some(Output::Journal { page, .. }) = outputs.into_iter().next() else {
        panic!("expected a journal");
    };
    page.journal
        .people
        .into_iter()
        .find(|person| person.name == npc)?
        .trust_why
}

#[test]
fn trust_why_names_the_talk_that_changed_it() {
    let folder = fresh_folder("why-talk");
    let mut story = story(&folder);
    meet(&mut story, 2, "Innkeeper Farley");
    let call = talk(&mut story, "Innkeeper Farley");

    answer(
        &mut story,
        call,
        r#"{"say": "Nothing but rain.", "trust": -3}"#,
    );

    let why = TrustWhy {
        by: TrustCause::Talk,
        up: false,
        at: Tick(50),
    };
    assert_eq!(trust_why_of(&mut story, "Innkeeper Farley"), Some(why));
}

#[test]
fn trust_why_names_a_slap() {
    let folder = fresh_folder("why-slap");
    let mut story = story(&folder);

    story
        .handle(Input::NpcSlapped {
            at: Tick(7),
            name: "Innkeeper Farley".to_string(),
        })
        .unwrap();

    let why = TrustWhy {
        by: TrustCause::Slap,
        up: false,
        at: Tick(7),
    };
    assert_eq!(trust_why_of(&mut story, "Innkeeper Farley"), Some(why));
}

#[test]
fn an_npc_that_nothing_changed_has_no_why() {
    let folder = fresh_folder("why-none");
    let mut story = story(&folder);

    meet(&mut story, 2, "Innkeeper Farley");

    assert_eq!(trust_why_of(&mut story, "Innkeeper Farley"), None);
}

#[test]
fn a_trust_change_whose_cause_is_lost_shows_no_why() {
    let folder = fresh_folder("why-lost");
    let mut first = story(&folder);
    first
        .handle(Input::NpcSlapped {
            at: Tick(7),
            name: "Innkeeper Farley".to_string(),
        })
        .unwrap();
    drop(first);
    let connection = sql(&folder);
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    connection
        .execute("DELETE FROM inputs WHERE kind = 'npc_slapped'", [])
        .unwrap();
    drop(connection);

    let mut second = story(&folder);

    assert_eq!(trust_why_of(&mut second, "Innkeeper Farley"), None);
}

#[test]
fn a_lore_question_leaves_no_input_and_no_call() {
    let folder = fresh_folder("lore-no-row");
    let tower = Passage {
        text: "The tower of Elwynn fell long ago.".to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![Link::Place("Elwynn Forest".to_string())],
        origin: Origin::Pack,
    };
    Pack::write(&folder.join("pack.sqlite"), &[tower]).unwrap();
    let mut story = story(&folder);
    enter(&mut story, 1, "Elwynn Forest", None);
    let inputs_before = count(&folder, "SELECT count(*) FROM inputs");

    let outputs = story
        .handle(Input::LoreAsked {
            id: MessageId(7),
            question: "why did the tower fall?".to_string(),
            target: None,
        })
        .unwrap();
    let call = call_of(&outputs);
    story
        .handle(Input::ModelAnswered {
            call,
            text: "Nobody knows [1].".to_string(),
        })
        .unwrap();
    drop(story);

    assert_eq!(count(&folder, "SELECT count(*) FROM inputs"), inputs_before);
    assert_eq!(count(&folder, "SELECT count(*) FROM calls"), 0);
}
