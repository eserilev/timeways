//! `/twdev fps`: the frame rate once a second, sent to the desktop as one dev line at the
//! stop (TESTING.md, "Testing the local model and the frame rate").

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use serde_json::Value;
use timeways_story::dev_mode::DevMode;
use timeways_story::input::Input;
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

fn journal_line(mode: DevMode) -> String {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_dev_mode(mode);
    let character = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
    let _ = serve::line(&mut story, character.as_bytes().to_vec());
    let ask = r#"{"type":"journal_asked","id":1}"#;
    serve::line(&mut story, ask.as_bytes().to_vec())
        .lines
        .remove(0)
}

fn dev_game() -> Game {
    let game = Game::new();
    game.reply(&journal_line(DevMode::On));
    game
}

fn twdev(game: &Game, message: &str) {
    let slash: mlua::Function = game.eval("return function(m) wow.Slash('/twdev', m) end");
    slash.call::<()>(message).unwrap();
}

/// One tick of every ticker at this frame rate: the FPS run takes one sample.
fn tick_at(game: &Game, fps: u32) {
    game.run(&format!(
        "wow.framerate = {fps} wow.now = wow.now + 1 wow.RunTickers()"
    ));
}

/// The FPS lines that went to the desktop.
fn fps_lines(game: &Game) -> Vec<Value> {
    game.sent()
        .iter()
        .flat_map(|batch| {
            batch
                .lines()
                .skip(1)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .map(|line| serde_json::from_str::<Value>(&line).unwrap())
        .filter(|line| line["type"] == "dev_fps")
        .collect()
}

fn last_printed(game: &Game) -> String {
    game.printed().last().cloned().unwrap_or_default()
}

#[test]
fn a_run_sends_one_dev_line_with_its_samples_and_their_numbers() {
    let game = dev_game();
    game.run("wow.framerate = 60");
    let started: u64 = game.eval("wow.now");

    twdev(&game, "fps start bench");
    tick_at(&game, 30);
    tick_at(&game, 45);
    twdev(&game, "fps stop");

    let lines = fps_lines(&game);
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(line["label"], "bench");
    assert_eq!(line["dev"], true);
    assert_eq!(line["started"], started);
    assert_eq!(line["ended"], started + 2);
    assert_eq!(line["samples"], serde_json::json!([60, 30, 45]));
    assert_eq!(
        (&line["min"], &line["p5"], &line["median"], &line["mean"]),
        (&30.into(), &32.into(), &45.into(), &45.into())
    );
    assert_eq!(line["hidden"], 0);
}

const BACKGROUND_HINT: &str = "the cap of maxFPSBk while the game window is in the background";

#[test]
fn a_run_that_sat_at_the_background_cap_says_so_and_gives_the_cap() {
    let game = dev_game();
    game.run("wow.framerate = 30 wow.cvars.maxFPSBk = '30'");

    twdev(&game, "fps start bench");
    tick_at(&game, 30);
    tick_at(&game, 30);
    let cap: Option<u32> = game.eval("return ns.DevFps.Stop().background_cap");

    assert_eq!(cap, Some(30));
    let printed = game.printed().join("\n");
    assert!(printed.contains(BACKGROUND_HINT), "{printed}");
}

#[test]
fn a_run_above_the_background_cap_says_nothing_of_it() {
    let game = dev_game();
    game.run("wow.framerate = 60 wow.cvars.maxFPSBk = '30'");

    twdev(&game, "fps start bench");
    tick_at(&game, 30);
    let cap: Option<u32> = game.eval("return ns.DevFps.Stop().background_cap");

    assert_eq!(cap, None);
    assert!(!game.printed().join("\n").contains(BACKGROUND_HINT));
}

#[test]
fn a_run_with_no_background_cap_says_nothing_of_it() {
    let game = dev_game();
    game.run("wow.framerate = 30 wow.cvars.maxFPSBk = '0'");

    twdev(&game, "fps start bench");
    tick_at(&game, 30);
    let cap: Option<u32> = game.eval("return ns.DevFps.Stop().background_cap");

    assert_eq!(cap, None);
}

#[test]
fn the_line_reads_as_the_story_program_reads_it() {
    let game = dev_game();

    twdev(&game, "fps start");
    twdev(&game, "fps stop");

    let runs: Vec<Input> = game
        .sent_inputs()
        .into_iter()
        .filter(|input| matches!(input, Input::DevFps(_)))
        .collect();
    let [Input::DevFps(run)] = runs.as_slice() else {
        panic!("{runs:?}")
    };
    assert_eq!(run.label, "fps");
    assert!(run.is_sane());
}

#[test]
fn a_hidden_frame_rate_counts_as_hidden_and_in_no_number() {
    let game = dev_game();
    game.run("wow.framerate = 60 wow.secrets[20] = true");

    twdev(&game, "fps start bench");
    tick_at(&game, 20);
    tick_at(&game, 50);
    twdev(&game, "fps stop");

    let line = &fps_lines(&game)[0];
    assert_eq!(line["samples"], serde_json::json!([60, 50]));
    assert_eq!(line["hidden"], 1);
    assert_eq!(line["min"], 50);
}

#[test]
fn a_run_of_hidden_samples_alone_sends_nothing() {
    let game = dev_game();
    game.run("wow.framerate = 20 wow.secrets[20] = true");

    twdev(&game, "fps start bench");
    twdev(&game, "fps stop");

    assert!(fps_lines(&game).is_empty());
    assert!(
        last_printed(&game).contains("no samples"),
        "{}",
        last_printed(&game)
    );
}

#[test]
fn the_numbers_of_zero_one_and_two_samples() {
    let game = dev_game();

    let none: bool = game.eval("ns.DevFps.Summary({}) == nil");
    let one: Vec<f64> =
        game.eval("local s = ns.DevFps.Summary({ 42 }) return { s.min, s.p5, s.median, s.mean }");
    let two: Vec<f64> = game
        .eval("local s = ns.DevFps.Summary({ 60, 40 }) return { s.min, s.p5, s.median, s.mean }");

    assert!(none);
    assert_eq!(one, [42.0, 42.0, 42.0, 42.0]);
    assert_eq!(two, [40.0, 41.0, 50.0, 50.0]);
}

#[test]
fn a_long_run_sends_at_most_240_samples_as_the_means_of_groups() {
    let game = dev_game();

    let sent: Vec<u32> = game.eval(
        "local samples = {} for i = 1, 480 do samples[i] = i % 2 == 0 and 60 or 30 end \
         return ns.DevFps.Sent(samples)",
    );

    assert_eq!(sent.len(), 240);
    assert!(sent.iter().all(|fps| *fps == 45), "{sent:?}");
}

#[test]
fn a_run_stops_by_itself_after_15_minutes_and_its_line_fits_one_message() {
    let game = dev_game();

    twdev(&game, "fps start bench");
    game.run("for i = 1, 900 do wow.now = wow.now + 1 wow.RunTickers() end");

    let lines = fps_lines(&game);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["dev"], true);
    assert!(lines[0]["samples"].as_array().unwrap().len() <= 240);
    let stopped: bool = game.eval("not ns.DevFps.IsRunning()");
    assert!(stopped);
    let printed = game.printed().join("\n");
    assert!(printed.contains("reached 15 minutes"), "{printed}");
}

#[test]
fn the_line_holds_the_memory_and_with_profiling_the_cpu_of_timeways() {
    let game = dev_game();
    game.run(
        "wow.addonMemory.Timeways = 812.4 wow.cvars.scriptProfile = '1' \
         wow.addonCpu.Timeways = 100",
    );

    twdev(&game, "fps start bench");
    game.run("wow.addonCpu.Timeways = 135.2");
    twdev(&game, "fps stop");

    let line = &fps_lines(&game)[0];
    assert_eq!(line["memory_kb"], 812);
    assert_eq!(line["cpu_ms"], 35);
    assert!(last_printed(&game).contains("35 ms of CPU"));
}

#[test]
fn without_profiling_the_line_has_no_cpu_and_the_chat_says_how_to_turn_it_on() {
    let game = dev_game();

    twdev(&game, "fps start bench");
    twdev(&game, "fps stop");

    let line = &fps_lines(&game)[0];
    assert_eq!(line.get("cpu_ms"), None);
    assert!(last_printed(&game).contains("/console scriptProfile 1"));
}

#[test]
fn a_hidden_memory_or_cpu_stays_out_of_the_line() {
    let game = dev_game();
    game.run(
        "wow.addonMemory.Timeways = 812 wow.secrets[812] = true wow.cvars.scriptProfile = '1' \
         wow.addonCpu.Timeways = 99 wow.secrets[99] = true",
    );

    twdev(&game, "fps start bench");
    twdev(&game, "fps stop");

    let line = &fps_lines(&game)[0];
    assert_eq!(line.get("memory_kb"), None);
    assert_eq!(line.get("cpu_ms"), None);
}

#[test]
fn a_second_start_keeps_the_first_run() {
    let game = dev_game();

    twdev(&game, "fps start first");
    twdev(&game, "fps start second");
    twdev(&game, "fps stop");

    assert_eq!(fps_lines(&game)[0]["label"], "first");
}

#[test]
fn a_stop_with_no_run_and_a_bad_label_send_nothing() {
    let game = dev_game();

    twdev(&game, "fps stop");
    let no_run = last_printed(&game);
    twdev(&game, "fps start a/b");
    let bad_label = last_printed(&game);

    assert!(no_run.contains("no FPS run"), "{no_run}");
    assert!(bad_label.contains("usage"), "{bad_label}");
    assert!(fps_lines(&game).is_empty());
}

#[test]
fn a_run_ends_with_no_line_when_dev_mode_turns_off() {
    let game = dev_game();

    twdev(&game, "fps start bench");
    game.reply(&journal_line(DevMode::Off));
    tick_at(&game, 60);

    let stopped: bool = game.eval("not ns.DevFps.IsRunning()");
    assert!(stopped);
    assert!(fps_lines(&game).is_empty());
}
