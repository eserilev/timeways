//! The quests of the game that you take and turn in (GAMEPLAY.md 5.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{GameQuestKind, Input};

const NOW: Tick = Tick(1_790_000_000);

/// A quest log with a zone header and a class header, as the client shows it.
fn with_quest_log() -> Game {
    let game = Game::new();
    game.run(
        "wow.questLog = {
             { title = 'Tirisfal Glades', isHeader = true },
             { title = 'Rattling the Rattlecages', questID = 1, isHeader = false },
             { title = 'Paladin', isHeader = true },
             { title = 'Rediscovering the Light', questID = 2, isHeader = false },
         }",
    );
    game
}

fn taken(title: &str, kind: GameQuestKind) -> Input {
    Input::GameQuestAccepted {
        at: NOW,
        title: title.to_string(),
        kind,
    }
}

fn done(title: &str, kind: GameQuestKind) -> Input {
    Input::GameQuestDone {
        at: NOW,
        title: title.to_string(),
        kind,
    }
}

#[test]
fn taking_a_quest_sends_its_title_and_kind() {
    let game = with_quest_log();

    game.run("wow.Fire('QUEST_ACCEPTED', 1); wow.RunTickers()");

    assert_eq!(
        game.sent_inputs(),
        [taken("Rattling the Rattlecages", GameQuestKind::Normal)]
    );
}

#[test]
fn a_quest_under_the_header_of_your_class_is_a_class_quest() {
    let game = with_quest_log();

    game.run("wow.Fire('QUEST_ACCEPTED', 2); wow.RunTickers()");

    assert_eq!(
        game.sent_inputs(),
        [taken("Rediscovering the Light", GameQuestKind::Class)]
    );
}

#[test]
fn the_header_of_another_class_makes_no_class_quest() {
    let game = with_quest_log();

    game.run("wow.class = 'Priest'; wow.Fire('QUEST_ACCEPTED', 2); wow.RunTickers()");

    assert_eq!(
        game.sent_inputs(),
        [taken("Rediscovering the Light", GameQuestKind::Normal)]
    );
}

#[test]
fn a_turn_in_after_the_quest_left_the_log_still_sends_it_once() {
    let game = with_quest_log();
    game.run("wow.Fire('QUEST_ACCEPTED', 2)");

    game.run(
        "wow.questLog = {}
         wow.Fire('QUEST_TURNED_IN', 2, 450, 0)
         wow.Fire('QUEST_TURNED_IN', 2, 450, 0)
         wow.RunTickers()",
    );

    assert_eq!(
        game.sent_inputs(),
        [
            taken("Rediscovering the Light", GameQuestKind::Class),
            done("Rediscovering the Light", GameQuestKind::Class),
        ]
    );
}

#[test]
fn a_quest_from_before_the_login_is_known_after_the_scan_at_login() {
    let game = with_quest_log();

    game.run(
        "wow.Fire('PLAYER_ENTERING_WORLD')
         wow.Fire('QUEST_TURNED_IN', 1, 100, 0)
         wow.RunTickers()",
    );

    assert!(
        game.sent_inputs()
            .contains(&done("Rattling the Rattlecages", GameQuestKind::Normal)),
        "{:?}",
        game.sent_inputs()
    );
}

#[test]
fn a_turn_in_of_a_quest_that_the_log_never_showed_sends_nothing() {
    let game = with_quest_log();

    game.run("wow.Fire('QUEST_TURNED_IN', 99, 0, 0); wow.RunTickers()");

    assert_eq!(game.sent_inputs(), []);
}

#[test]
fn a_hidden_title_is_never_sent_and_a_hidden_class_makes_no_class_quest() {
    let game = with_quest_log();

    game.run(
        "wow.secrets['Rattling the Rattlecages'] = true
         wow.secrets['Paladin'] = true
         wow.Fire('QUEST_ACCEPTED', 1)
         wow.Fire('QUEST_ACCEPTED', 2)
         wow.RunTickers()",
    );

    assert_eq!(
        game.sent_inputs(),
        [taken("Rediscovering the Light", GameQuestKind::Normal)]
    );
}

#[test]
fn a_broken_entry_of_the_quest_log_is_skipped() {
    let game = Game::new();
    game.run(
        "wow.questLog = { 5, { title = 7, questID = 1 }, { title = 'Real', questID = 'x' },
                          { title = 'Real', questID = 3, isHeader = false } }",
    );

    game.run("wow.Fire('QUEST_ACCEPTED', 3); wow.RunTickers()");

    assert_eq!(game.sent_inputs(), [taken("Real", GameQuestKind::Normal)]);
}
