//! The journal of a character: the places, the people, and the deeds, all from the world
//! and its history. No model takes part.

use crate::character::Character;
use crate::vocabulary::{DEFEATED, LEVEL, MET, VISITED};
use hourglass::{EntityId, EventKind, LOCATED_IN, Tick, World};
use serde::Serialize;

/// The largest reply line that the bridge takes (Gnomish Relay SPEC.md 9.7 and S12).
pub const PAGE_BYTES: usize = 24_576;

/// Room for the type, the id, the page numbers, and the empty lists of a page line.
const FRAME_BYTES: usize = 160;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Journal {
    pub places: Vec<Place>,
    pub people: Vec<Person>,
    pub deeds: Vec<Deed>,
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
        .places
        .into_iter()
        .map(Item::Place)
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
    Place(Place),
    Person(Person),
    Deed(Deed),
}

impl Item {
    /// The JSON bytes of the item. These plain types always serialize, so the fallback
    /// never happens, and it would only put the item on a page of its own.
    fn size(&self) -> usize {
        let bytes = match self {
            Item::Place(place) => serde_json::to_vec(place),
            Item::Person(person) => serde_json::to_vec(person),
            Item::Deed(deed) => serde_json::to_vec(deed),
        };
        bytes.map_or(PAGE_BYTES, |bytes| bytes.len())
    }

    fn add_to(self, journal: &mut Journal) {
        match self {
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
    Journal {
        places: first_links(world, you, VISITED)
            .into_iter()
            .map(|(place, first_visit)| Place {
                name: name_of(world, place),
                within: world.location_of(place).map(|zone| name_of(world, zone)),
                first_visit,
            })
            .collect(),
        people: first_links(world, you, MET)
            .into_iter()
            .map(|(npc, first_met)| Person {
                name: name_of(world, npc),
                place: world.location_of(npc).map(|place| name_of(world, place)),
                first_met,
            })
            .collect(),
        deeds: deeds(world, you),
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

fn name_of(world: &World, id: EntityId) -> String {
    world
        .entity(id)
        .map(|entity| entity.name.clone())
        .unwrap_or_default()
}
