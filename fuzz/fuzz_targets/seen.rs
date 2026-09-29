//! Random game text and a random name of the character, through Seen.lua of the real addon.
//! The bridge takes every line that goes out, the name never goes out as a word, and no
//! text passes the limit of 2000 bytes (GAMEPLAY.md 5.10 and 5.11).

#![no_main]

#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod game;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

const NAMES: [&str; 5] = ["Ada", "Ed", "Éowyn", "Al", "Zzzz"];

#[derive(Arbitrary, Debug)]
struct Text {
    /// A name of the list, or any name.
    name: Result<u8, String>,
    /// The name, cut into the text at these places, in any case.
    marks: Vec<(u16, bool)>,
    text: String,
    book: bool,
}

/// The text with the name in it at the marks, upper case when the mark says so.
fn with_name(text: &str, name: &str, marks: &[(u16, bool)]) -> String {
    let mut out = text.to_string();
    for (at, upper) in marks {
        let at = usize::from(*at) % (out.len() + 1);
        let at = (0..=at)
            .rev()
            .find(|at| out.is_char_boundary(*at))
            .unwrap_or(0);
        let name = if *upper {
            name.to_uppercase()
        } else {
            name.to_string()
        };
        out.insert_str(at, &name);
    }
    out
}

/// The name as a whole word, in any case.
fn holds_word(text: &str, name: &str) -> bool {
    let words = |text: &str| -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .map(str::to_lowercase)
            .collect()
    };
    let name = name.to_lowercase();
    words(text).contains(&name)
}

fuzz_target!(|input: Text| {
    let name = match &input.name {
        Ok(index) => NAMES[usize::from(*index) % NAMES.len()].to_string(),
        Err(name) => name.clone(),
    };
    let text = with_name(&input.text, &name, &input.marks);
    let game = game::Game::new();
    let set: mlua::Function = game.eval(
        "return function(name, text, book)
             wow.zone = 'Elwynn Forest'
             wow.units.player = { name = name, player = true }
             wow.units.npc = { name = 'Innkeeper Farley' }
             if book then
                 wow.text.item = 'A Book'
                 wow.text.page = text
                 wow.Fire('ITEM_TEXT_READY')
             else
                 wow.text.gossip = text
                 wow.Fire('GOSSIP_SHOW')
             end
             wow.RunTickers()
         end",
    );
    set.call::<()>((name.as_str(), text.as_str(), input.book))
        .unwrap();

    // A name in the game is letters only. The addon folds the case of ASCII and Latin-1, the
    // letters of names on US and EU realms.
    let latin = |c: char| c.is_alphabetic() && u32::from(c) <= 0xFF;
    let real_name = name.chars().all(latin) && !name.is_empty();
    for batch in game.sent() {
        assert_eq!(fake_bridge::dropped_lines_of(&batch), Some(0), "{batch}");
        for line in batch.lines() {
            let value: Value = serde_json::from_str(line).unwrap();
            let Some(seen) = value["text"].as_str() else {
                continue;
            };
            assert!(seen.len() <= 2000, "{} bytes", seen.len());
            // `$N` is the mark of the name, so a name like "n" is in it.
            let without_marks = seen.replace("$N", " ");
            let leaks = real_name && holds_word(&without_marks, &name);
            assert!(!leaks, "{seen:?} holds {name:?}");
        }
    }
});
