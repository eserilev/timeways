//! The text of a story: its title, its paragraphs, and the signs that no wire takes
//! (GAMEPLAY.md 4.8).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

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
