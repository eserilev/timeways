//! One input in, at most one output out (GAMEPLAY.md 3.1, 5.2, and 5.6).

use crate::character::{Character, Refusal};
use crate::input::{CallId, Input};
use crate::lore::{Answer, LoreCall, Next};
use crate::pack::{Link, Pack, PackError, Passage};
use crate::prompt::Context;
use serde::Serialize;
use std::collections::BTreeMap;
use thiserror::Error;

/// Enough candidates that the spoiler limit still leaves a full answer.
const CANDIDATES: u32 = 50;
const ANSWER_SIZE: usize = 8;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Output {
    LoreAnswer(Answer),
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

pub struct Story {
    character: Character,
    pack: Pack,
    calls: BTreeMap<CallId, LoreCall>,
    next_call: CallId,
}

impl Story {
    #[must_use]
    pub fn new(character: Character, pack: Pack) -> Story {
        Story {
            character,
            pack,
            calls: BTreeMap::new(),
            next_call: CallId(1),
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
            Input::ZoneEntered { at, zone, subzone } => {
                character.enter_zone(at, &zone, subzone.as_deref())
            }
            Input::NpcMet { at, name } => character.meet_npc(at, &name),
            Input::LevelReached { at, level } => character.reach_level(at, level),
            Input::LoreAsked {
                question, target, ..
            } => {
                return Ok(Some(self.ask(&question, target.as_deref())?));
            }
            Input::ModelAnswered { call, text } => {
                let next = self.take_call(call)?.answered(&text);
                return Ok(Some(self.follow(next)));
            }
            Input::ModelFailed { call } => {
                let answer = self.take_call(call)?.failed();
                return Ok(Some(Output::LoreAnswer(answer)));
            }
        };
        changed.map_err(StoryError::Refused)?;
        Ok(None)
    }

    /// With no passage, a model has nothing to cite, so no call goes out.
    fn ask(&mut self, question: &str, target: Option<&str>) -> Result<Output, PackError> {
        let passages = self.passages_for(question, target)?;
        if passages.is_empty() {
            return Ok(Output::LoreAnswer(Answer {
                text: None,
                passages,
            }));
        }
        let context = Context {
            places: self.character.place_names(),
            target,
            level: self.character.level(),
        };
        let call = LoreCall::new(question, &context, passages);
        Ok(self.follow(Next::Ask(call)))
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

    fn follow(&mut self, next: Next) -> Output {
        match next {
            Next::Done(answer) => Output::LoreAnswer(answer),
            Next::Ask(lore_call) => {
                let call = self.next_call;
                self.next_call = CallId(call.0 + 1);
                let prompt = lore_call.prompt().to_string();
                self.calls.insert(call, lore_call);
                Output::ModelCall { call, prompt }
            }
        }
    }

    fn take_call(&mut self, call: CallId) -> Result<LoreCall, StoryError> {
        self.calls
            .remove(&call)
            .ok_or(StoryError::UnknownCall(call))
    }
}

fn reasons(refusal: &Refusal) -> String {
    let reasons: Vec<String> = refusal.iter().map(ToString::to_string).collect();
    reasons.join("; ")
}
