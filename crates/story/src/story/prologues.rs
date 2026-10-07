//! The prologue in the story program (GAMEPLAY.md 3.3): the past that the addon sends at a
//! login, when the prologue is due, its one call, and its answer. The rule lives in
//! `timeways_rules::prologue`, where Lean proves it.

use super::aliases::{TextLimits, shows_with_names};
use super::{Active, Output, Pending, Story, StoryError, reads};
use crate::chapters::{CURRENT_RULE, SpanState};
use crate::journal::{Chapter, EntryState, OpenedBy};
use crate::past::{Past, PastRow, WorldAge};
use crate::prologue;
use crate::store::{CharacterKey, Node, Outcome, SagaSpan, Table, Written};
use hourglass::EventId;
use timeways_rules::prologue::{self as rule, Prologue};

/// The prologue is chapter 0: its saga row is keyed by the founding of the character, which
/// no chapter holds. So it never moves, and no chapter moves for it.
pub const PROLOGUE_KEY: EventId = EventId(0);

/// A refused prologue gets a new try at a later batch, at most this many in all. The calls
/// table counts them, so a restart gives no extra try.
pub const MOST_PROLOGUE_CALLS: u64 = 3;

/// The kind of the call in the `calls` table.
pub const PROLOGUE_CALL: &str = "prologue";

/// The line of Timeways when the prologue is written.
pub const PROLOGUE_READY: &str = "Your Chronicle has a prologue now.";

const PROLOGUE_LIMITS: TextLimits = TextLimits {
    chars: prologue::MAX_PROLOGUE_CHARS,
    bytes: prologue::MAX_PROLOGUE_BYTES,
};

impl Story {
    /// Only the first past decides the prologue, so only the first one is kept.
    pub(super) fn read_past(&mut self, past: Past) -> Result<Vec<Output>, StoryError> {
        past.check()?;
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        if active.past.rows().is_empty() {
            let world = world_age(active);
            active.past.add(PastRow { past, world })?;
        }
        Ok(Vec::new())
    }

    /// Wanted until the world keeps a past. Only the first one decides, so the addon asks the
    /// game only once in a life.
    pub(super) fn past_wanted(&self) -> Option<super::PastWanted> {
        let active = self.active.as_ref()?;
        active
            .past
            .rows()
            .is_empty()
            .then_some(super::PastWanted::Wanted)
    }

    /// The call of the prologue, while it is due. It opens only while no other call is open
    /// and the pace window is not tight, and at most `MOST_PROLOGUE_CALLS` times in a life.
    pub(super) fn prologue_call(&mut self) -> Option<Output> {
        let busy = !self.calls.is_empty() || self.saga_round.is_some();
        if busy || self.pace.is_tight(self.newest) {
            return None;
        }
        let active = self.active.as_ref()?;
        let tries = active.count_calls(&[PROLOGUE_CALL]).ok()?;
        if tries >= MOST_PROLOGUE_CALLS || !rule::is_due(prologue_state(active)) {
            return None;
        }
        let row = active.past.rows().first()?;
        let facts = match prologue::facts_of(&self.pack, &active.character, &row.past) {
            Ok(facts) => facts,
            Err(error) => {
                self.notes
                    .push(format!("the lore of the prologue: {error}"));
                return None;
            }
        };
        let mut read = vec![Node::Row(Table::Past, 0)];
        read.extend(reads::level_read(active));
        let pending = Pending::Prologue {
            key: active.key.clone(),
            told: prologue::told(&facts),
        };
        self.open_call(pending, prologue::prompt(&facts), read)
    }

    /// `text` is None for a failed call. A refused prologue or a failed call writes nothing,
    /// so a later batch tries again.
    pub(super) fn prologue_answered(
        &mut self,
        key: &CharacterKey,
        told: &str,
        text: Option<&str>,
    ) -> Result<(Vec<Output>, Outcome), StoryError> {
        let player_text = self.player_text(key);
        let checked = text.and_then(|text| prologue::checked_prologue(text, told, &player_text));
        let active = self.active.as_mut().filter(|active| &active.key == key);
        let (Some(active), Some(text)) = (active, checked) else {
            return Ok((Vec::new(), Outcome::Refused));
        };
        if !rule::is_due(prologue_state(active))
            || !shows_with_names(active, &text, PROLOGUE_LIMITS)
        {
            return Ok((Vec::new(), Outcome::Refused));
        }
        let span = SagaSpan {
            rule: CURRENT_RULE,
            first: PROLOGUE_KEY,
            last: PROLOGUE_KEY,
        };
        let written = Written {
            text,
            footnotes: Vec::new(),
        };
        active.prose.add(span, written)?;
        self.notice = Some(PROLOGUE_READY.to_string());
        Ok((Vec::new(), Outcome::Accepted))
    }
}

/// Where the prologue stands: the kept past folds through the rule, and then the prologue
/// row, when it is written.
#[must_use]
pub(super) fn prologue_state(active: &Active) -> Prologue {
    let mut state = Prologue::Unseen;
    for row in active.past.rows() {
        state = rule::after_past(state, rule_past(row));
    }
    if has_prologue(active) {
        state = rule::after_written(state);
    }
    state
}

fn rule_past(row: &PastRow) -> rule::Past {
    let zones = u32::try_from(row.past.places().len()).unwrap_or(u32::MAX);
    rule::Past {
        level: row.past.level,
        quests: row.past.quests,
        zones,
        world: match row.world {
            WorldAge::New => rule::WorldAge::New,
            WorldAge::Played => rule::WorldAge::Played,
        },
    }
}

/// A world with a closed chapter or a tale has play of Timeways already.
fn world_age(active: &Active) -> WorldAge {
    let closed = active
        .book
        .chapters()
        .iter()
        .any(|chapter| chapter.state == SpanState::Closed);
    if closed || !active.book.tales().is_empty() {
        WorldAge::Played
    } else {
        WorldAge::New
    }
}

/// True once the prologue is written.
#[must_use]
pub(super) fn has_prologue(active: &Active) -> bool {
    active.prose.get(PROLOGUE_KEY).is_some()
}

/// The prologue as chapter 0 of the journal, once it is written. Its text comes from the
/// saga row of `PROLOGUE_KEY`, as a chapter's does.
#[must_use]
pub(super) fn journal_prologue(active: &Active) -> Option<Chapter> {
    if !has_prologue(active) {
        return None;
    }
    let row = active.past.rows().first()?;
    let level = i64::from(row.past.level);
    Some(Chapter {
        number: 0,
        first: PROLOGUE_KEY.0,
        began: row.past.at,
        ended: row.past.at,
        title: Some(prologue::TITLE.to_string()),
        opened_by: OpenedBy::Prologue,
        state: EntryState::Closed,
        levels: Some([level, level]),
        zones: row
            .past
            .places()
            .into_iter()
            .take(crate::journal::CHAPTER_LIST)
            .map(str::to_string)
            .collect(),
        ..Chapter::default()
    })
}

/// The text of the prologue for the prompt of a summary, with its row.
#[must_use]
pub(super) fn prologue_text(active: &Active) -> Option<(String, Node)> {
    let written = active.prose.get(PROLOGUE_KEY)?;
    let row = active.prose.row_of(PROLOGUE_KEY)?;
    Some((written.text.clone(), Node::Row(Table::Chapters, row)))
}
