//! "Help me write this" for a player task (GAMEPLAY.md 4.7). A player gives an idea, a
//! model turns it into a title, a text, and steps, and the code checks each step against
//! the world before the draft goes back (5.2). The words of the prompt live here.

use crate::check::{in_voice, json_object, plain_text};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The limits of the addon messages of a player task, in bytes: the draft must fit a task
/// that the addon can send.
pub const MAX_TITLE_BYTES: usize = 60;
pub const MAX_TEXT_BYTES: usize = 400;
pub const MAX_TARGET_BYTES: usize = 96;
pub const MAX_STEPS: usize = 5;
pub const MAX_COUNT: u32 = 250;

/// The limit of the relay for an idea (Gnomish Relay SPEC.md 9.8).
pub const MAX_IDEA_BYTES: usize = 255;

/// The places and NPCs of one list in the prompt. A long list costs tokens and adds little.
const PROMPT_NAMES: usize = 20;

/// The draft as the relay carries it: each step a goal and a target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub title: String,
    pub text: String,
    pub steps: Vec<DraftStep>,
}

/// `goal` is a step kind of the addon: "place", "npc", "kill", or "item". A "kill" or an
/// "item" target can start with a count: "3 Rattlecage Soldier", "10 Linen Cloth".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftStep {
    pub goal: String,
    pub target: String,
}

/// What the world of the player holds, as far as a draft can use it. A player task never
/// names another player to a model (5.11), so no list holds a player.
#[derive(Debug, Default)]
pub struct Known<'a> {
    pub zones: Vec<&'a str>,
    pub subzones: Vec<&'a str>,
    /// The NPCs that a task can send you to meet (GAMEPLAY.md 3.4).
    pub npcs: Vec<&'a str>,
    /// The creatures that a task can send you to kill: seen hostile, or a rare or a boss
    /// that you defeated.
    pub foes: Vec<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum DraftFault {
    #[error("the answer holds no draft JSON")]
    NotJson,
    #[error("the title is empty, too long, holds a `|`, or names something after the cutoff")]
    BadTitle,
    #[error("the text is empty, too long, holds a `|`, or names something after the cutoff")]
    BadText,
    #[error("a draft has 1 to {MAX_STEPS} steps, not {0}")]
    StepCount(usize),
    #[error("\"{0}\" is no goal of a player task")]
    UnknownGoal(String),
    #[error("you never heard of the place \"{0}\"")]
    UnknownPlace(String),
    #[error("you never met \"{0}\", or it is dead")]
    UnknownNpc(String),
    #[error("the target \"{0}\" is empty, too long, or holds a bad count")]
    BadTarget(String),
    #[error("two steps are the same")]
    RepeatedStep,
}

/// The idea as the player wrote it, or None when it is empty, too long for the relay, or
/// holds a control character.
#[must_use]
pub fn checked_idea(idea: &str) -> Option<&str> {
    let idea = idea.trim();
    let fits = !idea.is_empty() && idea.len() <= MAX_IDEA_BYTES;
    (fits && !idea.chars().any(char::is_control)).then_some(idea)
}

#[must_use]
pub fn prompt(known: &Known<'_>, idea: &str) -> String {
    let places: Vec<&str> = known.zones.iter().chain(&known.subzones).copied().collect();
    format!(
        "You help a player of World of Warcraft write a small task for a friend. The player is \
         the author: keep their idea, and make it read well.\n{HOUSE_RULES}\n\n\
         Places that the player knows:\n{}\n\n\
         People that the player can talk to:\n{}\n\n\
         People and creatures that the player can defeat:\n{}\n\n\
         Rules:\n\
         - 1 to {MAX_STEPS} steps. A step is {{\"goal\": \"place\", \"target\": \"<a place \
         above>\"}}, {{\"goal\": \"npc\", \"target\": \"<a person to talk to>\"}}, \
         {{\"goal\": \"kill\", \"target\": \"<a number> <a person or creature to defeat>\"}}, \
         or {{\"goal\": \"item\", \"target\": \"<a number> <an item to bring>\"}}.\n\
         - Copy each name exactly as the list writes it. Use no other place or person.\n\
         - Name no player. Call the one who gets the task \"my friend\".\n\
         - The title has at most {MAX_TITLE_BYTES} characters. The text has at most 60 words, \
         in the voice of the player.\n\n\
         The idea of the player:\n{}\n\n\
         Reply with JSON only: {{\"title\": \"...\", \"text\": \"...\", \"steps\": [...]}}",
        list(&places),
        list(&known.npcs),
        list(&known.foes),
        fenced(idea)
    )
}

fn list(names: &[&str]) -> String {
    fenced(&bulleted(&names[..names.len().min(PROMPT_NAMES)]))
}

/// # Errors
///
/// Returns the first rule that the answer breaks.
pub fn checked_draft(answer: &str, known: &Known<'_>) -> Result<Draft, DraftFault> {
    let json = json_object(answer).ok_or(DraftFault::NotJson)?;
    let reply: Draft = serde_json::from_str(json).map_err(|_| DraftFault::NotJson)?;
    let title = task_text(&reply.title, MAX_TITLE_BYTES).ok_or(DraftFault::BadTitle)?;
    let text = task_text(&reply.text, MAX_TEXT_BYTES).ok_or(DraftFault::BadText)?;
    if !(1..=MAX_STEPS).contains(&reply.steps.len()) {
        return Err(DraftFault::StepCount(reply.steps.len()));
    }
    for (n, step) in reply.steps.iter().enumerate() {
        check_step(step, known)?;
        if reply.steps[..n].contains(step) {
            return Err(DraftFault::RepeatedStep);
        }
    }
    Ok(Draft {
        title,
        text,
        steps: reply.steps,
    })
}

/// A plain text in voice that the addon can send: a `|` starts a WoW escape, and the addon
/// refuses one.
fn task_text(text: &str, max_bytes: usize) -> Option<String> {
    let line = plain_text(text, max_bytes, max_bytes)?;
    (in_voice(&line) && !line.contains('|')).then_some(line)
}

fn check_step(step: &DraftStep, known: &Known<'_>) -> Result<(), DraftFault> {
    let target = step.target.as_str();
    match step.goal.as_str() {
        "place" if known.zones.contains(&target) || known.subzones.contains(&target) => Ok(()),
        "place" => Err(DraftFault::UnknownPlace(target.to_string())),
        "npc" if known.npcs.contains(&target) => Ok(()),
        "npc" => Err(DraftFault::UnknownNpc(target.to_string())),
        "kill" => {
            let foe = counted_name(target)?;
            known
                .foes
                .contains(&foe)
                .then_some(())
                .ok_or_else(|| DraftFault::UnknownNpc(foe.to_string()))
        }
        "item" => task_text(counted_name(target)?, MAX_TARGET_BYTES)
            .map(|_| ())
            .ok_or_else(|| DraftFault::BadTarget(target.to_string())),
        goal => Err(DraftFault::UnknownGoal(goal.to_string())),
    }
}

/// The name of "3 Rattlecage Soldier", with its count checked. A target with no count is
/// one.
fn counted_name(target: &str) -> Result<&str, DraftFault> {
    let bad = || DraftFault::BadTarget(target.to_string());
    if target.is_empty() || target.len() > MAX_TARGET_BYTES {
        return Err(bad());
    }
    let Some((count, name)) = target.split_once(' ') else {
        return Ok(target);
    };
    if !count.chars().all(|c| c.is_ascii_digit()) {
        return Ok(target);
    }
    let count: u32 = count.parse().map_err(|_| bad())?;
    if !(1..=MAX_COUNT).contains(&count) || name.is_empty() {
        return Err(bad());
    }
    Ok(name)
}
