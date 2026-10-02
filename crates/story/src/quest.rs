//! Personal side quests (GAMEPLAY.md 3.4). The model proposes a quest, and the code checks
//! each step against the world before the offer shows (5.2). The words of the prompt live
//! here.

use crate::check::{json_object, mentions, plain_text, same_words};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::seen::{SeenText, TextKind};
use crate::talk::persona;
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_TITLE_CHARS: usize = 60;
pub const MAX_TEXT_CHARS: usize = 400;
pub const MAX_STEPS: usize = 3;

/// The offer goes out as a narrator line, so it has the limit of one (Gnomish Relay
/// SPEC.md 9.8).
pub const MAX_OFFER_BYTES: usize = 1000;

/// The names of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

/// A kill step asks for 1 to this many kills.
pub const MAX_KILLS: u8 = 10;

/// A goal that the addon sees in game events (GAMEPLAY.md 3.4). Each name is a string of
/// the game, so progress matches the event byte for byte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "goal", rename_all = "snake_case")]
pub enum Step {
    Visit {
        place: String,
    },
    Meet {
        npc: String,
    },
    Kill {
        creature: String,
        #[serde(deserialize_with = "whole_number")]
        count: u8,
    },
}

/// A small model often writes a count as text: "3" counts as 3. The step check still
/// holds the count to its range.
fn whole_number<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Number(u8),
        Text(String),
    }
    match Written::deserialize(deserializer)? {
        Written::Number(count) => Ok(count),
        Written::Text(text) => text.trim().parse().map_err(serde::de::Error::custom),
    }
}

impl Step {
    /// The place, NPC, or creature that the step names.
    #[must_use]
    pub fn target(&self) -> &str {
        match self {
            Step::Visit { place } => place,
            Step::Meet { npc } => npc,
            Step::Kill { creature, .. } => creature,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quest {
    pub title: String,
    pub text: String,
    pub steps: Vec<Step>,
}

/// What the world of the player holds, as far as a quest can use it.
#[derive(Debug, Default)]
pub struct Known<'a> {
    pub giver: &'a str,
    /// The zones that you visited.
    pub zones: Vec<&'a str>,
    pub subzones: Vec<&'a str>,
    /// The NPCs that you can meet: you met them, or saw them friendly. Never a beast, and
    /// never the dead of your story.
    pub npcs: Vec<&'a str>,
    /// The creatures that you saw hostile, except the dead of your story.
    pub foes: Vec<&'a str>,
    /// The places, NPCs, and creatures of your newest task. The next task names none of
    /// them, so two tasks in a row never send you to the same target.
    pub last_targets: Vec<&'a str>,
    /// Every text that you read. Only the quests count.
    pub seen: &'a [SeenText],
}

impl Known<'_> {
    /// The places that a visit step can name, as the check allows them.
    #[must_use]
    pub fn places(&self) -> Vec<&str> {
        let all = self.zones.iter().chain(&self.subzones);
        let visit = |place: &str| Step::Visit {
            place: place.to_string(),
        };
        all.copied()
            .filter(|place| self.step_fault(&visit(place)).is_none())
            .collect()
    }

    /// The NPCs that a meet step can name, as the check allows them.
    #[must_use]
    pub fn people(&self) -> Vec<&str> {
        let meet = |npc: &str| Step::Meet {
            npc: npc.to_string(),
        };
        let allowed = |npc: &&str| self.step_fault(&meet(npc)).is_none();
        self.npcs.iter().copied().filter(allowed).collect()
    }

    /// The creatures that a kill step can name, as the check allows them.
    #[must_use]
    pub fn prey(&self) -> Vec<&str> {
        let kill = |creature: &str| Step::Kill {
            creature: creature.to_string(),
            count: 1,
        };
        let allowed = |creature: &&str| self.step_fault(&kill(creature)).is_none();
        self.foes.iter().copied().filter(allowed).collect()
    }

    /// The first rule of 3.4 that one step breaks. A zone never counts as a goal of a game
    /// quest: most quest texts name their zone, so the rule then bans every zone.
    fn step_fault(&self, step: &Step) -> Option<QuestFault> {
        let target = step.target();
        if self.last_targets.contains(&target) {
            return Some(QuestFault::LastTask(target.to_string()));
        }
        match step {
            Step::Visit { place } if self.zones.contains(&place.as_str()) => None,
            Step::Visit { place } if self.subzones.contains(&place.as_str()) => {
                in_game_quests(place, self.seen)
            }
            Step::Visit { place } => Some(QuestFault::UnknownPlace(place.clone())),
            Step::Meet { npc } if npc == self.giver => Some(QuestFault::MeetGiver),
            Step::Meet { npc } if self.npcs.contains(&npc.as_str()) => {
                in_game_quests(npc, self.seen)
            }
            Step::Meet { npc } => Some(QuestFault::UnknownNpc(npc.clone())),
            Step::Kill { creature, .. } if creature == self.giver => Some(QuestFault::KillGiver),
            Step::Kill { count, .. } if !(1..=MAX_KILLS).contains(count) => {
                Some(QuestFault::KillCount(*count))
            }
            Step::Kill { creature, .. } if self.foes.contains(&creature.as_str()) => {
                in_game_quests(creature, self.seen)
            }
            Step::Kill { creature, .. } => Some(QuestFault::UnknownFoe(creature.clone())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum QuestFault {
    #[error("the answer holds no quest JSON")]
    NotJson,
    #[error("the title is empty, too long, or names something after the cutoff")]
    BadTitle,
    #[error("the text is empty, too long, or names something after the cutoff")]
    BadText,
    #[error("the offer does not fit in one narrator line")]
    TooLong,
    #[error("a quest has 1 to {MAX_STEPS} steps, not {0}")]
    StepCount(usize),
    #[error("you never heard of the place \"{0}\"")]
    UnknownPlace(String),
    #[error("you never met or saw \"{0}\" as a friend, or it is dead or a beast")]
    UnknownNpc(String),
    #[error("you never saw \"{0}\" as a foe, or it is dead")]
    UnknownFoe(String),
    #[error("a kill step asks for 1 to {MAX_KILLS} kills, not {0}")]
    KillCount(u8),
    #[error("a step sends you back to the giver")]
    MeetGiver,
    #[error("a step asks you to kill the giver")]
    KillGiver,
    #[error("two steps are the same")]
    RepeatedStep,
    #[error("\"{0}\" belongs to a quest of the game")]
    GameQuest(String),
    #[error("\"{0}\" is a target of your last quest")]
    LastTask(String),
}

#[derive(Deserialize)]
struct Reply {
    title: String,
    text: String,
    steps: Vec<Step>,
}

/// # Errors
///
/// Returns the first rule that the answer breaks.
pub fn checked_quest(answer: &str, known: &Known<'_>) -> Result<Quest, QuestFault> {
    let json = json_object(answer).ok_or(QuestFault::NotJson)?;
    let reply: Reply = serde_json::from_str(json).map_err(|_| QuestFault::NotJson)?;
    let title =
        plain_text(&reply.title, MAX_TITLE_CHARS, MAX_OFFER_BYTES).ok_or(QuestFault::BadTitle)?;
    let text =
        plain_text(&reply.text, MAX_TEXT_CHARS, MAX_OFFER_BYTES).ok_or(QuestFault::BadText)?;
    if !(1..=MAX_STEPS).contains(&reply.steps.len()) {
        return Err(QuestFault::StepCount(reply.steps.len()));
    }
    let game_title = |seen: &SeenText| seen.title.as_deref().is_some_and(|t| same_words(t, &title));
    if game_quests(known.seen).any(game_title) {
        return Err(QuestFault::GameQuest(title));
    }
    for (n, step) in reply.steps.iter().enumerate() {
        if let Some(fault) = known.step_fault(step) {
            return Err(fault);
        }
        let same_target = |earlier: &Step| earlier.target() == step.target();
        if reply.steps[..n].iter().any(same_target) {
            return Err(QuestFault::RepeatedStep);
        }
    }
    let quest = Quest {
        title,
        text,
        steps: reply.steps,
    };
    if offer_line(known.giver, &quest).len() > MAX_OFFER_BYTES {
        return Err(QuestFault::TooLong);
    }
    Ok(quest)
}

/// Only the quests that you read count. A quest of the game that you never saw can still
/// share a goal with a side quest.
fn in_game_quests(name: &str, seen: &[SeenText]) -> Option<QuestFault> {
    let named = game_quests(seen).any(|quest| {
        mentions(&quest.text, name) || quest.title.as_deref().is_some_and(|t| mentions(t, name))
    });
    named.then(|| QuestFault::GameQuest(name.to_string()))
}

fn game_quests(seen: &[SeenText]) -> impl Iterator<Item = &SeenText> {
    seen.iter().filter(|text| text.kind == TextKind::Quest)
}

/// The words that the player sees in the chat.
#[must_use]
pub fn offer_line(giver: &str, quest: &Quest) -> String {
    format!(
        "{giver} has a quest for you: {}. {} Type /quest accept.",
        quest.title, quest.text
    )
}

/// The lists hold only the targets that the check allows, so the model has nothing else
/// to pick.
#[must_use]
pub fn prompt(known: &Known<'_>, place: Option<&str>) -> String {
    format!(
        "{}\n{HOUSE_RULES}\n\nGive the player a small task of your own: a rumor, a favor, or \
         an errand.\n\nPlaces that the player can visit:\n{}\n\n\
         People that the player can meet:\n{}\n\n\
         Creatures that the player can hunt:\n{}\n\n\
         Rules:\n\
         - 1 to {MAX_STEPS} steps. A step is {{\"goal\": \"visit\", \"place\": \"<a place \
         above>\"}}, {{\"goal\": \"meet\", \"npc\": \"<a person above>\"}}, or \
         {{\"goal\": \"kill\", \"creature\": \"<a creature above>\", \"count\": <1 to \
         {MAX_KILLS}>}}.\n\
         - Copy each name exactly as the list writes it. Use no other place, person, or \
         creature. An empty list has nothing to use.\n\
         - Each step names a different place, person, or creature.\n\
         - The task is not a quest of the game, and it does not continue one.\n\
         - The title has at most {MAX_TITLE_CHARS} characters. The text has at most 60 \
         words, in your own voice.\n\n\
         Reply with JSON only: {{\"title\": \"...\", \"text\": \"...\", \"steps\": [...]}}",
        persona(known.giver, place),
        list(&known.places()),
        list(&known.people()),
        list(&known.prey())
    )
}

fn list(names: &[&str]) -> String {
    fenced(&bulleted(&names[..names.len().min(PROMPT_NAMES)]))
}

/// Past this, a new quest waits until you finish one.
pub const MAX_OPEN_QUESTS: usize = 3;

/// One change of the quest log. The quest file holds these lines, oldest first. The words
/// of a quest are not facts, so they live next to the history, as the hero does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum QuestChange {
    Offered {
        number: u64,
        at: Tick,
        giver: String,
        title: String,
        text: String,
        steps: Vec<Step>,
    },
    Accepted {
        number: u64,
        at: Tick,
    },
    Declined {
        number: u64,
        at: Tick,
    },
    StepDone {
        number: u64,
        step: usize,
        at: Tick,
    },
    /// One kill for the kill step with this index.
    Killed {
        number: u64,
        step: usize,
        at: Tick,
    },
    Abandoned {
        number: u64,
        at: Tick,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Offered,
    Accepted,
    Declined,
    Done,
    /// Given up after it was accepted, or before.
    Abandoned,
}

/// A quest as the log stands now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tracked {
    pub number: u64,
    pub offered_at: Tick,
    pub giver: String,
    pub title: String,
    pub text: String,
    pub steps: Vec<Step>,
    /// The steps done, from the first. The next step has this index.
    pub steps_done: usize,
    /// The kills for the next step, when it is a kill step.
    pub kills: u8,
    pub status: Status,
    /// When the last step was done.
    pub done_at: Option<Tick>,
}

impl Tracked {
    #[must_use]
    pub fn next_step(&self) -> Option<&Step> {
        (self.status == Status::Accepted)
            .then(|| self.steps.get(self.steps_done))
            .flatten()
    }

    /// Does the next step hold at this moment: you stand in its place, you meet its NPC,
    /// or you made its kills?
    #[must_use]
    pub fn next_step_holds(&self, places_here: &[&str], npc: Option<&str>) -> bool {
        match self.next_step() {
            Some(Step::Visit { place }) => places_here.contains(&place.as_str()),
            Some(Step::Meet { npc: wanted }) => npc == Some(wanted.as_str()),
            Some(Step::Kill { count, .. }) => self.kills >= *count,
            None => false,
        }
    }

    /// Is the next step a kill of this creature?
    #[must_use]
    pub fn hunts(&self, name: &str) -> bool {
        matches!(self.next_step(), Some(Step::Kill { creature, .. }) if creature == name)
    }
}

/// The quests of a log, oldest first. At most one offer of each giver waits: a new offer
/// ends the old one of the same giver. A change that does not fit the state of its quest
/// changes nothing.
#[must_use]
pub fn quest_log(changes: &[QuestChange]) -> Vec<Tracked> {
    let mut quests: Vec<Tracked> = Vec::new();
    for change in changes {
        apply(&mut quests, change);
    }
    quests
}

fn apply(quests: &mut Vec<Tracked>, change: &QuestChange) {
    match change {
        QuestChange::Offered {
            number,
            at,
            giver,
            title,
            text,
            steps,
        } => {
            let same_giver = |q: &&mut Tracked| q.status == Status::Offered && &q.giver == giver;
            for waiting in quests.iter_mut().filter(same_giver) {
                waiting.status = Status::Declined;
            }
            quests.push(Tracked {
                number: *number,
                offered_at: *at,
                giver: giver.clone(),
                title: title.clone(),
                text: text.clone(),
                steps: steps.clone(),
                steps_done: 0,
                kills: 0,
                status: Status::Offered,
                done_at: None,
            });
        }
        QuestChange::Accepted { number, .. } => answer_offer(quests, *number, Status::Accepted),
        QuestChange::Declined { number, .. } => answer_offer(quests, *number, Status::Declined),
        QuestChange::Abandoned { number, .. } => abandon(quests, *number),
        QuestChange::Killed { number, step, .. } => count_kill(quests, *number, *step),
        QuestChange::StepDone { number, step, at } => {
            let Some(quest) = quests.iter_mut().find(|q| q.number == *number) else {
                return;
            };
            let next = quest.status == Status::Accepted && quest.steps_done == *step;
            if !next || *step >= quest.steps.len() {
                return;
            }
            quest.steps_done += 1;
            quest.kills = 0;
            if quest.steps_done == quest.steps.len() {
                quest.status = Status::Done;
                quest.done_at = Some(*at);
            }
        }
    }
}

/// A kill counts only for the next step, and only up to its count.
fn count_kill(quests: &mut [Tracked], number: u64, step: usize) {
    let Some(quest) = quests.iter_mut().find(|q| q.number == number) else {
        return;
    };
    let Some(Step::Kill { count, .. }) = quest.next_step() else {
        return;
    };
    if quest.steps_done == step && quest.kills < *count {
        quest.kills += 1;
    }
}

/// Only an open quest can be abandoned: one that waits, or one that you hold.
fn abandon(quests: &mut [Tracked], number: u64) {
    let open = |q: &&mut Tracked| {
        q.number == number && matches!(q.status, Status::Offered | Status::Accepted)
    };
    if let Some(quest) = quests.iter_mut().find(open) {
        quest.status = Status::Abandoned;
    }
}

fn answer_offer(quests: &mut [Tracked], number: u64, status: Status) {
    if let Some(quest) = quests
        .iter_mut()
        .find(|q| q.number == number && q.status == Status::Offered)
    {
        quest.status = status;
    }
}

/// Numbers are never reused, so a line names one quest for good.
#[must_use]
pub fn next_number(changes: &[QuestChange]) -> u64 {
    let offers = changes
        .iter()
        .filter(|change| matches!(change, QuestChange::Offered { .. }));
    u64::try_from(offers.count()).map_or(u64::MAX, |count| count.saturating_add(1))
}

/// The name of the quest thing in the world. The number keeps it apart from a title or
/// another quest with the same words.
#[must_use]
pub fn thing_name(number: u64, title: &str) -> String {
    format!("quest {number}: {title}")
}

/// The title of a quest thing, or None for a thing that is no quest.
#[must_use]
pub fn title_of_thing(name: &str) -> Option<&str> {
    let (number, title) = name.strip_prefix("quest ")?.split_once(": ")?;
    number.parse::<u64>().ok()?;
    Some(title)
}
