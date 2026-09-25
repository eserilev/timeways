//! Reads game events from the bridge on stdin (GAMEPLAY.md 5.12).
//!
//! Stdout carries only the protocol, so every problem goes to stderr.

use std::io::{self, BufRead, Write};
use timeways_story::character::{Character, Refusal};
use timeways_story::record::Record;

fn main() -> io::Result<()> {
    let mut character = Character::new();
    let mut log = io::stderr().lock();
    for line in io::stdin().lock().lines() {
        let line = line?;
        match serde_json::from_str::<Record>(&line) {
            Err(error) => writeln!(log, "bad record: {error}: {line}")?,
            Ok(record) => {
                if let Err(refusal) = character.apply(&record) {
                    writeln!(log, "refused: {}: {line}", reasons(&refusal))?;
                }
            }
        }
    }
    Ok(())
}

fn reasons(refusal: &Refusal) -> String {
    let reasons: Vec<String> = refusal.iter().map(ToString::to_string).collect();
    reasons.join("; ")
}
