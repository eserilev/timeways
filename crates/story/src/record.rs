//! The game events that the bridge sends, one JSON object per line (GAMEPLAY.md 5.4).

use hourglass::Tick;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Record {
    /// Seconds since the Unix epoch, from `time()` in the addon.
    pub at: Tick,
    #[serde(flatten)]
    pub event: GameEvent,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    ZoneEntered {
        zone: String,
        subzone: Option<String>,
    },
    NpcMet {
        name: String,
    },
    LevelReached {
        level: u8,
    },
}
