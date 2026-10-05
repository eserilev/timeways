//! What kind of place a zone is: a plain zone, a dungeon, a raid, or a capital city
//! (GAMEPLAY.md 3.3). The kind goes into the facts of a saga and the narrator line.

use crate::vocabulary::{BATTLEGROUND, DUNGEON, RAID};
use hourglass::{EntityId, World};
use serde::{Deserialize, Serialize};

/// The six capitals of the classic world.
pub const CAPITALS: [&str; 6] = [
    "Stormwind City",
    "Ironforge",
    "Darnassus",
    "Orgrimmar",
    "Thunder Bluff",
    "Undercity",
];

/// What `IsInInstance` of the game says about an instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceKind {
    #[serde(rename = "party")]
    Dungeon,
    #[serde(rename = "raid")]
    Raid,
    #[serde(rename = "pvp")]
    Battleground,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    #[default]
    Zone,
    Dungeon,
    Raid,
    Battleground,
    Capital,
}

impl PlaceKind {
    /// The words after the name in a fact: "The Deadmines (a dungeon)".
    #[must_use]
    pub fn described(self, name: &str) -> String {
        match self {
            PlaceKind::Zone => name.to_string(),
            PlaceKind::Dungeon => format!("{name} (a dungeon)"),
            PlaceKind::Raid => format!("{name} (a raid)"),
            PlaceKind::Battleground => format!("{name} (a battleground)"),
            PlaceKind::Capital => format!("{name} (a capital city)"),
        }
    }
}

#[must_use]
pub fn is_capital(name: &str) -> bool {
    CAPITALS.contains(&name)
}

/// A zone that the game called an instance is one, and a capital is known by its name.
#[must_use]
pub fn kind_of(world: &World, place: EntityId) -> PlaceKind {
    let Some(entity) = world.entity(place) else {
        return PlaceKind::Zone;
    };
    if entity.fact(RAID, None).is_some() {
        return PlaceKind::Raid;
    }
    if entity.fact(DUNGEON, None).is_some() {
        return PlaceKind::Dungeon;
    }
    if entity.fact(BATTLEGROUND, None).is_some() {
        return PlaceKind::Battleground;
    }
    let is_zone = world.location_of(place).is_none();
    if is_zone && is_capital(&entity.name) {
        return PlaceKind::Capital;
    }
    PlaceKind::Zone
}
