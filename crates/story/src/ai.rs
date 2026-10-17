//! AI on or off: the player chose to play with a model or with none (GAMEPLAY.md 5.6).
//! The bridge says it in its hello. With AI off, the story program asks no model.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::dev_mode::SETTINGS_FILE;

/// A bridge from before this switch sends no `ai`, and it has AI on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ai {
    #[default]
    On,
    Off,
}

#[derive(Deserialize)]
struct Settings {
    #[serde(default)]
    ai: Option<String>,
}

impl Ai {
    #[must_use]
    pub fn is_on(self) -> bool {
        self == Ai::On
    }

    /// The dev switch `ai = "off"` in `settings.toml`. It tests AI off with no change to
    /// the config of the bridge. Any other value, or a file that does not read, is on.
    #[must_use]
    pub fn of_settings(text: &str) -> Ai {
        match toml::from_str::<Settings>(text) {
            Ok(Settings { ai: Some(word) }) if word == "off" => Ai::Off,
            _ => Ai::On,
        }
    }

    #[must_use]
    pub fn of_folder(folder: &Path) -> Ai {
        std::fs::read_to_string(folder.join(SETTINGS_FILE))
            .map_or(Ai::On, |text| Ai::of_settings(&text))
    }
}
