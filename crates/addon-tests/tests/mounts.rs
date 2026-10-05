//! The mounts that you ride (GAMEPLAY.md 5.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::Input;

const RAM: &str = "{ name = 'Gray Ram', spellId = 6777, duration = 0, sourceUnit = 'player' }";

/// Casts the aura, mounts up at a run speed in yards a second, and waits for the check.
fn ride(game: &Game, aura: &str, speed: f64) {
    game.run(&format!(
        "wow.Fire('UNIT_AURA', 'player', {{ addedAuras = {{ {aura} }} }})
         wow.mounted = true
         wow.runSpeed = {speed}
         wow.Fire('PLAYER_MOUNT_DISPLAY_CHANGED')
         local timers = wow.after
         wow.after = {{}}
         for _, timer in ipairs(timers) do timer.callback() end"
    ));
}

fn rides(game: &Game) -> Vec<(String, Option<u16>)> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::MountRidden { mount, speed, .. } => Some((mount, speed)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_ride_sends_the_mount_and_its_speed_in_percent() {
    let game = Game::new();

    ride(&game, RAM, 11.2);

    assert_eq!(rides(&game), [("Gray Ram".to_string(), Some(160))]);
}

#[test]
fn an_epic_mount_runs_at_twice_the_speed() {
    let game = Game::new();

    ride(
        &game,
        "{ name = 'Swift Gray Ram', spellId = 23238, duration = 0, sourceUnit = 'player' }",
        14.0,
    );

    assert_eq!(rides(&game), [("Swift Gray Ram".to_string(), Some(200))]);
}

#[test]
fn the_same_mount_goes_once_in_a_session() {
    let game = Game::new();

    ride(&game, RAM, 11.2);
    game.run("wow.mounted = false; wow.Fire('PLAYER_MOUNT_DISPLAY_CHANGED')");
    ride(&game, RAM, 11.2);

    assert_eq!(rides(&game).len(), 1);
}

#[test]
fn an_aura_of_another_caster_or_with_an_end_names_no_mount() {
    let game = Game::new();

    ride(
        &game,
        "{ name = 'Blessing of Might', spellId = 19740, duration = 300, sourceUnit = 'player' }",
        11.2,
    );
    ride(
        &game,
        "{ name = 'Gift of the Wild', spellId = 21849, duration = 0, sourceUnit = 'party1' }",
        11.2,
    );

    assert!(rides(&game).is_empty());
}

#[test]
fn an_old_aura_names_no_mount() {
    let game = Game::new();
    game.run(&format!(
        "wow.Fire('UNIT_AURA', 'player', {{ addedAuras = {{ {RAM} }} }})"
    ));

    game.run("wow.now = wow.now + 60");
    ride(&game, "{ name = 'X', spellId = 1, duration = 30 }", 11.2);

    assert!(rides(&game).is_empty());
}

#[test]
fn a_hidden_speed_sends_the_mount_with_no_speed() {
    let game = Game::new();
    game.run("wow.secrets[11.2] = true");

    ride(&game, RAM, 11.2);

    assert_eq!(rides(&game), [("Gray Ram".to_string(), None)]);
}

#[test]
fn a_full_update_or_a_broken_aura_raises_no_error_and_sends_nothing() {
    let game = Game::new();

    game.run(
        "wow.Fire('UNIT_AURA', 'player', { isFullUpdate = true, addedAuras = { { name = 'Gray Ram', duration = 0, sourceUnit = 'player' } } })
         wow.Fire('UNIT_AURA', 'player', { addedAuras = { 5, { name = 3 } } })
         wow.Fire('UNIT_AURA', 'player', 7)
         wow.mounted = true
         wow.Fire('PLAYER_MOUNT_DISPLAY_CHANGED')
         for _, timer in ipairs(wow.after) do timer.callback() end",
    );

    assert!(rides(&game).is_empty());
}
