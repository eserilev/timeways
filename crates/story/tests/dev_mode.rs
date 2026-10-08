//! Dev mode: a fake line of `/twdev` lands only while the desktop turned dev mode on
//! (TESTING.md, "Dev mode").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::{FakeBridge, Reply};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use timeways_story::dev_mode::{DevMode, SETTINGS_FILE, is_dev_line};
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
const DEV_LEVEL: &str = r#"{"type":"level_reached","at":100,"level":10,"dev":true}"#;

fn story(mode: DevMode) -> Story {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_dev_mode(mode);
    send(&mut story, CHARACTER);
    story
}

fn send(story: &mut Story, line: &str) -> serve::Served {
    serve::line(story, line.as_bytes().to_vec())
}

fn journal(story: &mut Story) -> Value {
    let served = send(story, r#"{"type":"journal_asked","id":9}"#);
    serde_json::from_str(&served.lines[0]).unwrap()
}

fn deeds(story: &mut Story) -> Vec<Value> {
    journal(story)["deeds"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-mode-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn dev_mode_is_off_with_no_settings_file() {
    assert_eq!(DevMode::of_folder(&folder("none")), DevMode::Off);
}

#[test]
fn only_dev_true_in_the_settings_turns_dev_mode_on() {
    assert_eq!(DevMode::of_settings("dev = true\n"), DevMode::On);
    for text in [
        "",
        "dev = false",
        "dev = \"true\"",
        "dev = 1",
        "dev = tru",
        "[x]\ndev = true",
    ] {
        assert_eq!(DevMode::of_settings(text), DevMode::Off, "{text}");
    }
}

#[test]
fn the_settings_file_in_the_story_folder_turns_dev_mode_on() {
    let folder = folder("on");
    std::fs::write(folder.join(SETTINGS_FILE), "dev = true\n").unwrap();

    assert_eq!(DevMode::of_folder(&folder), DevMode::On);
}

#[test]
fn any_line_with_a_dev_key_is_a_dev_line() {
    assert!(is_dev_line(DEV_LEVEL));
    assert!(is_dev_line(r#"{"type":"level_reached","dev":false}"#));
    assert!(is_dev_line(r#"{"type":"level_reached","dev":null}"#));
    assert!(!is_dev_line(
        r#"{"type":"level_reached","at":1,"level":10}"#
    ));
    assert!(!is_dev_line(r#"{"type":"npc_met","at":1,"name":"dev"}"#));
    assert!(!is_dev_line("not json"));
}

#[test]
fn a_dev_line_is_refused_while_dev_mode_is_off() {
    let mut story = story(DevMode::Off);

    let served = send(&mut story, DEV_LEVEL);

    assert!(served.error.unwrap().contains("dev mode is off"));
    assert!(deeds(&mut story).is_empty());
}

#[test]
fn a_dev_line_lands_while_dev_mode_is_on() {
    let mut story = story(DevMode::On);

    let served = send(&mut story, DEV_LEVEL);

    assert_eq!(served.error, None);
    assert_eq!(deeds(&mut story).len(), 1);
}

#[test]
fn a_refused_dev_request_still_gets_its_empty_answer() {
    let mut story = story(DevMode::Off);

    let served = send(
        &mut story,
        r#"{"type":"talk_asked","id":5,"at":1,"npc":"Hogger","text":"hi","dev":true}"#,
    );

    assert_eq!(
        serde_json::from_str::<Value>(&served.lines[0]).unwrap(),
        json!({"type": "talk_answer", "id": 5, "npc": "Hogger", "text": null})
    );
}

#[test]
fn the_journal_carries_the_dev_mark_only_while_dev_mode_is_on() {
    assert_eq!(journal(&mut story(DevMode::On))["dev"], json!(true));
    assert_eq!(journal(&mut story(DevMode::Off)).get("dev"), None);
}

#[test]
fn a_dev_event_line_passes_the_checks_of_the_bridge_with_its_mark() {
    let mut bridge = FakeBridge::new(story(DevMode::On));

    let reply = bridge.batch(&format!("{CHARACTER}\n{DEV_LEVEL}"));

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
    assert_eq!(bridge.dropped_lines(), 0);
    assert_eq!(deeds(bridge.story()).len(), 1);
}

#[test]
fn the_journal_with_the_dev_mark_passes_the_checks_of_the_bridge() {
    let mut bridge = FakeBridge::new(story(DevMode::On));

    let reply = bridge.batch(&format!("{CHARACTER}\n{}", r#"{"type":"journal_asked"}"#));

    let Reply::Done(text) = reply else {
        panic!("{reply:?}")
    };
    assert!(text.contains(r#""dev":true"#), "{text}");
}

/// The bridge passes the dev mark of a line with a reply on, so a fake question of
/// `/twdev` gets its answer while dev mode is on.
#[test]
fn a_dev_line_with_a_reply_passes_the_checks_of_the_bridge_with_its_mark() {
    let mut bridge = FakeBridge::new(story(DevMode::On));

    let reply = bridge.batch(&format!(
        "{CHARACTER}\n{}",
        r#"{"type":"journal_asked","dev":true}"#
    ));

    let Reply::Done(text) = reply else {
        panic!("{reply:?}")
    };
    assert!(text.contains(r#""type":"journal""#), "{text}");
    assert_eq!(bridge.dropped_lines(), 0);
}

#[test]
fn a_dev_line_with_a_reply_gets_no_answer_while_dev_mode_is_off() {
    let mut story = story(DevMode::Off);

    let served = send(
        &mut story,
        r#"{"type":"journal_asked","id":5,"page":0,"dev":true}"#,
    );

    assert!(served.error.unwrap().contains("dev mode is off"));
}
