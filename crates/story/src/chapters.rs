//! The chapters and tales of the Chronicle in the story program (docs/plans/chapters.md):
//! the walk of the history and the fold of `timeways_rules::chapters`, kept in memory and
//! folded one event at a time. The code decides every entry, never a model.

use crate::character::Character;
use crate::walk::{NO_ZONE, RuleRow, Walk};
use hourglass::{EntityId, EventHistory, EventId};
use std::ops::RangeInclusive;
use timeways_rules::chapters::{self as fold, Fold, Opening, Step, Track};
use timeways_rules::weights::RULE_ONE;

/// The rule of the chapters in this program. A change of the fold or of its constants gets
/// a new rule, and the old rule stays as it was (docs/plans/chapters.md 13).
pub const CURRENT_RULE: u8 = RULE_ONE;

/// The row of `chapter_rules` that a world needs before it folds, or None. A world with no
/// row gets the first rule from its first event. A world with an older rule gets the rule
/// of this program from its next event, so no closed chapter and no saga moves.
#[must_use]
pub fn new_epoch(rows: &[RuleRow], next_event: EventId, current: u8) -> Option<RuleRow> {
    let Some(newest) = rows.iter().max_by_key(|row| row.from) else {
        return Some(RuleRow {
            rule: current,
            from: EventId(0),
        });
    };
    (newest.rule != current).then_some(RuleRow {
        rule: current,
        from: next_event,
    })
}

/// The first event of every world: the founding of the character.
const FOUNDING: EventId = EventId(0);

/// The walk and the fold of one character.
#[derive(Clone, Debug)]
pub struct Book {
    walk: Walk,
    fold: Fold,
    /// The event of each step. A rule step stands for the event after it.
    events: Vec<EventId>,
    /// The dense zone of each step.
    zones: Vec<usize>,
    /// The events of the history that the walk read.
    walked: usize,
}

/// One chapter, as a range of steps and of events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChapterSpan {
    /// The key of the chapter: the event of its first step.
    pub first: EventId,
    pub last: EventId,
    pub steps: RangeInclusive<usize>,
    pub opening: Opening,
    /// The zone of its title.
    pub zone: Option<EntityId>,
    pub rule: u8,
    pub weight: u16,
    pub state: SpanState,
}

/// A closed entry never changes. An open one still grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanState {
    Open,
    Closed,
}

/// One run of an instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisitSpan {
    pub first: EventId,
    pub last: EventId,
    pub steps: RangeInclusive<usize>,
    pub gain: u32,
    pub state: SpanState,
}

/// The one tale of an instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaleSpan {
    pub instance: EntityId,
    /// The key of the tale: the event of its first step.
    pub first: EventId,
    pub first_step: usize,
    pub weight: u32,
    pub runs: u32,
    /// Its visits, the closed ones first, in order.
    pub visits: Vec<VisitSpan>,
}

impl Book {
    #[must_use]
    pub fn new(you: EntityId, rules: Vec<RuleRow>) -> Book {
        Book {
            walk: Walk::new(you, rules),
            fold: fold::start(),
            events: Vec::new(),
            zones: Vec::new(),
            walked: 0,
        }
    }

    /// The book of a whole history under the first rule.
    #[must_use]
    pub fn of(character: &Character) -> Book {
        let mut book = Book::new(character.you(), Vec::new());
        book.catch_up(character.world().history());
        book
    }

    /// Walks and folds the events of the history that came since the last call.
    pub fn catch_up(&mut self, history: &EventHistory) {
        let mut steps: Vec<Step> = Vec::new();
        for event in history.iter().skip(self.walked) {
            self.walked += 1;
            // The founding of the character is no play, so it is in no chapter.
            if event.id == FOUNDING {
                continue;
            }
            for (step, id) in self.walk.step(event) {
                let zone = match step {
                    Step::Play(play) => play.zone,
                    Step::Rule(_) => NO_ZONE,
                };
                steps.push(step);
                self.events.push(id);
                self.zones.push(zone);
            }
        }
        fold::advance(&mut self.fold, &steps);
    }

    #[must_use]
    pub fn fold(&self) -> &Fold {
        &self.fold
    }

    /// The event of a step.
    #[must_use]
    pub fn event_of(&self, step: usize) -> Option<EventId> {
        self.events.get(step).copied()
    }

    /// The step of an event: the last step that stands for it.
    #[must_use]
    pub fn step_of(&self, event: EventId) -> Option<usize> {
        let after = self.events.partition_point(|id| *id <= event);
        let step = after.checked_sub(1)?;
        (self.events[step] == event).then_some(step)
    }

    /// The gain of the step of an event, and where it counted.
    #[must_use]
    pub fn gain_of(&self, event: EventId) -> Option<fold::Gain> {
        self.fold.gains.get(self.step_of(event)?).copied()
    }

    /// Every chapter, oldest first. The open chapter comes last, when it has a step.
    #[must_use]
    pub fn chapters(&self) -> Vec<ChapterSpan> {
        let mut spans: Vec<ChapterSpan> = self
            .fold
            .closed
            .iter()
            .filter_map(|closed| self.chapter_span(&closed.chapter, closed.last, SpanState::Closed))
            .collect();
        if let Some(last) = self.fold.gains.len().checked_sub(1) {
            spans.extend(self.chapter_span(&self.fold.open, last, SpanState::Open));
        }
        spans
    }

    fn chapter_span(
        &self,
        chapter: &fold::Chapter,
        last: usize,
        state: SpanState,
    ) -> Option<ChapterSpan> {
        if chapter.first > last {
            return None;
        }
        Some(ChapterSpan {
            first: self.event_of(chapter.first)?,
            last: self.event_of(last)?,
            steps: chapter.first..=last,
            opening: chapter.opening,
            zone: chapter.zone.and_then(|zone| self.walk.zone_of_id(zone)),
            rule: chapter.rule,
            weight: chapter.weight,
            state,
        })
    }

    /// Every tale, in the order of its first step.
    #[must_use]
    pub fn tales(&self) -> Vec<TaleSpan> {
        let mut tales: Vec<TaleSpan> = Vec::new();
        for tale in &self.fold.tales {
            let Some(instance) = self.walk.zone_of_id(tale.instance) else {
                continue;
            };
            let Some(first) = self.event_of(tale.first) else {
                continue;
            };
            tales.push(TaleSpan {
                instance,
                first,
                first_step: tale.first,
                weight: tale.weight,
                runs: tale.runs,
                visits: Vec::new(),
            });
        }
        let open = self.fold.visit.map(|visit| (visit, SpanState::Open));
        let closed = self
            .fold
            .visits
            .iter()
            .map(|visit| (*visit, SpanState::Closed));
        for (visit, state) in closed.chain(open) {
            let span = self.visit_span(&visit, state);
            let tale = self.fold.tales.get(visit.tale).map(|tale| tale.first);
            let found = tales
                .iter_mut()
                .find(|found| Some(found.first_step) == tale);
            if let (Some(found), Some(span)) = (found, span) {
                found.visits.push(span);
            }
        }
        tales
    }

    fn visit_span(&self, visit: &fold::Visit, state: SpanState) -> Option<VisitSpan> {
        Some(VisitSpan {
            first: self.event_of(visit.first)?,
            last: self.event_of(visit.last)?,
            steps: visit.first..=visit.last,
            gain: visit.gain,
            state,
        })
    }

    /// The zones where steps of the open world in this range gained weight, in the order
    /// of their first gain: the places where a chapter took place. A flight over a zone
    /// gains nothing, so it names no zone.
    #[must_use]
    pub fn zones_with_gain(&self, steps: RangeInclusive<usize>) -> Vec<EntityId> {
        let mut zones: Vec<EntityId> = Vec::new();
        for step in steps {
            let gained = self
                .fold
                .gains
                .get(step)
                .is_some_and(|gain| gain.amount > 0 && gain.track == Track::World);
            let zone = self
                .zones
                .get(step)
                .and_then(|zone| self.walk.zone_of_id(*zone));
            if let (true, Some(zone)) = (gained, zone)
                && !zones.contains(&zone)
            {
                zones.push(zone);
            }
        }
        zones
    }

    /// True when the step counted in the open world.
    #[must_use]
    pub fn is_world_step(&self, step: usize) -> bool {
        self.fold
            .gains
            .get(step)
            .is_some_and(|gain| gain.track == Track::World)
    }
}
