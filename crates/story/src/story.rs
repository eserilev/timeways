//! One input in, and the outputs for it out (GAMEPLAY.md 3.1, 5.2, and 5.6).

use crate::aliases::AliasRow;
use crate::atlas_lore::lore_of;
use crate::best_of_two::Round;
use crate::chapters::{self, Book};
use crate::character::{Character, Item, Refusal};
use crate::check;
use crate::dev_fps::{self, FpsRun};
use crate::dev_mode::{DevMode, DevOn};
use crate::draft::Draft;
use crate::flavor::{self, Flavor, HUMBLING_GAP, Kind, Teller, Told};
use crate::game_names;
use crate::gear::{self, Before, Quality};
use crate::hero::{self, Change, Entry};
use crate::input::{CallId, FoeKind, GameQuestKind, Input, MessageId, Reaction, SlotWas, Taxi};
use crate::journal::{Page, pages};
use crate::learned::{Read, learned};
use crate::lore::{Answer, LoreCall, Next};
use crate::moments::{Moment, best, moments};
use crate::mounts;
use crate::narrator::Budget;
use crate::npc_memory::{self, Memory, Past};
use crate::pace::Pace;
use crate::pack::{Pack, PackError, Passage};
use crate::passage_limits;
use crate::places::InstanceKind;
use crate::prompt::Context;
use crate::quest::{Encounter, QuestView, Status, quest_log};
use crate::race_class::{Class, Race};
use crate::reply_size::{MAX_LINE, MAX_SLOT, Size};
use crate::seen::{MAX_SEEN_BYTES, SeenIndex, SeenText, TextKind};
use crate::spoiler;
use crate::spot::Spot;
use crate::store::{CharacterKey, Node, Opened, Shared, Store, StoreError, Table};
use crate::stories::MAX_PARAGRAPHS;
use crate::talk::{self, QuestTalk, Scene};
use crate::titles;
use hourglass::{EventId, Tick};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{Duration, Instant};
use thiserror::Error;

mod active;
mod aliases;
mod calls;
mod drafts;
mod edits;
mod narration;
mod prologues;
mod quests;
mod ratings;
mod reads;
mod sagas;
mod stories;
mod summaries;
mod tales;
pub mod why;
mod zone_histories;

use active::{Active, Kept};
use calls::{OpenCall, Pending};
use quests::QuestRequest;

const DAY_SECONDS: u64 = 24 * 3600;

/// The names of the values in `timeways.sqlite`.
const BUDGET: &str = "budget";
const PACE: &str = "pace";

fn now() -> Tick {
    let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    Tick(since_epoch.map_or(0, |elapsed| elapsed.as_secs()))
}

/// The longest name that the story takes from the game. WoW names are far shorter, so a
/// longer one comes from a bug or a hostile addon, and it breaks the page limit.
pub const MAX_NAME_BYTES: usize = 96;

/// A flavor moment needs this score to reach the narrator (GAMEPLAY.md 5.4.1).
const FLAVOR_MIN_SCORE: i64 = 8;

/// At most one flavor line in this time.
const FLAVOR_GAP_SECONDS: u64 = 20 * 60;

/// "No two rabbit jokes in one evening."
const KIND_COOLDOWN_SECONDS: u64 = 12 * 3600;

/// "The top 5 moments of a session go to the model" (GAMEPLAY.md 5.4.1).
const CHAPTER_MOMENTS: usize = 5;

/// The chat box of WoW stops a line at 255 bytes.
pub const MAX_WORDS_BYTES: usize = 255;

/// The passages about an NPC that its prompt carries.
const TALK_PASSAGES: usize = 3;

/// The bridge runs at most this many model calls of the story program at once, and fails
/// one more at once (relay SPEC.md 9.8).
const MAX_OPEN_CALLS: usize = 2;

/// The bridge drops an `events_seen` after 60 seconds. The margin covers the time that a
/// line takes between the two programs.
const EVENTS_DEADLINE: Duration = Duration::from_secs(55);

/// The version of the lines between the bridge and the story program.
pub const PROTOCOL: u32 = 1;

/// Enough candidates that the spoiler limit still leaves a full answer.
const CANDIDATES: u32 = 50;
const ANSWER_SIZE: usize = 8;

/// The passages of one answer. The rest of a reply holds the model text of at most
/// `check::MAX_CHARS` characters, and the frame. JSON escaping makes the text up to 6 times
/// longer in the line. In the slot, a character takes up to 4 bytes, and the Lua escape
/// makes each byte up to 4.
const PASSAGES: Size = Size {
    line: MAX_LINE - 8 * 1024,
    slot: MAX_SLOT - 16 * check::MAX_CHARS - 2048,
};

/// The largest reply that the bridge takes, in its line and in the slot of the game.
const REPLY: Size = Size {
    line: MAX_LINE,
    slot: MAX_SLOT,
};

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
        #[serde(skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },
    /// The page is boxed, because it is far larger than the other outputs. `dev` is there
    /// only while dev mode is on.
    Journal {
        id: MessageId,
        #[serde(flatten)]
        page: Box<Page>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        dev: Option<DevOn>,
        /// `"past": "wanted"` until a past came, so the addon sends one at a login
        /// (GAMEPLAY.md 3.3, the prologue).
        #[serde(skip_serializing_if = "Option::is_none")]
        past: Option<PastWanted>,
    },
    /// What the NPC says, or null when no model answered.
    TalkAnswer {
        id: MessageId,
        npc: String,
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },
    /// The answer to `batch_end`. The bridge shows `narrator` in the game.
    EventsSeen {
        id: MessageId,
        narrator: Option<String>,
        /// A line of Timeways itself, not of the narrator: "You already have 3 tasks.".
        #[serde(skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },
    /// A draft of a player task, or null when no model answered or the draft broke a rule.
    DraftAnswer {
        id: MessageId,
        draft: Option<Draft>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
    },
    /// The bridge runs the model with no tools, and answers with `model_answered` or
    /// `model_failed` for the same call.
    ModelCall {
        call: CallId,
        prompt: String,
    },
}

impl Output {
    /// The notice of an answer, or None for a line that answers no batch.
    fn notice_mut(&mut self) -> Option<&mut Option<String>> {
        match self {
            Output::LoreAnswer { notice, .. }
            | Output::Journal { notice, .. }
            | Output::TalkAnswer { notice, .. }
            | Output::EventsSeen { notice, .. }
            | Output::DraftAnswer { notice, .. } => Some(notice),
            Output::Hello { .. } | Output::ModelCall { .. } => None,
        }
    }

    /// True when the notice of this answer is free, and the answer with `line` as its
    /// notice still fits one reply.
    fn has_room_for(&self, line: &str) -> bool {
        let mut with_notice = self.clone();
        let Some(notice) = with_notice.notice_mut().filter(|notice| notice.is_none()) else {
            return false;
        };
        *notice = Some(line.to_string());
        Size::of(&with_notice).fits(REPLY)
    }
}

/// The journal asks the addon for the past of the character.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PastWanted {
    Wanted,
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
    #[error(
        "a seen text is empty, longer than {MAX_SEEN_BYTES} bytes, or holds a control character"
    )]
    BadSeenText,
    #[error("seen text: {0}")]
    Seen(#[from] rusqlite::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(
        "a story has no paragraph, more than {MAX_PARAGRAPHS}, an empty one, a space at the end of one, a title or a body over its limit, or a control character or a |"
    )]
    BadStory,
    #[error("a story with the number {0} came before")]
    StoryTaken(u64),
    #[error("no story with the number {0} stands")]
    NoStory(u64),
    #[error("the story with the number {0} is part of the story now, so it stays")]
    StoryInUse(u64),
    #[error("the time {0} is more than a day after the clock of this computer")]
    FutureTime(u64),
    #[error("an emote or a cause of death is empty, longer than 24 bytes, or not lowercase")]
    BadToken,
    #[error("an hour is past 23")]
    BadHour,
    #[error("an item has a slot outside 1 to 19, or a quality that Classic does not have")]
    BadItem,
    #[error("refused: a run of /twdev fps, and dev mode is off")]
    DevModeOff,
    #[error("a run of /twdev fps has a bad label, step, or sample list")]
    BadFpsRun,
    #[error("the file of the FPS runs: {0}")]
    FpsFile(std::io::Error),
    #[error(transparent)]
    BadPast(#[from] crate::past::PastError),
}

/// A flavor moment of the batch, scored when it came in.
struct Candidate {
    flavor: Flavor,
    score: i64,
    /// 1 for the first moment of its kind.
    count: usize,
}

/// A batch of game events that waits for its `events_seen`, since its `batch_end`.
#[derive(Clone, Copy)]
struct EventsBatch {
    id: MessageId,
    ended: Instant,
}

pub struct Story {
    pack: Pack,
    store: Store,
    active: Option<Active>,
    /// Each call that the bridge runs or that waits for a slot.
    calls: BTreeMap<CallId, OpenCall>,
    /// The calls that the bridge runs now.
    open: BTreeSet<CallId>,
    /// The calls of the player that wait for a slot, oldest first.
    queued: VecDeque<CallId>,
    /// Lines for the log of the program, and never for the player.
    notes: Vec<String>,
    next_call: CallId,
    /// The pages of the last journal request. Later pages come from here, so events that
    /// arrive between two requests never shift an entry to another page.
    journal: Vec<Page>,
    /// The big moments of the batch so far. `batch_end` takes them.
    moments: Vec<Moment>,
    /// The flavor moments of the batch so far, each with its score and its count.
    candidates: Vec<Candidate>,
    budget: Budget,
    /// What all characters share. It opens with the first character, and holds the
    /// budget and the pace across a restart.
    shared: Option<Shared>,
    /// The chapters of the active character that the narrator was asked a saga for in
    /// this run. A failed chapter keeps its plain list, and gets no second round.
    chronicle_asked: BTreeSet<EventId>,
    /// The drafts of the saga that is written now. Only a final saga goes to the disk.
    saga_round: Option<Round>,
    /// The runs of the active character that a tale text was asked for in this run.
    tale_asked: BTreeSet<EventId>,
    /// The chapters of the active character that a zone history was asked for in this run.
    history_asked: BTreeSet<EventId>,
    pace: Pace,
    /// The rows that the batch so far added: its events and its flavor moments. The
    /// narrator line of the batch reads them.
    batch_rows: Vec<Node>,
    /// What every call of the saga that is written now reads: the rows of its chapter.
    round_read: Vec<Node>,
    /// The calls of the saga that is written now. A later call of the round reads them.
    round_calls: Vec<u64>,
    /// The chapters of the active character that a summary was asked for in this run. The
    /// rows tell which summary waits, so a restart never loses one.
    summary_asked: BTreeSet<EventId>,
    quest_request: Option<QuestRequest>,
    /// The newest time of an input from the addon. An emote or a book changes no world, so
    /// the tick of the world can be much older.
    newest: Tick,
    /// A line of Timeways for the next answer of any kind: an offer that came too late for
    /// its batch, or why a task was refused.
    notice: Option<String>,
    /// A line of the program itself, such as "Lore is ready.". A change of character keeps
    /// it, and it waits while a line of the character takes the slot.
    program_notice: Option<String>,
    /// After this time, the bridge drops the `events_seen` of a batch.
    events_deadline: Duration,
    /// The main part of the shape of the narrator line that the answer just accepted. The
    /// end of its call keeps it, for the window of the rotation.
    told_shape: Option<String>,
    dev_mode: DevMode,
}

impl Story {
    #[must_use]
    pub fn new(pack: Pack, store: Store) -> Story {
        Story {
            pack,
            store,
            active: None,
            calls: BTreeMap::new(),
            open: BTreeSet::new(),
            queued: VecDeque::new(),
            notes: Vec::new(),
            next_call: CallId(1),
            journal: Vec::new(),
            moments: Vec::new(),
            candidates: Vec::new(),
            budget: Budget::default(),
            shared: None,
            chronicle_asked: BTreeSet::new(),
            saga_round: None,
            tale_asked: BTreeSet::new(),
            history_asked: BTreeSet::new(),
            pace: Pace::default(),
            batch_rows: Vec::new(),
            round_read: Vec::new(),
            round_calls: Vec::new(),
            summary_asked: BTreeSet::new(),
            quest_request: None,
            newest: Tick(0),
            notice: None,
            program_notice: None,
            events_deadline: EVENTS_DEADLINE,
            told_shape: None,
            dev_mode: DevMode::Off,
        }
    }

    pub fn set_dev_mode(&mut self, mode: DevMode) {
        self.dev_mode = mode;
    }

    #[must_use]
    pub fn dev_mode(&self) -> DevMode {
        self.dev_mode
    }

    /// A line of the program for the next answer of any kind, such as "Lore is ready.".
    pub fn set_program_notice(&mut self, line: String) {
        self.program_notice = Some(line);
    }

    /// Tests set a short deadline to play a slow model.
    pub fn set_events_deadline(&mut self, deadline: Duration) {
        self.events_deadline = deadline;
    }

    /// # Errors
    ///
    /// Returns the refusal of the world for a game event, the error of the pack for a
    /// question, `UnknownCall` for the answer to a call that is not open, `NoCharacter`
    /// before the first `character_entered`, and the error of the store.
    pub fn handle(&mut self, input: Input) -> Result<Vec<Output>, StoryError> {
        let before = self.calls_before();
        let handled = self.handle_and_save(input);
        if handled.is_err() {
            self.take_back_calls(before);
        }
        handled
    }

    fn handle_and_save(&mut self, mut input: Input) -> Result<Vec<Output>, StoryError> {
        let mut time = None;
        if let Some(at) = input.at_mut() {
            *at = self.checked_time(*at)?;
            self.newest = self.newest.max(*at);
            time = Some(*at);
        }
        let kept = Kept::of(&input)?;
        let outputs = self.handle_unsaved(input, time);
        self.save_active(kept)?;
        self.save_shared();
        outputs
    }

    /// The world of the line is on the disk already, so a failed save of a shared value
    /// is only a note. The next line tries it again.
    fn save_shared(&mut self) {
        let Some(shared) = self.shared.as_mut() else {
            return;
        };
        let saved = shared
            .save(BUDGET, &self.budget)
            .and_then(|()| shared.save(PACE, &self.pace));
        if let Err(error) = saved {
            self.notes.push(format!("shared values not saved: {error}"));
        }
    }

    /// A shared file that does not open or read never locks a character out: the budget
    /// and the pace start from their defaults, and stay in memory in this run (GAMEPLAY.md
    /// 5.7).
    fn open_shared(&mut self) -> Result<(), StoryError> {
        if self.shared.is_some() {
            return Ok(());
        }
        let shared = match self.load_shared() {
            Ok(shared) => shared,
            Err(error) => {
                let note = format!("{error}: the budget and the pace stay in memory in this run");
                self.notes.push(note);
                self.budget = Budget::default();
                self.pace = Pace::default();
                Shared::in_memory()?
            }
        };
        self.shared = Some(shared);
        Ok(())
    }

    fn load_shared(&mut self) -> Result<Shared, StoreError> {
        let mut shared = self.store.open_shared()?;
        self.budget = shared.load::<Budget>(BUDGET)?.on_load();
        self.pace = shared.load(PACE)?;
        Ok(shared)
    }

    /// The rows of a refused input stay too, because the events before a refusal landed.
    /// A failed save loses the rows of its line, so the character comes back from the
    /// disk, and the memory holds what the disk holds.
    fn save_active(&mut self, kept: Option<Kept>) -> Result<(), StoryError> {
        let Some(active) = &mut self.active else {
            return Ok(());
        };
        let Err(error) = active.save(kept) else {
            return Ok(());
        };
        let key = active.key.clone();
        self.forget_run_state();
        self.active = self.opened(key).ok();
        Err(error.into())
    }

    /// After the line changed the world, the quests move once, because any line with a time
    /// can do a step (docs/plans/quest-variety.md 6.6).
    fn handle_unsaved(
        &mut self,
        input: Input,
        at: Option<Tick>,
    ) -> Result<Vec<Output>, StoryError> {
        let question = input.is_question();
        let ends_a_call = matches!(
            input,
            Input::ModelAnswered { .. } | Input::ModelFailed { .. }
        );
        let met = quests::encounter(&input);
        let hour = input.hour();
        let mut outputs = self.dispatch(input)?;
        if let Some(active) = self.active.as_mut() {
            active.catch_up();
        }
        if let Some(at) = at {
            self.advance_quests(at, hour, &met)?;
        }
        // A question ends its batch, so a `/quest` of the batch gets no `batch_end`.
        if question && let Some(request) = self.quest_request.take() {
            outputs.extend(self.quest_call(None, request));
        }
        if ends_a_call {
            outputs.extend(self.send_queued());
        }
        self.attach_notice(&mut outputs);
        Ok(outputs)
    }

    #[allow(clippy::too_many_lines, reason = "one arm for each kind of line")]
    fn dispatch(&mut self, input: Input) -> Result<Vec<Output>, StoryError> {
        match input {
            Input::Hello => Ok(vec![Output::Hello { protocol: PROTOCOL }]),
            Input::CharacterEntered { realm, name } => {
                self.enter_character(&realm, &name)?;
                Ok(Vec::new())
            }
            Input::CharacterDescribed { at, race, class } => self.describe(at, race, class),
            Input::ZoneEntered {
                at,
                zone,
                subzone,
                spot,
                hour,
                taxi: Some(Taxi::Yes),
            } => self.fly_through(at, &zone, subzone.as_deref(), spot, hour),
            Input::ZoneEntered {
                at,
                zone,
                subzone,
                spot,
                hour,
                taxi: None,
            } => self.enter_zone(at, &zone, subzone.as_deref(), spot, hour),
            Input::BgWon { at, zone } => self.win_battleground(at, &zone),
            Input::PvpRank { at, rank } => {
                self.change(|character| character.reach_pvp_rank(at, rank))
            }
            Input::RestChanged { at, resting } => {
                self.change(|character| character.rest(at, resting))
            }
            // An hour is no fact of the world. It only moves a step (`advance_quests`).
            Input::HourChanged { hour, .. } => checked_hour(Some(hour)).map(|()| Vec::new()),
            Input::InstanceEntered { at, zone, kind } => self.mark_instance(at, &zone, kind),
            Input::NpcMet { at, name, spot } => self.meet_npc(at, &name, spot),
            Input::NpcSeen {
                at,
                name,
                reaction,
                creature,
            } => self.see_npc(at, &name, reaction, creature.as_deref()),
            Input::NpcKilled { at, name } => self.kill_npc(at, &name),
            Input::NpcDefeated { at, name, kind } => self.defeat_npc(at, &name, kind),
            Input::GameQuestAccepted { at, title, kind } => self.take_game_quest(at, &title, kind),
            Input::GameQuestDone { at, title, kind } => self.finish_game_quest(at, &title, kind),
            Input::QuestMarked { at, quest, mark } => self.take_quest_mark(at, &quest, &mark),
            Input::MountRidden { at, mount, speed } => self.ride_mount(at, &mount, speed),
            Input::ItemEquipped {
                at,
                slot,
                item,
                quality,
                level,
                replaced,
                was,
            } => {
                let before = if was == SlotWas::Empty {
                    Before::Empty
                } else {
                    Before::Worn { level: replaced }
                };
                self.put_on(at, slot, &item, quality, level, before)
            }
            Input::NpcSlapped { at, name } => self.slap_npc(at, &name),
            Input::Died {
                at,
                killer,
                cause,
                killer_level,
                hour,
            } => self.die(at, killer, cause, killer_level, hour),
            Input::HeroSet { at, field, text } => self.set_hero_field(at, field, &text),
            Input::HeroAdded { at, text, npc } => {
                npc.as_deref().map(checked_name).transpose()?;
                self.add_hero_entry(at, &text, npc)
            }
            Input::HeroRemoved { at, number } => {
                self.hero_change(Change::Removed { at, number })?;
                Ok(Vec::new())
            }
            Input::EmoteDone {
                at,
                emote,
                target,
                hour,
            } => {
                checked_token(&emote)?;
                target.as_deref().map(checked_name).transpose()?;
                checked_hour(hour)?;
                self.record_flavor(at, hour, Kind::Emote { emote, target })
            }
            // A count is no fact of the world. It only moves a step (`advance_quests`).
            Input::ItemsHeld { npc, item, .. } => checked_name(&item)
                .and(checked_name(&npc))
                .map(|_| Vec::new()),
            Input::LevelReached { at, level } => {
                self.change(|character| character.reach_level(at, level))
            }
            Input::TextSeen {
                at,
                kind,
                title,
                npc,
                zone,
                text,
            } => {
                let seen = checked_seen(SeenText {
                    kind,
                    title,
                    npc,
                    zone,
                    text,
                })?;
                self.add_seen(at, seen)
            }
            Input::LoreAsked {
                id,
                question,
                target,
            } => self.ask(id, &question, target.as_deref()),
            Input::TalkAsked { id, at, npc, text } => self.talk(id, at, &npc, &text),
            Input::DraftAsked { id, idea, .. } => self.ask_draft(id, &idea),
            Input::QuestAsked { at, npc } => self.ask_quest(at, npc),
            Input::QuestAccepted { at, number } => self.answer_quest(at, Status::Accepted, number),
            Input::QuestDeclined { at, number } => self.answer_quest(at, Status::Declined, number),
            Input::QuestAbandoned { at, number } => self.abandon_quest(at, number),
            Input::JournalAsked { id, page } => self.journal_answer(id, page),
            Input::DevFps(run) => self.keep_fps_run(&run),
            Input::PastRead(past) => self.read_past(past),
            Input::StoryAccepted {
                at,
                number,
                title,
                paragraphs,
            } => self.accept_story(at, number, title.as_deref(), &paragraphs),
            Input::PlayerDescribed {
                name, race, class, ..
            } => self.describe_player(&AliasRow { name, race, class }),
            Input::StoryRemoved { at, number } => self.remove_story(at, number),
            Input::EntryEdited {
                at,
                entry,
                title,
                text,
                paragraphs,
            } => self.edit_entry(at, entry, title.as_deref(), text, &paragraphs),
            Input::LineRated {
                at,
                rated,
                first,
                rating,
            } => self.rate(at, rated, first, rating),
            Input::BatchEnd { id } => Ok(self.end_batch(id)),
            Input::ModelAnswered { call, text } => self.answered(call, &text),
            Input::ModelFailed { call } => self.failed(call),
        }
    }

    fn describe(&mut self, at: Tick, race: Race, class: Class) -> Result<Vec<Output>, StoryError> {
        self.change(|character| character.describe(at, race, class))
    }

    fn enter_zone(
        &mut self,
        at: Tick,
        zone: &str,
        subzone: Option<&str>,
        spot: Option<Spot>,
        hour: Option<u8>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_hour(hour)?;
        checked_name(zone)?;
        subzone.map(checked_name).transpose()?;
        self.change(|character| {
            character.enter_zone(at, zone, subzone)?;
            spot.map_or(Ok(()), |spot| character.mark_here(at, spot))
        })
    }

    fn meet_npc(
        &mut self,
        at: Tick,
        name: &str,
        spot: Option<Spot>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(name)?;
        self.change(|character| {
            character.meet_npc(at, name)?;
            character.befriend(at, name)?;
            spot.map_or(Ok(()), |spot| character.mark_npc(at, name, spot))
        })
    }

    /// A world boss gets its mark before the kill, so the kill weighs as a raid boss.
    fn defeat_npc(
        &mut self,
        at: Tick,
        name: &str,
        kind: Option<FoeKind>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(name)?;
        self.change(|character| {
            if kind == Some(FoeKind::WorldBoss) {
                character.mark_world_boss(at, name)?;
            }
            character.defeat_npc(at, name)
        })
    }

    /// A common kill moves the kill steps of quests. It is a defeat only when a tag of the
    /// pack names the foe, such as Mor'Ladim: the addon sends no defeat of a foe that is
    /// neither rare nor a boss, and its outcome would never unlock.
    fn kill_npc(&mut self, at: Tick, name: &str) -> Result<Vec<Output>, StoryError> {
        self.count_kill(at, checked_name(name)?)?;
        if !self.pack.tags_foe(game_names::wiki_name(name))? {
            return Ok(Vec::new());
        }
        self.defeat_npc(at, name, None)
    }

    fn win_battleground(&mut self, at: Tick, zone: &str) -> Result<Vec<Output>, StoryError> {
        checked_name(zone)?;
        self.change(|character| character.win_battleground(at, zone))
    }

    /// A flight path: you stand in the place, and visited nothing.
    fn fly_through(
        &mut self,
        at: Tick,
        zone: &str,
        subzone: Option<&str>,
        spot: Option<Spot>,
        hour: Option<u8>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_hour(hour)?;
        checked_name(zone)?;
        subzone.map(checked_name).transpose()?;
        self.change(|character| {
            character.fly_through(at, zone, subzone)?;
            spot.map_or(Ok(()), |spot| character.mark_here(at, spot))
        })
    }

    fn see_npc(
        &mut self,
        at: Tick,
        name: &str,
        reaction: Reaction,
        creature: Option<&str>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(name)?;
        creature.map(checked_token).transpose()?;
        self.change(|character| character.see_npc(at, name, reaction, creature))
    }

    fn mark_instance(
        &mut self,
        at: Tick,
        zone: &str,
        kind: InstanceKind,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(zone)?;
        self.change(|character| character.mark_instance(at, zone, kind))
    }

    fn take_quest_mark(
        &mut self,
        at: Tick,
        quest: &str,
        mark: &str,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(quest)?;
        checked_name(mark)?;
        self.change(|character| character.take_quest_mark(at, quest, mark))
    }

    fn ride_mount(
        &mut self,
        at: Tick,
        mount: &str,
        speed: Option<u16>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(mount)?;
        self.change(|character| character.ride_mount(at, mount, mounts::is_epic(speed)))
    }

    fn put_on(
        &mut self,
        at: Tick,
        slot: u8,
        name: &str,
        quality: u8,
        level: Option<u16>,
        before: Before,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(name)?;
        let quality = Quality::of_number(quality).ok_or(StoryError::BadItem)?;
        if !gear::SLOTS.contains(&slot) {
            return Err(StoryError::BadItem);
        }
        let upgrade = gear::is_big_upgrade(quality, level, before);
        let item = Item {
            name,
            slot,
            quality,
        };
        self.change(|character| character.put_on(at, &item, upgrade))
    }

    /// A slap is a meeting too, and it can earn a title (5.4.1).
    fn slap_npc(&mut self, at: Tick, name: &str) -> Result<Vec<Output>, StoryError> {
        checked_name(name)?;
        self.change(|character| character.slap(at, name))?;
        self.award_titles(at)
    }

    fn take_game_quest(
        &mut self,
        at: Tick,
        title: &str,
        kind: GameQuestKind,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(title)?;
        self.change(|character| character.take_game_quest(at, title, kind))
    }

    fn finish_game_quest(
        &mut self,
        at: Tick,
        title: &str,
        kind: GameQuestKind,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(title)?;
        self.change(|character| character.finish_game_quest(at, title, kind))
    }

    /// The addon and this program share the clock of one computer. A time far ahead comes
    /// from a clock that jumped, and it freezes the world, because Hourglass refuses
    /// every event older than its last one. A time before the last event, from a clock that
    /// went back, counts as the time of the last event.
    fn checked_time(&self, at: Tick) -> Result<Tick, StoryError> {
        if at.0 > now().0.saturating_add(DAY_SECONDS) {
            return Err(StoryError::FutureTime(at.0));
        }
        let last = self
            .active
            .as_ref()
            .map_or(Tick(0), |active| active.character.world().tick);
        Ok(at.max(last))
    }

    /// A silly death is also a flavor moment (GAMEPLAY.md 5.4.1).
    fn die(
        &mut self,
        at: Tick,
        killer: Option<String>,
        cause: Option<String>,
        killer_level: Option<u8>,
        hour: Option<u8>,
    ) -> Result<Vec<Output>, StoryError> {
        killer.as_deref().map(checked_name).transpose()?;
        cause.as_deref().map(checked_token).transpose()?;
        checked_hour(hour)?;
        let level = self.character()?.level();
        self.change(|character| character.die(at, killer.as_deref()))?;
        match silly_death(killer, cause, killer_level, level) {
            Some(kind) => self.record_flavor(at, hour, kind),
            None => Ok(Vec::new()),
        }
    }

    /// The hero of the active character in the player's own words, or nothing when `key`
    /// names another character.
    fn player_text(&self, key: &CharacterKey) -> String {
        let active = self.active.as_ref().filter(|active| &active.key == key);
        active.map_or_else(String::new, |active| {
            hero::player_text(&hero::hero(active.hero.changes()))
        })
    }

    /// The lines for the log since the last call, oldest first.
    pub fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notes)
    }

    /// A text that the character saw before changes nothing, also on the disk. A new book
    /// is a small moment for the narrator (GAMEPLAY.md 3.1.1). The game shows a book one
    /// page at a time, so only its first page is a new book.
    fn add_seen(&mut self, at: Tick, text: SeenText) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        if active.seen_index.contains(&text) {
            return Ok(Vec::new());
        }
        let book = match (text.kind, &text.title) {
            (TextKind::Book, Some(title)) => Some(title.clone()),
            _ => None,
        };
        let read_before = |title: &String| {
            let moments = active.flavor.moments();
            moments
                .iter()
                .any(|moment| matches!(&moment.kind, Kind::Read { title: read } if read == title))
        };
        let book = book.filter(|title| !read_before(title));
        let read = Read {
            at,
            text: text.clone(),
        };
        active.learned.add_read(read)?;
        active.seen_index.add(text)?;
        match book {
            Some(title) => self.record_flavor(at, None, Kind::Read { title }),
            None => Ok(Vec::new()),
        }
    }

    /// The same character again changes nothing, so each batch can name it.
    /// A refused switch leaves no character active, so the events of the new character
    /// never land in the world of the old one.
    fn enter_character(&mut self, realm: &str, name: &str) -> Result<(), StoryError> {
        self.open_shared()?;
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
        self.forget_run_state();
        self.active = Some(self.opened(key?)?);
        Ok(())
    }

    /// The run state points to rows and calls of the world in memory. After a change of
    /// character or a reopen, those rows are gone or belong to another world.
    fn forget_run_state(&mut self) {
        self.journal.clear();
        self.moments.clear();
        self.batch_rows.clear();
        self.candidates.clear();
        self.chronicle_asked.clear();
        self.tale_asked.clear();
        self.history_asked.clear();
        self.saga_round = None;
        self.round_read.clear();
        self.round_calls.clear();
        self.summary_asked.clear();
        self.quest_request = None;
        self.notice = None;
    }

    fn opened(&self, key: CharacterKey) -> Result<Active, StoryError> {
        let Opened {
            character,
            database,
            saved_events,
            next,
            prose,
            flavor,
            hero,
            learned,
            quests,
            stories,
            aliases,
            summaries,
            mut rules,
            tales,
            zone_histories,
            entry_edits,
            ratings,
            past,
        } = self.store.open(&key)?;
        let read: Vec<SeenText> = learned
            .read()
            .iter()
            .map(|read| read.text.clone())
            .collect();
        let seen_index = SeenIndex::new(&read)?;
        let history = character.world().history();
        let next_event = EventId(history.len() as u64);
        if let Some(row) = chapters::new_epoch(rules.rows(), next_event, chapters::CURRENT_RULE) {
            rules.add(row)?;
        }
        let mut book = Book::new(character.you(), rules.rows().to_vec());
        book.catch_up(history);
        Ok(Active {
            key,
            character,
            database,
            saved_events,
            next,
            new_calls: Vec::new(),
            ended: Vec::new(),
            answering: None,
            prose,
            flavor,
            hero,
            learned,
            quests,
            stories,
            aliases,
            summaries,
            rules,
            tales,
            zone_histories,
            entry_edits,
            ratings,
            past,
            edit_refused: None,
            book,
            seen_index,
            hero_refused: None,
            talk_quest: None,
            shown_line: None,
        })
    }

    fn character(&self) -> Result<&Character, StoryError> {
        self.active
            .as_ref()
            .map(|active| &active.character)
            .ok_or(StoryError::NoCharacter)
    }

    /// The events that landed before a refusal stay, so their moments count.
    fn change(
        &mut self,
        act: impl FnOnce(&mut Character) -> Result<(), Refusal>,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let before = active.character.world().history().len();
        let changed = act(&mut active.character);
        let world = active.character.world();
        let added: Vec<_> = world.history().iter().skip(before).cloned().collect();
        active.catch_up();
        let world = active.character.world();
        self.batch_rows.extend(
            added
                .iter()
                .map(|event| Node::Row(Table::Events, event.id.0)),
        );
        self.moments
            .extend(moments(world, active.character.you(), &added));
        changed.map_err(StoryError::Refused)?;
        Ok(Vec::new())
    }

    /// An empty text clears the field. A text that breaks a rule waits as the reason for the
    /// next journal page, because an edit has no reply of its own.
    fn set_hero_field(
        &mut self,
        at: Tick,
        field: String,
        text: &str,
    ) -> Result<Vec<Output>, StoryError> {
        let limit = hero::limit_of(&field).ok_or(StoryError::BadName)?;
        let text = if text.trim().is_empty() {
            String::new()
        } else {
            match hero::checked_text(text, limit) {
                Ok(text) => text,
                Err(reason) => return self.refuse_hero(reason),
            }
        };
        self.hero_change(Change::Set { at, field, text })?;
        Ok(Vec::new())
    }

    /// The entry keeps the place where the hero stands now.
    fn add_hero_entry(
        &mut self,
        at: Tick,
        text: &str,
        npc: Option<String>,
    ) -> Result<Vec<Output>, StoryError> {
        let text = match hero::checked_text(text, hero::LONG) {
            Ok(text) => text,
            Err(reason) => return self.refuse_hero(reason),
        };
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let number = hero::next_number(active.hero.changes());
        let place = active
            .character
            .place_names()
            .first()
            .map(|place| (*place).to_string());
        self.hero_change(Change::Added(Entry {
            number,
            at,
            text,
            place,
            npc,
        }))?;
        Ok(Vec::new())
    }

    fn hero_change(&mut self, change: Change) -> Result<(), StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        active.hero.add(change)?;
        active.hero_refused = None;
        Ok(())
    }

    fn refuse_hero(&mut self, reason: String) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        active.hero_refused = Some(reason);
        Ok(Vec::new())
    }

    /// A flavor moment where you stand now, scored against the moments before it, and then
    /// the titles that it earns.
    fn record_flavor(
        &mut self,
        at: Tick,
        hour: Option<u8>,
        kind: Kind,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let places = active.character.place_names();
        let moment = Flavor {
            at,
            hour,
            place: places.first().map(|place| (*place).to_string()),
            zone: places.get(1).map(|zone| (*zone).to_string()),
            kind,
        };
        let earlier = active.flavor.moments();
        let score = flavor::score(&moment, earlier, active.flavor.told(), &active.character);
        let key = moment.kind.key();
        let count = earlier.iter().filter(|old| old.kind.key() == key).count() + 1;
        active.flavor.add_moment(moment.clone())?;
        let row = active.flavor.moments_with_rows().last().map(|(row, _)| row);
        self.batch_rows
            .extend(row.map(|row| Node::Row(Table::Flavor, row)));
        self.candidates.push(Candidate {
            flavor: moment,
            score,
            count,
        });
        self.award_titles(at)
    }

    /// Each title whose rule now holds and that you do not hold yet lands in the world.
    fn award_titles(&mut self, at: Tick) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let new: Vec<&str> = titles::earned(active.flavor.moments(), &active.character)
            .into_iter()
            .filter(|title| !active.character.has_title(title))
            .collect();
        for title in new {
            self.change(|character| character.earn_title(at, title))?;
        }
        Ok(Vec::new())
    }

    /// At most one narrator line for a batch: about its best moment, within the budget.
    /// A batch can also start the saga of a finished chapter.
    /// A notice that waits takes the place of the narrator line.
    fn end_batch(&mut self, batch: MessageId) -> Vec<Output> {
        let ended = EventsBatch {
            id: batch,
            ended: Instant::now(),
        };
        // The prologue goes first, once in a life, so the narrator of the batch still finds a
        // free slot.
        let prologue = self.prologue_call();
        let mut outputs = match self.quest_request.take() {
            Some(request) => self.quest_call(Some(ended), request),
            None if self.notice.is_some() => vec![quests::quiet(batch)],
            None => vec![self.narrator_call(batch)],
        };
        outputs.extend(prologue);
        outputs.extend(self.saga_call());
        outputs.extend(self.tale_call());
        outputs.extend(self.summary_call());
        outputs.extend(self.zone_history_call());
        self.batch_rows.clear();
        outputs
    }

    /// The narrator never waits for a slot, and it stays quiet while a saga is written, so
    /// the player keeps a slot.
    fn narrator_call(&mut self, batch: MessageId) -> Output {
        let quiet = quests::quiet(batch);
        let chronicle_writes = self
            .calls
            .values()
            .any(|call| matches!(call.pending, Pending::Chronicle { .. }));
        if chronicle_writes || !self.has_free_slot() {
            return quiet;
        }
        let now = self.newest;
        let candidates = std::mem::take(&mut self.candidates);
        let (moment, telling) = match best(std::mem::take(&mut self.moments)) {
            Some(moment) => (moment, None),
            None => match self.flavor_line(candidates, now) {
                Some((moment, told)) => (moment, Some(told)),
                None => return quiet,
            },
        };
        let Some((prompt, call)) = self.narration(batch, &moment) else {
            return quiet;
        };
        if !self.budget.take(now) {
            return quiet;
        }
        if let Some(told) = telling {
            let Some(active) = self.active.as_mut() else {
                return quiet;
            };
            // A telling that is not on the disk comes again after a restart, so the line waits.
            if active.flavor.add_told(told).is_err() {
                return quiet;
            }
        }
        let reads = call.reads.clone();
        self.open_call(Pending::Narrator(Box::new(call)), prompt, reads)
            .unwrap_or(quiet)
    }

    /// The best flavor moment of the batch, when it scores enough, no flavor line came in
    /// the last 20 minutes, and its kind was not told this evening. The telling counts from
    /// the call, whatever the model answers.
    fn flavor_line(&self, candidates: Vec<Candidate>, now: Tick) -> Option<(Moment, Told)> {
        let mut best: Option<Candidate> = None;
        for candidate in candidates
            .into_iter()
            .filter(|candidate| candidate.score >= FLAVOR_MIN_SCORE)
        {
            if best
                .as_ref()
                .is_none_or(|kept| candidate.score >= kept.score)
            {
                best = Some(candidate);
            }
        }
        let best = best?;
        let key = best.flavor.kind.key();
        let told = self.active.as_ref()?.flavor.told();
        let since = |telling: &Told| now.0.saturating_sub(telling.at.0);
        let recent_line = told.iter().any(|telling| {
            telling.teller == Teller::Narrator && since(telling) < FLAVOR_GAP_SECONDS
        });
        let kind_told = told
            .iter()
            .any(|telling| telling.key == key && since(telling) < KIND_COOLDOWN_SECONDS);
        if recent_line || kind_told {
            return None;
        }
        let what = flavor::describe(&best.flavor, best.count);
        let book = best.flavor.kind.book().map(str::to_string);
        let told = Told {
            key,
            at: now,
            teller: Teller::Narrator,
        };
        Some((Moment::Flavor { what, book }, told))
    }

    /// A run with no data folder goes nowhere: that is a run by hand.
    fn keep_fps_run(&self, run: &FpsRun) -> Result<Vec<Output>, StoryError> {
        if !self.dev_mode.is_on() {
            return Err(StoryError::DevModeOff);
        }
        if !run.is_sane() {
            return Err(StoryError::BadFpsRun);
        }
        if let Store::Folder(folder) = &self.store {
            dev_fps::keep_run(folder, run).map_err(StoryError::FpsFile)?;
        }
        Ok(Vec::new())
    }

    fn journal_answer(&mut self, id: MessageId, page: usize) -> Result<Vec<Output>, StoryError> {
        let page = self.journal_page(page)?;
        Ok(vec![Output::Journal {
            id,
            page: Box::new(page),
            notice: None,
            dev: self.dev_mode.is_on().then_some(DevOn),
            past: self.past_wanted(),
        }])
    }

    /// A page past the end gets the last page, because the bridge refuses a page number
    /// that is not below the count. The addon drops a page that it did not ask for.
    fn journal_page(&mut self, page: usize) -> Result<Page, StoryError> {
        if page == 0 || self.journal.is_empty() {
            let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
            let mut journal = active.journal();
            journal
                .chapters
                .splice(0..0, prologues::journal_prologue(active));
            for chapter in &mut journal.chapters {
                let first = EventId(chapter.first);
                let rows: Vec<u64> = active.prose.row_of(first).into_iter().collect();
                let shown = edits::shown_of(active, edits::chapter_key(chapter.first), &rows);
                let written = active.prose.get(first).cloned().unwrap_or_default();
                let prose = shown
                    .narrator
                    .map(|_| aliases::with_names(active, &written.text));
                chapter.prose = prose.filter(|text| !text.is_empty());
                chapter.footnotes = written
                    .footnotes
                    .iter()
                    .map(|note| aliases::with_names(active, note))
                    .collect();
            }
            journal.edits = edits::journal_edits(active, &journal);
            journal.edit_refused = active.edit_refused.take().map(String::into_boxed_str);
            for person in &mut journal.people {
                person.trust_why = why::trust_why(active, &person.name)?;
            }
            journal.stories = stories::journal_stories(active)?;
            journal.summary = summaries::journal_summary(active);
            for tale in &mut journal.tales {
                tale.text = tales::shown_text(active, EventId(tale.first));
            }
            journal.histories = zone_histories::journal_histories(active);
            journal.lore = lore_of(
                &self.pack,
                &active.character,
                &journal.places,
                &journal.people,
            )?;
            journal.hero = hero::hero(active.hero.changes());
            journal.hero_refused = active.hero_refused.take().map(String::into_boxed_str);
            journal.talk_quest.clone_from(&active.talk_quest);
            journal.learned = learned(active.learned.read());
            journal.quests = quest_log(active.quests.changes())
                .into_iter()
                .filter(|quest| !matches!(quest.status, Status::Declined | Status::Abandoned))
                .map(|quest| QuestView::of(&quest))
                .collect();
            self.journal = pages(journal);
        }
        let last = self.journal.len().saturating_sub(1);
        Ok(self
            .journal
            .get(page.min(last))
            .cloned()
            .unwrap_or_default())
    }

    /// Talking is meeting, so the NPC enters the world before the model answers.
    fn talk(
        &mut self,
        id: MessageId,
        at: Tick,
        npc: &str,
        words: &str,
    ) -> Result<Vec<Output>, StoryError> {
        checked_name(npc)?;
        if npc.len() > talk::MAX_NPC_BYTES {
            return Err(StoryError::BadName);
        }
        checked_words(words)?;
        self.change(|character| character.meet_npc(at, npc))?;
        // The talk step is done before the prompt, so the NPC hears of the quest at once.
        self.advance_quests(at, None, &Encounter::Talk(npc.to_string()))?;
        let (prompt, read) = self.talk_prompt(at, npc, words)?;
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let pending = Pending::Talk {
            question: id,
            key: active.key.clone(),
            npc: npc.to_string(),
            at,
        };
        Ok(self.open_call(pending, prompt, read).into_iter().collect())
    }

    /// The prompt of a talk, and every row that it reads (GAMEPLAY.md 5.14).
    fn talk_prompt(
        &self,
        at: Tick,
        npc: &str,
        words: &str,
    ) -> Result<(String, Vec<Node>), StoryError> {
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let character = &active.character;
        let quest_log = quest_log(active.quests.changes());
        let (numbers, quests): (Vec<u64>, Vec<QuestTalk>) =
            quests::quest_talks(&quest_log, npc, at).into_iter().unzip();
        let mut passages =
            passages_for(&self.pack, &active.seen_index, character, words, Some(npc))?;
        passages.truncate(TALK_PASSAGES);
        let place = character.place_of(npc);
        let hero = hero::hero(active.hero.changes());
        let about = |entry: &Entry| {
            entry.npc.as_deref() == Some(npc)
                || (place.is_some() && entry.place.as_deref() == place)
        };
        let memories = remembered(active, npc, at);
        // A failed count gives no hook, and the talk still goes out.
        let (hook, hook_read) = calls::hook_for(active, &hero).unwrap_or_default();
        let mut read = reads::events_about(active, [npc]);
        read.extend(reads::entries_read(active, about));
        read.extend(reads::passages_read(active, &passages));
        read.extend(reads::memories_read(&memories));
        read.extend(reads::quest_rows(active, &numbers));
        read.extend(hook_read);
        let scene = Scene {
            npc,
            place,
            level: character.level(),
            trust: character.trust_of(npc),
            slapped: character.slaps_of(npc),
            own_lore: hero::newest_texts(&hero.entries, about),
            memories: memories
                .iter()
                .map(|memory| npc_memory::line(memory, at))
                .collect(),
            quests,
            hook,
        };
        let prompt = talk::prompt(&scene, &passages, words, self.turn());
        Ok((prompt, read))
    }

    /// With no passage, a model has nothing to cite, so no call goes out.
    fn ask(
        &mut self,
        id: MessageId,
        question: &str,
        target: Option<&str>,
    ) -> Result<Vec<Output>, StoryError> {
        checked_words(question)?;
        target.map(checked_name).transpose()?;
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let character = &active.character;
        let passages = passages_for(&self.pack, &active.seen_index, character, question, target)?;
        if passages.is_empty() {
            let answer = Answer {
                text: None,
                passages,
            };
            return Ok(vec![Output::LoreAnswer {
                id,
                answer,
                notice: None,
            }]);
        }
        let context = Context {
            places: character.place_names(),
            target,
            level: character.level(),
        };
        let call = LoreCall::new(question, &context, passages);
        Ok(self.follow(id, Next::Ask(call)).into_iter().collect())
    }

    fn follow(&mut self, question: MessageId, next: Next) -> Option<Output> {
        match next {
            Next::Done(answer) => Some(Output::LoreAnswer {
                id: question,
                answer,
                notice: None,
            }),
            Next::Ask(lore) => {
                let prompt = lore.prompt().to_string();
                self.open_call(Pending::Lore { question, lore }, prompt, Vec::new())
            }
        }
    }

    /// The notice that waits goes on the first answer with room for it. A full page of the
    /// journal has no room, so the notice waits for a later answer.
    fn attach_notice(&mut self, outputs: &mut [Output]) {
        let Some(line) = self.notice.as_ref().or(self.program_notice.as_ref()) else {
            return;
        };
        let Some(output) = outputs.iter_mut().find(|output| output.has_room_for(line)) else {
            return;
        };
        if let Some(slot) = output.notice_mut() {
            *slot = self.notice.take().or_else(|| self.program_notice.take());
        }
    }

    /// The line goes on the answer of its batch, or on the next answer when the bridge
    /// no longer waits for that batch.
    fn deliver(&mut self, batch: Option<EventsBatch>, line: String) -> Vec<Output> {
        match batch {
            Some(batch) if batch.ended.elapsed() < self.events_deadline => {
                vec![quests::notice_line(batch.id, line)]
            }
            _ => {
                self.notice = Some(line);
                Vec::new()
            }
        }
    }
}

/// What the NPC remembers of the active character, as of the talk at `now`.
fn remembered(active: &Active, npc: &str, now: Tick) -> Vec<Memory> {
    let past = Past {
        character: &active.character,
        rumors: active.learned.rumors_with_rows().collect(),
        quests: active.quests.changes(),
        own_name: active.key.name(),
    };
    npc_memory::memories(&past, npc, now)
}

/// The question and where you stand pick the passages. The spoiler limit then drops each
/// passage of the pack about something that your world does not hold, or that tells a deed
/// of adventurers that you did not do (`spoiler`). The text that you
/// read passes it, and comes first: the pack only fills the gaps (GAMEPLAY.md 3.1.1). The
/// rest stop at the size that leaves room for a model answer in one reply. Each passage
/// keeps the limits of the bridge, also from an old pack.
fn passages_for(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    question: &str,
    target: Option<&str>,
) -> Result<Vec<Passage>, StoryError> {
    let mut words = vec![question];
    words.extend(character.place_names());
    words.extend(target);
    let words = words.join(" ");
    let mut found = seen.search(&words, CANDIDATES)?;
    found.extend(
        pack.search(&words, CANDIDATES)?
            .into_iter()
            .filter(|passage| spoiler::may_show(character, passage)),
    );
    let mut passages = Vec::new();
    let mut used = Size::default();
    for passage in found.into_iter().filter_map(passage_limits::fitted) {
        used = used
            .plus(Size::of(&passage))
            .plus(Size { line: 1, slot: 1 });
        if passages.len() == ANSWER_SIZE || !used.fits(PASSAGES) {
            break;
        }
        passages.push(passage);
    }
    Ok(passages)
}

/// A seen text keeps its line breaks. Any other control character comes from a bug or a
/// hostile addon.
fn checked_seen(seen: SeenText) -> Result<SeenText, StoryError> {
    seen.title.as_deref().map(checked_name).transpose()?;
    seen.npc.as_deref().map(checked_name).transpose()?;
    seen.zone.as_deref().map(checked_name).transpose()?;
    let text = &seen.text;
    let control = text.chars().any(|c| c.is_control() && c != '\n');
    if text.trim().is_empty() || text.len() > MAX_SEEN_BYTES || control {
        return Err(StoryError::BadSeenText);
    }
    Ok(seen)
}

/// A death is silly when the world killed you, or an NPC far below your level.
fn silly_death(
    killer: Option<String>,
    cause: Option<String>,
    killer_level: Option<u8>,
    level: Option<i64>,
) -> Option<Kind> {
    if let Some(cause) = cause {
        return Some(Kind::FellTo { cause });
    }
    let killer = killer?;
    let gap = level? - i64::from(killer_level?);
    (gap >= HUMBLING_GAP).then_some(Kind::Humbled { killer, gap })
}

/// An emote or a cause of death from the game: 1 to 24 lowercase letters.
fn checked_token(token: &str) -> Result<&str, StoryError> {
    let letters = token.bytes().all(|byte| byte.is_ascii_lowercase());
    if token.is_empty() || token.len() > 24 || !letters {
        return Err(StoryError::BadToken);
    }
    Ok(token)
}

fn checked_hour(hour: Option<u8>) -> Result<(), StoryError> {
    if hour.is_some_and(|hour| hour > 23) {
        return Err(StoryError::BadHour);
    }
    Ok(())
}

fn checked_name(name: &str) -> Result<&str, StoryError> {
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control) {
        return Err(StoryError::BadName);
    }
    Ok(name)
}

/// Words that the player typed: one chat line.
fn checked_words(words: &str) -> Result<&str, StoryError> {
    if words.trim().is_empty()
        || words.len() > MAX_WORDS_BYTES
        || words.chars().any(char::is_control)
    {
        return Err(StoryError::BadWords);
    }
    Ok(words)
}

fn reasons(refusal: &Refusal) -> String {
    let reasons: Vec<String> = refusal.iter().map(ToString::to_string).collect();
    reasons.join("; ")
}
