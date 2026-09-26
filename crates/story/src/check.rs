//! The checks on a model answer before it shows (GAMEPLAY.md 3.1 and 5.9, layer 4).

use std::fmt;

const LATER_NAMES: &str = include_str!("../data/later_names.txt");

/// About 150 words. The prompt asks for 80, so this catches only a runaway answer.
pub const MAX_CHARS: usize = 1000;

/// The words that mark an answer that no passage supports.
const NO_SOURCE: [&str; 2] = ["nobody knows", "legend says"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Empty,
    TooLong { chars: usize },
    NoCitation,
    UnknownCitation { number: usize },
    LaterName { name: String },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Empty => write!(f, "The answer is empty."),
            Fault::TooLong { chars } => {
                write!(
                    f,
                    "The answer has {chars} characters. The limit is {MAX_CHARS}."
                )
            }
            Fault::NoCitation => write!(
                f,
                "The answer cites no passage. Cite one, or say \"Nobody knows\" or \"Legend says\"."
            ),
            Fault::UnknownCitation { number } => write!(f, "No passage has the number [{number}]."),
            Fault::LaterName { name } => {
                write!(f, "\"{name}\" is from after the year 25 ADP. Leave it out.")
            }
        }
    }
}

/// Every fault at once, so one retry fixes all of them.
#[must_use]
pub fn check(answer: &str, passage_count: usize) -> Vec<Fault> {
    let answer = answer.trim();
    if answer.is_empty() {
        return vec![Fault::Empty];
    }
    let mut faults = Vec::new();
    let chars = answer.chars().count();
    if chars > MAX_CHARS {
        faults.push(Fault::TooLong { chars });
    }
    let cited = citations(answer);
    if cited.is_empty() && !admits_no_source(answer) {
        faults.push(Fault::NoCitation);
    }
    for number in cited {
        if number == 0 || number > passage_count {
            faults.push(Fault::UnknownCitation { number });
        }
    }
    for name in names_after_cutoff(answer) {
        faults.push(Fault::LaterName {
            name: name.to_string(),
        });
    }
    faults
}

/// The names of the cutoff list, as the data file holds them.
pub fn later_names() -> impl Iterator<Item = &'static str> {
    LATER_NAMES
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// The text on one line, or None when it is empty, longer than `max_chars`, holds a
/// control character, or names something from after the cutoff. For the short texts of
/// the companion, the bard, and a talk.
#[must_use]
pub fn plain_text(text: &str, max_chars: usize) -> Option<String> {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let too_long = line.chars().count() > max_chars;
    if line.is_empty() || too_long || line.chars().any(char::is_control) {
        return None;
    }
    if !names_after_cutoff(&line).is_empty() {
        return None;
    }
    Some(line)
}

/// The names of the cutoff list that the text holds, as whole words in any case.
#[must_use]
pub fn names_after_cutoff(answer: &str) -> Vec<&'static str> {
    let words = words_of(answer);
    later_names()
        .filter(|name| contains_phrase(&words, name))
        .collect()
}

fn contains_phrase(words: &[String], name: &str) -> bool {
    let phrase = words_of(name);
    !phrase.is_empty()
        && words
            .windows(phrase.len())
            .any(|window| window == phrase.as_slice())
}

fn words_of(text: &str) -> Vec<String> {
    words(text).map(str::to_lowercase).collect()
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

fn admits_no_source(answer: &str) -> bool {
    let answer = answer.to_lowercase();
    NO_SOURCE.iter().any(|marker| answer.contains(marker))
}

/// The numbers of each `[n]` and `[n, m]` in the text, in order. Brackets with anything
/// else inside are not citations.
fn citations(text: &str) -> Vec<usize> {
    let mut numbers = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find(']') else {
            break;
        };
        let parsed: Result<Vec<usize>, _> = rest[..close]
            .split(',')
            .map(|part| part.trim().parse())
            .collect();
        if let Ok(parsed) = parsed {
            numbers.extend(parsed);
        }
        rest = &rest[close + 1..];
    }
    numbers
}
