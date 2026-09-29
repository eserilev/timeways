//! Side quests in the story program (GAMEPLAY.md 3.4): the offer at the end of a batch,
//! the answer of the player, and the progress from game events.

use super::{Active, Pending, Story, StoryError, checked_name};
use crate::character::Character;
use crate::input::MessageId;
use crate::quest::{
    self, Known, MAX_OPEN_QUESTS, QuestChange, Status, Tracked, next_number, quest_log, step_holds,
};
use crate::seen::SeenText;
use crate::store::CharacterKey;
use crate::story::Output;
use hourglass::Tick;

/// A `/quest` of this batch. It waits for `batch_end`, because the offer comes back as
/// the narrator line.
pub(super) struct QuestRequest {
    giver: String,
    at: Tick,
}

/// The name of the quest thing in the world. The number keeps it apart from a title or
/// another quest with the same words.
fn thing_name(number: u64, title: &str) -> String {
    format!("quest {number}: {title}")
}

impl Story {
    /// Asking is meeting, as a talk is.
    pub(super) fn ask_quest(&mut self, at: Tick, npc: String) -> Result<Vec<Output>, StoryError> {
        checked_name(&npc)?;
        self.change(|character| character.meet_npc(at, &npc))?;
        self.quest_request = Some(QuestRequest { giver: npc, at });
        Ok(Vec::new())
    }

    /// The offer takes the place of the narrator line. A request that breaks a rule gets
    /// a line of the code, and no model call.
    pub(super) fn quest_call(&mut self, batch: MessageId, request: QuestRequest) -> Output {
        let Some(active) = self.active.as_ref() else {
            return narrator_line(batch, None);
        };
        let quests = quest_log(active.quests.changes());
        let giver = request.giver.as_str();
        if let Some(refusal) = refusal(&quests, giver, active.character.is_dead(giver)) {
            return narrator_line(batch, Some(refusal));
        }
        let seen = seen_texts(active);
        let known = known(&active.character, giver, &seen);
        let prompt = quest::prompt(&known, active.character.place_of(giver));
        let pending = Pending::Quest {
            batch,
            key: active.key.clone(),
            giver: request.giver,
            at: request.at,
        };
        self.open_call(pending, prompt)
    }

    /// An offer for another character, or one that breaks a rule, shows no task.
    pub(super) fn quest_answered(
        &mut self,
        batch: MessageId,
        key: &CharacterKey,
        giver: &str,
        asked_at: Tick,
        text: &str,
    ) -> Output {
        let none = no_offer(batch, giver);
        let Some(active) = self.active.as_mut().filter(|active| &active.key == key) else {
            return none;
        };
        let seen = seen_texts(active);
        let Ok(offer) = quest::checked_quest(text, &known(&active.character, giver, &seen)) else {
            return none;
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
            return none;
        }
        // The log holds the offer, so a refusal of the world loses only the fact.
        let _ = self.change(|character| {
            character.offer_quest(at, giver, &thing_name(number, &offer.title))
        });
        narrator_line(batch, Some(line))
    }

    /// Accepting can finish a step at once: you can stand in its place already.
    pub(super) fn answer_quest(
        &mut self,
        at: Tick,
        status: Status,
    ) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        let quests = quest_log(active.quests.changes());
        let Some(offer) = quests.iter().find(|quest| quest.status == Status::Offered) else {
            return Ok(Vec::new());
        };
        let number = offer.number;
        if status == Status::Declined {
            active.quests.add(QuestChange::Declined { number, at })?;
            return Ok(Vec::new());
        }
        active.quests.add(QuestChange::Accepted { number, at })?;
        let name = thing_name(number, &offer.title);
        self.change(|character| character.accept_quest(at, &name))?;
        self.advance_quests(at, None)
    }

    /// Each accepted quest whose next step holds now moves on, one step at a time, while
    /// the steps hold. The last step finishes the quest.
    pub(super) fn advance_quests(
        &mut self,
        at: Tick,
        npc: Option<&str>,
    ) -> Result<Vec<Output>, StoryError> {
        loop {
            let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
            let places = active.character.place_names();
            let quests = quest_log(active.quests.changes());
            let Some(quest) = quests.iter().find(|quest| {
                quest
                    .next_step()
                    .is_some_and(|step| step_holds(step, &places, npc))
            }) else {
                return Ok(Vec::new());
            };
            let (number, step) = (quest.number, quest.steps_done);
            active
                .quests
                .add(QuestChange::StepDone { number, step, at })?;
            if step + 1 == quest.steps.len() {
                let (giver, name) = (quest.giver.clone(), thing_name(number, &quest.title));
                self.change(|character| character.finish_quest(at, &giver, &name))?;
            }
        }
    }
}

fn narrator_line(batch: MessageId, line: Option<String>) -> Output {
    Output::EventsSeen {
        id: batch,
        narrator: line,
    }
}

/// With no model, or with an offer that breaks a rule, the giver has nothing to say.
pub(super) fn no_offer(batch: MessageId, giver: &str) -> Output {
    narrator_line(batch, Some(no_task(giver)))
}

fn no_task(giver: &str) -> String {
    format!("{giver} has no task for you now.")
}

/// The reason in words when the giver cannot offer a quest now.
fn refusal(quests: &[Tracked], giver: &str, dead: bool) -> Option<String> {
    if dead {
        return Some(no_task(giver));
    }
    let open: Vec<&Tracked> = quests
        .iter()
        .filter(|quest| quest.status == Status::Accepted)
        .collect();
    if let Some(quest) = open.iter().find(|quest| quest.giver == giver) {
        return Some(format!(
            "{giver} waits for you to finish \"{}\".",
            quest.title
        ));
    }
    if open.len() >= MAX_OPEN_QUESTS {
        return Some("Your quest log is full. Finish a quest first.".to_string());
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

/// A zone that a text that you read names counts as a place that you heard of.
fn known<'a>(character: &'a Character, giver: &'a str, seen: &'a [SeenText]) -> Known<'a> {
    let mut zones = character.visited_zones();
    for zone in seen.iter().filter_map(|text| text.zone.as_deref()) {
        if !zones.contains(&zone) {
            zones.push(zone);
        }
    }
    Known {
        giver,
        zones,
        subzones: character.visited_subzones(),
        npcs: character.living_npcs_met(),
        seen,
    }
}
