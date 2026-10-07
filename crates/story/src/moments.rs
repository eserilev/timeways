//! The big moments of a batch, found in the events that the batch added to the history
//! (GAMEPLAY.md 3.2). The narrator speaks about the best one. Each moment holds a name or
//! a milestone, because a line with nothing concrete to tell is slop: silence is better.

use crate::character::title_of_game_quest;
use crate::gear::title_of_item;
use crate::journal::mark_and_quest;
use crate::mounts::{people_of, title_of_mount};
use crate::places::{InstanceKind, is_capital};
use crate::quest::title_of_thing;
use crate::race_class::Race;
use crate::vocabulary::{
    ANIMAL, BATTLEGROUND, CLASS_QUEST, DEFEATED, DUNGEON, FIRST_EPIC_ITEM, FIRST_EPIC_MOUNT,
    FIRST_MOUNT, GAME_QUEST_DONE, LEVEL, MARKED_BY, QUEST_DONE, QUEST_OFFERED, RACE, RAID, SLAPPED,
    SLOT_NUMBERS, TITLE, UPGRADED, VISITED,
};
use crate::walk::LEVEL_STEP;
use hourglass::{EntityId, Event, EventKind, LOCATED_IN, World};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// A small, silly moment in plain words (5.4.1). It speaks only when no big moment does.
    /// `book` is the title of a book that the player read.
    Flavor { what: String, book: Option<String> },
    /// A joke title, the rarest moment of all (5.4.1).
    Titled { title: String },
    /// The true kill of a rare or a boss (5.13). `zone` is where it came.
    FirstKill {
        foe: String,
        zone: Option<String>,
        creature: Option<Creature>,
    },
    /// The first kill of a foe that killed you before (docs/plans/chapters.md 4). `deaths`
    /// counts your deaths to it.
    Revenge {
        foe: String,
        deaths: i64,
        zone: Option<String>,
    },
    /// The same NPC killed you again: "Third time this murloc got you."
    SlainAgain {
        killer: String,
        times: i64,
        zone: Option<String>,
    },
    /// "The innkeeper remembers it."
    Slapped {
        npc: String,
        times: i64,
        zone: Option<String>,
    },
    /// Only a milestone level: 10, 20, 30, and so on. `zone` is where it came.
    LevelUp { level: i64, zone: Option<String> },
    /// A finished quest of your class: a turn of your own story.
    ClassQuestDone { title: String },
    /// A finished side quest of Timeways (3.4). `giver` is the NPC who gave it.
    QuestDone {
        title: String,
        giver: Option<String>,
    },
    /// The first visit of a zone, not of a subzone.
    NewZone { zone: String },
    /// The first entry into a dungeon or a raid: a zone that the game called an instance.
    FirstInstance { zone: String, kind: InstanceKind },
    /// A later entry into a dungeon or a raid: you stand in it again after you stood
    /// outside it. It tells a passage of the instance that you were never told
    /// (`narrator_lore`), and it adds no weight to a chapter or a tale.
    InstanceAgain { zone: String, kind: InstanceKind },
    /// A quest of the game left a lasting buff or debuff on you.
    QuestMarked { mark: String, quest: String },
    /// The first visit of a capital city.
    FirstCapital { city: String },
    /// The first ride on a mount of your own. `people` is the place of the people who
    /// breed such mounts (`mounts::people_of`): its lore is the lore of the moment.
    FirstMount {
        mount: String,
        people: Option<String>,
    },
    /// The first ride at the speed of an epic mount.
    FirstEpicMount {
        mount: String,
        people: Option<String>,
    },
    /// The first item of epic quality that you put on. `zone` is where.
    FirstEpicItem {
        item: String,
        zone: Option<String>,
        slot: Option<SlotKind>,
    },
    /// An item far better than what its slot held (`gear::is_big_upgrade`).
    BigUpgrade {
        item: String,
        zone: Option<String>,
        slot: Option<SlotKind>,
    },
}

/// The creature type of a foe, for the kill parts of one type
/// (docs/plans/narrator-templates.md 2.3). The world holds only the beasts today: the
/// `animal` fact of a sighting. TODO: the other types need the creature type of
/// `npc_defeated` and a new fact; they come with that change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Creature {
    Beast,
    Undead,
    Demon,
    Dragonkin,
    Elemental,
}

/// Where an item goes: a weapon slot (16 to 18) or any other slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    Weapon,
    Worn,
}

/// The weapon slots of the game: main hand, off hand, and ranged.
const WEAPON_SLOTS: std::ops::RangeInclusive<i64> = 16..=18;

impl SlotKind {
    /// The kind of an inventory slot of the game, from 1 to 19.
    #[must_use]
    pub fn of(slot: i64) -> Option<SlotKind> {
        if WEAPON_SLOTS.contains(&slot) {
            return Some(SlotKind::Weapon);
        }
        SLOT_NUMBERS.holds(slot).then_some(SlotKind::Worn)
    }
}

impl Moment {
    /// What the lore of the moment must be about, best first. An item needs a story of its
    /// own: the lore of its zone tells of another subject (docs/plans/item-stories.md). A
    /// tenth level takes the lore of the order or the people of the hero, which
    /// `narrator_lore` finds from the race and the class (docs/plans/level-lines.md).
    #[must_use]
    pub fn subjects(&self) -> Vec<&str> {
        match self {
            Moment::LevelUp { .. } => Vec::new(),
            Moment::Slapped { npc, zone, .. } => [Some(npc.as_str()), zone.as_deref()]
                .into_iter()
                .flatten()
                .collect(),
            Moment::QuestDone { title, giver } => [Some(title.as_str()), giver.as_deref()]
                .into_iter()
                .flatten()
                .collect(),
            _ => self.subject().into_iter().collect(),
        }
    }

    /// The place, the foe, the person, or the quest of the moment: what its lore is about.
    /// A title and a flavor moment have no lore.
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        match self {
            Moment::FirstMount { people, .. } | Moment::FirstEpicMount { people, .. } => {
                people.as_deref()
            }
            Moment::FirstEpicItem { item, .. } | Moment::BigUpgrade { item, .. } => Some(item),
            Moment::Flavor { .. } | Moment::Titled { .. } => None,
            Moment::FirstKill { foe, .. } | Moment::Revenge { foe, .. } => Some(foe),
            Moment::SlainAgain { killer, .. } => Some(killer),
            Moment::Slapped { npc, .. } => Some(npc),
            Moment::LevelUp { zone, .. } => zone.as_deref(),
            Moment::ClassQuestDone { title } | Moment::QuestDone { title, .. } => Some(title),
            Moment::NewZone { zone }
            | Moment::FirstInstance { zone, .. }
            | Moment::InstanceAgain { zone, .. } => Some(zone),
            Moment::QuestMarked { quest, .. } => Some(quest),
            Moment::FirstCapital { city } => Some(city),
        }
    }

    /// Every name that the moment holds. A line must name one of them, or a name of its
    /// lore (`check::grounded`).
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        match self {
            Moment::Titled { title } => vec![title],
            Moment::QuestMarked { mark, quest } => vec![quest, mark],
            Moment::FirstMount { mount, people } | Moment::FirstEpicMount { mount, people } => {
                [Some(mount.as_str()), people.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect()
            }
            Moment::FirstEpicItem { item, zone, .. } | Moment::BigUpgrade { item, zone, .. } => {
                [Some(item.as_str()), zone.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect()
            }
            _ => self.subject().into_iter().collect(),
        }
    }

    /// The names that the game gave and no lore holds: a mount, an item, a quest, a buff,
    /// or a book. Such a name allows no word of the history that the checks refuse
    /// (GAMEPLAY.md 3.2.1).
    #[must_use]
    pub fn outside_names(&self) -> Vec<&str> {
        match self {
            Moment::FirstMount { mount, .. } | Moment::FirstEpicMount { mount, .. } => {
                vec![mount]
            }
            Moment::FirstEpicItem { item, .. } | Moment::BigUpgrade { item, .. } => vec![item],
            Moment::ClassQuestDone { title } | Moment::QuestDone { title, .. } => vec![title],
            Moment::QuestMarked { mark, quest } => vec![quest, mark],
            Moment::Flavor { book, .. } => book.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        }
    }

    /// A moment where the hero did something: every moment but an arrival and a flavor
    /// moment. A deed with thin lore is silence (`narrator_lore::is_silent`).
    #[must_use]
    pub fn is_deed(&self) -> bool {
        !self.is_arrival() && !matches!(self, Moment::Flavor { .. })
    }

    /// The hero only came to a place: a zone, a capital, a dungeon, or a raid. Such a
    /// line tells the place, and leaves the hero out (GAMEPLAY.md 3.2.1).
    #[must_use]
    pub fn is_arrival(&self) -> bool {
        matches!(
            self,
            Moment::NewZone { .. }
                | Moment::FirstCapital { .. }
                | Moment::FirstInstance { .. }
                | Moment::InstanceAgain { .. }
        )
    }

    /// An entry into a dungeon, a raid, or a battleground, first or later.
    #[must_use]
    pub fn enters_an_instance(&self) -> bool {
        matches!(
            self,
            Moment::FirstInstance { .. } | Moment::InstanceAgain { .. }
        )
    }

    /// A higher rank wins when one batch holds several moments.
    fn rank(&self) -> u8 {
        match self {
            Moment::ClassQuestDone { .. } => 7,
            Moment::Revenge { .. } | Moment::Titled { .. } | Moment::FirstEpicMount { .. } => 6,
            Moment::Flavor { .. } => 0,
            Moment::FirstKill { .. }
            | Moment::FirstInstance { .. }
            | Moment::FirstMount { .. }
            | Moment::FirstEpicItem { .. } => 5,
            Moment::SlainAgain { .. } => 4,
            Moment::Slapped { .. }
            | Moment::QuestMarked { .. }
            | Moment::BigUpgrade { .. }
            | Moment::QuestDone { .. } => 3,
            Moment::LevelUp { .. } | Moment::FirstCapital { .. } => 2,
            Moment::NewZone { .. } | Moment::InstanceAgain { .. } => 1,
        }
    }
}

#[must_use]
pub fn moments(world: &World, you: EntityId, events: &[Event]) -> Vec<Moment> {
    events
        .iter()
        .filter_map(|event| moment(world, you, event))
        .collect()
}

/// The last moment of the highest rank, because a later moment of one kind holds the newer
/// count: two level ups in one batch end at the second level.
#[must_use]
pub fn best(moments: Vec<Moment>) -> Option<Moment> {
    let mut best: Option<Moment> = None;
    for moment in moments {
        if best
            .as_ref()
            .is_none_or(|kept| moment.rank() >= kept.rank())
        {
            best = Some(moment);
        }
    }
    best
}

fn moment(world: &World, you: EntityId, event: &Event) -> Option<Moment> {
    let name_of = |id: &EntityId| world.entity(*id).map(|entity| entity.name.clone());
    match &event.kind {
        EventKind::FactStart {
            entity,
            name,
            value: Some(1),
            linked_to: Some(foe),
        } if *entity == you && name == DEFEATED => first_kill(world, you, *foe),
        EventKind::FactUpdate {
            entity: killer,
            name,
            linked_to: Some(target),
            to,
            ..
        } if *target == you && name == DEFEATED => Some(Moment::SlainAgain {
            killer: name_of(killer)?,
            times: *to,
            zone: zone_of(world, you),
        }),
        EventKind::FactStart {
            entity,
            name,
            value: Some(times),
            linked_to: Some(npc),
        }
        | EventKind::FactUpdate {
            entity,
            name,
            to: times,
            linked_to: Some(npc),
            ..
        } if *entity == you && name == SLAPPED => Some(Moment::Slapped {
            npc: name_of(npc)?,
            times: *times,
            zone: zone_of(world, you),
        }),
        EventKind::FactUpdate {
            entity, name, to, ..
        } if *entity == you && name == LEVEL && is_milestone(*to) => Some(Moment::LevelUp {
            level: *to,
            zone: zone_of(world, you),
        }),
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(place),
            ..
        } if *entity == you && name == VISITED && world.location_of(*place).is_none() => {
            let zone = name_of(place)?;
            if is_capital(&zone) {
                return Some(Moment::FirstCapital { city: zone });
            }
            Some(Moment::NewZone { zone })
        }
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(mark),
            ..
        } if *entity == you && name == MARKED_BY => {
            let (mark, quest) = mark_and_quest(world, *mark)?;
            Some(Moment::QuestMarked { mark, quest })
        }
        EventKind::FactStart {
            entity: place,
            name,
            linked_to: None,
            ..
        } if name == DUNGEON || name == RAID || name == BATTLEGROUND => {
            first_instance(world, *place, name)
        }
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(title),
            ..
        } if *entity == you && name == TITLE => Some(Moment::Titled {
            title: name_of(title)?,
        }),
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(thing),
            ..
        } if *entity == you && is_gear(name) => gear_moment(world, you, name, *thing),
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(place),
            ..
        } if *entity == you && name == LOCATED_IN => instance_again(world, you, *place, event),
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(quest),
            ..
        } if *entity == you => quest_moment(world, name, *quest),
        _ => None,
    }
}

/// A finished class quest, or a finished side quest of Timeways.
fn quest_moment(world: &World, fact: &str, quest: EntityId) -> Option<Moment> {
    match fact {
        GAME_QUEST_DONE => {
            let quest = world.entity(quest)?;
            quest.fact(CLASS_QUEST, None)?;
            Some(Moment::ClassQuestDone {
                title: title_of_game_quest(&quest.name)?.to_string(),
            })
        }
        QUEST_DONE => side_quest(world, quest),
        _ => None,
    }
}

/// A first kill, or a revenge when the foe killed you before.
fn first_kill(world: &World, you: EntityId, foe: EntityId) -> Option<Moment> {
    let entity = world.entity(foe)?;
    let zone = zone_of(world, you);
    let deaths = entity
        .fact(DEFEATED, Some(you))
        .and_then(|fact| fact.value)
        .filter(|deaths| *deaths > 0);
    if let Some(deaths) = deaths {
        return Some(Moment::Revenge {
            foe: entity.name.clone(),
            deaths,
            zone,
        });
    }
    let creature = entity.has(ANIMAL).then_some(Creature::Beast);
    Some(Moment::FirstKill {
        foe: entity.name.clone(),
        zone,
        creature,
    })
}

fn side_quest(world: &World, quest: EntityId) -> Option<Moment> {
    let name = &world.entity(quest)?.name;
    let title = title_of_thing(name)?.to_string();
    let giver = world
        .holders_of(QUEST_OFFERED, quest)
        .first()
        .and_then(|giver| world.entity(*giver))
        .map(|giver| giver.name.clone());
    Some(Moment::QuestDone { title, giver })
}

fn first_instance(world: &World, place: EntityId, fact: &str) -> Option<Moment> {
    let kind = match fact {
        RAID => InstanceKind::Raid,
        BATTLEGROUND => InstanceKind::Battleground,
        _ => InstanceKind::Dungeon,
    };
    let zone = world.entity(place)?.name.clone();
    Some(Moment::FirstInstance { zone, kind })
}

/// You stand in a dungeon or a raid again, and you stood outside it before this move. A
/// move inside it, from one of its subzones to another, is no entry. The first entry
/// comes before the mark of the instance, so it is never this moment.
fn instance_again(world: &World, you: EntityId, place: EntityId, event: &Event) -> Option<Moment> {
    let zone = zone_around(world, place);
    let entity = world.entity(zone)?;
    let kind = if entity.has(RAID) {
        InstanceKind::Raid
    } else if entity.has(DUNGEON) {
        InstanceKind::Dungeon
    } else {
        return None;
    };
    let before = place_before(world, you, event)?;
    if zone_around(world, before) == zone {
        return None;
    }
    Some(Moment::InstanceAgain {
        zone: entity.name.clone(),
        kind,
    })
}

fn zone_around(world: &World, place: EntityId) -> EntityId {
    world.ancestry(place).last().copied().unwrap_or(place)
}

/// Where you stood before the move of `event`.
fn place_before(world: &World, you: EntityId, event: &Event) -> Option<EntityId> {
    world
        .history()
        .iter()
        .rev()
        .skip_while(|earlier| earlier.id >= event.id)
        .find_map(|earlier| match &earlier.kind {
            EventKind::FactStart {
                entity,
                name,
                linked_to: Some(place),
                ..
            } if *entity == you && name == LOCATED_IN => Some(*place),
            _ => None,
        })
}

/// The facts of a first mount, a first epic item, and a big upgrade.
fn is_gear(fact: &str) -> bool {
    [FIRST_MOUNT, FIRST_EPIC_MOUNT, FIRST_EPIC_ITEM, UPGRADED].contains(&fact)
}

fn gear_moment(world: &World, you: EntityId, fact: &str, thing: EntityId) -> Option<Moment> {
    let name = &world.entity(thing)?.name;
    if let Some(mount) = title_of_mount(name) {
        let people = people_of(mount, race_of(world, you)).map(str::to_string);
        let mount = mount.to_string();
        return match fact {
            FIRST_MOUNT => Some(Moment::FirstMount { mount, people }),
            FIRST_EPIC_MOUNT => Some(Moment::FirstEpicMount { mount, people }),
            _ => None,
        };
    }
    let item = title_of_item(name)?.to_string();
    let zone = zone_of(world, you);
    let slot = slot_of(world, you, thing);
    match fact {
        FIRST_EPIC_ITEM => Some(Moment::FirstEpicItem { item, zone, slot }),
        UPGRADED => Some(Moment::BigUpgrade { item, zone, slot }),
        _ => None,
    }
}

/// The slot of an item, from its big upgrade. A first epic item that was no upgrade has
/// no known slot.
fn slot_of(world: &World, you: EntityId, thing: EntityId) -> Option<SlotKind> {
    let slot = world.entity(you)?.fact(UPGRADED, Some(thing))?.value?;
    SlotKind::of(slot)
}

fn race_of(world: &World, you: EntityId) -> Option<Race> {
    world
        .entity(you)?
        .facts_named(RACE)
        .filter_map(|fact| world.entity(fact.linked_to?))
        .find_map(|race| Race::from_word(&race.name))
}

/// A plain level up has nothing to tell, so only every tenth level speaks.
fn is_milestone(level: i64) -> bool {
    level % LEVEL_STEP == 0
}

/// The zone where you stand: the outermost place around you.
fn zone_of(world: &World, you: EntityId) -> Option<String> {
    let here = world.location_of(you)?;
    let zone = world.ancestry(here).last().copied().unwrap_or(here);
    world.entity(zone).map(|zone| zone.name.clone())
}
