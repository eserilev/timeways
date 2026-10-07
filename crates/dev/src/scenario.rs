//! A scenario file: invented input lines of the addon, in batches (TESTING.md, "Dev mode").
//!
//! The format, one item for each line:
//!
//! - A line that starts with `#` is a comment.
//! - A blank line ends a batch, as a flush of the outbox does in the game.
//! - `{"type": ..., "at": ...}` is an input line of `crates/story/src/input.rs`. Its `at`
//!   counts the seconds after the start of the scenario. The tool adds the character line
//!   at the start of each batch, and the bridge adds the ids.
//! - `{"model": "<kind of call>", "answer": ...}` is the answer of the next model call of
//!   that kind, such as "quest". The answer is a text, or an object that goes as its JSON.
//! - `{"bench": "<moment>"}` names the moment of its batch, for `timeways-dev bench-model`
//!   (`bench.rs`). Other commands read no name.

use serde_json::{Map, Value};
use thiserror::Error;

/// The scenarios that ship with the tool, each with one line about it.
pub const BUILT_IN: &[(&str, &str, &str)] = &[
    (
        "fresh",
        "A new human paladin at level 2, in the first open chapter.",
        include_str!("../scenarios/fresh.jsonl"),
    ),
    (
        "level-30-paladin",
        "About 10 chapters, 2 dungeon tales, a nemesis with revenge, quests, and a mount.",
        include_str!("../scenarios/level-30-paladin.jsonl"),
    ),
    (
        "raider-60",
        "An orc warrior at 60: repeat raid clears, tales with run counts, epic mount and gear.",
        include_str!("../scenarios/raider-60.jsonl"),
    ),
    (
        "story-inbox",
        "Two accepted stories from players. In the game, /twdev inbox adds 5 that wait.",
        include_str!("../scenarios/story-inbox.jsonl"),
    ),
    (
        "edits",
        "Chapters, a tale, and the title page with the player's own words, and a restore.",
        include_str!("../scenarios/edits.jsonl"),
    ),
    (
        "side-quests",
        "Side quests of every step kind, in every state, with fixed model answers.",
        include_str!("../scenarios/side-quests.jsonl"),
    ),
    (
        "flavor-and-hero",
        "A full Hero sheet and notes, every joke title, a quest mark, PvP, an inn, and a flight.",
        include_str!("../scenarios/flavor-and-hero.jsonl"),
    ),
    (
        "outcome-lore",
        "A human warrior who left the Deadmines before VanCleef, for the lore of his death.",
        include_str!("../scenarios/outcome-lore.jsonl"),
    ),
    (
        "dungeon-setups",
        "A human warrior in Westfall who never entered the Deadmines, for its setups and entries.",
        include_str!("../scenarios/dungeon-setups.jsonl"),
    ),
];

#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub batches: Vec<Batch>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Batch {
    pub lines: Vec<Map<String, Value>>,
    pub answers: Vec<Answer>,
    /// The moment that a bench measures in this batch.
    pub moment: Option<String>,
}

/// A fixed answer for the next model call of `kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScenarioError {
    #[error("line {line}: not a JSON object")]
    NotAnObject { line: usize },
    #[error("line {line}: no \"type\" and no \"model\"")]
    NoType { line: usize },
    #[error("line {line}: \"at\" is not a whole number of seconds")]
    BadTime { line: usize },
    #[error("line {line}: the tool adds the character line itself")]
    CharacterLine { line: usize },
    #[error("line {line}: a model answer needs \"model\" and \"answer\"")]
    BadAnswer { line: usize },
    #[error("line {line}: \"bench\" needs the name of a moment, once in a batch")]
    BadMoment { line: usize },
}

/// The built-in scenario of this name.
#[must_use]
pub fn built_in(name: &str) -> Option<&'static str> {
    BUILT_IN
        .iter()
        .find(|(known, _, _)| *known == name)
        .map(|(_, _, text)| *text)
}

impl Scenario {
    /// # Errors
    ///
    /// Returns the first line that breaks the format, with its number from 1.
    pub fn parse(text: &str) -> Result<Scenario, ScenarioError> {
        let mut batches = Vec::new();
        let mut batch = Batch::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if trimmed.is_empty() {
                end_batch(&mut batches, &mut batch);
                continue;
            }
            read_item(trimmed, line, &mut batch)?;
        }
        end_batch(&mut batches, &mut batch);
        Ok(Scenario { batches })
    }

    /// The time of the last line, in seconds after the start.
    #[must_use]
    pub fn span(&self) -> u64 {
        self.batches
            .iter()
            .flat_map(|batch| &batch.lines)
            .filter_map(|line| line.get("at").and_then(Value::as_u64))
            .max()
            .unwrap_or(0)
    }

    /// The same lines, with each `at` moved to start at `start`.
    #[must_use]
    pub fn starting_at(&self, start: u64) -> Scenario {
        let mut moved = self.clone();
        for line in moved.batches.iter_mut().flat_map(|batch| &mut batch.lines) {
            if let Some(at) = line.get("at").and_then(Value::as_u64) {
                line.insert("at".to_string(), Value::from(start + at));
            }
        }
        moved
    }
}

fn end_batch(batches: &mut Vec<Batch>, batch: &mut Batch) {
    if !batch.lines.is_empty() || !batch.answers.is_empty() || batch.moment.is_some() {
        batches.push(std::mem::take(batch));
    }
}

fn read_item(text: &str, line: usize, batch: &mut Batch) -> Result<(), ScenarioError> {
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
        return Err(ScenarioError::NotAnObject { line });
    };
    if let Some(moment) = object.get("bench") {
        let name = moment.as_str().filter(|name| !name.is_empty());
        let (Some(name), None) = (name, &batch.moment) else {
            return Err(ScenarioError::BadMoment { line });
        };
        batch.moment = Some(name.to_string());
        return Ok(());
    }
    if object.contains_key("model") {
        batch.answers.push(answer_of(&object, line)?);
        return Ok(());
    }
    match object.get("type").and_then(Value::as_str) {
        None => return Err(ScenarioError::NoType { line }),
        Some("character_entered") => return Err(ScenarioError::CharacterLine { line }),
        Some(_) => {}
    }
    if object.get("at").is_some_and(|at| at.as_u64().is_none()) {
        return Err(ScenarioError::BadTime { line });
    }
    batch.lines.push(object);
    Ok(())
}

fn answer_of(object: &Map<String, Value>, line: usize) -> Result<Answer, ScenarioError> {
    let kind = object.get("model").and_then(Value::as_str);
    let text = match object.get("answer") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(value @ Value::Object(_)) => Some(value.to_string()),
        _ => None,
    };
    match (kind, text) {
        (Some(kind), Some(text)) => Ok(Answer {
            kind: kind.to_string(),
            text,
        }),
        _ => Err(ScenarioError::BadAnswer { line }),
    }
}
