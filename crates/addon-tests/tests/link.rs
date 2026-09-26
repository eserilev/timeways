//! The real seam: the shared `Messages.lua` of Gnomish Relay behind `ns.Link`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

fn logged_in() -> Game {
    let game = Game::with_transport();
    game.run("ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))");
    game
}

#[test]
fn a_question_goes_into_the_saved_outbox_and_starts_a_strip() {
    let game = logged_in();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }
         wow.Slash('/lore', 'any news?')
         for _, timer in ipairs(wow.after) do timer.callback() end",
    );

    let text: String = game.eval("TimewaysDB.sent[1].text");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        r#"{"name":"Ada","realm":"Stormrage","type":"character_entered"}"#
    );
    assert!(lines[1].contains(r#""type":"lore_asked""#), "{text}");
    assert!(game.eval::<u32>("wow.shots") >= 1);
}

#[test]
fn fits_leaves_room_for_the_flags_of_the_transport() {
    let game = logged_in();

    assert!(game.eval::<bool>("ns.Link.Fits(string.rep('a', 2000))"));
    assert!(!game.eval::<bool>("ns.Link.Fits(string.rep('a', 3000))"));
}

#[test]
fn a_done_reply_goes_to_the_handlers_of_its_json_lines() {
    let game = logged_in();

    game.run(
        r#"ns.Messages.OnReply({ id = "story" }, 1, "done", '{"type":"events_seen","companion":"Hello!"}')"#,
    );

    assert_eq!(game.printed(), ["|cff8fbfffSprocket|r: Hello!"]);
}

#[test]
fn an_error_reply_shows_as_plain_text() {
    let game = logged_in();

    game.run(r#"ns.Messages.OnReply({ id = "story" }, 1, "error", "Timeways story program not running.")"#);

    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Timeways story program not running."]
    );
}

#[test]
fn a_message_that_cannot_be_sent_tells_the_player() {
    let game = logged_in();

    game.run(r#"ns.Messages.OnGiveUp({ id = "story" }, 1, "Not sent. Send it again.")"#);

    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Not sent. Send it again."]
    );
}
