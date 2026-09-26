#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;

const NOW: Tick = Tick(1_790_000_000);

fn defeated(name: &str) -> Input {
    Input::NpcDefeated {
        at: NOW,
        name: name.to_string(),
    }
}

fn died(killer: Option<&str>) -> Input {
    Input::Died {
        at: NOW,
        killer: killer.map(str::to_string),
        cause: None,
        killer_level: None,
        hour: Some(14),
    }
}

/// Puts a unit on the target and fires the event, as the game does.
fn target(game: &Game, name: &str, guid: &str, classification: &str) {
    game.run(&format!(
        "wow.units.target = {{ name = '{name}', guid = '{guid}', classification = '{classification}' }}
         wow.Fire('PLAYER_TARGET_CHANGED')"
    ));
}

fn sent_after_flush(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
}

#[test]
fn a_rare_that_you_saw_and_your_group_killed_is_sent() {
    let game = Game::new();
    target(&game, "Mother Fang", "Creature-1", "rare");

    game.run("wow.Fire('PARTY_KILL', 'Player-1', 'Creature-1')");

    assert_eq!(sent_after_flush(&game), [defeated("Mother Fang")]);
}

#[test]
fn a_rare_on_a_nameplate_counts_as_seen() {
    let game = Game::new();
    game.run(
        "wow.units.nameplate1 = { name = 'Mother Fang', guid = 'Creature-1', classification = 'rareelite' }
         wow.Fire('NAME_PLATE_UNIT_ADDED', 'nameplate1')
         wow.Fire('PARTY_KILL', 'Player-1', 'Creature-1')",
    );

    assert_eq!(sent_after_flush(&game), [defeated("Mother Fang")]);
}

#[test]
fn a_common_mob_kill_is_not_sent() {
    let game = Game::new();
    target(&game, "Kobold Vermin", "Creature-2", "normal");

    game.run("wow.Fire('PARTY_KILL', 'Player-1', 'Creature-2')");

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_kill_of_a_unit_that_you_never_saw_is_not_sent() {
    let game = Game::new();

    game.run("wow.Fire('PARTY_KILL', 'Player-1', 'Creature-9')");

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_killed_player_is_never_sent() {
    let game = Game::new();
    game.run(
        "wow.units.target = { name = 'Grimtusk', guid = 'Player-7', classification = 'rare', player = true }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.Fire('PARTY_KILL', 'Player-1', 'Player-7')",
    );

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_hidden_guid_is_never_used() {
    let game = Game::new();
    game.run("wow.secrets['Creature-1'] = true");
    target(&game, "Mother Fang", "Creature-1", "rare");

    game.run("wow.Fire('PARTY_KILL', 'Player-1', 'Creature-1')");

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_won_encounter_is_a_boss_kill() {
    let game = Game::new();

    game.run("wow.Fire('ENCOUNTER_END', 1084, 'Onyxia', 9, 40, 1)");

    assert_eq!(sent_after_flush(&game), [defeated("Onyxia")]);
}

#[test]
fn a_lost_encounter_is_not_a_kill() {
    let game = Game::new();

    game.run("wow.Fire('ENCOUNTER_END', 1084, 'Onyxia', 9, 40, 0)");

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_raid_boss_that_fires_both_events_counts_once() {
    let game = Game::new();
    target(&game, "Onyxia", "Creature-3", "worldboss");

    game.run(
        "wow.Fire('PARTY_KILL', 'Player-1', 'Creature-3')
         wow.now = wow.now + 5
         wow.Fire('ENCOUNTER_END', 1084, 'Onyxia', 9, 40, 1)",
    );

    assert_eq!(sent_after_flush(&game), [defeated("Onyxia")]);
}

#[test]
fn the_same_boss_after_the_next_reset_counts_again() {
    let game = Game::new();

    game.run(
        "wow.Fire('ENCOUNTER_END', 1084, 'Onyxia', 9, 40, 1)
         wow.now = wow.now + 7 * 24 * 3600
         wow.Fire('ENCOUNTER_END', 1084, 'Onyxia', 9, 40, 1)",
    );

    let later = Input::NpcDefeated {
        at: Tick(NOW.0 + 7 * 24 * 3600),
        name: "Onyxia".to_string(),
    };
    assert_eq!(sent_after_flush(&game), [defeated("Onyxia"), later]);
}

#[test]
fn a_death_to_an_npc_that_you_saw_names_the_killer() {
    let game = Game::new();
    target(&game, "Murloc Forager", "Creature-4", "normal");

    game.run("wow.recap = { { sourceName = 'Murloc Forager' } }; wow.Fire('PLAYER_DEAD')");

    assert_eq!(sent_after_flush(&game), [died(Some("Murloc Forager"))]);
}

#[test]
fn a_death_to_a_player_never_names_them() {
    let game = Game::new();
    game.run(
        "wow.units.target = { name = 'Grimtusk', guid = 'Player-7', player = true }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.recap = { { sourceName = 'Grimtusk' } }
         wow.Fire('PLAYER_DEAD')",
    );

    assert!(!game.sent().concat().contains("Grimtusk"));
    assert_eq!(sent_after_flush(&game), [died(None)]);
}

#[test]
fn a_name_that_was_ever_on_a_player_is_never_sent_as_a_killer() {
    let game = Game::new();
    target(&game, "Hogger", "Creature-5", "normal");
    game.run(
        "wow.units.target = { name = 'Hogger', guid = 'Player-8', player = true }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.recap = { { sourceName = 'Hogger' } }
         wow.Fire('PLAYER_DEAD')",
    );

    assert_eq!(sent_after_flush(&game), [died(None)]);
}

#[test]
fn a_death_with_an_unseen_killer_no_recap_or_a_hidden_caster_names_no_one() {
    let game = Game::new();
    target(&game, "Hogger", "Creature-5", "normal");

    game.run(
        "wow.recap = { { sourceName = 'Stranger' } }
         wow.Fire('PLAYER_DEAD')
         wow.recap = nil
         wow.Fire('PLAYER_DEAD')
         wow.recap = { { sourceName = 'Hogger', hideCaster = true } }
         wow.Fire('PLAYER_DEAD')",
    );

    assert_eq!(
        sent_after_flush(&game),
        [died(None), died(None), died(None)]
    );
}

#[test]
fn a_hidden_killer_name_is_never_used() {
    let game = Game::new();
    target(&game, "Hogger", "Creature-5", "normal");

    game.run("wow.secrets['Hogger'] = true; wow.recap = { { sourceName = 'Hogger' } }; wow.Fire('PLAYER_DEAD')");

    assert_eq!(sent_after_flush(&game), [died(None)]);
}

#[test]
fn a_pet_that_kills_you_is_never_named() {
    let game = Game::new();
    game.run(
        "wow.units.mouseover = { name = 'Fluffy', guid = 'Pet-1', controlled = true }
         wow.Fire('UPDATE_MOUSEOVER_UNIT')
         wow.recap = { { sourceName = 'Fluffy' } }
         wow.Fire('PLAYER_DEAD')",
    );

    assert_eq!(sent_after_flush(&game), [died(None)]);
}

#[test]
fn a_fall_goes_out_as_the_cause_of_a_death() {
    let game = Game::new();

    game.run("wow.recap = { { environmentalType = 'Falling' } }; wow.Fire('PLAYER_DEAD')");

    let fall = Input::Died {
        at: NOW,
        killer: None,
        cause: Some("falling".to_string()),
        killer_level: None,
        hour: Some(14),
    };
    assert_eq!(sent_after_flush(&game), [fall]);
}

#[test]
fn a_known_killer_goes_out_with_its_level() {
    let game = Game::new();
    game.run(
        "wow.units.target = { name = 'Cow', guid = 'Creature-9', level = 1 }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.recap = { { sourceName = 'Cow' } }
         wow.Fire('PLAYER_DEAD')",
    );

    let cow = Input::Died {
        at: NOW,
        killer: Some("Cow".to_string()),
        cause: None,
        killer_level: Some(1),
        hour: Some(14),
    };
    assert_eq!(sent_after_flush(&game), [cow]);
}
