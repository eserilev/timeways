#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::hero_hook::Hook;
use timeways_story::quest::{
    AnyOrder, DAY_SECONDS, Encounter, Here, Known, MAX_CARRY, MAX_KILLS, MAX_OFFER_BYTES,
    MAX_TITLE_CHARS, MAX_TOPIC_CHARS, MAX_WAIT_DAYS, Quest, QuestChange, QuestFault, Status, Step,
    checked_quest, goods_for, offer_line, prompt, quest_log, thing_name, title_of_thing,
};
use timeways_story::seen::{SeenText, TextKind};

const GIVER: &str = "Keeper Tessa";

fn known(seen: &[SeenText]) -> Known<'_> {
    Known {
        giver: GIVER,
        zones: vec!["Testvale"],
        subzones: vec!["Old Tower", "Mill Pond"],
        npcs: vec![GIVER, "Farmer Bram", "Miller Oda"],
        foes: vec!["Duskbat", "Mill Rat"],
        last_targets: Vec::new(),
        seen,
        goods: vec!["Linen Cloth", "Light Leather"],
    }
}

fn game_quest(title: &str, text: &str) -> SeenText {
    SeenText {
        kind: TextKind::Quest,
        title: Some(title.to_string()),
        npc: Some("Farmer Bram".to_string()),
        zone: Some("Testvale".to_string()),
        text: text.to_string(),
    }
}

fn answer(title: &str, steps: &str) -> String {
    format!(r#"{{"title": "{title}", "text": "Bring word to the tower.", "steps": [{steps}]}}"#)
}

const VISIT_TOWER: &str = r#"{"goal": "visit", "place": "Old Tower"}"#;
const MEET_BRAM: &str = r#"{"goal": "meet", "npc": "Farmer Bram"}"#;
const VISIT_POND: &str = r#"{"goal": "visit", "place": "Mill Pond"}"#;

#[test]
fn a_quest_with_known_places_and_people_passes() {
    let seen = [];
    let text = answer("The Lost Lantern", &format!("{VISIT_TOWER}, {MEET_BRAM}"));

    let quest = checked_quest(&text, &known(&seen)).unwrap();

    assert_eq!(quest.title, "The Lost Lantern");
    assert_eq!(
        quest.steps,
        [
            Step::Visit {
                place: "Old Tower".to_string()
            },
            Step::Meet {
                npc: "Farmer Bram".to_string()
            },
        ]
    );
}

#[test]
fn a_quest_inside_a_code_fence_passes() {
    let seen = [];
    let text = format!("```json\n{}\n```", answer("The Lost Lantern", VISIT_TOWER));

    assert!(checked_quest(&text, &known(&seen)).is_ok());
}

#[test]
fn an_answer_with_no_json_is_refused() {
    let seen = [];

    let fault = checked_quest("I have no task for you.", &known(&seen)).unwrap_err();

    assert_eq!(fault, QuestFault::NotJson);
}

#[test]
fn a_step_with_an_unknown_goal_is_refused() {
    let seen = [];
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "collect", "item": "Lantern"}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::NotJson
    );
}

#[test]
fn a_quest_has_one_to_four_steps() {
    let seen = [];
    let none = answer("The Lost Lantern", "");
    let four = [VISIT_TOWER, MEET_BRAM, KILL_BATS, VISIT_POND].join(", ");
    let five = format!(r#"{four}, {{"goal": "meet", "npc": "Miller Oda"}}"#);

    assert_eq!(
        checked_quest(&none, &known(&seen)).unwrap_err(),
        QuestFault::StepCount(0)
    );
    assert!(checked_quest(&answer("The Lost Lantern", &four), &known(&seen)).is_ok());
    assert_eq!(
        checked_quest(&answer("The Lost Lantern", &five), &known(&seen)).unwrap_err(),
        QuestFault::StepCount(5)
    );
}

#[test]
fn a_quest_never_sends_you_to_a_place_that_you_never_heard_of() {
    let seen = [];
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "visit", "place": "Far Harbor"}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownPlace("Far Harbor".to_string())
    );
}

#[test]
fn a_place_name_must_match_the_game_exactly() {
    let seen = [];
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "visit", "place": "old tower"}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownPlace("old tower".to_string())
    );
}

#[test]
fn a_quest_never_sends_you_to_an_npc_that_you_never_met() {
    let seen = [];
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "meet", "npc": "Captain Vorn"}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownNpc("Captain Vorn".to_string())
    );
}

#[test]
fn the_dead_of_your_story_are_not_in_the_known_npcs_so_no_quest_sends_you_to_them() {
    let seen = [];
    let mut world = known(&seen);
    world.npcs.retain(|npc| *npc != "Farmer Bram");
    let text = answer("The Lost Lantern", MEET_BRAM);

    assert_eq!(
        checked_quest(&text, &world).unwrap_err(),
        QuestFault::UnknownNpc("Farmer Bram".to_string())
    );
}

#[test]
fn a_quest_never_asks_you_to_kill_its_giver() {
    let seen = [];
    let mut known = known(&seen);
    known.foes.push(GIVER);
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "kill", "creature": "Keeper Tessa", "count": 1}"#,
    );

    assert_eq!(
        checked_quest(&text, &known).unwrap_err(),
        QuestFault::KillGiver
    );
    assert!(!known.prey().contains(&GIVER));
}

#[test]
fn a_quest_never_sends_you_back_to_its_giver() {
    let seen = [];
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "meet", "npc": "Keeper Tessa"}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::MeetGiver
    );
}

#[test]
fn a_quest_never_sends_you_to_the_npc_of_a_game_quest_that_you_read() {
    let seen = [game_quest(
        "Wheat for the Mill",
        "Speak with farmer bram about his wheat.",
    )];
    let text = answer("The Lost Lantern", MEET_BRAM);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::GameQuest("Farmer Bram".to_string())
    );
}

#[test]
fn a_quest_never_sends_you_to_the_subzone_of_a_game_quest_that_you_read() {
    let seen = [game_quest(
        "Rats in the Tower",
        "Clear the rats from the Old Tower.",
    )];
    let text = answer("The Lost Lantern", VISIT_TOWER);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::GameQuest("Old Tower".to_string())
    );
}

#[test]
fn a_zone_that_a_game_quest_names_is_still_a_goal() {
    let seen = [game_quest(
        "Rats in the Tower",
        "Rats fill all of Testvale.",
    )];
    let text = answer(
        "The Lost Lantern",
        r#"{"goal": "visit", "place": "Testvale"}"#,
    );

    assert!(checked_quest(&text, &known(&seen)).is_ok());
}

#[test]
fn gossip_and_books_do_not_count_as_game_quests() {
    let mut gossip = game_quest("", "Farmer Bram sells good wheat.");
    gossip.kind = TextKind::Gossip;
    let seen = [gossip];
    let text = answer("The Lost Lantern", MEET_BRAM);

    assert!(checked_quest(&text, &known(&seen)).is_ok());
}

#[test]
fn a_quest_never_takes_the_title_of_a_game_quest() {
    let seen = [game_quest("Rats in the Tower", "Clear the rats.")];
    let text = answer("Rats in the Tower", MEET_BRAM);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::GameQuest("Rats in the Tower".to_string())
    );
}

#[test]
fn a_title_with_a_name_after_the_cutoff_is_refused() {
    let seen = [];
    let text = answer("The Road to Pandaria", MEET_BRAM);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::BadTitle
    );
}

#[test]
fn a_text_that_is_too_long_is_refused() {
    let seen = [];
    let long = "word ".repeat(100);
    let text =
        format!(r#"{{"title": "The Lost Lantern", "text": "{long}", "steps": [{MEET_BRAM}]}}"#);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::BadText
    );
}

#[test]
fn an_offer_longer_than_one_narrator_line_is_refused() {
    let seen = [];
    let text = format!(
        r#"{{"title": "{}", "text": "{}", "steps": [{MEET_BRAM}]}}"#,
        "€".repeat(60),
        "€".repeat(280),
    );

    let result = checked_quest(&text, &known(&seen));

    assert_eq!(result.unwrap_err(), QuestFault::TooLong);
}

#[test]
fn the_offer_line_names_the_giver_the_title_and_the_command() {
    let quest = Quest {
        title: "The Lost Lantern".to_string(),
        text: "Bring word to the tower.".to_string(),
        steps: Vec::new(),
        any_order: None,
    };

    let line = offer_line(GIVER, &quest);

    assert_eq!(
        line,
        "Keeper Tessa has a quest for you: The Lost Lantern. Bring word to the tower. Type /quest accept."
    );
    assert!(line.len() <= MAX_OFFER_BYTES);
}

#[test]
fn the_prompt_lists_the_names_that_a_quest_can_use() {
    let seen = [];

    let text = prompt(&known(&seen), Some("Testvale"), None);

    assert!(text.contains("Name: Keeper Tessa"), "{text}");
    for name in ["- Testvale", "- Old Tower", "- Mill Pond", "- Farmer Bram"] {
        assert!(text.contains(name), "{name}: {text}");
    }
}

#[test]
fn a_quest_thing_gives_back_its_title() {
    let name = thing_name(12, "The Lost Lantern: Part 2");

    assert_eq!(name, "quest 12: The Lost Lantern: Part 2");
    assert_eq!(title_of_thing(&name), Some("The Lost Lantern: Part 2"));
}

#[test]
fn a_title_that_looks_like_a_quest_is_no_quest() {
    assert_eq!(title_of_thing("quest of doom: the end"), None);
    assert_eq!(title_of_thing("Scourge of Squirrels"), None);
}

#[test]
fn a_quest_with_the_same_step_twice_is_refused() {
    let seen = [];
    let text = answer("The Lost Lantern", &format!("{MEET_BRAM}, {MEET_BRAM}"));

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::RepeatedStep
    );
}

#[test]
fn a_step_done_out_of_order_changes_nothing() {
    let offered = QuestChange::Offered {
        number: 1,
        at: Tick(1),
        giver: GIVER.to_string(),
        title: "The Lost Lantern".to_string(),
        text: "Go.".to_string(),
        steps: vec![
            Step::Visit {
                place: "Mill Pond".to_string(),
            },
            Step::Meet {
                npc: "Farmer Bram".to_string(),
            },
        ],
        any_order: None,
    };
    let accepted = QuestChange::Accepted {
        number: 1,
        at: Tick(2),
    };
    let second_first = QuestChange::StepDone {
        number: 1,
        step: 1,
        at: Tick(3),
    };

    let quests = quest_log(&[offered, accepted, second_first]);

    assert_eq!(quests[0].steps_done(), 0);
    assert_eq!(quests[0].status, Status::Accepted);
}

#[test]
fn an_offer_of_exactly_one_narrator_line_passes() {
    let seen = [];
    let frame = offer_line(
        GIVER,
        &Quest {
            title: String::new(),
            text: String::new(),
            steps: Vec::new(),
            any_order: None,
        },
    )
    .len();
    // A text has at most 400 characters, so it reaches the byte limit with 3-byte ones.
    let title = "T".repeat(MAX_TITLE_CHARS);
    let rest = MAX_OFFER_BYTES - frame - MAX_TITLE_CHARS;
    let text = "€".repeat(rest / 3) + &"w".repeat(rest % 3);
    let answer = format!(r#"{{"title": "{title}", "text": "{text}", "steps": [{MEET_BRAM}]}}"#);

    let offer = checked_quest(&answer, &known(&seen));

    let line = offer_line(GIVER, &offer.unwrap());
    assert_eq!(line.len(), MAX_OFFER_BYTES);
}

#[test]
fn a_title_that_differs_from_a_game_quest_only_in_case_or_marks_is_refused() {
    let seen = [game_quest("The Lost Lantern", "Find it.")];
    let text = answer("the lost lantern!", MEET_BRAM);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::GameQuest("the lost lantern!".to_string())
    );
}

#[test]
fn an_accept_by_number_takes_that_offer_when_another_one_waits_first() {
    let offer = |number: u64, giver: &str| QuestChange::Offered {
        number,
        at: Tick(1),
        giver: giver.to_string(),
        title: format!("Task {number}"),
        text: "Go.".to_string(),
        steps: vec![Step::Meet {
            npc: "Farmer Bram".to_string(),
        }],
        any_order: None,
    };
    let accepted = QuestChange::Accepted {
        number: 2,
        at: Tick(2),
    };

    let quests = quest_log(&[offer(1, GIVER), offer(2, "Innkeeper Pell"), accepted]);

    assert_eq!(quests[0].status, Status::Offered);
    assert_eq!(quests[1].status, Status::Accepted);
}

const KILL_BATS: &str = r#"{"goal": "kill", "creature": "Duskbat", "count": 6}"#;

#[test]
fn a_kill_step_names_a_hostile_creature_that_you_saw() {
    let seen = [];
    let text = answer("Bats in the Belfry", KILL_BATS);

    let quest = checked_quest(&text, &known(&seen)).unwrap();

    assert_eq!(
        quest.steps,
        [Step::Kill {
            creature: "Duskbat".to_string(),
            count: 6
        }]
    );
}

#[test]
fn a_kill_step_never_names_a_creature_that_you_never_saw_hostile() {
    let seen = [];
    let text = answer(
        "Bats in the Belfry",
        r#"{"goal": "kill", "creature": "Farmer Bram", "count": 1}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownFoe("Farmer Bram".to_string())
    );
}

#[test]
fn a_meet_step_never_names_a_foe() {
    let seen = [];
    let text = answer(
        "Bats in the Belfry",
        r#"{"goal": "meet", "npc": "Duskbat"}"#,
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownNpc("Duskbat".to_string())
    );
}

#[test]
fn a_kill_step_asks_for_one_to_ten_kills() {
    let seen = [];
    let kill = |count: u8| {
        answer(
            "Bats in the Belfry",
            &format!(r#"{{"goal": "kill", "creature": "Duskbat", "count": {count}}}"#),
        )
    };

    assert_eq!(
        checked_quest(&kill(0), &known(&seen)).unwrap_err(),
        QuestFault::KillCount(0)
    );
    assert!(checked_quest(&kill(1), &known(&seen)).is_ok());
    assert!(checked_quest(&kill(MAX_KILLS), &known(&seen)).is_ok());
    assert_eq!(
        checked_quest(&kill(MAX_KILLS + 1), &known(&seen)).unwrap_err(),
        QuestFault::KillCount(MAX_KILLS + 1)
    );
}

#[test]
fn two_steps_never_name_the_same_target() {
    let seen = [];
    let twice = r#"{"goal": "kill", "creature": "Duskbat", "count": 2}"#;
    let text = answer("Bats in the Belfry", &format!("{KILL_BATS}, {twice}"));

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::RepeatedStep
    );
}

#[test]
fn a_creature_of_a_game_quest_that_you_read_is_no_kill_target() {
    let seen = [game_quest("Pests", "Kill 8 Duskbat in the barn.")];
    let text = answer("Bats in the Belfry", KILL_BATS);

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::GameQuest("Duskbat".to_string())
    );
}

#[test]
fn a_task_never_names_a_target_of_the_last_task() {
    let seen = [];
    let mut known = known(&seen);
    known.last_targets = vec!["Old Tower", "Duskbat"];

    let tower = checked_quest(&answer("The Lost Lantern", VISIT_TOWER), &known);
    let bats = checked_quest(&answer("Bats in the Belfry", KILL_BATS), &known);

    assert_eq!(
        tower.unwrap_err(),
        QuestFault::LastTask("Old Tower".to_string())
    );
    assert_eq!(
        bats.unwrap_err(),
        QuestFault::LastTask("Duskbat".to_string())
    );
    assert!(checked_quest(&answer("The Lost Lantern", MEET_BRAM), &known).is_ok());
}

#[test]
fn the_prompt_lists_only_the_targets_that_the_check_allows() {
    let seen = [game_quest("Rats", "Miller Oda needs help at Mill Pond.")];
    let mut known = known(&seen);
    known.last_targets = vec!["Mill Rat"];

    let text = prompt(&known, Some("Testvale"), None);

    for name in ["- Testvale", "- Old Tower", "- Farmer Bram", "- Duskbat"] {
        assert!(text.contains(name), "{name}: {text}");
    }
    for name in [
        "- Keeper Tessa",
        "- Miller Oda",
        "- Mill Pond",
        "- Mill Rat",
    ] {
        assert!(!text.contains(name), "{name}: {text}");
    }
    assert!(text.contains(r#""goal": "kill""#), "{text}");
}

fn bat_hunt(status: Status) -> Vec<QuestChange> {
    let mut changes = vec![QuestChange::Offered {
        number: 1,
        at: Tick(1),
        giver: GIVER.to_string(),
        title: "Bats in the Belfry".to_string(),
        text: "Clear the belfry.".to_string(),
        steps: vec![
            Step::Visit {
                place: "Old Tower".to_string(),
            },
            Step::Kill {
                creature: "Duskbat".to_string(),
                count: 2,
            },
        ],
        any_order: None,
    }];
    if status == Status::Accepted {
        changes.push(QuestChange::Accepted {
            number: 1,
            at: Tick(2),
        });
    }
    changes
}

fn killed(step: usize) -> QuestChange {
    QuestChange::Killed {
        number: 1,
        step,
        at: Tick(3),
    }
}

#[test]
fn a_kill_counts_only_for_an_open_step_and_only_up_to_its_count() {
    let mut changes = bat_hunt(Status::Accepted);
    changes.push(killed(1));
    changes.push(QuestChange::StepDone {
        number: 1,
        step: 0,
        at: Tick(3),
    });
    changes.extend([killed(1), killed(1), killed(1)]);

    let quest = &quest_log(&changes)[0];

    assert_eq!(quest.kills[1], 2);
    assert!(quest.step_holds(1, &Here::default(), &Encounter::None));
}

#[test]
fn a_kill_for_an_offer_counts_nothing() {
    let mut changes = bat_hunt(Status::Offered);
    changes.push(killed(0));

    assert_eq!(quest_log(&changes)[0].kills, [0, 0]);
}

#[test]
fn only_an_open_kill_step_hunts() {
    let changes = bat_hunt(Status::Accepted);

    let quest = &quest_log(&changes)[0];

    assert_eq!(quest.hunts("Duskbat"), None);
}

#[test]
fn a_kill_count_written_as_text_counts_and_other_text_is_refused() {
    let known = known(&[]);
    let as_text = r#"{"title": "Wolves", "text": "Thin them out.", "steps": [{"goal": "kill", "creature": "Duskbat", "count": "3"}]}"#;
    let not_a_number = r#"{"title": "Wolves", "text": "Thin them out.", "steps": [{"goal": "kill", "creature": "Duskbat", "count": "many"}]}"#;

    let offer = checked_quest(as_text, &known).unwrap();

    assert_eq!(
        offer.steps,
        [Step::Kill {
            creature: "Duskbat".to_string(),
            count: 3
        }]
    );
    assert!(checked_quest(not_a_number, &known).is_err());
}

#[test]
fn a_step_opens_when_the_step_before_it_is_done() {
    let mut changes = bat_hunt(Status::Accepted);
    let before = quest_log(&changes).remove(0);
    changes.push(QuestChange::StepDone {
        number: 1,
        step: 0,
        at: Tick(5),
    });

    let after = quest_log(&changes).remove(0);

    assert_eq!(before.open_steps(), [0]);
    assert_eq!(before.opened_at(0), Some(Tick(2)));
    assert_eq!(before.opened_at(1), None);
    assert_eq!(after.open_steps(), [1]);
    assert_eq!(after.opened_at(1), Some(Tick(5)));
}

fn talk_to(npc: &str, about: Option<&str>) -> String {
    match about {
        Some(about) => format!(r#"{{"goal": "talk", "npc": "{npc}", "about": "{about}"}}"#),
        None => format!(r#"{{"goal": "talk", "npc": "{npc}"}}"#),
    }
}

#[test]
fn a_talk_step_names_an_npc_that_you_can_meet() {
    let seen = [];
    let text = answer(
        "A Word with Bram",
        &talk_to("Farmer Bram", Some("the missing cask")),
    );

    let quest = checked_quest(&text, &known(&seen)).unwrap();

    assert_eq!(
        quest.steps,
        [Step::Talk {
            npc: "Farmer Bram".to_string(),
            about: Some("the missing cask".to_string()),
        }]
    );
}

#[test]
fn a_talk_step_never_names_a_foe_or_an_animal() {
    let seen = [];
    let text = answer("A Word with a Bat", &talk_to("Duskbat", None));

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownNpc("Duskbat".to_string())
    );
}

#[test]
fn a_talk_topic_over_sixty_characters_is_refused() {
    let seen = [];
    let sixty = "a".repeat(MAX_TOPIC_CHARS);
    let long = answer(
        "A Long Word",
        &talk_to("Farmer Bram", Some(&format!("{sixty}a"))),
    );
    let fits = answer("A Long Word", &talk_to("Farmer Bram", Some(&sixty)));

    assert_eq!(
        checked_quest(&long, &known(&seen)).unwrap_err(),
        QuestFault::BadTopic
    );
    assert!(checked_quest(&fits, &known(&seen)).is_ok());
}

#[test]
fn a_talk_topic_with_a_name_after_the_cutoff_is_refused() {
    let seen = [];
    let text = answer(
        "A Word with Bram",
        &talk_to("Farmer Bram", Some("the road to Pandaria")),
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::BadTopic
    );
}

#[test]
fn a_talk_step_never_sends_you_back_to_the_giver() {
    let seen = [];
    let text = answer("A Word with Me", &talk_to(GIVER, None));

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::MeetGiver
    );
}

#[test]
fn the_prompt_shows_a_goal_only_when_its_list_has_a_name() {
    let seen = [];
    let mut known = known(&seen);
    known.foes = Vec::new();
    known.npcs = vec![GIVER];

    let text = prompt(&known, Some("Testvale"), None);

    assert!(text.contains(r#""goal": "visit""#), "{text}");
    for goal in ["meet", "talk", "kill"] {
        let shown = format!(r#""goal": "{goal}""#);
        assert!(!text.contains(&shown), "{goal}: {text}");
    }
}

fn wait(days: u8) -> String {
    format!(r#"{{"goal": "wait", "days": {days}}}"#)
}

#[test]
fn a_wait_lasts_one_to_three_days() {
    let seen = [];
    let steps = |days| format!("{VISIT_TOWER}, {}, {MEET_BRAM}", wait(days));

    for days in [1, MAX_WAIT_DAYS] {
        assert!(checked_quest(&answer("Later", &steps(days)), &known(&seen)).is_ok());
    }
    for days in [0, MAX_WAIT_DAYS + 1] {
        assert_eq!(
            checked_quest(&answer("Later", &steps(days)), &known(&seen)).unwrap_err(),
            QuestFault::WaitDays(days)
        );
    }
}

#[test]
fn a_wait_is_never_the_first_or_the_last_step() {
    let seen = [];
    let first = answer("Later", &format!("{}, {MEET_BRAM}", wait(1)));
    let last = answer("Later", &format!("{MEET_BRAM}, {}", wait(1)));

    for text in [first, last] {
        assert_eq!(
            checked_quest(&text, &known(&seen)).unwrap_err(),
            QuestFault::WaitPlace
        );
    }
}

#[test]
fn a_quest_has_at_most_one_wait() {
    let seen = [];
    // Neither wait is first or last, so only the count breaks a rule.
    let text = answer(
        "Later",
        &format!("{VISIT_TOWER}, {}, {}, {MEET_BRAM}", wait(1), wait(2)),
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::WaitPlace
    );
}

#[test]
fn a_step_after_a_wait_can_name_the_giver() {
    let seen = [];
    let back = talk_to(GIVER, Some("what you found"));
    let text = answer(
        "Come Back Later",
        &format!("{VISIT_TOWER}, {}, {back}", wait(2)),
    );

    let quest = checked_quest(&text, &known(&seen)).unwrap();

    assert_eq!(quest.steps[2].person(), Some(GIVER));
}

#[test]
fn a_step_with_no_wait_before_it_never_names_the_giver() {
    let seen = [];
    let meet_giver = format!(r#"{{"goal": "meet", "npc": "{GIVER}"}}"#);
    let text = answer(
        "Come Back Later",
        &format!("{meet_giver}, {}, {VISIT_TOWER}", wait(1)),
    );

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::MeetGiver
    );
}

#[test]
fn the_prompt_never_lists_the_giver_as_a_person_to_meet() {
    let seen = [];

    let text = prompt(&known(&seen), Some("Testvale"), None);

    assert!(!text.contains("- Keeper Tessa"), "{text}");
    assert!(text.contains(r#""goal": "wait""#), "{text}");
}

/// A quest of three steps: visit the tower, wait 2 days, meet Bram. Accepted at 100.
fn wait_quest() -> Vec<QuestChange> {
    vec![
        QuestChange::Offered {
            number: 1,
            at: Tick(1),
            giver: GIVER.to_string(),
            title: "Come Back Later".to_string(),
            text: "Go.".to_string(),
            steps: vec![
                Step::Visit {
                    place: "Old Tower".to_string(),
                },
                Step::Wait { days: 2 },
                Step::Meet {
                    npc: "Farmer Bram".to_string(),
                },
            ],
            any_order: None,
        },
        QuestChange::Accepted {
            number: 1,
            at: Tick(100),
        },
    ]
}

fn done(step: usize, at: u64) -> QuestChange {
    QuestChange::StepDone {
        number: 1,
        step,
        at: Tick(at),
    }
}

#[test]
fn a_wait_opens_when_the_step_before_it_is_done() {
    let mut changes = wait_quest();
    let before = quest_log(&changes).remove(0);
    changes.push(done(0, 500));

    let after = quest_log(&changes).remove(0);

    assert_eq!(before.ready_at(1), None);
    assert_eq!(after.opened_at(1), Some(Tick(500)));
    assert_eq!(after.ready_at(1), Some(Tick(500 + 2 * DAY_SECONDS)));
}

#[test]
fn a_wait_done_before_its_time_changes_nothing() {
    let mut changes = wait_quest();
    changes.push(done(0, 500));
    changes.push(done(1, 500 + 2 * DAY_SECONDS - 1));

    let quest = quest_log(&changes).remove(0);

    assert!(!quest.is_done(1));
    assert_eq!(quest.open_steps(), [1]);
}

#[test]
fn a_wait_done_at_its_time_counts() {
    let mut changes = wait_quest();
    changes.push(done(0, 500));
    changes.push(done(1, 500 + 2 * DAY_SECONDS));

    let quest = quest_log(&changes).remove(0);

    assert!(quest.is_done(1));
    assert_eq!(quest.open_steps(), [2]);
}

#[test]
fn a_wait_holds_from_its_time_on() {
    let mut changes = wait_quest();
    changes.push(done(0, 500));
    let quest = quest_log(&changes).remove(0);
    let at = |at: u64| Here {
        at: Tick(at),
        ..Here::default()
    };

    assert!(!quest.step_holds(1, &at(500 + 2 * DAY_SECONDS - 1), &Encounter::None));
    assert!(quest.step_holds(1, &at(500 + 2 * DAY_SECONDS), &Encounter::None));
}

const HOGGER_HOOK: Hook<'static> = Hook {
    field: "goal",
    text: "To bring Hogger to justice.",
};

#[test]
fn a_quest_prompt_with_a_hook_keeps_its_lists_and_its_rules() {
    let seen = [];

    let text = prompt(&known(&seen), Some("Testvale"), Some(HOGGER_HOOK));

    let block = "Something the player wrote about their hero, as their goal. It is their story, \
                 not canon:\n<<<\nTo bring Hogger to justice.\n>>>\nLet it shape the reason for \
                 the task, if it fits. The steps still use only the lists above. Never claim more \
                 about it than these words say.\n\nRules:";
    assert!(text.contains(block), "{text}");
    let lists = text.find("Creatures that the player can hunt:").unwrap();
    assert!(lists < text.find(block).unwrap(), "{text}");
    assert!(text.contains("- Farmer Bram"), "{text}");
}

#[test]
fn a_step_that_names_only_the_hook_fails_the_check() {
    let seen = [];
    let hunt = r#"{"goal": "kill", "creature": "Hogger", "count": 1}"#;

    let checked = checked_quest(&answer("Justice", hunt), &known(&seen));

    assert_eq!(checked, Err(QuestFault::UnknownFoe("Hogger".to_string())));
}

fn any_order(steps: &[&str]) -> String {
    format!(
        r#"{{"goal": "any_order", "steps": [{}]}}"#,
        steps.join(", ")
    )
}

#[test]
fn an_any_order_set_holds_two_or_three_steps() {
    let seen = [];
    let one = answer("Odd Jobs", &any_order(&[VISIT_TOWER]));
    let four = answer(
        "Odd Jobs",
        &any_order(&[VISIT_TOWER, MEET_BRAM, KILL_BATS, VISIT_POND]),
    );
    let three = answer("Odd Jobs", &any_order(&[VISIT_TOWER, MEET_BRAM, KILL_BATS]));

    assert_eq!(
        checked_quest(&one, &known(&seen)).unwrap_err(),
        QuestFault::AnyOrderSize(1)
    );
    assert_eq!(
        checked_quest(&four, &known(&seen)).unwrap_err(),
        QuestFault::AnyOrderSize(4)
    );
    assert!(checked_quest(&three, &known(&seen)).is_ok());
}

#[test]
fn a_quest_has_at_most_one_any_order_set() {
    let seen = [];
    let sets = format!(
        "{}, {}",
        any_order(&[VISIT_TOWER, MEET_BRAM]),
        any_order(&[KILL_BATS, VISIT_POND])
    );

    assert_eq!(
        checked_quest(&answer("Odd Jobs", &sets), &known(&seen)).unwrap_err(),
        QuestFault::AnyOrderTwice
    );
}

#[test]
fn an_any_order_set_holds_no_wait() {
    let seen = [];
    let steps = format!("{VISIT_POND}, {}", any_order(&[VISIT_TOWER, &wait(1)]));

    assert_eq!(
        checked_quest(&answer("Odd Jobs", &steps), &known(&seen)).unwrap_err(),
        QuestFault::AnyOrderWait
    );
}

#[test]
fn a_set_inside_a_set_is_refused() {
    let seen = [];
    let inner = any_order(&[VISIT_TOWER, MEET_BRAM]);
    let steps = any_order(&[&inner, KILL_BATS]);

    assert_eq!(
        checked_quest(&answer("Odd Jobs", &steps), &known(&seen)).unwrap_err(),
        QuestFault::NotJson
    );
}

#[test]
fn an_any_order_set_is_flattened_with_its_span() {
    let seen = [];
    let steps = format!("{VISIT_POND}, {}", any_order(&[VISIT_TOWER, MEET_BRAM]));

    let quest = checked_quest(&answer("Odd Jobs", &steps), &known(&seen)).unwrap();

    assert_eq!(quest.steps.len(), 3);
    assert_eq!(quest.steps[1].target(), Some("Old Tower"));
    assert_eq!(quest.any_order, Some(AnyOrder { first: 1, last: 2 }));
}

/// A quest of a visit, then a set of two kills, then a meeting. Accepted at 2.
fn set_quest() -> Vec<QuestChange> {
    vec![
        QuestChange::Offered {
            number: 1,
            at: Tick(1),
            giver: GIVER.to_string(),
            title: "Odd Jobs".to_string(),
            text: "Go.".to_string(),
            steps: vec![
                Step::Visit {
                    place: "Old Tower".to_string(),
                },
                Step::Kill {
                    creature: "Duskbat".to_string(),
                    count: 2,
                },
                Step::Kill {
                    creature: "Mill Rat".to_string(),
                    count: 2,
                },
                Step::Meet {
                    npc: "Farmer Bram".to_string(),
                },
            ],
            any_order: Some(AnyOrder { first: 1, last: 2 }),
        },
        QuestChange::Accepted {
            number: 1,
            at: Tick(2),
        },
        done(0, 3),
    ]
}

#[test]
fn each_step_of_an_open_set_can_be_done_in_any_order() {
    let mut changes = set_quest();
    let open = quest_log(&changes).remove(0).open_steps();
    changes.push(done(2, 4));

    let quest = quest_log(&changes).remove(0);

    assert_eq!(open, [1, 2]);
    assert!(quest.is_done(2));
    assert_eq!(quest.open_steps(), [1]);
}

#[test]
fn the_step_after_a_set_opens_only_when_the_whole_set_is_done() {
    let mut changes = set_quest();
    changes.push(done(3, 4));
    changes.push(done(1, 5));
    let half = quest_log(&changes).remove(0);
    changes.push(done(2, 6));

    let whole = quest_log(&changes).remove(0);

    assert!(!half.is_done(3));
    assert_eq!(half.open_steps(), [2]);
    assert_eq!(whole.open_steps(), [3]);
    assert_eq!(whole.opened_at(3), Some(Tick(6)));
}

#[test]
fn two_kill_steps_in_a_set_count_their_kills_apart() {
    let mut changes = set_quest();
    changes.extend([killed(1), killed(2), killed(2)]);

    let quest = quest_log(&changes).remove(0);

    assert_eq!(quest.kills, [0, 1, 2, 0]);
    assert_eq!(quest.hunts("Duskbat"), Some(1));
    assert_eq!(quest.hunts("Mill Rat"), Some(2));
}

#[test]
fn a_span_past_the_steps_counts_as_no_span() {
    let mut changes = set_quest();
    if let QuestChange::Offered { any_order, .. } = &mut changes[0] {
        *any_order = Some(AnyOrder { first: 2, last: 9 });
    }

    let quest = quest_log(&changes).remove(0);

    assert_eq!(quest.any_order, None);
    assert_eq!(quest.open_steps(), [1]);
}

#[test]
fn the_prompt_shows_how_to_ask_for_steps_in_any_order() {
    let seen = [];

    let text = prompt(&known(&seen), Some("Testvale"), None);

    assert!(
        text.contains(r#"{"goal": "any_order", "steps": [...]}"#),
        "{text}"
    );
}

fn carry(item: &str, count: u8, npc: &str) -> String {
    format!(r#"{{"goal": "carry", "item": "{item}", "count": {count}, "npc": "{npc}"}}"#)
}

#[test]
fn a_carry_step_names_a_good_of_your_level_band() {
    let goods = goods_for(Some(12));
    let seen = [];
    let mut known = known(&seen);
    known.goods = goods.clone();
    let text = answer("Cloth for Bram", &carry("Linen Cloth", 10, "Farmer Bram"));

    let quest = checked_quest(&text, &known).unwrap();

    assert!(goods.contains(&"Linen Cloth") && goods.contains(&"Light Leather"));
    assert!(!goods.contains(&"Runecloth"));
    assert!(goods_for(None).is_empty());
    assert_eq!(quest.steps[0].person(), Some("Farmer Bram"));
}

#[test]
fn a_good_outside_your_level_band_is_refused() {
    let seen = [];
    let text = answer("Cloth for Bram", &carry("Runecloth", 10, "Farmer Bram"));

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::UnknownGood("Runecloth".to_string())
    );
}

#[test]
fn a_carry_step_asks_for_one_to_twenty_items() {
    let seen = [];
    let steps = |count| {
        answer(
            "Cloth for Bram",
            &carry("Linen Cloth", count, "Farmer Bram"),
        )
    };

    for count in [1, MAX_CARRY] {
        assert!(checked_quest(&steps(count), &known(&seen)).is_ok());
    }
    for count in [0, MAX_CARRY + 1] {
        assert_eq!(
            checked_quest(&steps(count), &known(&seen)).unwrap_err(),
            QuestFault::CarryCount(count)
        );
    }
}

#[test]
fn a_good_in_the_text_of_a_game_quest_is_still_a_goal() {
    let seen = [game_quest("Cloth Drive", "Bring me 10 Linen Cloth.")];
    let text = answer("Cloth for Bram", &carry("Linen Cloth", 10, "Farmer Bram"));

    assert!(checked_quest(&text, &known(&seen)).is_ok());
}

#[test]
fn the_npc_of_a_carry_step_keeps_the_overlap_rule() {
    let seen = [game_quest("Cloth Drive", "Farmer Bram needs cloth.")];
    let text = answer("Cloth for Bram", &carry("Linen Cloth", 10, "Farmer Bram"));

    assert_eq!(
        checked_quest(&text, &known(&seen)).unwrap_err(),
        QuestFault::GameQuest("Farmer Bram".to_string())
    );
}

/// An accepted quest of one step: bring 10 Linen Cloth to Farmer Bram.
fn cloth_quest() -> timeways_story::quest::Tracked {
    let changes = [
        QuestChange::Offered {
            number: 1,
            at: Tick(1),
            giver: GIVER.to_string(),
            title: "Cloth for Bram".to_string(),
            text: "Go.".to_string(),
            steps: vec![Step::Carry {
                item: "Linen Cloth".to_string(),
                count: 10,
                npc: "Farmer Bram".to_string(),
            }],
            any_order: None,
        },
        QuestChange::Accepted {
            number: 1,
            at: Tick(2),
        },
    ];
    quest_log(&changes).remove(0)
}

fn held(npc: &str, item: &str, count: u16) -> Encounter {
    Encounter::Carry {
        npc: npc.to_string(),
        item: item.to_string(),
        count,
    }
}

#[test]
fn a_carry_step_holds_for_a_count_at_its_number() {
    let quest = cloth_quest();

    assert!(quest.step_holds(0, &Here::default(), &held("Farmer Bram", "Linen Cloth", 10)));
    assert!(quest.step_holds(
        0,
        &Here::default(),
        &held("Farmer Bram", "Linen Cloth", 300)
    ));
}

#[test]
fn a_count_below_the_number_leaves_the_carry_step_open() {
    let quest = cloth_quest();

    assert!(!quest.step_holds(0, &Here::default(), &held("Farmer Bram", "Linen Cloth", 9)));
}

#[test]
fn a_count_for_another_npc_or_item_leaves_the_carry_step_open() {
    let quest = cloth_quest();
    let here = Here::default();

    assert!(!quest.step_holds(0, &here, &held("Miller Oda", "Linen Cloth", 10)));
    assert!(!quest.step_holds(0, &here, &held("Farmer Bram", "Wool Cloth", 10)));
    assert!(!quest.step_holds(0, &here, &Encounter::Gossip("Farmer Bram".to_string())));
}
