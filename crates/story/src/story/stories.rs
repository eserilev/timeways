//! Player stories in the story program (GAMEPLAY.md 4.8): an accepted story is a row with
//! the proof of another player, and the player removes it only while nothing used it.

use super::{Active, Output, Story, StoryError, aliases};
use crate::aliases::{Marked, unmarked, with_names};
use crate::store::{Node, StoreError, Table};
use crate::stories::{PlayerStory, ShelfLine, StoryChange, is_good_story, lands, standing};
use hourglass::Tick;

impl Story {
    /// A number comes once: a story removed and sent again is a new story. The story keeps
    /// an ID in place of each name of a player (5.11), and the limits hold for the text as
    /// its author wrote it. An empty title is no title.
    pub(super) fn accept_story(
        &mut self,
        at: Tick,
        number: u64,
        title: Option<&str>,
        paragraphs: &[String],
    ) -> Result<Vec<Output>, StoryError> {
        let title = title.filter(|title| !title.is_empty()).map(unmarked);
        let paragraphs: Vec<Marked> = paragraphs.iter().map(|text| unmarked(text)).collect();
        let plain: Vec<String> = paragraphs
            .iter()
            .map(|marked| marked.text.clone())
            .collect();
        let plain_title = title.as_ref().map(|marked| marked.text.as_str());
        if !is_good_story(plain_title, &plain) {
            return Err(StoryError::BadStory);
        }
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        if !lands(
            active.stories.changes(),
            ShelfLine::Accepted { number },
            false,
        ) {
            return Err(StoryError::StoryTaken(number));
        }
        let names: Vec<String> = title
            .iter()
            .chain(&paragraphs)
            .flat_map(|marked| marked.names.iter().cloned())
            .collect();
        aliases::learn_names(active, &names)?;
        let accepted = StoryChange::Accepted {
            number,
            at,
            title: title.map(|marked| aliases::with_ids(active, &marked.text)),
            paragraphs: plain
                .iter()
                .map(|text| aliases::with_ids(active, text))
                .collect(),
        };
        active.stories.add(accepted)?;
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
        let removal = ShelfLine::Removed { number };
        if !lands(active.stories.changes(), removal, is_used(active, row)?) {
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
        let StoryChange::Accepted {
            number,
            at,
            title,
            paragraphs,
        } = change
        else {
            continue;
        };
        let table = active.aliases.table();
        stories.push(PlayerStory {
            number: *number,
            title: title.as_ref().map(|title| with_names(table, title)),
            paragraphs: paragraphs
                .iter()
                .map(|text| with_names(table, text))
                .collect(),
            at: *at,
            used: is_used(active, row)?,
        });
    }
    Ok(stories)
}
