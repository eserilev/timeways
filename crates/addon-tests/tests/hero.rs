#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};

const NOW: Tick = Tick(1_790_000_000);

/// A journal page with this hero and nothing else.
fn hero_reply(hero: &str, refused: &str) -> String {
    format!(r#"{{"type":"journal","page":0,"pages":1,"hero":{hero},"hero_refused":{refused}}}"#)
}

const FILLED: &str = r#"{"sheet":[{"field":"goal","text":"Find my brother."}],"entries":[{"number":4,"at":1790000000,"text":"An oath.","place":"Goldshire","npc":"Innkeeper Farley"}]}"#;

fn lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('hero')) do
             local action = line.action and (' [' .. line.action.label .. ']') or ''
             table.insert(out, line.style .. ': ' .. line.text .. action)
         end
         return out",
    )
}

/// Runs the button of the line with this text.
fn click(game: &Game, text: &str) {
    game.run(&format!(
        "for _, line in ipairs(ns.Journal.Lines('hero')) do
             if line.text == '{text}' then line.action.run() end
         end"
    ));
}

fn journal_asked() -> Input {
    Input::JournalAsked {
        id: MessageId(1),
        page: 0,
    }
}

#[test]
fn the_hero_page_shows_each_field_with_its_button_and_each_entry() {
    let game = Game::new();

    game.reply(&hero_reply(FILLED, "null"));

    let lines = lines(&game);
    assert_eq!(lines[0], "heading: Who you are");
    assert!(lines.contains(&"entry: Goal [Edit]".to_string()));
    assert!(lines.contains(&"text: Find my brother.".to_string()));
    assert!(lines.contains(&"note: Where your hero comes from.".to_string()));
    let own = lines
        .iter()
        .position(|line| line == "heading: Your own lore [Add]")
        .unwrap();
    assert_eq!(lines[own + 1], "entry: An oath. [Remove]");
    assert!(
        lines[own + 2].starts_with("text: About Innkeeper Farley. Goldshire, "),
        "{}",
        lines[own + 2]
    );
}

#[test]
fn an_edit_sends_the_field_and_asks_for_the_journal_in_one_batch() {
    let game = Game::new();
    game.reply(&hero_reply(FILLED, "null"));

    click(&game, "Goal");
    game.run("wow.SavePopup('  Avenge my brother.  ')");

    let set = Input::HeroSet {
        at: NOW,
        field: "goal".to_string(),
        text: "Avenge my brother.".to_string(),
    };
    assert_eq!(game.sent().len(), 1);
    assert_eq!(game.sent_inputs(), [set, journal_asked()]);
}

#[test]
fn the_edit_box_starts_with_the_text_of_the_field() {
    let game = Game::new();
    game.reply(&hero_reply(FILLED, "null"));

    click(&game, "Goal");

    assert_eq!(
        game.eval::<String>("wow.popups[1].data.text"),
        "Find my brother."
    );
    assert_eq!(
        game.eval::<String>("wow.popups[1].which"),
        "TIMEWAYS_HERO_TEXT"
    );
}

#[test]
fn add_writes_an_entry_and_an_empty_text_writes_nothing() {
    let game = Game::new();
    game.reply(&hero_reply(FILLED, "null"));

    click(&game, "Your own lore");
    game.run("wow.SavePopup('   ')");
    click(&game, "Your own lore");
    game.run("wow.SavePopup('A stranger knew my name.')");

    let added = Input::HeroAdded {
        at: NOW,
        text: "A stranger knew my name.".to_string(),
        npc: None,
    };
    assert_eq!(game.sent_inputs(), [added, journal_asked()]);
}

#[test]
fn remove_asks_first_and_then_removes_the_entry() {
    let game = Game::new();
    game.reply(&hero_reply(FILLED, "null"));

    click(&game, "An oath.");
    let asked_first = game.sent().is_empty();
    game.run("StaticPopupDialogs[wow.popups[1].which].OnAccept(nil, wow.popups[1].data)");

    assert!(asked_first);
    assert_eq!(
        game.sent_inputs(),
        [Input::HeroRemoved { at: NOW, number: 4 }, journal_asked()]
    );
}

#[test]
fn a_refused_edit_tells_the_player_why() {
    let game = Game::new();

    game.reply(&hero_reply(
        r#"{"sheet":[],"entries":[]}"#,
        r#""Not saved: too long.""#,
    ));

    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Not saved: too long."]
    );
}

#[test]
fn hero_commands_set_add_and_note() {
    let game = Game::new();

    game.run("wow.Slash('/hero', 'set goal Find my brother.')");
    game.run("wow.Slash('/hero', 'add I swore an oath.')");
    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/hero', 'note He knew my father.')");

    let inputs = game.sent_inputs();
    assert_eq!(
        inputs[0],
        Input::HeroSet {
            at: NOW,
            field: "goal".to_string(),
            text: "Find my brother.".to_string()
        }
    );
    assert_eq!(
        inputs[2],
        Input::HeroAdded {
            at: NOW,
            text: "I swore an oath.".to_string(),
            npc: None
        }
    );
    let note = Input::HeroAdded {
        at: NOW,
        text: "He knew my father.".to_string(),
        npc: Some("Innkeeper Farley".to_string()),
    };
    assert_eq!(inputs[4], note);
}

#[test]
fn a_note_needs_an_npc_target_and_a_field_must_exist() {
    let game = Game::new();

    game.run("wow.Slash('/hero', 'note He knew my father.')");
    game.run(
        "wow.units.target = { name = 'Grimtusk', player = true }; wow.Slash('/hero', 'note Hm.')",
    );
    game.run("wow.Slash('/hero', 'set wealth Much.')");

    assert!(game.sent().is_empty());
    let printed = game.printed();
    assert!(
        printed[0].contains("Target someone first") && printed[1].contains("Target someone first")
    );
    assert!(printed[2].contains("origin, background, goal, bond, flaw, traits"));
}

#[test]
fn hero_alone_opens_the_hero_page() {
    let game = Game::new();

    game.run("wow.Slash('/hero', '')");

    assert_eq!(game.eval::<String>("ns.JournalFrame.Section()"), "hero");
}

#[test]
fn an_empty_hero_opens_the_hero_page_once_and_a_written_one_does_not() {
    let empty = Game::new();
    empty.run("wow.Slash('/journal', '')");
    empty.reply(&hero_reply(r#"{"sheet":[],"entries":[]}"#, "null"));
    empty.run("ns.JournalFrame.Open('places')");
    empty.reply(&hero_reply(r#"{"sheet":[],"entries":[]}"#, "null"));

    let written = Game::new();
    written.run("wow.Slash('/journal', '')");
    written.reply(&hero_reply(FILLED, "null"));

    assert_eq!(empty.eval::<String>("ns.JournalFrame.Section()"), "places");
    assert_eq!(
        written.eval::<String>("ns.JournalFrame.Section()"),
        "chapters"
    );
}

#[test]
fn the_book_has_five_tabs_with_the_hero_first() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    let tabs: Vec<String> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.template == 'UIPanelButtonTemplate' and widget.parent == TimewaysJournalFrame then
                 table.insert(out, widget.text)
             end
         end
         return out",
    );
    assert_eq!(tabs, ["Hero", "Chronicle", "Places", "People", "Deeds"]);
}
