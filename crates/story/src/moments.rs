//! The big moments of a batch, found in the events that the batch added to the history
//! (GAMEPLAY.md 3.2). The narrator speaks about the best one.

use crate::character::title_of_game_quest;
use crate::places::{InstanceKind, is_capital};
use crate::vocabulary::{
    CLASS_QUEST, DEFEATED, DUNGEON, GAME_QUEST_DONE, LEVEL, RAID, SLAPPED, TITLE, VISITED,
};
use hourglass::{EntityId, Event, EventKind, World};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// A small, silly moment in plain words (5.4.1). It speaks only when no big moment does.
    Flavor {
        what: String,
    },
    /// A joke title, the rarest moment of all (5.4.1).
    Titled {
        title: String,
    },
    /// The true kill of a rare or a boss (5.13).
    FirstKill {
        foe: String,
    },
    /// The same NPC killed you again: "Third time this murloc got you."
    SlainAgain {
        killer: String,
        times: i64,
    },
    /// "The innkeeper remembers it."
    Slapped {
        npc: String,
        times: i64,
    },
    LevelUp {
        level: i64,
    },
    /// A finished quest of your class: a turn of your own story.
    ClassQuestDone {
        title: String,
    },
    /// The first visit of a zone, not of a subzone.
    NewZone {
        zone: String,
    },
    /// The first entry into a dungeon or a raid: a zone that the game called an instance.
    FirstInstance {
        zone: String,
        kind: InstanceKind,
    },
    /// The first visit of a capital city.
    FirstCapital {
        city: String,
    },
}

impl Moment {
    /// A higher rank wins when one batch holds several moments.
    fn rank(&self) -> u8 {
        match self {
            Moment::ClassQuestDone { .. } => 7,
            Moment::Titled { .. } => 6,
            Moment::Flavor { .. } => 0,
            Moment::FirstKill { .. } | Moment::FirstInstance { .. } => 5,
            Moment::SlainAgain { .. } => 4,
            Moment::Slapped { .. } => 3,
            Moment::LevelUp { .. } | Moment::FirstCapital { .. } => 2,
            Moment::NewZone { .. } => 1,
        }
    }
}

#[must_use]
pub fn moments(world: &World, you: EntityId, events: &[Event]) -> Vec<Moment> {
    events
        .iter()
        .filter_map(|event| moment(world, you, &event.kind))
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

fn moment(world: &World, you: EntityId, kind: &EventKind) -> Option<Moment> {
    let name_of = |id: &EntityId| world.entity(*id).map(|entity| entity.name.clone());
    match kind {
        EventKind::FactStart {
            entity,
            name,
            value: Some(1),
            linked_to: Some(foe),
        } if *entity == you && name == DEFEATED => Some(Moment::FirstKill { foe: name_of(foe)? }),
        EventKind::FactUpdate {
            entity: killer,
            name,
            linked_to: Some(target),
            to,
            ..
        } if *target == you && name == DEFEATED => Some(Moment::SlainAgain {
            killer: name_of(killer)?,
            times: *to,
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
        }),
        EventKind::FactUpdate {
            entity, name, to, ..
        } if *entity == you && name == LEVEL => Some(Moment::LevelUp { level: *to }),
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
            entity: place,
            name,
            linked_to: None,
            ..
        } if name == DUNGEON || name == RAID => {
            let kind = if name == RAID {
                InstanceKind::Raid
            } else {
                InstanceKind::Dungeon
            };
            Some(Moment::FirstInstance {
                zone: name_of(place)?,
                kind,
            })
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
            linked_to: Some(quest),
            ..
        } if *entity == you && name == GAME_QUEST_DONE => {
            let quest = world.entity(*quest)?;
            quest.fact(CLASS_QUEST, None)?;
            Some(Moment::ClassQuestDone {
                title: title_of_game_quest(&quest.name)?.to_string(),
            })
        }
        _ => None,
    }
}
