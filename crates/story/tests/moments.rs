use hourglass::Tick;
use timeways_story::character::{Character, Item};
use timeways_story::gear::Quality;
use timeways_story::input::GameQuestKind;
use timeways_story::moments::{Moment, best, moments};
use timeways_story::places::InstanceKind;
use timeways_story::race_class::{Class, Race};

/// The moments of what `act` adds to the world of `character`.
fn moments_of(character: &mut Character, act: impl FnOnce(&mut Character)) -> Vec<Moment> {
    let before = character.world().history().len();
    act(character);
    let added: Vec<_> = character
        .world()
        .history()
        .iter()
        .skip(before)
        .cloned()
        .collect();
    moments(character.world(), character.you(), &added)
}

#[test]
fn a_first_kill_is_a_moment_and_an_echo_is_not() {
    let mut character = Character::new();

    let first = moments_of(&mut character, |c| c.defeat_npc(Tick(1), "Hogger").unwrap());
    let echo = moments_of(&mut character, |c| c.defeat_npc(Tick(2), "Hogger").unwrap());

    assert_eq!(
        first,
        [Moment::FirstKill {
            foe: "Hogger".to_string()
        }]
    );
    assert!(echo.is_empty());
}

#[test]
fn a_second_death_to_the_same_npc_is_a_moment_and_the_first_is_not() {
    let mut character = Character::new();

    let first = moments_of(&mut character, |c| {
        c.die(Tick(1), Some("Murloc Forager")).unwrap();
    });
    let second = moments_of(&mut character, |c| {
        c.die(Tick(2), Some("Murloc Forager")).unwrap();
    });

    let again = Moment::SlainAgain {
        killer: "Murloc Forager".to_string(),
        times: 2,
    };
    assert!(first.is_empty());
    assert_eq!(second, [again]);
}

fn level_up(level: i64, zone: Option<&str>) -> Moment {
    Moment::LevelUp {
        level,
        zone: zone.map(str::to_string),
    }
}

#[test]
fn a_plain_level_up_has_nothing_to_tell_and_is_no_moment() {
    let mut character = Character::new();

    let login = moments_of(&mut character, |c| c.reach_level(Tick(1), 12).unwrap());
    let plain = moments_of(&mut character, |c| c.reach_level(Tick(2), 13).unwrap());

    assert!(login.is_empty());
    assert!(plain.is_empty(), "{plain:?}");
}

#[test]
fn a_milestone_level_is_a_moment_with_its_zone() {
    let mut character = Character::new();
    character.reach_level(Tick(1), 19).unwrap();
    character
        .enter_zone(Tick(2), "Westfall", Some("Moonbrook"))
        .unwrap();

    let milestone = moments_of(&mut character, |c| c.reach_level(Tick(3), 20).unwrap());

    assert_eq!(milestone, [level_up(20, Some("Westfall"))]);
}

#[test]
fn a_milestone_level_with_no_zone_yet_is_still_a_moment() {
    let mut character = Character::new();
    character.reach_level(Tick(1), 29).unwrap();

    let milestone = moments_of(&mut character, |c| c.reach_level(Tick(2), 30).unwrap());

    assert_eq!(milestone, [level_up(30, None)]);
}

#[test]
fn a_new_zone_is_a_moment_and_its_subzone_is_not() {
    let mut character = Character::new();

    let found = moments_of(&mut character, |c| {
        c.enter_zone(Tick(1), "Westfall", Some("Moonbrook"))
            .unwrap();
    });

    assert_eq!(
        found,
        [Moment::NewZone {
            zone: "Westfall".to_string()
        }]
    );
}

#[test]
fn a_zone_seen_before_is_no_moment() {
    let mut character = Character::new();
    character.enter_zone(Tick(1), "Westfall", None).unwrap();
    character.enter_zone(Tick(2), "Duskwood", None).unwrap();

    let back = moments_of(&mut character, |c| {
        c.enter_zone(Tick(3), "Westfall", None).unwrap();
    });

    assert!(back.is_empty());
}

#[test]
fn the_best_moment_is_the_last_of_the_highest_rank() {
    let found = vec![
        Moment::NewZone {
            zone: "Westfall".to_string(),
        },
        level_up(20, None),
        Moment::FirstKill {
            foe: "Hogger".to_string(),
        },
        Moment::FirstKill {
            foe: "Mother Fang".to_string(),
        },
    ];

    assert_eq!(
        best(found),
        Some(Moment::FirstKill {
            foe: "Mother Fang".to_string()
        })
    );
}

#[test]
fn no_moments_have_no_best() {
    assert_eq!(best(Vec::new()), None);
}

#[test]
fn a_slap_is_a_moment_with_its_count() {
    let mut character = Character::new();
    character.slap(Tick(1), "Innkeeper Farley").unwrap();

    let found = moments_of(&mut character, |c| {
        c.slap(Tick(2), "Innkeeper Farley").unwrap();
    });

    assert_eq!(
        found,
        [Moment::Slapped {
            npc: "Innkeeper Farley".to_string(),
            times: 2
        }]
    );
}

#[test]
fn two_level_ups_in_one_batch_speak_of_the_newest_level() {
    let found = vec![level_up(10, None), level_up(20, None)];

    assert_eq!(best(found), Some(level_up(20, None)));
}

#[test]
fn a_level_up_wins_over_a_new_zone_that_comes_after_it() {
    let level = level_up(20, None);
    let zone = Moment::NewZone {
        zone: "Westfall".to_string(),
    };

    let chosen = best(vec![level.clone(), zone]);

    assert_eq!(chosen, Some(level));
}

#[test]
fn a_finished_class_quest_is_the_biggest_moment_and_a_plain_quest_is_none() {
    let mut character = Character::new();

    let plain = moments_of(&mut character, |c| {
        c.finish_game_quest(Tick(1), "Rattling the Rattlecages", GameQuestKind::Normal)
            .unwrap();
    });
    let class = moments_of(&mut character, |c| {
        c.earn_title(Tick(2), "Dance Machine").unwrap();
        c.finish_game_quest(Tick(2), "Rediscovering the Light", GameQuestKind::Class)
            .unwrap();
    });

    assert!(plain.is_empty(), "{plain:?}");
    assert_eq!(
        best(class),
        Some(Moment::ClassQuestDone {
            title: "Rediscovering the Light".to_string()
        })
    );
}

#[test]
fn a_first_dungeon_outranks_its_new_zone_and_a_second_entry_is_no_moment() {
    let mut character = Character::new();

    let first = moments_of(&mut character, |c| {
        c.enter_zone(Tick(1), "The Deadmines", None).unwrap();
        c.mark_instance(Tick(1), "The Deadmines", InstanceKind::Dungeon)
            .unwrap();
    });
    let again = moments_of(&mut character, |c| {
        c.enter_zone(Tick(2), "The Deadmines", None).unwrap();
        c.mark_instance(Tick(2), "The Deadmines", InstanceKind::Dungeon)
            .unwrap();
    });

    assert_eq!(
        best(first),
        Some(Moment::FirstInstance {
            zone: "The Deadmines".to_string(),
            kind: InstanceKind::Dungeon,
        })
    );
    assert!(again.is_empty(), "{again:?}");
}

#[test]
fn the_first_visit_of_a_capital_is_its_own_moment_and_outranks_a_new_zone() {
    let mut character = Character::new();

    let moments = moments_of(&mut character, |c| {
        c.enter_zone(Tick(1), "Undercity", Some("Trade Quarter"))
            .unwrap();
        c.enter_zone(Tick(1), "Tirisfal Glades", None).unwrap();
    });

    assert_eq!(
        best(moments),
        Some(Moment::FirstCapital {
            city: "Undercity".to_string()
        })
    );
}

#[test]
fn a_quest_mark_is_a_moment_below_a_finished_class_quest() {
    let mut character = Character::new();

    let moments = moments_of(&mut character, |c| {
        c.take_quest_mark(Tick(1), "Rediscovering the Light", "Touched by the Light")
            .unwrap();
    });
    let both = moments_of(&mut character, |c| {
        c.take_quest_mark(Tick(2), "Rediscovering the Light", "Blessed")
            .unwrap();
        c.finish_game_quest(Tick(2), "Rediscovering the Light", GameQuestKind::Class)
            .unwrap();
    });

    assert_eq!(
        moments,
        [Moment::QuestMarked {
            mark: "Touched by the Light".to_string(),
            quest: "Rediscovering the Light".to_string(),
        }]
    );
    assert!(matches!(best(both), Some(Moment::ClassQuestDone { .. })));
}

#[test]
fn taking_a_class_quest_is_no_moment() {
    let mut character = Character::new();

    let taken = moments_of(&mut character, |c| {
        c.take_game_quest(Tick(1), "Rediscovering the Light", GameQuestKind::Class)
            .unwrap();
    });

    assert!(taken.is_empty(), "{taken:?}");
}

fn item(name: &str, slot: u8, quality: Quality) -> Item<'_> {
    Item {
        name,
        slot,
        quality,
    }
}

#[test]
fn a_first_mount_is_a_moment_with_its_people_and_a_second_mount_is_not() {
    let mut character = Character::new();
    character
        .describe(Tick(1), Race::Human, Class::Paladin)
        .unwrap();

    let first = moments_of(&mut character, |c| {
        c.ride_mount(Tick(2), "Gray Ram", false).unwrap();
    });
    let second = moments_of(&mut character, |c| {
        c.ride_mount(Tick(3), "Pinto", false).unwrap();
    });

    assert_eq!(
        first,
        [Moment::FirstMount {
            mount: "Gray Ram".to_string(),
            people: Some("Ironforge".to_string()),
        }]
    );
    assert!(second.is_empty(), "{second:?}");
}

#[test]
fn a_first_epic_mount_comes_once_and_outranks_a_first_kill() {
    let mut character = Character::new();
    character.ride_mount(Tick(1), "Pinto", false).unwrap();

    let both = moments_of(&mut character, |c| {
        c.ride_mount(Tick(2), "Swift Palomino", true).unwrap();
        c.defeat_npc(Tick(2), "Hogger").unwrap();
    });
    let again = moments_of(&mut character, |c| {
        c.ride_mount(Tick(3), "Swift White Steed", true).unwrap();
    });

    assert_eq!(
        best(both),
        Some(Moment::FirstEpicMount {
            mount: "Swift Palomino".to_string(),
            people: Some("Stormwind City".to_string()),
        })
    );
    assert!(again.is_empty(), "{again:?}");
}

#[test]
fn an_epic_mount_as_the_first_ride_is_both_firsts_and_tells_the_epic_one() {
    let mut character = Character::new();

    let moments = moments_of(&mut character, |c| {
        c.ride_mount(Tick(1), "Felsteed", true).unwrap();
    });

    assert_eq!(moments.len(), 2);
    assert!(matches!(best(moments), Some(Moment::FirstEpicMount { .. })));
}

#[test]
fn a_first_epic_item_is_a_moment_with_its_zone_once() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Stranglethorn Vale", None)
        .unwrap();

    let first = moments_of(&mut character, |c| {
        c.put_on(Tick(2), &item("Barman Shanker", 16, Quality::Epic), false)
            .unwrap();
    });
    let second = moments_of(&mut character, |c| {
        c.put_on(Tick(3), &item("Destiny", 16, Quality::Epic), false)
            .unwrap();
    });

    assert_eq!(
        first,
        [Moment::FirstEpicItem {
            item: "Barman Shanker".to_string(),
            zone: Some("Stranglethorn Vale".to_string()),
        }]
    );
    assert!(second.is_empty(), "{second:?}");
}

#[test]
fn a_big_upgrade_counts_once_for_each_slot_and_quality() {
    let mut character = Character::new();

    let first = moments_of(&mut character, |c| {
        c.put_on(Tick(1), &item("Cruel Barb", 16, Quality::Rare), true)
            .unwrap();
    });
    let same_slot = moments_of(&mut character, |c| {
        c.put_on(Tick(2), &item("Thief's Blade", 16, Quality::Rare), true)
            .unwrap();
    });
    let other_slot = moments_of(&mut character, |c| {
        c.put_on(
            Tick(3),
            &item("Robe of the Moccasin", 5, Quality::Rare),
            true,
        )
        .unwrap();
    });
    let no_upgrade = moments_of(&mut character, |c| {
        c.put_on(
            Tick(4),
            &item("Smite's Mighty Hammer", 15, Quality::Rare),
            false,
        )
        .unwrap();
    });

    assert!(
        matches!(first.as_slice(), [Moment::BigUpgrade { .. }]),
        "{first:?}"
    );
    assert!(same_slot.is_empty(), "{same_slot:?}");
    assert!(matches!(other_slot.as_slice(), [Moment::BigUpgrade { .. }]));
    assert!(no_upgrade.is_empty());
}

#[test]
fn an_epic_upgrade_tells_the_first_epic_item() {
    let mut character = Character::new();

    let moments = moments_of(&mut character, |c| {
        c.put_on(Tick(1), &item("Barman Shanker", 16, Quality::Epic), true)
            .unwrap();
    });

    assert_eq!(moments.len(), 2);
    assert!(matches!(best(moments), Some(Moment::FirstEpicItem { .. })));
}

#[test]
fn the_lore_of_an_item_is_about_the_item_then_its_zone() {
    let moment = Moment::BigUpgrade {
        item: "Cruel Barb".to_string(),
        zone: Some("Westfall".to_string()),
    };

    assert_eq!(moment.subjects(), ["Cruel Barb", "Westfall"]);
    assert_eq!(moment.outside_names(), ["Cruel Barb"]);
    assert!(!moment.is_arrival());
}
