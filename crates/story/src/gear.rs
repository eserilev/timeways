//! The gear that the hero puts on: the first epic item, and a big upgrade over what a slot
//! held before (GAMEPLAY.md 3.2). The addon sends only items of rare quality or better.

use serde::{Deserialize, Serialize};

/// A big upgrade gains at least this many item levels over what the slot held. In Classic
/// the level of an item runs close to its required level plus 5, so 10 item levels are the
/// gear of about 10 more character levels. A quest reward or a dungeon drop of your own
/// level gains a few item levels, so it stays below.
pub const BIG_UPGRADE_LEVELS: u16 = 10;

/// The inventory slots of the game, from the head (1) to the tabard (19).
pub const SLOTS: std::ops::RangeInclusive<u8> = 1..=19;

/// The item quality of the game, as `C_Item.GetItemInfo` gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Quality {
    Poor,
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

const QUALITIES: [Quality; 6] = [
    Quality::Poor,
    Quality::Common,
    Quality::Uncommon,
    Quality::Rare,
    Quality::Epic,
    Quality::Legendary,
];

impl Quality {
    /// The quality of the game number. A number past Legendary (an artifact or an heirloom)
    /// does not exist in Classic, so it gives None.
    #[must_use]
    pub fn of_number(number: u8) -> Option<Quality> {
        QUALITIES.get(usize::from(number)).copied()
    }

    #[must_use]
    pub fn number(self) -> u8 {
        self as u8
    }

    /// Purple, or the orange above it.
    #[must_use]
    pub fn is_epic(self) -> bool {
        self >= Quality::Epic
    }
}

/// What the slot held before the new item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Before {
    /// Nothing, since the login.
    Empty,
    /// An item. Its level is None when the game did not tell it.
    Worn { level: Option<u16> },
}

/// A rare or better item with a known level, at least `BIG_UPGRADE_LEVELS` above what the
/// slot held. An empty slot counts as level 0. An unknown level on either side is no
/// upgrade, because silence is better than a wrong deed.
#[must_use]
pub fn is_big_upgrade(quality: Quality, level: Option<u16>, before: Before) -> bool {
    let Some(level) = level else {
        return false;
    };
    let old = match before {
        Before::Empty => 0,
        Before::Worn { level: Some(old) } => old,
        Before::Worn { level: None } => return false,
    };
    let gain = level.checked_sub(old);
    quality >= Quality::Rare && gain.is_some_and(|gain| gain >= BIG_UPGRADE_LEVELS)
}

const ITEM_PREFIX: &str = "item: ";

/// An item lives as a thing apart from the quests, the marks, and the titles.
#[must_use]
pub fn item_name(item: &str) -> String {
    format!("{ITEM_PREFIX}{item}")
}

/// The name of an item, from the name of its thing.
#[must_use]
pub fn title_of_item(name: &str) -> Option<&str> {
    name.strip_prefix(ITEM_PREFIX)
}
