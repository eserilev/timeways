//! The fact names of a Timeways world, and their rules (GAMEPLAY.md 5.1).

use crate::spot::{MAP_IDS, THOUSANDTHS};
use hourglass::EntityType::{Faction, Person, Place, Thing};
use hourglass::{Band, Count, Direction, EntityType, FactRules, FactVocabulary, Shape};

/// A change to a declared name needs a new version and a migration (`hourglass::migrate`).
pub const VERSION: u32 = 9;

pub const MET: &str = "met";
/// You saw this NPC, by a hover or a target. Talking is meeting; seeing is not.
pub const SEEN: &str = "seen";
/// You can attack this NPC. The last sighting decides, so it can end.
pub const HOSTILE: &str = "hostile";
/// A beast or a critter: no one to talk to.
pub const ANIMAL: &str = "animal";
pub const TRUSTS: &str = "trusts";
pub const VISITED: &str = "visited";
pub const KNOWS_LORE: &str = "knows_lore";
pub const DEAD: &str = "dead";
/// Your own deaths, with a known killer or not. A known killer also holds `defeated`.
pub const DEATHS: &str = "deaths";
pub const DEFEATED: &str = "defeated";
pub const NEMESIS: &str = "nemesis";
pub const QUEST_OFFERED: &str = "quest_offered";
pub const QUEST_ACCEPTED: &str = "quest_accepted";
pub const QUEST_DONE: &str = "quest_done";
/// A quest of the game, not a side quest of Timeways.
pub const GAME_QUEST_TAKEN: &str = "game_quest_taken";
pub const GAME_QUEST_DONE: &str = "game_quest_done";
/// A quest of the game that only your class gets. The quest holds it.
pub const CLASS_QUEST: &str = "class_quest";
/// An instance of the game. The place holds it.
pub const DUNGEON: &str = "dungeon";
/// A lasting buff or debuff that a quest of the game put on you.
pub const MARKED_BY: &str = "marked_by";
/// The quest that put the mark. The mark holds it.
pub const MARK_OF: &str = "mark_of";
pub const RAID: &str = "raid";
pub const LEVEL: &str = "level";
pub const SLAPPED: &str = "slapped";
pub const TITLE: &str = "title";
/// The race of the character. The world keeps its word: "Forsaken", "night elf".
pub const RACE: &str = "race";
/// The class of the character, as its word: "paladin".
pub const CLASS: &str = "class";
/// The first mount that you rode, once in a life. You hold it about the mount.
pub const FIRST_MOUNT: &str = "first_mount";
/// The first mount that ran at the speed of an epic mount, once in a life.
pub const FIRST_EPIC_MOUNT: &str = "first_epic_mount";
/// The first item of epic quality that you put on, once in a life.
pub const FIRST_EPIC_ITEM: &str = "first_epic_item";
/// An item far better than what its slot held. The value is the inventory slot.
pub const UPGRADED: &str = "upgraded";
/// The quality of an item, as the number of the game: 3 rare, 4 epic. The item holds it.
pub const QUALITY: &str = "quality";
pub const MEMBER_OF: &str = "member_of";
pub const LEADER_OF: &str = "leader_of";
/// The map of the game where a place began or an NPC was met. `map_x` and `map_y` give
/// the point on it.
pub const ON_MAP: &str = "on_map";
pub const MAP_X: &str = "map_x";
pub const MAP_Y: &str = "map_y";

/// The band of `trusts`. Public, because a change of trust stops at its ends.
pub const TRUST: Band = Band {
    min: timeways_rules::trust::MIN_TRUST,
    max: timeways_rules::trust::MAX_TRUST,
};
/// The levels of the game.
pub const LEVELS: Band = Band { min: 1, max: 60 };
/// The inventory slots of the game.
pub const SLOT_NUMBERS: Band = Band { min: 1, max: 19 };
/// The item qualities of Classic, from poor (0) to legendary (5).
pub const QUALITIES: Band = Band { min: 0, max: 5 };
/// The band of the counts: deaths, kills, and slaps. Public, because a count stops at its
/// top.
pub const TALLY: Band = Band { min: 0, max: 1000 };

#[must_use]
pub fn vocabulary() -> FactVocabulary {
    let mut vocabulary = FactVocabulary::new(VERSION);
    vocabulary
        .declare(MET, linked(up_flag(), Person, &[Person]))
        .declare(SEEN, linked(up_flag(), Person, &[Person]))
        .declare(HOSTILE, FactRules::solo(Shape::flag()))
        .declare(ANIMAL, FactRules::solo(up_flag()))
        .declare(TRUSTS, linked(Shape::number(TRUST), Person, &[Person]))
        .declare(VISITED, linked(up_flag(), Person, &[Place]))
        .declare(KNOWS_LORE, linked(up_flag(), Person, &[Thing, Place]))
        .declare(DEAD, FactRules::solo(up_flag()))
        .declare(
            DEFEATED,
            linked(up_tally(), Person, &[Person]).allowing(Faction, &[Person]),
        )
        .declare(NEMESIS, linked(Shape::number(TALLY), Person, &[Person]))
        .declare(QUEST_OFFERED, linked(up_flag(), Person, &[Thing]))
        .declare(QUEST_ACCEPTED, linked(up_flag(), Person, &[Thing]))
        .declare(QUEST_DONE, linked(up_flag(), Person, &[Thing]))
        .declare(GAME_QUEST_TAKEN, linked(up_flag(), Person, &[Thing]))
        .declare(GAME_QUEST_DONE, linked(up_flag(), Person, &[Thing]))
        .declare(CLASS_QUEST, FactRules::solo(up_flag()))
        .declare(DUNGEON, FactRules::solo(up_flag()))
        .declare(MARKED_BY, linked(up_flag(), Person, &[Thing]))
        .declare(MARK_OF, linked(up_flag(), Thing, &[Thing]))
        .declare(RAID, FactRules::solo(up_flag()))
        .declare(
            LEVEL,
            FactRules::solo(Shape::number(LEVELS).moving(Direction::Up)),
        )
        .declare(DEATHS, FactRules::solo(up_tally()))
        .declare(SLAPPED, linked(up_tally(), Person, &[Person]))
        .declare(TITLE, linked(up_flag(), Person, &[Thing]))
        .declare(RACE, linked(up_flag(), Person, &[Thing]))
        .declare(CLASS, linked(up_flag(), Person, &[Thing]))
        .declare(FIRST_MOUNT, linked(up_flag(), Person, &[Thing]))
        .declare(FIRST_EPIC_MOUNT, linked(up_flag(), Person, &[Thing]))
        .declare(FIRST_EPIC_ITEM, linked(up_flag(), Person, &[Thing]))
        .declare(
            UPGRADED,
            linked(Shape::number(SLOT_NUMBERS), Person, &[Thing]),
        )
        .declare(QUALITY, FactRules::solo(Shape::number(QUALITIES)))
        .declare(MEMBER_OF, linked(Shape::flag(), Person, &[Faction]))
        .declare(LEADER_OF, linked(Shape::flag(), Person, &[Faction]))
        .declare(ON_MAP, FactRules::solo(Shape::number(MAP_IDS)))
        .declare(MAP_X, FactRules::solo(Shape::number(THOUSANDTHS)))
        .declare(MAP_Y, FactRules::solo(Shape::number(THOUSANDTHS)));
    vocabulary
}

fn linked(shape: Shape, holder: EntityType, targets: &[EntityType]) -> FactRules {
    FactRules::linked(shape, Count::Many, Count::Many).allowing(holder, targets)
}

fn up_flag() -> Shape {
    Shape::flag().moving(Direction::Up)
}

fn up_tally() -> Shape {
    Shape::number(TALLY).moving(Direction::Up)
}
