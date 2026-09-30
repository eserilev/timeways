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

/// The question of the goal, which opens the editor of the goal.
const GOAL: &str = "What does your character want?";

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

/// The request for the first page of the journal, in the message with this number.
fn journal_asked(message: u64) -> Input {
    Input::JournalAsked {
        id: MessageId(message),
        page: 0,
    }
}

/// The Hero page of the book, open on the goal, as the player sees it before an edit.
/// Opening the book sends the first message.
fn open_book(hero: &str) -> Game {
    let game = Game::new();
    game.run("wow.Slash('/hero', '')");
    game.reply(&hero_reply(hero, "null"));
    game.run("ns.JournalFrame.Select('goal')");
    game
}

/// Types this text in the editor and clicks Save.
fn write(game: &Game, text: &str) {
    game.run(&format!(
        "wow.EditBox():SetText('{text}'); wow.Button('Save'):Click()"
    ));
}

fn editor_text(game: &Game) -> String {
    game.eval("wow.EditBox():GetText()")
}

#[test]
fn the_hero_page_shows_the_open_question_with_its_button_and_each_entry() {
    let game = Game::new();

    game.reply(&hero_reply(FILLED, "null"));

    let lines = lines(&game);
    assert_eq!(
        lines[..3],
        [
            "note: Question 1 of 6",
            "heading: Where is your character from? [Edit]",
            "hint: Not answered yet."
        ]
    );
    let own = lines
        .iter()
        .position(|line| line == "heading: Your Notes [Add a note]")
        .unwrap();
    assert_eq!(lines[own + 1], "entry: An oath. [Remove]");
    assert!(
        lines[own + 2].starts_with("text: About Innkeeper Farley. Goldshire, "),
        "{}",
        lines[own + 2]
    );
}

/// The rows of the list of the Hero page, as `style: text (detail)`.
fn sheet(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('hero').list) do
             local detail = row.detail and (' (' .. row.detail .. ')') or ''
             table.insert(out, row.style .. ': ' .. row.text .. detail)
         end
         return out",
    )
}

#[test]
fn the_sheet_lists_each_question_with_its_answer() {
    let game = Game::new();

    game.reply(&hero_reply(FILLED, "null"));

    let rows = sheet(&game);
    assert_eq!(rows[0], "group: About your hero");
    assert_eq!(rows.len(), 2 + 6);
    assert_eq!(
        rows[2..5],
        [
            "item: Where is your character from? (Not answered yet.)",
            "item: What did your character do before adventuring? (Not answered yet.)",
            "item: What does your character want? (Find my brother.)",
        ]
    );
}

#[test]
fn a_question_of_the_sheet_opens_on_the_parchment() {
    let game = open_book(FILLED);

    let lines = lines(&game);

    assert_eq!(
        lines[..3],
        [
            "note: Question 3 of 6",
            "heading: What does your character want? [Edit]",
            "text: Find my brother."
        ]
    );
}

#[test]
fn next_and_previous_step_through_the_questions() {
    let game = open_book(FILLED);

    game.run("wow.Button('Next'):Click()");
    let after_next = lines(&game)[0].clone();
    game.run("wow.Button('Previous'):Click(); wow.Button('Previous'):Click()");

    assert_eq!(after_next, "note: Question 4 of 6");
    assert_eq!(lines(&game)[0], "note: Question 2 of 6");
}

#[test]
fn the_footer_counts_the_answered_questions() {
    let game = open_book(FILLED);

    let footer: String = game.eval("ns.Journal.Page('hero').footer");

    assert_eq!(footer, "1 of 6 answered");
}

#[test]
fn an_edit_sends_the_field_and_asks_for_the_journal_in_one_batch() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "  Avenge my brother.  ");

    let set = Input::HeroSet {
        at: NOW,
        field: "goal".to_string(),
        text: "Avenge my brother.".to_string(),
    };
    assert_eq!(game.sent().len(), 2);
    assert_eq!(game.sent_inputs()[1..], [set, journal_asked(2)]);
}

#[test]
fn the_editor_starts_with_the_text_of_the_field() {
    let game = open_book(FILLED);

    click(&game, GOAL);

    assert_eq!(editor_text(&game), "Find my brother.");
}

#[test]
fn the_editor_takes_the_place_of_the_page_until_the_player_cancels() {
    let game = open_book(FILLED);
    let page_shows = "return TimewaysJournalFrameScroll:IsShown()";

    click(&game, GOAL);
    let hidden_while_writing = !game.eval::<bool>(page_shows);
    game.run("wow.Button('Cancel'):Click()");

    assert!(hidden_while_writing);
    assert!(game.eval::<bool>(page_shows));
    assert!(!game.eval::<bool>("return ns.Editor.IsShown()"));
    assert_eq!(game.sent().len(), 1);
}

#[test]
fn enter_saves_the_text_and_a_line_break_becomes_a_space() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    game.run(
        "local box = wow.EditBox()
         box:SetText('Avenge\\nmy brother.')
         box.scripts.OnEnterPressed(box)",
    );

    let set = Input::HeroSet {
        at: NOW,
        field: "goal".to_string(),
        text: "Avenge my brother.".to_string(),
    };
    assert_eq!(game.sent_inputs()[1], set);
}

#[test]
fn the_editor_stops_at_the_limit_of_the_desktop_and_counts_the_letters() {
    let game = open_book(FILLED);

    click(&game, GOAL);

    let limit: usize = game.eval("wow.EditBox().maxLetters");
    assert_eq!(limit, timeways_story::hero::MAX_TEXT_CHARS);
    let count = format!("16 / {limit}");
    let shown: Vec<String> = game.eval("wow.ShownTexts(wow.EditBox().parent.parent)");
    assert!(shown.contains(&count), "{shown:?}");
}

#[test]
fn the_editor_stops_at_the_byte_limit_of_the_desktop() {
    let game = Game::new();

    let limit: usize = game.eval("return ns.Hero.MAX_BYTES");

    assert_eq!(limit, timeways_story::hero::MAX_TEXT_BYTES);
}

#[test]
fn a_text_too_long_to_save_stays_in_the_editor_with_the_reason() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    let wide = "\u{e9}".repeat(700);

    write(&game, &wide);

    assert!(game.eval::<bool>("return ns.Editor.IsShown()"));
    assert_eq!(editor_text(&game), wide);
    let shown: Vec<String> = game.eval("wow.ShownTexts(wow.EditBox().parent.parent)");
    assert!(
        shown.contains(&"Too long to save. Try a shorter version.".to_string()),
        "{shown:?}"
    );
    assert_eq!(game.sent().len(), 1);
}

#[test]
fn the_box_scrolls_to_keep_the_cursor_in_view() {
    let game = open_book(FILLED);
    click(&game, GOAL);

    let offset: f64 = game.eval(
        "local box = wow.EditBox()
         box.parent:SetHeight(100)
         box.scripts.OnCursorChanged(box, 0, -130, 2, 14)
         return box.parent:GetVerticalScroll()",
    );

    assert!((offset - 44.0).abs() < f64::EPSILON, "{offset}");
}

#[test]
fn an_unchanged_field_sends_nothing() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "Find my brother.");

    assert_eq!(game.sent().len(), 1);
}

#[test]
fn add_writes_an_entry_and_an_empty_text_writes_nothing() {
    let game = open_book(FILLED);

    click(&game, "Your Notes");
    write(&game, "   ");
    click(&game, "Your Notes");
    write(&game, "A stranger knew my name.");

    let added = Input::HeroAdded {
        at: NOW,
        text: "A stranger knew my name.".to_string(),
        npc: None,
    };
    assert_eq!(game.sent_inputs()[1..], [added, journal_asked(2)]);
}

#[test]
fn remove_asks_first_and_then_removes_the_entry() {
    let game = open_book(FILLED);

    click(&game, "An oath.");
    let asked_first = game.sent().len() == 1;
    game.run("wow.AcceptPopup()");

    assert!(asked_first);
    assert_eq!(
        game.sent_inputs()[1..],
        [Input::HeroRemoved { at: NOW, number: 4 }, journal_asked(2)]
    );
}

#[test]
fn a_saved_field_shows_at_once_with_a_saving_mark() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    let lines = lines(&game);
    let goal = lines
        .iter()
        .position(|line| line == "heading: What does your character want? [Edit]")
        .unwrap();
    assert_eq!(
        lines[goal + 1..goal + 3],
        ["text: Avenge my brother.", "hint: Saving..."]
    );
    let shown: Vec<String> =
        game.eval("wow.ShownTexts(TimewaysJournalFrameScroll:GetScrollChild())");
    assert!(shown.contains(&"Avenge my brother.".to_string()));
}

#[test]
fn a_cleared_field_shows_as_not_answered_at_once() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "");

    let lines = lines(&game);
    assert!(lines.contains(&"hint: Not answered yet.".to_string()));
    assert!(!lines.contains(&"text: Find my brother.".to_string()));
}

#[test]
fn the_editor_shows_the_saved_text_before_the_journal_comes() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    click(&game, GOAL);

    assert_eq!(editor_text(&game), "Avenge my brother.");
}

#[test]
fn a_new_entry_shows_at_once_with_a_saving_mark() {
    let game = open_book(FILLED);

    click(&game, "Your Notes");
    write(&game, "A stranger knew my name.");

    let lines = lines(&game);
    assert_eq!(
        lines[lines.len() - 2..],
        ["entry: A stranger knew my name.", "hint: Saving..."]
    );
}

#[test]
fn a_removed_entry_leaves_the_page_at_once() {
    let game = open_book(FILLED);

    click(&game, "An oath.");
    game.run("wow.AcceptPopup()");

    let lines = lines(&game);
    assert!(!lines.contains(&"entry: An oath. [Remove]".to_string()));
    assert!(
        lines
            .last()
            .unwrap()
            .starts_with("help: Write anything the game")
    );
}

#[test]
fn the_next_journal_replaces_the_unsaved_edits_and_clears_the_mark() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    let saved = r#"{"sheet":[{"field":"goal","text":"Avenge them all."}],"entries":[]}"#;
    game.reply(&hero_reply(saved, "null"));

    let lines = lines(&game);
    assert!(lines.contains(&"text: Avenge them all.".to_string()));
    assert!(!lines.iter().any(|line| line.contains("Saving")));
}

#[test]
fn a_refused_edit_goes_away_and_the_reason_shows() {
    let game = open_book(FILLED);
    click(&game, "Your Notes");
    write(&game, "A stranger knew my name.");
    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    game.reply(&hero_reply(FILLED, r#""Not saved: too long.""#));

    let lines = lines(&game);
    assert!(lines.contains(&"text: Find my brother.".to_string()));
    assert!(!lines.contains(&"entry: A stranger knew my name.".to_string()));
    assert!(!lines.iter().any(|line| line.contains("Saving")));
    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Not saved: too long."]
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
    empty.run("ns.JournalFrame.Open('deeds')");
    empty.reply(&hero_reply(r#"{"sheet":[],"entries":[]}"#, "null"));

    let written = Game::new();
    written.run("wow.Slash('/journal', '')");
    written.reply(&hero_reply(FILLED, "null"));

    assert_eq!(empty.eval::<String>("ns.JournalFrame.Section()"), "deeds");
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
    assert_eq!(tabs, ["Hero", "Chronicle", "Deeds", "Knowledge", "Tasks"]);
}

#[test]
fn a_sheet_field_with_no_name_shows_gaps_and_no_error() {
    let game = Game::new();

    game.reply(&hero_reply(
        r#"{"sheet":[{"text":"x"}],"entries":[]}"#,
        "null",
    ));

    assert!(!lines(&game).is_empty());
}

#[test]
fn an_entry_number_that_is_no_whole_number_removes_nothing_and_raises_no_error() {
    let game = Game::new();
    let hero = r#"{"sheet":[],"entries":[{"number":1.5,"at":1790000000,"text":"An oath."}]}"#;
    game.reply(&hero_reply(hero, "null"));

    click(&game, "An oath.");

    assert_eq!(game.eval::<u32>("#wow.popups"), 0);
    assert!(game.sent().is_empty());
}
