//! A `story_accepted` line with any title, any paragraphs, and any number (GAMEPLAY.md
//! 4.8). A line that the bridge passes lands or is refused as `BadStory` or `StoryTaken`,
//! each page of the journal fits the game, and a kept story keeps its limits.
//!
//! An input that reads as a JSON object is the story itself, as the seeds are:
//! `{"number": 7, "title": "...", "paragraphs": ["..."]}`. Any other input builds one.

#![no_main]

#[path = "common.rs"]
mod common;

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use serde_json::{Value, json};
use timeways_story::input::{Input, MessageId};
use timeways_story::stories::{MAX_PARAGRAPHS, PlayerStory};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story, StoryError};

/// The longest string of a journal (relay SPEC 9.8).
const JOURNAL_STRING_BYTES: usize = 1600;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

#[derive(Arbitrary, Debug)]
struct Told {
    number: u64,
    title: Option<String>,
    paragraphs: Vec<String>,
}

fn told(data: &[u8]) -> Option<Value> {
    if let Ok(Value::Object(fields)) = serde_json::from_slice::<Value>(data) {
        return Some(Value::Object(fields));
    }
    let told = Told::arbitrary(&mut Unstructured::new(data)).ok()?;
    Some(json!({"number": told.number, "title": told.title, "paragraphs": told.paragraphs}))
}

fn line_of(mut told: Value) -> Value {
    told["type"] = json!("story_accepted");
    told["at"] = json!(1_790_000_000u64);
    told
}

fn story() -> Story {
    let mut story = Story::new(common::pack(), Store::Memory);
    let entered: Input = serde_json::from_str(CHARACTER).unwrap();
    story.handle(entered).unwrap();
    story
}

/// Every page of the journal, each checked as the bridge checks a reply.
fn journal_stories(story: &mut Story) -> Vec<PlayerStory> {
    let mut stories = Vec::new();
    let mut page = 0;
    loop {
        let outputs = story
            .handle(Input::JournalAsked {
                id: MessageId(1),
                page,
            })
            .unwrap();
        let line = serde_json::to_string(&outputs[0]).unwrap();
        assert!(fake_bridge::game_reply(&line).is_some(), "{line}");
        let Output::Journal { page: shown, .. } = &outputs[0] else {
            panic!("expected a journal");
        };
        stories.extend(shown.journal.stories.iter().cloned());
        page += 1;
        if page >= shown.pages {
            return stories;
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let Some(told) = told(data) else {
        return;
    };
    let line = line_of(told);
    let batch = format!("{CHARACTER}\n{line}");
    if fake_bridge::dropped_lines_of(&batch) != Some(0) {
        return;
    }
    let Ok(input) = serde_json::from_value::<Input>(line) else {
        return;
    };
    let mut story = story();

    let handled = story.handle(input);

    assert!(
        matches!(
            handled,
            Ok(_) | Err(StoryError::BadStory | StoryError::StoryTaken(_))
        ),
        "{handled:?}"
    );
    for kept in journal_stories(&mut story) {
        assert!((1..=MAX_PARAGRAPHS).contains(&kept.paragraphs.len()));
        for paragraph in &kept.paragraphs {
            assert!(paragraph.len() <= JOURNAL_STRING_BYTES, "{paragraph:?}");
            assert!(!paragraph.chars().any(char::is_control), "{paragraph:?}");
        }
    }
});
