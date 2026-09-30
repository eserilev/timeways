//! Talk to an NPC (GAMEPLAY.md 3.5). The model plays the NPC and proposes a change of its
//! trust. The code checks both before anything shows or lands in the world (5.2).

use crate::check::{json_object, voice_text};
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::pack::Passage;
use crate::samples::{self, Voice};
use serde::Deserialize;
use std::fmt::Write;

/// About 70 words. The prompt asks for 60.
pub const MAX_SAY_CHARS: usize = 400;

/// The limit of the bridge for the words of a talk (Gnomish Relay SPEC.md 9.8).
pub const MAX_SAY_BYTES: usize = 1600;

/// The limit of the bridge for the NPC of a talk (Gnomish Relay SPEC.md 9.8).
pub const MAX_NPC_BYTES: usize = 64;

/// The largest change of trust that one talk proposes.
pub const MAX_TRUST_CHANGE: i64 = 5;

/// What the NPC knows about the player and its place.
#[derive(Debug, Default)]
pub struct Scene<'a> {
    pub npc: &'a str,
    pub place: Option<&'a str>,
    pub level: Option<i64>,
    pub trust: Option<i64>,
    pub slapped: Option<i64>,
    /// The entries of the player's own lore about this NPC or its place.
    pub own_lore: Vec<&'a str>,
}

/// The short persona of an NPC, from the facts alone. An NPC never gets the persona of the
/// narrator (GAMEPLAY.md 3.2.1).
#[must_use]
pub fn persona(npc: &str, place: Option<&str>) -> String {
    let place = place
        .map(|place| format!(" in {place}"))
        .unwrap_or_default();
    format!(
        "You are {npc}, a person of the world of Warcraft{place}. Speak as {npc} would: \
         plainly, in your own voice, and only of what a person of your place knows."
    )
}

/// `turn` picks the golden samples of the prompt.
#[must_use]
pub fn prompt(scene: &Scene<'_>, passages: &[Passage], words: &str, turn: usize) -> String {
    let npc = scene.npc;
    format!(
        "{}\n{HOUSE_RULES}\n\nAnswer the player, and say how this talk changes your trust. \
         Stay true to the lore below. When you do not know, say so as {npc} would.{}\n\n\
         {}\n\nThe player says:\n{}\n\n\
         Remember: you are {npc}. Speak plainly, in your own voice, in at most 60 words.\n\
         Reply with JSON only: {{\"say\": \"<your answer>\", \"trust\": <a whole number from \
         -{MAX_TRUST_CHANGE} to {MAX_TRUST_CHANGE}: how this talk changes your trust in the \
         player>}}",
        who_you_are(scene),
        what_you_know(scene, passages),
        samples::section(Voice::NpcReply, turn),
        fenced(words)
    )
}

fn who_you_are(scene: &Scene<'_>) -> String {
    let mut who = persona(scene.npc, scene.place);
    let _ = write!(who, " A player speaks to you. {}", trust_words(scene.trust));
    if let Some(slapped) = scene.slapped {
        let _ = write!(
            who,
            " The player slapped you {slapped} times, and you remember each one."
        );
    }
    who
}

/// The trust of the NPC in words, never as a number, with the bands of the People page.
fn trust_words(trust: Option<i64>) -> &'static str {
    match trust {
        None => "You do not know the player yet.",
        Some(50..) => "You trust the player.",
        Some(10..) => "You like the player.",
        Some(-9..) => "You have no strong feeling about the player.",
        Some(-49..) => "You are wary of the player.",
        Some(_) => "You distrust the player.",
    }
}

fn what_you_know(scene: &Scene<'_>, passages: &[Passage]) -> String {
    let mut known = String::new();
    if let Some(level) = scene.level {
        let _ = write!(known, "\n\nThe player is level {level}.");
    }
    if !passages.is_empty() {
        let texts: Vec<&str> = passages
            .iter()
            .map(|passage| passage.text.as_str())
            .collect();
        let _ = write!(
            known,
            "\n\nLore that you know:\n{}",
            fenced(&bulleted(&texts))
        );
    }
    if !scene.own_lore.is_empty() {
        let _ = write!(
            known,
            "\n\nWhat the player told of their own story, about you or this place. It is their \
             story, not canon:\n{}",
            fenced(&bulleted(&scene.own_lore))
        );
    }
    known
}

#[derive(Deserialize)]
struct Reply {
    say: String,
    trust: i64,
}

/// The words of the NPC, and the change of trust that passed the check.
#[derive(Debug, PartialEq, Eq)]
pub struct Answer {
    pub say: String,
    pub trust_change: i64,
}

/// None when the words break a rule. A change of trust outside the band is dropped, and
/// the words still show.
#[must_use]
pub fn checked_answer(text: &str) -> Option<Answer> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let say = voice_text(&reply.say, MAX_SAY_CHARS, MAX_SAY_BYTES)?;
    let in_band = (-MAX_TRUST_CHANGE..=MAX_TRUST_CHANGE).contains(&reply.trust);
    Some(Answer {
        say,
        trust_change: if in_band { reply.trust } else { 0 },
    })
}
