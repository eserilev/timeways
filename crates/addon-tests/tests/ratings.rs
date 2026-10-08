//! Ratings of narrator text (GAMEPLAY.md 3.2.2): off by default, a [Rate] link at the end
//! of each narrator line, and two thumbs on the pages of the Chronicle with a story.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::Input;
use timeways_story::ratings::{Rated, Rating, Reason};

/// One closed chapter with a story, and the open chapter after it with none.
const JOURNAL: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"summary":"$N keeps an old oath.","chapters":["#,
    r#"{"number":1,"first":4,"state":"closed","began":1790000000,"title":"Westfall","#,
    r#""zones":["Westfall"],"people":[],"deeds":[],"left_out":0,"prose":"Westfall burned."},"#,
    r#"{"number":2,"first":40,"state":"open","began":1790000000,"title":"Duskwood","#,
    r#""zones":["Duskwood"],"people":[],"deeds":[],"left_out":0}]}"#,
);

/// The same journal after a reload, with the player's like of chapter 1.
const LIKED_JOURNAL: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"summary":"$N keeps an old oath.","chapters":["#,
    r#"{"number":1,"first":4,"state":"closed","began":1790000000,"title":"Westfall","#,
    r#""zones":["Westfall"],"people":[],"deeds":[],"left_out":0,"prose":"Westfall burned.","#,
    r#""rating":"up"}]}"#,
);

const RATE_SEVEN: &str = "|cff808080|Haddon:timeways:rate:7|h[Rate]|h|r";

fn slash(game: &Game, message: &str) {
    game.run(&format!("wow.Slash('/timeways', '{message}')"));
}

fn narrator_line(game: &Game, text: &str, id: u64) {
    game.reply(&format!(
        r#"{{"type":"events_seen","narrator":"{text}","narrator_id":{id}}}"#
    ));
}

fn click_link(game: &Game, id: u64) {
    game.run(&format!("wow.ClickChatLink('addon:timeways:rate:{id}')"));
}

fn choose(game: &Game, path: &[&str]) {
    let quoted: Vec<String> = path.iter().map(|text| format!("'{text}'")).collect();
    game.run(&format!("wow.Choose({})", quoted.join(", ")));
}

fn menu(game: &Game, path: &[&str]) -> Vec<String> {
    let quoted: Vec<String> = path.iter().map(|text| format!("'{text}'")).collect();
    game.eval(&format!("return wow.MenuTexts({})", quoted.join(", ")))
}

/// What each rating that went to the desktop names, as the story program reads it.
type Sent = (Rated, Option<u64>, Option<u64>, Rating, Option<Reason>);

fn ratings_sent(game: &Game) -> Vec<Sent> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::LineRated {
                rated,
                first,
                line,
                rating,
                reason,
                ..
            } => Some((rated, first, line, rating, reason)),
            _ => None,
        })
        .collect()
}

fn narrator_lines(game: &Game) -> Vec<String> {
    game.printed()
        .into_iter()
        .filter(|line| line.contains("Narrator"))
        .collect()
}

/// The Chronicle, open on the page with this key, from the journal `journal`.
fn book_of(game: &Game, journal: &str, key: &str) {
    game.run("wow.Slash('/journal', '')");
    game.reply(journal);
    game.run(&format!(
        "ns.JournalFrame.Open('chapters') ns.JournalFrame.Select({key})"
    ));
}

fn book(game: &Game, key: &str) {
    book_of(game, JOURNAL, key);
}

fn has_thumbs(game: &Game) -> bool {
    game.eval("return ns.Journal.Page('chapters').thumbs ~= nil")
}

fn thumb_rating(game: &Game) -> Option<String> {
    game.eval("return ns.Journal.Page('chapters').thumbs.rating")
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

/// Whether each thumb of the footer shows filled: like first, then dislike.
fn filled_thumbs(game: &Game) -> Vec<bool> {
    game.eval(
        r#"local like, dislike
           for _, widget in ipairs(wow.widgets) do
               if widget.kind == "Texture" and widget.parent.shown then
                   if widget.file == "Interface\\RaidFrame\\ReadyCheck-Ready" then
                       like = not widget.desaturated
                   elseif widget.file == "Interface\\RaidFrame\\ReadyCheck-NotReady" then
                       dislike = not widget.desaturated
                   end
               end
           end
           return { like == true, dislike == true }"#,
    )
}

#[test]
fn ratings_are_off_by_default_and_a_narrator_line_has_no_rate_link() {
    let game = Game::new();

    narrator_line(&game, "Westfall burned.", 7);

    assert_eq!(
        narrator_lines(&game),
        ["|cffe6cc80Narrator|r: Westfall burned."]
    );
}

#[test]
fn a_narrator_line_ends_in_a_grey_rate_link_while_ratings_are_on() {
    let game = Game::new();
    slash(&game, "ratings on");

    narrator_line(&game, "Westfall burned.", 7);

    assert_eq!(
        narrator_lines(&game),
        [format!(
            "|cffe6cc80Narrator|r: Westfall burned. {RATE_SEVEN}"
        )]
    );
}

#[test]
fn a_narrator_line_with_no_id_has_no_rate_link() {
    let game = Game::new();
    slash(&game, "ratings on");

    game.reply(r#"{"type":"events_seen","narrator":"Westfall burned."}"#);
    game.reply(r#"{"type":"events_seen","narrator":"Duskwood fell.","narrator_id":"7"}"#);

    let lines = narrator_lines(&game);
    assert_eq!(lines.len(), 2);
    assert!(
        lines.iter().all(|line| !line.contains("[Rate]")),
        "{lines:?}"
    );
}

#[test]
fn a_rate_link_names_its_own_line_when_two_lines_came_in_one_minute() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);
    narrator_line(&game, "Duskwood fell.", 9);

    click_link(&game, 7);
    choose(&game, &["Like"]);

    assert_eq!(
        ratings_sent(&game),
        [(Rated::Narrator, None, Some(7), Rating::Up, None)]
    );
}

#[test]
fn the_menu_of_a_rate_link_offers_like_and_dislike_with_the_reasons() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);

    click_link(&game, 7);

    assert_eq!(menu(&game, &[]), ["Like", "Dislike"]);
    assert_eq!(
        menu(&game, &["Dislike"]),
        [
            "Wrong lore",
            "Made-up name",
            "Boring",
            "Too long",
            "Doesn't fit the moment",
            "Other"
        ]
    );
}

#[test]
fn a_dislike_with_a_reason_sends_the_reason() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);

    click_link(&game, 7);
    choose(&game, &["Dislike", "Made-up name"]);

    assert_eq!(
        ratings_sent(&game),
        [(
            Rated::Narrator,
            None,
            Some(7),
            Rating::Down,
            Some(Reason::MadeUpName)
        )]
    );
}

#[test]
fn the_rate_link_changes_in_place_and_the_chat_gets_no_new_line() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);
    narrator_line(&game, "Duskwood fell.", 9);
    let printed = game.printed().len();

    click_link(&game, 7);
    choose(&game, &["Like"]);

    assert_eq!(game.printed().len(), printed);
    let lines = narrator_lines(&game);
    assert_eq!(
        lines[0],
        "|cffe6cc80Narrator|r: Westfall burned. |cff808080Liked|r"
    );
    assert!(
        lines[1].contains("|Haddon:timeways:rate:9|h[Rate]|h"),
        "{lines:?}"
    );
}

#[test]
fn a_dislike_shows_its_reason_in_place() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);

    click_link(&game, 7);
    choose(&game, &["Dislike", "Boring"]);

    assert_eq!(
        narrator_lines(&game),
        ["|cffe6cc80Narrator|r: Westfall burned. |cff808080Disliked: Boring|r"]
    );
}

#[test]
fn a_rate_link_opens_no_menu_after_ratings_go_off() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);
    slash(&game, "ratings off");

    click_link(&game, 7);

    assert!(menu(&game, &[]).is_empty());
    assert!(ratings_sent(&game).is_empty());
}

#[test]
fn another_addon_link_opens_no_menu() {
    let game = Game::new();
    slash(&game, "ratings on");

    game.run("wow.ClickChatLink('addon:timeways:rate:x')");
    game.run("wow.ClickChatLink('addon:other:rate:7')");

    assert!(menu(&game, &[]).is_empty());
}

#[test]
fn like_and_dislike_are_no_commands_any_more() {
    let game = Game::new();
    slash(&game, "ratings on");
    narrator_line(&game, "Westfall burned.", 7);

    let handled: Vec<bool> =
        game.eval("return { ns.Ratings.Command('like'), ns.Ratings.Command('dislike') }");

    assert_eq!(handled, [false, false]);
    assert!(ratings_sent(&game).is_empty());
}

#[test]
fn ratings_on_says_how_to_rate() {
    let game = Game::new();

    slash(&game, "ratings on");

    assert!(game.printed().contains(
        &"|cffc8a064Timeways|r: Ratings are on. Click [Rate] after a narrator line. Ratings stay on your computer."
            .to_string()
    ));
}

/// Any addon can write the saved variables, so only `true` turns ratings on.
#[test]
fn a_setting_that_is_not_true_leaves_ratings_off() {
    let game = Game::new();
    game.run("TimewaysSettings = { ratings = 'yes' }");

    narrator_line(&game, "Westfall burned.", 7);
    click_link(&game, 7);

    assert!(
        narrator_lines(&game)
            .iter()
            .all(|line| !line.contains("[Rate]"))
    );
    assert!(menu(&game, &[]).is_empty());
}

#[test]
fn the_chronicle_shows_thumbs_only_while_ratings_are_on() {
    let game = Game::new();
    book(&game, "1");
    let off = has_thumbs(&game);

    slash(&game, "ratings on");

    assert!(!off);
    assert!(has_thumbs(&game));
}

#[test]
fn the_footer_has_no_like_or_dislike_button() {
    let game = Game::new();
    slash(&game, "ratings on");

    book(&game, "1");

    let labels = buttons(&game);
    assert!(!labels.contains(&"Like".to_string()), "{labels:?}");
    assert!(!labels.contains(&"Dislike".to_string()), "{labels:?}");
}

#[test]
fn a_chapter_with_no_story_shows_no_thumbs() {
    let game = Game::new();
    slash(&game, "ratings on");

    book(&game, "2");

    assert!(!has_thumbs(&game));
}

#[test]
fn the_dislike_thumb_offers_the_reasons_and_rates_the_chapter_by_its_first_event() {
    let game = Game::new();
    slash(&game, "ratings on");
    book(&game, "1");

    game.run("ns.Journal.Page('chapters').thumbs.dislike(UIParent)");
    choose(&game, &["Too long"]);

    assert_eq!(
        ratings_sent(&game),
        [(
            Rated::Chapter,
            Some(4),
            None,
            Rating::Down,
            Some(Reason::TooLong)
        )]
    );
}

#[test]
fn the_like_thumb_on_the_title_page_rates_the_summary() {
    let game = Game::new();
    slash(&game, "ratings on");
    book(&game, "'title'");

    game.run("ns.Journal.Page('chapters').thumbs.like()");

    assert_eq!(
        ratings_sent(&game),
        [(Rated::Summary, None, None, Rating::Up, None)]
    );
}

#[test]
fn the_thumbs_show_the_rating_of_the_journal_after_a_reload() {
    let game = Game::new();
    slash(&game, "ratings on");

    book_of(&game, LIKED_JOURNAL, "1");

    assert_eq!(thumb_rating(&game).as_deref(), Some("up"));
    assert_eq!(filled_thumbs(&game), [true, false]);
}

#[test]
fn a_click_on_a_thumb_fills_it_at_once() {
    let game = Game::new();
    slash(&game, "ratings on");
    book(&game, "1");
    let before = filled_thumbs(&game);

    game.run("ns.Journal.Page('chapters').thumbs.like()");

    assert_eq!(before, [false, false]);
    assert_eq!(filled_thumbs(&game), [true, false]);
}
