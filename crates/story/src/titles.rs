//! The joke titles of the journal (GAMEPLAY.md 5.4.1). Each one is a rule over the flavor
//! moments and the world. A title is earned once, and it stays: the world keeps it as a
//! `title` fact.

use crate::character::Character;
use crate::flavor::{Flavor, Kind};

struct Rule {
    title: &'static str,
    earned: fn(&[Flavor], &Character) -> bool,
}

const RULES: [Rule; 7] = [
    Rule {
        title: "Lord of the Goldshire Dance Floor",
        earned: |moments, _| dances(moments, Some("Goldshire")) >= 3,
    },
    Rule {
        title: "Dance Machine",
        earned: |moments, _| dances(moments, None) >= 25,
    },
    Rule {
        title: "Friend of Gravity",
        earned: |moments, _| deaths_to(moments, &["falling"]) >= 3,
    },
    Rule {
        title: "Student of the Deep",
        earned: |moments, _| deaths_to(moments, &["drowning"]) >= 3,
    },
    Rule {
        title: "Lava Enthusiast",
        earned: |moments, _| deaths_to(moments, &["lava", "fire"]) >= 2,
    },
    Rule {
        title: "The Humbled",
        earned: |moments, _| {
            moments
                .iter()
                .any(|m| matches!(m.kind, Kind::Humbled { .. }))
        },
    },
    Rule {
        title: "Slap Happy",
        earned: |_, character| character.slaps() >= 5,
    },
];

/// Every title whose rule holds, in the order of the rules.
#[must_use]
pub fn earned(moments: &[Flavor], character: &Character) -> Vec<&'static str> {
    RULES
        .iter()
        .filter(|rule| (rule.earned)(moments, character))
        .map(|rule| rule.title)
        .collect()
}

/// Dances in `place`, or anywhere with None.
fn dances(moments: &[Flavor], place: Option<&str>) -> usize {
    moments
        .iter()
        .filter(|moment| matches!(&moment.kind, Kind::Emote { emote, .. } if emote == "dance"))
        .filter(|moment| place.is_none() || moment.place.as_deref() == place)
        .count()
}

fn deaths_to(moments: &[Flavor], causes: &[&str]) -> usize {
    moments
        .iter()
        .filter(|moment| matches!(&moment.kind, Kind::FellTo { cause } if causes.contains(&cause.as_str())))
        .count()
}
