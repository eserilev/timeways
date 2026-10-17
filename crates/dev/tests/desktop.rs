//! The folders and the config of the desktop app, and the switch of dev mode.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;
use timeways_dev::desktop::{Desktop, ModelSetting};
use timeways_dev::model_runner::LocalModel;
use timeways_story::ai::Ai;
use timeways_story::dev_mode::{DevMode, SETTINGS_FILE};

fn desktop_with_config(name: &str, config: &str) -> Desktop {
    let root = common::folder(name);
    let desktop = Desktop::under(&root.join("config"), &root.join("data"));
    std::fs::create_dir_all(desktop.config.parent().unwrap()).unwrap();
    std::fs::write(&desktop.config, config).unwrap();
    desktop
}

#[test]
fn the_story_folder_and_the_config_follow_the_bridge() {
    let desktop = Desktop::under(Path::new("/c"), Path::new("/d"));

    assert_eq!(desktop.story, Path::new("/d/gnomish-relay/timeways/story"));
    assert_eq!(desktop.config, Path::new("/c/gnomish-relay/config.toml"));
}

#[test]
fn the_claude_model_of_the_config_runs_claude_with_no_tools_and_its_model_name() {
    let desktop = desktop_with_config(
        "claude",
        "[story]\nmodel = \"claude\"\nclaude_model = \"haiku\"\n",
    );

    let ModelSetting::Command(command) = desktop.model() else {
        panic!("{:?}", desktop.model());
    };
    assert!(command.starts_with("claude -p --tools ''"), "{command}");
    assert!(command.ends_with("--model 'haiku'"), "{command}");
}

#[test]
fn no_model_in_the_config_is_no_model() {
    assert_eq!(
        desktop_with_config("none", "[wow]\npath = \"~/wow\"\n").model(),
        ModelSetting::None
    );
}

#[test]
fn a_local_model_of_the_config_keeps_its_url_and_its_name() {
    let config =
        "[story]\nmodel = \"local\"\nlocal_url = \"http://[::1]:1234\"\nlocal_model = \"qwen3\"\n";

    let model = desktop_with_config("local", config).model();

    let expected = LocalModel {
        url: "http://[::1]:1234".to_string(),
        model: "qwen3".to_string(),
    };
    assert_eq!(model, ModelSetting::Local(expected));
}

#[test]
fn with_no_local_model_in_the_config_the_bench_takes_the_one_that_the_relay_installs() {
    let local = desktop_with_config("local-default", "[story]\nmodel = \"claude\"\n").local_model();

    assert_eq!(local.url, "http://127.0.0.1:11434");
    assert_eq!(local.model, "llama3.2:3b");
}

#[test]
fn the_lore_pack_of_the_config_reads_a_home_path() {
    let desktop = desktop_with_config("pack", "[story]\nlore_pack = \"/opt/lore.sqlite\"\n");

    assert_eq!(
        desktop.lore_pack().as_deref(),
        Some(Path::new("/opt/lore.sqlite"))
    );
}

#[test]
fn on_and_off_write_the_settings_that_the_story_program_reads() {
    let desktop = desktop_with_config("switch", "");
    assert_eq!(desktop.dev_mode(), DevMode::Off);

    desktop.set_dev_mode(DevMode::On).unwrap();
    assert_eq!(desktop.dev_mode(), DevMode::On);

    desktop.set_dev_mode(DevMode::Off).unwrap();
    assert_eq!(desktop.dev_mode(), DevMode::Off);
}

#[test]
fn on_and_off_keep_the_ai_switch_of_the_settings() {
    let desktop = desktop_with_config("switch-keeps-ai", "");
    std::fs::create_dir_all(&desktop.story).unwrap();
    std::fs::write(desktop.story.join(SETTINGS_FILE), "ai = \"off\"\n").unwrap();

    desktop.set_dev_mode(DevMode::On).unwrap();
    desktop.set_dev_mode(DevMode::Off).unwrap();
    desktop.set_dev_mode(DevMode::On).unwrap();

    assert_eq!(desktop.dev_mode(), DevMode::On);
    assert_eq!(desktop.dev_ai(), Ai::Off);
}

fn desktop_with_settings(name: &str, settings: &[u8]) -> Desktop {
    let desktop = desktop_with_config(name, "");
    std::fs::create_dir_all(&desktop.story).unwrap();
    std::fs::write(desktop.story.join(SETTINGS_FILE), settings).unwrap();
    desktop
}

fn settings_text(desktop: &Desktop) -> Vec<u8> {
    std::fs::read(desktop.story.join(SETTINGS_FILE)).unwrap()
}

#[test]
fn a_dev_key_in_a_table_never_turns_dev_mode_on_or_off() {
    let desktop = desktop_with_settings("switch-table", b"[later]\ndev = true\n");

    desktop.set_dev_mode(DevMode::On).unwrap();
    assert_eq!(desktop.dev_mode(), DevMode::On);

    desktop.set_dev_mode(DevMode::Off).unwrap();
    assert_eq!(desktop.dev_mode(), DevMode::Off);
}

#[test]
fn on_and_off_read_a_quoted_dev_key() {
    let desktop = desktop_with_settings("switch-quoted", b"\"dev\" = true\nai = \"off\"\n");

    desktop.set_dev_mode(DevMode::Off).unwrap();

    assert_eq!(desktop.dev_mode(), DevMode::Off);
    assert_eq!(desktop.dev_ai(), Ai::Off);
}

#[test]
fn on_refuses_settings_that_do_not_read_and_keeps_them() {
    let desktop = desktop_with_settings("switch-unread", b"ai = off\n");

    assert!(desktop.set_dev_mode(DevMode::On).is_err());
    assert_eq!(settings_text(&desktop), b"ai = off\n");
}

#[test]
fn on_refuses_settings_that_are_not_text_and_keeps_them() {
    let desktop = desktop_with_settings("switch-bytes", b"dev = \xff\n");

    assert!(desktop.set_dev_mode(DevMode::On).is_err());
    assert_eq!(settings_text(&desktop), b"dev = \xff\n");
}
