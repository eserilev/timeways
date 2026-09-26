//! The big moments of a batch, found in the events that the batch added to the history
//! (GAMEPLAY.md 3.2). The companion speaks about the best one.

use crate::vocabulary::{DEFEATED, LEVEL, VISITED};
use hourglass::{EntityId, Event, EventKind, World};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// The true kill of a rare or a boss (5.13).
    FirstKill {
        foe: String,
    },
    /// The same NPC killed you again: "Third time this murloc got you."
    SlainAgain {
        killer: String,
        times: i64,
    },
    LevelUp {
        level: i64,
    },
    /// The first visit of a zone, not of a subzone.
    NewZone {
        zone: String,
    },
}

impl Moment {
    /// A higher rank wins when one batch holds several moments.
    fn rank(&self) -> u8 {
        match self {
            Moment::FirstKill { .. } => 4,
            Moment::SlainAgain { .. } => 3,
            Moment::LevelUp { .. } => 2,
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

/// The first moment of the highest rank.
#[must_use]
pub fn best(moments: Vec<Moment>) -> Option<Moment> {
    let mut best: Option<Moment> = None;
    for moment in moments {
        if best.as_ref().is_none_or(|kept| moment.rank() > kept.rank()) {
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
        EventKind::FactUpdate {
            entity, name, to, ..
        } if *entity == you && name == LEVEL => Some(Moment::LevelUp { level: *to }),
        EventKind::FactStart {
            entity,
            name,
            linked_to: Some(place),
            ..
        } if *entity == you && name == VISITED && world.location_of(*place).is_none() => {
            Some(Moment::NewZone {
                zone: name_of(place)?,
            })
        }
        _ => None,
    }
}
