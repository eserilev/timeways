//! Player stories (GAMEPLAY.md 4.8): a player in your party tells a short story about you,
//! and you accept it into your world. In this first version no story reaches a prompt, so
//! no real name leaves the addon.

use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// The addon sends at most this, as for the text of a player quest.
pub const MAX_STORY_BYTES: usize = 400;

/// One row of the `stories` table. Tables only grow, so a removal is a row too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum StoryChange {
    Accepted { number: u64, at: Tick, text: String },
    Removed { number: u64, at: Tick },
}

/// An accepted story that the player did not remove, as the journal shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStory {
    /// The number of the addon. It names the author there, and only there.
    pub number: u64,
    pub text: String,
    pub at: Tick,
    /// True once an accepted model call read it. A used story stays.
    pub used: bool,
}

/// The text of a story: not empty, at most `MAX_STORY_BYTES`, and with no control
/// character or `|`, because a `|` starts an escape of the game.
#[must_use]
pub fn checked_text(text: &str) -> Option<&str> {
    let fits = !text.trim().is_empty() && text.len() <= MAX_STORY_BYTES;
    let plain = !text.chars().any(|c| c.is_control() || c == '|');
    (fits && plain).then_some(text)
}

/// Each story that stands, with the row of its acceptance, oldest first.
#[must_use]
pub fn standing(changes: &[StoryChange]) -> Vec<(u64, &StoryChange)> {
    let removed = |number: u64| {
        changes
            .iter()
            .any(|change| matches!(change, StoryChange::Removed { number: n, .. } if *n == number))
    };
    (0..)
        .zip(changes)
        .filter(|(_, change)| matches!(change, StoryChange::Accepted { number, .. } if !removed(*number)))
        .collect()
}

/// True when a story with this number was ever accepted. A number is never used twice.
#[must_use]
pub fn is_taken(changes: &[StoryChange], number: u64) -> bool {
    changes
        .iter()
        .any(|change| matches!(change, StoryChange::Accepted { number: n, .. } if *n == number))
}
