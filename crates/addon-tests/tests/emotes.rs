#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;

const NOW: Tick = Tick(1_790_000_000);
/// The hour of `NOW` in UTC, as the fake game gives it.
const HOUR: u8 = 14;

fn sent_after_flush(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
}

fn emote(emote: &str, target: Option<&str>) -> Input {
    Input::EmoteDone {
        at: NOW,
        emote: emote.to_string(),
        target: target.map(str::to_string),
        hour: Some(HOUR),
    }
}

#[test]
fn a_slap_on_an_npc_target_sends_the_emote_and_the_slap() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }; C_ChatInfo.PerformEmote('SLAP', '')",
    );

    let slapped = Input::NpcSlapped {
        at: NOW,
        name: "Innkeeper Farley".to_string(),
    };
    assert_eq!(
        sent_after_flush(&game),
        [emote("slap", Some("Innkeeper Farley")), slapped]
    );
}

#[test]
fn a_dance_goes_out_with_its_npc_target_and_the_hour() {
    let game = Game::new();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; C_ChatInfo.PerformEmote('Dance')");

    assert_eq!(
        sent_after_flush(&game),
        [emote("dance", Some("Innkeeper Farley"))]
    );
}

#[test]
fn an_emote_at_a_player_or_a_pet_goes_out_with_no_target() {
    let game = Game::new();

    game.run("wow.units.target = { name = 'Grimtusk', player = true }; C_ChatInfo.PerformEmote('SLAP', '')");
    game.run("wow.units.target = { name = 'Fluffy', controlled = true }; C_ChatInfo.PerformEmote('SLAP', '')");

    let text = game.sent().concat();
    assert!(
        !text.contains("Grimtusk") && !text.contains("Fluffy"),
        "{text}"
    );
    assert_eq!(
        sent_after_flush(&game),
        [emote("slap", None), emote("slap", None)]
    );
}

#[test]
fn a_typed_name_is_never_sent_because_it_can_be_a_player() {
    let game = Game::new();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; C_ChatInfo.PerformEmote('SLAP', 'Grimtusk')");

    assert!(!game.sent().concat().contains("Grimtusk"));
    assert_eq!(sent_after_flush(&game), [emote("slap", None)]);
}

#[test]
fn a_hidden_target_name_is_never_used() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }
         wow.secrets['Innkeeper Farley'] = true
         C_ChatInfo.PerformEmote('SLAP', '')",
    );

    assert_eq!(sent_after_flush(&game), [emote("slap", None)]);
}

#[test]
fn a_token_that_is_not_a_word_is_no_emote() {
    let game = Game::new();

    game.run("C_ChatInfo.PerformEmote('DANCE2'); C_ChatInfo.PerformEmote(string.rep('A', 25)); C_ChatInfo.PerformEmote(nil)");

    assert!(sent_after_flush(&game).is_empty());
}
