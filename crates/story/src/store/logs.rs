//! The logs of a character next to its world: sagas, flavor, the hero, what the player
//! learned, and quests. Each keeps its rows in memory, and the rows that the database does
//! not hold yet.

use super::StoreError;
use super::database::NewRow;
use crate::aliases::{self, AliasRow};
use crate::flavor::{Flavor, Told};
use crate::hero::Change;
use crate::learned::{Read, Rumor};
use crate::quest::QuestChange;
use crate::stories::StoryChange;
use hourglass::EventId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use timeways_rules::aliases::{self as rules, Alias, PlayerId};

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

/// The saga of one chapter, as one row of the chronicle: the rule that cut the chapter, and
/// its first and last events.
#[derive(Serialize, Deserialize)]
pub(super) struct ChapterProse {
    pub(super) rule: u8,
    pub(super) first: EventId,
    pub(super) last: EventId,
    pub(super) text: String,
    #[serde(default)]
    pub(super) footnotes: Vec<String>,
}

/// The chapter of a saga: the rule that cut it, and its first and last events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SagaSpan {
    pub rule: u8,
    pub first: EventId,
    pub last: EventId,
}

/// The saga of one chapter and its footnotes, as the player reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub text: String,
    pub footnotes: Vec<String>,
}

/// The saga of each chapter, by the first event of the chapter. The world holds facts
/// only, so the words live in a table of their own.
#[derive(Debug, Default)]
pub struct Prose {
    chapters: BTreeMap<EventId, Written>,
    /// The row of the saga of each chapter, for the reads of a call.
    rows: BTreeMap<EventId, u64>,
    unsaved: Unsaved,
}

impl Prose {
    pub(super) fn from_rows(rows: Vec<ChapterProse>) -> Prose {
        let unsaved = Unsaved::after(rows.len());
        let positions = (0..)
            .zip(&rows)
            .map(|(row, prose)| (prose.first, row))
            .collect();
        let chapters = rows
            .into_iter()
            .map(|row| {
                let written = Written {
                    text: row.text,
                    footnotes: row.footnotes,
                };
                (row.first, written)
            })
            .collect();
        Prose {
            chapters,
            rows: positions,
            unsaved,
        }
    }

    /// The row that holds the saga of the chapter whose first event is `first`.
    #[must_use]
    pub fn row_of(&self, first: EventId) -> Option<u64> {
        self.rows.get(&first).copied()
    }

    #[must_use]
    pub fn get(&self, first: EventId) -> Option<&Written> {
        self.chapters.get(&first)
    }

    /// The sagas of the chapters before the one whose first event is `first`.
    pub fn before(&self, first: EventId) -> impl Iterator<Item = &Written> {
        self.chapters.range(..first).map(|(_, written)| written)
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
    pub fn add(&mut self, span: SagaSpan, written: Written) -> Result<(), StoreError> {
        let row = ChapterProse {
            rule: span.rule,
            first: span.first,
            last: span.last,
            text: written.text.clone(),
            footnotes: written.footnotes.clone(),
        };
        let position = self.unsaved.push(&row)?;
        self.chapters.insert(span.first, written);
        self.rows.insert(span.first, position);
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
    /// The row of each rumor, in the order of `rumors`.
    rumor_rows: Vec<u64>,
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
                LearnedLine::Rumor(rumor) => {
                    learned.rumors.push(rumor);
                    learned.rumor_rows.push(row);
                }
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

    /// Each rumor with its row.
    pub fn rumors_with_rows(&self) -> impl Iterator<Item = (u64, &Rumor)> {
        self.rumor_rows.iter().copied().zip(&self.rumors)
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
        let row = self.unsaved.push(&LearnedLine::Rumor(rumor.clone()))?;
        self.rumors.push(rumor);
        self.rumor_rows.push(row);
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

/// One summary of the character: written after the chapter whose first event is `after`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub after: EventId,
    pub text: String,
}

/// The summaries of the character, oldest first. The newest one stands.
#[derive(Debug, Default)]
pub struct SummaryLog {
    rows: Vec<Summary>,
    unsaved: Unsaved,
}

impl SummaryLog {
    pub(super) fn from_rows(rows: Vec<Summary>) -> SummaryLog {
        let unsaved = Unsaved::after(rows.len());
        SummaryLog { rows, unsaved }
    }

    /// The newest summary, and its row.
    #[must_use]
    pub fn newest(&self) -> Option<(u64, &Summary)> {
        let row = self.rows.len().checked_sub(1)?;
        Some((row as u64, &self.rows[row]))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// # Errors
    ///
    /// Returns `Json` for a summary that does not serialize, and then keeps nothing.
    pub fn add(&mut self, summary: Summary) -> Result<(), StoreError> {
        self.unsaved.push(&summary)?;
        self.rows.push(summary);
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

/// The alias table (GAMEPLAY.md 5.11): each player that a text named, at the place of its
/// ID. Rows only grow, so an ID is never reused.
pub struct AliasLog {
    rows: Vec<AliasRow>,
    table: Vec<Alias>,
    unsaved: Unsaved,
}

impl AliasLog {
    /// The rows must hold names, each once (`is_new_name`).
    pub(super) fn from_rows(rows: Vec<AliasRow>) -> AliasLog {
        let mut table = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(alias) = aliases::alias_of(&row.name) {
                rules::learn(&mut table, alias);
            }
        }
        let unsaved = Unsaved::after(rows.len());
        AliasLog {
            rows,
            table,
            unsaved,
        }
    }

    #[must_use]
    pub fn table(&self) -> &[Alias] {
        &self.table
    }

    #[must_use]
    pub fn row(&self, id: PlayerId) -> Option<&AliasRow> {
        self.rows.get(id.0)
    }

    /// The ID of the player of the row. A new name gets the next ID, and keeps the race
    /// and the class of its row. A text that is no name gets none.
    ///
    /// # Errors
    ///
    /// Returns `Json` for a row that does not serialize, and then keeps nothing.
    pub fn learn(&mut self, row: &AliasRow) -> Result<Option<PlayerId>, StoreError> {
        let Some(alias) = aliases::alias_of(&row.name) else {
            return Ok(None);
        };
        if let Some(id) = rules::find(&self.table, &alias.key) {
            return Ok(Some(id));
        }
        let row = AliasRow {
            name: alias.shown.clone(),
            race: row.race,
            class: row.class,
        };
        self.unsaved.push(&row)?;
        self.rows.push(row);
        Ok(Some(rules::learn(&mut self.table, alias)))
    }

    /// The cards of the players of a text for a model, in order.
    #[must_use]
    pub fn cards(&self, text: &str) -> Vec<String> {
        aliases::ids_in(text)
            .into_iter()
            .filter_map(|id| self.row(id).map(|row| row.card(id)))
            .collect()
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}

/// A row that the alias table can hold: a name, and not one that a row before it holds.
/// `seen` gathers the keys of the rows before.
pub(super) fn is_new_name(row: &AliasRow, seen: &mut std::collections::BTreeSet<String>) -> bool {
    aliases::player_name(&row.name) == Some(row.name.as_str())
        && seen.insert(aliases::key_of(&row.name))
}

/// The rows of a table that only grows, oldest first. The row of an item is its place in
/// the list.
#[derive(Debug)]
pub struct RowLog<T> {
    rows: Vec<T>,
    unsaved: Unsaved,
}

impl<T> Default for RowLog<T> {
    fn default() -> Self {
        RowLog {
            rows: Vec::new(),
            unsaved: Unsaved::default(),
        }
    }
}

impl<T: Serialize> RowLog<T> {
    pub(super) fn from_rows(rows: Vec<T>) -> RowLog<T> {
        let unsaved = Unsaved::after(rows.len());
        RowLog { rows, unsaved }
    }

    #[must_use]
    pub fn rows(&self) -> &[T] {
        &self.rows
    }

    /// The rows with their places.
    pub fn with_rows(&self) -> impl Iterator<Item = (u64, &T)> {
        (0..).zip(&self.rows)
    }

    /// The row of the new item.
    ///
    /// # Errors
    ///
    /// Returns `Json` for an item that does not serialize, and then keeps nothing.
    pub fn add(&mut self, item: T) -> Result<u64, StoreError> {
        let row = self.unsaved.push(&item)?;
        self.rows.push(item);
        Ok(row)
    }

    pub fn take_unsaved(&mut self) -> Vec<NewRow> {
        self.unsaved.take()
    }
}
