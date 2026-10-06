//! Dev mode: fake lines from `/twdev` in the game, for tests with no hours of play
//! (TESTING.md, "Dev mode"). It is off unless the file `settings.toml` in the story folder
//! holds `dev = true`. No player makes that file by accident: only `timeways-dev on` or a
//! person at the desktop writes it.

use serde::Deserialize;
use serde::ser::{Serialize, Serializer};
use std::path::Path;

/// The settings of the story program, in its data folder.
pub const SETTINGS_FILE: &str = "settings.toml";

/// The key of a fake line. Any line that holds it is a dev line, whatever its value.
pub const DEV_KEY: &str = "dev";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DevMode {
    #[default]
    Off,
    On,
}

#[derive(Deserialize)]
struct Settings {
    #[serde(default)]
    dev: bool,
}

impl DevMode {
    /// A missing file, a file that does not read, or any value but `dev = true` is off.
    #[must_use]
    pub fn of_folder(folder: &Path) -> DevMode {
        let Ok(text) = std::fs::read_to_string(folder.join(SETTINGS_FILE)) else {
            return DevMode::Off;
        };
        DevMode::of_settings(&text)
    }

    #[must_use]
    pub fn of_settings(text: &str) -> DevMode {
        match toml::from_str::<Settings>(text) {
            Ok(Settings { dev: true }) => DevMode::On,
            _ => DevMode::Off,
        }
    }

    #[must_use]
    pub fn is_on(self) -> bool {
        self == DevMode::On
    }
}

/// The mark of the journal while dev mode is on: `"dev": true`. The addon turns `/twdev` on
/// only when it reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DevOn;

impl Serialize for DevOn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(true)
    }
}

/// True for a JSON object that holds the key `dev`. A line that does not read is no dev
/// line: the parse of the input refuses it anyway.
#[must_use]
pub fn is_dev_line(line: &str) -> bool {
    serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(line)
        .is_ok_and(|object| object.contains_key(DEV_KEY))
}
