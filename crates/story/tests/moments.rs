use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::GameQuestKind;
use timeways_story::moments::{Moment, best, moments};
use timeways_story::places::InstanceKind;

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

#[test]
fn a_level_up_is_a_moment_and_the_first_level_is_not() {
    let mut character = Character::new();

    let login = moments_of(&mut character, |c| c.reach_level(Tick(1), 12).unwrap());
    let level_up = moments_of(&mut character, |c| c.reach_level(Tick(2), 13).unwrap());

    assert!(login.is_empty());
    assert_eq!(level_up, [Moment::LevelUp { level: 13 }]);
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
        Moment::LevelUp { level: 13 },
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
    let found = vec![Moment::LevelUp { level: 12 }, Moment::LevelUp { level: 13 }];

    assert_eq!(best(found), Some(Moment::LevelUp { level: 13 }));
}

#[test]
fn a_level_up_wins_over_a_new_zone_that_comes_after_it() {
    let level = Moment::LevelUp { level: 13 };
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
