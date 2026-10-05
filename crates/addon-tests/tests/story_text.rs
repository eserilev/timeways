//! The text of a story: its title, its paragraphs, and the signs that no wire takes
//! (GAMEPLAY.md 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::stories::{
    MAX_BODY_BYTES, MAX_BODY_CHARS, MAX_PARAGRAPHS, MAX_TITLE_BYTES, MAX_TITLE_CHARS,
};

fn is_clean(game: &Game, text: &str) -> bool {
    let check: mlua::Function = game.eval("return ns.TaskWire.IsCleanText");
    check.call(text).unwrap()
}

#[test]
fn a_c1_control_character_is_not_clean_text() {
    let game = Game::new();

    for c1 in ['\u{80}', '\u{85}', '\u{9f}'] {
        assert!(
            !is_clean(&game, &format!("We held{c1} the bridge.")),
            "{c1:?}"
        );
    }
    assert!(is_clean(&game, "We held the bridge, \u{a0}twice."));
}

fn body(game: &Game, typed: &str) -> String {
    let body: mlua::Function = game.eval("return ns.StoryText.Body");
    body.call(typed).unwrap()
}

fn problem(game: &Game, title: &str, body: &str) -> Option<String> {
    let problem: mlua::Function = game.eval("return ns.StoryText.Problem");
    problem.call((title, body)).unwrap()
}

/// The first and the last byte of the first bad sign, from 1, or none.
fn bad_sign(game: &Game, text: &str) -> Option<(usize, usize)> {
    let bad: mlua::Function = game.eval(
        "return function(text)
             local first, last = ns.StoryText.BadSign(text)
             return first and (first .. ':' .. last)
         end",
    );
    let found: Option<String> = bad.call(text).unwrap();
    let (first, last) = found?
        .split_once(':')
        .map(|(a, b)| (a.to_string(), b.to_string()))?;
    Some((first.parse().unwrap(), last.parse().unwrap()))
}

#[test]
fn a_blank_line_and_a_line_break_give_the_same_story() {
    let game = Game::new();

    assert_eq!(body(&game, "One.\n\n\nTwo."), "One.\nTwo.");
    assert_eq!(body(&game, "One.\nTwo."), "One.\nTwo.");
}

#[test]
fn a_carriage_return_becomes_a_paragraph_break() {
    let game = Game::new();

    assert_eq!(body(&game, "One.\r\nTwo.\rThree."), "One.\nTwo.\nThree.");
}

#[test]
fn spaces_at_the_ends_of_a_paragraph_go() {
    let game = Game::new();

    assert_eq!(body(&game, "  One.  \n   \n Two. "), "One.\nTwo.");
}

#[test]
fn a_body_of_twenty_one_paragraphs_has_a_problem() {
    let game = Game::new();
    let twenty = vec!["Hi."; 20].join("\n");
    let twenty_one = vec!["Hi."; 21].join("\n");

    assert_eq!(problem(&game, "", &twenty), None);
    assert_eq!(
        problem(&game, "", &twenty_one).as_deref(),
        Some("That's more than 20 paragraphs. Join some and try again.")
    );
}

#[test]
fn a_body_of_a_thousand_letters_fits_and_one_more_does_not() {
    let game = Game::new();

    assert_eq!(problem(&game, "", &"é".repeat(600)), None);
    assert_eq!(problem(&game, "", &"a".repeat(1000)), None);
    assert_eq!(
        problem(&game, "", &"a".repeat(1001)).as_deref(),
        Some("Too long to send. Try a shorter version.")
    );
    assert_eq!(
        problem(&game, "", &"é".repeat(601)).as_deref(),
        Some("Too long to send. Try a shorter version.")
    );
}

#[test]
fn a_typed_bar_is_found_at_its_place() {
    let game = Game::new();

    assert_eq!(bad_sign(&game, "We é| held."), Some((6, 6)));
    assert_eq!(bad_sign(&game, "One.\nTwo\u{85}."), Some((9, 10)));
    assert_eq!(bad_sign(&game, "One.\nTwo."), None);
    assert_eq!(
        problem(&game, "", "a | b").as_deref(),
        Some("Stories can't hold the || sign. Take it out and try again.")
    );
    assert_eq!(
        problem(&game, "A\ttitle", "b").as_deref(),
        Some("Some of these characters can't be sent. Take them out and try again.")
    );
}

#[test]
fn a_broken_letter_is_a_bad_sign() {
    let game = Game::new();
    let check: mlua::Function = game.eval(
        "return function()
             local first, last = ns.StoryText.BadSign('ok\\195')
             return first .. ':' .. last
         end",
    );

    assert_eq!(check.call::<String>(()).unwrap(), "3:3");
}

/// The desktop keeps a story that the addon sends, so both sides hold the same numbers.
#[test]
fn the_story_limits_of_the_desktop_match_the_addon() {
    let game = Game::new();

    let limits: Vec<usize> = game.eval(
        "return { ns.StoryText.TITLE_LETTERS, ns.StoryText.TITLE_BYTES,
             ns.StoryText.BODY_LETTERS, ns.StoryText.BODY_BYTES, ns.StoryText.MAX_PARAGRAPHS }",
    );

    assert_eq!(
        limits,
        [
            MAX_TITLE_CHARS,
            MAX_TITLE_BYTES,
            MAX_BODY_CHARS,
            MAX_BODY_BYTES,
            MAX_PARAGRAPHS
        ]
    );
}
