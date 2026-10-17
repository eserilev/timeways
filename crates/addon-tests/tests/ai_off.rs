//! AI off: the player chose no model at all (GAMEPLAY.md 5.6). The journal says so, and the
//! addon hides what needs a model or says how to turn AI on.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::ai::Ai;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{Journal, pages};
use timeways_story::story::Output;

const TALK_OFF: &str = "|cffc8a064Timeways|r: Talking to NPCs needs AI. To turn it on, run gnomish-relay setup --timeways --ai on.";
const QUEST_OFF: &str = "|cffc8a064Timeways|r: Quests from NPCs need AI. To turn it on, run gnomish-relay setup --timeways --ai on.";

fn journal_reply(ai: Ai) -> String {
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page: Box::new(pages(Journal::default()).remove(0)),
        notice: None,
        dev: None,
        past: None,
        ai,
    })
    .unwrap()
}

/// The addon after a journal with AI off came, with nothing printed or sent yet.
fn game_with_ai_off() -> Game {
    let game = Game::new();
    game.reply(&journal_reply(Ai::Off));
    game.run("wow.printed = {}");
    game
}

fn page_lines(game: &Game, section: &str) -> Vec<String> {
    game.eval(&format!(
        "local out = {{}}
         for _, line in ipairs(ns.Journal.Lines('{section}')) do table.insert(out, line.text or '') end
         return out"
    ))
}

fn quest_buttons(game: &Game) -> Vec<String> {
    game.eval(
        "ns.JournalFrame.Open('quests')
         ns.TaskForm.Open()
         local out = {}
         for _, button in ipairs(ns.Journal.Page('quests').buttons) do table.insert(out, button.label) end
         return out",
    )
}

#[test]
fn a_journal_with_no_ai_field_keeps_ai_on() {
    let game = Game::new();
    let mut old: serde_json::Value = serde_json::from_str(&journal_reply(Ai::Off)).unwrap();
    old.as_object_mut().unwrap().remove("ai");

    game.reply(&old.to_string());

    assert!(game.eval::<bool>("return ns.Ai.IsOn()"));
}

#[test]
fn a_journal_says_ai_is_off_and_a_later_one_turns_it_on() {
    let game = game_with_ai_off();
    assert!(!game.eval::<bool>("return ns.Ai.IsOn()"));

    game.reply(&journal_reply(Ai::On));

    assert!(game.eval::<bool>("return ns.Ai.IsOn()"));
}

#[test]
fn talk_with_ai_off_opens_no_window_and_says_how_to_turn_it_on() {
    let game = game_with_ai_off();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', 'any news?')");

    assert!(!game.eval::<bool>("return ns.TalkWindow.IsShown()"));
    assert_eq!(game.printed(), [TALK_OFF]);
}

#[test]
fn talk_with_ai_off_still_sends_the_talk_for_quest_steps() {
    let game = game_with_ai_off();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', 'any news?')");

    let talk = Input::TalkAsked {
        id: MessageId(1),
        at: Tick(1_790_000_000),
        npc: "Innkeeper Farley".to_string(),
        text: "any news?".to_string(),
    };
    assert_eq!(game.sent_inputs(), [talk]);
}

#[test]
fn the_silent_answer_of_a_talk_with_ai_off_adds_no_line() {
    let game = game_with_ai_off();
    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', 'any news?')");

    game.reply(r#"{"type":"talk_answer","id":1,"npc":"Innkeeper Farley","text":null}"#);

    assert_eq!(game.printed(), [TALK_OFF]);
}

#[test]
fn quest_with_ai_off_sends_nothing_and_says_how_to_turn_it_on() {
    let game = game_with_ai_off();
    let before = game.sent().len();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/quest', '')");

    assert_eq!(game.sent().len(), before);
    assert_eq!(game.printed(), [QUEST_OFF]);
}

#[test]
fn help_me_write_hides_with_ai_off() {
    let on = Game::new();
    let off = game_with_ai_off();

    assert!(quest_buttons(&on).contains(&"Help me write".to_string()));
    assert!(!quest_buttons(&off).contains(&"Help me write".to_string()));
}

#[test]
fn a_typed_step_with_ai_off_stays_as_written_and_asks_nothing() {
    let game = game_with_ai_off();
    quest_buttons(&game);
    let before = game.sent().len();

    game.run(
        "ns.TaskForm.SetStepText('wave at the pond')
         ns.TaskForm.AddStep()
         ns.Outbox.Flush()",
    );

    let page = page_lines(&game, "quests");
    assert!(page.contains(&"wave at the pond.".to_string()), "{page:?}");
    assert!(!page.contains(&"Checking...".to_string()), "{page:?}");
    assert_eq!(game.sent().len(), before);
}

#[test]
fn the_title_page_with_ai_off_promises_no_summary() {
    let on = Game::new();
    on.reply(&journal_reply(Ai::On));
    let off = game_with_ai_off();
    let promise = "Fills in when your first chapter ends.".to_string();

    assert!(page_lines(&on, "chapters").contains(&promise));
    assert!(!page_lines(&off, "chapters").contains(&promise));
}

#[test]
fn the_hero_page_with_ai_off_says_only_what_to_write() {
    let game = game_with_ai_off();

    let first: String = game.eval("return ns.Journal.Page('hero').cards.rows[1].text");

    assert_eq!(first, "Tell us about your character.");
}

#[test]
fn the_quests_page_with_ai_off_names_only_quests_from_players() {
    let game = game_with_ai_off();

    let footer: String = game.eval("return ns.Journal.Page('quests').footer");
    let empty = page_lines(&game, "quests");

    assert_eq!(footer, "Quests from other players.");
    assert_eq!(empty, ["No quests yet."]);
}

#[test]
fn help_says_whether_ai_is_on() {
    let on = Game::new();
    on.reply(&journal_reply(Ai::On));
    let off = game_with_ai_off();

    on.run("wow.Slash('/timeways', 'help')");
    off.run("wow.Slash('/timeways', 'help')");

    assert!(
        on.printed()
            .contains(&"|cffc8a064Timeways|r: AI: On".to_string())
    );
    assert!(
        off.printed()
            .contains(&"|cffc8a064Timeways|r: AI: Off".to_string())
    );
}

#[test]
fn help_says_nothing_of_ai_before_a_journal_came() {
    let game = Game::new();

    game.run("wow.Slash('/timeways', 'help')");

    assert!(
        game.printed().iter().all(|line| !line.contains("AI:")),
        "{:?}",
        game.printed()
    );
}

#[test]
fn a_talk_sent_while_ai_was_on_still_ends_its_turn_when_ai_turns_off() {
    let game = Game::new();
    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', 'any news?')");
    game.reply(&journal_reply(Ai::Off));

    game.reply(r#"{"type":"talk_answer","id":1,"npc":"Innkeeper Farley","text":null}"#);

    let page: Vec<String> = game.eval("wow.ShownTexts(TimewaysTalkFrameScroll:GetScrollChild())");
    assert!(!page.contains(&"Thinking...".to_string()), "{page:?}");
    assert!(
        page.contains(&"Innkeeper Farley looks at you and says nothing.".to_string()),
        "{page:?}"
    );
}

#[test]
fn quest_with_no_words_and_ai_off_says_quests_from_npcs_need_ai() {
    let game = game_with_ai_off();

    game.run("wow.Slash('/quest', 'what')");

    assert_eq!(game.printed(), [QUEST_OFF]);
}

#[test]
fn a_note_with_ai_off_promises_no_story() {
    let on = Game::new();
    let off = game_with_ai_off();
    let hint = "local shown
         ns.JournalFrame.Edit = function(edit) shown = edit.hint end
         ns.Hero.Write()
         return shown";

    assert_eq!(
        on.eval::<String>(hint),
        "Anything about your character. It becomes part of your story."
    );
    assert_eq!(off.eval::<String>(hint), "Anything about your character.");
}
