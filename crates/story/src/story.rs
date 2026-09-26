//! One input in, at most one output out (GAMEPLAY.md 3.1, 5.2, and 5.6).

use crate::character::{Character, Refusal};
use crate::input::{CallId, Input, MessageId};
use crate::journal::{Page, journal, pages};
use crate::lore::{Answer, LoreCall, Next};
use crate::pack::{Link, Pack, PackError, Passage};
use crate::prompt::Context;
use crate::store::{CharacterKey, HistoryFile, Store, StoreError};
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
    #[error("no character yet: a batch starts with character_entered")]
    NoCharacter,
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The character of the last `character_entered`, and the file of its history.
struct Active {
    key: CharacterKey,
    character: Character,
    file: Option<HistoryFile>,
}

impl Active {
    /// Writes the events that the file does not hold yet.
    fn save(&mut self) -> Result<(), StoreError> {
        let Some(file) = &mut self.file else {
            return Ok(());
        };
        let new: Vec<_> = self
            .character
            .world()
            .history()
            .iter()
            .skip(file.len())
            .cloned()
            .collect();
        file.append(&new)
    }
}

/// A model call, and the question that it answers.
struct Pending {
    question: MessageId,
    lore: LoreCall,
}

pub struct Story {
    pack: Pack,
    store: Store,
    active: Option<Active>,
    calls: BTreeMap<CallId, Pending>,
    next_call: CallId,
    /// The pages of the last journal request. Later pages come from here, so events that
    /// arrive between two requests never shift an entry to another page.
    journal: Vec<Page>,
}

impl Story {
    #[must_use]
    pub fn new(pack: Pack, store: Store) -> Story {
        Story {
            pack,
            store,
            active: None,
            calls: BTreeMap::new(),
            next_call: CallId(1),
            journal: Vec::new(),
        }
    }

    /// # Errors
    ///
    /// Returns the refusal of the world for a game event, the error of the pack for a
    /// question, `UnknownCall` for the answer to a call that is not open, `NoCharacter`
    /// before the first `character_entered`, and the error of the store.
    pub fn handle(&mut self, input: Input) -> Result<Option<Output>, StoryError> {
        match input {
            Input::Hello => Ok(Some(Output::Hello { protocol: PROTOCOL })),
            Input::CharacterEntered { realm, name } => {
                self.enter_character(&realm, &name)?;
                Ok(None)
            }
            Input::ZoneEntered { at, zone, subzone } => {
                self.change(|character| character.enter_zone(at, &zone, subzone.as_deref()))
            }
            Input::NpcMet { at, name } => self.change(|character| character.meet_npc(at, &name)),
            Input::LevelReached { at, level } => {
                self.change(|character| character.reach_level(at, level))
            }
            Input::LoreAsked {
                id,
                question,
                target,
                ..
            } => Ok(Some(self.ask(id, &question, target.as_deref())?)),
            Input::JournalAsked { id, page } => {
                let page = self.journal_page(page)?;
                Ok(Some(Output::Journal { id, page }))
            }
            Input::ModelAnswered { call, text } => {
                let pending = self.take_call(call)?;
                let next = pending.lore.answered(&text);
                Ok(Some(self.follow(pending.question, next)))
            }
            Input::ModelFailed { call } => {
                let pending = self.take_call(call)?;
                let answer = pending.lore.failed();
                Ok(Some(Output::LoreAnswer {
                    id: pending.question,
                    answer,
                }))
            }
        }
    }

    /// The same character again changes nothing, so each batch can name it.
    fn enter_character(&mut self, realm: &str, name: &str) -> Result<(), StoryError> {
        let key = CharacterKey::new(realm, name)?;
        if self.active.as_ref().is_some_and(|active| active.key == key) {
            return Ok(());
        }
        let (character, file) = self.store.open(&key)?;
        self.active = Some(Active {
            key,
            character,
            file,
        });
        self.journal.clear();
        Ok(())
    }

    fn character(&self) -> Result<&Character, StoryError> {
        self.active
            .as_ref()
            .map(|active| &active.character)
            .ok_or(StoryError::NoCharacter)
    }

    /// The events that landed before a refusal stay, so they are saved in both cases.
    fn change(
        &mut self,
        act: impl FnOnce(&mut Character) -> Result<(), Refusal>,
    ) -> Result<Option<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let changed = act(&mut active.character);
        active.save()?;
        changed.map_err(StoryError::Refused)?;
        Ok(None)
    }

    /// A page past the end comes back empty, with the true number of pages.
    fn journal_page(&mut self, page: usize) -> Result<Page, StoryError> {
        if page == 0 || self.journal.is_empty() {
            self.journal = pages(journal(self.character()?));
        }
        let page = self.journal.get(page).cloned().unwrap_or_else(|| Page {
            page,
            pages: self.journal.len(),
            ..Page::default()
        });
        Ok(page)
    }

    /// With no passage, a model has nothing to cite, so no call goes out.
    fn ask(
        &mut self,
        id: MessageId,
        question: &str,
        target: Option<&str>,
    ) -> Result<Output, StoryError> {
        let character = self.character()?;
        let passages = passages_for(&self.pack, character, question, target)?;
        if passages.is_empty() {
            let answer = Answer {
                text: None,
                passages,
            };
            return Ok(Output::LoreAnswer { id, answer });
        }
        let context = Context {
            places: character.place_names(),
            target,
            level: character.level(),
        };
        let call = LoreCall::new(question, &context, passages);
        Ok(self.follow(id, Next::Ask(call)))
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

/// The question and where you stand pick the passages. The spoiler limit then drops each
/// passage about something that your world does not hold.
fn passages_for(
    pack: &Pack,
    character: &Character,
    question: &str,
    target: Option<&str>,
) -> Result<Vec<Passage>, PackError> {
    let mut words = vec![question];
    words.extend(character.place_names());
    words.extend(target);
    let found = pack.search(&words.join(" "), CANDIDATES)?;
    let passages = found
        .into_iter()
        .filter(|passage| knows_all(character, &passage.links))
        .take(ANSWER_SIZE)
        .collect();
    Ok(passages)
}

fn knows_all(character: &Character, links: &[Link]) -> bool {
    links.iter().all(|link| match link {
        Link::Place(name) => character.has_visited(name),
        Link::Npc(name) => character.has_met(name),
    })
}

fn reasons(refusal: &Refusal) -> String {
    let reasons: Vec<String> = refusal.iter().map(ToString::to_string).collect();
    reasons.join("; ")
}
