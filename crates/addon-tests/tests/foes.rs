#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{FoeKind, Input};

const NOW: Tick = Tick(1_790_000_000);

fn defeated(name: &str, kind: FoeKind) -> Input {
    Input::NpcDefeated {
        at: NOW,
        name: name.to_string(),
        kind: Some(kind),
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

    assert_eq!(
        sent_after_flush(&game),
        [defeated("Mother Fang", FoeKind::Rare)]
    );
}

#[test]
fn a_rare_on_a_nameplate_counts_as_seen() {
    let game = Game::new();
    game.run(
        "wow.units.nameplate1 = { name = 'Mother Fang', guid = 'Creature-1', classification = 'rareelite' }
         wow.Fire('NAME_PLATE_UNIT_ADDED', 'nameplate1')
         wow.Fire('PARTY_KILL', 'Player-1', 'Creature-1')",
    );

    assert_eq!(
        sent_after_flush(&game),
        [defeated("Mother Fang", FoeKind::RareElite)]
    );
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

    assert_eq!(sent_after_flush(&game), [defeated("Onyxia", FoeKind::Boss)]);
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

    assert_eq!(
        sent_after_flush(&game),
        [defeated("Onyxia", FoeKind::WorldBoss)]
    );
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
        kind: Some(FoeKind::Boss),
    };
    assert_eq!(
        sent_after_flush(&game),
        [defeated("Onyxia", FoeKind::Boss), later]
    );
}

#[test]
fn a_death_to_an_npc_that_you_saw_names_the_killer() {
    let game = Game::new();
    target(&game, "Murloc Forager", "Creature-4", "normal");

    game.run("wow.recap = { { sourceName = 'Murloc Forager', sourceGUID = 'Creature-4' } }; wow.Fire('PLAYER_DEAD')");

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
         wow.recap = { { sourceName = 'Hogger', sourceGUID = 'Creature-5' } }
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
fn a_killer_with_no_guid_is_never_named() {
    let game = Game::new();
    target(&game, "Hogger", "Creature-5", "normal");

    game.run("wow.recap = { { sourceName = 'Hogger' } }; wow.Fire('PLAYER_DEAD')");

    assert_eq!(sent_after_flush(&game), [died(None)]);
}

/// A player that you never saw has a name that no filter of names knows.
#[test]
fn a_killer_with_a_player_guid_is_never_named() {
    let game = Game::new();

    game.run("wow.recap = { { sourceName = 'Grimtusk', sourceGUID = 'Player-1-0ABC' } }; wow.Fire('PLAYER_DEAD')");

    assert!(!game.sent().concat().contains("Grimtusk"));
    assert_eq!(sent_after_flush(&game), [died(None)]);
}

#[test]
fn a_killer_with_a_vehicle_guid_is_named() {
    let game = Game::new();

    game.run("wow.recap = { { sourceName = 'Siege Engine', sourceGUID = 'Vehicle-0-1-2-3-4-5' } }; wow.Fire('PLAYER_DEAD')");

    assert_eq!(sent_after_flush(&game), [died(Some("Siege Engine"))]);
}

#[test]
fn a_hidden_killer_guid_is_never_used() {
    let game = Game::new();

    game.run(
        "wow.secrets['Creature-5'] = true
         wow.recap = { { sourceName = 'Hogger', sourceGUID = 'Creature-5' } }
         wow.Fire('PLAYER_DEAD')",
    );

    assert_eq!(sent_after_flush(&game), [died(None)]);
}

#[test]
fn a_hidden_killer_name_is_never_used() {
    let game = Game::new();
    target(&game, "Hogger", "Creature-5", "normal");

    game.run("wow.secrets['Hogger'] = true; wow.recap = { { sourceName = 'Hogger', sourceGUID = 'Creature-5' } }; wow.Fire('PLAYER_DEAD')");

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
         wow.recap = { { sourceName = 'Cow', sourceGUID = 'Creature-9' } }
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

const BAT: &str = "Creature-0-4372-0-17-1554-0000ABCDEF";

/// A journal with one task in progress. Its open step kills 2 Duskbats.
const HUNT: &str = r#"{"type":"journal","page":0,"pages":1,"quests":[{"number":1,"status":"accepted","steps":[{"goal":"visit","place":"Mill Pond","state":"done"},{"goal":"kill","creature":"Duskbat","count":2,"state":"open","kills":0}]}]}"#;

/// The same task one step earlier: the visit is open, and the kill step waits.
const VISIT_FIRST: [(&str, &str); 2] = [
    (r#""state":"done""#, r#""state":"open""#),
    (r#""state":"open","kills""#, r#""state":"later","kills""#),
];

fn hunt_with(changes: &[(&str, &str)]) -> String {
    changes
        .iter()
        .fold(HUNT.to_string(), |hunt, (from, to)| hunt.replace(from, to))
}

fn killed(name: &str) -> Input {
    Input::NpcKilled {
        at: NOW,
        name: name.to_string(),
    }
}

fn hunted_bat(game: &Game, guid: &str) {
    game.run(&format!(
        "wow.units.target = {{ name = 'Duskbat', guid = '{guid}', hostile = true }}
         wow.Fire('PLAYER_TARGET_CHANGED')"
    ));
}

/// The sightings of the units go out too, so the test keeps only the kills.
fn kills_after_flush(game: &Game) -> Vec<Input> {
    let sent = sent_after_flush(game);
    sent.into_iter()
        .filter(|input| matches!(input, Input::NpcKilled { .. }))
        .collect()
}

#[test]
fn a_kill_of_a_creature_that_a_task_hunts_goes_out() {
    let game = Game::new();
    game.reply(HUNT);
    hunted_bat(&game, BAT);

    game.run(&format!("wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"));

    assert_eq!(kills_after_flush(&game), [killed("Duskbat")]);
}

#[test]
fn a_unit_counts_once() {
    let game = Game::new();
    game.reply(HUNT);
    hunted_bat(&game, BAT);

    game.run(&format!(
        "wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')
         wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"
    ));

    assert_eq!(kills_after_flush(&game), [killed("Duskbat")]);
}

#[test]
fn a_kill_that_no_task_hunts_stays_home() {
    let game = Game::new();
    hunted_bat(&game, BAT);

    game.run(&format!("wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"));

    assert!(kills_after_flush(&game).is_empty());
}

#[test]
fn a_kill_step_that_is_not_next_hunts_nothing() {
    let game = Game::new();
    game.reply(&hunt_after_a_visit());
    hunted_bat(&game, BAT);

    game.run(&format!("wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"));

    assert!(kills_after_flush(&game).is_empty());
}

#[test]
fn a_finished_hunt_forgets_the_units_that_it_saw() {
    let game = Game::new();
    game.reply(HUNT);
    hunted_bat(&game, BAT);

    game.reply(&hunt_with(&[(r#""state":"open""#, r#""state":"done""#)]));
    game.run(&format!("wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"));

    assert!(kills_after_flush(&game).is_empty());
}

#[test]
fn a_friendly_unit_of_a_hunted_name_never_counts() {
    let game = Game::new();
    game.reply(HUNT);
    game.run(&format!(
        "wow.units.target = {{ name = 'Duskbat', guid = '{BAT}' }}
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"
    ));

    assert!(kills_after_flush(&game).is_empty());
}

/// The same task one step earlier: a visit comes before the kill step.
fn hunt_after_a_visit() -> String {
    hunt_with(&VISIT_FIRST)
}

fn journal_asks(game: &Game) -> usize {
    game.sent_inputs()
        .iter()
        .filter(|input| matches!(input, Input::JournalAsked { .. }))
        .count()
}

#[test]
fn a_kill_step_after_a_visit_counts_without_opening_the_book() {
    let game = Game::new();
    game.reply(&hunt_after_a_visit());
    hunted_bat(&game, BAT);

    game.run(
        "wow.zone, wow.subzone = 'Duskwood', 'Mill Pond'
         wow.Fire('ZONE_CHANGED')
         wow.RunTickers()",
    );
    let asked = journal_asks(&game);
    game.reply(HUNT);
    game.run(&format!("wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"));

    assert_eq!(asked, 1);
    assert_eq!(kills_after_flush(&game), [killed("Duskbat")]);
}

#[test]
fn a_task_with_no_later_kill_step_asks_for_no_journal() {
    let game = Game::new();
    game.reply(HUNT);

    game.run(
        "wow.zone, wow.subzone = 'Duskwood', 'Mill Pond'
         wow.Fire('ZONE_CHANGED')
         wow.RunTickers()",
    );

    assert_eq!(journal_asks(&game), 0);
}

#[test]
fn the_journal_is_asked_for_at_most_once_a_minute() {
    let game = Game::new();
    game.reply(&hunt_after_a_visit());

    game.run(
        "for n = 1, 5 do
             wow.now = wow.now + 10
             wow.zone, wow.subzone = 'Duskwood', 'Place ' .. n
             wow.Fire('ZONE_CHANGED')
             wow.RunTickers()
         end",
    );

    assert_eq!(journal_asks(&game), 1);
}

#[test]
fn a_hunted_unit_that_you_already_target_counts() {
    let game = Game::new();
    hunted_bat(&game, BAT);

    game.reply(HUNT);
    game.run(&format!("wow.Fire('PARTY_KILL', 'Player-1', '{BAT}')"));

    assert_eq!(kills_after_flush(&game), [killed("Duskbat")]);
}
