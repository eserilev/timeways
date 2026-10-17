//! The addon and the desktop cut a text into words by one rule (GAMEPLAY.md 5.11). A name
//! that one side reads as a word and the other side does not reaches a model unmarked.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::aliases::is_word_char;

/// Every code point of the Basic Multilingual Plane, and a few past it.
fn every_char() -> Vec<char> {
    let bmp = (0..=0xFFFF).filter_map(char::from_u32);
    let past = ['\u{10000}', '\u{1F600}', '\u{1D400}', '\u{10FFFF}'];
    bmp.chain(past).collect()
}

#[test]
fn the_addon_and_the_desktop_agree_on_every_letter() {
    let game = Game::new();
    let chars = every_char();
    let texts: Vec<String> = chars.iter().map(char::to_string).collect();
    let is_letter: mlua::Function = game.eval(
        "return function(texts)
             local out = {}
             for n, text in ipairs(texts) do out[n] = ns.TaskNames.IsWordChar(text) end
             return out
         end",
    );

    let lua: Vec<bool> = is_letter.call(texts).unwrap();

    let differ: Vec<String> = chars
        .iter()
        .zip(&lua)
        .filter(|(c, lua)| is_word_char(**c) != **lua)
        .map(|(c, lua)| format!("U+{:04X} lua={lua}", u32::from(*c)))
        .collect();
    assert!(
        differ.is_empty(),
        "{} differ: {:?}",
        differ.len(),
        &differ[..differ.len().min(20)]
    );
}

#[test]
fn a_mark_or_a_symbol_after_a_name_ends_it_on_both_sides() {
    let game = Game::new();
    game.run("wow.units.party1 = { name = 'Corvin', player = true }");
    let mark: mlua::Function =
        game.eval("return function(text) return (ns.TaskNames.Marked(text)) end");

    let marked: String = mark
        .call("Corvin\u{301} Corvin\u{FEFF} Corvinⁿ Corvinℕ CorvinⒶ Corvin\u{37E} Corvin\u{60C}")
        .unwrap();

    assert_eq!(
        marked,
        "{Corvin}\u{301} {Corvin}\u{FEFF} {Corvin}ⁿ {Corvin}ℕ {Corvin}Ⓐ {Corvin}\u{37E} {Corvin}\u{60C}"
    );
}
