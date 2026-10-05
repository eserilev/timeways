//! Side quests in the story program (GAMEPLAY.md 3.4): the offer at the end of a batch,
//! the answer of the player, and the progress from game events.

use super::{Active, EventsBatch, Pending, Story, StoryError, calls, checked_name, reads};
use crate::character::Character;
use crate::hero;
use crate::input::{Input, MessageId};
use crate::journal::{TalkQuest, TalkQuestState};
use crate::prompt::{self, Attempt};
use crate::quest::variety::{RECENT_IN_PROMPT, recent_quests};
use crate::quest::{
    self, Encounter, Here, Known, MAX_OPEN_QUESTS, QuestChange, QuestFault, Status, Step, Tracked,
    next_number, quest_log, thing_name,
};
use crate::seen::{SeenText, TextKind};
use crate::store::{CharacterKey, Node, Outcome};
use crate::story::Output;
use crate::talk::QuestTalk;
use hourglass::Tick;

/// A quest call: whose, from which giver, for which batch, and what it read. A retry
/// keeps all of it, so it keeps the batch and the reads of its first call.
pub(super) struct QuestCall {
    /// The offer is the notice of this batch, or of the next answer when none waits.
    pub(super) batch: Option<EventsBatch>,
    pub(super) key: CharacterKey,
    pub(super) giver: String,
    pub(super) at: Tick,
    pub(super) attempt: Attempt,
    pub(super) reads: Vec<Node>,
    pub(super) asker: Asker,
}

/// Who asked for the quest. The talk window shows the quest of a talk, so a talk gets no
/// notice (GAMEPLAY.md 3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Asker {
    Command,
    Talk,
}

/// Why an answer gives no offer.
enum NoOffer {
    /// A line of the code, such as a limit: no model can fix it.
    Line(String),
    /// The answer broke a rule of the check. A retry can fix it.
    Fault(QuestFault),
}

/// How a quest call ends: the offer by its number, or no quest. Each has its line.
enum Ending {
    Offered { number: u64, line: String },
    NoQuest { line: String },
}

/// A `/quest` of this batch. It waits for the end of the batch, because the offer comes
/// back as a notice of the batch answer.
pub(super) struct QuestRequest {
    giver: String,
    at: Tick,
}

/// The talk with work that asks for a quest: its words, and the row of its call, which
/// the quest call reads (GAMEPLAY.md 5.14).
pub(super) struct TalkWork {
    pub(super) npc: String,
    pub(super) at: Tick,
    pub(super) said: String,
    pub(super) call: Option<u64>,
}

impl Story {
    /// Asking is meeting, as a talk is. A hostile NPC or a beast is not met: it has
    /// nothing to say.
    pub(super) fn ask_quest(&mut self, at: Tick, npc: String) -> Result<Vec<Output>, StoryError> {
        checked_name(&npc)?;
        if !self.character()?.is_hostile_or_animal(&npc) {
            self.change(|character| character.meet_npc(at, &npc))?;
        }
        self.quest_request = Some(QuestRequest { giver: npc, at });
        Ok(Vec::new())
    }

    /// The offer takes the place of the narrator call, and comes back as a notice of
    /// `batch`. With no batch, the batch ended with a question, and the notice waits for the
    /// next answer. A request that breaks a rule gets a line of the code, and no model call.
    /// A giver gets one quest call at a time: the offer that comes answers both asks.
    pub(super) fn quest_call(
        &mut self,
        batch: Option<EventsBatch>,
        request: QuestRequest,
    ) -> Vec<Output> {
        let Some(active) = self.active.as_ref() else {
            return batch.map(|batch| quiet(batch.id)).into_iter().collect();
        };
        if self.writes_quest_of(&active.key, &request.giver) {
            return batch.map(|batch| quiet(batch.id)).into_iter().collect();
        }
        let quests = quest_log(active.quests.changes());
        if let Some(refusal) = refusal(&active.character, &quests, &request.giver) {
            return self.deliver(batch, refusal);
        }
        self.open_quest_call(batch, request, Asker::Command, None)
    }

    /// Work in a talk asks the NPC for a quest, as `/quest` does (GAMEPLAY.md 3.5). The
    /// talk window shows the quest, so the state goes into the journal, not a notice. The
    /// limits that refuse it now give their line at once, with no model call.
    pub(super) fn quest_from_talk(&mut self, work: TalkWork) -> Vec<Output> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        let quests = quest_log(active.quests.changes());
        let refusal = refusal(&active.character, &quests, &work.npc);
        let asks_model = refusal.is_none();
        let state = refusal.map_or(TalkQuestState::Writing, |line| TalkQuestState::Refused {
            line,
        });
        active.talk_quest = Some(Box::new(TalkQuest {
            npc: work.npc.clone(),
            at: work.at,
            state,
        }));
        let key = active.key.clone();
        if !asks_model || self.writes_quest_of(&key, &work.npc) {
            return Vec::new();
        }
        let request = QuestRequest {
            giver: work.npc,
            at: work.at,
        };
        let talk = (work.said, work.call);
        self.open_quest_call(None, request, Asker::Talk, Some(talk))
    }

    /// True while a quest call of this character for this giver runs or waits for a slot.
    fn writes_quest_of(&self, key: &CharacterKey, giver: &str) -> bool {
        self.calls.values().any(|call| {
            matches!(&call.pending, Pending::Quest(quest) if &quest.key == key && quest.giver == giver)
        })
    }

    /// `talk` holds the words of the giver and the row of the talk call, when a talk asked.
    fn open_quest_call(
        &mut self,
        batch: Option<EventsBatch>,
        request: QuestRequest,
        asker: Asker,
        talk: Option<(String, Option<u64>)>,
    ) -> Vec<Output> {
        let Some(active) = self.active.as_ref() else {
            return Vec::new();
        };
        let quests = quest_log(active.quests.changes());
        let giver = request.giver.as_str();
        let seen = seen_texts(active);
        let known = known(&active.character, giver, &seen, &quests);
        let hero = hero::hero(active.hero.changes());
        // A failed count gives no hook, and the offer still goes out.
        let (hook, hook_read) = calls::hook_for(active, &hero).unwrap_or_default();
        let said = talk.as_ref().map(|(said, _)| said.as_str());
        let prompt = quest::prompt(&known, active.character.place_of(giver), hook, said);
        let mut reads = reads::events_about(active, known_names(&known));
        reads.extend(hook_read);
        let recent: Vec<u64> = known.recent.iter().map(|recent| recent.number).collect();
        reads.extend(reads::quest_rows(active, &recent));
        reads.extend(reads::game_quests_read(active, &known.game_quests));
        reads.extend(reads::level_read(active));
        reads.extend(talk.and_then(|(_, call)| call).map(Node::Call));
        let pending = Pending::Quest(QuestCall {
            batch,
            key: active.key.clone(),
            giver: request.giver,
            at: request.at,
            attempt: Attempt::First,
            reads: reads.clone(),
            asker,
        });
        self.open_call(pending, prompt, reads).into_iter().collect()
    }

    /// An offer for another character, or one that breaks a rule, shows no task. The limits
    /// hold again here, because the world can move on while the model thinks. A first
    /// answer that breaks a rule of the check gets one more call with the reason, and the
    /// player sees nothing of it.
    pub(super) fn quest_answered(
        &mut self,
        row: Option<u64>,
        quest: QuestCall,
        prompt: &str,
        text: &str,
    ) -> (Vec<Output>, Outcome) {
        let no_offer = match self.offer(&quest.key, &quest.giver, quest.at, text) {
            Ok(offered) => return (self.end_quest_call(&quest, offered), Outcome::Accepted),
            Err(no_offer) => no_offer,
        };
        let line = match no_offer {
            NoOffer::Fault(fault) if quest.attempt == Attempt::First => {
                return (
                    self.retry_quest(row, quest, prompt, text, &fault),
                    Outcome::Refused,
                );
            }
            NoOffer::Fault(_) => no_task(&quest.giver),
            NoOffer::Line(line) => line,
        };
        let ending = Ending::NoQuest { line };
        (self.end_quest_call(&quest, ending), Outcome::Refused)
    }

    /// With no model, the giver has no quest for you now.
    pub(super) fn quest_failed(&mut self, quest: &QuestCall) -> Vec<Output> {
        let line = no_task(&quest.giver);
        self.end_quest_call(quest, Ending::NoQuest { line })
    }

    /// The quest of a talk goes into the journal. A `/quest` gets its notice too, also when
    /// a talk waits for the same giver.
    fn end_quest_call(&mut self, quest: &QuestCall, ending: Ending) -> Vec<Output> {
        let (state, line) = match ending {
            Ending::Offered { number, line } => (TalkQuestState::Offered { number }, line),
            Ending::NoQuest { line } => (TalkQuestState::Refused { line: line.clone() }, line),
        };
        self.settle_talk_quest(quest, state);
        match quest.asker {
            Asker::Command => self.deliver(quest.batch, line),
            Asker::Talk => Vec::new(),
        }
    }

    /// Only a talk quest of this character and giver that is still being written changes.
    fn settle_talk_quest(&mut self, quest: &QuestCall, state: TalkQuestState) {
        let Some(active) = self
            .active
            .as_mut()
            .filter(|active| active.key == quest.key)
        else {
            return;
        };
        let Some(talk_quest) = active.talk_quest.as_mut() else {
            return;
        };
        if talk_quest.npc == quest.giver && talk_quest.state == TalkQuestState::Writing {
            talk_quest.state = state;
        }
    }

    /// The second call: the first prompt, the first answer, and the reason. It reads what
    /// the first call read, and the first call (its row).
    fn retry_quest(
        &mut self,
        first_row: Option<u64>,
        quest: QuestCall,
        prompt: &str,
        text: &str,
        fault: &QuestFault,
    ) -> Vec<Output> {
        let mut reads = quest.reads;
        reads.extend(first_row.map(Node::Call));
        let retry = QuestCall {
            attempt: Attempt::Retry,
            reads: reads.clone(),
            ..quest
        };
        let prompt = prompt::retry(prompt, text, &[fault.to_string()]);
        self.open_call(Pending::Quest(retry), prompt, reads)
            .into_iter()
            .collect()
    }

    /// The offer with its line, or why there is none.
    fn offer(
        &mut self,
        key: &CharacterKey,
        giver: &str,
        asked_at: Tick,
        text: &str,
    ) -> Result<Ending, NoOffer> {
        let none = || NoOffer::Line(no_task(giver));
        let Some(active) = self.active.as_mut().filter(|active| &active.key == key) else {
            return Err(none());
        };
        let quests = quest_log(active.quests.changes());
        if let Some(refusal) = refusal(&active.character, &quests, giver) {
            return Err(NoOffer::Line(refusal));
        }
        let seen = seen_texts(active);
        let known = known(&active.character, giver, &seen, &quests);
        let offer = quest::checked_quest(text, &known).map_err(NoOffer::Fault)?;
        let at = asked_at.max(active.character.world().tick);
        let number = next_number(active.quests.changes());
        let line = quest::offer_line(giver, &offer);
        let change = QuestChange::Offered {
            number,
            at,
            giver: giver.to_string(),
            title: offer.title.clone(),
            text: offer.text,
            steps: offer.steps,
            genre: Some(offer.genre),
            any_order: offer.any_order,
        };
        if active.quests.add(change).is_err() {
            return Err(none());
        }
        // The log holds the offer, so a refusal of the world loses only the fact.
        let _ = self.change(|character| {
            character.offer_quest(at, giver, &thing_name(number, &offer.title))
        });
        Ok(Ending::Offered { number, line })
    }

    /// The answer names its offer by number. With no number, it takes the newest offer.
    /// Accepting holds the limits again, and can finish a step at once: you can stand in its
    /// place already.
    pub(super) fn answer_quest(
        &mut self,
        at: Tick,
        status: Status,
        number: Option<u64>,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let quests = quest_log(active.quests.changes());
        let answers = |quest: &&Tracked| {
            quest.status == Status::Offered && number.is_none_or(|n| quest.number == n)
        };
        let Some(offer) = quests.iter().rev().find(answers) else {
            return Ok(Vec::new());
        };
        let number = offer.number;
        if status == Status::Declined {
            active.quests.add(QuestChange::Declined { number, at })?;
            return Ok(Vec::new());
        }
        if let Some(refusal) = refusal(&active.character, &quests, &offer.giver) {
            self.notice = Some(refusal);
            return Ok(Vec::new());
        }
        active.quests.add(QuestChange::Accepted { number, at })?;
        let name = thing_name(number, &offer.title);
        self.change(|character| character.accept_quest(at, &name))
    }

    /// A number with no open quest changes nothing.
    pub(super) fn abandon_quest(
        &mut self,
        at: Tick,
        number: u64,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let quests = quest_log(active.quests.changes());
        let open = |quest: &Tracked| {
            quest.number == number && matches!(quest.status, Status::Offered | Status::Accepted)
        };
        if quests.iter().any(open) {
            active.quests.add(QuestChange::Abandoned { number, at })?;
        }
        Ok(Vec::new())
    }

    /// Each open step that holds now is done, one at a time, while steps hold. So one line
    /// can do a step and the step after it. The last step finishes the quest. With no
    /// character, no quest moves.
    pub(super) fn advance_quests(
        &mut self,
        at: Tick,
        hour: Option<u8>,
        met: &Encounter,
    ) -> Result<(), StoryError> {
        loop {
            let Some(active) = self.active.as_mut() else {
                return Ok(());
            };
            let character = &active.character;
            let here = Here {
                at,
                places: character.place_names(),
                hour,
                level: character.level(),
                game_quests_done: character.game_quests_done(),
            };
            let quests = quest_log(active.quests.changes());
            let Some((quest, step)) = holding_step(&quests, &here, met) else {
                return Ok(());
            };
            let number = quest.number;
            active
                .quests
                .add(QuestChange::StepDone { number, step, at })?;
            if quest.steps_done() + 1 == quest.steps.len() {
                let (giver, name) = (quest.giver.clone(), thing_name(number, &quest.title));
                self.change(|character| character.finish_quest(at, &giver, &name))?;
            }
        }
    }

    /// A kill counts for each accepted quest with an open kill step of this creature.
    pub(super) fn count_kill(&mut self, at: Tick, name: &str) -> Result<(), StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let quests = quest_log(active.quests.changes());
        for quest in &quests {
            if let Some(step) = quest.hunts(name) {
                let number = quest.number;
                active
                    .quests
                    .add(QuestChange::Killed { number, step, at })?;
            }
        }
        Ok(())
    }
}

/// The NPC of a talk step keeps the quest in mind for one conversation: the time after
/// which a conversation of `/talk` ends.
const QUEST_TALK_SECONDS: u64 = 10 * 60;

/// The quests whose talk step to this NPC was done in the last 10 minutes, with their
/// numbers.
pub(super) fn quest_talks<'q>(
    quests: &'q [Tracked],
    npc: &str,
    at: Tick,
) -> Vec<(u64, QuestTalk<'q>)> {
    let recent = |done: &Option<Tick>| {
        done.is_some_and(|done| at.0.saturating_sub(done.0) < QUEST_TALK_SECONDS)
    };
    let mut talks = Vec::new();
    for quest in quests {
        for (step, done) in quest.steps.iter().zip(&quest.done) {
            let Step::Talk { npc: wanted, about } = step else {
                continue;
            };
            if wanted == npc && recent(done) {
                let talk = QuestTalk {
                    giver: &quest.giver,
                    title: &quest.title,
                    about: about.as_deref(),
                };
                talks.push((quest.number, talk));
            }
        }
    }
    talks
}

/// What the line did, as far as a step can see it. Asking for a quest meets the NPC, but it
/// does no meet step.
pub(super) fn encounter(input: &Input) -> Encounter {
    match input {
        Input::NpcMet { name, .. } => Encounter::Gossip(name.clone()),
        Input::TalkAsked { npc, .. } => Encounter::Talk(npc.clone()),
        Input::NpcSlapped { name, .. } => Encounter::Slap(name.clone()),
        Input::NpcDefeated { name, .. } => Encounter::Defeat(name.clone()),
        Input::EmoteDone { emote, target, .. } => Encounter::Emote {
            emote: emote.clone(),
            target: target.clone(),
        },
        Input::ItemsHeld {
            npc, item, count, ..
        } => Encounter::Carry {
            npc: npc.clone(),
            item: item.clone(),
            count: *count,
        },
        _ => Encounter::None,
    }
}

/// The first open step that holds now, with its quest.
fn holding_step<'q>(
    quests: &'q [Tracked],
    here: &Here<'_>,
    met: &Encounter,
) -> Option<(&'q Tracked, usize)> {
    for quest in quests {
        let holds = |step: &usize| quest.step_holds(*step, here, met);
        if let Some(step) = quest.open_steps().into_iter().find(holds) {
            return Some((quest, step));
        }
    }
    None
}

/// A batch with nothing to say.
pub(super) fn quiet(batch: MessageId) -> Output {
    Output::EventsSeen {
        id: batch,
        narrator: None,
        notice: None,
    }
}

/// A line of Timeways, not of the narrator: an offer, or why there is none.
pub(super) fn notice_line(batch: MessageId, line: String) -> Output {
    Output::EventsSeen {
        id: batch,
        narrator: None,
        notice: Some(line),
    }
}

/// With no model, or with an offer that breaks a rule, the giver has nothing to say.
pub(super) fn no_task(giver: &str) -> String {
    format!("{giver} has no quest for you now.")
}

/// The reason in words when the giver cannot give you a quest now. A dead or hostile giver
/// and a beast have no quest.
fn refusal(character: &Character, quests: &[Tracked], giver: &str) -> Option<String> {
    if character.is_dead(giver) || character.is_hostile_or_animal(giver) {
        return Some(no_task(giver));
    }
    let open: Vec<&Tracked> = quests
        .iter()
        .filter(|quest| quest.status == Status::Accepted)
        .collect();
    if let Some(quest) = open.iter().find(|quest| quest.giver == giver) {
        return Some(format!(
            "{giver} is waiting for you to finish \"{}\".",
            quest.title
        ));
    }
    if open.len() >= MAX_OPEN_QUESTS {
        return Some(format!(
            "You already have {MAX_OPEN_QUESTS} quests. Finish one first."
        ));
    }
    None
}

fn seen_texts(active: &Active) -> Vec<SeenText> {
    active
        .learned
        .read()
        .iter()
        .map(|read| read.text.clone())
        .collect()
}

/// The zone of a text that you read is the zone where you read it, so the places that you
/// visited hold every place that you heard of. Each list holds the newest first, and the
/// targets in the zone of the giver before the rest, because the prompt shows only the
/// start of a list.
fn known<'a>(
    character: &'a Character,
    giver: &'a str,
    seen: &'a [SeenText],
    quests: &'a [Tracked],
) -> Known<'a> {
    let home = character.zone_of_npc(giver);
    let near_place = |place: &str| home.is_some() && character.zone_of_place(place) == home;
    let near_npc = |npc: &str| home.is_some() && character.zone_of_npc(npc) == home;
    Known {
        giver,
        zones: near_first(character.visited_zones(), near_place),
        subzones: near_first(character.visited_subzones(), near_place),
        npcs: near_first(character.npcs_to_meet(), near_npc),
        foes: near_first(character.foes_seen(), near_npc),
        last_targets: last_targets(quests),
        goods: quest::goods_for(character.level()),
        recent: recent_quests(quests, RECENT_IN_PROMPT),
        level: character.level(),
        dungeons: character.dungeons_entered(),
        bosses: character.bosses_defeated(),
        game_quests: game_quests_known(character, seen),
        game_quests_done: character.game_quests_done(),
        seen,
    }
}

/// The quests of the game that you hold, then the ones that you read, each once, and none
/// that you turned in.
fn game_quests_known<'a>(character: &'a Character, seen: &'a [SeenText]) -> Vec<&'a str> {
    let done = character.game_quests_done();
    let read = seen
        .iter()
        .filter(|text| text.kind == TextKind::Quest)
        .filter_map(|text| text.title.as_deref());
    let mut titles: Vec<&str> = Vec::new();
    for title in character.game_quests_open().into_iter().chain(read) {
        if !done.contains(&title) && !titles.contains(&title) {
            titles.push(title);
        }
    }
    titles
}

/// The giver, and every place, NPC, creature, dungeon, and boss that the prompt can offer.
fn known_names<'a>(known: &Known<'a>) -> Vec<&'a str> {
    let mut names = vec![known.giver];
    names.extend(&known.zones);
    names.extend(&known.subzones);
    names.extend(&known.npcs);
    names.extend(&known.foes);
    names.extend(&known.dungeons);
    names.extend(&known.bosses);
    names
}

/// The order stays the same inside each group.
fn near_first(mut names: Vec<&str>, near: impl Fn(&str) -> bool) -> Vec<&str> {
    names.sort_by_key(|name| !near(name));
    names
}

/// The targets of the newest task of the log, from any giver and in any state.
fn last_targets(quests: &[Tracked]) -> Vec<&str> {
    let newest = quests.iter().max_by_key(|quest| quest.number);
    newest.map_or_else(Vec::new, |quest| {
        quest.steps.iter().filter_map(quest::Step::target).collect()
    })
}
