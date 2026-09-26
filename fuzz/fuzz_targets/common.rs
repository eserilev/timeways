//! What the fuzz targets share: a small pack, and the limits of the bridge on each line.

use serde_json::Value;
use std::path::PathBuf;
use std::sync::OnceLock;
use timeways_story::journal::PAGE_BYTES;
use timeways_story::pack::{Link, Pack, Passage, Origin};

/// One pack for each fuzz process, written once.
pub fn pack() -> Pack {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    let path = PATH.get_or_init(|| {
        let path =
            std::env::temp_dir().join(format!("timeways-fuzz-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let passage = |text: &str, place: &str| Passage {
            text: text.to_string(),
            source: "https://example.test".to_string(),
            links: vec![Link::Place(place.to_string())],
            origin: Origin::Pack,
        };
        let passages = [
            passage("The tower fell.", "Testvale"),
            passage("The inn is old.", "Mockshire"),
        ];
        Pack::write(&path, &passages).unwrap();
        path
    });
    Pack::open(path).unwrap()
}

/// Every line of the story program has a known type, and each one stays inside the limits
/// that relay SPEC.md 9.8 checks.
pub fn check_output(line: &str) {
    let value: Value = serde_json::from_str(line).unwrap();
    let kind = value["type"].as_str().unwrap();
    match kind {
        "hello" => {}
        "lore_answer" | "journal" | "talk_answer" | "events_seen" => {
            assert!(line.len() <= PAGE_BYTES, "{kind} of {} bytes", line.len());
        }
        "model_call" => assert!(line.len() <= 256 * 1024, "a prompt of {} bytes", line.len()),
        other => panic!("an unknown output type {other}"),
    }
    if let Some(narrator) = value.get("narrator").and_then(Value::as_str) {
        assert!(narrator.len() <= 1000 && !narrator.chars().any(char::is_control));
    }
    if let Some(text) = value
        .get("text")
        .and_then(Value::as_str)
        .filter(|_| kind == "talk_answer")
    {
        assert!(text.len() <= 1600 && !text.chars().any(char::is_control));
    }
}
