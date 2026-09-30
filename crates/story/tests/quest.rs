#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::quest::{
    Known, MAX_KILLS, MAX_OFFER_BYTES, MAX_TITLE_CHARS, Quest, QuestChange, QuestFault, Status,
    Step, checked_quest, offer_line, prompt, quest_log, thing_name, title_of_thing,
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
fn a_quest_has_one_to_three_steps() {
    let seen = [];
    let none = answer("The Lost Lantern", "");
    let four = answer("The Lost Lantern", &[VISIT_TOWER; 4].join(", "));

    assert_eq!(
        checked_quest(&none, &known(&seen)).unwrap_err(),
        QuestFault::StepCount(0)
    );
    assert_eq!(
        checked_quest(&four, &known(&seen)).unwrap_err(),
        QuestFault::StepCount(4)
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
    };

    let line = offer_line(GIVER, &quest);

    assert_eq!(
        line,
        "Keeper Tessa has a task for you: The Lost Lantern. Bring word to the tower. Type /quest accept."
    );
    assert!(line.len() <= MAX_OFFER_BYTES);
}

#[test]
fn the_prompt_lists_the_names_that_a_quest_can_use() {
    let seen = [];

    let text = prompt(&known(&seen), Some("Testvale"));

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

    assert_eq!(quests[0].steps_done, 0);
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

    let text = prompt(&known, Some("Testvale"));

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
fn a_kill_counts_only_for_the_next_step_and_only_up_to_its_count() {
    let mut changes = bat_hunt(Status::Accepted);
    changes.push(killed(1));
    changes.push(QuestChange::StepDone {
        number: 1,
        step: 0,
        at: Tick(3),
    });
    changes.extend([killed(1), killed(1), killed(1)]);

    let quest = &quest_log(&changes)[0];

    assert_eq!(quest.kills, 2);
    assert!(quest.next_step_holds(&[], None));
}

#[test]
fn a_kill_for_an_offer_counts_nothing() {
    let mut changes = bat_hunt(Status::Offered);
    changes.push(killed(0));

    assert_eq!(quest_log(&changes)[0].kills, 0);
}

#[test]
fn only_the_next_kill_step_hunts() {
    let changes = bat_hunt(Status::Accepted);

    let quest = &quest_log(&changes)[0];

    assert!(!quest.hunts("Duskbat"));
}
