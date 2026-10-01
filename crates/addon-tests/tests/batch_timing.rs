//! When a batch of game events goes out (GAMEPLAY.md 5.4). Each batch costs a screenshot,
//! so small events wait for the flush timer, and a big moment goes within seconds.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

/// Runs the one-shot timers of the addon, as the clock passes them.
fn run_timers(game: &Game) {
    game.run(
        "local timers = wow.after
         wow.after = {}
         for _, timer in ipairs(timers) do timer.callback() end",
    );
}

fn planned_timers(game: &Game) -> usize {
    game.eval("#wow.after")
}

#[test]
fn small_events_wait_for_the_flush_every_ten_minutes() {
    let game = Game::new();

    let seconds: u32 = game.eval(
        "for _, ticker in ipairs(wow.tickers) do
             if ticker.callback == ns.Outbox.Flush then return ticker.seconds end
         end",
    );

    assert_eq!(seconds, 600);
}

#[test]
fn a_sighting_plans_no_batch() {
    let game = Game::new();

    game.run("ns.Outbox.Add(ns.Inputs.NpcSeen(time(), 'Duskbat', 'hostile', 'Beast'))");

    assert_eq!(planned_timers(&game), 0);
}

#[test]
fn a_level_up_goes_out_within_seconds() {
    let game = Game::new();
    game.run("ns.Outbox.Add(ns.Inputs.Level(time(), 13))");
    assert!(game.sent().is_empty());

    run_timers(&game);

    assert_eq!(game.sent().len(), 1);
}

#[test]
fn a_burst_of_big_moments_goes_in_one_batch() {
    let game = Game::new();
    game.run(
        "ns.Outbox.Add(ns.Inputs.Level(time(), 13))
         ns.Outbox.Add(ns.Inputs.Zone(time(), 'Westfall', nil))
         ns.Outbox.Add(ns.Inputs.Died(time(), nil, nil, nil, 3))",
    );

    run_timers(&game);

    assert_eq!(game.sent().len(), 1);
    assert_eq!(game.sent_inputs().len(), 3);
}

#[test]
fn a_new_zone_goes_out_within_seconds_but_a_new_subzone_waits() {
    let game = Game::new();
    game.run("ns.Outbox.Add(ns.Inputs.Zone(time(), 'Elwynn Forest', 'Goldshire'))");
    run_timers(&game);

    game.run("ns.Outbox.Add(ns.Inputs.Zone(time(), 'Elwynn Forest', 'Northshire'))");

    assert_eq!(planned_timers(&game), 0);
}

#[test]
fn camping_sends_the_events_that_wait() {
    let game = Game::new();
    game.run("ns.Outbox.Add(ns.Inputs.Npc(time(), 'Innkeeper Farley'))");

    game.run("wow.Fire('PLAYER_CAMPING')");

    assert_eq!(game.sent().len(), 1);
}

#[test]
fn quitting_sends_the_events_that_wait() {
    let game = Game::new();
    game.run("ns.Outbox.Add(ns.Inputs.Npc(time(), 'Innkeeper Farley'))");

    game.run("wow.Fire('PLAYER_QUITING')");

    assert_eq!(game.sent().len(), 1);
}
