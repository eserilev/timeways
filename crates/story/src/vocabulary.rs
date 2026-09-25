//! The fact names of a Timeways world, and their rules (GAMEPLAY.md 5.1).

use hourglass::EntityType::{Faction, Person, Place, Thing};
use hourglass::{Band, Count, Direction, EntityType, FactRules, FactVocabulary, Shape};

/// A change to a declared name needs a new version and a migration (`hourglass::migrate`).
pub const VERSION: u32 = 1;

pub const MET: &str = "met";
pub const TRUSTS: &str = "trusts";
pub const VISITED: &str = "visited";
pub const KNOWS_LORE: &str = "knows_lore";
pub const DEAD: &str = "dead";
pub const DEFEATED: &str = "defeated";
pub const NEMESIS: &str = "nemesis";
pub const QUEST_OFFERED: &str = "quest_offered";
pub const QUEST_ACCEPTED: &str = "quest_accepted";
pub const QUEST_DONE: &str = "quest_done";
pub const LEVEL: &str = "level";
pub const SLAPPED: &str = "slapped";
pub const TITLE: &str = "title";
pub const MEMBER_OF: &str = "member_of";
pub const LEADER_OF: &str = "leader_of";

const TRUST: Band = Band {
    min: -100,
    max: 100,
};
const LEVELS: Band = Band { min: 1, max: 60 };
const TALLY: Band = Band { min: 0, max: 1000 };

#[must_use]
pub fn vocabulary() -> FactVocabulary {
    let mut vocabulary = FactVocabulary::new(VERSION);
    vocabulary
        .declare(MET, linked(up_flag(), Person, &[Person]))
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
        .declare(
            LEVEL,
            FactRules::solo(Shape::number(LEVELS).moving(Direction::Up)),
        )
        .declare(SLAPPED, linked(up_tally(), Person, &[Person]))
        .declare(TITLE, linked(up_flag(), Person, &[Thing]))
        .declare(MEMBER_OF, linked(Shape::flag(), Person, &[Faction]))
        .declare(LEADER_OF, linked(Shape::flag(), Person, &[Faction]));
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
