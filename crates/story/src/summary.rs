//! The summary of the character on the title page of the Chronicle (docs/plans/hero-stories.md
//! 3.5): one paragraph, in the voice of the narrator, on who the character has become. It
//! reads the sheet, the sagas, the deeds, the level, the race, and the class. It reads no
//! story and no standing.

use crate::arrival::arrival_in;
use crate::check::{json_object, slop_in, voice_text};
use crate::hero::OWN_WORDS;
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::narrator::PERSONA;
use crate::tokens::{Call, largest_fit};
use serde::Deserialize;
use std::fmt::Write;

/// About 100 words. The prompt asks for 80.
pub const MAX_SUMMARY_CHARS: usize = 600;

/// The limit of the bridge for one string of the journal (Gnomish Relay SPEC.md 9.8).
pub const MAX_SUMMARY_BYTES: usize = 1600;

/// The sagas of the newest chapters that a prompt can hold.
pub const MAX_SAGAS: usize = 3;

/// The deeds of note that a prompt holds.
pub const MAX_DEEDS: usize = 10;

/// The hero's name, `$N`, comes at most this often.
pub const MAX_NAMES: usize = 2;

const TASK: &str = "Write who this character has become, in one paragraph of at most 80 \
words.";

/// The author's note, with the format last.
const NOTE: &str = "\
Remember: serious, concrete, and sparing. Tell who the hero has become from the facts \
below, and tell nothing of what comes next.
Write $N for the name of the hero, at most twice: the game puts the name there. Else say \
\"they\", name the race or the class, or name no one. Never write \"our hero\". No \
markdown.
Reply with JSON only: {\"summary\": \"<the paragraph>\"}";

/// What the prompt of a summary tells, each part in plain words.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// "a Forsaken warlock", or None before the addon tells.
    pub who: Option<String>,
    pub level: Option<i64>,
    /// The six answers of the sheet, each cut short (`hero::portrait`).
    pub sheet: Option<String>,
    /// The summary before this one.
    pub before: Option<String>,
    /// The saga of each of the newest chapters, newest first. A chapter with no saga gives
    /// its plain list (`memory::summary`).
    pub chapters: Vec<String>,
    /// The deeds of note, newest first.
    pub deeds: Vec<String>,
}

#[derive(Deserialize)]
struct Reply {
    summary: String,
}

fn hero_line(facts: &Facts) -> String {
    let who = facts
        .who
        .as_deref()
        .unwrap_or("a hero of unknown race and class");
    match facts.level {
        Some(level) => format!("The hero: {who}, level {level}."),
        None => format!("The hero: {who}."),
    }
}

/// The prompt with the newest `count` chapters.
fn prompt_with(facts: &Facts, count: usize) -> String {
    let mut prompt = format!("{PERSONA}\n{HOUSE_RULES}\n\n{TASK}\n\n{}", hero_line(facts));
    if let Some(sheet) = &facts.sheet {
        let _ = write!(prompt, "\n\n{OWN_WORDS}\n{}", fenced(sheet));
    }
    if let Some(before) = &facts.before {
        let _ = write!(
            prompt,
            "\n\nWho the hero was, as the chronicle told it before. Let the new paragraph \
             grow from it:\n{}",
            fenced(before)
        );
    }
    let chapters: Vec<&str> = facts
        .chapters
        .iter()
        .take(count)
        .map(String::as_str)
        .collect();
    if !chapters.is_empty() {
        let _ = write!(
            prompt,
            "\n\nThe newest chapters of the chronicle, newest first:\n{}",
            fenced(&bulleted(&chapters))
        );
    }
    if !facts.deeds.is_empty() {
        let deeds: Vec<&str> = facts.deeds.iter().map(String::as_str).collect();
        let _ = write!(
            prompt,
            "\n\nDeeds of note, newest first:\n{}",
            fenced(&bulleted(&deeds))
        );
    }
    let _ = write!(prompt, "\n\n{NOTE}");
    prompt
}

/// The prompt keeps as many of the newest chapters as fit its budget, at most 3.
#[must_use]
pub fn prompt(facts: &Facts) -> String {
    let most = facts.chapters.len().min(MAX_SAGAS);
    largest_fit(most, Call::Summary.prompt_budget(), |count| {
        prompt_with(facts, count)
    })
}

/// Every fact of the prompt on its own line, for the check of slop: a slop word that the
/// facts hold, such as a name, stays allowed.
#[must_use]
pub fn told(facts: &Facts) -> String {
    let parts = [facts.sheet.clone(), facts.before.clone()];
    let mut lines: Vec<String> = parts.into_iter().flatten().collect();
    lines.extend(facts.chapters.iter().cloned());
    lines.extend(facts.deeds.iter().cloned());
    lines.join("\n")
}

/// The summary as the player reads it, or None when it breaks a rule. `told` is the text of
/// the facts (`told`), and `player_text` the hero in the player's own words.
#[must_use]
pub fn checked_summary(text: &str, told: &str, player_text: &str) -> Option<String> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let summary = voice_text(
        &reply.summary,
        MAX_SUMMARY_CHARS,
        MAX_SUMMARY_BYTES,
        player_text,
    )?;
    let names = summary.matches("$N").count();
    let our_hero = summary.to_lowercase().contains("our hero");
    let clean = slop_in(&summary, told).is_empty() && arrival_in(&summary, &[]).is_none();
    (names <= MAX_NAMES && !our_hero && clean).then_some(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(summary: &str) -> String {
        serde_json::json!({ "summary": summary }).to_string()
    }

    #[test]
    fn a_plain_paragraph_passes() {
        let text = reply("Deathknell buried its dead twice, and $N climbed back out.");

        assert!(checked_summary(&text, "", "").is_some());
    }

    #[test]
    fn a_summary_that_names_the_hero_three_times_is_refused() {
        let text = reply("$N rose. $N fought. $N stayed.");

        assert_eq!(checked_summary(&text, "", ""), None);
    }

    #[test]
    fn a_summary_that_says_our_hero_is_refused() {
        let text = reply("Our hero walked to Brill.");

        assert_eq!(checked_summary(&text, "", ""), None);
    }

    #[test]
    fn a_summary_over_six_hundred_characters_is_refused() {
        let text = reply(&"word ".repeat(121));

        assert_eq!(checked_summary(&text, "", ""), None);
    }

    #[test]
    fn a_reply_that_is_no_json_is_refused() {
        assert_eq!(checked_summary("The hero walked on.", "", ""), None);
    }
}
