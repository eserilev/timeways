//! The saga of each finished chapter in the story program (GAMEPLAY.md 3.3): the drafts,
//! the judge, and the final saga on the disk.

use super::{Active, CHAPTER_MOMENTS, Output, Pending, Story, StoryError, edits, reads};
use crate::best_of_two::{Next, Round};
use crate::check;
use crate::chronicle::{self, Draft, OwnWords, Pick, Saga};
use crate::flavor::{self, Teller, Told};
use crate::hero::{self, Entry};
use crate::journal::{Chapter, EntryState};
use crate::memory;
use crate::store::{CharacterKey, Node, Outcome, SagaSpan, Written};
use hourglass::{EventId, Tick};

impl Story {
    /// The next call of the saga that is written now, or the first draft of the next
    /// chapter.
    pub(super) fn saga_call(&mut self) -> Option<Output> {
        if self.saga_round.is_none() {
            return self.first_draft_call();
        }
        // A failed write loses the saga, and the chapter keeps its plain list.
        self.advance_round().ok().flatten()
    }

    /// The first draft of the oldest finished chapter with no saga yet. The bridge runs at
    /// most 2 model calls at once, so the saga waits until no other call is open: a
    /// question of the player never fails for a saga. The last chapter can still grow, so
    /// it waits for the next session.
    /// The small moments of a chapter run until the next chapter begins: an emote changes
    /// nothing in the world, so it can come after the last event of the chapter.
    fn first_draft_call(&mut self) -> Option<Output> {
        if !self.calls.is_empty() {
            return None;
        }
        let active = self.active.as_ref()?;
        let journal = active.journal();
        let chapters = &journal.chapters;
        let index = self.chapter_waiting_for_saga(active)?;
        let chapter = &chapters[index];
        let next = next_began(chapters, index);
        let earlier = &chapters[index.saturating_sub(memory::MEMORY_CHAPTERS)..index];
        let top = flavor::top_moments(
            active.flavor.moments(),
            active.flavor.told(),
            &active.character,
            (chapter.began, Tick(next.0.saturating_sub(1))),
            CHAPTER_MOMENTS,
        );
        let words: Vec<String> = top
            .iter()
            .map(|moment| flavor::describe(&moment.flavor, moment.count))
            .collect();
        let kinds = top.iter().map(|moment| moment.flavor.kind.key()).collect();
        let first = EventId(chapter.first);
        let pending = Pending::Chronicle {
            key: active.key.clone(),
            first,
        };
        let hero = hero::hero(active.hero.changes());
        let in_chapter = |entry: &Entry| entry.at >= chapter.began && entry.at < next;
        let written = hero::newest_texts(&hero.entries, in_chapter);
        // The entries of the chapter and of the portrait share one limit, so a full story
        // fits the budget of a chapter.
        let mut others = hero::newest_texts(&hero.entries, |entry| !in_chapter(entry));
        others.truncate(hero::PROMPT_ENTRIES - written.len());
        // The sheet is news only once, or when the player changed it, so chapters do not
        // all open with the same portrait.
        let sheet_is_news =
            index == 0 || hero::sheet_changed(active.hero.changes(), chapter.began, next);
        let portrait = sheet_is_news
            .then(|| hero::portrait(&hero, &others))
            .flatten();
        let telling = edits::telling_of(active, edits::chapter_key(chapter.first));
        let own = OwnWords {
            portrait: portrait.as_deref(),
            told: &written,
            telling: telling.as_ref().map(|(text, _)| text.as_str()),
        };
        let draft =
            |draft| chronicle::draft_prompt(&journal.places, chapter, earlier, &words, &own, draft);
        let (first_prompt, second) = (draft(Draft::First), draft(Draft::Second));
        let facts = chronicle::facts(&journal.places, chapter);
        let round = Round::new(
            active.key.clone(),
            first,
            kinds,
            chapter.number,
            facts,
            second,
        );
        let range = reads::Range {
            first,
            last: span_last(active, first)?,
            began: chapter.began,
            next,
        };
        let mut read = reads::chapter_read(active, &range);
        read.extend(telling.map(|(_, row)| row));
        self.chronicle_asked.insert(first);
        self.saga_round = Some(round);
        self.round_read.clone_from(&read);
        self.round_calls.clear();
        self.open_call(pending, first_prompt, read)
    }

    /// The oldest closed chapter with no saga that was not asked for one in this run.
    pub(super) fn chapter_waiting_for_saga(&self, active: &Active) -> Option<usize> {
        let chapters = active.journal().chapters;
        chapters.iter().position(|chapter| {
            let first = EventId(chapter.first);
            chapter.state == EntryState::Closed
                && active.prose.get(first).is_none()
                && !self.chronicle_asked.contains(&first)
        })
    }

    /// `text` is None for a failed call. A failed judge picks the first draft. A draft
    /// that passes its checks, and every pick, is accepted.
    pub(super) fn saga_answered(
        &mut self,
        key: &CharacterKey,
        first: EventId,
        text: Option<&str>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let Some(round) = self
            .saga_round
            .as_ref()
            .filter(|round| &round.key == key && round.first == first)
        else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        if round.is_judged() {
            let pick = text.map_or(Pick::First, chronicle::checked_pick);
            let saga = round.picked(pick);
            self.finish_round(saga)?;
            return Ok((Vec::new(), Outcome::Accepted));
        }
        let draft = text.and_then(|text| self.checked_draft(round, text));
        let outcome = if draft.is_some() {
            Outcome::Accepted
        } else {
            Outcome::Refused
        };
        if let Some(round) = self.saga_round.as_mut() {
            round.add_draft(draft);
        }
        Ok((self.advance_round()?.into_iter().collect(), outcome))
    }

    /// A draft that repeats an earlier saga of the character is refused (3.3).
    fn checked_draft(&self, round: &Round, text: &str) -> Option<Saga> {
        let active = self.active.as_ref()?;
        let player_text = hero::player_text(&hero::hero(active.hero.changes()));
        let saga = chronicle::checked_saga(text, round.kinds.len(), round.facts(), &player_text)?;
        let telling = edits::telling_of(active, edits::chapter_key(round.first.0));
        let earlier: Vec<&str> = active
            .prose
            .before(round.first)
            .map(|written| written.text.as_str())
            .chain(telling.as_ref().map(|(text, _)| text.as_str()))
            .collect();
        (!check::copies_a_sample(&saga.text, &earlier)).then_some(saga)
    }

    /// The next call of the round, when no other call is open. The bridge runs at most 2
    /// calls at once, so a question of the player never fails for a saga.
    fn advance_round(&mut self) -> Result<Option<Output>, StoryError> {
        let Some(round) = &self.saga_round else {
            return Ok(None);
        };
        match round.next(self.pace.is_tight(self.newest)) {
            Next::Final(saga) => {
                self.finish_round(saga)?;
                Ok(None)
            }
            Next::Call(_) if !self.calls.is_empty() => Ok(None),
            Next::Call(prompt) => {
                let pending = Pending::Chronicle {
                    key: round.key.clone(),
                    first: round.first,
                };
                let earlier = self.round_calls.iter().map(|call| Node::Call(*call));
                let read = self.round_read.iter().copied().chain(earlier).collect();
                Ok(self.open_call(pending, prompt, read))
            }
        }
    }

    /// With no saga, the chapter keeps its plain list. A footnote tells its kind of moment,
    /// so the same joke waits (5.4.1).
    /// The summary of the chapter is due, with a saga or with none (docs/plans/hero-stories.md
    /// 3.5).
    fn finish_round(&mut self, saga: Option<Saga>) -> Result<(), StoryError> {
        let Some(round) = self.saga_round.take() else {
            return Ok(());
        };
        self.summary_is_due(round.key.clone(), round.first);
        let Some(saga) = saga else {
            return Ok(());
        };
        let Some(active) = self
            .active
            .as_mut()
            .filter(|active| active.key == round.key)
        else {
            return Ok(());
        };
        let now = self.newest;
        for (moment, _) in &saga.footnotes {
            let told = Told {
                key: round.kinds[moment - 1].clone(),
                at: now,
                teller: Teller::Chronicle,
            };
            active.flavor.add_told(told)?;
        }
        let footnotes = saga
            .footnotes
            .into_iter()
            .map(|(_, footnote)| footnote)
            .collect();
        let Some(span) = saga_span(active, round.first) else {
            return Ok(());
        };
        active.prose.add(
            span,
            Written {
                text: saga.text,
                footnotes,
            },
        )?;
        Ok(())
    }
}

/// The tick where the chapter after this one began. The chapter after a close at the most
/// weight can still have no step, so the tick after the last event stands in for it.
fn next_began(chapters: &[Chapter], index: usize) -> Tick {
    chapters.get(index + 1).map_or_else(
        || Tick(chapters[index].ended.0.saturating_add(1)),
        |next| next.began,
    )
}

/// The last event of the chapter whose first event is `first`.
fn span_last(active: &Active, first: EventId) -> Option<EventId> {
    saga_span(active, first).map(|span| span.last)
}

/// The rule and the range of the chapter whose first event is `first`, as its saga row
/// keeps them.
fn saga_span(active: &Active, first: EventId) -> Option<SagaSpan> {
    let span = active
        .book
        .chapters()
        .into_iter()
        .find(|span| span.first == first)?;
    Some(SagaSpan {
        rule: span.rule,
        first: span.first,
        last: span.last,
    })
}
