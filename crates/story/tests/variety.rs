//! No repeated quests: shapes, main words, genres, and the recent block of the prompt
//! (docs/plans/quest-variety.md 3).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::quest::variety::{Recent, Shape, main_words, recent_quests};
use timeways_story::quest::{
    AnyOrder, Genre, Known, QuestChange, QuestFault, Status, Step, checked_quest, prompt, quest_log,
};

const GIVER: &str = "Keeper Tessa";
const VISIT_TOWER: &str = r#"{"goal": "visit", "place": "Old Tower"}"#;
const MEET_BRAM: &str = r#"{"goal": "meet", "npc": "Farmer Bram"}"#;

fn known(recent: Vec<Recent>) -> Known<'static> {
    Known {
        giver: GIVER,
        zones: vec!["Testvale"],
        subzones: vec!["Old Tower", "Mill Pond"],
        npcs: vec![GIVER, "Farmer Bram", "Miller Oda"],
        foes: vec!["Duskbat"],
        recent,
        ..Known::default()
    }
}

fn answer(title: &str, steps: &str) -> String {
    format!(
        r#"{{"title": "{title}", "genre": "errand", "text": "I need your help.", "steps": [{steps}]}}"#
    )
}

fn visit(place: &str) -> Step {
    Step::Visit {
        place: place.to_string(),
    }
}

fn meet(npc: &str) -> Step {
    Step::Meet {
        npc: npc.to_string(),
    }
}

fn recent(title: &str, steps: &[Step]) -> Recent {
    Recent {
        number: 1,
        title: title.to_string(),
        shape: Shape::of(steps, None),
        genre: Some(Genre::Errand),
    }
}

/// Three newest offers, newest first: a meeting, a visit, and a visit then a meeting.
fn three_recent() -> Vec<Recent> {
    vec![
        recent("Old Debts", &[meet("Miller Oda")]),
        recent("Rats in the Cellar", &[visit("Mill Pond")]),
        recent("A Lost Lantern", &[visit("Mill Pond"), meet("Miller Oda")]),
    ]
}

#[test]
fn an_offer_with_the_shape_of_the_last_quest_is_refused() {
    let text = answer("Fresh Bread", MEET_BRAM);

    assert_eq!(
        checked_quest(&text, &known(three_recent())).unwrap_err(),
        QuestFault::SameShape("meet".to_string())
    );
}

#[test]
fn an_offer_with_the_shape_of_the_quest_before_the_last_is_refused() {
    let text = answer("Fresh Bread", VISIT_TOWER);

    assert_eq!(
        checked_quest(&text, &known(three_recent())).unwrap_err(),
        QuestFault::SameShape("visit".to_string())
    );
}

#[test]
fn an_offer_with_the_shape_of_the_third_newest_quest_passes() {
    let text = answer("Fresh Bread", &format!("{VISIT_TOWER}, {MEET_BRAM}"));

    assert!(checked_quest(&text, &known(three_recent())).is_ok());
}

#[test]
fn the_order_inside_an_any_order_set_does_not_change_the_shape() {
    let steps = [visit("Old Tower"), meet("Farmer Bram")];
    let turned = [meet("Farmer Bram"), visit("Old Tower")];
    let span = Some(AnyOrder { first: 0, last: 1 });

    assert_eq!(Shape::of(&steps, span), Shape::of(&turned, span));
    assert_eq!(
        Shape::of(&steps, span).to_string(),
        "any order (meet, visit)"
    );
}

#[test]
fn the_order_of_ordered_steps_changes_the_shape() {
    let steps = [visit("Old Tower"), meet("Farmer Bram")];
    let turned = [meet("Farmer Bram"), visit("Old Tower")];

    assert_ne!(Shape::of(&steps, None), Shape::of(&turned, None));
    assert_eq!(Shape::of(&steps, None).to_string(), "visit, meet");
}

#[test]
fn a_title_that_shares_a_main_word_with_one_of_the_last_three_titles_is_refused() {
    let steps = format!("{MEET_BRAM}, {VISIT_TOWER}");

    for title in ["More Debts", "The Cellar Door", "Lantern Light"] {
        let result = checked_quest(&answer(title, &steps), &known(three_recent()));
        assert!(
            matches!(result, Err(QuestFault::SameTitleWord(_))),
            "{title}: {result:?}"
        );
    }
}

#[test]
fn a_title_that_shares_a_main_word_only_with_the_fourth_newest_title_passes() {
    let mut recent = three_recent();
    recent.push(Recent {
        title: "The Broken Wheel".to_string(),
        ..recent[0].clone()
    });
    let text = answer("A Wheel of Cheese", &format!("{MEET_BRAM}, {VISIT_TOWER}"));

    assert!(checked_quest(&text, &known(recent)).is_ok());
}

#[test]
fn a_title_that_shares_only_stop_words_passes() {
    let text = answer(
        "The Quest of the Tower",
        &format!("{MEET_BRAM}, {VISIT_TOWER}"),
    );

    assert!(checked_quest(&text, &known(three_recent())).is_ok());
}

#[test]
fn a_main_word_matches_in_any_case() {
    let text = answer("LANTERN", &format!("{MEET_BRAM}, {VISIT_TOWER}"));

    assert_eq!(
        checked_quest(&text, &known(three_recent())).unwrap_err(),
        QuestFault::SameTitleWord("lantern".to_string())
    );
}

#[test]
fn a_word_of_one_letter_is_never_a_main_word() {
    assert_eq!(
        main_words("Farley's Lost Lantern"),
        ["farley", "lost", "lantern"]
    );
    assert!(main_words("A b C").is_empty());
}

#[test]
fn a_declined_offer_counts_as_a_recent_quest() {
    let offer = |number: u64, title: &str| QuestChange::Offered {
        number,
        at: Tick(number),
        giver: GIVER.to_string(),
        title: title.to_string(),
        text: "I need you.".to_string(),
        steps: vec![visit("Old Tower")],
        genre: Some(Genre::Hunt),
        any_order: None,
    };
    let changes = [
        offer(1, "First"),
        offer(2, "Second"),
        QuestChange::Declined {
            number: 2,
            at: Tick(3),
        },
    ];
    let quests = quest_log(&changes);

    let recent = recent_quests(&quests, 3);

    assert_eq!(quests[1].status, Status::Declined);
    let titles: Vec<&str> = recent.iter().map(|recent| recent.title.as_str()).collect();
    assert_eq!(titles, ["Second", "First"]);
    assert_eq!(recent[0].genre, Some(Genre::Hunt));
}

#[test]
fn an_offer_with_no_genre_is_refused() {
    let text = r#"{"title": "Fresh Bread", "text": "I need bread.", "steps": [{"goal": "visit", "place": "Old Tower"}]}"#;

    assert_eq!(
        checked_quest(text, &known(Vec::new())).unwrap_err(),
        QuestFault::NoGenre
    );
}

#[test]
fn an_offer_with_an_unknown_genre_names_it_in_the_fault() {
    let text = answer("Fresh Bread", VISIT_TOWER).replace("errand", "romance");

    let fault = checked_quest(&text, &known(Vec::new())).unwrap_err();

    assert_eq!(fault, QuestFault::UnknownGenre("romance".to_string()));
    assert!(fault.to_string().contains("romance"));
}

#[test]
fn a_text_that_never_speaks_for_the_giver_is_refused() {
    let text = answer("Fresh Bread", VISIT_TOWER).replace("I need your help.", "Visit the tower.");
    let ours =
        answer("Fresh Bread", VISIT_TOWER).replace("I need your help.", "Our tower is cold.");

    assert_eq!(
        checked_quest(&text, &known(Vec::new())).unwrap_err(),
        QuestFault::NotFromGiver
    );
    assert!(checked_quest(&ours, &known(Vec::new())).is_ok());
}

#[test]
fn the_prompt_lists_the_last_three_quests_with_shape_and_genre() {
    let mut recent = three_recent();
    recent[2].genre = None;

    let text = prompt(&known(recent), Some("Testvale"), None);

    let block = "The player's last quests, newest first:\n<<<\n\
                 - \"Old Debts\": meet. An errand.\n\
                 - \"Rats in the Cellar\": visit. An errand.\n\
                 - \"A Lost Lantern\": visit, meet.\n>>>\n\
                 Make this quest different from these";
    assert!(text.contains(block), "{text}");
    assert!(text.contains(r#""genre": "...""#), "{text}");
    assert!(text.contains("what it means for you in Testvale"), "{text}");
}

#[test]
fn the_prompt_leaves_out_the_recent_block_with_no_earlier_quest() {
    let text = prompt(&known(Vec::new()), Some("Testvale"), None);

    assert!(!text.contains("last quests"), "{text}");
}

#[test]
fn an_old_offer_with_no_genre_or_span_still_reads() {
    let line = r#"{"line":"offered","number":1,"at":5,"giver":"Keeper Tessa","title":"Old","text":"Go.","steps":[{"goal":"visit","place":"Old Tower"}]}"#;

    let change: QuestChange = serde_json::from_str(line).unwrap();
    let quest = quest_log(&[change]).remove(0);

    assert_eq!(quest.genre, None);
    assert_eq!(quest.any_order, None);
    assert_eq!(recent_quests(&[quest], 3)[0].shape.to_string(), "visit");
}
