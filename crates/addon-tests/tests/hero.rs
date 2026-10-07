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

/// The cards of the open sub-tab, as `Label: text [button] (tag) (note)`.
fn cards(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('hero').cards.rows) do
             if row.kind == 'card' then
                 local action = row.action and (' [' .. row.action.label .. ']') or ''
                 local tag = row.tag and (' (' .. row.tag .. ')') or ''
                 local note = row.note and (' (' .. row.note .. ')') or ''
                 table.insert(out, row.label .. ': ' .. row.text .. action .. tag .. note)
             end
         end
         return out",
    )
}

fn card(game: &Game, label: &str) -> String {
    cards(game)
        .into_iter()
        .find(|card| card.starts_with(&format!("{label}: ")))
        .unwrap()
}

/// Runs the button of the line with this text, or of the card of this question.
fn click(game: &Game, text: &str) {
    game.run(&format!(
        "for _, line in ipairs(ns.Journal.Lines('hero')) do
             if line.text == '{text}' then line.action.run() end
         end
         for _, row in ipairs(ns.Journal.Page('hero').cards.rows) do
             if row.kind == 'card' and ns.Hero.HINTS[row.key] == '{text}' then row.action.run() end
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

/// The box that the player writes in: the writing page of the book while it shows, and else
/// the card that is open for writing. `frame` is the frame that holds its labels and buttons.
const OPEN_BOX: &str = "local box, frame
     if ns.Editor.IsShown() then
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'EditBox' and widget.parent == TimewaysEditorScroll then box = widget end
         end
         frame = box.parent.parent
     else
         box = TimewaysHeroCardBox
         frame = box.parent
     end";

/// Types this text in the open box and clicks its Save.
fn write(game: &Game, text: &str) {
    game.run(&format!(
        "{OPEN_BOX}
         box:SetText('{text}')
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == frame and widget.text == 'Save' then
                 widget:Click()
             end
         end"
    ));
}

fn editor_text(game: &Game) -> String {
    game.eval(&format!("{OPEN_BOX} return box:GetText()"))
}

/// True while a card is open for writing.
fn card_is_open(game: &Game) -> bool {
    game.eval("return TimewaysHeroCardBox ~= nil and TimewaysHeroCardBox.parent:IsShown()")
}

#[test]
fn every_question_shows_as_a_card_with_the_notes_beside_it() {
    let game = Game::new();

    game.reply(&hero_reply(FILLED, "null"));

    assert_eq!(
        cards(&game),
        [
            "Origin: Where is your character from? [Answer]",
            "Background: What did your character do before adventuring? [Answer]",
            "Goal: Find my brother. [Edit]",
            "Bond: Who or what does your character care about most? [Answer]",
            "Flaw: What is your character's biggest flaw? [Answer]",
            "Traits: How would you describe your character's personality? [Answer]",
        ]
    );
    let day: String = game.eval("return date('%d %b', 1790000000)");
    assert_eq!(
        lines(&game),
        [
            "heading: Your Notes [Add a note]".to_string(),
            "entry: An oath. [Remove]".to_string(),
            format!("text: About Innkeeper Farley · Goldshire · {day}"),
        ]
    );
}

#[test]
fn the_page_says_what_the_answers_shape() {
    let game = Game::new();

    game.reply(&hero_reply(FILLED, "null"));

    let first: String = game.eval("return ns.Journal.Page('hero').cards.rows[1].text");
    assert_eq!(first, "Shapes your chapters and what NPCs say to you.");
}

#[test]
fn save_opens_the_next_empty_card() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    assert!(card_is_open(&game));
    let open: String = game.eval("return TimewaysHeroCardBox.parent.label.text");
    assert_eq!(open, "Bond");
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
fn a_card_opens_for_writing_in_place_and_cancel_closes_it() {
    let game = open_book(FILLED);
    let page_shows = "return TimewaysJournalFrameScroll:IsShown()";

    click(&game, GOAL);
    let open = card_is_open(&game);
    let page_while_writing = game.eval::<bool>(page_shows);
    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysHeroCardBox.parent
                 and widget.text == 'Cancel' then widget:Click() end
         end",
    );

    assert!(open && page_while_writing);
    assert!(!card_is_open(&game));
    assert!(!game.eval::<bool>("return ns.Editor.IsShown()"));
    assert_eq!(game.sent().len(), 1);
}

/// The question of the bond, the card after the goal.
const BOND: &str = "Who or what does your character care about most?";

fn cancel_writing(game: &Game) {
    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysHeroCardBox.parent
                 and widget.text == 'Cancel' then widget:Click() end
         end",
    );
}

fn box_has_focus(game: &Game) -> bool {
    game.eval("return TimewaysHeroCardBox:HasFocus()")
}

#[test]
fn opening_an_answer_card_puts_the_cursor_in_its_box() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    let first = box_has_focus(&game);
    cancel_writing(&game);

    click(&game, BOND);

    assert!(first);
    assert!(box_has_focus(&game));
}

#[test]
fn typing_in_an_answer_card_keeps_the_text_when_the_box_grows() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    let before: f64 = game.eval("return TimewaysHeroCardBox:GetHeight()");

    for _ in 0..80 {
        assert!(game.eval::<bool>("return wow.Type(' and more')"));
    }

    let after: f64 = game.eval("return TimewaysHeroCardBox:GetHeight()");
    assert!(after > before, "{before} {after}");
    let typed = format!("Find my brother.{}", " and more".repeat(80));
    assert_eq!(editor_text(&game), typed);
    assert!(box_has_focus(&game));
}

#[test]
fn clicking_the_box_of_an_open_card_puts_the_cursor_there() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    game.run("TimewaysHeroCardBox:ClearFocus()");

    // The press lands below the one line of text, where the box is not.
    game.run("wow.MouseDown(TimewaysHeroCardBox, false)");

    assert!(box_has_focus(&game));
}

#[test]
fn clicking_anywhere_on_an_open_card_puts_the_cursor_in_its_box() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    game.run("TimewaysHeroCardBox:ClearFocus()");

    game.run("wow.MouseDown(TimewaysHeroCardBox.parent.label)");

    assert!(box_has_focus(&game));
}

/// The closed card with this label, as the page shows it. The open card has no button of its
/// own, only Save and Cancel.
fn closed_card(label: &str) -> String {
    format!(
        "(function()
             for _, widget in ipairs(wow.widgets) do
                 if widget.kind == 'Frame' and widget.label and widget.label.text == '{label}'
                     and widget.shown and widget.button then
                     return widget
                 end
             end
         end)()"
    )
}

fn open_card_label(game: &Game) -> String {
    game.eval("return TimewaysHeroCardBox.parent.label.text")
}

fn cursor_at_end_of_card(game: &Game) -> bool {
    game.eval("return wow.CursorAtEnd(TimewaysHeroCardBox)")
}

#[test]
fn clicking_edit_on_an_answer_puts_the_cursor_at_the_end_of_it() {
    let game = open_book(FILLED);

    click(&game, GOAL);

    assert_eq!(editor_text(&game), "Find my brother.");
    assert!(cursor_at_end_of_card(&game));
}

#[test]
fn clicking_a_closed_answer_card_opens_it_with_the_cursor() {
    let game = open_book(FILLED);
    click(&game, BOND);
    cancel_writing(&game);

    game.run(&format!("wow.MouseDown({}.text)", closed_card("Goal")));

    assert!(card_is_open(&game));
    assert_eq!(open_card_label(&game), "Goal");
    assert!(cursor_at_end_of_card(&game));
}

#[test]
fn clicking_a_closed_profile_card_opens_it_with_the_cursor() {
    let game = open_book(FILLED);
    game.run("ns.Journal.Select('hero', 'profile') ns.JournalFrame.Refresh()");

    game.run(&format!("wow.MouseDown({}.label)", closed_card("Age")));

    assert!(card_is_open(&game));
    assert_eq!(open_card_label(&game), "Age");
    assert!(cursor_at_end_of_card(&game));
}

#[test]
fn a_press_on_the_button_of_a_closed_card_is_left_to_the_button() {
    let game = open_book(FILLED);
    click(&game, BOND);
    cancel_writing(&game);

    game.run(&format!("wow.MouseDown({}.button)", closed_card("Goal")));

    assert!(!card_is_open(&game));
}

#[test]
fn closing_the_book_while_writing_in_a_card_gives_the_cursor_back() {
    let game = open_book(FILLED);
    click(&game, GOAL);

    game.run("TimewaysJournalFrame:Hide()");

    assert!(game.eval::<bool>("return wow.focus == nil"));
    assert!(!game.eval::<bool>("return wow.Type('w')"));
}

#[test]
fn add_a_note_opens_the_writing_page_with_the_cursor() {
    let game = open_book(FILLED);

    click(&game, "Your Notes");

    let has_cursor: bool = game.eval(&format!("{OPEN_BOX} return wow.CursorAtEnd(box)"));
    assert!(has_cursor);
}

#[test]
fn a_box_that_grows_moves_the_cards_below_it() {
    let game = open_book(FILLED);
    let flaw_top = "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.text == 'Flaw' and widget.parent:IsShown() then
                 return widget.parent.point[5]
             end
         end";
    click(&game, GOAL);
    let before: f64 = game.eval(flaw_top);

    game.run(&format!(
        "TimewaysHeroCardBox:SetText('{}')
         TimewaysHeroCardBox.scripts.OnTextChanged(TimewaysHeroCardBox, true)",
        "word ".repeat(200)
    ));

    let after: f64 = game.eval(flaw_top);
    assert!(after < before, "{before} {after}");
}

#[test]
fn enter_saves_the_text_and_a_line_break_becomes_a_space() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    game.run(
        "local box = TimewaysHeroCardBox
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
fn a_pasted_tab_or_other_control_character_becomes_a_space() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "Avenge\\tmy\\abrother.");

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

    let limit: usize = game.eval("TimewaysHeroCardBox.maxLetters");
    assert_eq!(limit, timeways_story::hero::LONG.chars);
    let count = format!("16 / {limit}");
    let shown: Vec<String> = game.eval("wow.ShownTexts(TimewaysHeroCardBox.parent)");
    assert!(shown.contains(&count), "{shown:?}");
}

#[test]
fn the_editor_stops_at_the_byte_limit_of_the_desktop() {
    let game = Game::new();

    let limit: usize = game.eval("return ns.Hero.MAX_BYTES");

    assert_eq!(limit, timeways_story::hero::LONG.bytes);
}

#[test]
fn a_text_too_long_to_save_stays_in_the_editor_with_the_reason() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    let wide = "\u{e9}".repeat(700);

    write(&game, &wide);

    assert!(card_is_open(&game));
    assert_eq!(editor_text(&game), wide);
    let shown: Vec<String> = game.eval("wow.ShownTexts(TimewaysHeroCardBox.parent)");
    assert!(
        shown.contains(&"Too long to save. Try a shorter version.".to_string()),
        "{shown:?}"
    );
    assert_eq!(game.sent().len(), 1);
}

/// The label under the box: the count, or the reason that the text can't be saved.
fn count_label(game: &Game) -> (String, Vec<f64>) {
    game.run(&format!(
        "{OPEN_BOX}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.point and widget.point[1] == 'TOPRIGHT'
                 and widget.parent == frame then
                 countLabel = widget
             end
         end"
    ));
    (
        game.eval("countLabel.text"),
        game.eval("countLabel.textColor"),
    )
}

#[test]
fn the_reason_that_a_text_cannot_be_saved_shows_in_the_ink_of_the_text() {
    let game = open_book(FILLED);
    click(&game, GOAL);

    write(&game, &"\u{e9}".repeat(700));

    let (text, color) = count_label(&game);
    let ink: Vec<f64> = game.eval("ns.Ink.text");
    assert_eq!(text, "Too long to save. Try a shorter version.");
    assert_eq!(color, ink);
}

#[test]
fn a_text_with_accented_letters_counts_toward_the_byte_limit() {
    let game = open_book(FILLED);
    click(&game, GOAL);

    game.run(&format!(
        "local box = TimewaysHeroCardBox
         box:SetText('{}')
         box.scripts.OnTextChanged(box, true)",
        "\u{e9}".repeat(550)
    ));

    let (text, color) = count_label(&game);
    let faded: Vec<f64> = game.eval("ns.Ink.faded");
    assert_eq!(text, "About 100 left");
    assert_eq!(color, faded);
}

#[test]
fn a_text_past_the_byte_limit_has_none_left() {
    let game = open_book(FILLED);
    click(&game, GOAL);

    game.run(&format!(
        "local box = TimewaysHeroCardBox
         box:SetText('{}')
         box.scripts.OnTextChanged(box, true)",
        "\u{e9}".repeat(700)
    ));

    assert_eq!(count_label(&game).0, "None left");
}

#[test]
fn the_box_takes_the_width_that_the_layout_gives_its_scroll_frame() {
    let game = open_book(FILLED);
    click(&game, "Your Notes");

    let width: f64 = game.eval(
        "TimewaysEditorScroll:SetWidth(312)
         return wow.EditBox().width",
    );

    assert!((width - 312.0).abs() < f64::EPSILON);
}

#[test]
fn the_box_scrolls_to_keep_the_cursor_in_view() {
    let game = open_book(FILLED);
    click(&game, "Your Notes");

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

    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.parent == TimewaysHeroCardBox.parent
                 and widget.text == 'Cancel' then widget:Click() end
         end",
    );

    assert_eq!(
        card(&game, "Goal"),
        "Goal: Avenge my brother. [Edit] (Saving...)"
    );
    let shown: bool = game.eval(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.shown and widget.text == 'Avenge my brother.'
                 and widget.parent.parent == TimewaysJournalCardsScroll:GetScrollChild() then
                 return true
             end
         end
         return false",
    );
    assert!(shown);
}

#[test]
fn a_cleared_field_shows_as_not_answered_at_once() {
    let game = open_book(FILLED);

    click(&game, GOAL);
    write(&game, "");

    assert_eq!(
        card(&game, "Goal"),
        "Goal: What does your character want? [Answer] (Saving...)"
    );
}

#[test]
fn the_editor_shows_the_saved_text_before_the_journal_comes() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    click(&game, GOAL);

    assert_eq!(editor_text(&game), "Avenge my brother.");
}

/// The lines of the page: the open question, then the notes.
fn notes(game: &Game) -> Vec<String> {
    lines(game)
}

#[test]
fn a_new_entry_shows_at_once_with_a_saving_mark() {
    let game = open_book(FILLED);

    click(&game, "Your Notes");
    write(&game, "A stranger knew my name.");

    let lines = notes(&game);
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

    let lines = notes(&game);
    assert_eq!(
        lines,
        [
            "heading: Your Notes [Add a note]",
            "help: A grudge, a promise, a secret."
        ]
    );
}

#[test]
fn the_next_journal_replaces_the_unsaved_edits_and_clears_the_mark() {
    let game = open_book(FILLED);
    click(&game, GOAL);
    write(&game, "Avenge my brother.");

    let saved = r#"{"sheet":[{"field":"goal","text":"Avenge them all."}],"entries":[]}"#;
    game.reply(&hero_reply(saved, "null"));

    assert_eq!(card(&game, "Goal"), "Goal: Avenge them all. [Edit]");
    assert!(!cards(&game).iter().any(|card| card.contains("Saving")));
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
    assert_eq!(card(&game, "Goal"), "Goal: Find my brother. [Edit]");
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
    empty.run("ns.JournalFrame.Open('knowledge')");
    empty.reply(&hero_reply(r#"{"sheet":[],"entries":[]}"#, "null"));

    let written = Game::new();
    written.run("wow.Slash('/journal', '')");
    written.reply(&hero_reply(FILLED, "null"));

    assert_eq!(
        empty.eval::<String>("ns.JournalFrame.Section()"),
        "knowledge"
    );
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
    assert_eq!(
        tabs,
        ["Hero", "Chronicle", "Stories", "Knowledge", "Quests"]
    );
}

#[test]
fn a_sheet_field_with_no_name_shows_gaps_and_no_error() {
    let game = Game::new();

    game.reply(&hero_reply(
        r#"{"sheet":[{"text":"x"}],"entries":[]}"#,
        "null",
    ));

    assert_eq!(cards(&game).len(), 6);
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

/// A card prints in the quest fonts of the right page, in dark ink.
#[test]
fn the_cards_print_in_the_quest_fonts() {
    let game = open_book(FILLED);

    let fonts: Vec<String> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             local text = widget.text
             if widget.kind == 'FontString' and widget.shown
                 and (text == 'Goal' or text == 'Find my brother.') then
                 table.insert(out, text .. ': ' .. tostring(widget.font))
             end
         end
         table.sort(out)
         return out",
    );

    assert_eq!(
        fonts,
        ["Find my brother.: QuestFont", "Goal: QuestTitleFont"]
    );
}

#[test]
fn a_sheet_that_spreads_over_two_pages_shows_whole() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":2,"hero":{"sheet":[{"field":"origin","text":"A farm."}],"entries":[]},"hero_refused":null}"#,
    );
    game.reply(
        r#"{"type":"journal","page":1,"pages":2,"hero":{"sheet":[{"field":"goal","text":"Find my brother."}],"entries":[]}}"#,
    );

    assert_eq!(card(&game, "Origin"), "Origin: A farm. [Edit]");
    assert_eq!(card(&game, "Goal"), "Goal: Find my brother. [Edit]");
}
