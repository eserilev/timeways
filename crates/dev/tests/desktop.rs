//! The folders and the config of the desktop app, and the switch of dev mode.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::path::Path;
use timeways_dev::desktop::{Desktop, ModelSetting};
use timeways_story::dev_mode::DevMode;

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
fn no_model_in_the_config_is_no_model_and_a_local_one_needs_a_command() {
    assert_eq!(
        desktop_with_config("none", "[wow]\npath = \"~/wow\"\n").model(),
        ModelSetting::None
    );
    assert_eq!(
        desktop_with_config("local", "[story]\nmodel = \"local\"\n").model(),
        ModelSetting::Other("local".to_string())
    );
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
