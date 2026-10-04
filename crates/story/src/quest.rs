//! Personal side quests (GAMEPLAY.md 3.4). The model proposes a quest, and the code checks
//! each step against the world before the offer shows (5.2).

use crate::check::{json_object, plain_text, same_words};
use crate::seen::SeenText;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod answer;
mod known;
mod log;
mod progress;
mod prompt;
mod structure;
mod view;

use answer::{Reply, flattened};
pub use known::Known;
use known::game_quests;
pub use log::{QuestChange, Status, Tracked, next_number, quest_log};
pub use progress::{Encounter, Here};
pub use prompt::prompt;
use structure::structure_fault;
pub use view::{QuestView, StepState, StepView};

pub const MAX_TITLE_CHARS: usize = 60;
pub const MAX_TEXT_CHARS: usize = 400;
pub const MAX_STEPS: usize = 4;

/// The offer goes out as a narrator line, so it has the limit of one (Gnomish Relay
/// SPEC.md 9.8).
pub const MAX_OFFER_BYTES: usize = 1000;

/// A kill step asks for 1 to this many kills.
pub const MAX_KILLS: u8 = 10;

/// A wait step lasts 1 to this many days. More holds one of your 3 open quests too long.
pub const MAX_WAIT_DAYS: u8 = 3;

/// A day of the clock of the lines from the addon, not a day of the calendar.
pub const DAY_SECONDS: u64 = 24 * 3600;

/// The topic of a talk step: one short phrase in a step line.
pub const MAX_TOPIC_CHARS: usize = 60;
pub const MAX_TOPIC_BYTES: usize = 240;

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
    /// `/talk` to the NPC. `about` is a topic in a few words.
    Talk {
        npc: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        about: Option<String>,
    },
    Kill {
        creature: String,
        #[serde(deserialize_with = "whole_number")]
        count: u8,
    },
    /// Come back later: the steps after it open only after the wait.
    Wait {
        #[serde(deserialize_with = "whole_number")]
        days: u8,
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
    /// The place, NPC, or creature that the step names. A wait names none.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        match self {
            Step::Visit { place } => Some(place),
            Step::Meet { npc } | Step::Talk { npc, .. } => Some(npc),
            Step::Kill { creature, .. } => Some(creature),
            Step::Wait { .. } => None,
        }
    }

    /// The NPC that the step sends you to, as a meet or a talk does.
    #[must_use]
    pub fn person(&self) -> Option<&str> {
        match self {
            Step::Meet { npc } | Step::Talk { npc, .. } => Some(npc),
            Step::Visit { .. } | Step::Kill { .. } | Step::Wait { .. } => None,
        }
    }
}

/// The steps from `first` to `last`, both in, can be done in any order. The quest keeps
/// its steps in one flat list, so a line names a step by one index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnyOrder {
    pub first: usize,
    pub last: usize,
}

impl AnyOrder {
    #[must_use]
    pub fn contains(self, step: usize) -> bool {
        (self.first..=self.last).contains(&step)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quest {
    pub title: String,
    pub text: String,
    pub steps: Vec<Step>,
    pub any_order: Option<AnyOrder>,
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
    #[error(
        "a talk topic is empty, longer than {MAX_TOPIC_CHARS} characters, holds a |, or names \
         something after the cutoff"
    )]
    BadTopic,
    #[error("a step sends you back to the giver with no wait before it")]
    MeetGiver,
    #[error("a wait lasts 1 to {MAX_WAIT_DAYS} days, not {0}")]
    WaitDays(u8),
    #[error("a quest has at most one wait, and a wait is never the first or the last step")]
    WaitPlace,
    #[error("an any-order set holds 2 or 3 steps, not {0}")]
    AnyOrderSize(usize),
    #[error("an any-order set holds no wait")]
    AnyOrderWait,
    #[error("a quest has at most one any-order set")]
    AnyOrderTwice,
    #[error("a step asks you to kill the giver")]
    KillGiver,
    #[error("two steps are the same")]
    RepeatedStep,
    #[error("\"{0}\" belongs to a quest of the game")]
    GameQuest(String),
    #[error("\"{0}\" is a target of your last quest")]
    LastTask(String),
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
    let (steps, any_order) = flattened(reply.steps)?;
    if !(1..=MAX_STEPS).contains(&steps.len()) {
        return Err(QuestFault::StepCount(steps.len()));
    }
    let game_title = |seen: &SeenText| seen.title.as_deref().is_some_and(|t| same_words(t, &title));
    if game_quests(known.seen).any(game_title) {
        return Err(QuestFault::GameQuest(title));
    }
    if let Some(fault) = steps.iter().find_map(|step| known.step_fault(step)) {
        return Err(fault);
    }
    if let Some(fault) = structure_fault(&steps, known.giver) {
        return Err(fault);
    }
    let quest = Quest {
        title,
        text,
        steps,
        any_order,
    };
    if offer_line(known.giver, &quest).len() > MAX_OFFER_BYTES {
        return Err(QuestFault::TooLong);
    }
    Ok(quest)
}

/// The words that the player sees in the chat.
#[must_use]
pub fn offer_line(giver: &str, quest: &Quest) -> String {
    format!(
        "{giver} has a quest for you: {}. {} Type /quest accept.",
        quest.title, quest.text
    )
}

/// Past this, a new quest waits until you finish one.
pub const MAX_OPEN_QUESTS: usize = 3;

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
