//! `/timeways test`, with the real story program behind a fake bridge.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use std::path::Path;
use timeways_story::input::Input;
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const REPLY_LINES: [&str; 3] = ["lore_asked", "journal_asked", "talk_asked"];

/// The addon with the real `ns.Link`, except that `Send` keeps each text and gives its
/// number as the id.
fn game(character: bool) -> Game {
    let game = Game::with_transport();
    if character {
        game.run("ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))");
    }
    game.run(
        "sent = {}
         ns.Link.Send = function(text)
             table.insert(sent, text)
             return #sent
         end",
    );
    game
}

fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("self-test-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let tower = Passage {
        text: "The tower of Testvale fell.".to_string(),
        source: "https://example.test/1".to_string(),
        links: vec![Link::Place("Testvale".to_string())],
        origin: Origin::Pack,
    };
    Pack::write(&path, &[tower]).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

fn traveled(story: &mut Story) {
    let events = [
        r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#,
        r#"{"type":"zone_entered","at":1790000000,"zone":"Testvale","subzone":"Old Tower"}"#,
        r#"{"type":"npc_met","at":1790000000,"name":"Keeper Tessa"}"#,
    ];
    for event in events {
        assert_eq!(serve::line(story, event.as_bytes().to_vec()).error, None);
    }
}

/// The output lines of one input line. A model call fails at once, as with no model.
fn serve_line(story: &mut Story, line: String) -> Vec<String> {
    let mut output = Vec::new();
    for out in serve::line(story, line.into_bytes()).lines {
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        if value["type"] == "model_call" {
            let failed = serde_json::json!({ "type": "model_failed", "call": value["call"] });
            output.extend(serve_line(story, failed.to_string()));
        } else {
            output.push(out);
        }
    }
    output
}

/// Runs one message through the story program as the bridge does: it adds the id to the
/// line with a reply, and joins the output lines.
fn bridge(story: &mut Story, id: usize, message: &str) -> String {
    let mut output = Vec::new();
    for line in message.lines() {
        let mut value: serde_json::Value = serde_json::from_str(line).unwrap();
        if REPLY_LINES.contains(&value["type"].as_str().unwrap()) {
            value["id"] = serde_json::json!(id);
        }
        output.extend(serve_line(story, value.to_string()));
    }
    output.join("\n")
}

/// Answers each message that the addon sends, also the ones that a reply starts.
fn answer_all(game: &Game, story: &mut Story) {
    let receive: mlua::Function = game.eval("ns.Link.Receive");
    let mut answered = 0;
    loop {
        let sent: Vec<String> = game.eval("sent");
        let Some(message) = sent.get(answered) else {
            return;
        };
        answered += 1;
        let reply = bridge(story, answered, message);
        receive.call::<()>((answered, "done", reply)).unwrap();
    }
}

/// Each check of the saved report, as `name=true` or `name=false`.
fn checks(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, check in ipairs(ns.Saved().selfTest.checks) do
             table.insert(out, check.name .. '=' .. tostring(check.ok))
         end
         return out",
    )
}

fn check(name: &str, ok: bool) -> String {
    format!("{name}={ok}")
}

#[test]
fn the_self_test_passes_against_the_real_story_program() {
    let game = game(true);
    let mut story = story("passes");
    traveled(&mut story);

    game.run("wow.Slash('/timeways', 'test')");
    answer_all(&game, &mut story);

    assert_eq!(
        checks(&game),
        [
            check("client", true),
            check("journal", true),
            check("lore", true)
        ]
    );
    let printed = game.printed();
    assert!(
        printed.last().unwrap().ends_with("3 of 3 checks passed."),
        "{printed:?}"
    );
}

#[test]
fn the_self_test_asks_only_for_lines_that_change_nothing() {
    let game = game(true);
    let mut story = story("reads");

    game.run("wow.Slash('/timeways', 'test')");
    answer_all(&game, &mut story);

    let sent: Vec<String> = game.eval("sent");
    for line in sent.iter().flat_map(|message| message.lines().skip(1)) {
        let mut value: serde_json::Value = serde_json::from_str(line).unwrap();
        value["id"] = serde_json::json!(1);
        let input: Input = serde_json::from_value(value).unwrap();
        assert!(
            matches!(input, Input::JournalAsked { .. } | Input::LoreAsked { .. }),
            "{input:?}"
        );
    }
    assert_eq!(sent.len(), 2);
}

#[test]
fn a_reply_of_the_self_test_never_reaches_the_book() {
    let game = game(true);
    let mut story = story("book");
    traveled(&mut story);

    game.run("wow.Slash('/timeways', 'test')");
    answer_all(&game, &mut story);

    let first: String = game.eval("ns.Journal.Lines('deeds')[1].text");
    assert!(first.starts_with("Loading..."));
}

#[test]
fn an_error_from_the_bridge_fails_its_step_and_the_test_goes_on() {
    let game = game(true);

    game.run(
        "wow.Slash('/timeways', 'test')
         ns.Link.Receive(1, 'error', 'Timeways story program not running.')
         ns.Link.Receive(2, 'done', '{\"type\":\"lore_answer\",\"passages\":[]}')",
    );

    assert_eq!(
        checks(&game),
        [
            check("client", true),
            check("journal", false),
            check("lore", true)
        ]
    );
    let printed = game.printed().join("\n");
    assert!(
        printed.contains("Timeways story program not running."),
        "{printed}"
    );
}

#[test]
fn a_reply_without_the_expected_line_fails_its_step() {
    let game = game(true);

    game.run(
        "wow.Slash('/timeways', 'test')
         ns.Link.Receive(1, 'done', '{\"type\":\"events_seen\"}')
         ns.Link.Receive(2, 'done', 'not json')",
    );

    assert_eq!(
        checks(&game),
        [
            check("client", true),
            check("journal", false),
            check("lore", false)
        ]
    );
}

#[test]
fn before_the_login_the_steps_fail_and_nothing_goes_out() {
    let game = game(false);

    game.run("wow.Slash('/timeways', 'test')");

    assert_eq!(
        checks(&game),
        [
            check("client", true),
            check("journal", false),
            check("lore", false)
        ]
    );
    assert_eq!(game.eval::<u32>("#sent"), 0);
}

#[test]
fn a_second_start_waits_for_the_first_test_to_end() {
    let game = game(true);

    game.run(
        "wow.Slash('/timeways', 'test')
         wow.Slash('/timeways', 'test')",
    );

    assert_eq!(game.eval::<u32>("#sent"), 1);
    let printed = game.printed();
    assert!(
        printed
            .last()
            .unwrap()
            .ends_with("A self-test runs already."),
        "{printed:?}"
    );
}

#[test]
fn a_missing_client_function_fails_the_client_check() {
    let game = game(true);

    game.run(
        "Screenshot = nil
         wow.Slash('/timeways', 'test')",
    );

    let printed = game.printed().join("\n");
    assert!(printed.contains("no Screenshot"), "{printed}");
    assert_eq!(checks(&game)[0], check("client", false));
}

#[test]
fn a_step_with_no_reply_fails_after_five_minutes_and_the_test_goes_on() {
    let game = game(true);
    game.run("wow.Slash('/timeways', 'test')");

    game.run("for _, timer in ipairs(wow.after) do timer.callback() end");
    game.run("for _, timer in ipairs(wow.after) do timer.callback() end");

    assert_eq!(
        checks(&game),
        [
            check("client", true),
            check("journal", false),
            check("lore", false)
        ]
    );
}

#[test]
fn a_reply_after_its_step_timed_out_changes_nothing() {
    let game = game(true);
    game.run(
        "wow.Slash('/timeways', 'test')
         for _, timer in ipairs(wow.after) do timer.callback() end",
    );

    game.run("ns.Link.Receive(1, 'done', '{\"type\":\"journal\",\"page\":0,\"pages\":1}')");

    assert_eq!(checks(&game)[1], check("journal", false));
}
