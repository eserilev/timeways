//! Personal side quests (GAMEPLAY.md 3.4). The model proposes a quest, and the code checks
//! each step against the world before the offer shows (5.2). The words of the prompt live
//! here.

use crate::check::{json_object, mentions, plain_text};
use crate::seen::{SeenText, TextKind};
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use thiserror::Error;

pub const MAX_TITLE_CHARS: usize = 60;
pub const MAX_TEXT_CHARS: usize = 400;
pub const MAX_STEPS: usize = 3;

/// The offer goes out as a narrator line, so it has the limit of one (Gnomish Relay
/// SPEC.md 9.8).
pub const MAX_OFFER_BYTES: usize = 1000;

/// The places and NPCs of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

/// A goal that the addon sees in game events (GAMEPLAY.md 3.4). Each name is a string of
/// the game, so progress matches the event byte for byte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "goal", rename_all = "snake_case")]
pub enum Step {
    Visit { place: String },
    Meet { npc: String },
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
    /// The zones that you visited, or that a text that you read names.
    pub zones: Vec<&'a str>,
    pub subzones: Vec<&'a str>,
    /// The NPCs that you met, except the dead of your story.
    pub npcs: Vec<&'a str>,
    /// Every text that you read. Only the quests count.
    pub seen: &'a [SeenText],
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
    #[error("you never met \"{0}\", or it is dead")]
    UnknownNpc(String),
    #[error("a step sends you back to the giver")]
    MeetGiver,
    #[error("two steps are the same")]
    RepeatedStep,
    #[error("\"{0}\" belongs to a quest of the game")]
    GameQuest(String),
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
    if game_quests(known.seen).any(|seen| seen.title.as_deref() == Some(title.as_str())) {
        return Err(QuestFault::GameQuest(title));
    }
    for (n, step) in reply.steps.iter().enumerate() {
        check_step(step, known)?;
        if reply.steps[..n].contains(step) {
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

/// A zone never counts as a goal of a game quest. Most quest texts name their zone, so
/// the rule then bans every zone.
fn check_step(step: &Step, known: &Known<'_>) -> Result<(), QuestFault> {
    match step {
        Step::Visit { place } if known.zones.contains(&place.as_str()) => Ok(()),
        Step::Visit { place } if known.subzones.contains(&place.as_str()) => {
            not_in_game_quests(place, known.seen)
        }
        Step::Visit { place } => Err(QuestFault::UnknownPlace(place.clone())),
        Step::Meet { npc } if npc == known.giver => Err(QuestFault::MeetGiver),
        Step::Meet { npc } if known.npcs.contains(&npc.as_str()) => {
            not_in_game_quests(npc, known.seen)
        }
        Step::Meet { npc } => Err(QuestFault::UnknownNpc(npc.clone())),
    }
}

/// Only the quests that you read count. A quest of the game that you never saw can still
/// share a goal with a side quest.
fn not_in_game_quests(name: &str, seen: &[SeenText]) -> Result<(), QuestFault> {
    let named = game_quests(seen).any(|quest| {
        mentions(&quest.text, name) || quest.title.as_deref().is_some_and(|t| mentions(t, name))
    });
    if named {
        return Err(QuestFault::GameQuest(name.to_string()));
    }
    Ok(())
}

fn game_quests(seen: &[SeenText]) -> impl Iterator<Item = &SeenText> {
    seen.iter().filter(|text| text.kind == TextKind::Quest)
}

/// The words that the player sees in the chat.
#[must_use]
pub fn offer_line(giver: &str, quest: &Quest) -> String {
    format!(
        "{giver} has a task for you: {}. {} Type /quest accept.",
        quest.title, quest.text
    )
}

#[must_use]
pub fn prompt(known: &Known<'_>, place: Option<&str>) -> String {
    let giver = known.giver;
    let mut prompt = format!(
        "You are {giver}, a person in the world of Warcraft. The year is 25 ADP, before \
         Molten Core. You give the player a small task of your own: a rumor, a favor, or an \
         errand.\n"
    );
    if let Some(place) = place {
        let _ = writeln!(prompt, "You are in {place}.");
    }
    let places: Vec<&str> = known.zones.iter().chain(&known.subzones).copied().collect();
    list(&mut prompt, "Places that the player can visit", &places);
    list(&mut prompt, "People that the player can meet", &known.npcs);
    let _ = write!(
        prompt,
        "\nRules:\n\
         - 1 to {MAX_STEPS} steps. A step is {{\"goal\": \"visit\", \"place\": \"<a place \
         above>\"}} or {{\"goal\": \"meet\", \"npc\": \"<a person above>\"}}.\n\
         - Copy each name exactly as the list writes it. Use no other place or person.\n\
         - The task is not a quest of the game, and it does not continue one.\n\
         - The title has at most {MAX_TITLE_CHARS} characters. The text has at most 60 \
         words, in your own voice, in plain text.\n\
         - Name no place, person, or event from after the year 25 ADP.\n\
         - Reply with JSON only: {{\"title\": \"...\", \"text\": \"...\", \"steps\": [...]}}"
    );
    prompt
}

fn list(prompt: &mut String, heading: &str, names: &[&str]) {
    let _ = write!(prompt, "\n{heading}:\n");
    for name in names.iter().take(PROMPT_NAMES) {
        let _ = writeln!(prompt, "- {name}");
    }
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Offered,
    Accepted,
    Declined,
    Done,
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
                status: Status::Offered,
                done_at: None,
            });
        }
        QuestChange::Accepted { number, .. } => answer_offer(quests, *number, Status::Accepted),
        QuestChange::Declined { number, .. } => answer_offer(quests, *number, Status::Declined),
        QuestChange::StepDone { number, step, at } => {
            let Some(quest) = quests.iter_mut().find(|q| q.number == *number) else {
                return;
            };
            let next = quest.status == Status::Accepted && quest.steps_done == *step;
            if !next || *step >= quest.steps.len() {
                return;
            }
            quest.steps_done += 1;
            if quest.steps_done == quest.steps.len() {
                quest.status = Status::Done;
                quest.done_at = Some(*at);
            }
        }
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

/// Does the step hold at this moment: you stand in its place, or you meet its NPC?
#[must_use]
pub fn step_holds(step: &Step, places_here: &[&str], npc: Option<&str>) -> bool {
    match step {
        Step::Visit { place } => places_here.contains(&place.as_str()),
        Step::Meet { npc: wanted } => npc == Some(wanted.as_str()),
    }
}
