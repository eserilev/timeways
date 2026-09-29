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

/// A telling lowers the score of its kind for this long.
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
    /// A book that you read for the first time (GAMEPLAY.md 3.1.1).
    Read { title: String },
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
            Kind::Read { .. } => "read".to_string(),
        }
    }

    /// The NPC that the moment touches, for a callback.
    fn npc(&self) -> Option<&str> {
        match self {
            Kind::Emote { target, .. } => target.as_deref(),
            Kind::Humbled { killer, .. } => Some(killer),
            Kind::FellTo { .. } | Kind::Read { .. } => None,
        }
    }
}

/// One telling of a kind of joke, by the narrator or the bard.
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

/// +1 for each 10 levels of the gap, at most +4. A hero who reads is odd enough for +3, so
/// the first book reaches the narrator.
fn contrast(kind: &Kind) -> i64 {
    match kind {
        Kind::Humbled { gap, .. } => (gap / HUMBLING_GAP).clamp(0, 4),
        Kind::Read { .. } => 3,
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

/// The moment in plain words, for the prompt of the narrator. `count` is 1 for the first
/// moment of its kind.
#[must_use]
pub fn describe(flavor: &Flavor, count: usize) -> String {
    let place = flavor
        .place
        .as_deref()
        .map(|place| format!(" in {place}"))
        .unwrap_or_default();
    let when = flavor
        .hour
        .map(|hour| format!(", at {hour} o'clock"))
        .unwrap_or_default();
    let nth = ordinal(count);
    match &flavor.kind {
        Kind::Emote { emote, target } => {
            let at = target
                .as_deref()
                .map(|npc| format!(" at {npc}"))
                .unwrap_or_default();
            format!("The player used the emote /{emote}{at}{place}{when}, for the {nth} time.")
        }
        Kind::FellTo { cause } => {
            format!("The player died to {cause}{place}{when}, for the {nth} time.")
        }
        Kind::Humbled { killer, gap } => {
            format!("{killer}, {gap} levels below the player, killed the player{place}{when}.")
        }
        Kind::Read { title } => {
            format!("The player read \"{title}\"{place}{when}, the {nth} book that they read.")
        }
    }
}

fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// A flavor moment with its score, as the code gave it when it came in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scored {
    pub flavor: Flavor,
    pub score: i64,
    /// 1 for the first moment of its kind.
    pub count: usize,
}

/// The best `limit` moments from `began` to `ended`, best first. Each one is scored against
/// the moments and the tellings before it, as it was when it came in.
#[must_use]
pub fn top_moments(
    moments: &[Flavor],
    told: &[Told],
    character: &Character,
    (began, ended): (Tick, Tick),
    limit: usize,
) -> Vec<Scored> {
    let mut scored = Vec::new();
    for (index, flavor) in moments.iter().enumerate() {
        if flavor.at < began || flavor.at > ended {
            continue;
        }
        let earlier = &moments[..index];
        let told_before: Vec<Told> = told
            .iter()
            .filter(|telling| telling.at <= flavor.at)
            .cloned()
            .collect();
        let key = flavor.kind.key();
        let count = earlier.iter().filter(|old| old.kind.key() == key).count() + 1;
        let score = score(flavor, earlier, &told_before, character);
        scored.push(Scored {
            flavor: flavor.clone(),
            score,
            count,
        });
    }
    scored.sort_by_key(|moment| std::cmp::Reverse(moment.score));
    scored.truncate(limit);
    scored
}
