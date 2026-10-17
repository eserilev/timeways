//! Any story of a peer, a title and paragraphs, through the reader's addon and the story
//! program (GAMEPLAY.md 4.8 and 5.11). The page shows the same words that the desktop keeps,
//! the desktop keeps every story that the addon accepts, the marks name at most 32 other
//! players, and no Lua error comes of it.

#![no_main]

#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;
#[path = "../../crates/addon-tests/tests/players/mod.rs"]
mod players;

use libfuzzer_sys::fuzz_target;
use players::{Player, peer_marks_disagree};

thread_local! {
    static READER: Player = Player::new("Corvin");
}

fuzz_target!(|texts: Vec<Vec<u8>>| {
    // A title and one to twenty paragraphs.
    if !(2..=21).contains(&texts.len()) {
        return;
    }
    READER.with(|reader| {
        let differ = peer_marks_disagree(reader, "Corvin", &texts);
        assert!(differ.is_none(), "{}", differ.unwrap_or_default());
    });
});
