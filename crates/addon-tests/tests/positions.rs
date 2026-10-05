//! The addon sends where the player stands with a new place and a meeting (GAMEPLAY.md 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use serde_json::json;
use timeways_story::input::Input;
use timeways_story::spot::Spot;

const NOW: Tick = Tick(1_790_000_000);
const TIRISFAL: u32 = 1420;

fn spot(map: u32, x: i64, y: i64) -> Spot {
    serde_json::from_value(json!({"map": map, "x": x, "y": y})).unwrap()
}

/// A player in Tirisfal Glades at this point of its map.
fn standing_at(x: &str, y: &str) -> Game {
    let game = Game::new();
    game.run(&format!(
        "wow.maps[{TIRISFAL}] = {{ name = 'Tirisfal Glades', player = {{ x = {x}, y = {y} }} }}
         wow.playerMap = {TIRISFAL}"
    ));
    game
}

fn enter_brill(game: &Game) {
    game.run(
        "wow.zone, wow.subzone = 'Tirisfal Glades', 'Brill'
         wow.Fire('ZONE_CHANGED')
         wow.RunTickers()",
    );
}

fn sent_spot(game: &Game) -> Option<Spot> {
    match game.sent_inputs().first() {
        Some(Input::ZoneEntered { spot, .. } | Input::NpcMet { spot, .. }) => *spot,
        other => panic!("expected a zone or a meeting, got {other:?}"),
    }
}

#[test]
fn a_new_place_sends_where_the_player_stands() {
    let game = standing_at("0.5", "0.25");

    enter_brill(&game);

    let expected = Input::ZoneEntered {
        at: NOW,
        zone: "Tirisfal Glades".to_string(),
        subzone: Some("Brill".to_string()),
        spot: Some(spot(TIRISFAL, 500, 250)),
        hour: Some(14),
        taxi: None,
    };
    assert_eq!(game.sent_inputs(), [expected]);
}

#[test]
fn a_meeting_sends_where_the_player_stands() {
    let game = standing_at("0.61", "0.52");

    game.run(
        "wow.units.npc = { name = 'Keeper Tessa' }
         wow.Fire('GOSSIP_SHOW')
         wow.RunTickers()",
    );

    let expected = Input::NpcMet {
        at: NOW,
        name: "Keeper Tessa".to_string(),
        spot: Some(spot(TIRISFAL, 610, 520)),
    };
    assert_eq!(game.sent_inputs(), [expected]);
}

#[test]
fn a_position_rounds_to_whole_thousandths() {
    let game = standing_at("0.9996", "0.00049");

    enter_brill(&game);

    assert_eq!(sent_spot(&game), Some(spot(TIRISFAL, 1000, 0)));
}

#[test]
fn with_no_map_the_place_goes_with_no_position() {
    let game = Game::new();

    enter_brill(&game);

    assert_eq!(sent_spot(&game), None);
}

#[test]
fn a_hidden_position_goes_as_no_position() {
    let game = standing_at("0.5", "0.25");
    game.run("wow.secrets[0.25] = true");

    enter_brill(&game);

    assert_eq!(sent_spot(&game), None);
}

#[test]
fn a_hidden_map_goes_as_no_position() {
    let game = standing_at("0.5", "0.25");
    game.run(&format!("wow.secrets[{TIRISFAL}] = true"));

    enter_brill(&game);

    assert_eq!(sent_spot(&game), None);
}

/// The game gives 0, 0 where it knows no position.
#[test]
fn the_corner_of_the_map_goes_as_no_position() {
    let game = standing_at("0", "0");

    enter_brill(&game);

    assert_eq!(sent_spot(&game), None);
}

#[test]
fn a_position_off_the_map_goes_as_no_position() {
    let game = standing_at("1.2", "0.5");

    enter_brill(&game);

    assert_eq!(sent_spot(&game), None);
}

const BRILL_CELLAR: u32 = 2000;

/// A player in a cave of Tirisfal Glades: the best map is the cave, and the zone map holds
/// it. `zone_spot` is the position on the zone map, or `nil` when the zone knows none.
fn in_a_cave(zone_spot: &str) -> Game {
    let game = Game::new();
    game.run(&format!(
        "wow.maps[{TIRISFAL}] = {{ name = 'Tirisfal Glades', type = Enum.UIMapType.Zone, player = {zone_spot} }}
         wow.maps[{BRILL_CELLAR}] = {{ name = 'Brill Cellar', type = Enum.UIMapType.Micro, parent = {TIRISFAL}, player = {{ x = 0.1, y = 0.2 }} }}
         wow.playerMap = {BRILL_CELLAR}"
    ));
    game
}

#[test]
fn a_position_in_a_cave_goes_on_the_map_of_its_zone() {
    let game = in_a_cave("{ x = 0.6, y = 0.7 }");

    enter_brill(&game);

    assert_eq!(sent_spot(&game), Some(spot(TIRISFAL, 600, 700)));
}

#[test]
fn a_position_that_its_zone_does_not_know_stays_on_its_own_map() {
    let game = in_a_cave("nil");

    enter_brill(&game);

    assert_eq!(sent_spot(&game), Some(spot(BRILL_CELLAR, 100, 200)));
}
