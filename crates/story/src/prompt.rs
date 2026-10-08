//! The words of a lore prompt (GAMEPLAY.md 3.1 and 5.3). Timeways writes them, never
//! Hourglass.

use crate::house::{HOUSE_RULES, bulleted, fenced, first_chars};
use crate::pack::{Origin, Passage};
use crate::tokens::estimated_tokens;
use std::fmt::Write;

const RULES: &str = "\
You are a historian in the world of Warcraft.
A player asks you a question. Answer it from the numbered passages below, and from nothing else.

Rules:
- Cite the passage of each claim with its number, for example [1].
- A passage marked \"The player learned this\" is what the player read or heard in the game. \
Tell them where they learned it, for example \"You read in the book of the old tower that...\".
- If the passages do not answer the question, say \"Nobody knows\" or \"Legend says\".
- Answer in at most 80 words, in plain text, in the voice of a historian.";

/// What the player sees and is, without typing it.
#[derive(Debug, Default)]
pub struct Context<'a> {
    /// Where the player stands, then each place around it.
    pub places: Vec<&'a str>,
    pub target: Option<&'a str>,
    pub level: Option<i64>,
}

#[must_use]
pub fn lore(question: &str, context: &Context<'_>, passages: &[Passage]) -> String {
    let mut prompt = format!("{RULES}\n{HOUSE_RULES}\n\n");
    if let Some(around) = around(context) {
        let _ = writeln!(prompt, "Where the player is:\n{}", fenced(&around));
    }
    if let Some(level) = context.level {
        let _ = writeln!(prompt, "The player is level {level}.");
    }
    let _ = write!(
        prompt,
        "\nPassages:\n{}\n\nQuestion:\n{}",
        fenced(&numbered(passages)),
        fenced(question)
    );
    prompt
}

/// The names of the places and of the target come from the addon, so they are data.
fn around(context: &Context<'_>) -> Option<String> {
    let mut lines = Vec::new();
    if !context.places.is_empty() {
        lines.push(format!(
            "The player stands in: {}",
            context.places.join(", ")
        ));
    }
    if let Some(target) = context.target {
        lines.push(format!("The player looks at: {target}"));
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn numbered(passages: &[Passage]) -> String {
    let mut lines = String::new();
    for (index, passage) in passages.iter().enumerate() {
        let number = index + 1;
        let _ = match passage.origin {
            Origin::Pack => writeln!(lines, "[{number}] {}", passage.text),
            Origin::Read => writeln!(
                lines,
                "[{number}] (The player learned this from {}.) {}",
                passage.source, passage.text
            ),
        };
    }
    lines.trim_end().to_string()
}

/// A call that a check refuses gets one retry. A second refusal is final.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt {
    First,
    Retry,
}

/// A retry quotes at most this much of the last answer. The faults say what to fix, so the
/// start of a long answer is enough.
const RETRY_ANSWER_CHARS: usize = 400;

/// A fault can quote a name of the answer, so a retry cuts each one to this length.
const RETRY_FAULT_CHARS: usize = 100;

/// A retry names at most this many faults.
const RETRY_FAULTS: usize = 3;

/// The most tokens that a retry adds to its first prompt. A first prompt that leaves this
/// much room keeps its retry in the same budget.
#[must_use]
pub fn retry_tokens() -> usize {
    let longest_fault = "w".repeat(RETRY_FAULT_CHARS);
    let faults = vec![longest_fault; RETRY_FAULTS];
    estimated_tokens(&retry("", &"w".repeat(RETRY_ANSWER_CHARS), &faults))
}

/// A retry puts this after the first prompt, then the last answer.
pub const RETRY_MARK: &str = "\n\nYour last answer was:\n";

/// What opens the reasons of a retry, after the last answer.
const RULES_MARK: &str = "It broke these rules:\n";

/// The model wrote the answer, and a reason can quote a word of it, so both are data.
#[must_use]
pub fn retry(prompt: &str, answer: &str, reasons: &[String]) -> String {
    let faults: Vec<&str> = reasons
        .iter()
        .take(RETRY_FAULTS)
        .map(|reason| first_chars(reason, RETRY_FAULT_CHARS))
        .collect();
    let answer = first_chars(answer, RETRY_ANSWER_CHARS);
    format!(
        "{prompt}{RETRY_MARK}{}\n\n{RULES_MARK}{}\nWrite the answer again.",
        fenced(answer),
        fenced(&bulleted(&faults))
    )
}

/// The reasons that a retry prompt gives for the answer before it: the inverse of `retry`.
#[must_use]
pub fn reasons_of_retry(prompt: &str) -> Vec<String> {
    let Some(at) = prompt.rfind(RULES_MARK) else {
        return Vec::new();
    };
    prompt[at + RULES_MARK.len()..]
        .lines()
        .take_while(|line| !line.starts_with(">>>"))
        .filter_map(|line| line.strip_prefix("- "))
        .map(str::to_string)
        .collect()
}
