//! The words of a lore prompt (GAMEPLAY.md 3.1 and 5.3). Timeways writes them, never
//! Hourglass.

use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::pack::{Origin, Passage};
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

/// The model wrote the answer, and a reason can quote a word of it, so both are data.
#[must_use]
pub fn retry(prompt: &str, answer: &str, reasons: &[String]) -> String {
    let faults: Vec<&str> = reasons.iter().map(String::as_str).collect();
    format!(
        "{prompt}\n\nYour last answer was:\n{}\n\nIt broke these rules:\n{}\nWrite the answer again.",
        fenced(answer),
        fenced(&bulleted(&faults))
    )
}
