//! The addon with the real transport of Gnomish Relay, and no bridge: the desktop program
//! is closed. No game event is lost, and the player sees no error for a batch that they
//! never sent by hand.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

fn offline() -> Game {
    let game = Game::with_transport();
    game.run("ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))");
    game
}

/// Plays the game for this long with no bridge: the clock moves, and every timer runs.
fn wait(game: &Game, seconds: u32) {
    game.run(&format!(
        "for _ = 1, {seconds} do
             wow.now = wow.now + 1
             wow.RunTickers()
             local timers = wow.after
             wow.after = {{}}
             for _, timer in ipairs(timers) do timer.callback() end
         end"
    ));
}

fn enter_zone(game: &Game, zone: &str) {
    game.run(&format!(
        "ns.Outbox.Add(ns.Inputs.Zone(time(), '{zone}', nil))
         ns.Outbox.Flush()"
    ));
}

#[test]
fn an_event_goes_out_again_after_the_transport_gives_up_on_it() {
    let game = offline();
    enter_zone(&game, "Elwynn Forest");

    wait(&game, 600);

    let sent: Vec<String> = game.eval(
        "local out = {}
         for _, message in ipairs(TimewaysDB.sent) do table.insert(out, message.text) end
         return out",
    );
    assert!(sent.len() >= 2, "{sent:?}");
    assert!(sent.last().unwrap().contains("Elwynn Forest"), "{sent:?}");
}

#[test]
fn a_batch_of_events_that_the_bridge_never_took_shows_no_error() {
    let game = offline();
    enter_zone(&game, "Elwynn Forest");

    wait(&game, 600);

    assert_eq!(game.printed(), Vec::<String>::new());
}

#[test]
fn one_batch_of_events_at_a_time_is_on_its_way() {
    let game = offline();
    enter_zone(&game, "Elwynn Forest");

    enter_zone(&game, "Westfall");

    assert_eq!(game.eval::<u32>("#TimewaysDB.sent"), 1);
}

#[test]
fn a_done_reply_clears_the_events_of_its_batch() {
    let game = offline();
    enter_zone(&game, "Elwynn Forest");
    enter_zone(&game, "Westfall");

    game.run(r#"ns.Messages.OnReply({ id = "story" }, TimewaysDB.sent[1].id, "done", '{"type":"events_seen","narrator":null}')"#);
    game.run("ns.Outbox.Flush()");

    assert_eq!(game.eval::<u32>("ns.Outbox.Waiting()"), 0);
    assert_eq!(game.eval::<u32>("#TimewaysDB.sent"), 2);
}

#[test]
fn a_question_that_the_bridge_never_took_tells_the_player() {
    let game = offline();
    game.run("wow.Slash('/lore', 'any news?')");

    wait(&game, 600);

    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Not sent. Send it again."]
    );
}
