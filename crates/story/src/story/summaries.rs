//! The summary of the character in the story program (docs/plans/hero-stories.md 3.5): when it
//! is due, its call, and its answer.

use super::aliases::{TextLimits, shows_with_names};
use super::{Active, Output, Pending, Story, StoryError};
use crate::chapters::SpanState;
use crate::check::copies_a_sample;
use crate::chronicle::deed_fact;
use crate::hero::{self, Change, FIELDS};
use crate::journal::{Chapter, Deed, EntryState};
use crate::memory;
use crate::narrator::Who;
use crate::store::{CharacterKey, Node, Outcome, Summary, Table};
use crate::summary::{self, Facts, MAX_DEEDS, MAX_SAGAS};
use hourglass::EventId;

use super::prologues::{PROLOGUE_KEY, has_prologue, prologue_text};

const SUMMARY_LIMITS: TextLimits = TextLimits {
    chars: summary::MAX_SUMMARY_CHARS,
    bytes: summary::MAX_SUMMARY_BYTES,
};

/// The six questions of the sheet. The Roleplay Profile stays out: the title can name a
/// real player.
const QUESTIONS: usize = 6;

impl Story {
    /// The call of the summary that waits. It opens only while no other call is open, the
    /// pace window is not tight, and no chapter waits for its saga: the sagas go first.
    pub(super) fn summary_call(&mut self) -> Option<Output> {
        let busy = !self.calls.is_empty() || self.saga_round.is_some();
        if busy || self.pace.is_tight(self.newest) {
            return None;
        }
        let active = self.active.as_ref()?;
        let after = self.summary_waiting(active)?;
        if self.tale_waiting(active).is_some() {
            return None;
        }
        self.summary_asked.insert(after);
        let (facts, read) = facts_and_read(active, after);
        let pending = Pending::Summary {
            key: active.key.clone(),
            after,
            told: summary::told(&facts),
        };
        self.open_call(pending, summary::prompt(&facts), read)
    }

    /// The first event of the newest closed chapter, once no chapter waits for its saga,
    /// when no summary covers it and none was asked for it in this run. Only the newest
    /// counts: a summary of an older chapter would be replaced at once. Before the first
    /// closed chapter, a written prologue counts as chapter 0.
    pub(super) fn summary_waiting(&self, active: &Active) -> Option<EventId> {
        if self.chapter_waiting_for_saga(active).is_some() {
            return None;
        }
        let newest = active
            .book
            .chapters()
            .into_iter()
            .rev()
            .find(|chapter| chapter.state == SpanState::Closed)
            .map(|chapter| chapter.first)
            .or_else(|| has_prologue(active).then_some(PROLOGUE_KEY))?;
        let covered = active
            .summaries
            .newest()
            .is_some_and(|(_, summary)| summary.after >= newest);
        let asked = self.summary_asked.contains(&newest);
        (!covered && !asked).then_some(newest)
    }

    /// `text` is None for a failed call. A refused summary or a failed call keeps the one
    /// before it.
    pub(super) fn summary_answered(
        &mut self,
        key: &CharacterKey,
        after: EventId,
        told: &str,
        text: Option<&str>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let player_text = self.player_text(key);
        let tellings = self
            .active
            .as_ref()
            .map(summary_tellings)
            .unwrap_or_default();
        let tellings: Vec<&str> = tellings.iter().map(String::as_str).collect();
        // A summary that copies 8 words of the player's telling is refused, as a saga is.
        let checked = text
            .and_then(|text| summary::checked_summary(text, told, &player_text))
            .filter(|summary| !copies_a_sample(summary, &tellings))
            .filter(|summary| {
                let active = self.active.as_ref();
                active.is_some_and(|active| shows_with_names(active, summary, SUMMARY_LIMITS))
            });
        let active = self.active.as_mut().filter(|active| &active.key == key);
        let (Some(active), Some(text)) = (active, checked) else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        active.summaries.add(Summary { after, text })?;
        Ok((Vec::new(), Outcome::Accepted))
    }
}

/// The player's tellings of every closed chapter, for the copy check of a summary.
fn summary_tellings(active: &Active) -> Vec<String> {
    let journal = active.journal();
    let chapters: Vec<&Chapter> = journal.chapters.iter().collect();
    chapter_tellings(active, &chapters)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

/// The newest summary with the names of the players, for the title page of the Chronicle,
/// unless the player's own summary stands in its place.
pub(super) fn journal_summary(active: &Active) -> Option<Box<str>> {
    let (row, summary) = active.summaries.newest()?;
    let shown = super::edits::shown_of(active, super::edits::SUMMARY_KEY, &[row]);
    shown
        .narrator
        .map(|_| super::aliases::with_names(active, &summary.text).into_boxed_str())
}

/// The closed chapters up to the one whose first event is `after`, newest first.
fn finished_up_to(chapters: &[Chapter], after: EventId) -> Vec<&Chapter> {
    let mut finished: Vec<&Chapter> = chapters
        .iter()
        .filter(|chapter| chapter.state == EntryState::Closed && chapter.first <= after.0)
        .collect();
    finished.reverse();
    finished.truncate(MAX_SAGAS);
    finished
}

/// First kills and class quests, newest first.
fn deeds_of_note(deeds: &[Deed]) -> Vec<&Deed> {
    let of_note = |deed: &&Deed| {
        matches!(
            deed,
            Deed::Defeated { times: 1, .. } | Deed::ClassQuestDone { .. }
        )
    };
    deeds.iter().rev().filter(of_note).take(MAX_DEEDS).collect()
}

/// The names of a deed, for the events behind it.
fn deed_name(deed: &Deed) -> Option<&str> {
    match deed {
        Deed::Defeated { foe, .. } => Some(foe),
        Deed::ClassQuestDone { title, .. } => Some(title),
        _ => None,
    }
}

/// What the prompt tells, and the rows behind it: the hero rows of the six questions, the
/// saga and the events of each chapter of the prompt, the events behind each deed and the
/// level, and the summary before it. Reading too much only counts a row as used.
fn facts_and_read(active: &Active, after: EventId) -> (Facts, Vec<Node>) {
    let world = active.journal();
    let hero = hero::hero(active.hero.changes());
    let chapters = finished_up_to(&world.chapters, after);
    let tellings = chapter_tellings(active, &chapters);
    let deeds = deeds_of_note(&world.deeds);
    let before = active.summaries.newest();
    // The prologue is the oldest chapter, so it comes last, when the prompt has room.
    let prologue = prologue_text(active).filter(|_| chapters.len() < MAX_SAGAS);
    let mut texts: Vec<String> = chapters
        .iter()
        .map(|chapter| match active.prose.get(EventId(chapter.first)) {
            Some(written) => written.text.clone(),
            None => memory::summary(chapter),
        })
        .collect();
    texts.extend(prologue.as_ref().map(|(text, _)| text.clone()));
    let facts = Facts {
        who: Who::of(&active.character).described(),
        level: active.character.level(),
        sheet: hero::portrait(&hero, &[]),
        before: before.map(|(_, summary)| summary.text.clone()),
        chapters: texts,
        deeds: deeds.iter().map(|deed| deed_fact(deed)).collect(),
        tellings: tellings.iter().map(|(text, _)| text.clone()).collect(),
        sample_turn: active.summaries.len(),
    };
    let mut read = question_rows(active);
    for chapter in &chapters {
        let first = EventId(chapter.first);
        read.extend(
            active
                .prose
                .row_of(first)
                .map(|row| Node::Row(Table::Chapters, row)),
        );
        read.extend(chapter_events(active, first));
    }
    read.extend(super::reads::events_about(
        active,
        deeds.iter().filter_map(|deed| deed_name(deed)),
    ));
    read.extend(super::reads::level_read(active));
    read.extend(tellings.into_iter().map(|(_, row)| row));
    read.extend(before.map(|(row, _)| Node::Row(Table::Summaries, row)));
    read.extend(prologue.map(|(_, row)| row));
    (facts, read)
}

/// The player's telling of each chapter of the prompt that has one, and its row.
fn chapter_tellings(active: &Active, chapters: &[&Chapter]) -> Vec<(String, Node)> {
    chapters
        .iter()
        .filter_map(|chapter| {
            super::edits::telling_of(active, super::edits::chapter_key(chapter.first))
        })
        .collect()
}

fn question_rows(active: &Active) -> Vec<Node> {
    let question = |field: &str| FIELDS[..QUESTIONS].contains(&field);
    (0..)
        .zip(active.hero.changes())
        .filter(|(_, change)| matches!(change, Change::Set { field, .. } if question(field)))
        .map(|(row, _)| Node::Row(Table::Hero, row))
        .collect()
}

/// The events of the chapter whose first event is `first`.
fn chapter_events(active: &Active, first: EventId) -> Vec<Node> {
    let span = active
        .book
        .chapters()
        .into_iter()
        .find(|span| span.first == first);
    span.map(|span| span.first.0..=span.last.0)
        .into_iter()
        .flatten()
        .map(|event| Node::Row(Table::Events, event))
        .collect()
}
