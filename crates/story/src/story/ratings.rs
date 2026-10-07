//! A rating of the player in the story program (GAMEPLAY.md 3.2.2): the desktop finds the
//! rated text in its own rows, so the addon never sends a text, and a hostile addon can
//! put no name of a player into a rating.

use super::{Active, Output, Story, StoryError, tales};
use crate::ratings::{Rated, RatedLine, Rating};
use hourglass::{EventId, Tick};

/// What a rating is about, as the row keeps it.
struct Found {
    key: Option<u64>,
    moment: String,
    text: String,
    faults: Vec<String>,
}

impl Story {
    /// A rating of a text that the story program never showed writes no row. The line has
    /// no reply, so the note goes to the log only.
    pub(super) fn rate(
        &mut self,
        at: Tick,
        rated: Rated,
        first: Option<u64>,
        rating: Rating,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let Some(found) = rated_text(active, rated, first) else {
            self.notes.push(format!(
                "a rating of {rated:?} {first:?}, which shows no text"
            ));
            return Ok(Vec::new());
        };
        let row = RatedLine {
            at,
            rated,
            key: found.key,
            rating,
            moment: found.moment,
            text: found.text,
            faults: found.faults,
        };
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        active.ratings.add(row)?;
        Ok(Vec::new())
    }
}

/// The text of the narrator as the disk keeps it: with `$N` and the IDs of players.
fn rated_text(active: &Active, rated: Rated, first: Option<u64>) -> Option<Found> {
    let found = match rated {
        Rated::Narrator => {
            let line = active.shown_line.as_ref()?;
            Found {
                key: line.call,
                moment: line.moment.to_string(),
                text: line.text.clone(),
                faults: line.faults.clone(),
            }
        }
        Rated::Chapter => {
            let first = first?;
            let written = active.prose.get(EventId(first))?;
            entry(Some(first), "chapter", &written.text)
        }
        Rated::Tale => {
            let first = first?;
            let (_, tale) = tales::newest_text(active, EventId(first))?;
            entry(Some(first), "tale", &tale.text)
        }
        Rated::Summary => {
            let (_, summary) = active.summaries.newest()?;
            entry(None, "summary", &summary.text)
        }
    };
    (!found.text.is_empty()).then_some(found)
}

/// A chapter, a tale, and the summary get no retry, so they have no faults.
fn entry(key: Option<u64>, moment: &str, text: &str) -> Found {
    Found {
        key,
        moment: moment.to_string(),
        text: text.to_string(),
        faults: Vec::new(),
    }
}
