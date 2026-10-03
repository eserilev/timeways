//! `/timeways forget` (GAMEPLAY.md 5.14).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::Input;

#[test]
fn forget_asks_first_and_then_clears_the_history() {
    let game = Game::new();

    game.run("wow.Slash('/timeways', 'forget')");
    let asked_first = game.sent().is_empty();
    game.run("wow.AcceptPopup()");
    game.run("wow.RunTickers()");

    assert!(asked_first);
    let inputs = game.sent_inputs();
    assert!(
        matches!(inputs.as_slice(), [Input::HistoryCleared { .. }]),
        "{inputs:?}"
    );
    assert_eq!(game.printed(), ["|cffc8a064Timeways|r: History cleared."]);
}

#[test]
fn forget_sends_nothing_until_the_player_clears() {
    let game = Game::new();

    game.run("wow.Slash('/timeways', 'forget')");
    game.run("wow.RunTickers()");

    assert!(game.sent().is_empty());
}
