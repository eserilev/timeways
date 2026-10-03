//! The logs of a character next to its world: sagas, flavor, the hero, what the player
//! learned, and quests. Each keeps its rows in memory, and the rows that the database does
//! not hold yet.

use super::StoreError;
use super::database::NewRow;
use crate::flavor::{Flavor, Told};
use crate::hero::Change;
use crate::learned::{Read, Rumor};
use crate::quest::QuestChange;
use crate::stories::StoryChange;
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The new rows of one table, each at the next position.
#[derive(Debug, Default)]
pub struct Unsaved {
    next: u64,
    rows: Vec<NewRow>,
}

impl Unsaved {
    pub(super) fn after(saved: usize) -> Unsaved {
        Unsaved {
            next: saved as u64,
            rows: Vec::new(),
        }
    }

    /// The position of the new row.
    fn push<T: Serialize>(&mut self, item: &T) -> Result<u64, StoreError> {
        let position = self.next;
        self.rows.push(NewRow {
            position,
            body: serde_json::to_string(item)?,
        });
        self.next += 1;
        Ok(position)
    }

    fn take(&mut self) -> Vec<NewRow> {
        std::mem::take(&mut self.rows)
    }
}

/// The saga of one chapter, as one row of the chronicle. A row from before footnotes has
/// none.
#[derive(Serialize, Deserialize)]
pub(super) struct ChapterProse {
    pub(super) began: Tick,
    pub(super) text: String,
    #[serde(default)]
    pub(super) footnotes: Vec<String>,
}

/// The saga of one chapter and its footnotes, as the player reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub text: String,
    pub footnotes: Vec<String>,
}

/// The saga of each chapter, by the tick that began the chapter. The world holds facts
/// only, so the words live in a table of their own.
#[derive(Debug, Default)]
pub struct Prose {
    chapters: BTreeMap<Tick, Written>,
    unsaved: Unsaved,
}

impl Prose {
    pub(super) fn from_rows(rows: Vec<ChapterProse>) -> Prose {
        let unsaved = Unsaved::after(rows.len());
        let chapters = rows
            .into_iter()
            .map(|row| {
                let written = Written {
                    text: row.text,
                    footnotes: row.footnotes,
                };
                (row.began, written)
            })
            .collect();
        Prose { chapters, unsaved }
    }

    #[must_use]
    pub fn get(&self, began: Tick) -> Option<&Written> {
        self.chapters.get(&began)
    }

    /// The sagas of the chapters that began before `began`.
    pub fn before(&self, began: Tick) -> impl Iterator<Item = &Written> {
        self.chapters.range(..began).map(|(_, written)| written)
    }

    /// The chapters that have a saga.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chapters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chapters.is_empty()
    }

    /// # Errors
    ///
    /// Returns `Json` for a saga that does not serialize, and then keeps nothing.
    pub fn add(&mut self, began: Tick, written: Written) -> Result<(), StoreError> {
        let row = ChapterProse {
            began,
            text: written.text.clone(),
            footnotes: written.footnotes.clone(),
        };
        self.unsaved.push(&row)?;
        self.chapters.insert(began, written);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}

/// One row of the flavor table: a moment, or a telling of a kind of moment.
#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub(super) enum FlavorLine {
    Moment(Flavor),
    Told(Told),
}

/// The flavor moments of a character and the tellings of them (GAMEPLAY.md 5.4.1). They are
/// small and silly, not facts, so they live in a table of their own.
#[derive(Debug, Default)]
pub struct FlavorLog {
    moments: Vec<Flavor>,
    /// The row of each moment, in the order of `moments`.
    moment_rows: Vec<u64>,
    told: Vec<Told>,
    unsaved: Unsaved,
}

impl FlavorLog {
    pub(super) fn from_rows(rows: Vec<FlavorLine>) -> FlavorLog {
        let mut flavor = FlavorLog {
            unsaved: Unsaved::after(rows.len()),
            ..FlavorLog::default()
        };
        for (row, line) in (0..).zip(rows) {
            match line {
                FlavorLine::Moment(moment) => {
                    flavor.moments.push(moment);
                    flavor.moment_rows.push(row);
                }
                FlavorLine::Told(told) => flavor.told.push(told),
            }
        }
        flavor
    }

    #[must_use]
    pub fn moments(&self) -> &[Flavor] {
        &self.moments
    }

    /// Each moment with its row.
    pub fn moments_with_rows(&self) -> impl Iterator<Item = (u64, &Flavor)> {
        self.moment_rows.iter().copied().zip(&self.moments)
    }

    #[must_use]
    pub fn told(&self) -> &[Told] {
        &self.told
    }

    /// # Errors
    ///
    /// Returns `Json` for a moment that does not serialize, and then keeps nothing.
    pub fn add_moment(&mut self, moment: Flavor) -> Result<(), StoreError> {
        let row = self.unsaved.push(&FlavorLine::Moment(moment.clone()))?;
        self.moments.push(moment);
        self.moment_rows.push(row);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns `Json` for a telling that does not serialize, and then keeps nothing.
    pub fn add_told(&mut self, told: Told) -> Result<(), StoreError> {
        self.unsaved.push(&FlavorLine::Told(told.clone()))?;
        self.told.push(told);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}

/// The changes of the story of the hero, oldest first (see `hero`). They are the player's
/// own words, not facts, so they live in a table of their own. The row of a change is its
/// place in the list.
#[derive(Debug, Default)]
pub struct HeroLog {
    changes: Vec<Change>,
    unsaved: Unsaved,
}

impl HeroLog {
    pub(super) fn from_rows(changes: Vec<Change>) -> HeroLog {
        let unsaved = Unsaved::after(changes.len());
        HeroLog { changes, unsaved }
    }

    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns `Json` for a change that does not serialize, and then keeps nothing.
    pub fn add(&mut self, change: Change) -> Result<(), StoreError> {
        self.unsaved.push(&change)?;
        self.changes.push(change);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub(super) enum LearnedLine {
    Read(Read),
    Rumor(Rumor),
}

/// What the player read and heard, oldest first (see `learned`).
#[derive(Debug, Default)]
pub struct LearnedLog {
    read: Vec<Read>,
    /// The row of each text, in the order of `read`.
    read_rows: Vec<u64>,
    rumors: Vec<Rumor>,
    unsaved: Unsaved,
}

impl LearnedLog {
    pub(super) fn from_rows(rows: Vec<LearnedLine>) -> LearnedLog {
        let mut learned = LearnedLog {
            unsaved: Unsaved::after(rows.len()),
            ..LearnedLog::default()
        };
        for (row, line) in (0..).zip(rows) {
            match line {
                LearnedLine::Read(read) => {
                    learned.read.push(read);
                    learned.read_rows.push(row);
                }
                LearnedLine::Rumor(rumor) => learned.rumors.push(rumor),
            }
        }
        learned
    }

    #[must_use]
    pub fn read(&self) -> &[Read] {
        &self.read
    }

    /// Each text with its row.
    pub fn read_with_rows(&self) -> impl Iterator<Item = (u64, &Read)> {
        self.read_rows.iter().copied().zip(&self.read)
    }

    #[must_use]
    pub fn rumors(&self) -> &[Rumor] {
        &self.rumors
    }

    /// # Errors
    ///
    /// Returns `Json` for a text that does not serialize, and then keeps nothing.
    pub fn add_read(&mut self, read: Read) -> Result<(), StoreError> {
        let row = self.unsaved.push(&LearnedLine::Read(read.clone()))?;
        self.read.push(read);
        self.read_rows.push(row);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns `Json` for a rumor that does not serialize, and then keeps nothing.
    pub fn add_rumor(&mut self, rumor: Rumor) -> Result<(), StoreError> {
        self.unsaved.push(&LearnedLine::Rumor(rumor.clone()))?;
        self.rumors.push(rumor);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}

/// The changes of the side quests, oldest first (see `quest`). The row of a change is its
/// place in the list.
#[derive(Debug, Default)]
pub struct QuestLog {
    changes: Vec<QuestChange>,
    unsaved: Unsaved,
}

impl QuestLog {
    pub(super) fn from_rows(changes: Vec<QuestChange>) -> QuestLog {
        let unsaved = Unsaved::after(changes.len());
        QuestLog { changes, unsaved }
    }

    #[must_use]
    pub fn changes(&self) -> &[QuestChange] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns `Json` for a change that does not serialize, and then keeps nothing.
    pub fn add(&mut self, change: QuestChange) -> Result<(), StoreError> {
        self.unsaved.push(&change)?;
        self.changes.push(change);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}

/// The changes of the player stories, oldest first (see `stories`). The row of a change is
/// its place in the list.
#[derive(Debug, Default)]
pub struct StoryLog {
    changes: Vec<StoryChange>,
    unsaved: Unsaved,
}

impl StoryLog {
    pub(super) fn from_rows(changes: Vec<StoryChange>) -> StoryLog {
        let unsaved = Unsaved::after(changes.len());
        StoryLog { changes, unsaved }
    }

    #[must_use]
    pub fn changes(&self) -> &[StoryChange] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns `Json` for a change that does not serialize, and then keeps nothing.
    pub fn add(&mut self, change: StoryChange) -> Result<(), StoreError> {
        self.unsaved.push(&change)?;
        self.changes.push(change);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}
