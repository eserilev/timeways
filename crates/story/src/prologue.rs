//! The prologue of the Chronicle (GAMEPLAY.md 3.3): chapter 0, one paragraph of the
//! narrator about the lands and the people of a character whose past came before
//! Timeways. The words of its prompt and its checks live here.

use crate::arrival::arrival_in;
use crate::character::Character;
use crate::check::{json_object, mentions, slop_in, voice_text};
use crate::grounding::ungrounded_names;
use crate::house::{HOUSE_RULES, bulleted, fenced};
use crate::narrator::{PERSONA, Who, lore_excerpt};
use crate::narrator_lore::people_pages;
use crate::pack::{Link, Pack, PackError, Passage};
use crate::passage_limits;
use crate::past::Past;
use crate::prose::prose_faults;
use crate::samples::{self, Voice};
use crate::tokens::{Call, largest_fit};
use serde::Deserialize;
use std::fmt::Write;

/// About 100 words, as a chapter. The prompt asks for 80.
pub const MAX_PROLOGUE_CHARS: usize = 600;

/// The limit of the bridge for one string of the journal (Gnomish Relay SPEC.md 9.8).
pub const MAX_PROLOGUE_BYTES: usize = 1600;

/// The lore passages that a prompt can hold.
pub const MAX_LORE: usize = 3;

/// The hero's name, `$N`, comes at most this often.
pub const MAX_NAMES: usize = 2;

/// Enough candidates that the gate still leaves one passage of the subject.
const CANDIDATES: u32 = 10;

/// A name word of the facts or the lore needs this many letters to ground the prologue.
const NAME_LETTERS: usize = 4;

/// The title of the prologue in the journal.
pub const TITLE: &str = "Prologue";

const TASK: &str = "Write the prologue of the chronicle, in at most 80 words: the history of \
the lands and the people of this hero, up to the day the chronicle begins. Use the facts \
and the lore below, and nothing else.";

/// The author's note, with the format last.
const NOTE: &str = "\
Remember: the world is the subject. Tell the history of these places and peoples, and end \
on what holds there now. Tell nothing of what comes next.
Name the hero only as the doer of a deed of the facts, at most twice, as $N: the game puts \
the name there. Else name no one. Never write \"our hero\". Count nothing: no quests, no \
hours, and no levels.
Reply with JSON only: {\"prologue\": \"<the paragraph>\"}";

/// What the prompt of a prologue tells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// "a human paladin", or None before the addon tells.
    pub who: Option<String>,
    pub level: u8,
    /// The facts of the past, one on each line (`Past::facts`).
    pub past: Vec<String>,
    /// The lore of the places and the people, each passage cut short.
    pub lore: Vec<String>,
}

#[derive(Deserialize)]
struct Reply {
    prologue: String,
}

fn hero_line(facts: &Facts) -> String {
    let who = facts
        .who
        .as_deref()
        .unwrap_or("a hero of unknown race and class");
    format!("The hero: {who}, level {}.", facts.level)
}

fn prompt_with(facts: &Facts, lore: usize) -> String {
    let mut prompt = format!("{PERSONA}\n{HOUSE_RULES}\n\n{TASK}\n\n{}", hero_line(facts));
    if !facts.past.is_empty() {
        let past: Vec<&str> = facts.past.iter().map(String::as_str).collect();
        let _ = write!(
            prompt,
            "\n\nThe past of the hero, before the chronicle:\n{}",
            fenced(&bulleted(&past))
        );
    }
    let passages: Vec<&str> = facts.lore.iter().take(lore).map(String::as_str).collect();
    if passages.is_empty() {
        prompt.push_str("\n\nThe lore: none.");
    } else {
        let _ = write!(prompt, "\n\nThe lore:\n{}", fenced(&bulleted(&passages)));
    }
    let samples = samples::section(Voice::Chapter, 0);
    let _ = write!(prompt, "\n\n{samples}\n\n{NOTE}");
    prompt
}

/// The prompt keeps as many passages of lore as fit its budget, at most 3.
#[must_use]
pub fn prompt(facts: &Facts) -> String {
    let most = facts.lore.len().min(MAX_LORE);
    largest_fit(most, Call::Prologue.prompt_budget(), |count| {
        prompt_with(facts, count)
    })
}

/// What the prompt tells: the hero, the facts of the past, and the lore of its people and
/// its places.
///
/// # Errors
///
/// Returns the error of the pack.
pub fn facts_of(pack: &Pack, character: &Character, past: &Past) -> Result<Facts, PackError> {
    Ok(Facts {
        who: Who::of(character).described(),
        level: past.level,
        past: past.facts(),
        lore: lore_of(pack, character, past)?,
    })
}

/// The first passage of the people of the hero, then of each place of the past, at most
/// 3 and each once.
fn lore_of(pack: &Pack, character: &Character, past: &Past) -> Result<Vec<String>, PackError> {
    let people = character.race().map(people_pages).unwrap_or_default();
    let subjects = people.iter().copied().chain(past.places());
    let mut lore: Vec<String> = Vec::new();
    for subject in subjects {
        if lore.len() >= MAX_LORE {
            break;
        }
        let found = pack.about(subject, CANDIDATES)?;
        let first = found
            .into_iter()
            .filter(|passage| may_tell(character, past, passage))
            .find_map(passage_limits::fitted);
        let Some(passage) = first else {
            continue;
        };
        let excerpt = lore_excerpt(&passage.text);
        if !lore.contains(&excerpt) {
            lore.push(excerpt);
        }
    }
    Ok(lore)
}

/// A passage passes when the character knows each of its places, from the world or from
/// the past, and it tells no deed and sets up none.
fn may_tell(character: &Character, past: &Past, passage: &Passage) -> bool {
    let places = past.places();
    let known = passage.links.iter().all(|link| match link {
        Link::Place(name) => places.contains(&name.as_str()) || character.has_visited(name),
        Link::Npc(name) => character.has_met(name),
        Link::Common => true,
    });
    known && passage.depends_on.is_empty() && passage.setup_for.is_none()
}

/// Every fact and passage of the prompt on its own line, for the checks of slop and
/// grounds.
#[must_use]
pub fn told(facts: &Facts) -> String {
    let mut lines: Vec<String> = facts.who.iter().cloned().collect();
    lines.extend(facts.past.iter().cloned());
    lines.extend(facts.lore.iter().cloned());
    lines.join("\n")
}

/// The prologue as the player reads it, or None when it breaks a rule. It takes the checks
/// of a saga, and it must name a place, a people, or a person of its facts or its lore.
#[must_use]
pub fn checked_prologue(text: &str, told: &str, player_text: &str, given: &str) -> Option<String> {
    let reply: Reply = serde_json::from_str(json_object(text)?).ok()?;
    let prologue = voice_text(
        &reply.prologue,
        MAX_PROLOGUE_CHARS,
        MAX_PROLOGUE_BYTES,
        player_text,
    )?;
    let names = prologue.matches("$N").count();
    let our_hero = prologue.to_lowercase().contains("our hero");
    let clean = slop_in(&prologue, told).is_empty()
        && arrival_in(&prologue, &[]).is_none()
        && prose_faults(&prologue, &[]).is_empty();
    let invented = ungrounded_names(&prologue, &format!("{given}\n{player_text}"));
    let grounded = names_something_of(&prologue, told) && invented.is_empty();
    (names <= MAX_NAMES && !our_hero && clean && grounded).then_some(prologue)
}

/// True when the text names a word of a name of `told`: a capital word of 4 letters or
/// more, such as "Westfall" or "Defias".
fn names_something_of(text: &str, told: &str) -> bool {
    told.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| word.chars().count() >= NAME_LETTERS)
        .filter(|word| word.chars().next().is_some_and(char::is_uppercase))
        .any(|word| mentions(text, word))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(prologue: &str) -> String {
        serde_json::json!({ "prologue": prologue }).to_string()
    }

    const TOLD: &str = "a human paladin\nStands now in Westfall.\nHas traveled in: Elwynn Forest.";

    /// The facts and the lore of the prompt.
    const GIVEN: &str = "a human paladin\nStands now in Westfall.\nHas traveled in: Elwynn \
        Forest.\nThe Defias Brotherhood raids Westfall. The People's Militia holds Sentinel Hill.";

    #[test]
    fn a_prologue_about_the_lands_of_the_facts_passes() {
        let text = reply(
            "The Defias Brotherhood took the farms of Westfall from Stormwind, and the \
             People's Militia still holds Sentinel Hill against it.",
        );

        assert!(checked_prologue(&text, TOLD, "", GIVEN).is_some());
    }

    #[test]
    fn a_prologue_that_names_no_place_of_its_facts_is_refused() {
        let text = reply("The kingdom stood for a long age, and its people kept their roads safe.");

        assert_eq!(checked_prologue(&text, TOLD, "", GIVEN), None);
    }

    #[test]
    fn a_prologue_that_says_our_hero_is_refused() {
        let text = reply("Our hero grew up in Elwynn Forest and left it for Westfall.");

        assert_eq!(checked_prologue(&text, TOLD, "", GIVEN), None);
    }

    #[test]
    fn a_prologue_that_names_the_hero_three_times_is_refused() {
        let text = reply(
            "$N fought in Westfall for the militia. $N fought in Elwynn Forest for the \
             crown. $N fought for the Light in every town.",
        );

        assert_eq!(checked_prologue(&text, TOLD, "", GIVEN), None);
    }

    #[test]
    fn a_reply_that_is_no_json_is_refused() {
        assert_eq!(checked_prologue("Westfall burned.", TOLD, "", GIVEN), None);
    }

    #[test]
    fn the_prompt_names_no_count_and_holds_the_lore() {
        let facts = Facts {
            who: Some("a human paladin".to_string()),
            level: 35,
            past: vec!["Stands now in Westfall.".to_string()],
            lore: vec!["Westfall was the breadbasket of Stormwind.".to_string()],
        };

        let prompt = prompt(&facts);

        assert!(prompt.contains("The hero: a human paladin, level 35."));
        assert!(prompt.contains("breadbasket of Stormwind"));
        assert!(prompt.ends_with("{\"prologue\": \"<the paragraph>\"}"));
    }
}
