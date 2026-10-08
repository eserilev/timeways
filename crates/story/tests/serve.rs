//! One line in, the answer lines out. A request that fails still gets an answer, because
//! the bridge stops a story program that leaves a request with no answer (relay SPEC.md 9.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};
use std::path::Path;
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("serve-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

fn send(story: &mut Story, line: &Value) -> serve::Served {
    serve::line(story, line.to_string().into_bytes())
}

fn answer(served: &serve::Served) -> Value {
    assert_eq!(served.lines.len(), 1, "{served:?}");
    serde_json::from_str(&served.lines[0]).unwrap()
}

#[test]
fn a_question_before_any_character_gets_an_empty_answer_and_a_log_line() {
    let mut story = story("no-character");

    let served = send(
        &mut story,
        &json!({"type": "lore_asked", "id": 4, "question": "who?"}),
    );

    assert_eq!(
        answer(&served),
        json!({"type": "lore_answer", "id": 4, "text": null, "passages": []})
    );
    assert!(served.error.unwrap().contains("no character"));
}

#[test]
fn a_talk_that_fails_gets_an_answer_with_no_words() {
    let mut story = story("talk");
    send(
        &mut story,
        &json!({"type": "character_entered", "realm": "R", "name": "Ada"}),
    );

    let talk = json!({"type": "talk_asked", "id": 5, "at": 1, "npc": "Hogger", "text": " "});
    let served = send(&mut story, &talk);

    assert_eq!(
        answer(&served),
        json!({"type": "talk_answer", "id": 5, "npc": "Hogger", "text": null})
    );
}

#[test]
fn a_talk_reply_with_a_name_in_no_fact_shows_nothing() {
    let mut story = story("names");
    send(
        &mut story,
        &json!({"type": "character_entered", "realm": "R", "name": "Ada"}),
    );
    let talk = json!({"type": "talk_asked", "id": 5, "at": 1, "npc": "Hogger", "text": "hi"});
    let call = answer(&send(&mut story, &talk))["call"].clone();

    let say = r#"{"say": "Go ask Varian.", "trust": 0}"#;
    let served = send(
        &mut story,
        &json!({"type": "model_answered", "call": call, "text": say}),
    );

    assert_eq!(answer(&served)["text"], json!(null));
}

#[test]
fn a_journal_that_fails_has_no_pages() {
    let mut story = story("journal");

    let served = send(
        &mut story,
        &json!({"type": "journal_asked", "id": 6, "page": 0}),
    );

    let page = answer(&served);
    assert_eq!(
        (page["page"].clone(), page["pages"].clone()),
        (json!(0), json!(0))
    );
}

#[test]
fn a_game_event_that_fails_gets_no_answer_of_its_own() {
    let mut story = story("event");

    let served = send(
        &mut story,
        &json!({"type": "npc_met", "id": 7, "at": 1, "name": "X"}),
    );

    assert!(served.lines.is_empty());
    assert!(served.error.is_some());
}

#[test]
fn a_time_far_ahead_is_refused_and_later_events_still_land() {
    let mut story = story("far-ahead");
    send(
        &mut story,
        &json!({"type": "character_entered", "realm": "R", "name": "Ada"}),
    );

    let jump = send(
        &mut story,
        &json!({"type": "npc_met", "id": 1, "at": u64::MAX, "name": "X"}),
    );
    let later = send(
        &mut story,
        &json!({"type": "npc_met", "id": 2, "at": 100, "name": "Y"}),
    );

    assert!(jump.error.unwrap().contains("more than a day after"));
    assert_eq!(later.error, None);
}

#[test]
fn a_time_before_the_last_event_counts_as_the_last_event() {
    let mut story = story("back");
    send(
        &mut story,
        &json!({"type": "character_entered", "realm": "R", "name": "Ada"}),
    );
    send(
        &mut story,
        &json!({"type": "npc_met", "id": 1, "at": 500, "name": "X"}),
    );

    let earlier = send(
        &mut story,
        &json!({"type": "npc_met", "id": 2, "at": 100, "name": "Y"}),
    );

    assert_eq!(earlier.error, None);
}

#[test]
fn a_talk_to_a_name_past_the_limit_of_the_bridge_answers_with_no_name() {
    let mut story = story("long-npc");
    send(
        &mut story,
        &json!({"type": "character_entered", "realm": "R", "name": "Ada"}),
    );
    let npc = "N".repeat(65);

    let talk = json!({"type": "talk_asked", "id": 5, "at": 1, "npc": npc, "text": "hi"});
    let served = send(&mut story, &talk);

    assert_eq!(answer(&served)["npc"], json!(""));
}
