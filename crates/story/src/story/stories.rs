//! Player stories in the story program (GAMEPLAY.md 4.8): an accepted story is a row with
//! the proof of another player, and the player removes it only while nothing used it.

use super::{Active, Story, StoryError};
use crate::store::{Node, StoreError, Table};
use crate::stories::{PlayerStory, StoryChange, checked_text, is_taken, standing};
use hourglass::Tick;

impl Story {
    /// A number comes once: a story removed and sent again is a new story.
    pub(super) fn accept_story(
        &mut self,
        at: Tick,
        number: u64,
        text: &str,
    ) -> Result<(), StoryError> {
        let text = checked_text(text).ok_or(StoryError::BadStory)?;
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        if is_taken(active.stories.changes(), number) {
            return Err(StoryError::StoryTaken(number));
        }
        let text = text.to_string();
        active
            .stories
            .add(StoryChange::Accepted { number, at, text })?;
        Ok(())
    }

    /// A story that an accepted model call read stays, because the story rests on it now.
    pub(super) fn remove_story(&mut self, at: Tick, number: u64) -> Result<(), StoryError> {
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
        Ok(())
    }
}

/// A row of this line is not in the database yet, so no call read it.
fn is_used(active: &Active, row: u64) -> Result<bool, StoreError> {
    let uses = active.database.uses_of(Node::Row(Table::Stories, row))?;
    Ok(!uses.is_empty())
}

/// The stories that stand, oldest first, each with whether a call used it.
pub(super) fn journal_stories(active: &Active) -> Result<Vec<PlayerStory>, StoreError> {
    let mut stories = Vec::new();
    for (row, change) in standing(active.stories.changes()) {
        let StoryChange::Accepted { number, at, text } = change else {
            continue;
        };
        stories.push(PlayerStory {
            number: *number,
            text: text.clone(),
            at: *at,
            used: is_used(active, row)?,
        });
    }
    Ok(stories)
}
