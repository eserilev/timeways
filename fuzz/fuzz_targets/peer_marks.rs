//! Any story of a peer, a title and paragraphs, through the reader's addon and the story
//! program (GAMEPLAY.md 4.8 and 5.11). The page shows the same words that the desktop keeps,
//! the desktop keeps every story that the addon accepts, the addon refuses a story exactly
//! when its marks name more than 32 other players, and no Lua error comes of it.
//!
//! An input that reads as a JSON object is the story itself, as the seeds are:
//! `{"title": "...", "paragraphs": ["..."]}`, with an empty title for none. Any other input
//! builds the texts, the title first, as any bytes.

#![no_main]

#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;
#[path = "../../crates/addon-tests/tests/players/mod.rs"]
mod players;

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use players::{Player, peer_marks_disagree};
use serde::Deserialize;

thread_local! {
    static READER: Player = Player::new("Corvin");
}

#[derive(Deserialize)]
struct Told {
    #[serde(default)]
    title: String,
    paragraphs: Vec<String>,
}

fn texts(data: &[u8]) -> Option<Vec<Vec<u8>>> {
    if let Ok(told) = serde_json::from_slice::<Told>(data) {
        let texts = std::iter::once(told.title).chain(told.paragraphs);
        return Some(texts.map(String::into_bytes).collect());
    }
    Vec::<Vec<u8>>::arbitrary_take_rest(Unstructured::new(data)).ok()
}

fuzz_target!(|data: &[u8]| {
    let Some(texts) = texts(data) else {
        return;
    };
    // A title and one to twenty paragraphs.
    if !(2..=21).contains(&texts.len()) {
        return;
    }
    READER.with(|reader| {
        let differ = peer_marks_disagree(reader, "Corvin", &texts);
        assert!(differ.is_none(), "{}", differ.unwrap_or_default());
    });
});
