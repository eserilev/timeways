#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::quest::{
    Known, MAX_OFFER_BYTES, Quest, QuestChange, QuestFault, Status, Step, checked_quest,
    offer_line, prompt, quest_log, thing_name, title_of_thing,
};
use timeways_story::seen::{SeenText, TextKind};

const GIVER: &str = "Keeper Tessa";

fn known(seen: &[SeenText]) -> Known<'_> {
    Known {
        giver: GIVER,
        zones: vec!["Testvale"],
        subzones: vec!["Old Tower", "Mill Pond"],
        npcs: vec![GIVER, "Farmer Bram", "Miller Oda"],
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
        r#"{"goal": "kill", "npc": "Farmer Bram"}"#,
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

    assert!(text.contains("You are Keeper Tessa"), "{text}");
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
