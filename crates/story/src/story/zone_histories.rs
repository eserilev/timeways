//! "Your history here" in the story program (docs/plans/chapters.md 10): after a chapter
//! closes, its zone with the most new weight in the open world gets a rewrite. The call
//! waits behind the sagas, the tales, and the summary. A refused answer gets one retry.

use super::{Active, Output, Pending, Story, StoryError};
use crate::chapters::{ChapterSpan, SpanState};
use crate::chronicle::deed_fact;
use crate::journal::{History, deeds_in_zone};
use crate::prompt::{self, Attempt};
use crate::store::{CharacterKey, Node, Outcome, Table, ZoneHistory};
use crate::zone_history::{self, Facts, MIN_ZONE_WEIGHT};
use hourglass::{EntityId, EventId};

/// What a call of a history writes, and what its answer is checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HistoryCall {
    pub(super) key: CharacterKey,
    pub(super) zone: String,
    /// The first event of the chapter after which it is written.
    pub(super) after: EventId,
    pub(super) told: String,
    pub(super) attempt: Attempt,
    /// The prompt of the first attempt, for the retry.
    pub(super) prompt: String,
    pub(super) read: Vec<Node>,
}

impl Story {
    /// The call of the history of the newest closed chapter, once the sagas, the tales, and
    /// the summary are done.
    pub(super) fn zone_history_call(&mut self) -> Option<Output> {
        let busy = !self.calls.is_empty() || self.saga_round.is_some();
        if busy || self.summary_due.is_some() || self.pace.is_tight(self.newest) {
            return None;
        }
        let active = self.active.as_ref()?;
        if self.chapter_waiting_for_saga(active).is_some() || self.tale_waiting(active).is_some() {
            return None;
        }
        let (chapter, zone) = history_waiting(active)?;
        if !self.history_asked.insert(chapter.first) {
            return None;
        }
        let (facts, read) = facts_and_read(active, zone);
        let prompt = zone_history::prompt(&facts);
        let call = HistoryCall {
            key: active.key.clone(),
            zone: facts.zone.clone(),
            after: chapter.first,
            told: zone_history::told(&facts),
            attempt: Attempt::First,
            prompt: prompt.clone(),
            read: read.clone(),
        };
        self.open_call(Pending::ZoneHistory(Box::new(call)), prompt, read)
    }

    /// `text` is None for a failed call. A refused first answer gets one retry with its
    /// reasons. A second refusal, or a failed call, keeps the history before.
    pub(super) fn zone_history_answered(
        &mut self,
        call: HistoryCall,
        text: Option<&str>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let Some(text) = text else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        let player_text = self.player_text(&call.key);
        let Some(active) = self.active.as_mut().filter(|active| active.key == call.key) else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        let sagas = active.prose.texts();
        let sagas: Vec<&str> = sagas.iter().map(String::as_str).collect();
        match zone_history::checked_history(text, &call.told, &player_text, &sagas) {
            Ok(history) => {
                active.zone_histories.add(ZoneHistory {
                    zone: call.zone,
                    after: call.after,
                    text: history,
                })?;
                Ok((Vec::new(), Outcome::Accepted))
            }
            Err(faults) if call.attempt == Attempt::First => {
                let retry = prompt::retry(&call.prompt, text, &faults);
                let read = call.read.clone();
                let next = HistoryCall {
                    attempt: Attempt::Retry,
                    ..call
                };
                let opened = self.open_call(Pending::ZoneHistory(Box::new(next)), retry, read);
                Ok((opened.into_iter().collect(), Outcome::Refused))
            }
            Err(_) => Ok((Vec::new(), Outcome::Refused)),
        }
    }
}

/// The newest closed chapter, and its zone with the most new weight in the open world, when
/// that zone gained enough and has no history after the chapter.
fn history_waiting(active: &Active) -> Option<(ChapterSpan, EntityId)> {
    let chapter = active
        .book
        .chapters()
        .into_iter()
        .rev()
        .find(|chapter| chapter.state == SpanState::Closed)?;
    let (zone, weight) = active
        .book
        .zone_weights(chapter.steps.clone())
        .into_iter()
        .max_by_key(|(_, weight)| *weight)?;
    let written = active
        .zone_histories
        .rows()
        .iter()
        .any(|row| row.after == chapter.first);
    (weight >= MIN_ZONE_WEIGHT && !written).then_some((chapter, zone))
}

/// The newest history of each zone, for the journal.
pub(super) fn journal_histories(active: &Active) -> Vec<History> {
    let mut histories: Vec<History> = Vec::new();
    for row in active.zone_histories.rows() {
        histories.retain(|history| history.zone != row.zone);
        histories.push(History {
            zone: row.zone.clone(),
            text: row.text.clone(),
        });
    }
    histories
}

/// The newest history of a zone, and its row.
pub(super) fn newest_history<'a>(active: &'a Active, zone: &str) -> Option<(u64, &'a ZoneHistory)> {
    active
        .zone_histories
        .with_rows()
        .filter(|(_, row)| row.zone == zone)
        .last()
}

/// The deeds of the hero in the zone, newest first, and the rows behind them: their events
/// and the history before.
fn facts_and_read(active: &Active, zone: EntityId) -> (Facts, Vec<Node>) {
    let name = active
        .character
        .world()
        .entity(zone)
        .map(|entity| entity.name.clone())
        .unwrap_or_default();
    let rows = deeds_in_zone(&active.character, &active.book, zone);
    let before = newest_history(active, &name);
    let facts = Facts {
        zone: name,
        deeds: rows.iter().rev().map(|row| deed_fact(&row.deed)).collect(),
        before: before.map(|(_, row)| row.text.clone()),
        sample_turn: active.zone_histories.rows().len(),
    };
    let mut read: Vec<Node> = rows
        .iter()
        .flat_map(|row| row.events.iter())
        .map(|event| Node::Row(Table::Events, event.0))
        .collect();
    read.extend(before.map(|(row, _)| Node::Row(Table::ZoneHistories, row)));
    (facts, read)
}
