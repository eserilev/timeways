//! One input in, at most one output out (GAMEPLAY.md 3.1, 5.2, and 5.6).

use crate::bard;
use crate::character::{Character, Refusal};
use crate::companion::{self, Budget};
use crate::input::{CallId, Input, MessageId};
use crate::journal::{PAGE_BYTES, Page, journal, pages};
use crate::lore::{Answer, LoreCall, Next};
use crate::moments::{Moment, best, moments};
use crate::pack::{Link, Pack, PackError, Passage};
use crate::prompt::Context;
use crate::store::{CharacterKey, HistoryFile, Opened, Prose, Store, StoreError};
use crate::talk::{self, Scene};
use hourglass::Tick;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// The longest name that the story takes from the game. WoW names are far shorter, so a
/// longer one comes from a bug or a hostile addon, and it would break the page limit.
pub const MAX_NAME_BYTES: usize = 96;

/// The chat box of WoW stops a line at 255 bytes.
pub const MAX_WORDS_BYTES: usize = 255;

/// The passages about an NPC that its prompt carries.
const TALK_PASSAGES: usize = 3;

/// The version of the lines between the bridge and the story program.
pub const PROTOCOL: u32 = 1;

/// Enough candidates that the spoiler limit still leaves a full answer.
const CANDIDATES: u32 = 50;
const ANSWER_SIZE: usize = 8;

/// The passages of one answer. The rest of a reply holds the model text of at most
/// `check::MAX_CHARS` characters, which JSON escaping makes up to 6 times longer, and the
/// frame.
const PASSAGE_BYTES: usize = PAGE_BYTES - 8 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
    /// What the NPC says, or null when no model answered.
    TalkAnswer {
        id: MessageId,
        npc: String,
        text: Option<String>,
    },
    /// The answer to `batch_end`. The bridge shows `companion` in the game.
    EventsSeen {
        id: MessageId,
        companion: Option<String>,
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
    #[error("a name is empty, longer than {MAX_NAME_BYTES} bytes, or holds a control character")]
    BadName,
    #[error(
        "the words are empty, longer than {MAX_WORDS_BYTES} bytes, or hold a control character"
    )]
    BadWords,
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The character of the last `character_entered`, and the file of its history.
struct Active {
    key: CharacterKey,
    character: Character,
    file: Option<HistoryFile>,
    prose: Prose,
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

/// An open model call, and what its answer is for.
enum Pending {
    Lore {
        question: MessageId,
        lore: LoreCall,
    },
    Companion {
        batch: MessageId,
    },
    /// The saga of the chapter that began at `began`, for this character only.
    Bard {
        key: CharacterKey,
        began: Tick,
    },
    /// A talk to `npc`, whose change of trust lands at `at`, for this character only.
    Talk {
        question: MessageId,
        key: CharacterKey,
        npc: String,
        at: Tick,
    },
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
    /// The big moments of the batch so far. `batch_end` takes them.
    moments: Vec<Moment>,
    budget: Budget,
    /// The chapters of the active character that the bard was asked for in this run. A
    /// failed chapter keeps its plain list, and gets no second call.
    bard_asked: BTreeSet<Tick>,
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
            moments: Vec::new(),
            budget: Budget::default(),
            bard_asked: BTreeSet::new(),
        }
    }

    /// # Errors
    ///
    /// Returns the refusal of the world for a game event, the error of the pack for a
    /// question, `UnknownCall` for the answer to a call that is not open, `NoCharacter`
    /// before the first `character_entered`, and the error of the store.
    pub fn handle(&mut self, input: Input) -> Result<Vec<Output>, StoryError> {
        match input {
            Input::Hello => Ok(vec![Output::Hello { protocol: PROTOCOL }]),
            Input::CharacterEntered { realm, name } => {
                self.enter_character(&realm, &name)?;
                Ok(Vec::new())
            }
            Input::ZoneEntered { at, zone, subzone } => {
                checked_name(&zone)?;
                subzone.as_deref().map(checked_name).transpose()?;
                self.change(|character| character.enter_zone(at, &zone, subzone.as_deref()))
            }
            Input::NpcMet { at, name } => {
                checked_name(&name)?;
                self.change(|character| character.meet_npc(at, &name))
            }
            Input::NpcDefeated { at, name } => {
                checked_name(&name)?;
                self.change(|character| character.defeat_npc(at, &name))
            }
            Input::NpcSlapped { at, name } => {
                checked_name(&name)?;
                self.change(|character| character.slap(at, &name))
            }
            Input::Died { at, killer } => {
                killer.as_deref().map(checked_name).transpose()?;
                self.change(|character| character.die(at, killer.as_deref()))
            }
            Input::LevelReached { at, level } => {
                self.change(|character| character.reach_level(at, level))
            }
            Input::LoreAsked {
                id,
                question,
                target,
            } => Ok(vec![self.ask(id, &question, target.as_deref())?]),
            Input::TalkAsked { id, at, npc, text } => Ok(vec![self.talk(id, at, &npc, &text)?]),
            Input::JournalAsked { id, page } => {
                let page = self.journal_page(page)?;
                Ok(vec![Output::Journal { id, page }])
            }
            Input::BatchEnd { id } => Ok(self.end_batch(id)),
            Input::ModelAnswered { call, text } => self.answered(call, &text),
            Input::ModelFailed { call } => self.failed(call),
        }
    }

    fn answered(&mut self, call: CallId, text: &str) -> Result<Vec<Output>, StoryError> {
        Ok(match self.take_call(call)? {
            Pending::Lore { question, lore } => vec![self.follow(question, lore.answered(text))],
            Pending::Companion { batch } => vec![Output::EventsSeen {
                id: batch,
                companion: companion::checked_line(text),
            }],
            Pending::Bard { key, began } => {
                let active = self.active.as_mut().filter(|active| active.key == key);
                if let (Some(active), Some(chapter)) = (active, bard::checked_chapter(text)) {
                    active.prose.add(began, chapter)?;
                }
                Vec::new()
            }
            Pending::Talk {
                question,
                key,
                npc,
                at,
            } => vec![self.talk_answered(question, &key, npc, at, text)],
        })
    }

    /// The words always show. The change of trust lands only for the character that
    /// talked, and never before the last event, because the world can move on while the
    /// model thinks.
    fn talk_answered(
        &mut self,
        question: MessageId,
        key: &CharacterKey,
        npc: String,
        asked_at: Tick,
        text: &str,
    ) -> Output {
        let Some(answer) = talk::checked_answer(text) else {
            return Output::TalkAnswer {
                id: question,
                npc,
                text: None,
            };
        };
        let same = self
            .active
            .as_ref()
            .is_some_and(|active| &active.key == key);
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
        }
    }

    fn failed(&mut self, call: CallId) -> Result<Vec<Output>, StoryError> {
        Ok(match self.take_call(call)? {
            Pending::Lore { question, lore } => vec![Output::LoreAnswer {
                id: question,
                answer: lore.failed(),
            }],
            Pending::Companion { batch } => vec![Output::EventsSeen {
                id: batch,
                companion: None,
            }],
            Pending::Bard { .. } => Vec::new(),
            Pending::Talk { question, npc, .. } => vec![Output::TalkAnswer {
                id: question,
                npc,
                text: None,
            }],
        })
    }

    /// The same character again changes nothing, so each batch can name it.
    /// A refused switch leaves no character active, so the events of the new character
    /// never land in the world of the old one.
    fn enter_character(&mut self, realm: &str, name: &str) -> Result<(), StoryError> {
        let key = CharacterKey::new(realm, name);
        let same = |key: &CharacterKey| {
            self.active
                .as_ref()
                .is_some_and(|active| &active.key == key)
        };
        if key.as_ref().is_ok_and(same) {
            return Ok(());
        }
        self.active = None;
        self.journal.clear();
        self.moments.clear();
        self.bard_asked.clear();
        let key = key?;
        let Opened {
            character,
            history,
            prose,
        } = self.store.open(&key)?;
        self.active = Some(Active {
            key,
            character,
            file: history,
            prose,
        });
        Ok(())
    }

    fn character(&self) -> Result<&Character, StoryError> {
        self.active
            .as_ref()
            .map(|active| &active.character)
            .ok_or(StoryError::NoCharacter)
    }

    /// The events that landed before a refusal stay, so they are saved in both cases, and
    /// their moments count.
    fn change(
        &mut self,
        act: impl FnOnce(&mut Character) -> Result<(), Refusal>,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let before = active.character.world().history().len();
        let changed = act(&mut active.character);
        let world = active.character.world();
        let added: Vec<_> = world.history().iter().skip(before).cloned().collect();
        self.moments
            .extend(moments(world, active.character.you(), &added));
        active.save()?;
        changed.map_err(StoryError::Refused)?;
        Ok(Vec::new())
    }

    /// At most one companion line for a batch: about its best moment, within the budget.
    /// A batch can also start the saga of a finished chapter.
    fn end_batch(&mut self, batch: MessageId) -> Vec<Output> {
        let seen = self.companion_call(batch);
        let mut outputs = vec![seen];
        outputs.extend(self.bard_call());
        outputs
    }

    fn companion_call(&mut self, batch: MessageId) -> Output {
        let quiet = Output::EventsSeen {
            id: batch,
            companion: None,
        };
        let Some(moment) = best(std::mem::take(&mut self.moments)) else {
            return quiet;
        };
        let now = self
            .character()
            .map_or(Tick(0), |character| character.world().tick);
        if !self.budget.take(now) {
            return quiet;
        }
        self.open_call(Pending::Companion { batch }, companion::prompt(&moment))
    }

    /// The oldest finished chapter with no saga yet. The bridge runs at most 2 model calls
    /// at once, so the bard waits until no other call is open: a question of the player
    /// never fails for a saga. The last chapter can still grow, so it waits for the next
    /// session.
    fn bard_call(&mut self) -> Option<Output> {
        if !self.calls.is_empty() {
            return None;
        }
        let active = self.active.as_ref()?;
        let chapters = journal(&active.character).chapters;
        let (_, finished) = chapters.split_last()?;
        let chapter = finished.iter().find(|chapter| {
            active.prose.get(chapter.began).is_none() && !self.bard_asked.contains(&chapter.began)
        })?;
        let pending = Pending::Bard {
            key: active.key.clone(),
            began: chapter.began,
        };
        let prompt = bard::prompt(chapter);
        self.bard_asked.insert(chapter.began);
        Some(self.open_call(pending, prompt))
    }

    /// A page past the end comes back empty, with the true number of pages.
    fn journal_page(&mut self, page: usize) -> Result<Page, StoryError> {
        if page == 0 || self.journal.is_empty() {
            let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
            let mut journal = journal(&active.character);
            for chapter in &mut journal.chapters {
                chapter.prose = active.prose.get(chapter.began).map(str::to_string);
            }
            self.journal = pages(journal);
        }
        let page = self.journal.get(page).cloned().unwrap_or_else(|| Page {
            page,
            pages: self.journal.len(),
            ..Page::default()
        });
        Ok(page)
    }

    /// Talking is meeting, so the NPC enters the world before the model answers.
    fn talk(
        &mut self,
        id: MessageId,
        at: Tick,
        npc: &str,
        words: &str,
    ) -> Result<Output, StoryError> {
        checked_name(npc)?;
        if words.trim().is_empty()
            || words.len() > MAX_WORDS_BYTES
            || words.chars().any(char::is_control)
        {
            return Err(StoryError::BadWords);
        }
        self.change(|character| character.meet_npc(at, npc))?;
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let character = &active.character;
        let mut passages = passages_for(&self.pack, character, words, Some(npc))?;
        passages.truncate(TALK_PASSAGES);
        let scene = Scene {
            npc,
            place: character.place_of(npc),
            level: character.level(),
            trust: character.trust_of(npc),
            slapped: character.slaps_of(npc),
        };
        let prompt = talk::prompt(&scene, &passages, words);
        let pending = Pending::Talk {
            question: id,
            key: active.key.clone(),
            npc: npc.to_string(),
            at,
        };
        Ok(self.open_call(pending, prompt))
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
                let prompt = lore.prompt().to_string();
                self.open_call(Pending::Lore { question, lore }, prompt)
            }
        }
    }

    fn open_call(&mut self, pending: Pending, prompt: String) -> Output {
        let call = self.next_call;
        self.next_call = CallId(call.0 + 1);
        self.calls.insert(call, pending);
        Output::ModelCall { call, prompt }
    }

    fn take_call(&mut self, call: CallId) -> Result<Pending, StoryError> {
        self.calls
            .remove(&call)
            .ok_or(StoryError::UnknownCall(call))
    }
}

/// The question and where you stand pick the passages. The spoiler limit then drops each
/// passage about something that your world does not hold. The rest stop at the size that
/// leaves room for a model answer in one reply.
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
    let mut passages = Vec::new();
    let mut used = 0;
    for passage in found
        .into_iter()
        .filter(|passage| knows_all(character, &passage.links))
    {
        used += serde_json::to_vec(&passage).map_or(PAGE_BYTES, |bytes| bytes.len()) + 1;
        if passages.len() == ANSWER_SIZE || used > PASSAGE_BYTES {
            break;
        }
        passages.push(passage);
    }
    Ok(passages)
}

fn knows_all(character: &Character, links: &[Link]) -> bool {
    links.iter().all(|link| match link {
        Link::Place(name) => character.has_visited(name),
        Link::Npc(name) => character.has_met(name),
    })
}

fn checked_name(name: &str) -> Result<&str, StoryError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control) {
        return Err(StoryError::BadName);
    }
    Ok(name)
}

fn reasons(refusal: &Refusal) -> String {
    let reasons: Vec<String> = refusal.iter().map(ToString::to_string).collect();
    reasons.join("; ")
}
