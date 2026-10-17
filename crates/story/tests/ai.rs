//! AI off: the player chose no model at all (GAMEPLAY.md 5.6). The bridge says so in its
//! hello, and the journal tells the addon.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::{FakeBridge, Reply};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use timeways_story::ai::Ai;
use timeways_story::dev_mode::{DevMode, SETTINGS_FILE};
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
const HELLO_AI_OFF: &str = r#"{"type":"hello","protocol":1,"app":"timeways","ai":"off"}"#;

fn story() -> Story {
    Story::new(Pack::empty().unwrap(), Store::Memory)
}

fn send(story: &mut Story, line: &str) -> serve::Served {
    serve::line(story, line.as_bytes().to_vec())
}

fn journal(story: &mut Story) -> Value {
    send(story, CHARACTER);
    let served = send(story, r#"{"type":"journal_asked","id":9}"#);
    serde_json::from_str(&served.lines[0]).unwrap()
}

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ai-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn the_journal_says_ai_is_on_by_default() {
    assert_eq!(journal(&mut story())["ai"], json!("on"));
}

#[test]
fn the_journal_says_ai_is_off_after_a_hello_with_ai_off() {
    let mut story = story();

    send(&mut story, HELLO_AI_OFF);

    assert_eq!(journal(&mut story)["ai"], json!("off"));
}

#[test]
fn the_journal_with_ai_off_passes_the_checks_of_the_bridge() {
    let mut story = story();
    send(&mut story, HELLO_AI_OFF);
    let mut bridge = FakeBridge::new(story);

    let reply = bridge.batch(&format!("{CHARACTER}\n{}", r#"{"type":"journal_asked"}"#));

    let Reply::Done(text) = reply else {
        panic!("{reply:?}")
    };
    assert!(text.contains(r#""ai":"off""#), "{text}");
}

#[test]
fn a_journal_that_fails_still_says_ai_is_off() {
    let mut story = story();
    send(&mut story, HELLO_AI_OFF);

    // With no character yet, the request fails, and it gets the empty journal.
    let served = send(&mut story, r#"{"type":"journal_asked","id":9}"#);

    assert!(served.error.is_some());
    let empty: Value = serde_json::from_str(&served.lines[0]).unwrap();
    assert_eq!(empty["ai"], json!("off"));
}

#[test]
fn a_hello_with_an_unknown_ai_word_keeps_ai_on_and_gets_its_reply() {
    for word in [r#""local""#, r#""Off""#, "null", "1"] {
        let mut story = story();
        let hello = format!(r#"{{"type":"hello","protocol":1,"app":"timeways","ai":{word}}}"#);

        let served = send(&mut story, &hello);

        assert_eq!(
            served.lines,
            vec![r#"{"type":"hello","protocol":1}"#],
            "{word}"
        );
        assert_eq!(served.notes.len(), 1, "{word}");
        assert_eq!(story.ai(), Ai::On, "{word}");
    }
}

#[test]
fn only_ai_off_in_the_settings_turns_ai_off() {
    assert_eq!(Ai::of_settings("ai = \"off\"\n"), Ai::Off);
    for text in [
        "",
        "ai = \"on\"",
        "ai = false",
        "ai = \"of\"",
        "[x]\nai = \"off\"",
    ] {
        assert_eq!(Ai::of_settings(text), Ai::On, "{text}");
    }
}

#[test]
fn ai_off_in_the_settings_keeps_dev_mode_on() {
    let text = "dev = true\nai = \"off\"\n";

    assert_eq!(DevMode::of_settings(text), DevMode::On);
    assert_eq!(Ai::of_settings(text), Ai::Off);
}

#[test]
fn the_settings_file_in_the_story_folder_turns_ai_off() {
    let folder = folder("settings");
    std::fs::write(folder.join(SETTINGS_FILE), "ai = \"off\"\n").unwrap();

    assert_eq!(Ai::of_folder(&folder), Ai::Off);
    assert_eq!(Ai::of_folder(&folder.join("missing")), Ai::On);
}

#[test]
fn the_dev_switch_turns_ai_off_only_in_dev_mode() {
    let mut off_in_dev = story();
    off_in_dev.set_dev_mode(DevMode::On);
    off_in_dev.set_dev_ai(Ai::Off);
    let mut off_out_of_dev = story();
    off_out_of_dev.set_dev_ai(Ai::Off);

    assert_eq!(off_in_dev.ai(), Ai::Off);
    assert_eq!(off_out_of_dev.ai(), Ai::On);
}

#[test]
fn the_dev_switch_never_turns_ai_on() {
    let mut story = story();
    story.set_dev_mode(DevMode::On);
    send(&mut story, HELLO_AI_OFF);

    story.set_dev_ai(Ai::On);

    assert_eq!(story.ai(), Ai::Off);
}
