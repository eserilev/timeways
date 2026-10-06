//! The format of a scenario file (`crates/dev/src/scenario.rs`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::json;
use timeways_dev::scenario::{Answer, Scenario, ScenarioError, built_in};

#[test]
fn a_blank_line_ends_a_batch_and_a_comment_is_skipped() {
    let text = "# a comment\n{\"type\":\"level_reached\",\"at\":1,\"level\":2}\n\n\
                {\"type\":\"level_reached\",\"at\":2,\"level\":3}\n# another\n";

    let scenario = Scenario::parse(text).unwrap();

    assert_eq!(scenario.batches.len(), 2);
    assert_eq!(scenario.batches[1].lines[0]["level"], json!(3));
}

#[test]
fn a_model_answer_goes_with_its_batch_as_text_or_as_json() {
    let text = "{\"model\":\"quest\",\"answer\":{\"title\":\"T\"}}\n\
                {\"model\":\"talk\",\"answer\":\"SILENCE\"}\n\
                {\"type\":\"quest_asked\",\"at\":1,\"npc\":\"Ada\"}";

    let scenario = Scenario::parse(text).unwrap();

    assert_eq!(
        scenario.batches[0].answers,
        [
            Answer {
                kind: "quest".to_string(),
                text: r#"{"title":"T"}"#.to_string()
            },
            Answer {
                kind: "talk".to_string(),
                text: "SILENCE".to_string()
            },
        ]
    );
}

#[test]
fn the_times_move_to_start_at_the_start() {
    let scenario = Scenario::parse("{\"type\":\"level_reached\",\"at\":60,\"level\":2}").unwrap();

    let moved = scenario.starting_at(1_000);

    assert_eq!(moved.batches[0].lines[0]["at"], json!(1_060));
    assert_eq!(scenario.span(), 60);
}

#[test]
fn a_line_that_breaks_the_format_names_its_line() {
    let cases = [
        ("\n\nnot json", ScenarioError::NotAnObject { line: 3 }),
        ("[1]", ScenarioError::NotAnObject { line: 1 }),
        ("{\"at\":1}", ScenarioError::NoType { line: 1 }),
        (
            "{\"type\":\"level_reached\",\"at\":-1}",
            ScenarioError::BadTime { line: 1 },
        ),
        (
            "{\"type\":\"level_reached\",\"at\":\"1\"}",
            ScenarioError::BadTime { line: 1 },
        ),
        (
            "{\"type\":\"character_entered\",\"realm\":\"R\",\"name\":\"N\"}",
            ScenarioError::CharacterLine { line: 1 },
        ),
        (
            "{\"model\":\"quest\"}",
            ScenarioError::BadAnswer { line: 1 },
        ),
        (
            "{\"model\":1,\"answer\":\"x\"}",
            ScenarioError::BadAnswer { line: 1 },
        ),
    ];

    for (text, error) in cases {
        assert_eq!(Scenario::parse(text), Err(error), "{text}");
    }
}

#[test]
fn an_unknown_scenario_name_has_no_built_in_text() {
    assert!(built_in("fresh").is_some());
    assert!(built_in("level-99").is_none());
}
