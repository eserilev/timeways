//! The text of each tale in the story program (docs/plans/chapters.md 6): when a run with
//! something new closed, one call, and its answer. The calls wait behind the sagas, and the
//! summary waits behind them.

use super::{Active, Output, Pending, Story, StoryError};
use crate::chapters::{SpanState, TaleSpan, VisitSpan};
use crate::chronicle::deed_fact;
use crate::journal::{Tale, visit_deeds};
use crate::store::{CharacterKey, Node, Outcome, Table, TaleText};
use crate::tale::{self, Facts};
use hourglass::EventId;

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
        let (facts, read) = facts_and_read(active, &span, &visit, tale);
        self.tale_asked.insert(run.visit);
        let pending = Pending::Tale {
            key: active.key.clone(),
            run,
            instance: tale.instance.clone(),
            told: tale::told(&facts),
        };
        self.open_call(pending, tale::prompt(&facts), read)
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
        text: Option<&str>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let player_text = self.player_text(key);
        let checked = text.and_then(|text| tale::checked_tale(text, told, &player_text, &[]));
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

/// The newest text of the tale whose first event is `tale`, and its row.
pub(super) fn newest_text(active: &Active, tale: EventId) -> Option<(u64, &TaleText)> {
    active
        .tales
        .with_rows()
        .filter(|(_, row)| row.tale == tale)
        .last()
}

/// What the prompt tells, and the rows behind it: the events of every run of the tale and
/// the text before.
fn facts_and_read(
    active: &Active,
    span: &TaleSpan,
    visit: &VisitSpan,
    tale: &Tale,
) -> (Facts, Vec<Node>) {
    let before = newest_text(active, span.first);
    let new_run = visit_deeds(&active.character, &active.book, visit);
    let facts = Facts {
        instance: tale.kind.described(&tale.instance),
        runs: span.runs,
        deeds: tale.deeds.iter().map(deed_fact).collect(),
        new_run: new_run.iter().map(deed_fact).collect(),
        before: before.map(|(_, row)| row.text.clone()),
        telling: None,
        sample_turn: active.tales.rows().len(),
    };
    let mut read: Vec<Node> = span
        .visits
        .iter()
        .flat_map(|visit| visit.first.0..=visit.last.0)
        .map(|event| Node::Row(Table::Events, event))
        .collect();
    read.extend(before.map(|(row, _)| Node::Row(Table::Tales, row)));
    (facts, read)
}
