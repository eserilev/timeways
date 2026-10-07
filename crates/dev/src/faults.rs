//! Why the story program refused an answer of a model, in short names for a bench report:
//! "slop", "copy", "cutoff", "arrival", "inside-hero", "json-shape", and so on.
//!
//! A refused first answer gets a retry whose prompt holds the real reasons of the story
//! program, so those come first. An answer with no retry gets a check of its own text alone.
//! That check misses the faults that need the moment, such as "ungrounded", and names them
//! "other".

use serde_json::Value;
use timeways_story::check::json_object;
use timeways_story::line_check::SILENCE;
use timeways_story::narrated::{Limits, checked};

/// `prompt::retry` puts this after the first prompt.
pub const RETRY_MARK: &str = "\n\nYour last answer was:\n";
const RULES_MARK: &str = "It broke these rules:\n";

/// The short name for each reason, by a phrase of its text. The first match wins, so a
/// longer phrase comes before a shorter one that it holds.
const REASON_NAMES: &[(&str, &str)] = &[
    ("not one JSON object", "json-shape"),
    ("It must be JSON", "json-shape"),
    ("is empty, or longer than", "unreadable"),
    ("must be one paragraph", "unreadable"),
    ("The answer is empty", "unreadable"),
    ("after the year 25 ADP", "later-name"),
    ("after the time of the story", "later-name"),
    ("has an emoji", "emoji"),
    ("emoji or a word that the voice never uses", "slop"),
    ("is empty or invented", "slop"),
    ("which the facts do not hold", "slop"),
    ("our hero", "slop"),
    ("copies", "copy"),
    ("repeats the player's own story", "copy"),
    ("is not in the moment", "new-number"),
    ("more than one number", "ledger"),
    ("names nothing of the moment", "ungrounded"),
    ("has a bracket", "bracket"),
    ("names the hero twice", "named-twice"),
    ("names the hero more than", "named-twice"),
    ("only tells that the hero came", "arrival"),
    ("tell the place alone", "hero-at-a-place"),
    ("The history names the hero", "hero-at-a-place"),
    ("puts a power inside the hero", "inside-hero"),
    ("has the world know the hero", "recognition"),
    ("who spoke of the lore", "source-shape"),
    ("is a fragment", "fragment"),
    ("words. Keep each", "long-sentence"),
    ("\"not X, but Y\" turn", "pivot"),
    ("opens with the level", "level-opener"),
    ("sentences. ", "too-many-sentences"),
    ("longer than", "too-long"),
    ("did not offer", "unknown-choice"),
    ("says that the history names it", "choice-not-in-lore"),
    ("cites no passage", "no-citation"),
    ("No passage has the number", "no-citation"),
];

/// The short name of a reason of the story program, or "other".
#[must_use]
pub fn name_of_reason(reason: &str) -> &'static str {
    REASON_NAMES
        .iter()
        .find(|(phrase, _)| reason.contains(phrase))
        .map_or("other", |(_, name)| name)
}

#[must_use]
pub fn is_retry(prompt: &str) -> bool {
    prompt.contains(RETRY_MARK)
}

/// The reasons that a retry prompt gives for the answer before it.
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

/// The answer without a fence of Markdown around it.
fn without_fence(answer: &str) -> &str {
    let trimmed = answer.trim();
    let Some(inner) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let inner = inner.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    inner.strip_suffix("```").unwrap_or(inner).trim()
}

/// The field of a JSON answer that holds its prose, by kind of call. The other fields are
/// choices, numbers, or footnotes.
const TEXT_FIELDS: [&str; 6] = ["lore", "saga", "tale", "summary", "history", "say"];

/// The words that a player would read: the prose field of a JSON answer, or the answer. A
/// JSON answer with no prose, such as the pick of a judge, stays as it is.
#[must_use]
pub fn main_text(answer: &str) -> String {
    let body = without_fence(answer);
    let object = json_object(body).and_then(|json| serde_json::from_str::<Value>(json).ok());
    let prose = object.as_ref().and_then(|object| {
        TEXT_FIELDS
            .iter()
            .find_map(|field| object.get(field).and_then(Value::as_str))
    });
    prose.unwrap_or(body).to_string()
}

/// SILENCE alone, or a JSON answer whose lore is SILENCE: the model had nothing to tell.
#[must_use]
pub fn is_silence(answer: &str) -> bool {
    let body = without_fence(answer);
    let word = |text: &str| {
        text.trim()
            .trim_end_matches('.')
            .eq_ignore_ascii_case(SILENCE)
    };
    if word(body) {
        return true;
    }
    json_object(body)
        .and_then(|json| serde_json::from_str::<Value>(json).ok())
        .and_then(|value| value.get("lore").and_then(Value::as_str).map(word))
        .unwrap_or(false)
}

/// The names of the faults that the text of an answer shows alone. `wants_json` is true
/// when its prompt asks for JSON. An answer that opens a JSON object wants it too.
#[must_use]
pub fn faults_of_answer(answer: &str, wants_json: bool) -> Vec<&'static str> {
    let body = without_fence(answer);
    if body.is_empty() {
        return vec!["unreadable"];
    }
    if (wants_json || body.starts_with('{')) && json_object(body).is_none() {
        let cut = body.contains('{') && !body.contains('}');
        return vec![if cut { "cutoff" } else { "json-shape" }];
    }
    let text = main_text(answer);
    let limits = Limits {
        max_chars: 4000,
        max_bytes: 16_000,
        told: "",
        player_text: "",
        not_copied: &[],
    };
    let mut names: Vec<&'static str> = match checked(&text, &limits) {
        Ok(_) => Vec::new(),
        Err(reasons) => reasons
            .iter()
            .map(|reason| name_of_reason(reason))
            .collect(),
    };
    if !ends_a_sentence(&text) {
        names.push("cutoff");
    }
    names.sort_unstable();
    names.dedup();
    if names.is_empty() {
        names.push("other");
    }
    names
}

fn ends_a_sentence(text: &str) -> bool {
    text.trim_end()
        .ends_with(['.', '!', '?', '"', '\'', '\u{201d}', '\u{2019}', ')'])
}
