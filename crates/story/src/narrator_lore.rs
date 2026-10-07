//! The lore of a narrator moment: one passage about its place, foe, or person
//! (GAMEPLAY.md 3.2). It passes the spoiler limit as a `/lore` passage does, with the gates
//! of outcome and setup passages, and the text that the player read comes first. A deed
//! whose lore is thin gets no line. An entry into a dungeon or a raid tells a passage of
//! the instance that the character was never told: a setup first, when one is usable.

use crate::character::Character;
use crate::check::mentions;
use crate::game_names::{self, wiki_name};
use crate::house::fenced;
use crate::moments::Moment;
use crate::narrator::{Who, lore_excerpt};
use crate::pack::{Link, Pack, PackError, Passage};
use crate::passage_limits;
use crate::places::InstanceKind;
use crate::present_check::has_present_sentence;
use crate::race_class::Race;
use crate::seen::SeenIndex;
use crate::setup_passages;
use crate::spoiler;
use crate::walk::LEVEL_STEP;
use thiserror::Error;
use timeways_rules::instance_lore::next_passage;
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

/// The lore of the first subject of the moment that has some (`lore_subjects`). `told`
/// holds the lore that narrator calls told, fenced as their prompts showed it: an entry
/// into an instance never tells a passage of it. A setup passage comes as its window
/// (`setup_passages::window`).
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
    told: &[String],
) -> Result<Option<Passage>, LoreError> {
    Ok(any_lore(pack, seen, character, moment, who, told)?.map(shown))
}

fn any_lore(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    moment: &Moment,
    who: &Who,
    told: &[String],
) -> Result<Option<Passage>, LoreError> {
    let places = if moment.is_arrival() {
        Some(pack.place_names()?)
    } else {
        None
    };
    let all_places = places.as_deref().unwrap_or_default();
    if let Moment::InstanceAgain { zone, .. } = moment {
        return next_of_instance(pack, character, zone, told, all_places);
    }
    if let Moment::FirstInstance { zone, kind } = moment
        && *kind != InstanceKind::Battleground
        && let Some(setup) = first_setup(pack, character, zone, told, all_places)?
    {
        return Ok(Some(setup));
    }
    if let Moment::LevelUp { level, .. } = moment {
        let pages = lore_subjects(moment, who);
        let own = people_lore(pack, character, &pages)?;
        if !own.is_empty() {
            return Ok(nth_for_level(own, *level));
        }
        let found = people_search(pack, seen, character, &pages)?;
        return Ok(nth_for_level(found, *level));
    }
    for subject in lore_subjects(moment, who) {
        let found = match &places {
            Some(places) => place_lore(pack, seen, character, &subject, places)?,
            None => lore_about(pack, seen, character, &subject)?,
        };
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

/// The lore of a place moment, in the order of `lore_about`, with no passage of pure
/// biography (`tells_a_place`).
fn place_lore(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    place: &str,
    places: &[String],
) -> Result<Option<Passage>, LoreError> {
    let own = known(character, pack.about(wiki_name(place), CANDIDATES)?);
    let found = found_about(pack, seen, character, place)?;
    Ok(own
        .into_iter()
        .chain(found)
        .find(|passage| tells_a_place(passage, place, places)))
}

/// A place moment takes a passage of a page about a person or a group only when its shown
/// text names a place: the place of the moment, or any place of the pack
/// (docs/plans/lore-names-and-now.md 2.3 B). "Mathias Shaw trained Edwin as a rogue" tells
/// no place, so it never tells an entry into the Deadmines.
fn tells_a_place(passage: &Passage, place: &str, places: &[String]) -> bool {
    let Some(about) = &passage.about else {
        return true;
    };
    if places.iter().any(|known| game_names::same(known, about)) {
        return true;
    }
    let shown = lore_excerpt(&passage.text);
    let names = |name: &String| mentions(&shown, bare_place(name));
    mentions(&shown, bare_place(place)) || places.iter().any(names)
}

/// "The Deadmines" is named by "Deadmines" too.
fn bare_place(name: &str) -> &str {
    name.strip_prefix("The ").unwrap_or(name)
}

/// The first usable setup of the instance, in pack order, that no call told.
fn first_setup(
    pack: &Pack,
    character: &Character,
    instance: &str,
    told: &[String],
    places: &[String],
) -> Result<Option<Passage>, LoreError> {
    let setups = known(character, pack.setups_of(instance)?);
    Ok(setups
        .into_iter()
        .map(shown)
        .filter(|setup| tells_a_place(setup, instance, places))
        .find(|setup| times_told(setup, told) == 0))
}

/// The next passage of the instance that no call told, by the rule of
/// `timeways_rules::instance_lore`, where Lean proves it. None once every passage was told.
/// A passage of pure biography goes (`tells_a_place`).
fn next_of_instance(
    pack: &Pack,
    character: &Character,
    instance: &str,
    told: &[String],
    places: &[String],
) -> Result<Option<Passage>, LoreError> {
    let usable = known(character, pack.of_place(instance)?)
        .into_iter()
        .map(shown)
        .filter(|passage| tells_a_place(passage, instance, places));
    // A passage with a present sentence first: the line can end on what holds now. The
    // pick reads the candidates in any order (`an_instance_passage_is_told_at_most_once`).
    let (present, past): (Vec<Passage>, Vec<Passage>) =
        usable.partition(|passage| has_present_sentence(&lore_excerpt(&passage.text)));
    let candidates: Vec<Passage> = present.into_iter().chain(past).collect();
    let counts: Vec<u32> = candidates
        .iter()
        .map(|passage| times_told(passage, told))
        .collect();
    Ok(next_passage(&counts).and_then(|index| candidates.into_iter().nth(index)))
}

/// How many times a call told the lore of the passage, as a prompt shows it.
fn times_told(passage: &Passage, told: &[String]) -> u32 {
    let shown = fenced(&lore_excerpt(&passage.text));
    let count = told.iter().filter(|lore| **lore == shown).count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// A setup passage as a prompt shows it: the sentence that sets up its deed, and the
/// sentence before it.
fn shown(passage: Passage) -> Passage {
    let Some(setup) = &passage.setup_for else {
        return passage;
    };
    let Some(window) = setup_passages::window(&passage.text, &setup.deed) else {
        return passage;
    };
    Passage {
        text: window.to_string(),
        ..passage
    }
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
#[must_use]
pub fn people_pages(race: Race) -> &'static [&'static str] {
    match race {
        Race::Human => &["Stormwind City", "Elwynn Forest"],
        Race::Orc => &["Orgrimmar", "Durotar"],
        Race::Dwarf => &["Ironforge", "Dun Morogh"],
        Race::NightElf => &["Darnassus", "Teldrassil"],
        Race::Forsaken => &["Undercity", "Forsaken", "Ruins of Lordaeron"],
        Race::Tauren => &["Thunder Bluff", "Mulgore"],
        Race::Gnome => &["Gnomeregan"],
        Race::Troll => &["Echo Isles", "Sen'jin Village", "Darkspear tribe"],
    }
}

/// True when the moment gets no line: a deed whose lore is thin, or a later entry into an
/// instance with no passage left to tell. The rules live in `timeways_rules::thin_lore`
/// and `timeways_rules::instance_lore`, where Lean proves them.
#[must_use]
pub fn is_silent(moment: &Moment, subjects: &[String], passage: Option<&Passage>) -> bool {
    // A later entry tells only lore that the character was never told.
    if matches!(moment, Moment::InstanceAgain { .. }) && passage.is_none() {
        return true;
    }
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

/// Both names of a person get one id (`game_names`).
fn id_of<'a>(names: &mut Vec<&'a str>, name: &'a str) -> u32 {
    let name = wiki_name(name);
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

/// The passages that link to or name a page of a people, in page order, each once: the
/// fallback when the pages have no passage of their own.
fn people_search(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    pages: &[String],
) -> Result<Vec<Passage>, LoreError> {
    let mut found: Vec<Passage> = Vec::new();
    for page in pages {
        for passage in found_about(pack, seen, character, page)? {
            if !found.iter().any(|known| known.text == passage.text) {
                found.push(passage);
            }
        }
    }
    Ok(found)
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
        .filter(|passage| spoiler::may_show(character, passage))
        .filter_map(passage_limits::fitted)
        .collect()
}

/// The own page of `subject` comes first, in page order, so the history of the Deadmines
/// wins over the page of a boss inside it. The page of a game name is the page of its
/// wiki name: the kill of "High Inquisitor Whitemane" takes the page "Sally Whitemane". Then a passage linked to `subject`, then one
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
    let own = known(character, pack.about(wiki_name(subject), CANDIDATES)?);
    if let Some(own) = own.into_iter().next() {
        return Ok(Some(own));
    }
    Ok(found_about(pack, seen, character, subject)?
        .into_iter()
        .next())
}

/// The searched passages about `subject`: the linked ones first, then the ones that name it.
fn found_about(
    pack: &Pack,
    seen: &SeenIndex,
    character: &Character,
    subject: &str,
) -> Result<Vec<Passage>, LoreError> {
    let mut found = seen.search(subject, CANDIDATES)?;
    found.extend(pack.search(subject, CANDIDATES)?);
    let (linked, rest): (Vec<Passage>, Vec<Passage>) = known(character, found)
        .into_iter()
        .partition(|passage| is_linked(passage, subject));
    let named = rest
        .into_iter()
        .filter(|passage| mentions(&passage.text, subject));
    Ok(linked.into_iter().chain(named).collect())
}

fn is_linked(passage: &Passage, subject: &str) -> bool {
    passage.links.iter().any(|link| match link {
        Link::Place(name) | Link::Npc(name) => game_names::same(name, subject),
        Link::Common => false,
    })
}
