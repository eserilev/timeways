//! Random lines from the bridge. No line ends the program, the bridge takes every line that
//! comes out, and each request gets exactly one answer.

#![no_main]

#[path = "common.rs"]
mod common;

use fake_bridge::{Checked, checked_line};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &[u8] = br#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

const REQUESTS: [&str; 3] = ["lore_asked", "talk_asked", "journal_asked"];

/// The id of a request line that the story program reads, or None.
fn request_id(bytes: &[u8]) -> Option<u64> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let kind = value.get("type")?.as_str()?;
    REQUESTS
        .contains(&kind)
        .then(|| value.get("id")?.as_u64())?
}

/// Sends a line and every `model_failed` that its calls need, and gives the ids of the
/// answers that came out.
fn send(story: &mut Story, bytes: Vec<u8>) -> Vec<u64> {
    let mut answered = Vec::new();
    let mut lines = serve::line(story, bytes).lines;
    while let Some(line) = lines.pop() {
        match checked_line(&line) {
            Checked::Hello => {}
            Checked::ModelCall(call, _) => {
                let failed = format!(r#"{{"type":"model_failed","call":{}}}"#, call.0);
                lines.extend(serve::line(story, failed.into_bytes()).lines);
            }
            Checked::Answer { id, .. } => answered.push(id),
        }
    }
    answered
}

fuzz_target!(|data: &[u8]| {
    let mut story = Story::new(common::pack(), Store::Memory);
    send(&mut story, CHARACTER.to_vec());
    for bytes in data.split(|byte| *byte == b'\n') {
        let answered = send(&mut story, bytes.to_vec());
        if let Some(id) = request_id(bytes) {
            assert_eq!(
                answered,
                [id],
                "a request with no answer, or with two: {bytes:?}"
            );
        }
    }
});
