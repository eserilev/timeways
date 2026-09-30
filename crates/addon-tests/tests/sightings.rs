//! The NPCs that the player sees by a hover or a target (GAMEPLAY.md 3.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{Input, Reaction};

const NOW: Tick = Tick(1_790_000_000);
const BAT: &str = "Creature-0-4372-0-17-1554-0000ABCDEF";
const KEEPER: &str = "Creature-0-4372-0-17-240-0000123456";

fn seen(name: &str, reaction: Reaction, creature: Option<&str>) -> Input {
    Input::NpcSeen {
        at: NOW,
        name: name.to_string(),
        reaction,
        creature: creature.map(str::to_string),
    }
}

fn sent_after_flush(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
}

#[test]
fn a_hovered_hostile_beast_goes_out_with_its_reaction_and_type() {
    let game = Game::new();

    game.run(&format!(
        "wow.units.mouseover = {{ name = 'Duskbat', guid = '{BAT}', hostile = true, creature = {{ 'Bestie', 1 }} }}
         wow.Fire('UPDATE_MOUSEOVER_UNIT')"
    ));

    assert_eq!(
        sent_after_flush(&game),
        [seen("Duskbat", Reaction::Hostile, Some("beast"))]
    );
}

#[test]
fn a_targeted_friendly_npc_goes_out_as_friendly() {
    let game = Game::new();

    game.run(&format!(
        "wow.units.target = {{ name = 'Keeper Tessa', guid = '{KEEPER}', creature = {{ 'Humanoide', 7 }} }}
         wow.Fire('PLAYER_TARGET_CHANGED')"
    ));

    assert_eq!(
        sent_after_flush(&game),
        [seen("Keeper Tessa", Reaction::Friendly, Some("humanoid"))]
    );
}

#[test]
fn an_npc_goes_out_once_in_a_session() {
    let game = Game::new();

    game.run(&format!(
        "wow.units.mouseover = {{ name = 'Duskbat', guid = '{BAT}', hostile = true }}
         wow.Fire('UPDATE_MOUSEOVER_UNIT')
         wow.units.target = {{ name = 'Duskbat', guid = 'Creature-0-4372-0-17-1554-0000FFFFFF', hostile = true }}
         wow.Fire('PLAYER_TARGET_CHANGED')"
    ));

    assert_eq!(
        sent_after_flush(&game),
        [seen("Duskbat", Reaction::Hostile, None)]
    );
}

#[test]
fn a_player_or_a_pet_never_goes_out() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Grimtusk', guid = 'Player-1-00000001', player = true }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.units.mouseover = { name = 'Fluffy', guid = 'Pet-0-4372-0-17-1554-0000ABCDEF', controlled = true }
         wow.Fire('UPDATE_MOUSEOVER_UNIT')",
    );

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_hidden_guid_or_name_never_goes_out() {
    let game = Game::new();

    game.run(&format!(
        "wow.secrets['{BAT}'] = true
         wow.units.mouseover = {{ name = 'Duskbat', guid = '{BAT}', hostile = true }}
         wow.Fire('UPDATE_MOUSEOVER_UNIT')
         wow.secrets['Keeper Tessa'] = true
         wow.units.target = {{ name = 'Keeper Tessa', guid = '{KEEPER}' }}
         wow.Fire('PLAYER_TARGET_CHANGED')"
    ));

    assert!(sent_after_flush(&game).is_empty());
}

#[test]
fn a_hidden_creature_type_goes_out_as_none() {
    let game = Game::new();

    game.run(&format!(
        "wow.secrets[1] = true
         wow.units.mouseover = {{ name = 'Duskbat', guid = '{BAT}', hostile = true, creature = {{ 'Beast', 1 }} }}
         wow.Fire('UPDATE_MOUSEOVER_UNIT')"
    ));

    assert_eq!(
        sent_after_flush(&game),
        [seen("Duskbat", Reaction::Hostile, None)]
    );
}
