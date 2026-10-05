//! Battleground wins, the rank in battle against players, and leaving an inn
//! (docs/plans/chapters.md 9): the lines that give a tale or a chapter its weight and its
//! breaks.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::character::Resting;
use timeways_story::input::Input;

const NOW: Tick = Tick(1_790_000_000);

/// The lines that one event sends, after the flush.
fn sent_after(game: &Game, lua: &str) -> Vec<Input> {
    game.run(&format!("{lua}\nwow.RunTickers()"));
    game.sent_inputs()
}

fn won(zone: &str) -> Input {
    Input::BgWon {
        at: NOW,
        zone: zone.to_string(),
    }
}

/// The player is in Warsong Gulch, and the match is over with this winner.
const ENDED: &str = "wow.zone, wow.instance = 'Warsong Gulch', 'pvp'";

#[test]
fn a_battle_that_your_faction_won_sends_one_win() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        &format!(
            "{ENDED}
             wow.battlefieldWinner = 1
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)"
        ),
    );

    assert_eq!(sent, [won("Warsong Gulch")]);
}

#[test]
fn a_battle_that_the_other_faction_won_sends_nothing() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        &format!(
            "{ENDED}
             wow.battlefieldWinner = 0
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)"
        ),
    );

    assert!(sent.is_empty(), "{sent:?}");
}

#[test]
fn a_battle_that_still_runs_sends_nothing() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        &format!("{ENDED}\nwow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)"),
    );

    assert!(sent.is_empty(), "{sent:?}");
}

#[test]
fn the_next_match_that_you_win_sends_its_win_too() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        &format!(
            "{ENDED}
             wow.battlefieldWinner = 1
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)
             wow.battlefieldWinner = nil
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)
             wow.battlefieldWinner = 1
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)"
        ),
    );

    assert_eq!(sent, [won("Warsong Gulch"), won("Warsong Gulch")]);
}

#[test]
fn a_new_pvp_rank_is_sent_once() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        "wow.renown[2800] = 3
         wow.Fire('MAJOR_FACTION_RENOWN_LEVEL_CHANGED', 2800, 3, 2)
         wow.Fire('MAJOR_FACTION_RENOWN_LEVEL_CHANGED', 2800, 3, 2)",
    );

    assert_eq!(sent, [Input::PvpRank { at: NOW, rank: 3 }]);
}

#[test]
fn a_rank_that_the_client_does_not_know_yet_is_not_sent() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        "wow.Fire('MAJOR_FACTION_RENOWN_LEVEL_CHANGED', 2800, 1, 0)",
    );

    assert!(sent.is_empty(), "{sent:?}");
}

#[test]
fn leaving_an_inn_sends_the_change_of_rest() {
    let game = Game::new();

    let sent = sent_after(
        &game,
        "wow.resting = true
         wow.Fire('PLAYER_UPDATE_RESTING')
         wow.resting = false
         wow.Fire('PLAYER_UPDATE_RESTING')
         wow.Fire('PLAYER_UPDATE_RESTING')",
    );

    let rest = |resting| Input::RestChanged { at: NOW, resting };
    assert_eq!(sent, [rest(Resting::Yes), rest(Resting::No)]);
}
