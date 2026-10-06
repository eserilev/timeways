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
use timeways_rules::thin_lore::{self, MomentKind};

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

/// True when the moment gets no line: a deed whose lore is thin. The rule lives in
/// `timeways_rules::thin_lore`, where Lean proves it.
#[must_use]
pub fn is_silent(moment: &Moment, subjects: &[String], passage: Option<&Passage>) -> bool {
    let kind = if moment.is_arrival() {
        MomentKind::Arrival
    } else if moment.is_deed() {
        MomentKind::Deed
    } else {
        MomentKind::Flavor
    };
    let ids = SubjectIds::of(subjects, passage);
    thin_lore::is_silent(kind, &ids.moment, &ids.lore)
}

/// True when a deed has no lore of its own to tell: no passage, or a passage about none
/// of its subjects. A passage is about a subject when its page is, when it links to the
/// subject, or when the part that the prompt shows names it. The rule reads only what
/// the prompt holds, so a test can check it, and a model never decides it.
#[must_use]
pub fn is_thin(subjects: &[String], passage: Option<&Passage>) -> bool {
    let ids = SubjectIds::of(subjects, passage);
    thin_lore::is_silent(MomentKind::Deed, &ids.moment, &ids.lore)
}

/// The names of the moment and of its passage, as ids: one id for each distinct name.
struct SubjectIds {
    moment: Vec<u32>,
    lore: Vec<u32>,
}

impl SubjectIds {
    fn of(subjects: &[String], passage: Option<&Passage>) -> SubjectIds {
        let mut names: Vec<&str> = Vec::new();
        let moment = subjects
            .iter()
            .map(|subject| id_of(&mut names, subject))
            .collect();
        let mut lore = Vec::new();
        if let Some(passage) = passage {
            for name in passage_subjects(passage, subjects) {
                lore.push(id_of(&mut names, name));
            }
        }
        SubjectIds { moment, lore }
    }
}

/// What a passage is about: its page, its links, and each subject that its shown part
/// names.
fn passage_subjects<'a>(passage: &'a Passage, subjects: &'a [String]) -> Vec<&'a str> {
    let shown = lore_excerpt(&passage.text);
    let mut about: Vec<&str> = passage.about.iter().map(String::as_str).collect();
    for link in &passage.links {
        if let Link::Place(name) | Link::Npc(name) = link {
            about.push(name);
        }
    }
    let named = subjects.iter().filter(|subject| mentions(&shown, subject));
    about.extend(named.map(String::as_str));
    about
}

fn id_of<'a>(names: &mut Vec<&'a str>, name: &'a str) -> u32 {
    let index = names
        .iter()
        .position(|known| *known == name)
        .unwrap_or_else(|| {
            names.push(name);
            names.len() - 1
        });
    u32::try_from(index).unwrap_or(u32::MAX)
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
