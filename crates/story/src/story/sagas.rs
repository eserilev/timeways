//! The saga of each finished chapter in the story program (GAMEPLAY.md 3.3): the drafts,
//! the judge, and the final saga on the disk.

use super::{CHAPTER_MOMENTS, Output, Pending, Story, StoryError};
use crate::best_of_two::{Next, Round};
use crate::check;
use crate::chronicle::{self, Draft, Pick, Saga};
use crate::flavor::{self, Teller, Told};
use crate::hero::{self, Entry};
use crate::journal::journal;
use crate::memory;
use crate::store::{CharacterKey, Written};
use hourglass::Tick;

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
        let journal = journal(&active.character);
        let chapters = journal.chapters;
        let index = (1..chapters.len()).map(|next| next - 1).find(|&index| {
            let began = chapters[index].began;
            active.prose.get(began).is_none() && !self.chronicle_asked.contains(&began)
        })?;
        let (chapter, next) = (&chapters[index], &chapters[index + 1]);
        let earlier = &chapters[index.saturating_sub(memory::MEMORY_CHAPTERS)..index];
        let top = flavor::top_moments(
            active.flavor.moments(),
            active.flavor.told(),
            &active.character,
            (chapter.began, Tick(next.began.0 - 1)),
            CHAPTER_MOMENTS,
        );
        let words: Vec<String> = top
            .iter()
            .map(|moment| flavor::describe(&moment.flavor, moment.count))
            .collect();
        let kinds = top.iter().map(|moment| moment.flavor.kind.key()).collect();
        let pending = Pending::Chronicle {
            key: active.key.clone(),
            began: chapter.began,
        };
        let hero = hero::hero(active.hero.changes());
        let in_chapter = |entry: &Entry| entry.at >= chapter.began && entry.at < next.began;
        let written = hero::newest_texts(&hero.entries, in_chapter);
        // The sheet is news only once, or when the player changed it, so chapters do not
        // all open with the same portrait.
        let sheet_is_news =
            index == 0 || hero::sheet_changed(active.hero.changes(), chapter.began, next.began);
        let portrait = sheet_is_news.then(|| hero::portrait(&hero)).flatten();
        let draft = |draft| {
            chronicle::draft_prompt(
                &journal.places,
                chapter,
                earlier,
                &words,
                portrait.as_deref(),
                &written,
                draft,
            )
        };
        let (first, second) = (draft(Draft::First), draft(Draft::Second));
        let facts = chronicle::facts(&journal.places, chapter);
        let round = Round::new(
            active.key.clone(),
            chapter.began,
            kinds,
            chapter.number,
            facts,
            second,
        );
        self.chronicle_asked.insert(chapter.began);
        self.saga_round = Some(round);
        self.open_call(pending, first)
    }

    /// `text` is None for a failed call. A failed judge picks the first draft.
    pub(super) fn saga_answered(
        &mut self,
        key: &CharacterKey,
        began: Tick,
        text: Option<&str>,
    ) -> Result<Vec<Output>, StoryError> {
        let Some(round) = self
            .saga_round
            .as_ref()
            .filter(|round| &round.key == key && round.began == began)
        else {
            return Ok(Vec::new());
        };
        if round.is_judged() {
            let pick = text.map_or(Pick::First, chronicle::checked_pick);
            let saga = round.picked(pick);
            self.finish_round(saga)?;
            return Ok(Vec::new());
        }
        let draft = text.and_then(|text| self.checked_draft(round, text));
        if let Some(round) = self.saga_round.as_mut() {
            round.add_draft(draft);
        }
        Ok(self.advance_round()?.into_iter().collect())
    }

    /// A draft that repeats an earlier saga of the character is refused (3.3).
    fn checked_draft(&self, round: &Round, text: &str) -> Option<Saga> {
        let active = self.active.as_ref()?;
        let player_text = hero::player_text(&hero::hero(active.hero.changes()));
        let saga = chronicle::checked_saga(text, round.kinds.len(), &player_text)?;
        let earlier: Vec<&str> = active
            .prose
            .before(round.began)
            .map(|written| written.text.as_str())
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
                    began: round.began,
                };
                Ok(self.open_call(pending, prompt))
            }
        }
    }

    /// With no saga, the chapter keeps its plain list. A footnote tells its kind of moment,
    /// so the same joke waits (5.4.1).
    fn finish_round(&mut self, saga: Option<Saga>) -> Result<(), StoryError> {
        let Some(round) = self.saga_round.take() else {
            return Ok(());
        };
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
        active.prose.add(
            round.began,
            Written {
                text: saga.text,
                footnotes,
            },
        )?;
        Ok(())
    }
}
