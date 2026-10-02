//! Lasting buffs and debuffs that a quest of the game puts on you (GAMEPLAY.md 5.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;

/// The player took "Rediscovering the Light" a moment ago.
fn just_took_the_quest() -> Game {
    let game = Game::new();
    game.run(
        "wow.questLog = {
             { title = 'Paladin', isHeader = true },
             { title = 'Rediscovering the Light', questID = 2, isHeader = false },
         }
         wow.Fire('QUEST_ACCEPTED', 2)",
    );
    game
}

/// Fires `UNIT_AURA` for the player with one new aura, written as a Lua table.
fn gain(game: &Game, aura: &str) {
    game.run(&format!(
        "wow.Fire('UNIT_AURA', 'player', {{ addedAuras = {{ {aura} }} }})"
    ));
}

const MARK: &str = "{ name = 'Touched by the Light', spellId = 90001, duration = 0,
                      sourceUnit = 'npc', isFromPlayerOrPlayerPet = false }";

fn marks(game: &Game) -> Vec<(String, String)> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::QuestMarked { quest, mark, .. } => Some((quest, mark)),
            _ => None,
        })
        .collect()
}

fn the_mark() -> Vec<(String, String)> {
    vec![(
        "Rediscovering the Light".to_string(),
        "Touched by the Light".to_string(),
    )]
}

#[test]
fn a_lasting_aura_from_an_npc_just_after_a_quest_event_is_its_mark() {
    let game = just_took_the_quest();

    gain(&game, MARK);

    assert_eq!(marks(&game), the_mark());
    assert!(game.sent_inputs().iter().all(|input| match input {
        Input::QuestMarked { at, .. } => *at == Tick(1_790_000_000),
        _ => true,
    }));
}

#[test]
fn the_same_aura_twice_is_sent_once() {
    let game = just_took_the_quest();

    gain(&game, MARK);
    gain(&game, MARK);

    assert_eq!(marks(&game), the_mark());
}

#[test]
fn an_aura_long_after_the_quest_event_belongs_to_no_quest() {
    let game = just_took_the_quest();

    game.run("wow.now = wow.now + 61");
    gain(&game, MARK);

    assert!(marks(&game).is_empty());
}

#[test]
fn progress_on_a_quest_is_a_quest_event_too() {
    let game = just_took_the_quest();
    game.run("wow.now = wow.now + 600; wow.Fire('QUEST_WATCH_UPDATE', 2)");

    gain(&game, MARK);

    assert_eq!(marks(&game), the_mark());
}

#[test]
fn an_aura_of_a_player_a_short_aura_or_an_ignored_one_is_no_mark() {
    let game = just_took_the_quest();
    game.run("wow.units.party1 = { name = 'Bren', player = true }");

    gain(
        &game,
        "{ name = 'Arcane Intellect', spellId = 1, duration = 1800, isFromPlayerOrPlayerPet = true }",
    );
    gain(
        &game,
        "{ name = 'Blessing', spellId = 2, duration = 1800, sourceUnit = 'party1' }",
    );
    gain(&game, "{ name = 'Well Fed', spellId = 3, duration = 300 }");
    gain(
        &game,
        "{ name = 'Resurrection Sickness', spellId = 15007, duration = 600 }",
    );

    assert!(marks(&game).is_empty(), "{:?}", marks(&game));
}

#[test]
fn a_ten_minute_aura_counts_as_lasting() {
    let game = just_took_the_quest();

    gain(
        &game,
        "{ name = 'Touched by the Light', spellId = 90001, duration = 600 }",
    );

    assert_eq!(marks(&game), the_mark());
}

#[test]
fn in_combat_at_login_or_with_a_hidden_value_nothing_is_sent() {
    let game = just_took_the_quest();

    game.run("wow.combat = true");
    gain(&game, MARK);
    game.run(
        "wow.combat = false
         wow.Fire('UNIT_AURA', 'player', { isFullUpdate = true, addedAuras = { { name = 'X', spellId = 5, duration = 0 } } })
         wow.secrets['Touched by the Light'] = true",
    );
    gain(&game, MARK);

    assert!(marks(&game).is_empty(), "{:?}", marks(&game));
}

#[test]
fn an_aura_of_another_unit_or_a_broken_update_raises_no_error() {
    let game = just_took_the_quest();

    game.run(
        "wow.Fire('UNIT_AURA', 'target', { addedAuras = { { name = 'X', spellId = 5, duration = 0 } } })
         wow.Fire('UNIT_AURA', 'player', 7)
         wow.Fire('UNIT_AURA', 'player', { addedAuras = { 5, { name = 3, spellId = 'x' } } })",
    );

    assert!(marks(&game).is_empty());
}

/// While the game restricts auras, the whole `UNIT_AURA` update is secret
/// (`SecretWhenAurasRestricted`). A test of a secret field is a Lua error in the game, so
/// the addon reads none of it.
#[test]
fn a_hidden_update_sends_nothing_and_raises_no_error() {
    let game = just_took_the_quest();

    game.run(&format!(
        "local added = {{ {MARK} }}
         wow.secrets[added] = true
         wow.Fire('UNIT_AURA', 'player', {{ addedAuras = added }})"
    ));

    assert!(marks(&game).is_empty(), "{:?}", marks(&game));
}
