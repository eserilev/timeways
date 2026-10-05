//! The summary of the character in the story program (docs/plans/hero-stories.md 3.5): when it
//! is due, its call, and its answer.

use super::{Active, Output, Pending, Story, StoryError};
use crate::chronicle::deed_fact;
use crate::hero::{self, Change, FIELDS};
use crate::journal::{Chapter, Deed, journal};
use crate::memory;
use crate::narrator::Who;
use crate::store::{CharacterKey, Node, Outcome, Summary, Table};
use crate::summary::{self, Facts, MAX_DEEDS, MAX_SAGAS};
use hourglass::Tick;

/// The summary of the chapter that began at `began` waits for its call.
pub(super) struct Due {
    pub(super) key: CharacterKey,
    pub(super) began: Tick,
}

/// The six questions of the sheet. The Roleplay Profile stays out: the title can name a
/// real player.
const QUESTIONS: usize = 6;

impl Story {
    /// The saga round of a chapter ended, with a saga or with none. A newer chapter takes the
    /// place of an older one that never got its call: its summary would be replaced at once.
    pub(super) fn summary_is_due(&mut self, key: CharacterKey, began: Tick) {
        self.summary_due = Some(Due { key, began });
    }

    /// The call of the summary that is due. It opens only while no other call is open, the
    /// pace window is not tight, and no older chapter waits for its saga: the sagas go first.
    pub(super) fn summary_call(&mut self) -> Option<Output> {
        let due = self.summary_due.as_ref()?;
        let busy = !self.calls.is_empty() || self.saga_round.is_some();
        if busy || self.pace.is_tight(self.newest) {
            return None;
        }
        let active = self
            .active
            .as_ref()
            .filter(|active| active.key == due.key)?;
        if self.chapter_waiting_for_saga(active).is_some() {
            return None;
        }
        let due = self.summary_due.take()?;
        let (facts, read) = facts_and_read(active, due.began);
        let pending = Pending::Summary {
            key: due.key,
            after: due.began,
            told: summary::told(&facts),
        };
        self.open_call(pending, summary::prompt(&facts), read)
    }

    /// `text` is None for a failed call. A refused summary or a failed call keeps the one
    /// before it.
    pub(super) fn summary_answered(
        &mut self,
        key: &CharacterKey,
        after: Tick,
        told: &str,
        text: Option<&str>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let player_text = self.player_text(key);
        let checked = text.and_then(|text| summary::checked_summary(text, told, &player_text));
        let active = self.active.as_mut().filter(|active| &active.key == key);
        let (Some(active), Some(text)) = (active, checked) else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        active.summaries.add(Summary { after, text })?;
        Ok((Vec::new(), Outcome::Accepted))
    }
}

/// The newest summary, for the title page of the Chronicle.
pub(super) fn journal_summary(active: &Active) -> Option<Box<str>> {
    active
        .summaries
        .newest()
        .map(|(_, summary)| summary.text.clone().into_boxed_str())
}

/// The finished chapters up to the one that began at `began`, newest first, each with the
/// tick where the next one began.
fn finished_up_to(chapters: &[Chapter], began: Tick) -> Vec<(&Chapter, Tick)> {
    let mut finished: Vec<(&Chapter, Tick)> = chapters
        .windows(2)
        .map(|pair| (&pair[0], pair[1].began))
        .filter(|(chapter, _)| chapter.began <= began)
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
fn facts_and_read(active: &Active, began: Tick) -> (Facts, Vec<Node>) {
    let world = journal(&active.character);
    let hero = hero::hero(active.hero.changes());
    let chapters = finished_up_to(&world.chapters, began);
    let deeds = deeds_of_note(&world.deeds);
    let before = active.summaries.newest();
    let facts = Facts {
        who: Who::of(&active.character).described(),
        level: active.character.level(),
        sheet: hero::portrait(&hero, &[]),
        before: before.map(|(_, summary)| summary.text.clone()),
        chapters: chapters
            .iter()
            .map(|(chapter, _)| match active.prose.get(chapter.began) {
                Some(written) => written.text.clone(),
                None => memory::summary(chapter),
            })
            .collect(),
        deeds: deeds.iter().map(|deed| deed_fact(deed)).collect(),
    };
    let mut read = question_rows(active);
    for (chapter, next) in &chapters {
        read.extend(
            active
                .prose
                .row_of(chapter.began)
                .map(|row| Node::Row(Table::Chapters, row)),
        );
        read.extend(chapter_events(active, chapter.began, *next));
    }
    read.extend(super::reads::events_about(
        active,
        deeds.iter().filter_map(|deed| deed_name(deed)),
    ));
    read.extend(super::reads::level_read(active));
    read.extend(before.map(|(row, _)| Node::Row(Table::Summaries, row)));
    (facts, read)
}

fn question_rows(active: &Active) -> Vec<Node> {
    let question = |field: &str| FIELDS[..QUESTIONS].contains(&field);
    (0..)
        .zip(active.hero.changes())
        .filter(|(_, change)| matches!(change, Change::Set { field, .. } if question(field)))
        .map(|(row, _)| Node::Row(Table::Hero, row))
        .collect()
}

fn chapter_events(active: &Active, began: Tick, next: Tick) -> Vec<Node> {
    let history = active.character.world().history();
    history
        .iter()
        .filter(|event| event.tick >= began && event.tick < next)
        .map(|event| Node::Row(Table::Events, event.id.0))
        .collect()
}
