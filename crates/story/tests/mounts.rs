//! The first mount and the first epic mount (GAMEPLAY.md 3.2).

use timeways_story::mounts::{EPIC_MOUNT_SPEED, is_epic, people_of};
use timeways_story::race_class::Race;

#[test]
fn a_mount_of_level_forty_is_no_epic_mount() {
    assert!(!is_epic(Some(160)));
    assert!(!is_epic(Some(EPIC_MOUNT_SPEED - 1)));
    assert!(!is_epic(None));
}

#[test]
fn an_epic_mount_runs_at_twice_the_speed_of_a_runner() {
    assert!(is_epic(Some(EPIC_MOUNT_SPEED)));
    assert!(is_epic(Some(200)));
}

#[test]
fn a_mount_takes_the_people_who_breed_it() {
    assert_eq!(people_of("Gray Ram", Some(Race::Human)), Some("Ironforge"));
    assert_eq!(people_of("Swift Frostsaber", None), Some("Darnassus"));
    assert_eq!(people_of("Brown Kodo", None), Some("Thunder Bluff"));
    assert_eq!(people_of("Emerald Raptor", None), Some("Sen'jin Village"));
    assert_eq!(people_of("Timber Wolf", None), Some("Orgrimmar"));
    assert_eq!(people_of("Red Mechanostrider", None), Some("Gnomeregan"));
    assert_eq!(people_of("Chestnut Mare", None), Some("Stormwind City"));
}

#[test]
fn a_skeletal_horse_is_a_horse_of_the_forsaken() {
    assert_eq!(
        people_of("Red Skeletal Horse", Some(Race::Human)),
        Some("Undercity")
    );
}

#[test]
fn a_class_mount_takes_the_people_of_the_hero() {
    assert_eq!(
        people_of("Felsteed", Some(Race::Forsaken)),
        Some("Undercity")
    );
    assert_eq!(people_of("Felsteed", None), None);
}
