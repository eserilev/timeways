//! The mounts of the hero: the first one, and the first epic one (GAMEPLAY.md 3.2). The
//! addon sends the run speed on the mount, so this module decides what is epic.

use crate::check::words_of;
use crate::race_class::Race;

/// The run speed of an epic mount, in percent of a run on foot. A mount of level 40 runs
/// at 160, and up to about 170 with a speed item, such as Mithril Spurs or the Carrot on a
/// Stick. An epic mount runs at 200. 180 lies between the two.
pub const EPIC_MOUNT_SPEED: u16 = 180;

/// A ride at this speed, in percent of a run on foot, is a ride on an epic mount. An
/// unknown speed is no epic ride.
#[must_use]
pub fn is_epic(speed: Option<u16>) -> bool {
    speed.is_some_and(|speed| speed >= EPIC_MOUNT_SPEED)
}

/// The word of a mount's name, and the place of the people who breed such mounts. A
/// skeletal horse is a horse of the Forsaken, so it comes before the horses of Stormwind.
const BREEDS: [(&str, &str); 16] = [
    ("skeletal", "Undercity"),
    ("ram", "Ironforge"),
    ("frostsaber", "Darnassus"),
    ("nightsaber", "Darnassus"),
    ("mistsaber", "Darnassus"),
    ("stormsaber", "Darnassus"),
    ("mechanostrider", "Gnomeregan"),
    ("wolf", "Orgrimmar"),
    ("kodo", "Thunder Bluff"),
    ("raptor", "Sen'jin Village"),
    ("horse", "Stormwind City"),
    ("stallion", "Stormwind City"),
    ("pinto", "Stormwind City"),
    ("palomino", "Stormwind City"),
    ("mare", "Stormwind City"),
    ("steed", "Stormwind City"),
];

/// The place of the people of the mount: by a word of its name, else by the race of the
/// hero. A class mount, such as a Felsteed, and a mount of another language have no word
/// in the list, so they take the people of the hero.
#[must_use]
pub fn people_of(mount: &str, race: Option<Race>) -> Option<&'static str> {
    let words = words_of(mount);
    let bred = BREEDS
        .iter()
        .find(|(word, _)| words.iter().any(|name| name == word))
        .map(|(_, place)| *place);
    bred.or_else(|| race.map(home_of))
}

/// The place of each people where its mounts come from.
fn home_of(race: Race) -> &'static str {
    match race {
        Race::Human => "Stormwind City",
        Race::Dwarf => "Ironforge",
        Race::NightElf => "Darnassus",
        Race::Gnome => "Gnomeregan",
        Race::Orc => "Orgrimmar",
        Race::Troll => "Sen'jin Village",
        Race::Tauren => "Thunder Bluff",
        Race::Forsaken => "Undercity",
    }
}

const MOUNT_PREFIX: &str = "mount: ";

/// A mount lives as a thing apart from the items, the quests, and the titles.
#[must_use]
pub fn mount_name(mount: &str) -> String {
    format!("{MOUNT_PREFIX}{mount}")
}

/// The name of a mount, from the name of its thing.
#[must_use]
pub fn title_of_mount(name: &str) -> Option<&str> {
    name.strip_prefix(MOUNT_PREFIX)
}
