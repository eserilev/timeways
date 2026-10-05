//! Player stories (GAMEPLAY.md 4.8): a player in your party tells a short story about you,
//! and you accept it into your world. In this first version no story reaches a prompt, so
//! no real name leaves the addon.

use hourglass::Tick;
use serde::{Deserialize, Serialize};
use timeways_rules::story_shelf;
pub use timeways_rules::story_shelf::ShelfLine;

/// The limits of the addon (`StoryText.lua`). They hold for the text without its marks, so
/// a mark never makes a story too long. A paragraph break counts as one letter and one byte.
pub const MAX_TITLE_CHARS: usize = 60;
pub const MAX_TITLE_BYTES: usize = 72;
pub const MAX_BODY_CHARS: usize = 1000;
pub const MAX_BODY_BYTES: usize = 1200;
/// A list of the journal holds at most 200 items (relay SPEC 9.8), so the wire keeps far
/// below it.
pub const MAX_PARAGRAPHS: usize = 20;

/// One row of the `stories` table. Tables only grow, so a removal is a row too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum StoryChange {
    Accepted {
        number: u64,
        at: Tick,
        title: Option<String>,
        paragraphs: Vec<String>,
    },
    Removed {
        number: u64,
        at: Tick,
    },
}

/// An accepted story that the player did not remove, as the journal shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStory {
    /// The number of the addon. It names the author there, and only there.
    pub number: u64,
    pub title: Option<String>,
    pub paragraphs: Vec<String>,
    pub at: Tick,
    /// True once an accepted model call read it. A used story stays.
    pub used: bool,
}

/// No control character, because the bridge drops a line that holds one, and no `|`,
/// because a `|` starts an escape of the game.
fn is_plain(text: &str) -> bool {
    !text.chars().any(|c| c.is_control() || c == '|')
}

/// The addon trims each paragraph of its spaces, as Lua's `%s` knows them.
fn is_trimmed(text: &str) -> bool {
    !text.is_empty() && !text.starts_with(' ') && !text.ends_with(' ')
}

fn title_fits(title: &str) -> bool {
    title.chars().count() <= MAX_TITLE_CHARS && title.len() <= MAX_TITLE_BYTES && is_plain(title)
}

/// The size of the body, with one break between two paragraphs.
fn body_fits(paragraphs: &[String]) -> bool {
    let breaks = paragraphs.len().saturating_sub(1);
    let chars: usize = paragraphs.iter().map(|p| p.chars().count()).sum();
    let bytes: usize = paragraphs.iter().map(String::len).sum();
    chars + breaks <= MAX_BODY_CHARS && bytes + breaks <= MAX_BODY_BYTES
}

/// True for a story that the addon could send: a title that fits, or none, and 1 to 20
/// paragraphs that are not empty, start and end with no space, and fit together.
#[must_use]
pub fn is_good_story(title: Option<&str>, paragraphs: &[String]) -> bool {
    let count_fits = (1..=MAX_PARAGRAPHS).contains(&paragraphs.len());
    let each_fits = paragraphs.iter().all(|p| is_trimmed(p) && is_plain(p));
    count_fits && each_fits && body_fits(paragraphs) && title.is_none_or(title_fits)
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
