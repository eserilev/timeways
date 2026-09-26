//! What the bridge sends, one JSON object per line: game events from the addon, and the
//! answers to model calls (GAMEPLAY.md 3.1, 5.4, and 5.6).

use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// A game event carries the time from `time()` in the addon, in seconds since the Unix epoch.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    ZoneEntered {
        at: Tick,
        zone: String,
        subzone: Option<String>,
    },
    NpcMet {
        at: Tick,
        name: String,
    },
    LevelReached {
        at: Tick,
        level: u8,
    },
    /// `/lore`, with the name of the target when there is one.
    LoreAsked {
        at: Tick,
        question: String,
        target: Option<String>,
    },
    ModelAnswered {
        call: CallId,
        text: String,
    },
    /// No model, a timeout, or a spent budget. The bridge decides which.
    ModelFailed {
        call: CallId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CallId(pub u64);
