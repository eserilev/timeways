//! Player stories in the story program (GAMEPLAY.md 4.8): an accepted story is a row with
//! the proof of another player, and the player removes it only while nothing used it.

use super::{Active, Output, Story, StoryError, aliases};
use crate::aliases::{
    MAX_NAMES, Marked, for_matching, other_players, unlikely_marks_as_words, unmarked, with_names,
};
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
        // The author marked the story, so a mark teaches only a likely name, and a story
        // names at most `MAX_NAMES` other players: a longer list is no story of the addon.
        let title = title.filter(|title| !title.is_empty());
        let mut cleaned: Vec<String> = title
            .into_iter()
            .chain(paragraphs.iter().map(String::as_str))
            .map(|text| unlikely_marks_as_words(&for_matching(text)))
            .collect();
        let own = self
            .active
            .as_ref()
            .map(|active| active.key.name())
            .unwrap_or_default();
        if other_players(&cleaned, own) > MAX_NAMES {
            return Err(StoryError::BadStory);
        }
        let cleaned_title = title.map(|_| cleaned.remove(0));
        let title = cleaned_title.as_deref().map(unmarked);
        let paragraphs: Vec<Marked> = cleaned.iter().map(|text| unmarked(text)).collect();
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
            title: cleaned_title.map(|text| aliases::ids_of_marked(active, &text)),
            paragraphs: cleaned
                .iter()
                .map(|text| aliases::ids_of_marked(active, text))
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
