//! How much the NPCs that you dealt with trust you: a tooltip line and a chat line
//! (GAMEPLAY.md 3.5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

/// A journal whose only person is `name`, with this trust and these slaps.
fn journal_with(name: &str, trust: Option<i64>, slapped: Option<i64>) -> String {
    let person = serde_json::json!({
        "name": name,
        "first_met": 1_790_000_000,
        "trust": trust,
        "slapped": slapped,
    });
    serde_json::json!({ "type": "journal", "page": 0, "pages": 1, "people": [person] }).to_string()
}

fn tooltip(game: &Game, name: &str) -> Vec<String> {
    game.run(&format!("wow.units.mouseover = {{ name = '{name}' }}"));
    game.eval("return wow.ShowTooltip('mouseover')")
}

#[test]
fn the_tooltip_of_an_npc_you_dealt_with_shows_its_trust_and_slaps() {
    let game = Game::new();

    game.reply(&journal_with("Innkeeper Farley", Some(-20), Some(2)));

    assert_eq!(
        tooltip(&game, "Innkeeper Farley"),
        ["Timeways: Wary of you. Slapped 2 times."]
    );
}

#[test]
fn the_tooltip_of_an_npc_you_only_met_or_never_met_is_left_alone() {
    let game = Game::new();

    game.reply(&journal_with("Innkeeper Farley", None, None));

    assert!(tooltip(&game, "Innkeeper Farley").is_empty());
    assert!(tooltip(&game, "Hogger").is_empty());
}

#[test]
fn trust_shows_in_words_for_each_band() {
    let game = Game::new();
    let mut words = Vec::new();

    for trust in [60, 50, 49, 10, 9, -9, -10, -49, -50, -100] {
        game.reply(&journal_with("N", Some(trust), None));
        words.extend(tooltip(&game, "N"));
    }

    let expected = [
        "Trusts you.",
        "Trusts you.",
        "Likes you.",
        "Likes you.",
        "Neutral.",
        "Neutral.",
        "Wary of you.",
        "Wary of you.",
        "Distrusts you.",
        "Distrusts you.",
    ];
    assert_eq!(words, expected.map(|word| format!("Timeways: {word}")));
}

#[test]
fn a_change_of_feeling_shows_once_in_the_chat() {
    let game = Game::new();
    game.reply(&journal_with("Keeper Tessa", None, None));

    game.reply(&journal_with("Keeper Tessa", Some(10), None));
    game.reply(&journal_with("Keeper Tessa", Some(15), None));
    game.reply(&journal_with("Keeper Tessa", Some(-10), None));

    let printed = game.printed();
    assert_eq!(printed.len(), 2, "{printed:?}");
    assert!(printed[0].ends_with("Keeper Tessa now likes you."));
    assert!(printed[1].ends_with("Keeper Tessa is now wary of you."));
}

#[test]
fn the_first_journal_of_a_session_prints_no_change() {
    let game = Game::new();

    game.reply(&journal_with("Keeper Tessa", Some(80), None));

    assert!(game.printed().is_empty());
}

#[test]
fn a_talk_answer_asks_for_the_journal_so_the_trust_follows() {
    let game = Game::new();

    game.reply(r#"{"type":"talk_answer","npc":"Keeper Tessa","text":"Hello."}"#);
    game.run("wow.RunTickers()");

    let asked = game.sent_inputs().iter().any(|input| {
        matches!(
            input,
            timeways_story::input::Input::JournalAsked { page: 0, .. }
        )
    });
    assert!(asked, "{:?}", game.sent_inputs());
}

#[test]
fn a_broken_person_is_skipped_and_raises_no_error() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"people":[true,{"name":5,"trust":10},{"name":"Ada","trust":"x"}]}"#,
    );

    assert!(tooltip(&game, "Ada").is_empty());
}

/// In restricted content, the tooltip can give a hidden unit token.
#[test]
fn a_tooltip_with_a_hidden_unit_is_left_alone() {
    let game = Game::new();
    game.reply(&journal_with("Innkeeper Farley", Some(-20), None));
    game.run(
        "wow.units.nameplate7 = { name = 'Innkeeper Farley' }
         wow.secrets['nameplate7'] = true",
    );

    let lines: Vec<String> = game.eval("return wow.ShowTooltip('nameplate7')");

    assert!(lines.is_empty());
}
