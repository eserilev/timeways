//! On WoW Forever a character has a first name and, often, a surname (GAMEPLAY.md 5.11).
//! Timeways shows the whole name, and no part of a name reaches a model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use proptest::prelude::*;

/// Kobee Bean on WoW Forever, with Leborc James in the group.
fn kobee_bean() -> Game {
    let game = Game::new();
    game.run(
        "wow.regional = true
         wow.units.player = { name = 'Kobee', realm = 'Bean', player = true, guid = 'Player-1-Kobee', level = 12 }
         wow.units.party1 = { name = 'Leborc', realm = 'James', player = true, guid = 'Player-1-Leborc' }",
    );
    game
}

fn marked(game: &Game, text: &str) -> String {
    let mark: mlua::Function =
        game.eval("return function(text) return (ns.TaskNames.Marked(text)) end");
    mark.call(text).unwrap()
}

#[test]
fn your_name_is_your_first_name_and_surname() {
    let game = kobee_bean();

    assert_eq!(game.eval::<String>("ns.PlayerNames.Mine()"), "Kobee Bean");
}

#[test]
fn your_world_is_named_by_your_whole_name() {
    let game = kobee_bean();

    game.run("ns.Watch.Login() ns.Outbox.Flush()");

    let sent = game.sent().join("\n");
    assert!(sent.contains(r#""name":"Kobee Bean""#), "{sent}");
}

#[test]
fn a_character_with_no_surname_shows_the_first_name_alone() {
    let game = kobee_bean();
    game.run("wow.units.player.realm = nil");

    assert_eq!(game.eval::<String>("ns.PlayerNames.Mine()"), "Kobee");
}

#[test]
fn a_line_from_the_desktop_shows_your_full_name_for_the_hero_mark() {
    let game = kobee_bean();

    let line: String = game.eval("ns.WithName('$N stands at the gate.')");

    assert_eq!(line, "Kobee Bean stands at the gate.");
}

#[test]
fn the_roleplay_profile_names_you_with_your_surname() {
    let game = kobee_bean();

    assert_eq!(
        game.eval::<String>("ns.MspProfile.Fields().NA"),
        "Kobee Bean"
    );
}

#[test]
fn another_player_shows_with_a_surname() {
    let game = kobee_bean();

    assert_eq!(
        game.eval::<String>("ns.PlayerNames.Shown('Leborc-James')"),
        "Leborc James"
    );
}

#[test]
fn a_name_from_chat_with_a_space_becomes_the_key_of_the_game() {
    let game = kobee_bean();

    assert_eq!(
        game.eval::<String>("ns.TaskPeople.Full('Leborc James')"),
        "Leborc-James"
    );
}

#[test]
fn outside_forever_a_player_of_your_realm_shows_without_the_realm() {
    let game = Game::new();

    assert_eq!(
        game.eval::<String>("ns.PlayerNames.Shown('Bram-' .. GetNormalizedRealmName())"),
        "Bram"
    );
}

#[test]
fn your_typed_full_name_becomes_the_hero_mark_with_no_surname_left() {
    let game = kobee_bean();

    assert_eq!(marked(&game, "help Kobee Bean now"), "help $N now");
}

#[test]
fn a_typed_full_name_of_a_player_of_your_group_keeps_no_surname() {
    let game = kobee_bean();

    assert_eq!(
        marked(&game, "ask Leborc James, then leborc."),
        "ask {Leborc}, then {Leborc}."
    );
}

#[test]
fn npc_text_with_your_full_name_keeps_no_part_of_it() {
    let game = kobee_bean();

    game.run(
        "ns.Seen.Read('gossip', nil, 'Innkeeper Renee', 'Welcome, Kobee Bean. Sit, Kobee.')
         ns.Outbox.Flush()",
    );

    let sent = game.sent().join("\n");
    assert!(sent.contains("Welcome, $N. Sit, $N."), "{sent}");
    assert!(!sent.contains("Bean"), "{sent}");
}

proptest! {
    /// Any first name and surname, in any case, anywhere in a line: no part of your own name
    /// reaches a model (GAMEPLAY.md 5.11).
    #[test]
    fn no_part_of_your_full_name_survives_the_mark(
        first in "[A-Z][a-z]{2,9}",
        surname in "[A-Z][a-z]{2,9}",
        before in "[a-z ,.!]{0,12}",
        after in "[a-z ,.!]{0,12}",
        upper in any::<bool>(),
    ) {
        let (low_first, low_surname) = (first.to_lowercase(), surname.to_lowercase());
        prop_assume!(low_first != low_surname);
        let around = format!("{before} {after}");
        prop_assume!(!around.contains(&low_first) && !around.contains(&low_surname));
        let game = Game::new();
        game.run(&format!(
            "wow.regional = true
             wow.units.player = {{ name = '{first}', realm = '{surname}', player = true, guid = 'Player-1-X', level = 1 }}"
        ));
        let typed = if upper { format!("{first} {surname}").to_uppercase() } else { format!("{first} {surname}") };

        let text = marked(&game, &format!("{before} {typed} {after}"));

        let lower = text.to_lowercase();
        prop_assert!(!lower.contains(&low_first), "{text}");
        prop_assert!(!lower.contains(&low_surname), "{text}");
    }
}
