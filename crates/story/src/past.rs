//! The past of a character before Timeways saw it, as the game tells it at a login
//! (GAMEPLAY.md 3.3, the prologue). It is no event of the world: a prologue tells it, and
//! the fold of the chapters never reads it, so no chapter moves.

use crate::story::MAX_NAME_BYTES;
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The quest titles of a past line. The addon sends the newest ones that the game knows.
pub const MAX_QUEST_TITLES: usize = 8;

/// The discovered zones of a past line. Classic has 40 or so zones outside instances.
pub const MAX_ZONES: usize = 48;

/// The factions of a past line, furthest from neutral first.
pub const MAX_FACTIONS: usize = 8;

/// The primary and secondary professions of Classic, and some room.
pub const MAX_PROFESSIONS: usize = 6;

/// The mounts of a past line.
pub const MAX_MOUNTS: usize = 3;

/// Classic ends at level 60. A higher level comes from a bug or a hostile addon.
pub const MAX_LEVEL: u8 = 60;

/// Ten years of play: a longer time comes from a bug.
pub const MAX_PLAYED_SECONDS: u64 = 10 * 365 * 24 * 3600;

/// The standing words of the game, from 1 (hated) to 8 (exalted).
const STANDINGS: [&str; 8] = [
    "hated",
    "hostile",
    "unfriendly",
    "neutral",
    "friendly",
    "honored",
    "revered",
    "exalted",
];

/// The facts of the past, as the addon read them at a login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Past {
    pub at: Tick,
    pub level: u8,
    /// The zone where the character stands at the login.
    #[serde(default)]
    pub zone: Option<String>,
    /// The time played in all, in seconds, when the game told it in time.
    #[serde(default)]
    pub played: Option<u64>,
    /// The count of quests of the game that the character turned in.
    #[serde(default)]
    pub quests: u32,
    #[serde(default)]
    pub quest_titles: Vec<String>,
    /// The zones that the map shows as discovered.
    #[serde(default)]
    pub zones: Vec<String>,
    #[serde(default)]
    pub factions: Vec<Standing>,
    #[serde(default)]
    pub professions: Vec<Profession>,
    #[serde(default)]
    pub mounts: Vec<String>,
    #[serde(default)]
    pub gear: Gear,
}

/// The standing of the character with one faction: 1 is hated, 8 exalted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub name: String,
    pub standing: u8,
}

/// A profession and its rank, such as Mining at 150.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profession {
    pub name: String,
    pub rank: u16,
}

/// The kept row of a past: the past, and whether the world held play of Timeways when it
/// came.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PastRow {
    pub past: Past,
    pub world: WorldAge,
}

/// A world with a closed chapter or a tale has play of Timeways already.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldAge {
    New,
    Played,
}

/// The count of worn items of rare and of epic quality.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gear {
    #[serde(default)]
    pub rare: u8,
    #[serde(default)]
    pub epic: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PastError {
    #[error("a past has a level outside 1 to {MAX_LEVEL}")]
    BadLevel,
    #[error("a past has a played time over ten years")]
    BadPlayed,
    #[error("a past has a list over its limit")]
    TooMany,
    #[error(
        "a name of a past is empty, longer than {MAX_NAME_BYTES} bytes, or holds a control character"
    )]
    BadName,
    #[error("a past has a standing outside 1 to 8")]
    BadStanding,
    #[error("a past has more worn items than the 19 slots")]
    BadGear,
}

/// The inventory slots of the game.
const SLOTS: u8 = 19;

impl Past {
    /// # Errors
    ///
    /// Returns the first rule that the past breaks.
    pub fn check(&self) -> Result<(), PastError> {
        if self.level == 0 || self.level > MAX_LEVEL {
            return Err(PastError::BadLevel);
        }
        if self
            .played
            .is_some_and(|played| played > MAX_PLAYED_SECONDS)
        {
            return Err(PastError::BadPlayed);
        }
        self.check_counts()?;
        self.names().try_for_each(checked_name)?;
        if self
            .factions
            .iter()
            .any(|faction| !(1..=8).contains(&faction.standing))
        {
            return Err(PastError::BadStanding);
        }
        if u16::from(self.gear.rare) + u16::from(self.gear.epic) > u16::from(SLOTS) {
            return Err(PastError::BadGear);
        }
        Ok(())
    }

    fn check_counts(&self) -> Result<(), PastError> {
        let over = self.quest_titles.len() > MAX_QUEST_TITLES
            || self.zones.len() > MAX_ZONES
            || self.factions.len() > MAX_FACTIONS
            || self.professions.len() > MAX_PROFESSIONS
            || self.mounts.len() > MAX_MOUNTS;
        if over {
            Err(PastError::TooMany)
        } else {
            Ok(())
        }
    }

    /// Every name of the past that the game wrote.
    fn names(&self) -> impl Iterator<Item = &str> {
        let factions = self.factions.iter().map(|faction| faction.name.as_str());
        let professions = self.professions.iter().map(|skill| skill.name.as_str());
        self.zone
            .iter()
            .chain(&self.quest_titles)
            .chain(&self.zones)
            .chain(&self.mounts)
            .map(String::as_str)
            .chain(factions)
            .chain(professions)
    }

    /// The zones of the past, where the character stands first, each once.
    #[must_use]
    pub fn places(&self) -> Vec<&str> {
        let mut places: Vec<&str> = Vec::new();
        for zone in self.zone.iter().chain(&self.zones) {
            if !places.contains(&zone.as_str()) {
                places.push(zone);
            }
        }
        places
    }

    /// The facts of the past in plain words, one on each line, as a prompt shows them. It
    /// tells no count: a prologue is history, not a ledger.
    #[must_use]
    pub fn facts(&self) -> Vec<String> {
        let mut facts = Vec::new();
        if let Some(zone) = &self.zone {
            facts.push(format!("Stands now in {zone}."));
        }
        if !self.zones.is_empty() {
            facts.push(format!("Has traveled in: {}.", self.zones.join(", ")));
        }
        if !self.quest_titles.is_empty() {
            let titles: Vec<String> = self
                .quest_titles
                .iter()
                .map(|title| format!("\"{title}\""))
                .collect();
            facts.push(format!(
                "Finished quests, among them {}.",
                titles.join(", ")
            ));
        }
        facts.extend(self.standing_fact());
        if !self.professions.is_empty() {
            let names: Vec<&str> = self
                .professions
                .iter()
                .map(|skill| skill.name.as_str())
                .collect();
            facts.push(format!("Practices {}.", names.join(", ")));
        }
        if !self.mounts.is_empty() {
            facts.push(format!("Rides {}.", self.mounts.join(", ")));
        }
        facts.extend(self.gear_fact());
        facts
    }

    fn standing_fact(&self) -> Option<String> {
        let standings: Vec<String> = self
            .factions
            .iter()
            .filter_map(|faction| {
                let word = STANDINGS.get(usize::from(faction.standing).checked_sub(1)?)?;
                Some(format!("{} ({word})", faction.name))
            })
            .collect();
        (!standings.is_empty()).then(|| format!("Stands with: {}.", standings.join(", ")))
    }

    fn gear_fact(&self) -> Option<String> {
        if self.gear.epic > 0 {
            return Some("Wears gear of the finest kind.".to_string());
        }
        (self.gear.rare > 0).then(|| "Wears rare gear.".to_string())
    }
}

fn checked_name(name: &str) -> Result<(), PastError> {
    let bad = name.is_empty() || name.len() > MAX_NAME_BYTES || name.chars().any(char::is_control);
    if bad { Err(PastError::BadName) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn past() -> Past {
        Past {
            at: Tick(100),
            level: 35,
            zone: Some("Stranglethorn Vale".to_string()),
            played: Some(400_000),
            quests: 212,
            quest_titles: vec!["The Defias Brotherhood".to_string()],
            zones: vec!["Elwynn Forest".to_string(), "Westfall".to_string()],
            factions: vec![Standing {
                name: "Stormwind".to_string(),
                standing: 6,
            }],
            professions: vec![Profession {
                name: "Mining".to_string(),
                rank: 150,
            }],
            mounts: vec!["Brown Horse".to_string()],
            gear: Gear { rare: 4, epic: 0 },
        }
    }

    #[test]
    fn a_full_past_passes() {
        assert_eq!(past().check(), Ok(()));
    }

    #[test]
    fn a_past_at_level_0_is_refused() {
        let zero = Past { level: 0, ..past() };

        assert_eq!(zero.check(), Err(PastError::BadLevel));
    }

    #[test]
    fn a_past_with_too_many_mounts_is_refused() {
        let many = Past {
            mounts: vec!["Horse".to_string(); MAX_MOUNTS + 1],
            ..past()
        };

        assert_eq!(many.check(), Err(PastError::TooMany));
    }

    #[test]
    fn a_standing_past_exalted_is_refused() {
        let mut odd = past();
        odd.factions[0].standing = 9;

        assert_eq!(odd.check(), Err(PastError::BadStanding));
    }

    #[test]
    fn a_name_with_a_control_character_is_refused() {
        let odd = Past {
            zone: Some("Elwynn\nForest".to_string()),
            ..past()
        };

        assert_eq!(odd.check(), Err(PastError::BadName));
    }

    #[test]
    fn the_facts_tell_no_count() {
        let facts = past().facts().join("\n");

        assert!(!facts.contains("212"));
        assert!(!facts.contains("400000"));
        assert!(!facts.contains("150"));
        assert!(facts.contains("Stormwind (honored)"));
    }

    #[test]
    fn the_places_start_where_the_character_stands() {
        let past = Past {
            zones: vec!["Westfall".to_string(), "Stranglethorn Vale".to_string()],
            ..past()
        };

        assert_eq!(past.places(), ["Stranglethorn Vale", "Westfall"]);
    }
}
