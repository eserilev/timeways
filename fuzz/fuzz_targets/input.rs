//! Random lines from the bridge. No line ends the program, and every answer keeps the
//! limits of the bridge.

#![no_main]

#[path = "common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &[u8] = br#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;

fuzz_target!(|data: &[u8]| {
    let mut story = Story::new(common::pack(), Store::Memory);
    let _ = serve::line(&mut story, CHARACTER.to_vec());
    for bytes in data.split(|byte| *byte == b'\n') {
        if let Ok(lines) = serve::line(&mut story, bytes.to_vec()) {
            for line in lines {
                common::check_output(&line);
            }
        }
    }
});
