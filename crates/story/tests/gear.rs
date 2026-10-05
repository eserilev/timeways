//! The first epic item and a big upgrade (GAMEPLAY.md 3.2).

use timeways_story::gear::{BIG_UPGRADE_LEVELS, Before, Quality, is_big_upgrade};

const OLD: u16 = 30;

fn over_old(gain: u16) -> u16 {
    OLD + gain
}

fn worn() -> Before {
    Before::Worn { level: Some(OLD) }
}

#[test]
fn a_rare_item_ten_levels_over_the_old_one_is_a_big_upgrade() {
    assert!(is_big_upgrade(
        Quality::Rare,
        Some(over_old(BIG_UPGRADE_LEVELS)),
        worn()
    ));
    assert!(is_big_upgrade(
        Quality::Epic,
        Some(over_old(BIG_UPGRADE_LEVELS + 1)),
        worn()
    ));
}

#[test]
fn a_gain_of_nine_levels_is_no_big_upgrade() {
    assert!(!is_big_upgrade(
        Quality::Rare,
        Some(over_old(BIG_UPGRADE_LEVELS - 1)),
        worn()
    ));
    assert!(!is_big_upgrade(Quality::Epic, Some(over_old(0)), worn()));
}

#[test]
fn an_uncommon_item_is_never_a_big_upgrade() {
    assert!(!is_big_upgrade(
        Quality::Uncommon,
        Some(over_old(40)),
        worn()
    ));
    assert!(!is_big_upgrade(Quality::Uncommon, Some(60), Before::Empty));
}

#[test]
fn an_empty_slot_counts_as_level_zero() {
    assert!(is_big_upgrade(
        Quality::Rare,
        Some(BIG_UPGRADE_LEVELS),
        Before::Empty
    ));
    assert!(!is_big_upgrade(
        Quality::Rare,
        Some(BIG_UPGRADE_LEVELS - 1),
        Before::Empty
    ));
}

#[test]
fn an_unknown_level_on_either_side_is_no_big_upgrade() {
    assert!(!is_big_upgrade(Quality::Epic, None, worn()));
    assert!(!is_big_upgrade(
        Quality::Epic,
        Some(60),
        Before::Worn { level: None }
    ));
}

#[test]
fn a_quality_past_legendary_does_not_exist_in_classic() {
    assert_eq!(Quality::of_number(3), Some(Quality::Rare));
    assert_eq!(Quality::of_number(4), Some(Quality::Epic));
    assert_eq!(Quality::of_number(6), None);
    assert!(Quality::Legendary.is_epic());
    assert!(!Quality::Rare.is_epic());
}
