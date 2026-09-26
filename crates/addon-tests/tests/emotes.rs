#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;

const NOW: Tick = Tick(1_790_000_000);

fn sent_after_flush(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
}

#[test]
fn a_slap_on_an_npc_target_is_sent() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }
         C_ChatInfo.PerformEmote('SLAP', '')",
    );

    let slapped = Input::NpcSlapped {
        at: NOW,
        name: "Innkeeper Farley".to_string(),
    };
    assert_eq!(sent_after_flush(&game), [slapped]);
}

#[test]
fn a_slap_on_a_player_is_never_sent() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Grimtusk', player = true }
         C_ChatInfo.PerformEmote('SLAP', '')",
    );

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_typed_name_is_never_sent_because_it_can_be_a_player() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }
         C_ChatInfo.PerformEmote('SLAP', 'Grimtusk')",
    );

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_slap_with_no_target_or_a_hidden_name_is_not_sent() {
    let game = Game::new();

    game.run("C_ChatInfo.PerformEmote('SLAP', '')");
    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }
         wow.secrets['Innkeeper Farley'] = true
         C_ChatInfo.PerformEmote('SLAP', '')",
    );

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn other_emotes_are_not_sent_yet() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }
         C_ChatInfo.PerformEmote('DANCE', '')
         C_ChatInfo.PerformEmote('Wave')",
    );

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_slap_on_a_pet_is_never_sent_because_a_player_named_it() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Fluffy', controlled = true }
         C_ChatInfo.PerformEmote('SLAP', '')",
    );

    assert!(sent_after_flush(&game).is_empty());
}
