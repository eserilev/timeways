//! `/twdev smoke`: the story program writes each step of the run into one log of the dev
//! folder, with the check of the desktop, and a summary at the end. It does so only while
//! dev mode is on, and the lines change no world (TESTING.md, "One command to test
//! everything").

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::{FakeBridge, Reply};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use timeways_story::dev_mode::DevMode;
use timeways_story::dev_smoke::{
    self, CallSeen, CallTally, Desk, DeskVerdict, FpsNumbers, Logged, MAX_LOG_BYTES, MAX_STEPS,
    MAX_TEXT_BYTES, SmokeDone, SmokeStep, Stopped, Verdict, json_file, log_file, newest_log,
};
use timeways_story::pack::{Dependency, Link, Origin, Pack, Passage};
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
const RUN: u64 = 1_790_000_000;

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-smoke-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn story_with(folder: &Path, mode: DevMode, pack: Pack) -> Story {
    let mut story = Story::new(pack, Store::Folder(folder.to_path_buf()));
    story.set_dev_mode(mode);
    let _ = serve::line(&mut story, CHARACTER.as_bytes().to_vec());
    story
}

fn story(folder: &Path, mode: DevMode) -> Story {
    story_with(folder, mode, Pack::empty().unwrap())
}

fn step_line(step: u32, name: &str, result: &str, more: &Value) -> String {
    let mut line = json!({
        "type": "dev_smoke", "run": RUN, "step": step, "name": name, "result": result,
        "dev": true
    });
    for (key, value) in more.as_object().unwrap() {
        line[key] = value.clone();
    }
    line.to_string()
}

fn done_line(steps: u32) -> String {
    json!({
        "type": "dev_smoke_done", "run": RUN, "steps": steps, "passed": steps, "failed": 0,
        "waited": 0, "seconds": 42, "stopped": "done",
        "errors": ["Interface/AddOns/Timeways/Talk.lua:12: attempt to index a nil value"],
        "fps": { "samples": 10, "hidden": 0, "min": 55, "p5": 56, "median": 60, "mean": 59 },
        "dev": true
    })
    .to_string()
}

fn send(story: &mut Story, line: &str) -> serve::Served {
    serve::line(story, line.as_bytes().to_vec())
}

fn zone_line(at: u64, zone: &str) -> String {
    json!({ "type": "zone_entered", "at": at, "zone": zone, "subzone": null, "dev": true })
        .to_string()
}

fn log_text(folder: &Path) -> String {
    std::fs::read_to_string(log_file(folder, RUN)).unwrap_or_default()
}

#[test]
fn a_run_writes_two_lines_for_each_step_and_a_summary_at_its_end() {
    let folder = folder("run");
    let mut story = story(&folder, DevMode::On);

    send(&mut story, &step_line(0, "start", "pass", &json!({})));
    send(&mut story, &zone_line(RUN, "Elwynn Forest"));
    let zone = json!({ "reason": "Elwynn Forest", "expect": ["zone_entered"] });
    send(&mut story, &step_line(1, "zone-move", "pass", &zone));
    let served = send(&mut story, &done_line(2));

    assert_eq!(served.error, None);
    let log = log_text(&folder);
    assert!(
        log.starts_with("Timeways smoke test, run 1790000000"),
        "{log}"
    );
    assert!(log.contains("PASS   1 zone-move"), "{log}");
    assert!(
        log.contains("desk PASS: kept zone_entered; facts +3; no calls"),
        "{log}"
    );
    assert!(log.contains("Summary: 2 steps in 42 s (every step ran)."));
    assert!(log.contains("Game: 2 passed, 0 failed, 0 waited."));
    assert!(log.contains("Lua errors: 1\n  Interface/AddOns/Timeways/Talk.lua:12"));
    assert!(log.contains("FPS: 10 samples (0 hidden): min 55, low 5% 56, median 60, mean 59."));
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(json_file(&folder, RUN)).unwrap()).unwrap();
    assert_eq!(report["steps"].as_array().unwrap().len(), 2);
    assert_eq!(report["steps"][1]["desk"]["kept"]["zone_entered"], 1);
    assert_eq!(report["done"]["stopped"], "done");
}

#[test]
fn a_step_whose_line_never_landed_fails_on_the_desk() {
    let folder = folder("missing");
    let mut story = story(&folder, DevMode::On);
    send(&mut story, &step_line(0, "start", "pass", &json!({})));

    let level = json!({ "expect": ["level_reached"] });
    send(&mut story, &step_line(1, "level-10", "pass", &level));

    let log = log_text(&folder);
    assert!(
        log.contains("desk FAIL: no level_reached landed; kept nothing"),
        "{log}"
    );
}

#[test]
fn the_lines_of_a_smoke_run_are_refused_while_dev_mode_is_off() {
    let folder = folder("off");
    let mut story = story(&folder, DevMode::Off);

    let step = send(&mut story, &step_line(0, "start", "pass", &json!({})));
    let unmarked = step_line(0, "start", "pass", &json!({})).replace(r#","dev":true"#, "");
    let plain = send(&mut story, &unmarked);
    let done = send(&mut story, &done_line(1));

    assert!(
        step.error
            .is_some_and(|error| error.contains("dev mode is off"))
    );
    assert!(
        plain
            .error
            .is_some_and(|error| error.contains("dev mode is off"))
    );
    assert!(done.error.is_some());
    assert!(!log_file(&folder, RUN).exists());
}

#[test]
fn a_step_with_a_bad_name_or_a_long_text_is_refused() {
    let folder = folder("bad");
    let mut story = story(&folder, DevMode::On);
    let long = "x".repeat(MAX_TEXT_BYTES + 1);

    for line in [
        step_line(0, "", "pass", &json!({})),
        step_line(0, "Zone Move", "pass", &json!({})),
        step_line(0, &"x".repeat(41), "pass", &json!({})),
        step_line(0, "start", "maybe", &json!({})),
        step_line(MAX_STEPS + 1, "start", "pass", &json!({})),
        step_line(0, "start", "pass", &json!({ "text": long })),
        step_line(0, "start", "pass", &json!({ "reason": "two\nlines" })),
        step_line(0, "start", "pass", &json!({ "expect": ["Zone Entered"] })),
    ] {
        let served = send(&mut story, &line);
        assert!(served.error.is_some(), "{line}");
    }
    assert!(!log_file(&folder, RUN).exists());
}

#[test]
fn the_end_of_a_run_with_no_steps_here_is_refused() {
    let folder = folder("no-run");
    let mut story = story(&folder, DevMode::On);

    let served = send(&mut story, &done_line(3));

    assert!(served.error.is_some_and(|error| error.contains("no steps")));
}

#[test]
fn the_lines_of_a_smoke_run_change_no_world() {
    let folder = folder("no-world");
    let mut story = story(&folder, DevMode::On);
    let ask = r#"{"type":"journal_asked","id":2}"#;
    let before = send(&mut story, ask).lines;

    send(&mut story, &step_line(0, "start", "pass", &json!({})));
    send(&mut story, &done_line(1));

    assert_eq!(send(&mut story, ask).lines, before);
}

#[test]
fn a_run_takes_its_most_steps_and_refuses_one_more() {
    let folder = folder("most");
    let mut story = story(&folder, DevMode::On);

    for step in 0..MAX_STEPS {
        let served = send(&mut story, &step_line(step, "step", "pass", &json!({})));
        assert_eq!(served.error, None, "{step}");
    }
    let one_more = send(
        &mut story,
        &step_line(MAX_STEPS, "step", "pass", &json!({})),
    );

    assert!(one_more.error.is_some());
}

#[test]
fn a_step_of_the_largest_size_fits_one_line_of_the_bridge() {
    let folder = folder("largest");
    let mut bridge = FakeBridge::new(story(&folder, DevMode::On));
    let largest = json!({
        "reason": "r".repeat(160),
        "text": "é".repeat(MAX_TEXT_BYTES / 2),
        "expect": ["a", "b", "c", "d", "e", "f"],
        "gate": { "subject": "s".repeat(80), "expect": "blocked" },
    });
    let name = "n".repeat(40);

    let reply = bridge.batch(&format!(
        "{CHARACTER}\n{}",
        step_line(MAX_STEPS, &name, "wait", &largest)
    ));

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
    assert_eq!(bridge.dropped_lines(), 0);
    assert!(bridge.errors().is_empty(), "{:?}", bridge.errors());
    assert!(log_text(&folder).contains(&format!("WAIT 120 {name}")));
}

fn outcome(text: &str) -> Passage {
    Passage {
        text: text.to_string(),
        source: "the wiki page \"Edwin VanCleef\"".to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: Some("Edwin VanCleef".to_string()),
        depends_on: vec![Dependency::Foe("Edwin VanCleef".to_string())],
        setup_for: None,
    }
}

fn pack_with_his_end(name: &str) -> Pack {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-smoke-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let end = "Adventurers killed Edwin VanCleef on the deck of his ship in the Deadmines.";
    Pack::write(&path, &[outcome(end)]).unwrap();
    Pack::open(&path).unwrap()
}

#[test]
fn the_gate_keeps_the_lore_of_a_kill_shut_before_it_and_opens_it_after() {
    let folder = folder("gate");
    let mut story = story_with(&folder, DevMode::On, pack_with_his_end("gate"));
    let gate = |expect: &str| json!({ "gate": { "subject": "Edwin VanCleef", "expect": expect } });
    send(&mut story, &step_line(0, "start", "pass", &json!({})));

    send(
        &mut story,
        &step_line(1, "lore-before-kill", "pass", &gate("blocked")),
    );
    let kill = json!({
        "type": "npc_defeated", "at": RUN, "name": "Edwin VanCleef", "kind": "boss", "dev": true
    });
    send(&mut story, &kill.to_string());
    send(
        &mut story,
        &step_line(2, "lore-after-kill", "pass", &gate("open")),
    );

    let log = log_text(&folder);
    assert!(
        log.contains(
            "desk PASS: kept nothing; facts +0; no calls; gate Edwin VanCleef: 0 open, 1 blocked"
        ),
        "{log}"
    );
    assert!(
        log.contains(
            "kept npc_defeated; facts +2; no calls; gate Edwin VanCleef: 1 open, 0 blocked"
        ),
        "{log}"
    );
}

#[test]
fn the_gate_fails_on_lore_that_shows_before_the_kill() {
    let folder = folder("gate-early");
    let mut story = story_with(&folder, DevMode::On, pack_with_his_end("gate-early"));
    let kill = json!({
        "type": "npc_defeated", "at": RUN, "name": "Edwin VanCleef", "kind": "boss", "dev": true
    });
    send(&mut story, &kill.to_string());
    let gate = json!({ "gate": { "subject": "Edwin VanCleef", "expect": "blocked" } });

    send(&mut story, &step_line(1, "lore-before-kill", "pass", &gate));

    let log = log_text(&folder);
    assert!(
        log.contains("desk FAIL: lore of the defeat of Edwin VanCleef shows before it"),
        "{log}"
    );
}

#[test]
fn a_gate_with_no_lore_in_the_pack_waits() {
    let folder = folder("gate-empty");
    let mut story = story(&folder, DevMode::On);
    let gate = json!({ "gate": { "subject": "Edwin VanCleef", "expect": "open" } });

    send(&mut story, &step_line(1, "lore-after-kill", "pass", &gate));

    assert!(
        log_text(&folder).contains(
            "desk WAIT: the pack has no lore that waits for the defeat of Edwin VanCleef"
        )
    );
}

fn logged(step: u32, name: &str, result: Verdict, desk: Verdict) -> Logged {
    Logged {
        step: SmokeStep {
            run: RUN,
            step,
            name: name.to_string(),
            result,
            reason: None,
            text: Some("Edwin VanCleef swore that Stormwind would pay.".to_string()),
            expect: Vec::new(),
            gate: None,
        },
        desk: Desk {
            calls: vec![
                CallSeen {
                    kind: "narrator".to_string(),
                    result: "refused".to_string(),
                    retry_of: None,
                },
                CallSeen {
                    kind: "narrator".to_string(),
                    result: "accepted".to_string(),
                    retry_of: Some("The line names nothing of the moment.".to_string()),
                },
            ],
            ..Desk::default()
        },
        verdict: DeskVerdict {
            result: desk,
            reason: None,
        },
    }
}

#[test]
fn a_step_shows_the_text_of_the_desktop_and_each_call_with_the_reason_of_its_retry() {
    let text = dev_smoke::step_text(&logged(12, "vancleef-kill", Verdict::Pass, Verdict::Pass));

    assert_eq!(
        text,
        "PASS  12 vancleef-kill            \"Edwin VanCleef swore that Stormwind would pay.\"\n           \
         desk PASS: kept nothing; facts +0; calls narrator refused, narrator accepted \
         (retry after \"The line names nothing of the moment.\")\n"
    );
}

#[test]
fn the_summary_counts_both_verdicts_names_each_failed_step_and_tallies_the_calls() {
    let steps = [
        logged(1, "zone-move", Verdict::Pass, Verdict::Pass),
        logged(2, "talk", Verdict::Fail, Verdict::Pass),
        logged(3, "level-10", Verdict::Wait, Verdict::Fail),
    ];
    let calls: Vec<CallSeen> = steps
        .iter()
        .flat_map(|step| step.desk.calls.clone())
        .collect();
    let done = SmokeDone {
        run: RUN,
        steps: 4,
        passed: 1,
        failed: 1,
        waited: 1,
        seconds: 90,
        stopped: Stopped::Timeout,
        errors: Vec::new(),
        fps: Some(FpsNumbers {
            samples: 10,
            hidden: 1,
            min: 20,
            p5: 22,
            median: 30,
            mean: 29,
        }),
    };

    let text = dev_smoke::summary(&steps, &done, &CallTally::of(&calls));

    assert!(text.contains("Summary: 3 steps in 90 s (it reached its most time)."));
    assert!(text.contains("Game: 1 passed, 1 failed, 1 waited."));
    assert!(text.contains("Desk: 2 passed, 1 failed, 0 waited."));
    assert!(text.contains("The game ran 4 steps, and 3 reached the desktop."));
    assert!(text.contains("Failed: 2 talk, 3 level-10"));
    assert!(text.contains("Lua errors: none"));
    assert!(text.contains("Model calls: narrator 6 (3 accepted, 3 refused)"));
    assert!(text.contains("Refused: narrator: The line names nothing of the moment. (3)"));
    assert!(text.contains("FPS: 10 samples (1 hidden)"));
}

#[test]
fn a_full_log_takes_no_more_lines() {
    let folder = folder("full");
    let file = log_file(&folder, RUN);
    let big = "x".repeat(usize::try_from(MAX_LOG_BYTES).unwrap() + 1);
    dev_smoke::append_log(&file, &big).unwrap();

    dev_smoke::append_log(&file, "one more line\n").unwrap();

    assert_eq!(std::fs::metadata(&file).unwrap().len(), MAX_LOG_BYTES + 1);
}

#[test]
fn the_newest_log_is_the_run_with_the_highest_number() {
    let folder = folder("newest");
    for run in [1_790_000_100_u64, 1_790_000_900, 1_790_000_500] {
        dev_smoke::append_log(&log_file(&folder, run), "x\n").unwrap();
    }
    dev_smoke::append_log(&folder.join("dev-bench/smoke-notes.log"), "x\n").unwrap();

    assert_eq!(newest_log(&folder), Some(log_file(&folder, 1_790_000_900)));
    assert_eq!(newest_log(&folder.join("none")), None);
}
