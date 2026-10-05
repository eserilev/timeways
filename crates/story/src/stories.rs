//! Player stories (GAMEPLAY.md 4.8): a player in your party tells a short story about you,
//! and you accept it into your world. In this first version no story reaches a prompt, so
//! no real name leaves the addon.

use hourglass::Tick;
use serde::{Deserialize, Serialize};
use timeways_rules::story_shelf;
pub use timeways_rules::story_shelf::ShelfLine;

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

/// The line as the proved rules of the shelf read it (`timeways_rules::story_shelf`).
fn shelf_line(change: &StoryChange) -> ShelfLine {
    match change {
        StoryChange::Accepted { number, .. } => ShelfLine::Accepted { number: *number },
        StoryChange::Removed { number, .. } => ShelfLine::Removed { number: *number },
    }
}

#[must_use]
pub fn shelf(changes: &[StoryChange]) -> Vec<ShelfLine> {
    changes.iter().map(shelf_line).collect()
}

/// The row of the acceptance of a number.
fn accepted_row(changes: &[StoryChange], number: u64) -> Option<(u64, &StoryChange)> {
    (0..).zip(changes).find(
        |(_, change)| matches!(change, StoryChange::Accepted { number: n, .. } if *n == number),
    )
}

/// Each story that stands, with the row of its acceptance, oldest first.
#[must_use]
pub fn standing(changes: &[StoryChange]) -> Vec<(u64, &StoryChange)> {
    story_shelf::standing(&shelf(changes))
        .into_iter()
        .filter_map(|number| accepted_row(changes, number))
        .collect()
}

/// True when the line may become a row. `used` is true when an accepted call read the
/// story of a removal.
#[must_use]
pub fn lands(changes: &[StoryChange], line: ShelfLine, used: bool) -> bool {
    let used = match (line, used) {
        (ShelfLine::Removed { number }, true) => vec![number],
        _ => Vec::new(),
    };
    story_shelf::lands(&shelf(changes), line, &used)
}
