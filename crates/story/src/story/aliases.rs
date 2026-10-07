//! The alias table in the lines of the addon (GAMEPLAY.md 5.11): a line names players, and
//! the table gives each one an ID before any prompt is built.

use super::{Active, Output, Story, StoryError};
use crate::aliases::{self, AliasRow};
use crate::store::StoreError;

impl Story {
    /// A text that is no player name is refused, and its input stays.
    pub(super) fn describe_player(&mut self, row: &AliasRow) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        active.aliases.learn(row)?.ok_or(StoryError::BadName)?;
        Ok(Vec::new())
    }
}

/// Learns the names that the addon marked.
pub(super) fn learn_names(active: &mut Active, marked: &[String]) -> Result<(), StoreError> {
    for name in marked {
        active.aliases.learn(&AliasRow::named(name))?;
    }
    Ok(())
}

/// The text with each name that the table knows as its ID.
pub(super) fn with_ids(active: &Active, text: &str) -> String {
    aliases::without_names(active.aliases.table(), text)
}

/// The text for the player's own screen: each ID becomes the name of its player.
pub(super) fn with_names(active: &Active, text: &str) -> String {
    aliases::with_names(active.aliases.table(), text)
}

/// True when each ID of a model text names a player of the table, and the text with the
/// names still keeps its limits. A name is longer than its ID, and the page budget of the
/// journal counts on the limits.
pub(super) fn shows_with_names(active: &Active, text: &str, limits: TextLimits) -> bool {
    let table = active.aliases.table();
    let named = aliases::with_names(table, text);
    aliases::knows_every_id(table, text)
        && named.chars().count() <= limits.chars
        && named.len() <= limits.bytes
}

/// The most characters and bytes of one text of the narrator.
#[derive(Clone, Copy, Debug)]
pub(super) struct TextLimits {
    pub(super) chars: usize,
    pub(super) bytes: usize,
}

/// Learns the names that the addon marked, and gives the text with each name that the
/// table knows as its ID.
pub(super) fn without_names(
    active: &mut Active,
    marked: &[String],
    text: &str,
) -> Result<String, StoreError> {
    learn_names(active, marked)?;
    Ok(with_ids(active, text))
}
