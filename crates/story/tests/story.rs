#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::character::Character;
use timeways_story::input::{CallId, Input};
use timeways_story::lore::Answer;
use timeways_story::pack::{Link, Pack, Passage};
use timeways_story::story::{Output, Story, StoryError};

fn passage(text: &str, source: &str, links: Vec<Link>) -> Passage {
    Passage {
        text: text.to_string(),
        source: source.to_string(),
        links,
    }
}

fn place(name: &str) -> Link {
    Link::Place(name.to_string())
}

fn npc(name: &str) -> Link {
    Link::Npc(name.to_string())
}

fn tower() -> Passage {
    passage(
        "The tower of Testvale fell.",
        "https://example.test/1",
        vec![place("Testvale")],
    )
}

fn story_with(name: &str, passages: &[Passage]) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("story-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, passages).unwrap();
    Story::new(Character::new(), Pack::open(&path).unwrap())
}

fn enter(story: &mut Story, at: u64, zone: &str, subzone: Option<&str>) {
    let input = Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: subzone.map(str::to_string),
    };
    assert_eq!(story.handle(input).unwrap(), None);
}

fn meet(story: &mut Story, at: u64, name: &str) {
    let input = Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
    };
    assert_eq!(story.handle(input).unwrap(), None);
}

fn ask(story: &mut Story, question: &str, target: Option<&str>) -> Output {
    let input = Input::LoreAsked {
        at: Tick(100),
        question: question.to_string(),
        target: target.map(str::to_string),
    };
    story.handle(input).unwrap().unwrap()
}

fn model_call(output: Output) -> (CallId, String) {
    match output {
        Output::ModelCall { call, prompt } => (call, prompt),
        Output::LoreAnswer(answer) => panic!("expected a model call, got {answer:?}"),
    }
}

fn answer(output: Option<Output>) -> Answer {
    match output {
        Some(Output::LoreAnswer(answer)) => answer,
        other => panic!("expected a lore answer, got {other:?}"),
    }
}

/// The sources that pass the spoiler limit, as a player with no model sees them.
fn sources(story: &mut Story, question: &str, target: Option<&str>) -> Vec<String> {
    let answer = match ask(story, question, target) {
        Output::LoreAnswer(answer) => answer,
        Output::ModelCall { call, .. } => {
            answer(story.handle(Input::ModelFailed { call }).unwrap())
        }
    };
    answer
        .passages
        .into_iter()
        .map(|passage| passage.source)
        .collect()
}

#[test]
fn a_game_event_gets_no_output() {
    let mut story = story_with("no-output", &[]);

    let output = story
        .handle(Input::LevelReached {
            at: Tick(1),
            level: 3,
        })
        .unwrap();

    assert_eq!(output, None);
}

#[test]
fn a_refused_game_event_is_an_error() {
    let mut story = story_with("refused", &[]);
    story
        .handle(Input::LevelReached {
            at: Tick(1),
            level: 6,
        })
        .unwrap();

    let result = story.handle(Input::LevelReached {
        at: Tick(1),
        level: 5,
    });

    assert!(matches!(result, Err(StoryError::Refused(_))));
}

#[test]
fn a_question_with_passages_asks_the_model() {
    let mut story = story_with("asks", &[tower()]);
    enter(&mut story, 1, "Testvale", None);

    let (call, prompt) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    assert_eq!(call, CallId(1));
    assert!(
        prompt.contains("[1] The tower of Testvale fell."),
        "{prompt}"
    );
}

#[test]
fn a_question_with_no_passages_asks_no_model() {
    let mut story = story_with("no-passages", &[tower()]);

    let output = ask(&mut story, "why is this tower in ruins?", None);

    assert_eq!(
        output,
        Output::LoreAnswer(Answer {
            text: None,
            passages: Vec::new()
        })
    );
}

#[test]
fn the_model_answer_shows_with_the_passages() {
    let mut story = story_with("answered", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    let text = "Goblins burned it [1].".to_string();
    let output = story.handle(Input::ModelAnswered { call, text }).unwrap();

    let expected = Answer {
        text: Some("Goblins burned it [1].".to_string()),
        passages: vec![tower()],
    };
    assert_eq!(answer(output), expected);
}

#[test]
fn a_bad_model_answer_asks_again_under_a_new_call() {
    let mut story = story_with("asks-again", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    let text = "Goblins burned it.".to_string();
    let output = story
        .handle(Input::ModelAnswered { call, text })
        .unwrap()
        .unwrap();

    let (retry, prompt) = model_call(output);
    assert_eq!(retry, CallId(2));
    assert!(prompt.contains("The answer cites no passage."), "{prompt}");
}

#[test]
fn a_failed_model_call_shows_the_passages_alone() {
    let mut story = story_with("failed", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    let output = story.handle(Input::ModelFailed { call }).unwrap();

    assert_eq!(
        answer(output),
        Answer {
            text: None,
            passages: vec![tower()]
        }
    );
}

#[test]
fn an_answer_to_an_unknown_call_is_an_error() {
    let mut story = story_with("unknown-call", &[]);

    let result = story.handle(Input::ModelFailed { call: CallId(9) });

    assert!(matches!(result, Err(StoryError::UnknownCall(CallId(9)))));
}

#[test]
fn an_answer_closes_its_call() {
    let mut story = story_with("closes", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));
    story.handle(Input::ModelFailed { call }).unwrap();

    let result = story.handle(Input::ModelFailed { call });

    assert!(matches!(result, Err(StoryError::UnknownCall(_))));
}

#[test]
fn a_passage_about_a_visited_place_is_shown() {
    let mut story = story_with("visited", &[tower()]);
    enter(&mut story, 1, "Testvale", None);

    let found = sources(&mut story, "why is this tower in ruins?", None);

    assert_eq!(found, ["https://example.test/1"]);
}

#[test]
fn a_passage_about_a_place_you_never_visited_is_hidden() {
    let far = passage(
        "The tower of Farvale fell.",
        "https://example.test/1",
        vec![place("Farvale")],
    );
    let mut story = story_with("unvisited", &[far]);
    enter(&mut story, 1, "Testvale", None);

    let found = sources(&mut story, "why is this tower in ruins?", None);

    assert!(found.is_empty());
}

#[test]
fn a_passage_about_an_npc_you_never_met_is_hidden() {
    let secret = passage(
        "Keeper Stubbs hides a secret in Testvale.",
        "https://example.test/1",
        vec![place("Testvale"), npc("Keeper Stubbs")],
    );
    let mut story = story_with("unmet", &[secret]);
    enter(&mut story, 1, "Testvale", None);

    let before = sources(&mut story, "what about Keeper Stubbs?", None);
    meet(&mut story, 2, "Keeper Stubbs");
    let after = sources(&mut story, "what about Keeper Stubbs?", None);

    assert!(before.is_empty());
    assert_eq!(after, ["https://example.test/1"]);
}

#[test]
fn a_passage_needs_every_one_of_its_links() {
    let hideout = passage(
        "A tunnel under Testvale leads to the hideout in Farvale.",
        "https://example.test/1",
        vec![place("Testvale"), place("Farvale")],
    );
    let mut story = story_with("every-link", &[hideout]);
    enter(&mut story, 1, "Testvale", None);

    let found = sources(&mut story, "where does the tunnel go?", None);

    assert!(found.is_empty());
}

#[test]
fn where_you_stand_finds_passages_that_the_question_does_not_name() {
    let inn = passage(
        "Mockshire has an old inn.",
        "https://example.test/1",
        vec![place("Mockshire")],
    );
    let mut story = story_with("context", &[inn]);
    enter(&mut story, 1, "Testvale", Some("Mockshire"));

    let found = sources(&mut story, "what is this place?", None);

    assert_eq!(found, ["https://example.test/1"]);
}

#[test]
fn the_target_finds_passages_that_the_question_does_not_name() {
    let stubbs = passage(
        "Keeper Stubbs guards the gate.",
        "https://example.test/1",
        vec![npc("Keeper Stubbs")],
    );
    let mut story = story_with("target", &[stubbs]);
    meet(&mut story, 1, "Keeper Stubbs");

    let found = sources(&mut story, "who is this?", Some("Keeper Stubbs"));

    assert_eq!(found, ["https://example.test/1"]);
}

#[test]
fn an_answer_holds_at_most_eight_passages() {
    let passages: Vec<Passage> = (0..12)
        .map(|n| {
            passage(
                "A tower.",
                &format!("https://example.test/{n}"),
                vec![place("Testvale")],
            )
        })
        .collect();
    let mut story = story_with("answer-size", &passages);
    enter(&mut story, 1, "Testvale", None);

    let found = sources(&mut story, "tower", None);

    assert_eq!(found.len(), 8);
}
