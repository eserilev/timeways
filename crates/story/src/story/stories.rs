//! Player stories in the story program (GAMEPLAY.md 4.8): an accepted story is a row with
//! the proof of another player, and the player removes it only while nothing used it.

use super::{Active, Output, Story, StoryError, aliases};
use crate::aliases::{unmarked, with_names};
use crate::store::{Node, StoreError, Table};
use crate::stories::{PlayerStory, StoryChange, checked_text, is_taken, standing};
use hourglass::Tick;

impl Story {
    /// A number comes once: a story removed and sent again is a new story. The story keeps
    /// an ID in place of each name of a player (5.11), and the limits hold for the text as
    /// its author wrote it.
    pub(super) fn accept_story(
        &mut self,
        at: Tick,
        number: u64,
        text: &str,
    ) -> Result<Vec<Output>, StoryError> {
        let marked = unmarked(text);
        let text = checked_text(&marked.text).ok_or(StoryError::BadStory)?;
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        if is_taken(active.stories.changes(), number) {
            return Err(StoryError::StoryTaken(number));
        }
        let text = aliases::without_names(active, &marked.names, text)?;
        active
            .stories
            .add(StoryChange::Accepted { number, at, text })?;
        Ok(Vec::new())
    }

    /// A story that an accepted model call read stays, because the story rests on it now.
    pub(super) fn remove_story(
        &mut self,
        at: Tick,
        number: u64,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let row = standing(active.stories.changes())
            .into_iter()
            .find(|(_, change)| matches!(change, StoryChange::Accepted { number: n, .. } if *n == number))
            .map(|(row, _)| row)
            .ok_or(StoryError::NoStory(number))?;
        if is_used(active, row)? {
            return Err(StoryError::StoryInUse(number));
        }
        active.stories.add(StoryChange::Removed { number, at })?;
        Ok(Vec::new())
    }
}

/// True when a call read the row. A row that this line added is not saved yet, so the
/// database has no use of it: correct, because no call read it yet.
fn is_used(active: &Active, row: u64) -> Result<bool, StoreError> {
    let uses = active.database.uses_of(Node::Row(Table::Stories, row))?;
    Ok(!uses.is_empty())
}

/// The stories that stand, oldest first, each with whether a call used it. The player
/// reads the names, never the IDs.
pub(super) fn journal_stories(active: &Active) -> Result<Vec<PlayerStory>, StoreError> {
    let mut stories = Vec::new();
    for (row, change) in standing(active.stories.changes()) {
        let StoryChange::Accepted { number, at, text } = change else {
            continue;
        };
        stories.push(PlayerStory {
            number: *number,
            text: with_names(active.aliases.table(), text),
            at: *at,
            used: is_used(active, row)?,
        });
    }
    Ok(stories)
}
