//! Ratings of narrator text (GAMEPLAY.md 3.2.2): off by default, a like or a dislike of the
//! newest narrator line, and Like and Dislike on the pages of the Chronicle with a story.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::Input;
use timeways_story::ratings::{Rated, Rating};

const OFF: &str =
    "|cffc8a064Timeways|r: Ratings are off. Type /timeways ratings on to rate narrator lines.";

/// One closed chapter with a story, and the open chapter after it with none.
const JOURNAL: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"summary":"$N keeps an old oath.","chapters":["#,
    r#"{"number":1,"first":4,"state":"closed","began":1790000000,"title":"Westfall","#,
    r#""zones":["Westfall"],"people":[],"deeds":[],"left_out":0,"prose":"Westfall burned."},"#,
    r#"{"number":2,"first":40,"state":"open","began":1790000000,"title":"Duskwood","#,
    r#""zones":["Duskwood"],"people":[],"deeds":[],"left_out":0}]}"#,
);

fn slash(game: &Game, message: &str) {
    game.run(&format!("wow.Slash('/timeways', '{message}')"));
}

fn narrator_line(game: &Game) {
    game.reply(r#"{"type":"events_seen","narrator":"Westfall burned."}"#);
}

/// The ratings that went to the desktop, as the story program reads them.
fn ratings_sent(game: &Game) -> Vec<(Rated, Option<u64>, Rating)> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::LineRated {
                rated,
                first,
                rating,
                ..
            } => Some((rated, first, rating)),
            _ => None,
        })
        .collect()
}

/// The Chronicle, open on the page with this key.
fn book(game: &Game, key: &str) {
    game.run("wow.Slash('/journal', '')");
    game.reply(JOURNAL);
    game.run(&format!(
        "ns.JournalFrame.Open('chapters') ns.JournalFrame.Select({key})"
    ));
}

fn buttons(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, button in ipairs(ns.Journal.Page('chapters').buttons) do
             table.insert(out, button.label)
         end
         return out",
    )
}

fn press(game: &Game, label: &str) {
    game.run(&format!(
        "for _, button in ipairs(ns.Journal.Page('chapters').buttons) do
             if button.label == '{label}' then button.run() end
         end"
    ));
}

#[test]
fn ratings_are_off_by_default_and_like_says_how_to_turn_them_on() {
    let game = Game::new();
    narrator_line(&game);

    slash(&game, "like");

    assert!(game.printed().contains(&OFF.to_string()));
    assert!(ratings_sent(&game).is_empty());
}

#[test]
fn like_after_a_narrator_line_rates_the_newest_line() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game);

    slash(&game, "like");
    slash(&game, "dislike");

    assert_eq!(
        ratings_sent(&game),
        [
            (Rated::Narrator, None, Rating::Up),
            (Rated::Narrator, None, Rating::Down)
        ]
    );
}

#[test]
fn like_before_any_narrator_line_sends_nothing() {
    let game = Game::new();
    slash(&game, "ratings on");

    slash(&game, "like");

    assert!(
        game.printed()
            .contains(&"|cffc8a064Timeways|r: No narrator line to rate yet.".to_string())
    );
    assert!(ratings_sent(&game).is_empty());
}

#[test]
fn ratings_off_turns_them_off_again() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game);

    slash(&game, "ratings off");
    slash(&game, "like");

    assert!(ratings_sent(&game).is_empty());
}

/// Any addon can write the saved variables, so only `true` turns ratings on.
#[test]
fn a_setting_that_is_not_true_leaves_ratings_off() {
    let game = Game::new();
    game.run("TimewaysSettings = { ratings = 'yes' }");
    narrator_line(&game);

    slash(&game, "like");

    assert!(ratings_sent(&game).is_empty());
}

#[test]
fn the_chronicle_shows_like_and_dislike_only_while_ratings_are_on() {
    let game = Game::new();
    book(&game, "1");
    let off = buttons(&game);

    slash(&game, "ratings on");

    assert!(!off.contains(&"Like".to_string()), "{off:?}");
    let on = buttons(&game);
    assert!(on.contains(&"Like".to_string()), "{on:?}");
    assert!(on.contains(&"Dislike".to_string()), "{on:?}");
}

#[test]
fn a_chapter_with_no_story_shows_no_like() {
    let game = Game::new();
    slash(&game, "ratings on");

    book(&game, "2");

    assert!(!buttons(&game).contains(&"Like".to_string()));
}

#[test]
fn dislike_on_a_chapter_rates_it_by_its_first_event() {
    let game = Game::new();
    slash(&game, "ratings on");
    book(&game, "1");

    press(&game, "Dislike");

    assert_eq!(
        ratings_sent(&game),
        [(Rated::Chapter, Some(4), Rating::Down)]
    );
}

#[test]
fn like_on_the_title_page_rates_the_summary() {
    let game = Game::new();
    slash(&game, "ratings on");
    book(&game, "'title'");

    press(&game, "Like");

    assert_eq!(ratings_sent(&game), [(Rated::Summary, None, Rating::Up)]);
}
