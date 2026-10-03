//! The model calls of the story program: each one opens with its row and its reads, waits
//! for a free slot, and ends with its answer or its failure (GAMEPLAY.md 5.6 and 5.14).

use super::{EventsBatch, MAX_OPEN_CALLS, Output, Story, StoryError, drafts, narrator, quests};
use crate::input::{CallId, MessageId};
use crate::learned::Rumor;
use crate::lore::{LoreCall, Next};
use crate::store::{CharacterKey, Node, Outcome};
use crate::{check, talk};
use hourglass::Tick;

/// An open model call, and what its answer is for.
pub(super) enum Pending {
    Lore {
        question: MessageId,
        lore: LoreCall,
    },
    /// A narrator line for the batch, checked against the hero of this character only.
    Narrator {
        batch: MessageId,
        key: CharacterKey,
        /// The moment in plain words: a line holds no number that it does not.
        moment: String,
    },
    /// A draft of the saga of the chapter that began at `began`, or the pick of the judge,
    /// for this character only. The round of the chapter knows which.
    Chronicle {
        key: CharacterKey,
        began: Tick,
    },
    /// A side quest from `giver`, asked at `at`, for this character only. Its offer is the
    /// notice of `batch`, or of the next answer when no batch waits for it.
    Quest {
        batch: Option<EventsBatch>,
        key: CharacterKey,
        giver: String,
        at: Tick,
    },
    /// A draft of a player task, for this character only.
    Draft {
        question: MessageId,
        key: CharacterKey,
    },
    /// A talk to `npc`, whose change of trust lands at `at`, for this character only.
    Talk {
        question: MessageId,
        key: CharacterKey,
        npc: String,
        at: Tick,
    },
}

impl Pending {
    /// The name of the kind in the `calls` table.
    fn kind(&self) -> &'static str {
        match self {
            Pending::Lore { .. } => "lore",
            Pending::Narrator { .. } => "narrator",
            Pending::Chronicle { .. } => "saga",
            Pending::Quest { .. } => "quest",
            Pending::Draft { .. } => "draft",
            Pending::Talk { .. } => "talk",
        }
    }

    /// A lore call has no key: it always opens for the active character.
    fn is_for(&self, active: &CharacterKey) -> bool {
        match self {
            Pending::Lore { .. } => true,
            Pending::Narrator { key, .. }
            | Pending::Chronicle { key, .. }
            | Pending::Quest { key, .. }
            | Pending::Draft { key, .. }
            | Pending::Talk { key, .. } => key == active,
        }
    }
}

impl Story {
    /// The call ends in its row, and the rows of the line rest on it.
    pub(super) fn answered(&mut self, call: CallId, text: &str) -> Result<Vec<Output>, StoryError> {
        let (pending, prompt) = self.take_call(call)?;
        self.note_names_in_no_fact(call, text, &prompt);
        if let Some(active) = self.active.as_mut() {
            active.answer_with(call);
        }
        let (outputs, outcome) = match pending {
            Pending::Lore { question, lore } => {
                let next = lore.answered(text);
                let outcome =
                    accepted_if(!matches!(&next, Next::Done(answer) if answer.text.is_none()));
                (self.follow(question, next).into_iter().collect(), outcome)
            }
            Pending::Narrator { batch, key, moment } => {
                let narrator = narrator::checked_line(text, &self.player_text(&key))
                    .filter(|line| narrator::numbers_from(line, &moment));
                let outcome = accepted_if(narrator.is_some());
                let seen = Output::EventsSeen {
                    id: batch,
                    narrator,
                    notice: None,
                };
                (vec![seen], outcome)
            }
            Pending::Chronicle { key, began } => self.saga_answered(&key, began, Some(text))?,
            Pending::Talk {
                question,
                key,
                npc,
                at,
            } => {
                let answer = self.talk_answered(question, &key, npc, at, text);
                let outcome =
                    accepted_if(matches!(&answer, Output::TalkAnswer { text: Some(_), .. }));
                (vec![answer], outcome)
            }
            Pending::Quest {
                batch,
                key,
                giver,
                at,
            } => self.quest_answered(batch, &key, &giver, at, text),
            Pending::Draft { question, key } => {
                let answer = self.draft_answered(question, &key, text);
                let outcome = accepted_if(matches!(
                    &answer,
                    Output::DraftAnswer { draft: Some(_), .. }
                ));
                (vec![answer], outcome)
            }
        };
        if let Some(active) = self.active.as_mut() {
            active.end_call_row(call, Some(text), outcome);
        }
        Ok(outputs)
    }

    /// The words always show. The change of trust lands only for the character that
    /// talked, and never before the last event, because the world can move on while the
    /// model thinks.
    pub(super) fn talk_answered(
        &mut self,
        question: MessageId,
        key: &CharacterKey,
        npc: String,
        asked_at: Tick,
        text: &str,
    ) -> Output {
        let Some(answer) = talk::checked_answer(text, &self.player_text(key)) else {
            return Output::TalkAnswer {
                id: question,
                npc,
                text: None,
                notice: None,
            };
        };
        let same = self
            .active
            .as_ref()
            .is_some_and(|active| &active.key == key);
        if same {
            let rumor = Rumor {
                at: asked_at,
                npc: npc.clone(),
                text: answer.say.clone(),
            };
            // A failed write loses one rumor. The words still show.
            if let Some(active) = self.active.as_mut() {
                let _ = active.learned.add_rumor(rumor);
            }
        }
        if same && answer.trust_change != 0 {
            // A refusal is not possible here, and a failed save keeps the events in
            // memory for the next save, so the words need not wait for either.
            let _ = self.change(|character| {
                let at = asked_at.max(character.world().tick);
                character.adjust_trust(at, &npc, answer.trust_change)
            });
        }
        Output::TalkAnswer {
            id: question,
            npc,
            text: Some(answer.say),
            notice: None,
        }
    }

    /// The bridge answers a call over its budget with a failure, so each failure slows the
    /// saga (`Pace`).
    pub(super) fn failed(&mut self, call: CallId) -> Result<Vec<Output>, StoryError> {
        let (pending, _) = self.take_call(call)?;
        self.pace.failed(self.newest);
        if let Some(active) = self.active.as_mut() {
            active.answer_with(call);
            active.end_call_row(call, None, Outcome::Failed);
        }
        Ok(match pending {
            Pending::Lore { question, lore } => vec![Output::LoreAnswer {
                id: question,
                answer: lore.failed(),
                notice: None,
            }],
            Pending::Narrator { batch, .. } => vec![Output::EventsSeen {
                id: batch,
                narrator: None,
                notice: None,
            }],
            Pending::Chronicle { key, began } => self.saga_answered(&key, began, None)?.0,
            Pending::Talk { question, npc, .. } => vec![Output::TalkAnswer {
                id: question,
                npc,
                text: None,
                notice: None,
            }],
            Pending::Quest { batch, giver, .. } => self.deliver(batch, quests::no_task(&giver)),
            Pending::Draft { question, .. } => vec![drafts::draft_answer(question, None)],
        })
    }

    /// The number of the next call picks the golden samples, so they change from call to
    /// call.
    pub(super) fn turn(&self) -> usize {
        usize::try_from(self.next_call.0).unwrap_or_default()
    }

    /// A call with no free slot waits for one, so the bridge never fails it for a saga
    /// (GAMEPLAY.md 3.3).
    /// The call gets its row in the database of its character, with what it read.
    pub(super) fn open_call(
        &mut self,
        pending: Pending,
        prompt: String,
        reads: Vec<Node>,
    ) -> Option<Output> {
        let call = self.next_call;
        self.next_call = CallId(call.0 + 1);
        if let Some(active) = self
            .active
            .as_mut()
            .filter(|active| pending.is_for(&active.key))
        {
            let pack = self.pack.label().to_string();
            active.open_call_row(call, pending.kind(), pack, prompt.clone(), reads);
            if let (Pending::Chronicle { .. }, Some(row)) = (&pending, active.call_row(call)) {
                self.round_calls.push(row);
            }
        }
        self.calls.insert(call, pending);
        self.prompts.insert(call, prompt);
        if !self.has_free_slot() {
            self.queued.push_back(call);
            return None;
        }
        self.send_call(call)
    }

    /// A call that waits comes first, so a free slot goes to it.
    pub(super) fn has_free_slot(&self) -> bool {
        self.open.len() < MAX_OPEN_CALLS && self.queued.is_empty()
    }

    pub(super) fn send_call(&mut self, call: CallId) -> Option<Output> {
        let prompt = self.prompts.get(&call)?.clone();
        self.open.insert(call);
        self.pace.opened(self.newest);
        Some(Output::ModelCall { call, prompt })
    }

    /// The calls that waited, while slots are free.
    pub(super) fn send_queued(&mut self) -> Vec<Output> {
        let mut outputs = Vec::new();
        while self.open.len() < MAX_OPEN_CALLS {
            let Some(call) = self.queued.pop_front() else {
                break;
            };
            outputs.extend(self.send_call(call));
        }
        outputs
    }

    /// The open call and its prompt. The bridge never saw a call that waits for a slot.
    pub(super) fn take_call(&mut self, call: CallId) -> Result<(Pending, String), StoryError> {
        if !self.open.remove(&call) {
            return Err(StoryError::UnknownCall(call));
        }
        let pending = self
            .calls
            .remove(&call)
            .ok_or(StoryError::UnknownCall(call))?;
        let prompt = self.prompts.remove(&call).unwrap_or_default();
        Ok((pending, prompt))
    }

    /// Log only: the check refuses nothing yet (GAMEPLAY.md 3.2.1).
    pub(super) fn note_names_in_no_fact(&mut self, call: CallId, answer: &str, prompt: &str) {
        for name in check::names_in_no_fact(answer, prompt) {
            let note = format!("call {}: the answer names {name}, and no fact does", call.0);
            self.notes.push(note);
        }
    }
}

fn accepted_if(accepted: bool) -> Outcome {
    if accepted {
        Outcome::Accepted
    } else {
        Outcome::Refused
    }
}
