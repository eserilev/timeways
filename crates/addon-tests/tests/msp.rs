//! MSP, the profiles of roleplay addons (GAMEPLAY.md 3.7.1): the wire of `LibMSP` and Chomp,
//! the switch, the answers, the requests, the tooltip, and the import.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

fn text(game: &Game, code: &str) -> String {
    game.eval(code)
}

#[test]
fn the_version_is_the_crc32c_of_the_text_in_upper_case_hex() {
    let game = Game::new();

    assert_eq!(text(&game, "ns.MspWire.Version('123456789')"), "E3069283");
    assert_eq!(text(&game, "ns.MspWire.Version('a')"), "C1D04330");
    assert_eq!(text(&game, "ns.MspWire.Version('')"), "");
}

#[test]
fn a_field_carries_its_version_and_an_empty_field_is_bare() {
    let game = Game::new();

    assert_eq!(
        text(&game, "ns.MspWire.Field('NA', '123456789')"),
        "NAE3069283:123456789"
    );
    assert_eq!(text(&game, "ns.MspWire.Field('NA', '')"), "NA");
    assert_eq!(text(&game, "ns.MspWire.Field('NA', nil)"), "NA");
}

#[test]
fn a_backtick_in_a_text_becomes_a_quote() {
    let game = Game::new();

    let field = text(&game, "ns.MspWire.Field('MO', 'It`s me')");

    assert!(field.ends_with(":It's me"), "{field}");
}

#[test]
fn the_tooltip_lists_its_fields_in_order_and_ends_with_its_version() {
    let game = Game::new();

    let (block, version): (String, String) = game
        .lua
        .load("ns.MspWire.Tooltip({ VP = '3', NA = 'Ada', NT = 'Lady' })")
        .eval()
        .unwrap();

    let contents = "VP:3`VA`NA:Ada`NH`NI`NT:Lady`RA`CU`FR`FC";
    assert_eq!(block, format!("{contents}`TT{version}"));
    let expected: String = game.eval(&format!("ns.MspWire.Version('{contents}')"));
    assert_eq!(version, expected);
}

/// The commands of a message, as `kind field version text`.
fn commands(game: &Game, message: &str) -> Vec<String> {
    let parse: mlua::Function = game.eval(
        "return function(message)
             local out = {}
             for _, c in ipairs(ns.MspWire.Commands(message)) do
                 table.insert(out, table.concat({ c.kind, c.field, c.version, c.text or '-' }, ' '))
             end
             return out
         end",
    );
    parse.call(message).unwrap()
}

#[test]
fn a_message_holds_requests_fields_and_same_versions() {
    let game = Game::new();

    let parsed = commands(&game, "?NA`?DE1A2B`NA5F:Mary Sue`DE`!HI0C0FFEE`TTABC");

    assert_eq!(
        parsed,
        [
            "request NA  -",
            "request DE 1A2B -",
            "field NA 5F Mary Sue",
            "field DE  -",
            "same HI 0C0FFEE -",
            "field TT ABC -",
        ]
    );
}

#[test]
fn a_broken_command_is_skipped_and_a_version_of_zero_is_none() {
    let game = Game::new();

    let parsed = commands(&game, "na`N`?NAxyz`#NA`?DE0`CU:``");

    assert_eq!(parsed, ["request DE  -", "field CU  -"]);
}

#[test]
fn a_logged_text_escapes_four_letters_and_decodes_only_those() {
    let game = Game::new();

    let escaped = text(&game, r"ns.MspWire.Escape('a|b\\c~d\ne')");
    let decoded = text(&game, "ns.MspWire.Unescape('~7C~5C~7E~0A~41')");

    assert_eq!(escaped, "a~7Cb~5Cc~7Ed~0Ae");
    assert_eq!(decoded, "|\\~\n~41");
}

/// The parts of a text, from session 5.
fn split(game: &Game, code: &str) -> Vec<String> {
    game.eval(&format!("ns.MspParts.Split({code}, 5)"))
}

#[test]
fn a_part_starts_with_the_header_of_chomp() {
    let game = Game::new();

    let parts = split(&game, "'?TT'");

    assert_eq!(parts, ["00A005001001?TT"]);
}

#[test]
fn a_long_text_splits_into_parts_of_at_most_255_bytes_and_joins_again() {
    let game = Game::new();

    let parts = split(&game, "string.rep('\\195\\169', 300)");

    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|part| part.len() <= 255));
    assert!(parts[0].starts_with("00A005001003"), "{}", parts[0]);
    let whole: String = game.eval(
        "local collector = ns.MspParts.NewCollector()
         local whole
         for _, part in ipairs(ns.MspParts.Split(string.rep('\\195\\169', 300), 5)) do
             whole = ns.MspParts.Add(collector, 'Bo-Realm', part, false, 0)
         end
         return whole",
    );
    assert_eq!(whole, "é".repeat(300));
}

#[test]
fn a_split_never_cuts_an_escape_in_two() {
    let game = Game::new();

    let ok: bool = game.eval(
        "local text = ns.MspWire.Escape(string.rep('|', 200))
         for _, part in ipairs(ns.MspParts.Split(text, 1)) do
             local body = part:sub(13)
             if body:find('~%x?$') then return false end
         end
         return true",
    );

    assert!(ok);
}

#[test]
fn a_text_past_64_parts_has_no_parts() {
    let game = Game::new();

    let none: bool = game.eval("return ns.MspParts.Split(string.rep('a', 64 * 243 + 1), 1) == nil");

    assert!(none);
}

/// Adds each part for Bo, and returns the whole text and whether it came logged.
fn collect(game: &Game, parts: &[(&str, bool)]) -> (Option<String>, Option<bool>) {
    let add: mlua::Function = game.eval(
        "collector = collector or ns.MspParts.NewCollector()
         return function(part, logged)
             return ns.MspParts.Add(collector, 'Bo-Realm', part, logged, 0)
         end",
    );
    let mut last = (None, None);
    for (part, logged) in parts {
        last = add.call((*part, *logged)).unwrap();
    }
    last
}

#[test]
fn a_part_with_no_header_or_a_flag_of_a_broadcast_is_dropped() {
    let game = Game::new();

    assert_eq!(collect(&game, &[("?TT", false)]), (None, None));
    assert_eq!(collect(&game, &[("01A001001001?TT", false)]), (None, None));
    assert_eq!(collect(&game, &[("00A001002001?TT", false)]), (None, None));
    assert_eq!(
        collect(&game, &[("00A001001001?TT", false)]),
        (Some("?TT".to_string()), Some(false))
    );
}

#[test]
fn a_logged_part_is_decoded_and_one_unlogged_part_makes_the_message_unlogged() {
    let game = Game::new();

    let logged = collect(&game, &[("00A001001001NA:a~7Cb", true)]);
    let mixed = collect(
        &game,
        &[("00A002001002NA:a", true), ("00A002002002~7Cb", false)],
    );

    assert_eq!(logged, (Some("NA:a|b".to_string()), Some(true)));
    assert_eq!(mixed, (Some("NA:a~7Cb".to_string()), Some(false)));
}
