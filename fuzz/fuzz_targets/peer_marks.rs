//! Any text of a peer's story through the reader's addon and the story program (GAMEPLAY.md
//! 4.8 and 5.11). The page shows the same words that the desktop reads, the marks name at
//! most 32 players, and no Lua error comes of it.

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

fuzz_target!(|text: &[u8]| {
    // A typed `$N` reads as the reader on both sides, so it is no disagreement.
    if text.windows(2).any(|pair| pair == b"$N") {
        return;
    }
    READER.with(|reader| {
        let differ = peer_marks_disagree(reader, "Corvin", text);
        assert!(differ.is_none(), "{}", differ.unwrap_or_default());
    });
});
