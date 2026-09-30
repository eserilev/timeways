//! Dungeons and raids: the zone input, then a mark that the zone is an instance
//! (GAMEPLAY.md 3.3 and 5.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::Input;
use timeways_story::places::InstanceKind;

const NOW: Tick = Tick(1_790_000_000);

fn enter(game: &Game, zone: &str, instance: &str) -> Vec<Input> {
    game.run(&format!(
        "wow.zone, wow.subzone, wow.instance = '{zone}', '', '{instance}'
         wow.Fire('ZONE_CHANGED_NEW_AREA')
         wow.RunTickers()"
    ));
    game.sent_inputs()
}

fn zone(name: &str) -> Input {
    Input::ZoneEntered {
        at: NOW,
        zone: name.to_string(),
        subzone: None,
        spot: None,
    }
}

#[test]
fn a_dungeon_sends_its_zone_and_then_its_kind() {
    let game = Game::new();

    let sent = enter(&game, "The Deadmines", "party");

    let entered = Input::InstanceEntered {
        at: NOW,
        zone: "The Deadmines".to_string(),
        kind: InstanceKind::Dungeon,
    };
    assert_eq!(sent, [zone("The Deadmines"), entered]);
}

#[test]
fn a_raid_sends_its_kind() {
    let game = Game::new();

    let sent = enter(&game, "Molten Core", "raid");

    assert!(
        matches!(
            sent.last(),
            Some(Input::InstanceEntered {
                kind: InstanceKind::Raid,
                ..
            })
        ),
        "{sent:?}"
    );
}

#[test]
fn a_battleground_or_the_open_world_sends_only_the_zone() {
    let game = Game::new();

    let sent = enter(&game, "Warsong Gulch", "pvp");
    let more = enter(&game, "Westfall", "none");

    assert_eq!(sent[..1], [zone("Warsong Gulch")]);
    assert_eq!(more, [zone("Warsong Gulch"), zone("Westfall")]);
}
