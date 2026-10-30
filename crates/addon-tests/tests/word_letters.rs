//! The addon and the desktop cut a text into words by one rule (GAMEPLAY.md 5.11). A name
//! that one side reads as a word and the other side does not reaches a model unmarked.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use timeways_story::aliases::{is_capitalized, is_hidden, is_word_char, key_of};

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
fn the_addon_and_the_desktop_drop_the_same_hidden_characters() {
    let game = Game::new();
    let chars = every_char();
    let texts: Vec<String> = chars.iter().map(char::to_string).collect();
    let is_hidden_in_lua: mlua::Function = game.eval(
        "return function(texts)
             local out = {}
             for n, text in ipairs(texts) do out[n] = ns.TaskNames.Visible(text) == '' end
             return out
         end",
    );

    let lua: Vec<bool> = is_hidden_in_lua.call(texts).unwrap();

    let differ: Vec<String> = chars
        .iter()
        .zip(&lua)
        .filter(|(c, lua)| is_hidden(**c) != **lua)
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
        "{Corvin}\u{301} {Corvin} {Corvin}ⁿ {Corvin}ℕ {Corvin}Ⓐ {Corvin}\u{37E} {Corvin}\u{60C}"
    );
}

fn marked(game: &Game, text: &str) -> String {
    let mark: mlua::Function =
        game.eval("return function(text) return (ns.TaskNames.Marked(text)) end");
    mark.call(text).unwrap()
}

#[test]
fn only_a_letter_of_a_known_script_goes_on_with_a_name() {
    let game = Game::new();
    game.run("wow.units.party1 = { name = 'Ada', player = true }");

    let text = marked(
        &game,
        "Ada\u{1DC0} Ada\u{180E} Ada\u{1680}went Ada\u{3164} Ada฿ Ada। Ada\u{E000} Ada² Adaⅻ Adaé",
    );

    assert_eq!(
        text,
        "{Ada}\u{1DC0} {Ada} {Ada}\u{1680}went {Ada}\u{3164} {Ada}฿ {Ada}। {Ada}\u{E000} {Ada}² {Ada}ⅻ Adaé"
    );
}

#[test]
fn a_character_that_shows_nothing_inside_a_name_is_dropped_before_the_match() {
    let game = Game::new();
    game.run("wow.units.party1 = { name = 'Corvin', player = true }");

    let text = marked(
        &game,
        "Cor\u{200B}vin Co\u{AD}rvin Cor\u{FE0F}vin C\u{2060}orvin",
    );

    assert_eq!(text, "{Corvin} {Corvin} {Corvin} {Corvin}");
}

/// Texts that are no valid UTF-8: a lone continuation byte, a cut letter, a surrogate, a
/// code past U+10FFFF, an overlong form, and bytes that never start a letter.
const BROKEN: [&[u8]; 9] = [
    b"Ada\x80",
    b"Ada\xC3",
    b"Ada\xED\xA0\x80",
    b"Ada\xF4\x90\x80\x80",
    b"Ada\xC0\xAF",
    b"Ada\xFF\xFE",
    b"Ada\xE2\x82",
    b"\xF0\x9F\x98Ada",
    b"Ada\xE0\x80\x80x",
];

fn lossy_in_lua(game: &Game, bytes: &[u8]) -> Vec<u8> {
    let lossy: mlua::Function = game.eval("return function(text) return ns.Utf8.Lossy(text) end");
    let text = game.lua.create_string(bytes).unwrap();
    let out: mlua::LuaString = lossy.call(text).unwrap();
    out.as_bytes().to_vec()
}

#[test]
fn the_addon_replaces_broken_bytes_as_the_bridge_does() {
    let game = Game::new();

    for bytes in BROKEN {
        let lua = lossy_in_lua(&game, bytes);

        assert_eq!(lua, String::from_utf8_lossy(bytes).as_bytes(), "{bytes:?}");
    }
}

#[test]
fn a_name_before_broken_bytes_gets_its_mark() {
    let game = Game::new();
    game.run("wow.units.party1 = { name = 'Ada', player = true }");
    let mark: mlua::Function =
        game.eval("return function(text) return (ns.TaskNames.Marked(text)) end");

    for bytes in BROKEN {
        let text = game.lua.create_string(bytes).unwrap();
        let out: mlua::LuaString = mark.call(text).unwrap();

        let out = String::from_utf8(out.as_bytes().to_vec()).unwrap();
        assert!(out.contains("{Ada}"), "{bytes:?} became {out}");
    }
}

#[test]
fn any_bytes_become_the_text_that_the_bridge_reads() {
    let game = Game::new();
    let mut runner = TestRunner::new(Config::with_cases(512));
    let bytes = proptest::collection::vec(any::<u8>(), 0..24);

    let result = runner.run(&bytes, |bytes| {
        let lua = lossy_in_lua(&game, &bytes);
        prop_assert_eq!(lua, String::from_utf8_lossy(&bytes).as_bytes().to_vec());
        Ok(())
    });

    result.unwrap();
}

#[test]
fn the_addon_and_the_desktop_hold_the_same_common_words() {
    let game = Game::new();

    let mut lua: Vec<String> = game.eval("return ns.TaskNames.FunctionWords()");
    let mut desktop: Vec<String> = timeways_story::aliases::FUNCTION_WORDS
        .iter()
        .map(|word| (*word).to_string())
        .collect();

    lua.sort();
    desktop.sort();
    assert_eq!(lua, desktop);
}

#[test]
fn the_addon_and_the_desktop_fold_every_letter_alike() {
    let game = Game::new();
    let chars: Vec<char> = every_char()
        .into_iter()
        .filter(|c| is_word_char(*c))
        .collect();
    let texts: Vec<String> = chars.iter().map(char::to_string).collect();
    let fold: mlua::Function = game.eval(
        "return function(texts)
             local out = {}
             for n, text in ipairs(texts) do out[n] = ns.TaskNames.Fold(text) end
             return out
         end",
    );

    let lua: Vec<String> = fold.call(texts).unwrap();

    let differ: Vec<String> = chars
        .iter()
        .zip(&lua)
        .filter(|(c, lua)| key_of(&c.to_string()) != **lua)
        .map(|(c, lua)| format!("U+{:04X} lua={lua:?}", u32::from(*c)))
        .collect();
    assert!(
        differ.is_empty(),
        "{differ:?}: the test `the_capitals_table_is_the_one_that_the_desktop_folds` prints \
         the table to put in TaskNames.lua"
    );
}

/// One row of `CAPITALS` in TaskNames.lua: from `first` to `last`, every `step`-th code is
/// a capital, and its small letter is `offset` above it.
struct Run {
    first: u32,
    last: u32,
    step: u32,
    offset: i64,
}

/// Each letter past ASCII whose key is one other letter, with the distance to that letter.
/// İ folds to two letters, and the addon folds it on its own.
fn capitals() -> Vec<(u32, i64)> {
    let mut capitals = Vec::new();
    for c in every_char()
        .into_iter()
        .filter(|c| !c.is_ascii() && is_word_char(*c))
    {
        let small: Vec<char> = key_of(&c.to_string()).chars().collect();
        if let [small] = small[..]
            && small != c
        {
            capitals.push((
                u32::from(c),
                i64::from(u32::from(small)) - i64::from(u32::from(c)),
            ));
        }
    }
    capitals
}

/// The capitals in runs of one offset, a step of 1 or 2 apart.
fn runs(capitals: &[(u32, i64)]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for &(code, offset) in capitals {
        if let Some(run) = runs.last_mut()
            && run.offset == offset
            && code - run.last == run.step
        {
            run.last = code;
            continue;
        }
        if let Some(run) = runs.last_mut()
            && run.first == run.last
            && run.offset == offset
            && code - run.last == 2
        {
            run.step = 2;
            run.last = code;
            continue;
        }
        runs.push(Run {
            first: code,
            last: code,
            step: 1,
            offset,
        });
    }
    runs
}

fn letter(code: u32) -> char {
    char::from_u32(code).unwrap()
}

/// One row of `CAPITALS` as stylua writes it.
fn row(run: &Run) -> String {
    let small = letter(u32::try_from(i64::from(run.first) + run.offset).unwrap());
    format!(
        "\t{{ 0x{:04X}, 0x{:04X}, {}, {} }}, -- {} {small}\n",
        run.first,
        run.last,
        run.step,
        run.offset,
        letter(run.first)
    )
}

fn capitals_table() -> String {
    runs(&capitals()).iter().map(row).collect()
}

#[test]
fn the_capitals_table_is_the_one_that_the_desktop_folds() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../addon/Timeways/TaskNames.lua"
    );
    let source = std::fs::read_to_string(path).unwrap();
    let start = source.find("local CAPITALS = {\n").unwrap() + "local CAPITALS = {\n".len();
    let end = start + source[start..].find("}\n").unwrap();

    let table = capitals_table();

    assert!(
        source[start..end] == table,
        "Put these rows in CAPITALS of addon/Timeways/TaskNames.lua:\n{table}"
    );
}

#[test]
fn the_addon_and_the_desktop_find_the_same_capitals() {
    let game = Game::new();
    let chars: Vec<char> = every_char()
        .into_iter()
        .filter(|c| is_word_char(*c))
        .collect();
    let texts: Vec<String> = chars.iter().map(char::to_string).collect();
    let is_capital: mlua::Function = game.eval(
        "return function(texts)
             local out = {}
             for n, text in ipairs(texts) do out[n] = ns.TaskNames.IsCapitalized(text) end
             return out
         end",
    );

    let lua: Vec<bool> = is_capital.call(texts.clone()).unwrap();

    let differ: Vec<String> = texts
        .iter()
        .zip(&lua)
        .filter(|(text, lua)| is_capitalized(text) != **lua)
        .map(|(text, lua)| format!("{text} lua={lua}"))
        .collect();
    assert!(differ.is_empty(), "{differ:?}");
}
