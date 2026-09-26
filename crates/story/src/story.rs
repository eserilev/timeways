//! One input in, at most one output out (GAMEPLAY.md 3.1, 5.2, and 5.6).

use crate::character::{Character, Refusal};
use crate::input::{CallId, Input, MessageId};
use crate::journal::{Page, journal, pages};
use crate::lore::{Answer, LoreCall, Next};
use crate::pack::{Link, Pack, PackError, Passage};
use crate::prompt::Context;
use serde::Serialize;
use std::collections::BTreeMap;
use thiserror::Error;

/// The version of the lines between the bridge and the story program.
pub const PROTOCOL: u32 = 1;

/// Enough candidates that the spoiler limit still leaves a full answer.
const CANDIDATES: u32 = 50;
const ANSWER_SIZE: usize = 8;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Output {
    Hello {
        protocol: u32,
    },
    LoreAnswer {
        id: MessageId,
        #[serde(flatten)]
        answer: Answer,
    },
    Journal {
        id: MessageId,
        #[serde(flatten)]
        page: Page,
    },
    /// The bridge runs the model with no tools, and answers with `model_answered` or
    /// `model_failed` for the same call.
    ModelCall {
        call: CallId,
        prompt: String,
    },
}

#[derive(Debug, Error)]
pub enum StoryError {
    #[error("refused: {}", reasons(.0))]
    Refused(Refusal),
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error("no model call has the id {}", .0.0)]
    UnknownCall(CallId),
}

/// A model call, and the question that it answers.
struct Pending {
    question: MessageId,
    lore: LoreCall,
}

pub struct Story {
    character: Character,
    pack: Pack,
    calls: BTreeMap<CallId, Pending>,
    next_call: CallId,
    /// The pages of the last journal request. Later pages come from here, so events that
    /// arrive between two requests never shift an entry to another page.
    journal: Vec<Page>,
}

impl Story {
    #[must_use]
    pub fn new(character: Character, pack: Pack) -> Story {
        Story {
            character,
            pack,
            calls: BTreeMap::new(),
            next_call: CallId(1),
            journal: Vec::new(),
        }
    }

    #[must_use]
    pub fn character(&self) -> &Character {
        &self.character
    }

    /// # Errors
    ///
    /// Returns the refusal of the world for a game event, the error of the pack for a
    /// question, and `UnknownCall` for the answer to a call that is not open.
    pub fn handle(&mut self, input: Input) -> Result<Option<Output>, StoryError> {
        let character = &mut self.character;
        let changed = match input {
            Input::Hello => return Ok(Some(Output::Hello { protocol: PROTOCOL })),
            Input::ZoneEntered { at, zone, subzone } => {
                character.enter_zone(at, &zone, subzone.as_deref())
            }
            Input::NpcMet { at, name } => character.meet_npc(at, &name),
            Input::LevelReached { at, level } => character.reach_level(at, level),
            Input::LoreAsked {
                id,
                question,
                target,
                ..
            } => {
                return Ok(Some(self.ask(id, &question, target.as_deref())?));
            }
            Input::JournalAsked { id, page } => {
                let page = self.journal_page(page);
                return Ok(Some(Output::Journal { id, page }));
            }
            Input::ModelAnswered { call, text } => {
                let pending = self.take_call(call)?;
                let next = pending.lore.answered(&text);
                return Ok(Some(self.follow(pending.question, next)));
            }
            Input::ModelFailed { call } => {
                let pending = self.take_call(call)?;
                let answer = pending.lore.failed();
                return Ok(Some(Output::LoreAnswer {
                    id: pending.question,
                    answer,
                }));
            }
        };
        changed.map_err(StoryError::Refused)?;
        Ok(None)
    }

    /// A page past the end comes back empty, with the true number of pages.
    fn journal_page(&mut self, page: usize) -> Page {
        if page == 0 || self.journal.is_empty() {
            self.journal = pages(journal(&self.character));
        }
        self.journal.get(page).cloned().unwrap_or_else(|| Page {
            page,
            pages: self.journal.len(),
            ..Page::default()
        })
    }

    /// With no passage, a model has nothing to cite, so no call goes out.
    fn ask(
        &mut self,
        id: MessageId,
        question: &str,
        target: Option<&str>,
    ) -> Result<Output, PackError> {
        let passages = self.passages_for(question, target)?;
        if passages.is_empty() {
            let answer = Answer {
                text: None,
                passages,
            };
            return Ok(Output::LoreAnswer { id, answer });
        }
        let context = Context {
            places: self.character.place_names(),
            target,
            level: self.character.level(),
        };
        let call = LoreCall::new(question, &context, passages);
        Ok(self.follow(id, Next::Ask(call)))
    }

    /// The question and where you stand pick the passages. The spoiler limit then drops
    /// each passage about something that your world does not hold.
    fn passages_for(
        &self,
        question: &str,
        target: Option<&str>,
    ) -> Result<Vec<Passage>, PackError> {
        let mut words = vec![question];
        words.extend(self.character.place_names());
        words.extend(target);
        let found = self.pack.search(&words.join(" "), CANDIDATES)?;
        let passages = found
            .into_iter()
            .filter(|passage| self.knows_all(&passage.links))
            .take(ANSWER_SIZE)
            .collect();
        Ok(passages)
    }

    fn knows_all(&self, links: &[Link]) -> bool {
        links.iter().all(|link| match link {
            Link::Place(name) => self.character.has_visited(name),
            Link::Npc(name) => self.character.has_met(name),
        })
    }

    fn follow(&mut self, question: MessageId, next: Next) -> Output {
        match next {
            Next::Done(answer) => Output::LoreAnswer {
                id: question,
                answer,
            },
            Next::Ask(lore) => {
                let call = self.next_call;
                self.next_call = CallId(call.0 + 1);
                let prompt = lore.prompt().to_string();
                self.calls.insert(call, Pending { question, lore });
                Output::ModelCall { call, prompt }
            }
        }
    }

    fn take_call(&mut self, call: CallId) -> Result<Pending, StoryError> {
        self.calls
            .remove(&call)
            .ok_or(StoryError::UnknownCall(call))
    }
}

fn reasons(refusal: &Refusal) -> String {
    let reasons: Vec<String> = refusal.iter().map(ToString::to_string).collect();
    reasons.join("; ")
}
