//! The journal of a character: the chronicle, the places, the people, the deeds, the hero,
//! what you learned, and your side quests. No model takes part.

use crate::chapters::{chapter_starts, is_level_milestone, sessions};
use crate::character::{Character, title_of_game_quest};
use crate::hero::{Entry, Hero};
use crate::learned::Learned;
use crate::places::{self, PlaceKind};
use crate::quest::{Tracked, title_of_thing};
use crate::reply_size::{MAX_LINE, MAX_SLOT, Size};
use crate::vocabulary::{
    CLASS_QUEST, DEATHS, DEFEATED, GAME_QUEST_DONE, LEVEL, MET, QUEST_DONE, SLAPPED, TITLE, TRUSTS,
    VISITED,
};
use hourglass::{EntityId, EventKind, LOCATED_IN, Tick, World};
use serde::Serialize;

/// The bridge takes at most 200 items in one list (Gnomish Relay SPEC.md 9.8).
const PAGE_LIST_ITEMS: usize = 200;

/// Room for the type, the id, the page numbers, and the empty lists of a page line. In the
/// slot, also room for the record and the note of the bridge.
const FRAME: Size = Size {
    line: 256,
    slot: 2048,
};

/// The entries of each list of one chapter. With names of at most
/// `story::MAX_NAME_BYTES`, a chapter always fits on one page.
const CHAPTER_LIST: usize = 20;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Journal {
    /// The sheet of the hero goes on the first page. Its entries are a list like the others.
    pub hero: Hero,
    /// Why the last edit of the hero did not stand. It shows once.
    pub hero_refused: Option<String>,
    pub chapters: Vec<Chapter>,
    pub places: Vec<Place>,
    pub people: Vec<Person>,
    pub deeds: Vec<Deed>,
    /// What you read and heard (GAMEPLAY.md 3.1.1).
    pub learned: Vec<Learned>,
    /// The side quests that you did not decline (3.4).
    pub quests: Vec<Tracked>,
}

/// One chapter of the chronicle, from one milestone to the next, with no model: what was
/// new in it (GAMEPLAY.md 3.3 and 5.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Chapter {
    pub number: usize,
    pub began: Tick,
    /// The tick of the last event of the chapter.
    pub ended: Tick,
    pub zones: Vec<String>,
    pub people: Vec<String>,
    pub deeds: Vec<Deed>,
    /// The entries past the first 20 of each list. The other pages of the journal hold
    /// them all.
    pub left_out: usize,
    /// The saga of the chapter, once a model wrote it (3.3).
    pub prose: Option<String>,
    /// The footnotes of the saga: small moments of the chapter (5.4.1).
    pub footnotes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Place {
    pub name: String,
    pub kind: PlaceKind,
    /// The place around it: the zone of a subzone. None for a zone.
    pub within: Option<String>,
    pub first_visit: Tick,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Person {
    pub name: String,
    pub place: Option<String>,
    pub first_met: Tick,
    /// How much this NPC trusts you, from -100 to 100, once anything changed it.
    pub trust: Option<i64>,
    pub slapped: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Deed {
    /// `from` is None for the first level that the world saw, at the first login.
    Level {
        from: Option<i64>,
        to: i64,
        at: Tick,
        place: Option<String>,
    },
    /// `times` is 1 for the true kill, and more for each echo after a reset (5.13).
    Defeated {
        foe: String,
        times: i64,
        at: Tick,
        place: Option<String>,
    },
    /// A joke title of the journal (5.4.1).
    Titled {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A side quest that you finished (3.4).
    QuestDone {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A quest of the game that you turned in.
    GameQuestDone {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// A quest of the game that only your class gets.
    ClassQuestDone {
        title: String,
        at: Tick,
        place: Option<String>,
    },
    /// `killer` is None when the addon did not know it, or when it was a player.
    Died {
        killer: Option<String>,
        at: Tick,
        place: Option<String>,
    },
}

/// One reply of the journal. The addon joins the lists of pages 0 to `pages - 1`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Page {
    pub page: usize,
    pub pages: usize,
    #[serde(flatten)]
    pub journal: Journal,
}

/// Cuts the journal into pages that each fit in one reply, in the order of the lists.
/// There is always at least one page, so an empty journal still answers.
#[must_use]
pub fn pages(journal: Journal) -> Vec<Page> {
    let mut pages = Vec::new();
    let mut current = Journal {
        hero: Hero {
            sheet: journal.hero.sheet,
            entries: Vec::new(),
        },
        hero_refused: journal.hero_refused,
        ..Journal::default()
    };
    let mut used = Size::of(&current.hero.sheet).plus(Size::of(&current.hero_refused));
    let budget = Size {
        line: MAX_LINE - FRAME.line,
        slot: MAX_SLOT - FRAME.slot,
    };
    // In the order of the lists, so each list stays in order across the pages.
    let mut items: Vec<Item> = journal.hero.entries.into_iter().map(Item::Entry).collect();
    items.extend(journal.chapters.into_iter().map(Item::Chapter));
    items.extend(journal.places.into_iter().map(Item::Place));
    items.extend(journal.people.into_iter().map(Item::Person));
    items.extend(journal.deeds.into_iter().map(Item::Deed));
    items.extend(journal.learned.into_iter().map(Item::Learned));
    items.extend(journal.quests.into_iter().map(Item::Quest));
    // The comma after an item.
    let comma = Size { line: 1, slot: 1 };
    for item in items {
        let size = item.size().plus(comma);
        let list_full = item.list_len(&current) >= PAGE_LIST_ITEMS;
        let page_full = !used.plus(size).fits(budget);
        if (page_full || list_full) && used != Size::default() {
            pages.push(std::mem::take(&mut current));
            used = Size::default();
        }
        used = used.plus(size);
        item.add_to(&mut current);
    }
    pages.push(current);
    let count = pages.len();
    pages
        .into_iter()
        .enumerate()
        .map(|(page, journal)| Page {
            page,
            pages: count,
            journal,
        })
        .collect()
}

enum Item {
    Entry(Entry),
    Chapter(Chapter),
    Place(Place),
    Person(Person),
    Deed(Deed),
    Learned(Learned),
    Quest(Tracked),
}

impl Item {
    fn size(&self) -> Size {
        match self {
            Item::Entry(entry) => Size::of(entry),
            Item::Chapter(chapter) => Size::of(chapter),
            Item::Place(place) => Size::of(place),
            Item::Person(person) => Size::of(person),
            Item::Deed(deed) => Size::of(deed),
            Item::Learned(learned) => Size::of(learned),
            Item::Quest(quest) => Size::of(quest),
        }
    }

    /// The length of the list of `journal` that this item goes into.
    fn list_len(&self, journal: &Journal) -> usize {
        match self {
            Item::Entry(_) => journal.hero.entries.len(),
            Item::Chapter(_) => journal.chapters.len(),
            Item::Place(_) => journal.places.len(),
            Item::Person(_) => journal.people.len(),
            Item::Deed(_) => journal.deeds.len(),
            Item::Learned(_) => journal.learned.len(),
            Item::Quest(_) => journal.quests.len(),
        }
    }

    fn add_to(self, journal: &mut Journal) {
        match self {
            Item::Entry(entry) => journal.hero.entries.push(entry),
            Item::Chapter(chapter) => journal.chapters.push(chapter),
            Item::Place(place) => journal.places.push(place),
            Item::Person(person) => journal.people.push(person),
            Item::Deed(deed) => journal.deeds.push(deed),
            Item::Learned(learned) => journal.learned.push(learned),
            Item::Quest(quest) => journal.quests.push(quest),
        }
    }
}

/// Each list is in the order of the history, oldest first.
#[must_use]
pub fn journal(character: &Character) -> Journal {
    let world = character.world();
    let you = character.you();
    let places: Vec<Place> = first_links(world, you, VISITED)
        .into_iter()
        .map(|(place, first_visit)| Place {
            name: name_of(world, place),
            kind: places::kind_of(world, place),
            within: world.location_of(place).map(|zone| name_of(world, zone)),
            first_visit,
        })
        .collect();
    let people: Vec<Person> = first_links(world, you, MET)
        .into_iter()
        .map(|(npc, first_met)| Person {
            name: name_of(world, npc),
            place: world.location_of(npc).map(|place| name_of(world, place)),
            first_met,
            trust: fact_value(world, npc, TRUSTS, you),
            slapped: fact_value(world, you, SLAPPED, npc),
        })
        .collect();
    let deeds = deeds(world, you);
    let chapters = chapters(world, &places, &people, &deeds);
    Journal {
        chapters,
        places,
        people,
        deeds,
        ..Journal::default()
    }
}

/// A chapter with nothing new has no number.
fn chapters(world: &World, places: &[Place], people: &[Person], deeds: &[Deed]) -> Vec<Chapter> {
    let ticks: Vec<Tick> = world.history().iter().map(|event| event.tick).collect();
    let sessions = sessions(ticks.iter().copied());
    let starts = chapter_starts(&sessions, &milestones(places, deeds));
    let mut chapters = Vec::new();
    for (index, &began) in starts.iter().enumerate() {
        let next = starts.get(index + 1).copied();
        let within = |at: Tick| at >= began && next.is_none_or(|next| at < next);
        let Some(ended) = ticks.iter().copied().filter(|&at| within(at)).max() else {
            continue;
        };
        let mut left_out = 0;
        let zones = capped(
            places
                .iter()
                .filter(|place| place.within.is_none() && within(place.first_visit))
                .map(|place| place.name.clone()),
            &mut left_out,
        );
        let met = capped(
            people
                .iter()
                .filter(|person| within(person.first_met))
                .map(|person| person.name.clone()),
            &mut left_out,
        );
        let done = capped(
            deeds.iter().filter(|deed| within(deed.at())).cloned(),
            &mut left_out,
        );
        if zones.is_empty() && met.is_empty() && done.is_empty() {
            continue;
        }
        let number = chapters.len() + 1;
        chapters.push(Chapter {
            number,
            began,
            ended,
            zones,
            people: met,
            deeds: done,
            left_out,
            prose: None,
            footnotes: Vec::new(),
        });
    }
    chapters
}

/// The ticks that can begin a chapter: the first visit of a zone, every tenth level, and
/// the first kill of a rare or a boss, and a finished class quest.
fn milestones(places: &[Place], deeds: &[Deed]) -> Vec<Tick> {
    let zones = places
        .iter()
        .filter(|place| place.within.is_none())
        .map(|place| place.first_visit);
    let big_deeds = deeds.iter().filter(|deed| is_milestone(deed)).map(Deed::at);
    zones.chain(big_deeds).collect()
}

fn is_milestone(deed: &Deed) -> bool {
    match deed {
        Deed::Level { from, to, .. } => from.is_some() && is_level_milestone(*to),
        Deed::Defeated { times, .. } => *times == 1,
        Deed::ClassQuestDone { .. } => true,
        Deed::Titled { .. }
        | Deed::QuestDone { .. }
        | Deed::GameQuestDone { .. }
        | Deed::Died { .. } => false,
    }
}

fn capped<T>(items: impl Iterator<Item = T>, left_out: &mut usize) -> Vec<T> {
    let mut kept = Vec::new();
    for item in items {
        if kept.len() < CHAPTER_LIST {
            kept.push(item);
        } else {
            *left_out += 1;
        }
    }
    kept
}

impl Deed {
    #[must_use]
    pub fn at(&self) -> Tick {
        match self {
            Deed::Level { at, .. }
            | Deed::Defeated { at, .. }
            | Deed::Titled { at, .. }
            | Deed::QuestDone { at, .. }
            | Deed::GameQuestDone { at, .. }
            | Deed::ClassQuestDone { at, .. }
            | Deed::Died { at, .. } => *at,
        }
    }
}

/// The targets of one linked fact of `holder`, each with the tick that opened it. The
/// facts in this list never change, so the opening event is the first time.
fn first_links(world: &World, holder: EntityId, fact: &str) -> Vec<(EntityId, Tick)> {
    let Some(entity) = world.entity(holder) else {
        return Vec::new();
    };
    let mut links: Vec<_> = entity
        .facts_named(fact)
        .filter_map(|fact| Some((fact.opened, fact.linked_to?)))
        .collect();
    links.sort();
    links
        .into_iter()
        .filter_map(|(opened, target)| Some((target, world.history().get(opened)?.tick)))
        .collect()
}

/// A walk of the history, because the state holds only the current level, place, and
/// count of kills.
fn deeds(world: &World, you: EntityId) -> Vec<Deed> {
    let mut deeds = Vec::new();
    let mut here = None;
    // `die` writes the kill of the killer just before the count of deaths.
    let mut killer = None;
    for event in world.history() {
        match &event.kind {
            EventKind::FactStart {
                entity,
                name,
                linked_to,
                ..
            } if *entity == you && name == LOCATED_IN => {
                here = *linked_to;
            }
            EventKind::FactStart {
                entity,
                name,
                value: Some(to),
                ..
            } if *entity == you && name == LEVEL => {
                deeds.push(level_deed(world, None, *to, event.tick, here));
            }
            EventKind::FactUpdate {
                entity,
                name,
                from,
                to,
                ..
            } if *entity == you && name == LEVEL => {
                deeds.push(level_deed(world, Some(*from), *to, event.tick, here));
            }
            EventKind::FactStart {
                entity,
                name,
                value: Some(times),
                linked_to: Some(foe),
            }
            | EventKind::FactUpdate {
                entity,
                name,
                to: times,
                linked_to: Some(foe),
                ..
            } if *entity == you && name == DEFEATED => {
                let place = here.map(|place| name_of(world, place));
                let foe = name_of(world, *foe);
                deeds.push(Deed::Defeated {
                    foe,
                    times: *times,
                    at: event.tick,
                    place,
                });
            }
            EventKind::FactStart {
                entity: foe,
                name,
                linked_to: Some(target),
                ..
            }
            | EventKind::FactUpdate {
                entity: foe,
                name,
                linked_to: Some(target),
                ..
            } if *target == you && name == DEFEATED => {
                killer = Some(name_of(world, *foe));
            }
            EventKind::FactStart {
                entity,
                name,
                linked_to: Some(thing),
                ..
            } if *entity == you && (name == TITLE || name == QUEST_DONE) => {
                let place = here.map(|place| name_of(world, place));
                deeds.push(thing_deed(name, name_of(world, *thing), event.tick, place));
            }
            EventKind::FactStart {
                entity,
                name,
                linked_to: Some(quest),
                ..
            } if *entity == you && name == GAME_QUEST_DONE => {
                let place = here.map(|place| name_of(world, place));
                deeds.extend(game_quest_deed(world, *quest, event.tick, place));
            }
            EventKind::FactStart { entity, name, .. }
            | EventKind::FactUpdate { entity, name, .. }
                if *entity == you && name == DEATHS =>
            {
                let place = here.map(|place| name_of(world, place));
                deeds.push(Deed::Died {
                    killer: killer.take(),
                    at: event.tick,
                    place,
                });
            }
            _ => {}
        }
    }
    deeds
}

/// A title that you earned, or a quest that you finished: both are things that you hold.
fn thing_deed(fact: &str, thing: String, at: Tick, place: Option<String>) -> Deed {
    if fact != QUEST_DONE {
        return Deed::Titled {
            title: thing,
            at,
            place,
        };
    }
    let title = title_of_thing(&thing).map_or_else(|| thing.clone(), str::to_string);
    Deed::QuestDone { title, at, place }
}

fn game_quest_deed(
    world: &World,
    quest: EntityId,
    at: Tick,
    place: Option<String>,
) -> Option<Deed> {
    let entity = world.entity(quest)?;
    let title = title_of_game_quest(&entity.name)?.to_string();
    if entity.fact(CLASS_QUEST, None).is_some() {
        return Some(Deed::ClassQuestDone { title, at, place });
    }
    Some(Deed::GameQuestDone { title, at, place })
}

fn level_deed(world: &World, from: Option<i64>, to: i64, at: Tick, here: Option<EntityId>) -> Deed {
    let place = here.map(|place| name_of(world, place));
    Deed::Level {
        from,
        to,
        at,
        place,
    }
}

fn fact_value(world: &World, holder: EntityId, fact: &str, target: EntityId) -> Option<i64> {
    world.entity(holder)?.fact(fact, Some(target))?.value
}

fn name_of(world: &World, id: EntityId) -> String {
    world
        .entity(id)
        .map(|entity| entity.name.clone())
        .unwrap_or_default()
}
