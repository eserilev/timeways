//! Flavor moments and their score (GAMEPLAY.md 5.4.1). Code scores each moment, with whole
//! numbers, so a test can state each rule. No model takes part.

use crate::character::Character;
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// A moment that the spec example scores: a dance in Goldshire, or a death in a capital.
/// Names of the game, not claims of lore.
const FAMOUS_PLACES: [&str; 7] = [
    "Goldshire",
    "Stormwind City",
    "Ironforge",
    "Darnassus",
    "Orgrimmar",
    "Thunder Bluff",
    "Undercity",
];

/// The local hours that make a moment odd: "a dance in Goldshire at 3 AM".
const ODD_HOURS: std::ops::RangeInclusive<u8> = 2..=5;

/// A telling counts against its kind for this long: "no two rabbit jokes in one evening".
pub const TOLD_SECONDS: u64 = 72 * 3600;

/// A killer this far below your level makes a death a flavor moment.
pub const HUMBLING_GAP: i64 = 10;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flavor {
    pub at: Tick,
    /// The local hour of the player, from 0 to 23.
    pub hour: Option<u8>,
    /// Where you stood: the subzone, or the zone.
    pub place: Option<String>,
    #[serde(flatten)]
    pub kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    /// `target` is an NPC, never a player (5.11).
    Emote {
        emote: String,
        target: Option<String>,
    },
    /// A death to the world itself, from the death recap: "falling", "drowning", "lava".
    FellTo { cause: String },
    /// A death to an NPC far below your level: "a level 60 killed by a cow".
    Humbled { killer: String, gap: i64 },
}

impl Kind {
    /// The kind for "first time", "rare for you", and "told before": a dance is a dance,
    /// wherever it happens.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Kind::Emote { emote, .. } => format!("emote:{emote}"),
            Kind::FellTo { cause } => format!("fell:{cause}"),
            Kind::Humbled { .. } => "humbled".to_string(),
        }
    }

    /// The NPC that the moment touches, for a callback.
    fn npc(&self) -> Option<&str> {
        match self {
            Kind::Emote { target, .. } => target.as_deref(),
            Kind::Humbled { killer, .. } => Some(killer),
            Kind::FellTo { .. } => None,
        }
    }
}

/// One telling of a kind of joke, by the companion or the bard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Told {
    pub key: String,
    pub at: Tick,
}

/// The score of `flavor`, given the moments before it and the tellings so far.
#[must_use]
pub fn score(flavor: &Flavor, earlier: &[Flavor], told: &[Told], character: &Character) -> i64 {
    let key = flavor.kind.key();
    let same = earlier
        .iter()
        .filter(|moment| moment.kind.key() == key)
        .count();
    let callback = flavor
        .kind
        .npc()
        .is_some_and(|npc| character.has_history_with(npc));
    let famous = flavor
        .place
        .as_deref()
        .is_some_and(|place| FAMOUS_PLACES.contains(&place));
    let odd_hour = flavor.hour.is_some_and(|hour| ODD_HOURS.contains(&hour));
    first_time(same)
        + rare_for_you(same)
        + contrast(&flavor.kind)
        + if famous { 3 } else { 0 }
        + if callback { 4 } else { 0 }
        + if odd_hour { 2 } else { 0 }
        - 3 * told_before(&key, flavor.at, told)
}

fn first_time(same: usize) -> i64 {
    if same == 0 { 5 } else { 0 }
}

/// "The rarer in your own history, the more. The 50th rabbit scores low."
fn rare_for_you(same: usize) -> i64 {
    match same {
        1..=2 => 4,
        3..=9 => 3,
        10..=24 => 2,
        25..=49 => 1,
        _ => 0,
    }
}

/// +1 for each 10 levels of the gap, at most +4.
fn contrast(kind: &Kind) -> i64 {
    match kind {
        Kind::Humbled { gap, .. } => (gap / HUMBLING_GAP).clamp(0, 4),
        _ => 0,
    }
}

fn told_before(key: &str, at: Tick, told: &[Told]) -> i64 {
    let recent = told
        .iter()
        .filter(|telling| telling.key == key)
        .filter(|telling| telling.at <= at && at.0 - telling.at.0 < TOLD_SECONDS)
        .count();
    i64::try_from(recent).unwrap_or(i64::MAX / 3)
}
