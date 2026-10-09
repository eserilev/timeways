//! `/twdev smoke`: one command runs many features through the real addon, the real checks
//! of the bridge, and the real story program, and the desktop writes one log of the run
//! (TESTING.md, "One command to test everything").

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use fake_bridge::{FakeBridge, Reply};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use timeways_story::dev_mode::DevMode;
use timeways_story::dev_smoke::newest_log;
use timeways_story::moments::KINDS;
use timeways_story::narrator::Who;
use timeways_story::narrator_review::{Sources, reviews};
use timeways_story::pack::Pack;
use timeways_story::store::{CharacterKey, Store};
use timeways_story::story::Story;

/// The run stops by itself after 20 minutes, so this many seconds always end it.
const MOST_SECONDS: usize = 21 * 60;

/// The model answers each call as a model does: a line, a talk, a quest, or lore.
fn model(prompt: &str) -> String {
    if prompt.contains("Write chapter") {
        r#"{"saga": "The road led to Goldshire.", "footnotes": []}"#.to_string()
    } else if prompt.contains("small task of your own") {
        r#"{"title": "The Lost Lantern", "genre": "errand", "text": "I lost it. Find it.", "steps": [{"goal": "visit", "place": "Goldshire"}]}"#.to_string()
    } else if prompt.contains("A player speaks to you") && prompt.contains("Any work for me?") {
        r#"{"say": "I could use a hand.", "trust": 1, "work": true}"#.to_string()
    } else if prompt.contains("A player speaks to you") {
        r#"{"say": "Well met. The roads are quiet.", "trust": 2}"#.to_string()
    } else {
        "Nobody knows.".to_string()
    }
}

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("smoke-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn bridge(folder: &Path, mode: DevMode) -> FakeBridge {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.to_path_buf()));
    story.set_dev_mode(mode);
    FakeBridge::new(story).with_model(Box::new(|prompt| Some(model(prompt))))
}

/// The addon after the login, with each message of its link kept for the bridge.
fn game() -> Game {
    let game = Game::with_transport();
    game.run(
        "sent = {}
         ns.Link.Send = function(text)
             table.insert(sent, text)
             return #sent
         end
         wow.units.player = { name = 'Ada', level = 12, player = true }
         wow.zone, wow.subzone = 'Elwynn Forest', 'Goldshire'
         wow.Fire('PLAYER_ENTERING_WORLD')",
    );
    game
}

/// Each message that the addon sent and the bridge did not take yet goes to the bridge,
/// and its reply back to the addon.
struct Desk {
    bridge: FakeBridge,
    done: usize,
}

impl Desk {
    fn pump(&mut self, game: &Game) {
        let receive: mlua::Function = game.eval("ns.Link.Receive");
        loop {
            let sent: Vec<String> = game.eval("sent");
            let Some(text) = sent.get(self.done) else {
                return;
            };
            self.done += 1;
            match self.bridge.batch(text) {
                Reply::Done(reply) => receive.call::<()>((self.done, "done", reply)).unwrap(),
                Reply::Error(error) => panic!("the bridge refused a batch: {error}\n{text}"),
            }
        }
    }
}

/// Which runs first in a second of the game. The game gives no order, so the run must
/// work with both.
#[derive(Clone, Copy)]
enum Order {
    TimersFirst,
    TickersFirst,
}

const RUN_TIMERS: &str = "local due, later = {}, {}
     for _, timer in ipairs(wow.after) do
         if timer.at <= wow.now then due[#due + 1] = timer else later[#later + 1] = timer end
     end
     wow.after = later
     for _, timer in ipairs(due) do timer.callback() end";

/// One second of the game: its timers, its tickers, and the replies of the desktop.
fn second(game: &Game, desk: &mut Desk) {
    second_in(game, desk, Order::TimersFirst);
}

fn second_in(game: &Game, desk: &mut Desk, order: Order) {
    game.run("wow.now = wow.now + 1");
    match order {
        Order::TimersFirst => {
            game.run(RUN_TIMERS);
            game.run("wow.RunTickers()");
        }
        Order::TickersFirst => {
            game.run("wow.RunTickers()");
            game.run(RUN_TIMERS);
        }
    }
    desk.pump(game);
}

fn twdev(game: &Game, message: &str) {
    let slash: mlua::Function = game.eval("return function(m) wow.Slash('/twdev', m) end");
    slash.call::<()>(message).unwrap();
}

/// The game with dev mode on at the desktop: the first journal carries the mark.
fn dev_session(name: &str) -> (Game, Desk, PathBuf) {
    let folder = folder(name);
    let game = game();
    let mut desk = Desk {
        bridge: bridge(&folder, DevMode::On),
        done: 0,
    };
    game.run("ns.Journal.Request(0)");
    desk.pump(&game);
    assert!(game.eval::<bool>("return ns.Dev.IsOn()"));
    (game, desk, folder)
}

fn run_to_the_end(game: &Game, desk: &mut Desk) {
    run_to_the_end_in(game, desk, Order::TimersFirst);
}

fn run_to_the_end_in(game: &Game, desk: &mut Desk, order: Order) {
    twdev(game, "smoke");
    for _ in 0..MOST_SECONDS {
        if !game.eval::<bool>("return ns.DevSmoke.IsRunning()") {
            return;
        }
        second_in(game, desk, order);
    }
    panic!("the smoke test never ended");
}

/// The verdict of the game and the name of each step in the log: "PASS   3 capital".
fn results(log: &str) -> Vec<(String, String)> {
    let step = |line: &str| {
        let words: Vec<&str> = line.split_whitespace().collect();
        (words[0].to_string(), words[2].to_string())
    };
    log.lines()
        .filter(|line| ["PASS", "FAIL", "WAIT"].iter().any(|v| line.starts_with(v)))
        .map(step)
        .collect()
}

fn result_of(log: &str, step: &str) -> String {
    let Some((result, _)) = results(log).into_iter().find(|(_, name)| name == step) else {
        panic!("no step {step} in\n{log}");
    };
    result
}

fn the_log(folder: &Path) -> String {
    std::fs::read_to_string(newest_log(folder).expect("no smoke log")).unwrap()
}

/// The steps that need no model, or that the model of this test answers.
const WORKING: [&str; 30] = [
    "start",
    "subzone-move",
    "rare-kill",
    "lore-before-kill",
    "lore-after-kill",
    "level-up-again",
    "quest-step-kill",
    "meet-npc",
    "gossip",
    "book",
    "talk",
    "peer-story-accept",
    "peer-story-decline",
    "peer-quest-accept",
    "peer-quest-decline",
    "peer-full",
    "peer-blocked",
    "story-scroll-cursor",
    "quest-form-cursor",
    "journal",
    "tab-hero",
    "tab-chronicle",
    "tab-stories",
    "tab-knowledge",
    "tab-quests",
    "atlas",
    "atlas-empty",
    "welcome-setup",
    "welcome-offline",
    "fps",
];

#[test]
fn the_smoke_test_runs_every_step_and_the_desktop_logs_each_one() {
    let (game, mut desk, folder) = dev_session("all");

    run_to_the_end(&game, &mut desk);

    let log = the_log(&folder);
    let steps: Vec<String> = game.eval("ns.DevSmoke.Steps()");
    assert_eq!(results(&log).len(), steps.len(), "{log}");
    for step in WORKING {
        assert_eq!(result_of(&log, step), "PASS", "{step}\n{log}");
    }
    assert!(!log.contains("desk FAIL"), "{log}");
    assert!(log.contains("Summary: "), "{log}");
    assert!(log.contains("Lua errors: none"), "{log}");
    assert!(log.contains("median 60, mean 60."), "{log}");
    assert_eq!(desk.bridge.dropped_lines(), 0);
    let printed = game.printed().join("\n");
    assert!(printed.contains("Smoke test done:"), "{printed}");
}

#[test]
fn each_room_step_waits_for_its_own_answer_also_when_the_ticker_runs_first() {
    let (game, mut desk, folder) = dev_session("room-order");

    run_to_the_end_in(&game, &mut desk, Order::TickersFirst);

    let log = the_log(&folder);
    assert_eq!(result_of(&log, "peer-full"), "PASS", "{log}");
    assert_eq!(result_of(&log, "peer-blocked"), "PASS", "{log}");
}

#[test]
fn a_run_on_a_world_with_a_kill_from_before_says_in_the_chat_that_it_is_not_fresh() {
    let (game, mut desk, _) = dev_session("not-fresh");
    twdev(&game, "kill Edwin VanCleef boss");
    game.run("ns.Outbox.Flush()");
    desk.pump(&game);

    run_to_the_end(&game, &mut desk);

    let printed = game.printed().join("\n");
    assert_eq!(
        printed.matches("This world isn't fresh").count(),
        1,
        "{printed}"
    );
}

#[test]
fn a_feature_broken_on_purpose_fails_its_step_and_the_run_goes_on() {
    let (game, mut desk, folder) = dev_session("broken");
    game.run(
        "ns.Watch.Level = function() error('broken on purpose') end
         ns.TalkWindow.IsShown = function() return false end",
    );

    run_to_the_end(&game, &mut desk);

    let log = the_log(&folder);
    assert_eq!(result_of(&log, "level-up"), "FAIL", "{log}");
    assert!(log.contains("broken on purpose"), "{log}");
    assert!(log.contains("desk FAIL: no level_reached landed"), "{log}");
    assert_eq!(result_of(&log, "talk"), "FAIL", "{log}");
    assert!(log.contains("the talk window isn't open"), "{log}");
    assert_eq!(result_of(&log, "book"), "PASS", "{log}");
    assert!(log.contains("Lua errors: 2"), "{log}");
}

#[test]
fn the_smoke_test_does_nothing_while_dev_mode_is_off() {
    let folder = folder("off");
    let game = game();
    let mut desk = Desk {
        bridge: bridge(&folder, DevMode::Off),
        done: 0,
    };
    game.run("ns.Journal.Request(0)");
    desk.pump(&game);

    twdev(&game, "smoke");
    for _ in 0..30 {
        second(&game, &mut desk);
    }

    assert!(!game.eval::<bool>("return ns.DevSmoke.IsRunning()"));
    let printed = game.printed().join("\n");
    assert!(printed.contains("Dev mode is off."), "{printed}");
    let sent: Vec<String> = game.eval("sent");
    assert!(
        !sent.iter().any(|text| text.contains("dev_smoke")),
        "{sent:?}"
    );
    assert_eq!(newest_log(&folder), None);
}

/// The quick harness: a bridge that answers each batch with nothing, and dev mode on.
fn quick_dev_game() -> Game {
    let game = Game::new();
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_dev_mode(DevMode::On);
    let character = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
    let _ = timeways_story::serve::line(&mut story, character.as_bytes().to_vec());
    let ask = r#"{"type":"journal_asked","id":1}"#;
    let journal = timeways_story::serve::line(&mut story, ask.as_bytes().to_vec())
        .lines
        .remove(0);
    game.reply(&journal);
    game
}

fn quick_second(game: &Game) {
    game.run(
        "wow.now = wow.now + 1
         local due, later = {}, {}
         for _, timer in ipairs(wow.after) do
             if timer.at <= wow.now then due[#due + 1] = timer else later[#later + 1] = timer end
         end
         wow.after = later
         for _, timer in ipairs(due) do timer.callback() end
         wow.RunTickers()",
    );
}

/// The smoke lines that went to the desktop, as JSON.
fn smoke_lines(game: &Game, kind: &str) -> Vec<Value> {
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
        .filter(|line| line["type"] == kind)
        .collect()
}

#[test]
fn the_run_waits_while_you_fight_and_stops_by_itself_after_its_most_time() {
    let game = quick_dev_game();
    twdev(&game, "smoke");
    let before: usize = game.eval("#ns.DevSmoke.Results()");
    game.run("wow.combat = true");

    for _ in 0..60 {
        quick_second(&game);
    }
    let in_combat: usize = game.eval("#ns.DevSmoke.Results()");
    for _ in 0..MOST_SECONDS {
        quick_second(&game);
    }

    assert_eq!(in_combat, before);
    assert!(game.eval::<bool>("return wow.focus == nil"));
    assert!(!game.eval::<bool>("return ns.DevSmoke.IsRunning()"));
    let done = smoke_lines(&game, "dev_smoke_done");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0]["stopped"], "timeout");
    assert_eq!(done[0]["dev"], true);
}

#[test]
fn stop_ends_the_run_with_its_summary_and_the_errors_of_the_game() {
    let game = quick_dev_game();
    game.run("handlerBefore = geterrorhandler()");
    twdev(&game, "smoke");
    game.run("wow.ScriptError('Interface/AddOns/Timeways/Ink.lua:3: oops')");

    twdev(&game, "smoke stop");
    quick_second(&game);

    let done = smoke_lines(&game, "dev_smoke_done");
    assert_eq!(done.len(), 1);
    assert_eq!(done[0]["stopped"], "stopped");
    assert_eq!(
        done[0]["errors"],
        serde_json::json!(["Interface/AddOns/Timeways/Ink.lua:3: oops"])
    );
    assert!(game.eval::<bool>("return geterrorhandler() == handlerBefore"));
    let steps = smoke_lines(&game, "dev_smoke");
    assert_eq!(steps[0]["name"], "start");
    assert_eq!(steps[0]["dev"], true);
}

#[test]
fn every_line_of_the_run_reads_as_the_story_program_reads_it() {
    let game = quick_dev_game();
    twdev(&game, "smoke");
    for _ in 0..40 {
        quick_second(&game);
    }
    twdev(&game, "smoke stop");

    let inputs = game.sent_inputs();
    let smoke = inputs.iter().filter(|input| {
        matches!(
            input,
            timeways_story::input::Input::DevSmoke(_)
                | timeways_story::input::Input::DevSmokeDone(_)
        )
    });
    assert!(smoke.count() > 3);
    for input in &inputs {
        if let timeways_story::input::Input::DevSmoke(step) = input {
            assert!(step.is_sane(), "{step:?}");
        }
    }
}

// The smoke run grows with the features --------------------------------------------------

#[test]
fn every_command_of_twdev_has_a_smoke_step_or_a_reason_to_stay_out() {
    let game = quick_dev_game();
    let usages: Vec<String> = game.eval("ns.Dev.Usages()");
    let used: Vec<String> = game.eval("ns.DevSmoke.Commands()");
    let left_out: Vec<String> = game.eval(
        "local names = {}
         for name in pairs(ns.DevSmoke.LEFT_OUT) do names[#names + 1] = name end
         return names",
    );

    for usage in usages {
        let command = usage.split([' ', ':']).next().unwrap().to_string();
        let in_smoke = used.contains(&command);
        let out = left_out.contains(&command);
        assert!(
            in_smoke || out,
            "/twdev {command} has no smoke step: add one to DevSmoke.lua, or a reason to LEFT_OUT"
        );
        assert!(
            !(in_smoke && out),
            "/twdev {command} is in a step and in LEFT_OUT"
        );
    }
}

#[test]
fn every_kind_of_narrator_moment_has_a_smoke_step_or_a_reason_to_stay_out() {
    let (game, mut desk, folder) = dev_session("moments");
    run_to_the_end(&game, &mut desk);
    let key = CharacterKey::new("Stormrage", "Ada").unwrap();
    let opened = Store::Folder(folder).open(&key).unwrap();
    let events: Vec<_> = opened.character.world().history().iter().cloned().collect();
    let pack = Pack::empty().unwrap();
    let sources = Sources {
        pack: &pack,
        reads: &[],
        fallback: &Who::default(),
    };
    let mut made: BTreeSet<&str> = reviews(&events, &sources)
        .unwrap()
        .iter()
        .map(|review| review.moment.kind_name())
        .collect();
    // A flavor moment comes from the score of an emote or a book, never from an event of
    // the world, so the review finds none. The emote and book steps make it.
    made.insert("flavor");
    let left_out: Vec<String> = game.eval(
        "local names = {}
         for name in pairs(ns.DevSmoke.LEFT_OUT_MOMENTS) do names[#names + 1] = name end
         return names",
    );

    for kind in KINDS {
        let out = left_out.iter().any(|name| name == kind);
        assert!(
            made.contains(kind) || out,
            "no smoke step makes a {kind} moment: add one to DevSmoke.lua, or a reason to LEFT_OUT_MOMENTS"
        );
        assert!(
            !(made.contains(kind) && out),
            "{kind} is made and in LEFT_OUT_MOMENTS"
        );
    }
}
