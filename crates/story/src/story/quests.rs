//! Side quests in the story program (GAMEPLAY.md 3.4): the offer at the end of a batch,
//! the answer of the player, and the progress from game events.

use super::{Active, EventsBatch, Pending, Story, StoryError, calls, checked_name, reads};
use crate::character::Character;
use crate::hero;
use crate::input::{Input, MessageId};
use crate::quest::{
    self, Encounter, Here, Known, MAX_OPEN_QUESTS, QuestChange, Status, Step, Tracked, next_number,
    quest_log, thing_name,
};
use crate::seen::SeenText;
use crate::store::{CharacterKey, Outcome};
use crate::story::Output;
use crate::talk::QuestTalk;
use hourglass::Tick;

/// A `/quest` of this batch. It waits for the end of the batch, because the offer comes
/// back as a notice of the batch answer.
pub(super) struct QuestRequest {
    giver: String,
    at: Tick,
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
    pub(super) fn quest_call(
        &mut self,
        batch: Option<EventsBatch>,
        request: QuestRequest,
    ) -> Vec<Output> {
        let Some(active) = self.active.as_ref() else {
            return batch.map(|batch| quiet(batch.id)).into_iter().collect();
        };
        let quests = quest_log(active.quests.changes());
        let giver = request.giver.as_str();
        if active.character.is_dead(giver) || active.character.is_hostile_or_animal(giver) {
            let line = no_task(giver);
            return self.deliver(batch, line);
        }
        if let Some(refusal) = refusal(&quests, giver) {
            return self.deliver(batch, refusal);
        }
        let seen = seen_texts(active);
        let known = known(&active.character, giver, &seen, &quests);
        let hero = hero::hero(active.hero.changes());
        // A failed count gives no hook, and the offer still goes out.
        let (hook, hook_read) = calls::hook_for(active, &hero).unwrap_or_default();
        let prompt = quest::prompt(&known, active.character.place_of(giver), hook);
        let mut reads = reads::events_about(active, known_names(&known));
        reads.extend(hook_read);
        let pending = Pending::Quest {
            batch,
            key: active.key.clone(),
            giver: request.giver,
            at: request.at,
        };
        self.open_call(pending, prompt, reads).into_iter().collect()
    }

    /// An offer for another character, or one that breaks a rule, shows no task. The limits
    /// hold again here, because the world can move on while the model thinks.
    pub(super) fn quest_answered(
        &mut self,
        batch: Option<EventsBatch>,
        key: &CharacterKey,
        giver: &str,
        asked_at: Tick,
        text: &str,
    ) -> (Vec<Output>, Outcome) {
        let (line, outcome) = match self.offer(key, giver, asked_at, text) {
            Ok(line) => (line, Outcome::Accepted),
            Err(line) => (line, Outcome::Refused),
        };
        (self.deliver(batch, line), outcome)
    }

    /// The offer line, or why there is none.
    fn offer(
        &mut self,
        key: &CharacterKey,
        giver: &str,
        asked_at: Tick,
        text: &str,
    ) -> Result<String, String> {
        let none = no_task(giver);
        let Some(active) = self.active.as_mut().filter(|active| &active.key == key) else {
            return Err(none);
        };
        let quests = quest_log(active.quests.changes());
        if let Some(refusal) = refusal(&quests, giver) {
            return Err(refusal);
        }
        let seen = seen_texts(active);
        let known = known(&active.character, giver, &seen, &quests);
        let Ok(offer) = quest::checked_quest(text, &known) else {
            return Err(none);
        };
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
        };
        if active.quests.add(change).is_err() {
            return Err(none);
        }
        // The log holds the offer, so a refusal of the world loses only the fact.
        let _ = self.change(|character| {
            character.offer_quest(at, giver, &thing_name(number, &offer.title))
        });
        Ok(line)
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
        if let Some(refusal) = refusal(&quests, &offer.giver) {
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
    pub(super) fn advance_quests(&mut self, at: Tick, met: &Encounter) -> Result<(), StoryError> {
        loop {
            let Some(active) = self.active.as_mut() else {
                return Ok(());
            };
            let here = Here {
                at,
                places: active.character.place_names(),
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
    pub(super) fn count_kill(&mut self, at: Tick, name: &str) -> Result<Vec<Output>, StoryError> {
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
        Ok(Vec::new())
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

/// The reason in words when the giver cannot give you a quest now.
fn refusal(quests: &[Tracked], giver: &str) -> Option<String> {
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
        seen,
    }
}

/// The giver, and every place, NPC, and creature that the prompt can offer.
fn known_names<'a>(known: &Known<'a>) -> Vec<&'a str> {
    let mut names = vec![known.giver];
    names.extend(&known.zones);
    names.extend(&known.subzones);
    names.extend(&known.npcs);
    names.extend(&known.foes);
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
