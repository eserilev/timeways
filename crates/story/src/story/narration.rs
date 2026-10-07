//! The narrator line of a batch in the story program (GAMEPLAY.md 3.2): its lore, its
//! prompt, its check, and its one retry. A moment with templates asks the model for the
//! history and its choices, and the code builds the line (docs/plans/narrator-templates.md).

use super::{Output, Story, quests, reads};
use crate::input::MessageId;
use crate::line_check::{Checked, Grounds, LineFault, checked_line};
use crate::moments::Moment;
use crate::narrator::{self, Telling, Who};
use crate::narrator_build::{self, Answered, Built, Offer, Setup, answered, kind_of};
use crate::narrator_lore::{is_silent, lore_of_moment, lore_subjects};
use crate::prompt::{self, Attempt};
use crate::store::{CharacterKey, Node, Outcome};

/// A narrator call: whose, for which batch, and what the line was told from. A retry keeps
/// all of it.
pub(in crate::story) struct NarratorCall {
    pub(super) batch: MessageId,
    /// The line is checked against the hero of this character only.
    pub(super) key: CharacterKey,
    pub(super) grounds: Grounds,
    pub(super) attempt: Attempt,
    pub(super) reads: Vec<Node>,
    /// What the templates build the line from. None for a flavor moment, whose line is
    /// free text.
    pub(super) templated: Option<Box<(Setup, Offer)>>,
}

impl Story {
    /// The prompt and the call of the line about `moment`. The batch rows and the lore
    /// passage are what it read. A failed search gives no lore. A deed with thin lore gets
    /// no call: silence is better than a bare deed (`narrator_lore::is_silent`).
    pub(super) fn narration(
        &mut self,
        batch: MessageId,
        moment: &Moment,
    ) -> Option<(String, NarratorCall)> {
        let active = self.active.as_ref()?;
        let who = Who::of(&active.character);
        let told = if moment.enters_an_instance() {
            active.told_lore().ok()?
        } else {
            Vec::new()
        };
        let passage = lore_of_moment(
            &self.pack,
            &active.seen_index,
            &active.character,
            moment,
            &who,
            &told,
        )
        .ok()
        .flatten();
        if is_silent(moment, &lore_subjects(moment, &who), passage.as_ref()) {
            return None;
        }
        let lore = passage
            .as_ref()
            .map(|passage| narrator::lore_excerpt(&passage.text));
        let telling = Telling {
            moment,
            lore: lore.as_deref(),
            who: &who,
        };
        let turn = self.turn();
        let templated = match kind_of(moment) {
            Some(_) => {
                let setup = Setup {
                    moment: moment.clone(),
                    who: who.clone(),
                    turn,
                    recent: active.recent_shapes().ok()?,
                };
                // No shape fits the moment whatever the model says, so no call.
                let offer = narrator_build::offer(&setup)?;
                Some(Box::new((setup, offer)))
            }
            None => None,
        };
        let prompt = match templated.as_deref() {
            Some((_, offer)) => narrator::lore_prompt(&telling, turn, offer),
            None => narrator::line_prompt(&telling, turn),
        };
        let grounds = Grounds::of(&telling, turn);
        let mut reads = std::mem::take(&mut self.batch_rows);
        reads.extend(reads::passages_read(active, passage.as_slice()));
        let call = NarratorCall {
            batch,
            key: active.key.clone(),
            grounds,
            attempt: Attempt::First,
            reads,
            templated,
        };
        Some((prompt, call))
    }

    /// A first answer that fails the check gets one more call with the reasons, when a
    /// slot is free: the narrator never waits. A second failure is silence.
    pub(super) fn narrator_answered(
        &mut self,
        row: Option<u64>,
        call: NarratorCall,
        prompt: &str,
        text: &str,
    ) -> (Vec<Output>, Outcome) {
        let player_text = self.player_text(&call.key);
        let verdict = match call.templated.as_deref() {
            Some((setup, offer)) => answered(text, setup, offer, &call.grounds, &player_text),
            None => free_line(text, &call.grounds, &player_text),
        };
        match verdict {
            Answered::Line(built) => {
                self.told_shape = (!built.shape.is_empty()).then_some(built.shape);
                (vec![said(call.batch, built.line)], Outcome::Accepted)
            }
            Answered::Refused(faults) if call.attempt == Attempt::First && self.has_free_slot() => {
                let retry = self.retry_narrator(row, call, prompt, text, &faults);
                (retry, Outcome::Refused)
            }
            Answered::Silent | Answered::Refused(_) => {
                (vec![quests::quiet(call.batch)], Outcome::Refused)
            }
        }
    }

    /// The second call: the first prompt, the first answer, and the reasons. It reads what
    /// the first call read, and the first call.
    fn retry_narrator(
        &mut self,
        first_row: Option<u64>,
        call: NarratorCall,
        prompt: &str,
        text: &str,
        faults: &[LineFault],
    ) -> Vec<Output> {
        let batch = call.batch;
        let mut reads = call.reads;
        reads.extend(first_row.map(Node::Call));
        let retry = NarratorCall {
            attempt: Attempt::Retry,
            reads: reads.clone(),
            ..call
        };
        let reasons: Vec<String> = faults.iter().map(ToString::to_string).collect();
        let prompt = prompt::retry(prompt, text, &reasons);
        let pending = super::Pending::Narrator(retry);
        vec![
            self.open_call(pending, prompt, reads)
                .unwrap_or_else(|| quests::quiet(batch)),
        ]
    }
}

/// The verdict on a line of free text, for a flavor moment. It has no shape.
fn free_line(text: &str, grounds: &Grounds, player_text: &str) -> Answered {
    match checked_line(text, grounds, player_text) {
        Checked::Line(line) => Answered::Line(Built {
            line,
            shape: String::new(),
            parts: Vec::new(),
            named: false,
        }),
        Checked::Silent => Answered::Silent,
        Checked::Refused(faults) => Answered::Refused(faults),
    }
}

fn said(batch: MessageId, line: String) -> Output {
    Output::EventsSeen {
        id: batch,
        narrator: Some(line),
        notice: None,
    }
}
