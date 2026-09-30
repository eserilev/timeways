//! The narrator writes each finished chapter of the chronicle as a short saga (GAMEPLAY.md 3.3).
//! The words of its prompt live here, and the facts come from the chapter alone.

use crate::check::{json_object, voice_text};
use crate::hero::OWN_WORDS;
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::journal::{Chapter, Deed};
use crate::narrator::PERSONA;
use serde::Deserialize;
use std::fmt::Write;

/// About 100 words. The prompt asks for 80.
pub const MAX_CHAPTER_CHARS: usize = 600;

/// The limit of the bridge for one string of the journal (Gnomish Relay SPEC.md 9.8).
pub const MAX_CHAPTER_BYTES: usize = 1600;

/// A footnote is one dry line: "Nobody knows why."
pub const MAX_FOOTNOTE_CHARS: usize = 200;

/// "It picks at most 3 footnotes for the chapter" (GAMEPLAY.md 5.4.1).
pub const MAX_FOOTNOTES: usize = 3;

/// The limit of the bridge for one string of the journal is 1600 bytes. A footnote keeps
/// far below it, so a chapter with 3 of them still fits on one page.
const MAX_FOOTNOTE_BYTES: usize = 600;

const FOOTNOTES: &str = "\
Pick at most 3 of the small moments for footnotes, or none. A footnote is one short, dry \
line, for example: \"On the fourth day, our hero danced in Goldshire. Nobody knows why.\"";

/// The author's note, with the format last.
const NOTE: &str = "\
Remember: serious, concrete, and sparing, in one paragraph. Call the player \"our hero\", \
and tell nothing of what comes next.
Reply with JSON only: {\"saga\": \"<the chapter>\", \"footnotes\": [{\"moment\": <its number>, \
\"text\": \"<the footnote>\"}]}";

/// The saga of a chapter, and its footnotes with the number of the moment of each one.
#[derive(Debug, PartialEq, Eq)]
pub struct Saga {
    pub text: String,
    pub footnotes: Vec<(usize, String)>,
}

#[derive(Deserialize)]
struct Reply {
    saga: String,
    #[serde(default)]
    footnotes: Vec<Footnote>,
}

#[derive(Deserialize)]
struct Footnote {
    moment: usize,
    text: String,
}

/// `moments` are the small moments of the chapter in plain words, best first.
#[must_use]
pub fn prompt(
    chapter: &Chapter,
    moments: &[String],
    portrait: Option<&str>,
    told: &[&str],
) -> String {
    let number = chapter.number;
    let mut prompt = format!(
        "{PERSONA}\n{HOUSE_RULES}\n\nWrite chapter {number} of the chronicle, in at most 80 \
         words, from the facts below and from nothing else.\n\nThe facts of chapter {number}:\n{}",
        fenced(&facts(chapter))
    );
    if !moments.is_empty() {
        let _ = write!(
            prompt,
            "\n\nSmall moments:\n{}\n{FOOTNOTES}",
            fenced(&numbered(moments))
        );
    }
    if let Some(portrait) = portrait {
        let _ = write!(prompt, "\n\n{OWN_WORDS}\n{}", fenced(portrait));
    }
    if !told.is_empty() {
        let _ = write!(
            prompt,
            "\n\nWhat the player wrote in this chapter:\n{}",
            fenced(&bulleted(told))
        );
    }
    let _ = write!(prompt, "\n\n{NOTE}");
    prompt
}

fn facts(chapter: &Chapter) -> String {
    let mut facts = Vec::new();
    if !chapter.zones.is_empty() {
        facts.push(format!("- Traveled to: {}.", chapter.zones.join(", ")));
    }
    if !chapter.people.is_empty() {
        facts.push(format!("- Met: {}.", chapter.people.join(", ")));
    }
    for deed in &chapter.deeds {
        facts.push(format!("- {}.", deed_fact(deed)));
    }
    facts.join("\n")
}

fn numbered(moments: &[String]) -> String {
    let lines: Vec<String> = moments
        .iter()
        .enumerate()
        .map(|(index, moment)| format!("{}. {moment}", index + 1))
        .collect();
    lines.join("\n")
}

/// The saga as the player reads it, or None when it breaks a rule. A footnote that breaks
/// a rule, or names no moment of the list, is dropped alone. A chapter that fails keeps
/// its plain list, and gets no retry.
#[must_use]
pub fn checked_saga(text: &str, moment_count: usize) -> Option<Saga> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let saga = voice_text(&reply.saga, MAX_CHAPTER_CHARS, MAX_CHAPTER_BYTES)?;
    let mut footnotes: Vec<(usize, String)> = Vec::new();
    for footnote in reply.footnotes {
        let known = (1..=moment_count).contains(&footnote.moment);
        let new = footnotes
            .iter()
            .all(|(moment, _)| *moment != footnote.moment);
        let text = voice_text(&footnote.text, MAX_FOOTNOTE_CHARS, MAX_FOOTNOTE_BYTES);
        if let (true, true, Some(text)) = (known, new, text) {
            footnotes.push((footnote.moment, text));
        }
    }
    footnotes.truncate(MAX_FOOTNOTES);
    Some(Saga {
        text: saga,
        footnotes,
    })
}

fn deed_fact(deed: &Deed) -> String {
    match deed {
        Deed::Level { from: None, to, .. } => format!("Began the saga at level {to}"),
        Deed::Level { to, .. } => format!("Reached level {to}"),
        Deed::Defeated { foe, times: 1, .. } => format!("Defeated {foe} for the first time"),
        Deed::Defeated { foe, times, .. } => format!("Defeated {foe} again, {times} times in all"),
        Deed::Titled { title, .. } => format!("Earned the title \"{title}\""),
        Deed::QuestDone { title, .. } => format!("Finished the task \"{title}\""),
        Deed::Died {
            killer: Some(killer),
            ..
        } => format!("Fell to {killer}"),
        Deed::Died { killer: None, .. } => "Died".to_string(),
    }
}
