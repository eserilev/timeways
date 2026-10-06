//! The model calls of the story program: each one opens with its row and its reads, waits
//! for a free slot, and ends with its answer or its failure (GAMEPLAY.md 5.6 and 5.14).

use super::narration::NarratorCall;
use super::quests::{self, QuestCall, TalkWork};
use super::{Active, MAX_OPEN_CALLS, Output, Story, StoryError, drafts, reads};
use crate::hero::Hero;
use crate::hero_hook::{self, Hook};
use crate::input::{CallId, MessageId};
use crate::learned::Rumor;
use crate::lore::{LoreCall, Next};
use crate::prompt::Attempt;
use crate::store::{CharacterKey, Node, Outcome, StoreError};
use crate::talk::Work;
use crate::{check, talk};
use hourglass::EventId;
use hourglass::Tick;
use std::collections::VecDeque;

/// An open model call, and what its answer is for.
pub(super) enum Pending {
    Lore {
        question: MessageId,
        lore: LoreCall,
    },
    /// A narrator line for the batch (`NarratorCall`).
    Narrator(NarratorCall),
    /// A draft of the saga of the chapter whose first event is `first`, or the pick of the
    /// judge, for this character only. The round of the chapter knows which.
    Chronicle {
        key: CharacterKey,
        first: EventId,
    },
    /// A side quest for this character only (`QuestCall`).
    Quest(QuestCall),
    /// Who the character has become, after the chapter whose first event is `after`, for
    /// this character only. `told` holds the facts of the prompt, for the check of slop.
    Summary {
        key: CharacterKey,
        after: EventId,
        told: String,
    },
    /// The text of a tale, for this character only. `told` holds the facts of the prompt,
    /// for the check of slop.
    Tale {
        key: CharacterKey,
        run: super::tales::TaleRun,
        instance: String,
        told: String,
    },
    /// "Your history here" of a zone, for this character only.
    ZoneHistory(Box<super::zone_histories::HistoryCall>),
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
            Pending::Narrator(_) => "narrator",
            Pending::Chronicle { .. } => "saga",
            Pending::Summary { .. } => "summary",
            Pending::Tale { .. } => "tale",
            Pending::ZoneHistory(_) => "zone_history",
            Pending::Quest(quest) if quest.attempt == Attempt::Retry => QUEST_RETRY,
            Pending::Quest(_) => QUEST,
            Pending::Draft { .. } => "draft",
            Pending::Talk { .. } => TALK,
        }
    }

    /// A call gets a row only in the world of its character. A lore answer changes
    /// nothing in any world, so it gets none.
    fn has_row_in(&self, active: &CharacterKey) -> bool {
        match self {
            Pending::Lore { .. } => false,
            Pending::Narrator(NarratorCall { key, .. })
            | Pending::Chronicle { key, .. }
            | Pending::Summary { key, .. }
            | Pending::Tale { key, .. }
            | Pending::Quest(QuestCall { key, .. })
            | Pending::Draft { key, .. }
            | Pending::Talk { key, .. } => key == active,
            Pending::ZoneHistory(call) => &call.key == active,
        }
    }
}

/// The calls of the story before a line from the bridge.
pub(super) struct CallsBefore {
    next: CallId,
    queued: VecDeque<CallId>,
}

/// A call that the bridge runs or that waits for a slot.
pub(super) struct OpenCall {
    pub(super) pending: Pending,
    /// For the name check of the answer.
    prompt: String,
    /// The position of the call in `calls`, in the world of its character. It stays the
    /// same after a change of character or a reopen. A lore call has none.
    row: Option<u64>,
}

/// The kinds of the `calls` table that a hook counts.
const TALK: &str = "talk";
const QUEST: &str = "quest";
/// A retry is no new offer, so the hook does not count it (docs/plans/quest-variety.md 3.6).
const QUEST_RETRY: &str = "quest_retry";

/// The kinds of call that share the count of the hook (GAMEPLAY.md 3.7).
const HOOK_KINDS: [&str; 2] = [TALK, QUEST];

/// The hook of the next talk or quest call, and the hero row that it reads. The count
/// holds every earlier call of these kinds, so call it before the call opens.
pub(super) fn hook_for<'h>(
    active: &Active,
    hero: &'h Hero,
) -> Result<(Option<Hook<'h>>, Vec<Node>), StoreError> {
    let count = active.count_calls(&HOOK_KINDS)?;
    let hook = hero_hook::hook(hero, count);
    let read = hook.map_or_else(Vec::new, |hook| reads::hook_read(active, hook.field));
    Ok((hook, read))
}

impl Story {
    /// The call ends in its row, and the rows of the line rest on it.
    pub(super) fn answered(&mut self, call: CallId, text: &str) -> Result<Vec<Output>, StoryError> {
        let OpenCall {
            pending,
            prompt,
            row,
        } = self.take_call(call)?;
        self.note_names_in_no_fact(call, text, &prompt);
        let row = self.answer_with(&pending, row);
        let (outputs, outcome) = match pending {
            Pending::Lore { question, lore } => {
                let next = lore.answered(text);
                let outcome =
                    accepted_if(!matches!(&next, Next::Done(answer) if answer.text.is_none()));
                (self.follow(question, next).into_iter().collect(), outcome)
            }
            Pending::Narrator(narration) => self.narrator_answered(row, narration, &prompt, text),
            Pending::Chronicle { key, first } => self.saga_answered(&key, first, Some(text))?,
            Pending::Summary { key, after, told } => {
                self.summary_answered(&key, after, &told, Some(text))?
            }
            Pending::Tale {
                key,
                run,
                instance,
                told,
            } => self.tale_answered(&key, run, instance, &told, Some(text))?,
            Pending::ZoneHistory(call) => self.zone_history_answered(*call, Some(text))?,
            Pending::Talk {
                question,
                key,
                npc,
                at,
            } => {
                let outputs = self.talk_answered(question, &key, npc, at, text, row);
                let said = matches!(
                    outputs.first(),
                    Some(Output::TalkAnswer { text: Some(_), .. })
                );
                (outputs, accepted_if(said))
            }
            Pending::Quest(quest) => self.quest_answered(row, quest, &prompt, text),
            Pending::Draft { question, key } => {
                let answer = self.draft_answered(question, &key, text);
                let outcome = accepted_if(matches!(
                    &answer,
                    Output::DraftAnswer { draft: Some(_), .. }
                ));
                (vec![answer], outcome)
            }
        };
        if let (Some(active), Some(row)) = (self.active.as_mut(), row) {
            active.end_call_row(row, Some(text), outcome, self.told_shape.take());
        }
        Ok(outputs)
    }

    /// The rows of the line rest on the call, and its row ends. A call of another
    /// character has no row here, so the rows of its line rest on nothing. Returns the
    /// row of the call here.
    fn answer_with(&mut self, pending: &Pending, row: Option<u64>) -> Option<u64> {
        let active = self
            .active
            .as_mut()
            .filter(|active| pending.has_row_in(&active.key))?;
        active.answering = row;
        row
    }

    /// The words always show. The change of trust lands only for the character that
    /// talked, and never before the last event, because the world can move on while the
    /// model thinks. Work asks the NPC for a quest, also only for that character. `row` is
    /// the row of the talk call, which the quest call reads.
    pub(super) fn talk_answered(
        &mut self,
        question: MessageId,
        key: &CharacterKey,
        npc: String,
        asked_at: Tick,
        text: &str,
        row: Option<u64>,
    ) -> Vec<Output> {
        let Some(answer) = talk::checked_answer(text, &self.player_text(key)) else {
            return vec![Output::TalkAnswer {
                id: question,
                npc,
                text: None,
                notice: None,
            }];
        };
        let same = self
            .active
            .as_ref()
            .is_some_and(|active| &active.key == key);
        let mut outputs = Vec::new();
        if same {
            self.land_talk(&npc, asked_at, &answer);
        }
        if same && answer.work == Work::Offered {
            let work = TalkWork {
                npc: npc.clone(),
                at: asked_at,
                said: answer.say.clone(),
                call: row,
            };
            outputs = self.quest_from_talk(work);
        }
        outputs.insert(
            0,
            Output::TalkAnswer {
                id: question,
                npc,
                text: Some(answer.say),
                notice: None,
            },
        );
        outputs
    }

    /// The words become a rumor that the NPC remembers (GAMEPLAY.md 3.5), and the change of
    /// trust lands.
    fn land_talk(&mut self, npc: &str, asked_at: Tick, answer: &talk::Answer) {
        let rumor = Rumor {
            at: asked_at,
            npc: npc.to_string(),
            text: answer.say.clone(),
        };
        // A failed write loses one rumor. The words still show.
        if let Some(active) = self.active.as_mut() {
            let _ = active.learned.add_rumor(rumor);
        }
        if answer.trust_change != 0 {
            // A refusal is not possible here. A failed save loses the change, because the
            // character opens again from the disk, and the words still show.
            let _ = self.change(|character| {
                let at = asked_at.max(character.world().tick);
                character.adjust_trust(at, npc, answer.trust_change)
            });
        }
    }

    /// The bridge answers a call over its budget with a failure, so each failure slows the
    /// saga (`Pace`).
    pub(super) fn failed(&mut self, call: CallId) -> Result<Vec<Output>, StoryError> {
        let OpenCall { pending, row, .. } = self.take_call(call)?;
        self.pace.failed(self.newest);
        let row = self.answer_with(&pending, row);
        if let (Some(active), Some(row)) = (self.active.as_mut(), row) {
            active.end_call_row(row, None, Outcome::Failed, None);
        }
        Ok(match pending {
            Pending::Lore { question, lore } => vec![Output::LoreAnswer {
                id: question,
                answer: lore.failed(),
                notice: None,
            }],
            Pending::Narrator(narration) => vec![quests::quiet(narration.batch)],
            Pending::Chronicle { key, first } => self.saga_answered(&key, first, None)?.0,
            Pending::Summary { key, after, told } => {
                self.summary_answered(&key, after, &told, None)?.0
            }
            Pending::Tale {
                key,
                run,
                instance,
                told,
            } => self.tale_answered(&key, run, instance, &told, None)?.0,
            Pending::ZoneHistory(call) => self.zone_history_answered(*call, None)?.0,
            Pending::Talk { question, npc, .. } => vec![Output::TalkAnswer {
                id: question,
                npc,
                text: None,
                notice: None,
            }],
            Pending::Quest(quest) => self.quest_failed(&quest),
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
        let pack = self.pack.label().to_string();
        let row = self
            .active
            .as_mut()
            .filter(|active| pending.has_row_in(&active.key))
            .map(|active| active.open_call_row(pending.kind(), pack, prompt.clone(), reads));
        if let (Pending::Chronicle { .. }, Some(row)) = (&pending, row) {
            self.round_calls.push(row);
        }
        let open = OpenCall {
            pending,
            prompt,
            row,
        };
        self.calls.insert(call, open);
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
        let prompt = self.calls.get(&call)?.prompt.clone();
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

    pub(super) fn calls_before(&self) -> CallsBefore {
        CallsBefore {
            next: self.next_call,
            queued: self.queued.clone(),
        }
    }

    /// The bridge gets no output of a line that fails, so it runs no call of the line. A
    /// call that the line opened goes, and a call that it sent from the queue waits again.
    pub(super) fn take_back_calls(&mut self, before: CallsBefore) {
        let opened_here = |call: &CallId| call.0 >= before.next.0;
        self.calls.retain(|call, _| !opened_here(call));
        self.open
            .retain(|call| !opened_here(call) && !before.queued.contains(call));
        self.queued = before.queued;
    }

    /// The bridge never saw a call that waits for a slot.
    pub(super) fn take_call(&mut self, call: CallId) -> Result<OpenCall, StoryError> {
        if !self.open.remove(&call) {
            return Err(StoryError::UnknownCall(call));
        }
        self.calls
            .remove(&call)
            .ok_or(StoryError::UnknownCall(call))
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
