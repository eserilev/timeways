//! The commands of `timeways-dev` as a person runs them: a command that changes a world
//! needs dev mode (GAMEPLAY.md 5.15).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;
use std::process::{Command, Output};
use timeways_dev::worlds::world_file;

fn dev(args: &[&str], data: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_timeways-dev"))
        .args(args)
        .arg("--data")
        .arg(data)
        .output()
        .unwrap()
}

fn seed(data: &Path) -> Output {
    let no_pack = data.join("no-pack.sqlite");
    let args = [
        "seed",
        common::NAME,
        "--realm",
        common::REALM,
        "--scenario",
        "fresh",
        "--no-model",
        "--pack",
        no_pack.to_str().unwrap(),
    ];
    dev(&args, data)
}

fn restore(data: &Path) -> Output {
    let args = ["restore", common::NAME, "before", "--realm", common::REALM];
    dev(&args, data)
}

fn turn_dev_mode_on(data: &Path) {
    std::fs::write(data.join("settings.toml"), "dev = true\n").unwrap();
}

fn says_dev_mode_is_off(output: &Output) -> bool {
    String::from_utf8_lossy(&output.stderr).contains("Dev mode is off.")
}

#[test]
fn a_seed_refuses_while_dev_mode_is_off_and_writes_no_world() {
    let data = common::folder("seed-off");

    let output = seed(&data);

    assert!(!output.status.success());
    assert!(says_dev_mode_is_off(&output), "{output:?}");
    let world = world_file(&data, common::REALM, common::NAME).unwrap();
    assert!(!world.exists());
}

#[test]
fn a_restore_refuses_while_dev_mode_is_off() {
    let data = common::folder("restore-off");

    let output = restore(&data);

    assert!(!output.status.success());
    assert!(says_dev_mode_is_off(&output), "{output:?}");
}

#[test]
fn a_seed_writes_its_world_while_dev_mode_is_on() {
    let data = common::folder("seed-on");
    turn_dev_mode_on(&data);

    let output = seed(&data);

    assert!(output.status.success(), "{output:?}");
    let world = world_file(&data, common::REALM, common::NAME).unwrap();
    assert!(world.exists());
}

#[test]
fn a_command_that_changes_no_world_works_while_dev_mode_is_off() {
    let data = common::folder("read-only-off");
    common::seed("fresh", &data);
    let snapshot = ["snapshot", common::NAME, "before", "--realm", common::REALM];

    let saved = dev(&snapshot, &data);
    let listed = Command::new(env!("CARGO_BIN_EXE_timeways-dev"))
        .arg("scenarios")
        .output()
        .unwrap();

    assert!(saved.status.success(), "{saved:?}");
    assert!(listed.status.success(), "{listed:?}");
}

#[test]
fn export_ratings_writes_a_file_with_no_player_name_while_dev_mode_is_off() {
    let data = common::folder("export-ratings");
    common::seed("ratings", &data);
    let out = data.join("to-send.json");
    let args = [
        "export-ratings",
        common::NAME,
        "--realm",
        common::REALM,
        "--out",
        out.to_str().unwrap(),
        "--data",
        data.to_str().unwrap(),
    ];

    let output = Command::new(env!("CARGO_BIN_EXE_timeways-dev"))
        .args(args)
        .env("XDG_CONFIG_HOME", &data)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let text = std::fs::read_to_string(&out).unwrap();
    let export: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(export["ratings"].as_array().unwrap().len(), 2, "{text}");
    assert_eq!(export["ratings"][0]["model"], "none");
    assert!(!text.contains(common::NAME), "{text}");
}

fn prepare_smoke(data: &Path) -> Output {
    let no_pack = data.join("no-pack.sqlite");
    let args = [
        "smoke",
        "--prepare",
        common::NAME,
        "--realm",
        common::REALM,
        "--pack",
        no_pack.to_str().unwrap(),
    ];
    dev(&args, data)
}

#[test]
fn smoke_prepare_refuses_while_dev_mode_is_off_and_writes_no_world() {
    let data = common::folder("smoke-prepare-off");

    let output = prepare_smoke(&data);

    assert!(!output.status.success());
    assert!(says_dev_mode_is_off(&output), "{output:?}");
    let world = world_file(&data, common::REALM, common::NAME).unwrap();
    assert!(!world.exists());
}

#[test]
fn smoke_prepare_seeds_a_fresh_world_and_keeps_the_old_one_as_a_backup() {
    let data = common::folder("smoke-prepare-on");
    common::seed("level-30-paladin", &data);
    turn_dev_mode_on(&data);
    let world = world_file(&data, common::REALM, common::NAME).unwrap();
    let before = std::fs::metadata(&world).unwrap().len();

    let output = prepare_smoke(&data);

    assert!(output.status.success(), "{output:?}");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(said.contains("The old world is now"), "{said}");
    assert!(said.contains("/twdev smoke"), "{said}");
    assert!(std::fs::metadata(&world).unwrap().len() < before);
}

#[test]
fn smoke_prints_the_newest_log_of_the_dev_folder() {
    let data = common::folder("smoke-print");
    let bench = data.join("dev-bench");
    std::fs::create_dir_all(&bench).unwrap();
    std::fs::write(bench.join("smoke-1790000100.log"), "PASS   0 start\n").unwrap();
    std::fs::write(bench.join("smoke-1790000900.log"), "FAIL   3 capital\n").unwrap();

    let output = dev(&["smoke", "--latest"], &data);

    assert!(output.status.success(), "{output:?}");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(said.contains("smoke-1790000900.log"), "{said}");
    assert!(said.contains("FAIL   3 capital"), "{said}");
    assert!(!said.contains("PASS   0 start"), "{said}");
}

#[test]
fn smoke_with_no_log_says_how_to_make_one() {
    let data = common::folder("smoke-none");

    let output = dev(&["smoke"], &data);

    assert!(!output.status.success());
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("Type /twdev smoke in the game"), "{said}");
}
