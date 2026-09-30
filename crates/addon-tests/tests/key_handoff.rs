//! The key comes from `Timeways_Key`, an addon that the desktop app writes outside the
//! folder that `CurseForge` replaces (relay SPEC.md 7.3.2). The handoff tries again at our
//! `ADDON_LOADED` and at `PLAYER_LOGIN`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

const KEY_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// A game with no key yet. Loading the key addon sets its global, as the real file does.
fn game_with_a_key_addon() -> Game {
    let game = Game::new();
    game.run(&format!(
        "ns.key = nil
         wow.loaded.Timeways_Key = nil
         C_AddOns.LoadAddOn = function(name)
             if name == 'Timeways_Key' then TimewaysKey = '{KEY_HEX}' end
             return true
         end"
    ));
    game
}

fn has_key(game: &Game) -> bool {
    game.eval("return ns.key ~= nil")
}

#[test]
fn our_addon_loaded_takes_the_key_and_clears_the_global() {
    let game = game_with_a_key_addon();

    game.run("wow.Fire('ADDON_LOADED', 'Timeways')");

    assert!(has_key(&game));
    assert!(game.eval::<bool>("return TimewaysKey == nil"));
    assert_eq!(
        game.eval::<String>("return ns.KeyHandoff.step"),
        "ADDON_LOADED"
    );
}

#[test]
fn another_addon_loaded_does_not_take_the_key() {
    let game = game_with_a_key_addon();

    game.run("wow.Fire('ADDON_LOADED', 'SomeOtherAddon')");

    assert!(!has_key(&game));
}

#[test]
fn the_login_takes_the_key_when_the_earlier_tries_failed() {
    let game = game_with_a_key_addon();

    game.run("wow.Fire('PLAYER_LOGIN')");

    assert!(has_key(&game));
    assert_eq!(
        game.eval::<String>("return ns.KeyHandoff.step"),
        "PLAYER_LOGIN"
    );
}

#[test]
fn with_no_key_addon_the_login_takes_nothing_and_raises_no_error() {
    let game = Game::new();
    game.run(
        "ns.key = nil
         wow.Fire('ADDON_LOADED', 'Timeways')
         wow.Fire('PLAYER_LOGIN')",
    );

    assert!(!has_key(&game));
}
