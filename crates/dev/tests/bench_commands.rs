//! `timeways-dev bench-model` and `bench-fps` as a person runs them, with a fake model: a
//! script that answers fixed JSON (TESTING.md, "Testing the local model and the frame rate").

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};
use timeways_story::dev_fps::fps_file;

/// Answers JSON when the prompt asks for it, and a plain line else.
const FAKE_MODEL: &str = r#"#!/bin/sh
prompt=$(cat)
case "$prompt" in
  *JSON*) echo '{"say": "The roads are not safe.", "trust": 0, "work": false}' ;;
  *) echo 'The Defias Brotherhood holds Westfall now.' ;;
esac
"#;

fn fake_model(folder: &Path) -> PathBuf {
    let script = folder.join("fake-model.sh");
    std::fs::write(&script, FAKE_MODEL).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn dev(args: &[&str], data: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_timeways-dev"))
        .args(args)
        .arg("--data")
        .arg(data)
        .arg("--pack")
        .arg(data.join("no-pack.sqlite"))
        .output()
        .unwrap()
}

fn turn_dev_mode_on(data: &Path) {
    std::fs::write(data.join("settings.toml"), "dev = true\n").unwrap();
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// The files of the dev folder whose name starts with `prefix`.
fn results(data: &Path, prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(data.join("dev-bench"))
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .filter(|name| name.starts_with(prefix))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn the_benches_refuse_while_dev_mode_is_off_and_call_no_model() {
    let data = common::folder("bench-off");
    let model = fake_model(&data);
    let model = model.to_str().unwrap();

    let by_model = dev(&["bench-model", "--model", model], &data);
    let by_fps = dev(&["bench-fps", "--model", model, "--seconds", "1"], &data);

    for output in [by_model, by_fps] {
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("Dev mode is off."), "{error}");
    }
    assert!(results(&data, "").is_empty());
}

#[test]
fn bench_model_runs_the_moments_with_a_fake_model_and_writes_its_results() {
    let data = common::folder("bench-model");
    turn_dev_mode_on(&data);
    let model = fake_model(&data);

    let output = dev(&["bench-model", "--model", model.to_str().unwrap()], &data);

    assert!(output.status.success(), "{output:?}");
    let printed = stdout(&output);
    assert!(printed.contains("talk / talk"), "{printed}");
    assert!(
        printed.contains("[talk / talk] The roads are not safe."),
        "{printed}"
    );
    assert!(printed.contains("Moments with no model call"), "{printed}");
    let files = results(&data, "model-");
    assert_eq!(files.len(), 2, "{files:?}");
    let json = data.join("dev-bench").join(&files[0]);
    let reports: Value = serde_json::from_str(&std::fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(reports[0]["set"], "v1");
    assert!(reports[0]["total"]["asks"].as_u64().unwrap() > 0);
}

#[test]
fn bench_model_compares_two_models_side_by_side() {
    let data = common::folder("bench-compare");
    turn_dev_mode_on(&data);
    let model = fake_model(&data);
    let args = [
        "bench-model",
        "--compare",
        "model,model",
        "--model",
        model.to_str().unwrap(),
    ];

    let output = dev(&args, &data);

    assert!(output.status.success(), "{output:?}");
    let printed = stdout(&output);
    assert!(printed.contains("\nSummary:\nA: shown"), "{printed}");
    assert!(printed.contains("\nB: shown"), "{printed}");
    assert!(
        printed.contains("  A: The roads are not safe."),
        "{printed}"
    );
}

#[test]
fn bench_model_says_so_when_the_model_gives_no_answer() {
    let data = common::folder("bench-silent-model");
    turn_dev_mode_on(&data);

    let output = dev(&["bench-model", "--model", "exit 1"], &data);

    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("gave no answer"), "{error}");
}

#[test]
fn bench_fps_reports_each_phase_from_the_run_of_the_addon() {
    let data = common::folder("bench-fps");
    turn_dev_mode_on(&data);
    let model = fake_model(&data);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // A run of /twdev fps that started before the bench and stops after it.
    let run = json!({
        "label": "bench", "started": now - 10, "ended": now + 110,
        "samples": vec![60; 120], "hidden": 0,
        "min": 60, "p5": 60, "median": 60, "mean": 60, "memory_kb": 812
    });
    std::fs::create_dir_all(fps_file(&data).parent().unwrap()).unwrap();
    std::fs::write(fps_file(&data), format!("{run}\n")).unwrap();
    let args = [
        "bench-fps",
        "--model",
        model.to_str().unwrap(),
        "--seconds",
        "1",
        "--lead",
        "0",
        "--wait",
        "0",
    ];

    let output = dev(&args, &data);

    assert!(output.status.success(), "{output:?}");
    let printed = stdout(&output);
    assert!(printed.contains("/twdev fps start bench"), "{printed}");
    assert!(printed.contains("/twdev fps stop"), "{printed}");
    for phase in ["baseline", "load", "recovery"] {
        assert!(
            printed.contains(&format!("\n{phase}")),
            "{phase}: {printed}"
        );
    }
    assert!(printed.contains("Timeways memory 812 KB"), "{printed}");
    assert_eq!(results(&data, "fps-").len(), 2);
}
