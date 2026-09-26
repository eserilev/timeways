//! The journal of a character: the places, the people, and the deeds, all from the world
//! and its history. No model takes part.

use crate::character::Character;
use crate::vocabulary::{DEATHS, DEFEATED, LEVEL, MET, SLAPPED, TRUSTS, VISITED};
use hourglass::{EntityId, EventKind, LOCATED_IN, Tick, World};
use serde::Serialize;

/// The largest reply line that the bridge takes (Gnomish Relay SPEC.md 9.7 and S12).
pub const PAGE_BYTES: usize = 24_576;

/// Room for the type, the id, the page numbers, and the empty lists of a page line.
const FRAME_BYTES: usize = 160;

/// No event for this long ends a chapter of the chronicle: the player stopped playing.
const SESSION_GAP_SECONDS: u64 = 30 * 60;

/// The entries of each list of one chapter, so that a chapter always fits on a page.
const CHAPTER_LIST: usize = 30;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Journal {
    pub chapters: Vec<Chapter>,
    pub places: Vec<Place>,
    pub people: Vec<Person>,
    pub deeds: Vec<Deed>,
}

/// One play session of the chronicle, with no model: what was new in it (GAMEPLAY.md
/// 3.3 and 5.6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Chapter {
    pub number: usize,
    pub began: Tick,
    pub zones: Vec<String>,
    pub people: Vec<String>,
    pub deeds: Vec<Deed>,
    /// The entries past the first 30 of each list. The other pages of the journal hold
    /// them all.
    pub left_out: usize,
    /// The saga of the bard, once a model wrote it (3.3).
    pub prose: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Place {
    pub name: String,
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
    let mut current = Journal::default();
    let mut used = 0;
    let budget = PAGE_BYTES - FRAME_BYTES;
    let items = journal
        .chapters
        .into_iter()
        .map(Item::Chapter)
        .chain(journal.places.into_iter().map(Item::Place))
        .chain(journal.people.into_iter().map(Item::Person))
        .chain(journal.deeds.into_iter().map(Item::Deed));
    for item in items {
        let size = item.size() + 1;
        if used + size > budget && used > 0 {
            pages.push(std::mem::take(&mut current));
            used = 0;
        }
        used += size;
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
    Chapter(Chapter),
    Place(Place),
    Person(Person),
    Deed(Deed),
}

impl Item {
    /// The JSON bytes of the item. These plain types always serialize, so the fallback
    /// never happens, and it would only put the item on a page of its own.
    fn size(&self) -> usize {
        let bytes = match self {
            Item::Chapter(chapter) => serde_json::to_vec(chapter),
            Item::Place(place) => serde_json::to_vec(place),
            Item::Person(person) => serde_json::to_vec(person),
            Item::Deed(deed) => serde_json::to_vec(deed),
        };
        bytes.map_or(PAGE_BYTES, |bytes| bytes.len())
    }

    fn add_to(self, journal: &mut Journal) {
        match self {
            Item::Chapter(chapter) => journal.chapters.push(chapter),
            Item::Place(place) => journal.places.push(place),
            Item::Person(person) => journal.people.push(person),
            Item::Deed(deed) => journal.deeds.push(deed),
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
    let chapters = chapters(&sessions(world), &places, &people, &deeds);
    Journal {
        chapters,
        places,
        people,
        deeds,
    }
}

/// The first and last tick of each stretch of play. The founding of the character at
/// tick 0 belongs to no session.
fn sessions(world: &World) -> Vec<(Tick, Tick)> {
    let mut sessions: Vec<(Tick, Tick)> = Vec::new();
    for tick in world
        .history()
        .iter()
        .map(|event| event.tick)
        .filter(|tick| tick.0 > 0)
    {
        match sessions.last_mut() {
            Some((_, last)) if tick.0.saturating_sub(last.0) <= SESSION_GAP_SECONDS => *last = tick,
            _ => sessions.push((tick, tick)),
        }
    }
    sessions
}

/// A session that added nothing new to the journal has no chapter.
fn chapters(
    sessions: &[(Tick, Tick)],
    places: &[Place],
    people: &[Person],
    deeds: &[Deed],
) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    for &(began, ended) in sessions {
        let within = |at: Tick| at >= began && at <= ended;
        let mut left_out = 0;
        let zones = capped(
            places
                .iter()
                .filter(|place| place.within.is_none() && within(place.first_visit))
                .map(|place| place.name.clone()),
            &mut left_out,
        );
        let people = capped(
            people
                .iter()
                .filter(|person| within(person.first_met))
                .map(|person| person.name.clone()),
            &mut left_out,
        );
        let deeds = capped(
            deeds.iter().filter(|deed| within(deed.at())).cloned(),
            &mut left_out,
        );
        if zones.is_empty() && people.is_empty() && deeds.is_empty() {
            continue;
        }
        let number = chapters.len() + 1;
        chapters.push(Chapter {
            number,
            began,
            zones,
            people,
            deeds,
            left_out,
            prose: None,
        });
    }
    chapters
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
            Deed::Level { at, .. } | Deed::Defeated { at, .. } | Deed::Died { at, .. } => *at,
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
