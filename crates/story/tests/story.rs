#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::lore::Answer;
use timeways_story::pack::{Link, Pack, Passage};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story, StoryError};

/// The one output of an input, or none.
fn one(outputs: Vec<Output>) -> Option<Output> {
    assert!(outputs.len() <= 1, "{outputs:?}");
    outputs.into_iter().next()
}

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
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    let character = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    assert_eq!(one(story.handle(character).unwrap()), None);
    story
}

fn enter(story: &mut Story, at: u64, zone: &str, subzone: Option<&str>) {
    let input = Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: subzone.map(str::to_string),
    };
    assert_eq!(one(story.handle(input).unwrap()), None);
}

fn meet(story: &mut Story, at: u64, name: &str) {
    let input = Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
    };
    assert_eq!(one(story.handle(input).unwrap()), None);
}

fn ask(story: &mut Story, question: &str, target: Option<&str>) -> Output {
    let input = Input::LoreAsked {
        id: MessageId(7),
        question: question.to_string(),
        target: target.map(str::to_string),
    };
    one(story.handle(input).unwrap()).unwrap()
}

fn model_call(output: Output) -> (CallId, String) {
    match output {
        Output::ModelCall { call, prompt } => (call, prompt),
        other => panic!("expected a model call, got {other:?}"),
    }
}

fn answer(output: Option<Output>) -> Answer {
    match output {
        Some(Output::LoreAnswer { answer, .. }) => answer,
        other => panic!("expected a lore answer, got {other:?}"),
    }
}

/// The sources that pass the spoiler limit, as a player with no model sees them.
fn sources(story: &mut Story, question: &str, target: Option<&str>) -> Vec<String> {
    let answer = match ask(story, question, target) {
        Output::LoreAnswer { answer, .. } => answer,
        Output::ModelCall { call, .. } => {
            answer(one(story.handle(Input::ModelFailed { call }).unwrap()))
        }
        other => panic!("a question gets no {other:?}"),
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

    let output = one(story
        .handle(Input::LevelReached {
            at: Tick(1),
            level: 3,
        })
        .unwrap());

    assert_eq!(output, None);
}

#[test]
fn a_refused_game_event_is_an_error() {
    let mut story = story_with("refused", &[]);
    one(story
        .handle(Input::LevelReached {
            at: Tick(1),
            level: 6,
        })
        .unwrap());

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

    let answer = Answer {
        text: None,
        passages: Vec::new(),
    };
    assert_eq!(
        output,
        Output::LoreAnswer {
            id: MessageId(7),
            answer
        }
    );
}

#[test]
fn the_model_answer_shows_with_the_passages() {
    let mut story = story_with("answered", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    let text = "Goblins burned it [1].".to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

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
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap()).unwrap();

    let (retry, prompt) = model_call(output);
    assert_eq!(retry, CallId(2));
    assert!(prompt.contains("The answer cites no passage."), "{prompt}");
}

#[test]
fn a_failed_model_call_shows_the_passages_alone() {
    let mut story = story_with("failed", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    let output = one(story.handle(Input::ModelFailed { call }).unwrap());

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
    one(story.handle(Input::ModelFailed { call }).unwrap());

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

#[test]
fn a_hello_gets_the_protocol() {
    let mut story = story_with("hello", &[]);

    let output = one(story.handle(Input::Hello).unwrap());

    assert_eq!(output, Some(Output::Hello { protocol: 1 }));
}

#[test]
fn the_answer_carries_the_id_of_its_question_through_a_retry() {
    let mut story = story_with("id-through-retry", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));
    let text = "Goblins burned it.".to_string();
    let (retry, _) =
        model_call(one(story.handle(Input::ModelAnswered { call, text }).unwrap()).unwrap());

    let text = "Goblins burned it [1].".to_string();
    let output = one(story
        .handle(Input::ModelAnswered { call: retry, text })
        .unwrap());

    assert!(
        matches!(
            output,
            Some(Output::LoreAnswer {
                id: MessageId(7),
                ..
            })
        ),
        "{output:?}"
    );
}

#[test]
fn a_journal_request_gets_the_first_page_with_its_id() {
    let mut story = story_with("journal", &[]);
    enter(&mut story, 1, "Testvale", None);

    let output = one(story
        .handle(Input::JournalAsked {
            id: MessageId(4),
            page: 0,
        })
        .unwrap());

    let Some(Output::Journal { id, page }) = output else {
        panic!("expected a journal, got {output:?}");
    };
    assert_eq!(id, MessageId(4));
    assert_eq!((page.page, page.pages), (0, 1));
    assert_eq!(page.journal.places.len(), 1);
}

fn journal_page(story: &mut Story, page: usize) -> timeways_story::journal::Page {
    match one(story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page,
        })
        .unwrap())
    {
        Some(Output::Journal { page, .. }) => page,
        other => panic!("expected a journal, got {other:?}"),
    }
}

fn explorer(story: &mut Story, places: u64) {
    for n in 0..places {
        let zone = format!("The very long and winding zone name number {n}");
        enter(story, n + 1, &zone, None);
    }
}

#[test]
fn later_pages_come_from_the_snapshot_of_the_first() {
    let mut story = story_with("snapshot", &[]);
    explorer(&mut story, 600);
    let first = journal_page(&mut story, 0);
    assert!(first.pages > 1);

    enter(&mut story, 10_000, "A zone after the snapshot", None);
    let last = journal_page(&mut story, first.pages - 1);

    assert_eq!(last.pages, first.pages);
    assert!(
        last.journal
            .places
            .iter()
            .all(|place| place.name != "A zone after the snapshot")
    );
}

#[test]
fn a_page_past_the_end_is_empty_and_names_the_true_count() {
    let mut story = story_with("past-the-end", &[]);
    enter(&mut story, 1, "Testvale", None);

    let page = journal_page(&mut story, 5);

    assert_eq!((page.page, page.pages), (5, 1));
    assert!(page.journal.places.is_empty());
}

#[test]
fn a_game_event_before_any_character_is_refused() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("story-no-character.sqlite");
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);

    let result = story.handle(Input::LevelReached {
        at: Tick(1),
        level: 3,
    });

    assert!(matches!(result, Err(StoryError::NoCharacter)));
}

#[test]
fn a_character_with_an_empty_name_is_refused() {
    let mut story = story_with("empty-name", &[]);

    let result = story.handle(Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: String::new(),
    });

    assert!(matches!(result, Err(StoryError::Store(_))));
}

#[test]
fn a_lore_answer_with_long_passages_still_fits_in_one_reply() {
    let long = "The tower fell in a long and winding story. ".repeat(200);
    let passages: Vec<Passage> = (0..8)
        .map(|n| {
            passage(
                &long,
                &format!("https://example.test/{n}"),
                vec![place("Testvale")],
            )
        })
        .collect();
    let mut story = story_with("long-passages", &passages);
    enter(&mut story, 1, "Testvale", None);
    let (call, _) = model_call(ask(&mut story, "tower", None));

    let text = format!("{} [1]", "\u{1}".repeat(990));
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap()).unwrap();

    let line = serde_json::to_string(&output).unwrap();
    assert!(
        line.len() <= timeways_story::journal::PAGE_BYTES,
        "{} bytes",
        line.len()
    );
    assert!(matches!(output, Output::LoreAnswer { ref answer, .. } if answer.text.is_some()));
}

fn batch_end(story: &mut Story, id: u64) -> Output {
    one(story.handle(Input::BatchEnd { id: MessageId(id) }).unwrap()).unwrap()
}

fn level(story: &mut Story, at: u64, level: u8) {
    assert_eq!(
        one(story
            .handle(Input::LevelReached {
                at: Tick(at),
                level
            })
            .unwrap()),
        None
    );
}

#[test]
fn a_batch_with_no_big_moment_is_seen_at_once_with_no_line() {
    let mut story = story_with("quiet-batch", &[]);
    level(&mut story, 1, 12);

    let output = batch_end(&mut story, 3);

    assert_eq!(
        output,
        Output::EventsSeen {
            id: MessageId(3),
            companion: None
        }
    );
}

#[test]
fn a_big_moment_asks_the_model_for_a_companion_line() {
    let mut story = story_with("big-moment", &[]);
    level(&mut story, 1, 12);
    level(&mut story, 2, 13);

    let (call, prompt) = model_call(batch_end(&mut story, 3));
    let text = "Level 13! Your boots still squeak, though.".to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert!(
        prompt.ends_with("Moment: The player reached level 13."),
        "{prompt}"
    );
    let companion = Some("Level 13! Your boots still squeak, though.".to_string());
    assert_eq!(
        output,
        Some(Output::EventsSeen {
            id: MessageId(3),
            companion
        })
    );
}

#[test]
fn a_failed_or_bad_companion_line_is_silence() {
    let mut story = story_with("silent", &[]);
    level(&mut story, 1, 12);
    level(&mut story, 2, 13);
    let (failed, _) = model_call(batch_end(&mut story, 3));
    level(&mut story, 3, 14);
    let (bad, _) = model_call(batch_end(&mut story, 4));

    let after_failure = one(story.handle(Input::ModelFailed { call: failed }).unwrap());
    let text = "See you in Shattrath!".to_string();
    let after_bad_line = one(story
        .handle(Input::ModelAnswered { call: bad, text })
        .unwrap());

    assert_eq!(
        after_failure,
        Some(Output::EventsSeen {
            id: MessageId(3),
            companion: None
        })
    );
    assert_eq!(
        after_bad_line,
        Some(Output::EventsSeen {
            id: MessageId(4),
            companion: None
        })
    );
}

#[test]
fn a_spent_budget_asks_no_model() {
    let mut story = story_with("budget", &[]);
    level(&mut story, 1, 10);
    for (batch, level_now) in [(1, 11), (2, 12), (3, 13)] {
        level(&mut story, u64::from(level_now), level_now);
        let _ = model_call(batch_end(&mut story, batch));
    }
    level(&mut story, 20, 14);

    let output = batch_end(&mut story, 4);

    assert_eq!(
        output,
        Output::EventsSeen {
            id: MessageId(4),
            companion: None
        }
    );
}

#[test]
fn each_batch_starts_with_no_moments() {
    let mut story = story_with("fresh-batch", &[]);
    level(&mut story, 1, 12);
    level(&mut story, 2, 13);
    let _ = model_call(batch_end(&mut story, 3));

    let output = batch_end(&mut story, 4);

    assert_eq!(
        output,
        Output::EventsSeen {
            id: MessageId(4),
            companion: None
        }
    );
}

const HOUR: u64 = 3600;

/// Two sessions of play: the first one is a finished chapter. Meeting an NPC is no big
/// moment, so the companion stays out of these tests.
fn two_sessions(story: &mut Story) {
    meet(story, HOUR, "Gryan Stoutmantle");
    meet(story, 5 * HOUR, "Salma Saldean");
}

fn chapters(story: &mut Story) -> Vec<timeways_story::journal::Chapter> {
    match one(story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap())
    {
        Some(Output::Journal { page, .. }) => page.journal.chapters,
        other => panic!("expected a journal, got {other:?}"),
    }
}

#[test]
fn a_finished_chapter_asks_the_bard_after_the_batch() {
    let mut story = story_with("bard-asks", &[]);
    two_sessions(&mut story);

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();

    assert_eq!(
        outputs[0],
        Output::EventsSeen {
            id: MessageId(3),
            companion: None
        }
    );
    let [_, Output::ModelCall { prompt, .. }] = outputs.as_slice() else {
        panic!("expected a bard call, got {outputs:?}");
    };
    assert!(
        prompt.contains("Chapter 1. Facts:\n- Met: Gryan Stoutmantle."),
        "{prompt}"
    );
}

#[test]
fn the_saga_of_the_bard_goes_into_its_chapter() {
    let mut story = story_with("bard-writes", &[]);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a bard call, got {outputs:?}");
    };

    let text = "Our hero rode into the golden fields of Westfall.".to_string();
    assert!(
        story
            .handle(Input::ModelAnswered { call, text })
            .unwrap()
            .is_empty()
    );

    let chapters = chapters(&mut story);
    assert_eq!(
        chapters[0].prose.as_deref(),
        Some("Our hero rode into the golden fields of Westfall.")
    );
    assert_eq!(chapters[1].prose, None);
}

#[test]
fn the_bard_is_asked_once_for_each_chapter() {
    let mut story = story_with("bard-once", &[]);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a bard call, got {outputs:?}");
    };
    let while_open = story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();

    assert!(
        story
            .handle(Input::ModelFailed { call })
            .unwrap()
            .is_empty()
    );
    let after_failure = story.handle(Input::BatchEnd { id: MessageId(5) }).unwrap();

    assert_eq!(while_open.len(), 1);
    assert_eq!(after_failure.len(), 1);
    assert_eq!(chapters(&mut story)[0].prose, None);
}

#[test]
fn the_last_chapter_waits_for_the_next_session() {
    let mut story = story_with("bard-waits", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();

    assert_eq!(
        outputs,
        [Output::EventsSeen {
            id: MessageId(3),
            companion: None
        }]
    );
}

#[test]
fn a_saga_for_another_character_is_dropped() {
    let mut story = story_with("bard-switch", &[]);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a bard call, got {outputs:?}");
    };
    let bren = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Bren".to_string(),
    };
    story.handle(bren).unwrap();
    enter(&mut story, HOUR, "Durotar", None);
    enter(&mut story, 5 * HOUR, "The Barrens", None);

    let text = "Our hero rode into Westfall.".to_string();
    story.handle(Input::ModelAnswered { call, text }).unwrap();

    assert_eq!(chapters(&mut story)[0].prose, None);
}
