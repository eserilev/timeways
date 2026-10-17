//! AI on or off: the player chose to play with a model or with none (GAMEPLAY.md 5.6).
//! The addon says it in its `ai_set` line. The hello of the bridge can say it too, but the
//! bridge does not send it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// The `ai` word of the hello or of `ai_set`. Only "off" turns AI off. A word that this
/// version does not know keeps AI on, because a hello that does not read starts no story
/// program.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Value", into = "Value")]
pub enum AiWord {
    #[default]
    On,
    Off,
    /// The word as JSON, for the note of the program.
    Unknown(String),
}

impl AiWord {
    #[must_use]
    pub fn ai(&self) -> Ai {
        match self {
            AiWord::Off => Ai::Off,
            AiWord::On | AiWord::Unknown(_) => Ai::On,
        }
    }
}

impl From<Value> for AiWord {
    fn from(value: Value) -> AiWord {
        match value.as_str() {
            Some("on") => AiWord::On,
            Some("off") => AiWord::Off,
            _ => AiWord::Unknown(value.to_string()),
        }
    }
}

impl From<AiWord> for Value {
    fn from(ai: AiWord) -> Value {
        match ai {
            AiWord::On => Value::from("on"),
            AiWord::Off => Value::from("off"),
            AiWord::Unknown(word) => serde_json::from_str(&word).unwrap_or(Value::Null),
        }
    }
}
