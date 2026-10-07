//! A command that changes a world runs only while dev mode is on, so a slip of the keyboard
//! never writes over the world of a real character (GAMEPLAY.md 5.15).

use std::path::Path;
use thiserror::Error;
use timeways_story::dev_mode::DevMode;

/// The commands that write a world. `snapshot` only copies one.
const CHANGES_A_WORLD: [&str; 2] = ["seed", "restore"];

#[derive(Debug, Error, PartialEq, Eq)]
#[error("Dev mode is off. Run `timeways-dev on` first, then `timeways-dev {command}` again.")]
pub struct DevModeOff {
    pub command: String,
}

#[must_use]
pub fn changes_a_world(command: &str) -> bool {
    CHANGES_A_WORLD.contains(&command)
}

/// # Errors
///
/// Returns `DevModeOff` when the settings in the story folder do not turn dev mode on.
pub fn check_dev_mode(command: &str, story: &Path) -> Result<(), DevModeOff> {
    if DevMode::of_folder(story).is_on() {
        return Ok(());
    }
    Err(DevModeOff {
        command: command.to_string(),
    })
}
