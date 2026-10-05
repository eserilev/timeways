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

/// Learns the names that the addon marked, and gives the text with each name that the
/// table knows as its ID.
pub(super) fn without_names(
    active: &mut Active,
    marked: &[String],
    text: &str,
) -> Result<String, StoreError> {
    for name in marked {
        active.aliases.learn(&AliasRow::named(name))?;
    }
    Ok(aliases::without_names(active.aliases.table(), text))
}
