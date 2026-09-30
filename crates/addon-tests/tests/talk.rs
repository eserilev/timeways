#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};

#[test]
fn talk_sends_the_words_to_the_npc_that_you_target() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', '  any news?  ')",
    );

    let talk = Input::TalkAsked {
        id: MessageId(1),
        at: Tick(1_790_000_000),
        npc: "Innkeeper Farley".to_string(),
        text: "any news?".to_string(),
    };
    assert_eq!(game.sent_inputs(), [talk]);
}

#[test]
fn talk_to_a_player_or_a_pet_is_refused_and_sends_nothing() {
    let game = Game::new();

    game.run("wow.units.target = { name = 'Grimtusk', player = true }; wow.Slash('/talk', 'hi')");
    game.run("wow.units.target = { name = 'Fluffy', controlled = true }; wow.Slash('/talk', 'hi')");

    assert!(game.sent().is_empty());
    assert!(
        game.printed()
            .iter()
            .all(|line| line.contains("Target someone to talk to first."))
    );
}

#[test]
fn talk_with_no_words_shows_help() {
    let game = Game::new();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', ' ')");

    assert!(game.sent().is_empty());
    assert!(game.printed()[0].contains("/talk hello"));
}

#[test]
fn the_npc_says_its_answer_in_the_chat() {
    let game = Game::new();

    game.reply(r#"{"type":"talk_answer","id":1,"npc":"Innkeeper Farley","text":"Nothing but rain. ||Hx||h"}"#);

    assert_eq!(
        game.printed(),
        ["|cffffd100Innkeeper Farley says:|r Nothing but rain. ||Hx||h"]
    );
}

#[test]
fn with_no_model_the_npc_says_nothing() {
    let game = Game::new();

    game.reply(r#"{"type":"talk_answer","id":1,"npc":"Innkeeper Farley","text":null}"#);

    assert_eq!(
        game.printed(),
        ["|cffffd100Innkeeper Farley looks at you and says nothing.|r"]
    );
}

#[test]
fn a_question_never_takes_a_pet_as_its_target() {
    let game = Game::new();

    game.run("wow.units.target = { name = 'Fluffy', controlled = true }; wow.Slash('/lore', 'who is this?')");

    assert!(!game.sent().concat().contains("Fluffy"));
}

#[test]
fn a_target_that_you_can_attack_does_not_talk() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Duskbat', hostile = true }
         wow.Slash('/talk', 'hello')",
    );

    assert!(game.sent().is_empty());
    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Duskbat won't talk to you."]
    );
}
