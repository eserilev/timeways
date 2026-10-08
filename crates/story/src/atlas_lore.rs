//! The lore of the Knowledge atlas (GAMEPLAY.md 3.6): for each place that you visited and
//! each person that you met, the first passage of its own page that passes the spoiler
//! limit. A place also says when more of its lore waits behind the limit, but never what.
//! No model takes part.

use crate::character::Character;
use crate::journal::{Person, Place};
use crate::pack::{Dependency, Pack, PackError, Passage};
use crate::spoiler;
use serde::Serialize;

/// The mockup of the atlas shows passages of at most 600 characters.
pub const LORE_CHARS: usize = 600;

/// The bridge takes at most 1600 bytes in one string of the journal. A letter outside
/// ASCII takes up to 4 bytes, so the text also stops at this many bytes.
pub const LORE_BYTES: usize = 1200;

/// The subjects of the list. Zones come first, then subzones, then people, so a long
/// journal drops the lore of people first.
pub const LORE_SUBJECTS: usize = 48;

/// Enough passages of one page that the spoiler limit still leaves the lead.
const CANDIDATES: u32 = 20;

/// What the atlas knows of one place or person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lore {
    pub about: String,
    /// The first passage of the own page that the player may read, cut after a sentence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// A passage of this place waits behind the spoiler limit: a person to meet, a place
    /// to visit, or a deed to do.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub more: bool,
}

/// The lore of the places and people of the journal. A subject with no text and nothing
/// more to learn has no entry.
///
/// # Errors
///
/// Returns the error of the pack.
pub fn lore_of(
    pack: &Pack,
    character: &Character,
    places: &[Place],
    people: &[Person],
) -> Result<Vec<Lore>, PackError> {
    let mut lore = Vec::new();
    for (name, subject) in subjects(places, people) {
        if lore.len() == LORE_SUBJECTS {
            break;
        }
        let text = own_text(pack, character, name)?;
        let more = subject == Subject::Place && more_to_learn(pack, character, name)?;
        if text.is_some() || more {
            lore.push(Lore {
                about: name.to_string(),
                text,
                more,
            });
        }
    }
    Ok(lore)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Subject {
    Place,
    Person,
}

/// Zones first, then subzones, then people, each group newest first.
fn subjects<'a>(places: &'a [Place], people: &'a [Person]) -> Vec<(&'a str, Subject)> {
    let zones = places.iter().rev().filter(|place| is_zone(place));
    let subzones = places.iter().rev().filter(|place| !is_zone(place));
    let mut subjects: Vec<(&str, Subject)> = zones
        .chain(subzones)
        .map(|place| (place.name.as_str(), Subject::Place))
        .collect();
    let people = people.iter().rev();
    subjects.extend(people.map(|person| (person.name.as_str(), Subject::Person)));
    subjects
}

fn is_zone(place: &Place) -> bool {
    place.within.is_none()
}

fn own_text(pack: &Pack, character: &Character, name: &str) -> Result<Option<String>, PackError> {
    let own = pack.about(name, CANDIDATES)?;
    Ok(own
        .iter()
        .find(|passage| spoiler::may_show(character, passage))
        .map(|passage| clipped(&passage.text))
        .filter(|text| !text.is_empty()))
}

/// A passage of the place that the spoiler limit hides now, and that play can still
/// open. A stale setup and a deed that the pack never tied to anyone stay hidden for good,
/// so they promise nothing.
fn more_to_learn(pack: &Pack, character: &Character, place: &str) -> Result<bool, PackError> {
    let mut passages = pack.of_place(place)?;
    passages.extend(pack.about(place, CANDIDATES)?);
    Ok(passages
        .iter()
        .any(|passage| can_open_later(character, passage)))
}

fn can_open_later(character: &Character, passage: &Passage) -> bool {
    let never = passage.depends_on.contains(&Dependency::Unresolved)
        || !spoiler::setup_allowed(character, passage.setup_for.as_ref());
    !never && !spoiler::may_show(character, passage)
}

/// The text on one line, cut after the last sentence that fits both limits.
#[must_use]
pub fn clipped(text: &str) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let head = head_within_limits(&line);
    if head.len() == line.len() {
        return line;
    }
    let end = head
        .rmatch_indices(". ")
        .map(|(at, _)| at + 1)
        .next()
        .or_else(|| head.rfind(' '));
    end.map_or(head, |end| &head[..end]).to_string()
}

/// The longest start of `line` with at most `LORE_CHARS` characters and `LORE_BYTES` bytes.
fn head_within_limits(line: &str) -> &str {
    let mut end = 0;
    for (count, (at, letter)) in line.char_indices().enumerate() {
        let next = at + letter.len_utf8();
        if count == LORE_CHARS || next > LORE_BYTES {
            break;
        }
        end = next;
    }
    &line[..end]
}
