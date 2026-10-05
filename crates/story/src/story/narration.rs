//! The narrator line of a batch in the story program (GAMEPLAY.md 3.2): its lore, its
//! prompt, its check, and its one retry.

use super::{Output, Story, quests, reads};
use crate::input::MessageId;
use crate::line_check::{Checked, Grounds, LineFault, checked_line};
use crate::moments::Moment;
use crate::narrator::{self, Telling, Who};
use crate::narrator_lore::lore_of_moment;
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
}

impl Story {
    /// The prompt and the call of the line about `moment`. The batch rows and the lore
    /// passage are what it read. A failed search gives no lore, and the line still goes.
    pub(super) fn narration(
        &mut self,
        batch: MessageId,
        moment: &Moment,
    ) -> Option<(String, NarratorCall)> {
        let active = self.active.as_ref()?;
        let passage = lore_of_moment(&self.pack, &active.seen_index, &active.character, moment)
            .ok()
            .flatten();
        let lore = passage
            .as_ref()
            .map(|passage| narrator::lore_excerpt(&passage.text));
        let who = Who::of(&active.character);
        let telling = Telling {
            moment,
            lore: lore.as_deref(),
            who: &who,
        };
        let turn = self.turn();
        let prompt = narrator::prompt(&telling, turn);
        let grounds = Grounds::of(&telling, turn);
        let mut reads = std::mem::take(&mut self.batch_rows);
        reads.extend(reads::passages_read(active, passage.as_slice()));
        let call = NarratorCall {
            batch,
            key: active.key.clone(),
            grounds,
            attempt: Attempt::First,
            reads,
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
        match checked_line(text, &call.grounds, &player_text) {
            Checked::Line(line) => (vec![said(call.batch, line)], Outcome::Accepted),
            Checked::Refused(faults) if call.attempt == Attempt::First && self.has_free_slot() => {
                let retry = self.retry_narrator(row, call, prompt, text, &faults);
                (retry, Outcome::Refused)
            }
            Checked::Silent | Checked::Refused(_) => {
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

fn said(batch: MessageId, line: String) -> Output {
    Output::EventsSeen {
        id: batch,
        narrator: Some(line),
        notice: None,
    }
}
