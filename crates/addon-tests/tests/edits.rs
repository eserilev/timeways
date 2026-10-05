//! The player's edits of a chapter, a tale, and the summary in the book
//! (docs/plans/chapters.md 11): Edit, Save as keep or replace, Restore, and "Saving...".

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::entry_edits::{EditText, EntryKey, EntryKind};
use timeways_story::input::Input;

const SAGA: &str = "Westfall burned, and the militia held the hill.";

/// One closed chapter with a story, and the open chapter after it.
fn journal(edits: &str) -> String {
    format!(
        concat!(
            r#"{{"type":"journal","page":0,"pages":1,"edits":[{edits}],"chapters":["#,
            r#"{{"number":1,"first":4,"state":"closed","began":1790000000,"title":"Westfall","#,
            r#""zones":["Westfall"],"people":[],"deeds":[],"left_out":0,"prose":"{saga}"}},"#,
            r#"{{"number":2,"first":40,"state":"open","began":1790000000,"title":"Duskwood","#,
            r#""zones":["Duskwood"],"people":[],"deeds":[],"left_out":0}}]}}"#,
        ),
        edits = edits,
        saga = SAGA
    )
}

const NOTE: &str = r#"{"entry":{"kind":"chapter","first":4},"title":"The Long Night","text":"keep","paragraphs":["We sang."]}"#;

/// The book, open on the first chapter.
fn book(edits: &str) -> Game {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");
    game.run("wow.Slash('/journal', '')");
    game.reply(&journal(edits));
    game.run("ns.JournalFrame.Open('chapters') ns.JournalFrame.Select(1)");
    game
}

fn lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('chapters')) do
             table.insert(out, line.style .. ': ' .. line.text)
         end
         return out",
    )
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

/// Writes in the open box, and clicks Save.
fn save(game: &Game, text: &str, title: Option<&str>) {
    let heading = title.map_or(String::new(), |title| {
        format!(
            "for _, widget in ipairs(wow.widgets) do
                 if widget.kind == 'EditBox' and not widget.multiLine then widget:SetText('{title}') end
             end"
        )
    });
    game.run(&format!(
        "wow.MultiLineBox():SetText('{text}')
         {heading}
         wow.Button('Save'):Click()"
    ));
}

fn box_text(game: &Game) -> String {
    game.eval("return wow.MultiLineBox():GetText()")
}

fn edits_sent(game: &Game) -> Vec<Input> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter(|input| matches!(input, Input::EntryEdited { .. }))
        .collect()
}

fn chapter_one() -> EntryKey {
    EntryKey {
        kind: EntryKind::Chapter,
        first: Some(4),
    }
}

fn edited(title: Option<&str>, text: EditText, paragraphs: &[&str]) -> Input {
    Input::EntryEdited {
        at: Tick(1_790_000_000),
        entry: chapter_one(),
        title: title.map(str::to_string),
        text,
        paragraphs: paragraphs.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn a_chapter_page_shows_edit_and_restore_only_when_edited() {
    let plain = book("");
    let noted = book(NOTE);

    assert_eq!(
        buttons(&plain),
        ["Edit", "Previous chapter", "Next chapter"]
    );
    assert_eq!(
        buttons(&noted),
        ["Edit", "Restore", "Previous chapter", "Next chapter"]
    );
}

#[test]
fn a_note_follows_the_story_and_the_players_title_names_the_page() {
    let game = book(NOTE);

    let lines = lines(&game);

    assert_eq!(lines[1], "heading: The Long Night");
    assert_eq!(lines[3], format!("prose: {SAGA}"));
    assert_eq!(lines[4], "prose: We sang.");
}

#[test]
fn the_contents_show_the_players_title_and_edited() {
    let game = book(NOTE);

    let row: Vec<String> = game.eval(
        "for _, row in ipairs(ns.Journal.Page('chapters').list) do
             if row.key == 1 then return { row.text, row.detail } end
         end",
    );

    assert_eq!(row[0], "Chapter 1: The Long Night");
    assert!(row[1].ends_with(" · Edited"), "{}", row[1]);
}

#[test]
fn the_box_holds_the_story_and_the_players_paragraphs() {
    let game = book(NOTE);

    press(&game, "Edit");

    assert_eq!(box_text(&game), format!("{SAGA}\nWe sang."));
}

#[test]
fn save_with_the_story_unchanged_sends_keep_with_the_new_paragraphs() {
    let game = book("");
    press(&game, "Edit");

    save(&game, &format!("{SAGA}\\nWe sang at the camp."), None);

    assert_eq!(
        edits_sent(&game),
        [edited(None, EditText::Keep, &["We sang at the camp."])]
    );
}

#[test]
fn save_with_the_story_changed_sends_replace_and_a_new_title() {
    let game = book("");
    press(&game, "Edit");

    save(&game, "Our own words.", Some("The Long Night"));

    assert_eq!(
        edits_sent(&game),
        [edited(
            Some("The Long Night"),
            EditText::Replace,
            &["Our own words."]
        )]
    );
}

#[test]
fn a_chapter_with_no_story_yet_always_sends_keep() {
    let game = book("");
    game.run("ns.JournalFrame.Select(2)");
    press(&game, "Edit");

    save(&game, "We reached Darkshire.", None);

    let sent = edits_sent(&game);
    assert!(
        matches!(
            &sent[..],
            [Input::EntryEdited {
                text: EditText::Keep,
                ..
            }]
        ),
        "{sent:?}"
    );
}

#[test]
fn restore_asks_first_and_sends_the_narrator_text() {
    let game = book(NOTE);
    press(&game, "Restore");

    let asked: String = game.eval("return StaticPopupDialogs[wow.popups[#wow.popups].which].text");
    game.run("wow.AcceptPopup()");

    assert_eq!(asked, "Restore the narrator's version? Yours is removed.");
    assert_eq!(edits_sent(&game), [edited(None, EditText::Narrator, &[])]);
}

#[test]
fn the_page_says_saving_until_the_journal_comes() {
    let game = book("");
    press(&game, "Edit");
    save(&game, "Our own words.", None);

    let saving = lines(&game).contains(&"help: Saving...".to_string());
    game.reply(&journal(NOTE));

    assert!(saving);
    assert!(!lines(&game).contains(&"help: Saving...".to_string()));
}

/// The text of the writing page that tells why the box can't be saved.
fn reason_shown(game: &Game, reason: &str) -> bool {
    game.eval(&format!(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.text == '{reason}' then return true end
         end
         return false"
    ))
}

#[test]
fn an_edit_over_the_limits_of_a_story_stays_in_the_box() {
    let game = book("");
    press(&game, "Edit");
    let long = "\u{00e9}".repeat(990);

    save(&game, &long, None);

    assert!(edits_sent(&game).is_empty());
    assert_eq!(box_text(&game), long);
    assert!(reason_shown(
        &game,
        "Too long to save. Shorten it a little."
    ));
}

#[test]
fn an_edit_that_does_not_fit_the_strip_stays_in_the_box() {
    let game = book("");
    press(&game, "Edit");
    // The marks of many names can make a text that keeps the limits too long for the strip.
    game.run("ns.Link.Fits = function() return false end");

    save(&game, "Our own words.", None);

    assert!(edits_sent(&game).is_empty());
    assert_eq!(box_text(&game), "Our own words.");
    assert!(reason_shown(
        &game,
        "Too long to save. Shorten it a little."
    ));
}

#[test]
fn the_summary_takes_one_paragraph() {
    let game = book("");
    game.run("ns.JournalFrame.Select('title')");
    press(&game, "Edit");

    save(&game, "One.\\nTwo.", None);
    let refused = edits_sent(&game);
    save(&game, "Who I am.", None);

    assert!(refused.is_empty());
    let sent = edits_sent(&game);
    assert!(
        matches!(
            &sent[..],
            [Input::EntryEdited {
                entry: EntryKey {
                    kind: EntryKind::Summary,
                    first: None
                },
                text: EditText::Replace,
                ..
            }]
        ),
        "{sent:?}"
    );
}

#[test]
fn your_own_name_in_an_edit_goes_out_as_the_mark_of_the_hero() {
    let game = book("");
    press(&game, "Edit");

    save(&game, "Ada held the hill.", None);

    assert_eq!(
        edits_sent(&game),
        [edited(None, EditText::Replace, &["$N held the hill."])]
    );
}

#[test]
fn your_own_summary_goes_out_as_the_history_of_your_profile() {
    let summary = r#"{"entry":{"kind":"summary"},"title":null,"text":"replace","paragraphs":["$N keeps the hill."]}"#;
    let game = book(summary);

    game.run("ns.MspProfile.SetSharing(true)");

    let history: String = game.eval("return TimewaysProfile.fields.HI");
    assert_eq!(history, "Ada keeps the hill.");
}

#[test]
fn the_narrators_summary_stays_private() {
    let game = book("");

    game.run("ns.MspProfile.SetSharing(true)");

    let history: Option<String> = game.eval("return TimewaysProfile.fields.HI");
    assert_eq!(history, None);
}
