//! `/lore` and the lore book: the question as the heading, and the answer as the page
//! (GAMEPLAY.md 3.1).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::MessageId;
use timeways_story::lore::Answer;
use timeways_story::pack::{Origin, Passage};
use timeways_story::story::Output;

fn reply_line(text: Option<&str>, passages: &[&str]) -> String {
    let passages = passages
        .iter()
        .map(|text| Passage {
            text: (*text).to_string(),
            source: "https://example.test/tower".to_string(),
            links: Vec::new(),
            origin: Origin::Pack,
            about: None,
            depends_on: None,
        })
        .collect();
    let answer = Answer {
        text: text.map(str::to_string),
        passages,
    };
    serde_json::to_string(&Output::LoreAnswer {
        id: MessageId(1),
        answer,
        notice: None,
    })
    .unwrap()
}

/// The book as `[heading, page, counter]`, or nothing while it is closed.
fn book(game: &Game) -> Vec<String> {
    game.eval(
        "if not ns.LoreBook.IsShown() then return {} end
         local out = {}
         for _, widget in ipairs(wow.widgets) do
             local page = TimewaysLoreFrameScroll and TimewaysLoreFrameScroll:GetScrollChild()
             if widget.kind == 'FontString' and widget.parent == page then
                 table.insert(out, widget.text)
             end
         end
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.parent == TimewaysLoreFrame and (widget.text or ''):find(' of ') then
                 table.insert(out, widget.text)
             end
         end
         return out",
    )
}

fn ask(game: &Game, question: &str) {
    game.run(&format!("wow.Slash('/lore', '{question}')"));
}

fn click(game: &Game, label: &str) {
    game.run(&format!("wow.Button('{label}'):Click()"));
}

fn enabled(game: &Game, label: &str) -> bool {
    game.eval(&format!("wow.Button('{label}'):IsEnabled()"))
}

#[test]
fn a_question_opens_the_book_while_it_waits() {
    let game = Game::new();

    ask(&game, "why is this tower in ruins?");

    assert_eq!(
        book(&game),
        ["why is this tower in ruins?", "Asking...", "1 of 1"]
    );
}

#[test]
fn the_answer_fills_the_page_with_no_citations() {
    let game = Game::new();
    ask(&game, "why is this tower in ruins?");

    game.reply(&reply_line(
        Some("Goblins burned it [1], and the [2] rest fled [3]."),
        &["The tower fell."],
    ));

    assert_eq!(book(&game)[1], "Goblins burned it, and the rest fled.");
}

#[test]
fn an_answer_with_no_text_shows_the_passages_with_no_sources() {
    let game = Game::new();
    ask(&game, "why?");

    game.reply(&reply_line(None, &["The tower fell.", "The mages left."]));

    assert_eq!(book(&game)[1], "The tower fell.\n\nThe mages left.");
}

#[test]
fn an_answer_with_nothing_says_that_nobody_knows() {
    let game = Game::new();
    ask(&game, "why?");

    game.reply(&reply_line(None, &[]));

    assert_eq!(book(&game)[1], "Nobody here knows.");
}

#[test]
fn a_broken_passage_is_skipped_and_the_others_show() {
    let game = Game::new();
    ask(&game, "why?");

    game.reply(r#"{"type":"lore_answer","id":1,"passages":[1,{"text":"a","source":"https://b"}]}"#);

    assert_eq!(book(&game)[1], "a");
}

#[test]
fn an_answer_that_the_bridge_escaped_shows_as_it_is() {
    let game = Game::new();
    ask(&game, "why?");

    game.reply(&reply_line(
        Some("||cffff0000red||r ||Hitem:1||h[Fake]||h [1]"),
        &[],
    ));

    assert_eq!(book(&game)[1], "||cffff0000red||r ||Hitem:1||h[Fake]||h");
}

#[test]
fn an_answer_in_the_open_book_prints_nothing_in_the_chat() {
    let game = Game::new();
    ask(&game, "why?");

    game.reply(&reply_line(Some("Goblins."), &[]));

    assert!(game.printed().is_empty(), "{:?}", game.printed());
}

#[test]
fn an_answer_to_a_closed_book_says_so_in_one_chat_line() {
    let game = Game::new();
    ask(&game, "why?");
    game.run("TimewaysLoreFrame:Hide()");

    game.reply(&reply_line(Some("Goblins burned it."), &[]));

    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Your answer is ready. Type /lore to read it."]
    );
}

#[test]
fn lore_with_no_question_opens_the_book_on_the_newest_answer() {
    let game = Game::new();
    ask(&game, "why?");
    game.reply(&reply_line(Some("Goblins."), &[]));
    game.run("TimewaysLoreFrame:Hide()");

    ask(&game, "  ");

    assert_eq!(book(&game), ["why?", "Goblins.", "1 of 1"]);
}

#[test]
fn lore_with_no_question_and_no_answer_yet_says_how_to_ask() {
    let game = Game::new();

    ask(&game, "   ");

    assert!(game.printed()[0].contains("/lore why is this tower in ruins?"));
    assert!(book(&game).is_empty());
}

#[test]
fn answers_fill_the_questions_in_the_order_they_were_asked() {
    let game = Game::new();
    ask(&game, "who built it?");
    ask(&game, "who burned it?");

    game.reply(&reply_line(Some("The mages."), &[]));
    game.reply(&reply_line(Some("The goblins."), &[]));

    assert_eq!(book(&game), ["who burned it?", "The goblins.", "2 of 2"]);
    click(&game, "Previous");
    assert_eq!(book(&game), ["who built it?", "The mages.", "1 of 2"]);
}

#[test]
fn previous_and_next_stop_at_the_ends() {
    let game = Game::new();
    ask(&game, "one?");
    ask(&game, "two?");

    let at_newest = (enabled(&game, "Previous"), enabled(&game, "Next"));
    click(&game, "Previous");
    let at_oldest = (enabled(&game, "Previous"), enabled(&game, "Next"));

    assert_eq!(at_newest, (true, false));
    assert_eq!(at_oldest, (false, true));
}

#[test]
fn the_book_keeps_the_last_ten_questions() {
    let game = Game::new();

    for n in 1..=12 {
        ask(&game, &format!("question {n}?"));
    }

    assert_eq!(book(&game)[2], "10 of 10");
    let oldest: String = game.eval("ns.Lore.Entries()[1].question");
    assert_eq!(oldest, "question 3?");
}

/// The link takes the question, and the bridge answers with an error.
#[test]
fn a_question_that_fails_says_so_in_the_book() {
    let game = Game::new();
    game.run(
        "ns.Link.Send = function(text)
             table.insert(sent, text)
             answered = #sent
             return #sent
         end
         C_Timer.NewTicker(1, function()
             ns.Link.Receive(#sent, 'error', 'Timeways story program not running.')
         end)",
    );

    ask(&game, "why?");
    game.run("wow.RunTickers()");

    assert_eq!(book(&game)[1], "No answer came back. Try asking again.");
}

#[test]
fn an_answer_with_no_question_in_this_session_still_shows() {
    let game = Game::new();

    game.reply(&reply_line(Some("Goblins."), &[]));
    ask(&game, "");

    assert_eq!(book(&game), ["Your answer", "Goblins.", "1 of 1"]);
}

#[test]
fn escape_closes_the_book() {
    let game = Game::new();

    ask(&game, "why?");

    let special: Vec<String> = game.eval("UISpecialFrames");
    assert!(special.contains(&"TimewaysLoreFrame".to_string()));
}

#[test]
fn close_closes_the_book() {
    let game = Game::new();
    ask(&game, "why?");

    click(&game, "Close");

    assert!(book(&game).is_empty());
}

fn scroll_bar_shown(game: &Game) -> bool {
    game.eval("TimewaysLoreFrameScroll.ScrollBar:IsShown()")
}

fn page_height(game: &Game) -> f64 {
    game.eval("TimewaysLoreFrameScroll:GetScrollChild():GetHeight()")
}

#[test]
fn a_long_answer_grows_the_page_and_shows_the_scroll_bar() {
    let game = Game::new();
    ask(&game, "who built this tower?");

    game.reply(&reply_line(Some(&"The tower is old. ".repeat(200)), &[]));

    assert!(page_height(&game) > game.eval::<f64>("TimewaysLoreFrameScroll:GetHeight()"));
    assert!(scroll_bar_shown(&game));
}

#[test]
fn a_short_answer_fits_with_no_scroll_bar() {
    let game = Game::new();
    ask(&game, "who built this tower?");

    game.reply(&reply_line(Some("Nobody knows."), &[]));

    assert!(page_height(&game) < game.eval::<f64>("TimewaysLoreFrameScroll:GetHeight()"));
    assert!(!scroll_bar_shown(&game));
}
