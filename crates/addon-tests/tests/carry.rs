//! The count of the item of an open carry step, at a meeting with its NPC
//! (docs/plans/quest-variety.md 4.9).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;

const NOW: Tick = Tick(1_790_000_000);

/// A quest in progress whose open step brings 10 Linen Cloth to Farmer Bram. You hold 6.
fn cloth_quest() -> Game {
    let game = Game::new();
    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"quests":[{"number":1,"status":"accepted","steps":[{"goal":"carry","item":"Linen Cloth","count":10,"npc":"Farmer Bram","state":"open"}]}]}"#,
    );
    game.run("wow.bags['Linen Cloth'] = 6");
    game
}

fn gossip(game: &Game, npc: &str) {
    game.run(&format!(
        "wow.units.npc = {{ name = '{npc}' }}
         wow.Fire('GOSSIP_SHOW')"
    ));
}

fn counts(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter(|input| matches!(input, Input::ItemsHeld { .. }))
        .collect()
}

fn held(count: u16) -> Input {
    Input::ItemsHeld {
        at: NOW,
        npc: "Farmer Bram".to_string(),
        item: "Linen Cloth".to_string(),
        count,
    }
}

#[test]
fn meeting_the_npc_of_a_carry_step_sends_the_count() {
    let game = cloth_quest();

    gossip(&game, "Farmer Bram");

    assert_eq!(counts(&game), [held(6)]);
}

#[test]
fn talking_to_the_npc_of_a_carry_step_sends_the_count() {
    let game = cloth_quest();

    game.run("wow.units.target = { name = 'Farmer Bram' }; wow.Slash('/talk', 'here you go')");

    assert_eq!(counts(&game), [held(6)]);
}

#[test]
fn a_meeting_within_five_minutes_still_sends_the_count() {
    let game = cloth_quest();
    gossip(&game, "Farmer Bram");

    game.run("wow.now = wow.now + 60; wow.bags['Linen Cloth'] = 10");
    gossip(&game, "Farmer Bram");

    let later = Input::ItemsHeld {
        at: Tick(NOW.0 + 60),
        npc: "Farmer Bram".to_string(),
        item: "Linen Cloth".to_string(),
        count: 10,
    };
    assert_eq!(counts(&game), [held(6), later]);
}

#[test]
fn a_meeting_with_another_npc_sends_no_count() {
    let game = cloth_quest();

    gossip(&game, "Miller Oda");

    assert!(counts(&game).is_empty());
}

#[test]
fn a_hidden_count_is_never_sent() {
    let game = cloth_quest();
    game.run("wow.secrets[6] = true");

    gossip(&game, "Farmer Bram");

    assert!(counts(&game).is_empty());
}

#[test]
fn one_npc_and_item_send_at_most_once_in_ten_seconds() {
    let game = cloth_quest();

    gossip(&game, "Farmer Bram");
    game.run("wow.now = wow.now + 9");
    gossip(&game, "Farmer Bram");

    assert_eq!(counts(&game), [held(6)]);
}
