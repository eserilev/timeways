//! The race and the class of the character, as the addon sends them (GAMEPLAY.md 3.2.1).
//! The narrator calls the hero by them: "the night elf", "the paladin".

use serde::{Deserialize, Serialize};

/// The races of Classic. The addon sends the file token of `UnitRace`, which does not
/// change with the language of the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Race {
    Human,
    Orc,
    Dwarf,
    NightElf,
    /// The game calls them "Undead", and its token is "Scourge". In the lore of 25 ADP
    /// they are the Forsaken.
    #[serde(rename = "Scourge")]
    Forsaken,
    Tauren,
    Gnome,
    Troll,
}

/// The classes of Classic, as the file token of `UnitClass`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Class {
    Warrior,
    Paladin,
    Hunter,
    Rogue,
    Priest,
    Shaman,
    Mage,
    Warlock,
    Druid,
}

const RACES: [Race; 8] = [
    Race::Human,
    Race::Orc,
    Race::Dwarf,
    Race::NightElf,
    Race::Forsaken,
    Race::Tauren,
    Race::Gnome,
    Race::Troll,
];

const CLASSES: [Class; 9] = [
    Class::Warrior,
    Class::Paladin,
    Class::Hunter,
    Class::Rogue,
    Class::Priest,
    Class::Shaman,
    Class::Mage,
    Class::Warlock,
    Class::Druid,
];

impl Race {
    /// The word of a sentence: "the Forsaken", "the night elf".
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Race::Human => "human",
            Race::Orc => "orc",
            Race::Dwarf => "dwarf",
            Race::NightElf => "night elf",
            Race::Forsaken => "Forsaken",
            Race::Tauren => "tauren",
            Race::Gnome => "gnome",
            Race::Troll => "troll",
        }
    }

    /// The race whose word this is. The world keeps the word.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Race> {
        RACES.into_iter().find(|race| race.word() == word)
    }

    /// The people of the race in the plural: "dwarves", "Forsaken". The people data of
    /// the templates holds the same plural, and a test compares them.
    #[must_use]
    pub fn plural(self) -> &'static str {
        match self {
            Race::Human => "humans",
            Race::Orc => "orcs",
            Race::Dwarf => "dwarves",
            Race::NightElf => "night elves",
            Race::Forsaken => "Forsaken",
            Race::Tauren => "tauren",
            Race::Gnome => "gnomes",
            Race::Troll => "trolls",
        }
    }

    /// The words of a line that name the people of the race as a group, past the
    /// plural: "the undead", "the elves".
    #[must_use]
    pub fn people_words(self) -> &'static [&'static str] {
        match self {
            Race::Dwarf => &["dwarfs"],
            Race::NightElf => &["elves"],
            Race::Forsaken => &["undead"],
            Race::Human | Race::Orc | Race::Tauren | Race::Gnome | Race::Troll => &[],
        }
    }

    /// True when the word names the whole people and never one person: "the Forsaken".
    /// "The tauren" is its own plural too, but it names one tauren as well.
    #[must_use]
    pub fn names_only_the_people(self) -> bool {
        self == Race::Forsaken
    }

    /// Every race of Classic.
    #[must_use]
    pub fn all() -> [Race; 8] {
        RACES
    }
}

impl Class {
    /// The word of a sentence: "the paladin".
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Class::Warrior => "warrior",
            Class::Paladin => "paladin",
            Class::Hunter => "hunter",
            Class::Rogue => "rogue",
            Class::Priest => "priest",
            Class::Shaman => "shaman",
            Class::Mage => "mage",
            Class::Warlock => "warlock",
            Class::Druid => "druid",
        }
    }

    /// The class whose word this is. The world keeps the word.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Class> {
        CLASSES.into_iter().find(|class| class.word() == word)
    }

    /// "paladins". The plural of "shaman" is "shaman", as the game writes it.
    #[must_use]
    pub fn plural(self) -> &'static str {
        match self {
            Class::Warrior => "warriors",
            Class::Paladin => "paladins",
            Class::Hunter => "hunters",
            Class::Rogue => "rogues",
            Class::Priest => "priests",
            Class::Shaman => "shaman",
            Class::Mage => "mages",
            Class::Warlock => "warlocks",
            Class::Druid => "druids",
        }
    }

    /// Every class of Classic.
    #[must_use]
    pub fn all() -> [Class; 9] {
        CLASSES
    }
}
