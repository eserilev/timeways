//! The narrator writes each finished chapter of the chronicle as a short saga (GAMEPLAY.md 3.3).
//! The words of its prompt live here, and the facts come from the chapter alone.

use crate::check::{json_object, voice_text};
use crate::hero::OWN_WORDS;
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::journal::{Chapter, Deed, Place};
use crate::memory;
use crate::narrator::PERSONA;
use crate::places::PlaceKind;
use crate::samples::{self, Voice};
use serde::Deserialize;
use std::fmt::Write;

/// About 100 words. The prompt asks for 80.
pub const MAX_CHAPTER_CHARS: usize = 600;

/// The limit of the bridge for one string of the journal (Gnomish Relay SPEC.md 9.8).
pub const MAX_CHAPTER_BYTES: usize = 1600;

/// A footnote is one dry line of fact.
pub const MAX_FOOTNOTE_CHARS: usize = 200;

/// "It picks at most 3 footnotes for the chapter" (GAMEPLAY.md 5.4.1).
pub const MAX_FOOTNOTES: usize = 3;

/// The limit of the bridge for one string of the journal is 1600 bytes. A footnote keeps
/// far below it, so a chapter with 3 of them still fits on one page.
const MAX_FOOTNOTE_BYTES: usize = 600;

const FOOTNOTES: &str = "\
Pick at most 3 of the small moments for footnotes, or none. A footnote is one short, dry \
line of fact, for example: \"On the fourth day, our hero danced in Goldshire, alone, at \
three in the morning.\"";

/// The author's note, with the format last.
const NOTE: &str = "\
Remember: serious, concrete, and sparing, in one paragraph. Say \"our hero\" at most \
twice, and tell nothing of what comes next.
Tell the facts as a story, not as a list. Few facts make a short chapter of two or three \
sentences.
Reply with JSON only: {\"saga\": \"<the chapter>\", \"footnotes\": [{\"moment\": <its number>, \
\"text\": \"<the footnote>\"}]}";

/// Which of the two drafts of a saga (GAMEPLAY.md 3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draft {
    First,
    Second,
}

impl Draft {
    /// The second draft carries the next samples in turn, so the two drafts differ.
    fn sample_turn(self, number: usize) -> usize {
        match self {
            Draft::First => number,
            Draft::Second => number + Voice::Chapter.per_prompt(),
        }
    }
}

/// The draft that the judge picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    First,
    Second,
}

const JUDGE_TASK: &str = "Below are two drafts of one chapter of the chronicle. Pick the \
better one.";

/// The author's note of the judge, with the format last.
const JUDGE_NOTE: &str = "\
The better draft tells only the facts, names them plainly, and keeps your manner: serious, \
concrete, and sparing. A draft that adds a deed, a place, or a person is worse.
Reply with JSON only: {\"pick\": 1} or {\"pick\": 2}";

/// The saga of a chapter, and its footnotes with the number of the moment of each one.
#[derive(Clone, Debug, PartialEq, Eq)]
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
struct JudgeReply {
    pick: u8,
}

#[derive(Deserialize)]
struct Footnote {
    moment: usize,
    text: String,
}

/// `earlier` are the chapters just before this one, oldest first. `moments` are the small
/// moments of the chapter in plain words, best first. `told` is what the player wrote in
/// the chapter. `places` tell the kind of each zone: a dungeon, a raid, or a capital.
#[must_use]
pub fn prompt(
    places: &[Place],
    chapter: &Chapter,
    earlier: &[Chapter],
    moments: &[String],
    portrait: Option<&str>,
    told: &[&str],
) -> String {
    draft_prompt(
        places,
        chapter,
        earlier,
        moments,
        portrait,
        told,
        Draft::First,
    )
}

/// The prompt of one draft. Only the samples differ between the two drafts.
#[must_use]
pub fn draft_prompt(
    places: &[Place],
    chapter: &Chapter,
    earlier: &[Chapter],
    moments: &[String],
    portrait: Option<&str>,
    told: &[&str],
    draft: Draft,
) -> String {
    let number = chapter.number;
    let mut prompt = format!(
        "{PERSONA}\n{HOUSE_RULES}\n\nWrite chapter {number} of the chronicle, in at most 80 \
         words, from the facts below and from nothing else."
    );
    prompt.push_str(&what_came_before(earlier));
    let _ = write!(
        prompt,
        "\n\nThe facts of chapter {number}:\n{}",
        fenced(&facts(places, chapter))
    );
    prompt.push_str(&small_moments(moments));
    prompt.push_str(&own_words(portrait, told));
    let samples = samples::section(Voice::Chapter, draft.sample_turn(number));
    let _ = write!(prompt, "\n\n{samples}\n\n{NOTE}");
    prompt
}

/// Chapter memory, so the saga knows the road so far. Empty for the first chapter.
fn what_came_before(earlier: &[Chapter]) -> String {
    if earlier.is_empty() {
        return String::new();
    }
    let summaries: Vec<String> = earlier.iter().map(memory::summary).collect();
    let summaries: Vec<&str> = summaries.iter().map(String::as_str).collect();
    format!(
        "\n\nWhat came before, as the chronicle holds it. Do not tell it again:\n{}",
        fenced(&bulleted(&summaries))
    )
}

fn small_moments(moments: &[String]) -> String {
    if moments.is_empty() {
        return String::new();
    }
    format!(
        "\n\nSmall moments:\n{}\n{FOOTNOTES}",
        fenced(&numbered(moments))
    )
}

fn own_words(portrait: Option<&str>, told: &[&str]) -> String {
    let mut words = String::new();
    if let Some(portrait) = portrait {
        let _ = write!(words, "\n\n{OWN_WORDS}\n{}", fenced(portrait));
    }
    if !told.is_empty() {
        let _ = write!(
            words,
            "\n\nWhat the player wrote in this chapter:\n{}",
            fenced(&bulleted(told))
        );
    }
    words
}

/// The prompt of the judge: the facts of chapter `number`, and the saga text of each draft.
#[must_use]
pub fn judge_prompt(number: usize, facts: &str, first: &str, second: &str) -> String {
    format!(
        "{PERSONA}\n{HOUSE_RULES}\n\n{JUDGE_TASK}\n\nThe facts of chapter {number}:\n{}\n\n\
         Draft 1:\n{}\n\nDraft 2:\n{}\n\n{JUDGE_NOTE}",
        fenced(facts),
        fenced(first),
        fenced(second)
    )
}

/// A bad answer picks the first draft: both drafts passed every check.
#[must_use]
pub fn checked_pick(text: &str) -> Pick {
    let reply = json_object(text).and_then(|json| serde_json::from_str::<JudgeReply>(json).ok());
    match reply {
        Some(JudgeReply { pick: 2 }) => Pick::Second,
        _ => Pick::First,
    }
}

/// The facts of a chapter, one on each line, as its prompts show them.
#[must_use]
pub fn facts(places: &[Place], chapter: &Chapter) -> String {
    let mut facts = Vec::new();
    if !chapter.zones.is_empty() {
        let zones: Vec<String> = chapter
            .zones
            .iter()
            .map(|zone| described(places, zone))
            .collect();
        facts.push(format!("- Traveled to: {}.", zones.join(", ")));
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
/// its plain list, and gets no retry. `player_text` is the hero in the player's own words.
#[must_use]
pub fn checked_saga(text: &str, moment_count: usize, player_text: &str) -> Option<Saga> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let saga = voice_text(
        &reply.saga,
        MAX_CHAPTER_CHARS,
        MAX_CHAPTER_BYTES,
        player_text,
    )?;
    let mut footnotes: Vec<(usize, String)> = Vec::new();
    for footnote in reply.footnotes {
        let known = (1..=moment_count).contains(&footnote.moment);
        let new = footnotes
            .iter()
            .all(|(moment, _)| *moment != footnote.moment);
        let text = voice_text(
            &footnote.text,
            MAX_FOOTNOTE_CHARS,
            MAX_FOOTNOTE_BYTES,
            player_text,
        );
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

fn described(places: &[Place], zone: &str) -> String {
    let kind = places
        .iter()
        .find(|place| place.name == zone && place.within.is_none())
        .map_or(PlaceKind::Zone, |place| place.kind);
    kind.described(zone)
}

fn deed_fact(deed: &Deed) -> String {
    match deed {
        Deed::Level { from: None, to, .. } => format!("Began the saga at level {to}"),
        Deed::Level { to, .. } => format!("Reached level {to}"),
        Deed::Defeated { foe, times: 1, .. } => format!("Defeated {foe} for the first time"),
        Deed::Defeated { foe, times, .. } => format!("Defeated {foe} again, {times} times in all"),
        Deed::Titled { title, .. } => format!("Earned the title \"{title}\""),
        Deed::QuestDone { title, .. } => format!("Finished the quest \"{title}\""),
        Deed::GameQuestDone { title, .. } => format!("Finished the quest \"{title}\""),
        Deed::ClassQuestDone { title, .. } => format!("Finished the class quest \"{title}\""),
        Deed::QuestMarked { mark, quest, .. } => {
            format!("Gained the lasting effect \"{mark}\" during the quest \"{quest}\"")
        }
        Deed::Died {
            killer: Some(killer),
            ..
        } => format!("Fell to {killer}"),
        Deed::Died { killer: None, .. } => "Died".to_string(),
    }
}
