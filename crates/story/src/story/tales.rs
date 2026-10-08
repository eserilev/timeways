//! The text of each tale in the story program (docs/plans/chapters.md 6): when a run with
//! something new closed, one call, and its answer. The calls wait behind the sagas, and the
//! summary waits behind them.

use super::aliases::{TextLimits, shows_with_names};
use super::calls::Reply;
use super::{Active, Output, Pending, Story, StoryError};
use crate::chapters::{SpanState, TaleSpan, VisitSpan};
use crate::chronicle::deed_fact;
use crate::journal::{DeedRow, Tale, tale_deeds, visit_deeds};
use crate::store::{CharacterKey, Node, Outcome, Table, TaleText};
use crate::tale::{self, Facts};
use hourglass::EventId;

const TALE_LIMITS: TextLimits = TextLimits {
    chars: tale::MAX_TALE_CHARS,
    bytes: tale::MAX_TALE_BYTES,
};

/// The tale and the run that its next text covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TaleRun {
    pub(super) tale: EventId,
    pub(super) visit: EventId,
}

impl Story {
    /// The call of the oldest run with something new that no text covers yet. It opens only
    /// while no other call is open, the pace window is not tight, and no chapter waits for
    /// its saga.
    pub(super) fn tale_call(&mut self) -> Option<Output> {
        let busy = !self.calls.is_empty() || self.saga_round.is_some();
        if busy || self.pace.is_tight(self.newest) {
            return None;
        }
        let active = self.active.as_ref()?;
        if self.chapter_waiting_for_saga(active).is_some() {
            return None;
        }
        let (span, visit) = self.tale_waiting(active)?;
        let run = TaleRun {
            tale: span.first,
            visit: visit.first,
        };
        let journal = active.journal();
        let tale = journal
            .tales
            .iter()
            .find(|tale| tale.first == span.first.0)?;
        let (facts, prompt, read) = prompt_and_read(active, &span, &visit, tale);
        self.tale_asked.insert(run.visit);
        let pending = Pending::Tale {
            key: active.key.clone(),
            run,
            instance: tale.instance.clone(),
            told: tale::told(&facts),
        };
        self.open_call(pending, prompt, read)
    }

    /// The oldest tale whose newest closed run with something new has no text and was not
    /// asked for one in this run of the program.
    pub(super) fn tale_waiting(&self, active: &Active) -> Option<(TaleSpan, VisitSpan)> {
        let mut waiting: Option<(TaleSpan, VisitSpan)> = None;
        for span in active.book.tales() {
            let Some(visit) = newest_run_with_gain(&span) else {
                continue;
            };
            let written = active
                .tales
                .rows()
                .iter()
                .any(|row| row.tale == span.first && row.visit == visit.first);
            let asked = self.tale_asked.contains(&visit.first);
            let older = waiting
                .as_ref()
                .is_none_or(|(_, kept)| visit.first < kept.first);
            if !written && !asked && older {
                waiting = Some((span, visit));
            }
        }
        waiting
    }

    /// `text` is None for a failed call. A refused text or a failed call keeps the text
    /// before.
    pub(super) fn tale_answered(
        &mut self,
        key: &CharacterKey,
        run: TaleRun,
        instance: String,
        told: &str,
        reply: Option<&Reply<'_>>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let player_text = self.player_text(key);
        let telling = self.active.as_ref().and_then(|active| {
            super::edits::telling_of(active, super::edits::tale_key(run.tale.0))
        });
        let telling: Vec<&str> = telling.iter().map(|(text, _)| text.as_str()).collect();
        let checked = reply
            .and_then(|reply| {
                tale::checked_tale(reply.text, told, &player_text, &telling, reply.given)
            })
            .filter(|text| {
                let active = self.active.as_ref();
                active.is_some_and(|active| shows_with_names(active, text, TALE_LIMITS))
            });
        let active = self.active.as_mut().filter(|active| &active.key == key);
        let (Some(active), Some(text)) = (active, checked) else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        active.tales.add(TaleText {
            tale: run.tale,
            instance,
            visit: run.visit,
            text,
        })?;
        Ok((Vec::new(), Outcome::Accepted))
    }
}

fn newest_run_with_gain(span: &TaleSpan) -> Option<VisitSpan> {
    span.visits
        .iter()
        .rev()
        .find(|visit| visit.state == SpanState::Closed && visit.gain > 0)
        .cloned()
}

/// The text that the tale shows, with the names of the players: its newest text of the
/// narrator, unless the player's own text stands in its place.
pub(super) fn shown_text(active: &Active, tale: EventId) -> Option<String> {
    let rows: Vec<u64> = active
        .tales
        .with_rows()
        .filter(|(_, row)| row.tale == tale)
        .map(|(row, _)| row)
        .collect();
    let shown = super::edits::shown_of(active, super::edits::tale_key(tale.0), &rows);
    let row = shown.narrator?;
    active
        .tales
        .rows()
        .get(usize::try_from(row).ok()?)
        .map(|text| super::aliases::with_names(active, &text.text))
}

/// The newest text of the tale whose first event is `tale`, and its row.
pub(super) fn newest_text(active: &Active, tale: EventId) -> Option<(u64, &TaleText)> {
    active
        .tales
        .with_rows()
        .filter(|(_, row)| row.tale == tale)
        .last()
}

/// What the prompt tells, the prompt, and the rows behind it: the events of the deeds that
/// the prompt holds, the text before, and the player's telling. A raid cleared 200 times
/// still reads only a few rows.
fn prompt_and_read(
    active: &Active,
    span: &TaleSpan,
    visit: &VisitSpan,
    tale: &Tale,
) -> (Facts, String, Vec<Node>) {
    let before = newest_text(active, span.first);
    let deeds = tale_deeds(&active.character, &active.book, span);
    let mut new_run = visit_deeds(&active.character, &active.book, visit);
    new_run.truncate(tale::MAX_DEEDS);
    let telling = super::edits::telling_of(active, super::edits::tale_key(span.first.0));
    let facts = Facts {
        instance: tale.kind.described(&tale.instance),
        runs: span.runs,
        deeds: deeds.iter().map(|row| deed_fact(&row.deed)).collect(),
        new_run: new_run.iter().map(|row| deed_fact(&row.deed)).collect(),
        before: before.map(|(_, row)| row.text.clone()),
        telling: telling.as_ref().map(|(text, _)| text.clone()),
        sample_turn: active.tales.rows().len(),
    };
    let (prompt, kept) = tale::prompt_and_kept(&facts);
    let newest = &deeds[deeds.len() - kept..];
    let mut read = events_of(newest.iter().chain(&new_run));
    read.extend(before.map(|(row, _)| Node::Row(Table::Tales, row)));
    read.extend(telling.map(|(_, row)| row));
    (facts, prompt, read)
}

/// The events behind the deeds, each once.
pub(super) fn events_of<'a>(deeds: impl Iterator<Item = &'a DeedRow>) -> Vec<Node> {
    let mut read: Vec<Node> = Vec::new();
    for event in deeds.flat_map(|row| &row.events) {
        let node = Node::Row(Table::Events, event.0);
        if !read.contains(&node) {
            read.push(node);
        }
    }
    read
}
