//! The AI setting of the addon (GAMEPLAY.md 5.6): `/timeways ai on|off`, saved for the
//! account, and told to the story program in the `ai_set` line.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use fake_bridge::{FakeBridge, Reply};
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &str = r#"{"name":"Ada","realm":"Stormrage","type":"character_entered"}"#;
const AI_OFF_LINE: &str = r#"{"ai":"off","type":"ai_set"}"#;
const AI_ON_LINE: &str = r#"{"ai":"on","type":"ai_set"}"#;
const SAID_OFF: &str =
    "|cffc8a064Timeways|r: AI is off. NPCs won't talk and the narrator stays quiet.";
const SAID_ON: &str = "|cffc8a064Timeways|r: AI is on.";
const HELP_ON: &str = "|cffc8a064Timeways|r: AI: On. To turn it off, type /timeways ai off.";
const HELP_OFF: &str = "|cffc8a064Timeways|r: AI: Off. To turn it on, type /timeways ai on.";
const AI_USAGE: &str = "|cffc8a064Timeways|r: Type /timeways ai on or /timeways ai off.";

fn slash(game: &Game, message: &str) {
    game.run(&format!("wow.Slash('/timeways', '{message}')"));
}

fn is_on(game: &Game) -> bool {
    game.eval("return ns.Ai.IsOn()")
}

/// The addon after login, with AI turned off in an earlier session.
fn game_with_setting_off() -> Game {
    let game = Game::before_login();
    game.run(
        "TimewaysSettings = { ai = false }
         ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))",
    );
    game
}

fn level_up(game: &Game) {
    game.run("ns.Outbox.Add(ns.Inputs.Level(time(), 20)); ns.Outbox.Flush()");
}

fn lines_of(batch: &str) -> Vec<&str> {
    batch.lines().collect()
}

fn newest_batch(game: &Game) -> String {
    game.sent().pop().unwrap()
}

#[test]
fn ai_is_on_by_default() {
    assert!(is_on(&Game::new()));
}

#[test]
fn ai_off_is_saved_as_false_in_the_account_settings() {
    let game = Game::new();

    slash(&game, "ai off");

    assert!(!game.eval::<bool>("return TimewaysSettings.ai"));
    assert!(!is_on(&game));
}

/// The harness has no real reload: a new game gets the saved variables as WoW loads them.
#[test]
fn a_saved_false_keeps_ai_off_in_a_new_session() {
    let game = Game::new();

    game.run("TimewaysSettings = { ai = false }");

    assert!(!is_on(&game));
}

/// Any addon can write the saved variables, so only a plain `false` turns AI off.
#[test]
fn a_junk_saved_ai_setting_counts_as_on() {
    for junk in ["'no'", "0", "'false'", "{}", "nil"] {
        let game = Game::new();

        game.run(&format!("TimewaysSettings = {{ ai = {junk} }}"));

        assert!(is_on(&game), "{junk}");
    }
    let game = Game::new();
    game.run("TimewaysSettings = 'off'");
    assert!(is_on(&game));
}

#[test]
fn the_setting_needs_no_journal() {
    let game = game_with_setting_off();

    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', 'hi')");

    assert!(!game.eval::<bool>("return ns.TalkWindow.IsShown()"));
}

#[test]
fn turning_ai_off_and_on_says_so_in_the_chat() {
    let game = Game::new();

    slash(&game, "ai off");
    slash(&game, " AI  On ");

    assert_eq!(game.printed(), [SAID_OFF, SAID_ON]);
}

#[test]
fn help_says_whether_ai_is_on_with_no_journal() {
    let on = Game::new();
    let off = game_with_setting_off();

    on.run("wow.Slash('/timeways', 'help')");
    off.run("wow.Slash('/timeways', 'help')");

    assert!(on.printed().contains(&HELP_ON.to_string()));
    assert!(off.printed().contains(&HELP_OFF.to_string()));
}

#[test]
fn ai_with_no_word_or_a_wrong_word_says_how_to_use_it() {
    for message in ["ai", "ai maybe", " AI  of "] {
        let game = Game::new();
        game.run("opened = false; ns.JournalFrame.Toggle = function() opened = true end");

        slash(&game, message);

        assert_eq!(game.printed(), [AI_USAGE], "{message}");
        assert!(!game.eval::<bool>("return opened"), "{message}");
    }
}

#[test]
fn while_ai_is_off_each_batch_says_so_after_the_character_line() {
    let game = game_with_setting_off();

    level_up(&game);
    level_up(&game);

    for batch in game.sent() {
        assert_eq!(lines_of(&batch)[..2], [CHARACTER, AI_OFF_LINE]);
    }
}

#[test]
fn while_ai_is_on_each_batch_says_so_after_the_character_line() {
    let game = Game::new();

    level_up(&game);

    assert_eq!(lines_of(&newest_batch(&game))[..2], [CHARACTER, AI_ON_LINE]);
}

#[test]
fn a_batch_of_one_input_carries_the_ai_line() {
    let game = game_with_setting_off();

    let batch: String = game.eval("return ns.Outbox.Alone(ns.Inputs.JournalAsked(0))");

    assert_eq!(lines_of(&batch)[..2], [CHARACTER, AI_OFF_LINE]);
}

#[test]
fn events_kept_from_the_last_session_go_after_the_ai_line() {
    let game = Game::before_login();
    game.run(
        r#"TimewaysSettings = { ai = false }
           TimewaysWaiting = { '{"at":1790000000,"level":20,"type":"level_reached"}' }
           ns.Outbox.Load()
           ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))
           ns.Outbox.Flush()"#,
    );

    let level = r#"{"at":1790000000,"level":20,"type":"level_reached"}"#;
    assert_eq!(
        lines_of(&newest_batch(&game)),
        [CHARACTER, AI_OFF_LINE, level]
    );
}

#[test]
fn turning_ai_off_tells_the_story_program_at_once() {
    let game = Game::new();

    slash(&game, "ai off");

    assert_eq!(lines_of(&newest_batch(&game)), [CHARACTER, AI_OFF_LINE]);
}

#[test]
fn turning_ai_on_tells_the_story_program_at_once() {
    let game = game_with_setting_off();

    slash(&game, "ai on");

    assert_eq!(lines_of(&newest_batch(&game)), [CHARACTER, AI_ON_LINE]);
}

#[test]
fn an_on_line_lost_with_its_batch_is_sent_again_with_the_next() {
    let game = game_with_setting_off();
    slash(&game, "ai on");
    let lost = game.sent().len();

    game.run(&format!("ns.Link.Receive({lost}, 'error', 'Timed out.')"));
    level_up(&game);

    assert_eq!(lines_of(&newest_batch(&game))[..2], [CHARACTER, AI_ON_LINE]);
}

#[test]
fn the_ai_line_goes_with_the_first_batch_after_a_reload() {
    let game = Game::before_login();
    game.run(
        r#"TimewaysSettings = { ai = true }
           TimewaysWaiting = { '{"at":1790000000,"level":20,"type":"level_reached"}' }
           ns.Outbox.Load()
           ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))
           ns.Outbox.Flush()"#,
    );

    let level = r#"{"at":1790000000,"level":20,"type":"level_reached"}"#;
    assert_eq!(
        lines_of(&newest_batch(&game)),
        [CHARACTER, AI_ON_LINE, level]
    );
}

const JOURNAL_SAYS_OFF: &str = r#"{"type":"journal","page":0,"pages":1,"ai":"off"}"#;

/// Answers batch `id` with a journal that says AI is off.
fn answer_with_journal_off(game: &Game, id: usize) {
    game.run(&format!(
        "ns.Link.Receive({id}, 'done', [[{JOURNAL_SAYS_OFF}]])"
    ));
}

#[test]
fn a_journal_of_no_known_batch_counts_before_any_change() {
    let game = Game::new();

    game.reply(JOURNAL_SAYS_OFF);

    assert!(!is_on(&game));
}

/// A frame that the transport sends again after a reload has no batch, so it can be old.
#[test]
fn a_journal_of_no_known_batch_is_ignored_after_a_change() {
    let game = game_with_setting_off();
    slash(&game, "ai on");

    game.reply(JOURNAL_SAYS_OFF);

    assert!(is_on(&game));
}

#[test]
fn a_journal_asked_before_ai_on_never_turns_ai_off_again() {
    let game = game_with_setting_off();
    game.run("ns.Journal.Request(0)");
    let asked = game.sent().len();

    slash(&game, "ai on");
    answer_with_journal_off(&game, asked);

    assert!(is_on(&game));
}

#[test]
fn a_journal_asked_after_ai_on_can_still_say_off() {
    let game = game_with_setting_off();
    slash(&game, "ai on");
    game.run("wow.RunTickers(); ns.Journal.Request(0)");
    let asked = game.sent().len();

    answer_with_journal_off(&game, asked);

    assert!(!is_on(&game));
}

#[test]
fn a_change_while_the_desktop_is_away_goes_with_the_next_batch() {
    let game = Game::new();
    game.run("linkUp = false");
    slash(&game, "ai off");
    slash(&game, "ai on");

    game.run("linkUp = true");
    level_up(&game);

    assert_eq!(game.sent().len(), 1);
    assert_eq!(lines_of(&newest_batch(&game))[..2], [CHARACTER, AI_ON_LINE]);
}

/// Adds `input` while the desktop is away, in a link that fits it only with AI on, and
/// turns AI off. Gives the error of the question.
fn error_of_a_question_that_no_longer_fits(game: &Game, input: &str) -> Option<String> {
    let question: String = game.eval(&format!("return ns.Json.Encode({input})"));
    let fits_with_on = [CHARACTER, AI_ON_LINE, &question].join("\n").len();
    game.run(&format!(
        "linkUp = false
         linkLimit = {fits_with_on}
         ns.Outbox.Add({input}, function(text) failedWith = text end)"
    ));

    slash(game, "ai off");

    assert_eq!(game.eval::<usize>("return ns.Outbox.Waiting()"), 0);
    game.eval("return failedWith")
}

#[test]
fn a_typed_question_that_no_longer_fits_asks_for_a_shorter_one() {
    let game = Game::new();
    let lore = "{ type = 'lore_asked', at = 1790000000, question = 'Who rules here?' }";

    let error = error_of_a_question_that_no_longer_fits(&game, lore);

    let shorter = "Too long to send. Try a shorter message.";
    assert_eq!(error.as_deref(), Some(shorter));
    assert!(
        game.printed()
            .contains(&format!("|cffc8a064Timeways|r: {shorter}"))
    );
}

#[test]
fn a_journal_request_that_no_longer_fits_says_the_journal_did_not_load() {
    let game = Game::new();

    let error = error_of_a_question_that_no_longer_fits(&game, "ns.Inputs.JournalAsked(0)");

    let failed = "Couldn't load your journal. Try again.";
    assert_eq!(error.as_deref(), Some(failed));
    assert!(
        game.printed()
            .contains(&format!("|cffc8a064Timeways|r: {failed}"))
    );
}

fn passage(text: &str, links: Vec<Link>, about: Option<&str>) -> Passage {
    Passage {
        text: text.to_string(),
        source: "https://example.test/1".to_string(),
        links,
        origin: Origin::Pack,
        about: about.map(str::to_string),
        depends_on: Vec::new(),
        setup_for: None,
    }
}

fn story_with_lore(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ai-setting-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let testvale = vec![Link::Place("Testvale".to_string())];
    let tower = passage("The tower of Testvale fell.", testvale, None);
    let king =
        "King Barathen Wrynn scattered the gnolls of Elwynn, and his line rules Stormwind City.";
    let stormwind = passage(king, vec![Link::Common], Some("Stormwind City"));
    Pack::write(&path, &[tower, stormwind]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

/// The model calls that the first batch of `game` opens in a fresh story program.
fn model_calls_of_first_batch(game: &Game, name: &str) -> usize {
    game.run(
        "ns.Outbox.Add(ns.Inputs.Described(time(), 'Human', 'PALADIN'))
         ns.Outbox.Add(ns.Inputs.Zone(time(), 'Testvale'))
         ns.Outbox.Add(ns.Inputs.Level(time() + 10, 19))
         ns.Outbox.Add(ns.Inputs.Level(time() + 20, 20))
         ns.Outbox.Flush()",
    );
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let mut bridge = FakeBridge::new(story_with_lore(name)).with_model(Box::new(move |_| {
        counted.set(counted.get() + 1);
        None
    }));

    let reply = bridge.batch(&game.sent()[0]);

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
    calls.get()
}

#[test]
fn with_the_setting_off_the_first_batch_after_login_opens_no_model_call() {
    let with_ai_on = model_calls_of_first_batch(&Game::new(), "on");
    let with_ai_off = model_calls_of_first_batch(&game_with_setting_off(), "off");

    assert!(with_ai_on > 0);
    assert_eq!(with_ai_off, 0);
}
