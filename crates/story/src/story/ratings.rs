//! A rating of the player in the story program (GAMEPLAY.md 3.2.2): the line names the exact
//! text, and the desktop finds it in its own rows. So the addon never sends a text, and a
//! hostile addon can put no name of a player into a rating.

use super::{Active, Output, Story, StoryError, tales};
use crate::ratings::{Rated, RatedLine, Rating, Reason, kept_reason};
use hourglass::{EventId, Tick};

/// What a rating is about, as the row keeps it.
struct Found {
    key: Option<u64>,
    moment: String,
    text: String,
    faults: Vec<String>,
}

/// The key that the line names for what it rates: the call of a narrator line, or the first
/// event of a chapter or a tale. The summary takes the row of the newest one later.
pub(super) fn named_key(rated: Rated, first: Option<u64>, line: Option<u64>) -> Option<u64> {
    match rated {
        Rated::Narrator => line,
        Rated::Chapter | Rated::Tale => first,
        Rated::Summary => None,
    }
}

impl Story {
    /// A rating of a text that the story program never showed writes no row. The line has
    /// no reply, so the note goes to the log only.
    pub(super) fn rate(
        &mut self,
        at: Tick,
        rated: Rated,
        key: Option<u64>,
        rating: Rating,
        reason: Option<Reason>,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let Some(found) = rated_text(active, rated, key) else {
            self.notes.push(format!(
                "a rating of {rated:?} {key:?}, which shows no text"
            ));
            return Ok(Vec::new());
        };
        let row = RatedLine {
            at,
            rated,
            key: found.key,
            rating,
            reason: kept_reason(rating, reason),
            moment: found.moment,
            text: found.text,
            faults: found.faults,
        };
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        active.ratings.add(row)?;
        Ok(Vec::new())
    }
}

/// The newest rating of one text, for its thumbs in the journal.
pub(super) fn rating_of(active: &Active, rated: Rated, key: Option<u64>) -> Option<Rating> {
    let rows = active.ratings.rows();
    let newest = rows
        .iter()
        .rev()
        .find(|row| row.rated == rated && row.key == key)?;
    Some(newest.rating)
}

/// The rating of the newest summary, for the thumbs of the title page.
pub(super) fn summary_rating(active: &Active) -> Option<Rating> {
    let (row, _) = active.summaries.newest()?;
    rating_of(active, Rated::Summary, Some(row))
}

/// The text of the narrator as the disk keeps it: with `$N` and the IDs of players.
fn rated_text(active: &Active, rated: Rated, key: Option<u64>) -> Option<Found> {
    let found = match rated {
        Rated::Narrator => {
            let call = key?;
            let rows = active.narrator_lines.rows();
            let line = rows.iter().rev().find(|line| line.call == call)?;
            Found {
                key: Some(call),
                moment: line.moment.clone(),
                text: line.text.clone(),
                faults: line.faults.clone(),
            }
        }
        Rated::Chapter => {
            let first = key?;
            let written = active.prose.get(EventId(first))?;
            entry(Some(first), "chapter", &written.text)
        }
        Rated::Tale => {
            let first = key?;
            let (_, tale) = tales::newest_text(active, EventId(first))?;
            entry(Some(first), "tale", &tale.text)
        }
        Rated::Summary => {
            let (row, summary) = active.summaries.newest()?;
            entry(Some(row), "summary", &summary.text)
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
