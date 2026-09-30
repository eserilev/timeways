//! Where on a map of the game something happened: the place of a first visit, or of a
//! meeting (GAMEPLAY.md 3.6). The journal map draws its pins from these.

use crate::vocabulary::{MAP_X, MAP_Y, ON_MAP};
use hourglass::{Band, EntityId, World};
use serde::{Deserialize, Deserializer, Serialize};

/// The ids of `C_Map`. A classic zone map has an id of a few thousand.
pub const MAP_IDS: Band = Band {
    min: 1,
    max: 1_000_000,
};

/// A point from the left or the top edge of a map, in thousandths of its width or height.
pub const THOUSANDTHS: Band = Band { min: 0, max: 1000 };

/// A map of `C_Map`, by its id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct MapId(i64);

/// Whole thousandths, because the journal of the bridge takes no fractions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct Thousandths(i64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spot {
    pub map: MapId,
    pub x: Thousandths,
    pub y: Thousandths,
}

#[derive(Debug, thiserror::Error)]
#[error("{0} is outside its band")]
pub struct OutOfBand(i64);

impl TryFrom<i64> for MapId {
    type Error = OutOfBand;

    fn try_from(id: i64) -> Result<Self, Self::Error> {
        MAP_IDS.holds(id).then_some(MapId(id)).ok_or(OutOfBand(id))
    }
}

impl From<MapId> for i64 {
    fn from(id: MapId) -> i64 {
        id.0
    }
}

impl TryFrom<i64> for Thousandths {
    type Error = OutOfBand;

    fn try_from(n: i64) -> Result<Self, Self::Error> {
        THOUSANDTHS
            .holds(n)
            .then_some(Thousandths(n))
            .ok_or(OutOfBand(n))
    }
}

impl From<Thousandths> for i64 {
    fn from(n: Thousandths) -> i64 {
        n.0
    }
}

/// A position that does not read is no position. The event still counts, because the
/// position is only a detail of it.
///
/// # Errors
///
/// Returns the error of the deserializer only for input that is no JSON value at all.
pub fn lenient<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Spot>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

/// The position that the world holds for a place or an NPC, if any.
#[must_use]
pub fn spot_of(world: &World, entity: EntityId) -> Option<Spot> {
    let entity = world.entity(entity)?;
    let read = |name: &str| entity.value(name);
    Some(Spot {
        map: MapId::try_from(read(ON_MAP)?).ok()?,
        x: Thousandths::try_from(read(MAP_X)?).ok()?,
        y: Thousandths::try_from(read(MAP_Y)?).ok()?,
    })
}
