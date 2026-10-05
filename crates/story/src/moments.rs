//! The big moments of a batch, found in the events that the batch added to the history
//! (GAMEPLAY.md 3.2). The narrator speaks about the best one. Each moment holds a name or
//! a milestone, because a line with nothing concrete to tell is slop: silence is better.

use crate::chapters::LEVEL_STEP;
use crate::character::title_of_game_quest;
use crate::journal::mark_and_quest;
use crate::places::{InstanceKind, is_capital};
use crate::vocabulary::{
    CLASS_QUEST, DEFEATED, DUNGEON, GAME_QUEST_DONE, LEVEL, MARKED_BY, RAID, SLAPPED, TITLE,
    VISITED,
};
use hourglass::{EntityId, Event, EventKind, World};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// A small, silly moment in plain words (5.4.1). It speaks only when no big moment does.
    Flavor { what: String },
    /// A joke title, the rarest moment of all (5.4.1).
    Titled { title: String },
    /// The true kill of a rare or a boss (5.13).
    FirstKill { foe: String },
    /// The same NPC killed you again: "Third time this murloc got you."
    SlainAgain { killer: String, times: i64 },
    /// "The innkeeper remembers it."
    Slapped { npc: String, times: i64 },
    /// Only a milestone level: 10, 20, 30, and so on. `zone` is where it came.
    LevelUp { level: i64, zone: Option<String> },
    /// A finished quest of your class: a turn of your own story.
    ClassQuestDone { title: String },
    /// The first visit of a zone, not of a subzone.
    NewZone { zone: String },
    /// The first entry into a dungeon or a raid: a zone that the game called an instance.
    FirstInstance { zone: String, kind: InstanceKind },
    /// A quest of the game left a lasting buff or debuff on you.
    QuestMarked { mark: String, quest: String },
    /// The first visit of a capital city.
    FirstCapital { city: String },
}

impl Moment {
    /// The place, the foe, the person, or the quest of the moment: what its lore is about.
    /// A title and a flavor moment have no lore.
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        match self {
            Moment::Flavor { .. } | Moment::Titled { .. } => None,
            Moment::FirstKill { foe } => Some(foe),
            Moment::SlainAgain { killer, .. } => Some(killer),
            Moment::Slapped { npc, .. } => Some(npc),
            Moment::LevelUp { zone, .. } => zone.as_deref(),
            Moment::ClassQuestDone { title } => Some(title),
            Moment::NewZone { zone } | Moment::FirstInstance { zone, .. } => Some(zone),
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
            _ => self.subject().into_iter().collect(),
        }
    }

    /// The hero only came to a place: a zone, a capital, a dungeon, or a raid. Such a
    /// line tells the place, and leaves the hero out (GAMEPLAY.md 3.2.1).
    #[must_use]
    pub fn is_arrival(&self) -> bool {
        matches!(
            self,
            Moment::NewZone { .. } | Moment::FirstCapital { .. } | Moment::FirstInstance { .. }
        )
    }

    /// A higher rank wins when one batch holds several moments.
    fn rank(&self) -> u8 {
        match self {
            Moment::ClassQuestDone { .. } => 7,
            Moment::Titled { .. } => 6,
            Moment::Flavor { .. } => 0,
            Moment::FirstKill { .. } | Moment::FirstInstance { .. } => 5,
            Moment::SlainAgain { .. } => 4,
            Moment::Slapped { .. } | Moment::QuestMarked { .. } => 3,
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
