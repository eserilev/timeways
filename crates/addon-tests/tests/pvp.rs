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

/// The player is in Warsong Gulch.
const ENDED: &str = "wow.zone, wow.instance = 'Warsong Gulch', 'pvp'";

/// Your faction won the match in Warsong Gulch.
const YOU_WON: &str = "wow.zone, wow.instance = 'Warsong Gulch', 'pvp'
     wow.battlefieldWinner = 1
     wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)";

/// A new game with the saved wins of the old one, `seconds` later, as a `/reload` gives.
fn reloaded(game: &Game, seconds: i64) -> Game {
    let wins: String = game.eval(
        "local out = {}
         for _, win in ipairs(ns.Saved().bgWins) do
             out[#out + 1] = string.format('{ zone = %q, won = %d }', win.zone, win.won)
         end
         return '{ ' .. table.concat(out, ', ') .. ' }'",
    );
    let after = Game::new();
    after.run(&format!(
        "ns.Saved().bgWins = {wins}
         wow.now = wow.now + {seconds}"
    ));
    after
}

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
             wow.now = wow.now + 900
             wow.battlefieldWinner = 1
             wow.Fire('UPDATE_BATTLEFIELD_STATUS', 1)"
        ),
    );

    let later = Input::BgWon {
        at: Tick(NOW.0 + 900),
        zone: "Warsong Gulch".to_string(),
    };
    assert_eq!(sent, [won("Warsong Gulch"), later]);
}

#[test]
fn a_reload_on_the_score_screen_sends_the_win_once() {
    let game = Game::new();
    game.run(YOU_WON);

    let after = reloaded(&game, 60);
    let sent = sent_after(&after, YOU_WON);

    assert!(sent.is_empty(), "{sent:?}");
}

#[test]
fn a_later_match_in_the_same_battleground_sends_its_win_after_a_reload() {
    let game = Game::new();
    game.run(YOU_WON);

    let after = reloaded(&game, 1200);
    let sent = sent_after(&after, YOU_WON);

    assert_eq!(sent.len(), 1, "{sent:?}");
}

#[test]
fn the_saved_wins_keep_only_the_newest_ten() {
    let game = Game::new();

    for _ in 0..12 {
        game.run(&format!("{YOU_WON}\nwow.now = wow.now + 1200"));
    }

    assert_eq!(game.eval::<u32>("return #ns.Saved().bgWins"), 10);
}

#[test]
fn broken_saved_wins_raise_no_error_and_the_win_goes_out() {
    let game = Game::new();
    game.run("ns.Saved().bgWins = { 5, { zone = 3 }, { zone = 'Warsong Gulch', won = 'x' } }");

    let sent = sent_after(&game, YOU_WON);

    assert_eq!(sent, [won("Warsong Gulch")]);
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
