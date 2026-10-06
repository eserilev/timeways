//! The lore of a narrator moment: one passage about its place, foe, or person
//! (GAMEPLAY.md 3.2). It passes the spoiler limit as a `/lore` passage does, and the text
//! that the player read comes first. A deed whose lore is thin gets no line.

use crate::character::Character;
use crate::check::mentions;
use crate::moments::Moment;
use crate::narrator::{Who, lore_excerpt};
use crate::pack::{Link, Pack, PackError, Passage};
use crate::passage_limits;
use crate::race_class::Race;
use crate::seen::SeenIndex;
use crate::walk::LEVEL_STEP;
use thiserror::Error;

/// Enough candidates that the spoiler limit still leaves one about the subject.
const CANDIDATES: u32 = 20;

#[derive(Debug, Error)]
pub enum LoreError {
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error("seen text: {0}")]
    Seen(#[from] rusqlite::Error),
}

/// The lore of the first subject of the moment that has some (`lore_subjects`).
///
/// # Errors
///
/// Returns the error of the pack or of the index of seen text.
pub fn lore_of_moment(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    moment: &Moment,
    who: &Who,
) -> Result<Option<Passage>, LoreError> {
    if let Moment::LevelUp { level, .. } = moment {
        let own = people_lore(pack, character, &lore_subjects(moment, who))?;
        if let Some(passage) = nth_for_level(own, *level) {
            return Ok(Some(passage));
        }
    }
    for subject in lore_subjects(moment, who) {
        if let Some(passage) = lore_about(pack, seen, character, &subject)? {
            return Ok(Some(passage));
        }
    }
    Ok(None)
}

/// What the lore of a moment must be about, best first. A tenth level tells of the
/// people of the hero (docs/plans/level-lines.md), and every other moment of its own
/// subjects (`Moment::subjects`).
#[must_use]
pub fn lore_subjects(moment: &Moment, who: &Who) -> Vec<String> {
    if let Moment::LevelUp { .. } = moment {
        return who
            .race
            .map(people_pages)
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect();
    }
    moment.subjects().into_iter().map(str::to_string).collect()
}

/// The pages of the lore of a people: its capital first, then the land or the name of
/// the people. The Undercity has no page of its own in every pack, so the Forsaken also
/// take the ruins above it.
fn people_pages(race: Race) -> &'static [&'static str] {
    match race {
        Race::Human => &["Stormwind City", "Elwynn Forest"],
        Race::Orc => &["Orgrimmar", "Durotar"],
        Race::Dwarf => &["Ironforge", "Dun Morogh"],
        Race::NightElf => &["Darnassus", "Teldrassil"],
        Race::Forsaken => &["Undercity", "Forsaken", "Ruins of Lordaeron"],
        Race::Tauren => &["Thunder Bluff", "Mulgore"],
        Race::Gnome => &["Gnomeregan"],
        Race::Troll => &["Echo Isles", "Sen'jin Village", "Darkspear"],
    }
}

/// True when a deed has no lore of its own to tell: no passage, or a passage about none
/// of its subjects. A passage is about a subject when its page is, when it links to the
/// subject, or when the part that the prompt shows names it. The rule reads only what
/// the prompt holds, so a test can check it, and a model never decides it.
#[must_use]
pub fn is_thin(subjects: &[String], passage: Option<&Passage>) -> bool {
    let Some(passage) = passage else {
        return true;
    };
    let shown = lore_excerpt(&passage.text);
    !subjects
        .iter()
        .any(|subject| is_about(passage, &shown, subject))
}

fn is_about(passage: &Passage, shown: &str, subject: &str) -> bool {
    passage.about.as_deref() == Some(subject)
        || is_linked(passage, subject)
        || mentions(shown, subject)
}

/// The passages of the own pages of a people, in page order, that pass the spoiler limit.
fn people_lore(
    pack: &Pack,
    character: &Character,
    pages: &[String],
) -> Result<Vec<Passage>, LoreError> {
    let mut own = Vec::new();
    for page in pages {
        own.extend(known(character, pack.about(page, CANDIDATES)?));
    }
    Ok(own)
}

/// Level 10 takes the first passage, level 20 the second, and so on, so two tenth levels
/// of one character tell two passages while the pages hold enough
/// (docs/plans/level-lines.md, rule 5).
fn nth_for_level(passages: Vec<Passage>, level: i64) -> Option<Passage> {
    let count = u64::try_from(passages.len())
        .ok()
        .filter(|count| *count > 0)?;
    let step = u64::try_from(level / LEVEL_STEP)
        .unwrap_or(1)
        .saturating_sub(1);
    let index = usize::try_from(step % count).ok()?;
    passages.into_iter().nth(index)
}

fn known(character: &Character, passages: Vec<Passage>) -> Vec<Passage> {
    passages
        .into_iter()
        .filter(|passage| character.knows_all(&passage.links))
        .filter_map(passage_limits::fitted)
        .collect()
}

/// The own page of `subject` comes first, in page order, so the history of the Deadmines
/// wins over the page of a boss inside it. Then a passage linked to `subject`, then one
/// that names it. A search for "The Deadmines" also finds every passage with "the", so a
/// passage about something else never counts.
///
/// # Errors
///
/// Returns the error of the pack or of the index of seen text.
pub fn lore_about(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    subject: &str,
) -> Result<Option<Passage>, LoreError> {
    let own = known(character, pack.about(subject, CANDIDATES)?);
    if let Some(own) = own.into_iter().next() {
        return Ok(Some(own));
    }
    let mut found = seen.search(subject, CANDIDATES)?;
    found.extend(pack.search(subject, CANDIDATES)?);
    let found = known(character, found);
    let linked = found.iter().find(|passage| is_linked(passage, subject));
    let named = || {
        found
            .iter()
            .find(|passage| mentions(&passage.text, subject))
    };
    Ok(linked.or_else(named).cloned())
}

fn is_linked(passage: &Passage, subject: &str) -> bool {
    passage.links.iter().any(|link| match link {
        Link::Place(name) | Link::Npc(name) => name == subject,
        Link::Common => false,
    })
}
