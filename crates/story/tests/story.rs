#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::hero::MAX_TEXT_CHARS;
use timeways_story::input::{CallId, GameQuestKind, Input, MessageId};
use timeways_story::lore::Answer;
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::places::InstanceKind;
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
        origin: Origin::Pack,
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
        spot: None,
    };
    assert_eq!(one(story.handle(input).unwrap()), None);
}

fn meet(story: &mut Story, at: u64, name: &str) {
    let input = Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
        spot: None,
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
            answer,
            notice: None,
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
        text: Some("Goblins burned it.".to_string()),
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
fn common_knowledge_is_shown_before_you_go_anywhere() {
    let history = passage(
        "The testers of Testvale came from the sea.",
        "https://example.test/1",
        vec![Link::Common],
    );
    let mut story = story_with("common", &[history]);
    enter(&mut story, 1, "Mockshire", None);

    let found = sources(&mut story, "where did the testers come from?", None);

    assert_eq!(found, ["https://example.test/1"]);
}

#[test]
fn common_knowledge_about_a_place_still_waits_for_the_place() {
    let history = passage(
        "The testers of Farvale came from the sea.",
        "https://example.test/1",
        vec![Link::Common, place("Farvale")],
    );
    let mut story = story_with("common-place", &[history]);
    enter(&mut story, 1, "Mockshire", None);

    let found = sources(&mut story, "where did the testers come from?", None);

    assert!(found.is_empty());
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

    let Some(Output::Journal { id, page, .. }) = output else {
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
fn a_page_past_the_end_gets_the_last_page() {
    let mut story = story_with("past-the-end", &[]);
    enter(&mut story, 1, "Testvale", None);

    let page = journal_page(&mut story, 5);

    assert_eq!((page.page, page.pages), (0, 1));
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
        line.len() <= timeways_story::reply_size::MAX_LINE,
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
            narrator: None,
            notice: None,
        }
    );
}

#[test]
fn a_big_moment_asks_the_model_for_a_narrator_line() {
    let mut story = story_with("big-moment", &[]);
    level(&mut story, 1, 12);
    level(&mut story, 2, 13);

    let (call, prompt) = model_call(batch_end(&mut story, 3));
    let text = "Level 13! Your boots still squeak, though.".to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert!(
        prompt.contains("The moment:\n<<<\nThe player reached level 13.\n>>>"),
        "{prompt}"
    );
    let narrator = Some("Level 13! Your boots still squeak, though.".to_string());
    assert_eq!(
        output,
        Some(Output::EventsSeen {
            id: MessageId(3),
            narrator,
            notice: None,
        })
    );
}

#[test]
fn a_turned_in_class_quest_asks_the_narrator_for_a_line() {
    let mut story = story_with("class-quest", &[]);
    let done = Input::GameQuestDone {
        at: Tick(1),
        title: "Rediscovering the Light".to_string(),
        kind: GameQuestKind::Class,
    };
    assert!(story.handle(done).unwrap().is_empty());

    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(
        prompt.contains(
            "The moment:\n<<<\nThe player finished \"Rediscovering the Light\", a quest of their class.\n>>>"
        ),
        "{prompt}"
    );
}

#[test]
fn a_quest_mark_asks_the_narrator_for_a_line() {
    let mut story = story_with("quest-mark", &[]);
    let marked = Input::QuestMarked {
        at: Tick(1),
        quest: "Rediscovering the Light".to_string(),
        mark: "Touched by the Light".to_string(),
    };
    assert!(story.handle(marked).unwrap().is_empty());

    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(
        prompt.contains(
            "During the quest \"Rediscovering the Light\", a lasting effect came on the player: \"Touched by the Light\"."
        ),
        "{prompt}"
    );
}

#[test]
fn a_first_dungeon_asks_the_narrator_for_a_line() {
    let mut story = story_with("first-dungeon", &[]);
    enter(&mut story, 1, "The Deadmines", None);
    let entered = Input::InstanceEntered {
        at: Tick(1),
        zone: "The Deadmines".to_string(),
        kind: InstanceKind::Dungeon,
    };
    assert!(story.handle(entered).unwrap().is_empty());

    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(
        prompt.contains("The player entered the dungeon The Deadmines for the first time."),
        "{prompt}"
    );
}

#[test]
fn a_quest_of_the_game_with_a_bad_title_is_refused() {
    let mut story = story_with("bad-quest-title", &[]);
    let done = Input::GameQuestDone {
        at: Tick(1),
        title: "a\nb".to_string(),
        kind: GameQuestKind::Normal,
    };

    assert!(matches!(story.handle(done), Err(StoryError::BadName)));
}

#[test]
fn a_name_in_no_fact_is_logged_and_the_line_still_shows() {
    let mut story = story_with("unknown-name", &[]);
    level(&mut story, 1, 12);
    level(&mut story, 2, 13);
    let (call, _) = model_call(batch_end(&mut story, 3));

    let text = "Our hero reached level 13 under the eyes of Varian.".to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    let shown = Some("Our hero reached level 13 under the eyes of Varian.".to_string());
    assert!(matches!(output, Some(Output::EventsSeen { narrator, .. }) if narrator == shown));
    assert_eq!(
        story.take_notes(),
        [format!(
            "call {}: the answer names Varian, and no fact does",
            call.0
        )]
    );
    assert!(story.take_notes().is_empty());
}

#[test]
fn a_failed_or_bad_narrator_line_is_silence() {
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
            narrator: None,
            notice: None,
        })
    );
    assert_eq!(
        after_bad_line,
        Some(Output::EventsSeen {
            id: MessageId(4),
            narrator: None,
            notice: None,
        })
    );
}

#[test]
fn a_spent_budget_asks_no_model() {
    let mut story = story_with("budget", &[]);
    level(&mut story, 1, 10);
    for (batch, level_now) in [(1, 11), (2, 12), (3, 13)] {
        level(&mut story, u64::from(level_now), level_now);
        let (call, _) = model_call(batch_end(&mut story, batch));
        story.handle(Input::ModelFailed { call }).unwrap();
    }
    level(&mut story, 20, 14);

    let output = batch_end(&mut story, 4);

    assert_eq!(
        output,
        Output::EventsSeen {
            id: MessageId(4),
            narrator: None,
            notice: None,
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
            narrator: None,
            notice: None,
        }
    );
}

const HOUR: u64 = 3600;

/// 50 minutes of play in one zone from `from`, so that a milestone after it starts a new
/// chapter (GAMEPLAY.md 3.3). The batch stays open, so no saga is asked here.
fn play_fifty_minutes(story: &mut Story, from: u64, zone: &str) {
    enter(story, from, zone, None);
    enter(story, from + 25 * 60, zone, Some("Camp One"));
    enter(story, from + 50 * 60, zone, Some("Camp Two"));
}

/// The first visit of a zone: the milestone that starts the next chapter. The narrator
/// line of the zone gets no answer, and it keeps the saga from this batch.
fn new_chapter(story: &mut Story, at: u64, zone: &str, batch: u64) {
    enter(story, at, zone, None);
    let _ = close_narrator(story, batch);
}

/// Two chapters: the first one is finished. The narrator lines of the zones get no
/// answer, so no call keeps the saga waiting.
fn two_sessions(story: &mut Story) {
    meet(story, HOUR, "Gryan Stoutmantle");
    play_fifty_minutes(story, HOUR, "Westfall");
    new_chapter(story, 5 * HOUR, "Duskwood", 91);
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
fn a_finished_chapter_asks_for_its_saga_after_the_batch() {
    let mut story = story_with("saga-asks", &[]);
    two_sessions(&mut story);

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();

    assert_eq!(
        outputs[0],
        Output::EventsSeen {
            id: MessageId(3),
            narrator: None,
            notice: None,
        }
    );
    let [_, Output::ModelCall { prompt, .. }] = outputs.as_slice() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    assert!(
        prompt.contains(
            "The facts of chapter 1:\n<<<\n- Traveled to: Westfall.\n- Met: Gryan Stoutmantle."
        ),
        "{prompt}"
    );
}

#[test]
fn the_saga_goes_into_its_chapter() {
    let mut story = story_with("saga-writes", &[]);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a saga call, got {outputs:?}");
    };

    let text = r#"{"saga": "Our hero rode into the golden fields of Westfall."}"#.to_string();
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
fn a_saga_is_asked_once_for_each_chapter() {
    let mut story = story_with("saga-once", &[]);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a saga call, got {outputs:?}");
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
fn a_saga_recalls_the_last_chapters_before_it() {
    let mut story = story_with("saga-memory", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");
    play_fifty_minutes(&mut story, HOUR, "Westfall");
    enter(&mut story, 5 * HOUR, "Duskwood", None);
    play_fifty_minutes(&mut story, 5 * HOUR, "Duskwood");
    enter(&mut story, 9 * HOUR, "Redridge Mountains", None);
    meet(&mut story, 9 * HOUR, "Marshal Dughan");
    let _ = close_narrator(&mut story, 90);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, prompt } = outputs[1].clone() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    story.handle(Input::ModelFailed { call }).unwrap();

    let second = saga_prompt(&mut story, 4);

    assert!(!prompt.contains("What came before"), "{prompt}");
    assert!(
        second.contains("<<<\n- Chapter 1: traveled to Westfall; met Gryan Stoutmantle.\n>>>"),
        "{second}"
    );
}

/// Three chapters, the last one still open. The narrator lines get no answer.
fn three_chapters(story: &mut Story) {
    meet(story, HOUR, "Gryan Stoutmantle");
    play_fifty_minutes(story, HOUR, "Westfall");
    enter(story, 5 * HOUR, "Duskwood", None);
    play_fifty_minutes(story, 5 * HOUR, "Duskwood");
    enter(story, 9 * HOUR, "Redridge Mountains", None);
    let _ = close_narrator(story, 90);
}

/// The prompt of the saga call of the next batch. The call fails, so the next chapter
/// gets its turn.
fn failed_saga_prompt(story: &mut Story, batch: u64) -> String {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    let [_, Output::ModelCall { call, prompt }] = outputs.as_slice() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    story.handle(Input::ModelFailed { call: *call }).unwrap();
    prompt.clone()
}

/// Answers the saga call of the next batch with `saga`.
fn write_saga(story: &mut Story, batch: u64, saga: &str) {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    let [_, Output::ModelCall { call, .. }] = outputs.as_slice() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    let text = serde_json::json!({ "saga": saga }).to_string();
    story
        .handle(Input::ModelAnswered { call: *call, text })
        .unwrap();
}

#[test]
fn the_hero_sheet_goes_only_into_the_first_chapter() {
    let mut story = story_with("sheet-once", &[]);
    set_field(&mut story, "origin", "Born in a Brill cellar.").unwrap();
    three_chapters(&mut story);

    let first = failed_saga_prompt(&mut story, 3);
    let second = saga_prompt(&mut story, 4);

    assert!(first.contains("Born in a Brill cellar."), "{first}");
    assert!(!second.contains("Born in a Brill cellar."), "{second}");
}

#[test]
fn a_changed_hero_sheet_goes_into_the_chapter_where_it_changed() {
    let mut story = story_with("sheet-changed", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");
    play_fifty_minutes(&mut story, HOUR, "Westfall");
    enter(&mut story, 5 * HOUR, "Duskwood", None);
    set_field(&mut story, "origin", "Born in a Brill cellar.").unwrap();
    play_fifty_minutes(&mut story, 5 * HOUR, "Duskwood");
    enter(&mut story, 9 * HOUR, "Redridge Mountains", None);
    let _ = close_narrator(&mut story, 90);
    let _ = close_narrator(&mut story, 91);

    let second = saga_prompt(&mut story, 3);

    assert!(second.contains("Write chapter 2"), "{second}");
    assert!(second.contains("Born in a Brill cellar."), "{second}");
}

#[test]
fn a_saga_that_repeats_an_earlier_saga_is_refused() {
    let mut story = story_with("saga-repeat", &[]);
    three_chapters(&mut story);
    let saga = "Our hero walked the long road west and met a farmer by the old mill.";

    write_saga(&mut story, 3, saga);
    write_saga(&mut story, 4, saga);

    let chapters = chapters(&mut story);
    assert_eq!(chapters[0].prose.as_deref(), Some(saga));
    assert_eq!(chapters[1].prose, None);
}

#[test]
fn the_last_chapter_waits_for_the_next_session() {
    let mut story = story_with("saga-waits", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();

    assert_eq!(
        outputs,
        [Output::EventsSeen {
            id: MessageId(3),
            narrator: None,
            notice: None,
        }]
    );
}

#[test]
fn a_saga_for_another_character_is_dropped() {
    let mut story = story_with("saga-switch", &[]);
    two_sessions(&mut story);
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let Output::ModelCall { call, .. } = outputs[1].clone() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    let bren = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Bren".to_string(),
    };
    story.handle(bren).unwrap();
    enter(&mut story, HOUR, "Durotar", None);
    enter(&mut story, 5 * HOUR, "The Barrens", None);

    let text = r#"{"saga": "Our hero rode into Westfall."}"#.to_string();
    story.handle(Input::ModelAnswered { call, text }).unwrap();

    assert_eq!(chapters(&mut story)[0].prose, None);
}

#[test]
fn the_saga_waits_while_another_model_call_is_open() {
    let mut story = story_with("saga-waits-for-calls", &[tower()]);
    enter(&mut story, 1, "Testvale", None);
    meet(&mut story, HOUR, "Gryan Stoutmantle");
    play_fifty_minutes(&mut story, HOUR, "Westfall");
    enter(&mut story, 5 * HOUR, "Duskwood", None);
    let (narrator, _) =
        model_call(one(story.handle(Input::BatchEnd { id: MessageId(2) }).unwrap()).unwrap());
    let (question, _) = model_call(ask(&mut story, "why is this tower in ruins?", None));

    let while_open = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    story.handle(Input::ModelFailed { call: question }).unwrap();
    story.handle(Input::ModelFailed { call: narrator }).unwrap();
    let after = story.handle(Input::BatchEnd { id: MessageId(4) }).unwrap();

    assert_eq!(while_open.len(), 1, "{while_open:?}");
    assert!(
        matches!(after.as_slice(), [_, Output::ModelCall { .. }]),
        "{after:?}"
    );
}

#[test]
fn a_name_that_no_game_sends_is_refused() {
    let mut story = story_with("bad-name", &[]);

    let long = story.handle(Input::NpcMet {
        at: Tick(1),
        name: "N".repeat(97),
        spot: None,
    });
    let control = story.handle(Input::NpcMet {
        at: Tick(1),
        name: "A\u{7}B".to_string(),
        spot: None,
    });
    let empty_killer = story.handle(Input::Died {
        at: Tick(1),
        killer: Some(String::new()),
        cause: None,
        killer_level: None,
        hour: None,
    });

    assert!(matches!(long, Err(StoryError::BadName)));
    assert!(matches!(control, Err(StoryError::BadName)));
    assert!(matches!(empty_killer, Err(StoryError::BadName)));
}

#[test]
fn moments_of_one_character_never_reach_another() {
    let mut story = story_with("moments-switch", &[]);
    level(&mut story, 1, 12);
    level(&mut story, 2, 13);
    let bren = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Bren".to_string(),
    };
    story.handle(bren).unwrap();

    let output = batch_end(&mut story, 3);

    assert_eq!(
        output,
        Output::EventsSeen {
            id: MessageId(3),
            narrator: None,
            notice: None,
        }
    );
}

#[test]
fn a_moment_before_a_refusal_still_counts() {
    let mut story = story_with("moment-before-refusal", &[]);
    level(&mut story, 1, 12);
    let zone = Input::ZoneEntered {
        at: Tick(5),
        zone: "Westfall".to_string(),
        subzone: None,
        spot: None,
    };
    assert!(story.handle(zone).is_ok());
    let old_event = Input::LevelReached {
        at: Tick(6),
        level: 11,
    };
    assert!(story.handle(old_event).is_err());

    let (_, prompt) = model_call(batch_end(&mut story, 3));

    assert!(
        prompt
            .contains("The moment:\n<<<\nThe player arrived in Westfall for the first time.\n>>>"),
        "{prompt}"
    );
}

fn talk(story: &mut Story, npc: &str, text: &str) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::TalkAsked {
        id: MessageId(8),
        at: Tick(50),
        npc: npc.to_string(),
        text: text.to_string(),
    })
}

fn people(story: &mut Story) -> Vec<timeways_story::journal::Person> {
    match one(story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap())
    {
        Some(Output::Journal { page, .. }) => page.journal.people,
        other => panic!("expected a journal, got {other:?}"),
    }
}

#[test]
fn talking_meets_the_npc_and_asks_the_model_as_that_npc() {
    let mut story = story_with("talk-asks", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));

    let (_, prompt) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "any news?").unwrap()).unwrap());

    assert!(prompt.starts_with("You are Innkeeper Farley,"), "{prompt}");
    assert!(
        prompt.contains("of the world of Warcraft in Goldshire."),
        "{prompt}"
    );
    assert_eq!(people(&mut story)[0].name, "Innkeeper Farley");
}

#[test]
fn the_answer_of_the_npc_shows_and_its_change_of_trust_lands() {
    let mut story = story_with("talk-answers", &[]);
    let (call, _) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "any news?").unwrap()).unwrap());

    let text = r#"{"say": "Nothing but rain.", "trust": 3}"#.to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    let answer = Output::TalkAnswer {
        id: MessageId(8),
        npc: "Innkeeper Farley".to_string(),
        text: Some("Nothing but rain.".to_string()),
        notice: None,
    };
    assert_eq!(output, Some(answer));
    assert_eq!(people(&mut story)[0].trust, Some(3));
}

#[test]
fn a_talk_with_no_model_gets_no_words() {
    let mut story = story_with("talk-no-model", &[]);
    let (call, _) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "any news?").unwrap()).unwrap());

    let output = one(story.handle(Input::ModelFailed { call }).unwrap());

    let silent = Output::TalkAnswer {
        id: MessageId(8),
        npc: "Innkeeper Farley".to_string(),
        text: None,
        notice: None,
    };
    assert_eq!(output, Some(silent));
    assert_eq!(people(&mut story)[0].trust, None);
}

#[test]
fn empty_long_or_odd_words_are_refused() {
    let mut story = story_with("talk-bad-words", &[]);

    let empty = talk(&mut story, "Innkeeper Farley", "  ");
    let long = talk(&mut story, "Innkeeper Farley", &"w".repeat(256));
    let control = talk(&mut story, "Innkeeper Farley", "hi\u{7}");

    assert!(matches!(empty, Err(StoryError::BadWords)));
    assert!(matches!(long, Err(StoryError::BadWords)));
    assert!(matches!(control, Err(StoryError::BadWords)));
}

#[test]
fn a_talk_answer_after_a_later_event_still_shows_and_lands() {
    let mut story = story_with("talk-later-event", &[]);
    let (call, _) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "any news?").unwrap()).unwrap());
    level(&mut story, 60, 12);

    let text = r#"{"say": "Nothing but rain.", "trust": 2}"#.to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert!(
        matches!(output, Some(Output::TalkAnswer { text: Some(_), .. })),
        "{output:?}"
    );
    assert_eq!(people(&mut story)[0].trust, Some(2));
}

#[test]
fn a_change_of_trust_for_another_character_is_dropped() {
    let mut story = story_with("talk-switch", &[]);
    let (call, _) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "any news?").unwrap()).unwrap());
    let bren = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Bren".to_string(),
    };
    story.handle(bren).unwrap();

    let text = r#"{"say": "Nothing but rain.", "trust": 3}"#.to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert!(
        matches!(output, Some(Output::TalkAnswer { text: Some(_), .. })),
        "{output:?}"
    );
    assert!(people(&mut story).is_empty());
}

fn emote(story: &mut Story, at: u64, emote: &str) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::EmoteDone {
        at: Tick(at),
        emote: emote.to_string(),
        target: None,
        hour: Some(12),
    })
}

fn deeds(story: &mut Story) -> Vec<timeways_story::journal::Deed> {
    match one(story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap())
    {
        Some(Output::Journal { page, .. }) => page.journal.deeds,
        other => panic!("expected a journal, got {other:?}"),
    }
}

#[test]
fn a_third_dance_in_goldshire_earns_a_title_and_the_narrator_speaks_of_it() {
    let mut story = story_with("title-earned", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let _ = batch_end(&mut story, 1);

    for at in 2..=4 {
        assert!(emote(&mut story, at, "dance").unwrap().is_empty());
    }
    let (_, prompt) = model_call(batch_end(&mut story, 2));

    let titled = deeds(&mut story).into_iter().any(|deed| {
        matches!(deed, timeways_story::journal::Deed::Titled { ref title, .. } if title == "Lord of the Goldshire Dance Floor")
    });
    assert!(titled);
    assert!(
        prompt.contains("Lord of the Goldshire Dance Floor"),
        "{prompt}"
    );
}

#[test]
fn a_title_is_earned_only_once() {
    let mut story = story_with("title-once", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));

    for at in 2..=8 {
        emote(&mut story, at, "dance").unwrap();
    }

    let titles = deeds(&mut story)
        .into_iter()
        .filter(|deed| matches!(deed, timeways_story::journal::Deed::Titled { .. }))
        .count();
    assert_eq!(titles, 1);
}

#[test]
fn a_death_to_a_fall_and_to_a_weak_npc_are_flavor_and_a_fair_fight_is_not() {
    let mut story = story_with("silly-deaths", &[]);
    level(&mut story, 1, 60);
    let died = |killer: Option<&str>, cause: Option<&str>, killer_level| Input::Died {
        at: Tick(2),
        killer: killer.map(str::to_string),
        cause: cause.map(str::to_string),
        killer_level,
        hour: Some(3),
    };

    story.handle(died(None, Some("falling"), None)).unwrap();
    story.handle(died(Some("Cow"), None, Some(1))).unwrap();
    story.handle(died(Some("Onyxia"), None, Some(63))).unwrap();

    let humbled = deeds(&mut story).into_iter().any(|deed| {
        matches!(deed, timeways_story::journal::Deed::Titled { ref title, .. } if title == "The Humbled")
    });
    assert!(humbled);
}

#[test]
fn an_odd_emote_or_hour_is_refused() {
    let mut story = story_with("bad-flavor", &[]);

    let upper = emote(&mut story, 1, "DANCE");
    let long = emote(&mut story, 1, &"a".repeat(25));
    let hour = story.handle(Input::EmoteDone {
        at: Tick(1),
        emote: "dance".to_string(),
        target: None,
        hour: Some(24),
    });

    assert!(upper.is_err() && long.is_err() && hour.is_err());
}

fn dance_at(story: &mut Story, at: u64, hour: u8) {
    let input = Input::EmoteDone {
        at: Tick(at),
        emote: "dance".to_string(),
        target: None,
        hour: Some(hour),
    };
    assert!(story.handle(input).unwrap().is_empty());
}

/// Ends a batch and lets each model call of it fail, so no call stays open. Gives the
/// first output: the narrator part of the answer.
fn close_narrator(story: &mut Story, batch: u64) -> Output {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    for output in &outputs {
        if let Output::ModelCall { call, .. } = output {
            story.handle(Input::ModelFailed { call: *call }).unwrap();
        }
    }
    outputs.into_iter().next().unwrap()
}

fn humbled_by(story: &mut Story, at: u64, killer: &str) {
    let input = Input::Died {
        at: Tick(at),
        killer: Some(killer.to_string()),
        cause: None,
        killer_level: Some(1),
        hour: Some(3),
    };
    story.handle(input).unwrap();
}

#[test]
fn a_funny_moment_in_a_quiet_batch_gets_a_flavor_line() {
    let mut story = story_with("flavor-line", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);

    dance_at(&mut story, 100, 3);
    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(
        prompt.contains("The moment:\n<<<\nThe player used the emote /dance in Goldshire, at 3 o'clock, for the 1st time.\n>>>"),
        "{prompt}"
    );
}

/// In a capital, the game names the subzone, such as the Trade District.
#[test]
fn a_moment_in_a_subzone_of_a_famous_city_counts_as_famous() {
    let mut story = story_with("flavor-capital", &[]);
    enter(&mut story, 1, "Stormwind City", Some("Trade District"));
    let _ = close_narrator(&mut story, 1);

    dance_at(&mut story, 100, 12);
    let line = batch_end(&mut story, 2);

    assert!(matches!(line, Output::ModelCall { .. }), "{line:?}");
}

#[test]
fn a_plain_moment_gets_no_line() {
    let mut story = story_with("flavor-plain", &[]);
    enter(&mut story, 1, "Westfall", None);
    let _ = close_narrator(&mut story, 1);

    dance_at(&mut story, 100, 12);

    assert_eq!(
        batch_end(&mut story, 2),
        Output::EventsSeen {
            id: MessageId(2),
            narrator: None,
            notice: None,
        }
    );
}

#[test]
fn a_big_moment_wins_over_a_funny_one() {
    let mut story = story_with("flavor-loses", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    level(&mut story, 2, 12);
    let _ = close_narrator(&mut story, 1);

    dance_at(&mut story, 100, 3);
    level(&mut story, 101, 13);
    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(
        prompt.contains("The moment:\n<<<\nThe player reached level 13.\n>>>"),
        "{prompt}"
    );
}

#[test]
fn a_second_flavor_line_waits_twenty_minutes() {
    let mut story = story_with("flavor-gap", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);
    dance_at(&mut story, 100, 3);
    let _ = close_narrator(&mut story, 2);

    let fall = |at| Input::Died {
        at: Tick(at),
        killer: None,
        cause: Some("falling".to_string()),
        killer_level: None,
        hour: Some(3),
    };
    story.handle(fall(200)).unwrap();
    let soon = batch_end(&mut story, 3);
    story.handle(fall(100 + 1200)).unwrap();
    let later = batch_end(&mut story, 4);

    assert_eq!(
        soon,
        Output::EventsSeen {
            id: MessageId(3),
            narrator: None,
            notice: None,
        }
    );
    assert!(matches!(later, Output::ModelCall { .. }), "{later:?}");
}

/// An emote changes no world, so the time of a telling comes from the newest input, not
/// from the last event of the world.
#[test]
fn the_gap_between_flavor_lines_counts_from_the_time_of_the_line() {
    let mut story = story_with("flavor-clock", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);
    let line_at = 11 * HOUR + 50 * 60;
    dance_at(&mut story, line_at, 3);
    let _ = close_narrator(&mut story, 2);
    meet(&mut story, line_at + 5 * 60, "Innkeeper Farley");
    let _ = close_narrator(&mut story, 3);

    let fall = Input::Died {
        at: Tick(line_at + 6 * 60),
        killer: None,
        cause: Some("falling".to_string()),
        killer_level: None,
        hour: Some(3),
    };
    story.handle(fall).unwrap();
    let six_minutes_later = batch_end(&mut story, 4);

    assert_eq!(
        six_minutes_later,
        Output::EventsSeen {
            id: MessageId(4),
            narrator: None,
            notice: None,
        }
    );
}

#[test]
fn the_same_kind_of_joke_waits_for_the_next_evening() {
    let mut story = story_with("flavor-kind", &[]);
    level(&mut story, 1, 60);
    enter(&mut story, 2, "Elwynn Forest", Some("Goldshire"));
    humbled_by(&mut story, 10, "Cow");
    let _ = close_narrator(&mut story, 1);
    humbled_by(&mut story, 20_000, "Sheep");
    let _ = close_narrator(&mut story, 2);

    humbled_by(&mut story, 20_000 + 2 * 3600, "Goat");
    let same_evening = close_narrator(&mut story, 3);
    humbled_by(&mut story, 20_000 + 13 * 3600, "Boar");
    let next_day = close_narrator(&mut story, 4);

    assert_eq!(
        same_evening,
        Output::EventsSeen {
            id: MessageId(3),
            narrator: None,
            notice: None,
        }
    );
    let Output::ModelCall { prompt, .. } = next_day else {
        panic!("expected a flavor line, got {next_day:?}");
    };
    assert!(
        prompt.contains("Boar, 59 levels below the player"),
        "{prompt}"
    );
}

#[test]
fn the_saga_gets_the_small_moments_of_its_chapter_and_its_footnotes_are_kept_and_told() {
    let mut story = story_with("footnotes", &[]);
    enter(&mut story, HOUR, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);
    dance_at(&mut story, HOUR + 60, 3);
    let _ = close_narrator(&mut story, 2);
    play_fifty_minutes(&mut story, HOUR + 120, "Elwynn Forest");
    new_chapter(&mut story, 5 * HOUR, "Westfall", 91);
    meet(&mut story, 5 * HOUR, "Salma Saldean");

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let [_, Output::ModelCall { call, prompt }] = outputs.as_slice() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    assert!(prompt.contains("Small moments:\n<<<\n1. The player used the emote /dance in Goldshire, at 3 o'clock, for the 1st time."), "{prompt}");
    let text = r#"{"saga": "Our hero came to Goldshire.", "footnotes": [{"moment": 1, "text": "Nobody knows why."}]}"#;
    story
        .handle(Input::ModelAnswered {
            call: *call,
            text: text.to_string(),
        })
        .unwrap();

    let chapters = chapters(&mut story);
    assert_eq!(chapters[0].footnotes, ["Nobody knows why."]);
    dance_at(&mut story, 5 * HOUR + 60, 3);
    assert_eq!(
        close_narrator(&mut story, 4),
        Output::EventsSeen {
            id: MessageId(4),
            narrator: None,
            notice: None,
        }
    );
}

/// Only a flavor line of the narrator starts the gap of 20 minutes (GAMEPLAY.md 5.4.1).
#[test]
fn a_footnote_of_the_chronicle_does_not_hold_back_the_next_flavor_line() {
    let mut story = story_with("footnote-gap", &[]);
    enter(&mut story, HOUR, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);
    dance_at(&mut story, HOUR + 60, 12);
    let _ = close_narrator(&mut story, 2);
    play_fifty_minutes(&mut story, HOUR + 120, "Elwynn Forest");
    new_chapter(&mut story, 4 * HOUR, "Westfall", 91);
    // Back in Goldshire, a famous place, so the fall scores as high as the dance did.
    enter(&mut story, 5 * HOUR, "Elwynn Forest", Some("Goldshire"));
    meet(&mut story, 5 * HOUR, "Salma Saldean");
    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();
    let [_, Output::ModelCall { call, .. }] = outputs.as_slice() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    let text = r#"{"saga": "Our hero came.", "footnotes": [{"moment": 1, "text": "A dance."}]}"#;
    let saga = Input::ModelAnswered {
        call: *call,
        text: text.to_string(),
    };
    // The window is calm, so a second draft comes. It breaks a rule, and the first wins.
    let second = story.handle(saga).unwrap();
    let (second, _) = model_call(one(second).unwrap());
    let text = "no saga".to_string();
    story
        .handle(Input::ModelAnswered { call: second, text })
        .unwrap();

    let fall = Input::Died {
        at: Tick(5 * HOUR + 60),
        killer: None,
        cause: Some("falling".to_string()),
        killer_level: None,
        hour: Some(3),
    };
    story.handle(fall).unwrap();
    let line = batch_end(&mut story, 4);

    assert!(matches!(line, Output::ModelCall { .. }), "{line:?}");
}

fn hero_page(story: &mut Story) -> (timeways_story::hero::Hero, Option<String>) {
    match one(story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap())
    {
        Some(Output::Journal { page, .. }) => (page.journal.hero, page.journal.hero_refused),
        other => panic!("expected a journal, got {other:?}"),
    }
}

fn set_field(story: &mut Story, field: &str, text: &str) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::HeroSet {
        at: Tick(5),
        field: field.to_string(),
        text: text.to_string(),
    })
}

fn add_entry(story: &mut Story, at: u64, text: &str, npc: Option<&str>) {
    let input = Input::HeroAdded {
        at: Tick(at),
        text: text.to_string(),
        npc: npc.map(str::to_string),
    };
    assert!(story.handle(input).unwrap().is_empty());
}

#[test]
fn the_sheet_and_the_entries_of_the_hero_show_on_the_journal() {
    let mut story = story_with("hero-page", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));

    set_field(&mut story, "goal", "Find my brother.").unwrap();
    add_entry(
        &mut story,
        6,
        "A stranger knew my father's name.",
        Some("Innkeeper Farley"),
    );

    let (hero, refused) = hero_page(&mut story);
    assert_eq!(hero.sheet[0].text, "Find my brother.");
    let entry = &hero.entries[0];
    assert_eq!(
        (entry.number, entry.place.as_deref(), entry.npc.as_deref()),
        (1, Some("Goldshire"), Some("Innkeeper Farley"))
    );
    assert_eq!(refused, None);
}

#[test]
fn a_refused_edit_shows_its_reason_once_on_the_next_journal_page() {
    let mut story = story_with("hero-refused", &[]);

    assert!(
        set_field(&mut story, "goal", &"a".repeat(MAX_TEXT_CHARS + 1))
            .unwrap()
            .is_empty()
    );

    let (hero, first) = hero_page(&mut story);
    let (_, second) = hero_page(&mut story);
    assert!(hero.sheet.is_empty());
    assert!(first.is_some_and(|reason| reason.contains("too long")));
    assert_eq!(second, None);
}

#[test]
fn the_player_may_name_anything_in_their_own_text() {
    let mut story = story_with("hero-free", &[]);

    set_field(&mut story, "goal", "Sail to Pandaria.").unwrap();

    let (hero, refused) = hero_page(&mut story);
    assert_eq!(hero.sheet[0].text, "Sail to Pandaria.");
    assert_eq!(refused, None);
}

#[test]
fn a_field_that_the_sheet_does_not_have_is_refused() {
    let mut story = story_with("hero-field", &[]);

    assert!(matches!(
        set_field(&mut story, "wealth", "Much."),
        Err(StoryError::BadName)
    ));
}

#[test]
fn a_removed_entry_leaves_the_journal() {
    let mut story = story_with("hero-removed", &[]);
    add_entry(&mut story, 6, "A", None);
    add_entry(&mut story, 7, "B", None);

    story
        .handle(Input::HeroRemoved {
            at: Tick(8),
            number: 1,
        })
        .unwrap();

    let texts: Vec<String> = hero_page(&mut story)
        .0
        .entries
        .into_iter()
        .map(|entry| entry.text)
        .collect();
    assert_eq!(texts, ["B"]);
}

#[test]
fn the_narrator_knows_who_our_hero_is() {
    let mut story = story_with("hero-narrator", &[]);
    set_field(&mut story, "flaw", "Trusts strangers too fast.").unwrap();
    level(&mut story, 10, 12);
    level(&mut story, 11, 13);

    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(
        prompt.contains("in the player's own words. It is the hero's own story, not canon"),
        "{prompt}"
    );
    assert!(
        prompt.contains("- flaw: Trusts strangers too fast."),
        "{prompt}"
    );
    assert!(
        prompt.contains("The moment:\n<<<\nThe player reached level 13.\n>>>"),
        "{prompt}"
    );
}

#[test]
fn the_narrator_may_name_a_later_place_that_the_player_wrote() {
    let mut story = story_with("hero-later-name", &[]);
    set_field(&mut story, "goal", "Find the road to Shattrath.").unwrap();
    level(&mut story, 10, 12);
    level(&mut story, 11, 13);
    let (call, _) = model_call(batch_end(&mut story, 2));

    let text = "Still no road to Shattrath.".to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert_eq!(
        output,
        Some(Output::EventsSeen {
            id: MessageId(2),
            narrator: Some("Still no road to Shattrath.".to_string()),
            notice: None,
        })
    );
}

#[test]
fn a_narrator_line_may_not_name_what_another_character_wrote() {
    let mut story = story_with("hero-switch-narrator", &[]);
    level(&mut story, 10, 12);
    level(&mut story, 11, 13);
    let (call, _) = model_call(batch_end(&mut story, 2));
    let bren = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Bren".to_string(),
    };
    story.handle(bren).unwrap();
    set_field(&mut story, "goal", "Find the road to Shattrath.").unwrap();

    let text = "Still no road to Shattrath.".to_string();
    let output = one(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert_eq!(
        output,
        Some(Output::EventsSeen {
            id: MessageId(2),
            narrator: None,
            notice: None,
        })
    );
}

#[test]
fn a_talk_carries_the_start_of_a_long_note() {
    let mut story = story_with("hero-long-note", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let long = format!("{}{}", "q".repeat(300), "Q".repeat(700));
    add_entry(&mut story, 2, &long, Some("Innkeeper Farley"));

    let (_, prompt) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "hello").unwrap()).unwrap());

    assert!(prompt.contains(&"q".repeat(300)), "{prompt}");
    assert!(!prompt.contains("qQ"), "{prompt}");
}

#[test]
fn an_npc_hears_only_the_entries_about_it_or_its_place() {
    let mut story = story_with("hero-talk", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    add_entry(&mut story, 2, "About Farley.", Some("Innkeeper Farley"));
    add_entry(&mut story, 3, "About Goldshire.", None);
    enter(&mut story, 4, "Westfall", None);
    add_entry(&mut story, 5, "About Westfall.", None);
    enter(&mut story, 6, "Elwynn Forest", Some("Goldshire"));

    let (_, prompt) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "hello").unwrap()).unwrap());

    assert!(
        prompt.contains("- About Farley.") && prompt.contains("- About Goldshire."),
        "{prompt}"
    );
    assert!(!prompt.contains("About Westfall."), "{prompt}");
}

#[test]
fn the_saga_reads_what_the_player_wrote_in_its_chapter() {
    let mut story = story_with("hero-saga", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");
    add_entry(
        &mut story,
        HOUR + 60,
        "I swore an oath at the Sentinel Hill.",
        None,
    );
    play_fifty_minutes(&mut story, HOUR + 60, "Westfall");
    new_chapter(&mut story, 5 * HOUR, "Duskwood", 91);
    meet(&mut story, 5 * HOUR, "Salma Saldean");

    let outputs = story.handle(Input::BatchEnd { id: MessageId(3) }).unwrap();

    let [_, Output::ModelCall { prompt, .. }] = outputs.as_slice() else {
        panic!("expected a saga call, got {outputs:?}");
    };
    assert!(
        prompt.contains(
            "What the player wrote in this chapter:\n<<<\n- I swore an oath at the Sentinel Hill."
        ),
        "{prompt}"
    );
}

#[test]
fn an_npc_hears_at_most_five_entries_the_newest_first() {
    let mut story = story_with("hero-talk-cap", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    for n in 1..=7 {
        add_entry(
            &mut story,
            1 + n,
            &format!("Entry {n}."),
            Some("Innkeeper Farley"),
        );
    }

    let (_, prompt) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "hello").unwrap()).unwrap());

    let heard: Vec<&str> = prompt
        .lines()
        .filter(|line| line.starts_with("- Entry"))
        .collect();
    assert_eq!(
        heard,
        [
            "- Entry 7.",
            "- Entry 6.",
            "- Entry 5.",
            "- Entry 4.",
            "- Entry 3."
        ]
    );
}

#[test]
fn an_npc_knows_at_most_three_lore_passages() {
    let farley = |n: u32| {
        passage(
            &format!("Innkeeper Farley story {n}."),
            &format!("https://example.test/{n}"),
            vec![npc("Innkeeper Farley")],
        )
    };
    let passages: Vec<Passage> = (1..=5).map(farley).collect();
    let mut story = story_with("talk-passages", &passages);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));

    let (_, prompt) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "your story").unwrap()).unwrap());

    let known = prompt
        .lines()
        .filter(|line| line.contains("story "))
        .count();
    assert_eq!(known, 3, "{prompt}");
}

#[test]
fn an_hour_past_23_and_an_emote_that_is_no_word_are_refused() {
    let mut story = story_with("bad-hour-token", &[]);
    let emote = |emote: &str, hour| Input::EmoteDone {
        at: Tick(1),
        emote: emote.to_string(),
        target: None,
        hour: Some(hour),
    };

    let late = story.handle(emote("dance", 24));
    let shouted = story.handle(emote("Dance!", 3));

    assert!(matches!(late, Err(StoryError::BadHour)), "{late:?}");
    assert!(matches!(shouted, Err(StoryError::BadToken)), "{shouted:?}");
}

fn clock_seconds() -> u64 {
    let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    since_epoch.unwrap().as_secs()
}

const DAY: u64 = 24 * HOUR;

/// The clock of the story reads after the clock of the test, so a day ahead of the test is
/// never more than a day ahead of the story.
#[test]
fn a_time_one_day_ahead_is_taken_and_a_later_one_is_refused() {
    let mut story = story_with("one-day-ahead", &[]);
    let now = clock_seconds();

    let one_day = story.handle(Input::NpcMet {
        at: Tick(now + DAY),
        name: "Tomorrow".to_string(),
        spot: None,
    });
    let past_the_day = story.handle(Input::NpcMet {
        at: Tick(now + DAY + 2 * 60),
        name: "Later".to_string(),
        spot: None,
    });

    assert!(one_day.is_ok(), "{one_day:?}");
    assert!(
        matches!(past_the_day, Err(StoryError::FutureTime(_))),
        "{past_the_day:?}"
    );
}

/// Each passage takes about 3,000 bytes in the slot of the game. The slot has room for
/// four of them next to the longest model text.
#[test]
fn long_passages_stop_where_the_model_text_needs_the_room() {
    let text = "tower ".repeat(492);
    let passages: Vec<Passage> = (0..8)
        .map(|n| {
            passage(
                &text,
                &format!("https://example.test/{n}"),
                vec![place("Testvale")],
            )
        })
        .collect();
    let mut story = story_with("passage-budget", &passages);
    enter(&mut story, 1, "Testvale", None);

    let found = sources(&mut story, "tower", None);

    assert_eq!(found.len(), 4);
}

#[test]
fn a_talk_answer_with_no_change_of_trust_leaves_the_trust_unset() {
    let mut story = story_with("talk-no-trust", &[]);
    let (call, _) =
        model_call(one(talk(&mut story, "Innkeeper Farley", "any news?").unwrap()).unwrap());

    let text = r#"{"say": "Nothing but rain.", "trust": 0}"#.to_string();
    story.handle(Input::ModelAnswered { call, text }).unwrap();

    assert_eq!(people(&mut story)[0].trust, None);
}

fn emote_at(story: &mut Story, at: u64, emote: &str, hour: u8) {
    let input = Input::EmoteDone {
        at: Tick(at),
        emote: emote.to_string(),
        target: None,
        hour: Some(hour),
    };
    assert!(story.handle(input).unwrap().is_empty());
}

#[test]
fn a_moment_of_another_kind_does_not_count_toward_the_times_of_a_dance() {
    let mut story = story_with("flavor-count", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);
    emote_at(&mut story, 100, "wave", 3);
    let _ = close_narrator(&mut story, 2);

    dance_at(&mut story, 100 + 1300, 3);
    let (_, prompt) = model_call(batch_end(&mut story, 3));

    assert!(
        prompt.contains("The moment:\n<<<\nThe player used the emote /dance in Goldshire, at 3 o'clock, for the 1st time.\n>>>"),
        "{prompt}"
    );
}

/// The dance at 3 o'clock scores 10, and the wave at noon scores 8.
#[test]
fn the_flavor_line_is_about_the_best_moment_of_the_batch_not_the_last() {
    let mut story = story_with("flavor-best", &[]);
    enter(&mut story, 1, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);

    dance_at(&mut story, 100, 3);
    emote_at(&mut story, 101, "wave", 12);
    let (_, prompt) = model_call(batch_end(&mut story, 2));

    assert!(prompt.contains("/dance"), "{prompt}");
}

/// The first death to a weak NPC earns a title, and the title takes the line. The joke is
/// told first for the Sheep.
#[test]
fn the_same_kind_of_joke_is_told_again_exactly_twelve_hours_later() {
    let mut story = story_with("flavor-kind-edge", &[]);
    level(&mut story, 1, 60);
    enter(&mut story, 2, "Elwynn Forest", Some("Goldshire"));
    let _ = close_narrator(&mut story, 1);
    humbled_by(&mut story, 10, "Cow");
    let _ = close_narrator(&mut story, 2);
    humbled_by(&mut story, 2000, "Sheep");
    let _ = close_narrator(&mut story, 3);

    humbled_by(&mut story, 2000 + 12 * HOUR, "Boar");
    let twelve_hours_later = close_narrator(&mut story, 4);

    assert!(
        matches!(twelve_hours_later, Output::ModelCall { .. }),
        "{twelve_hours_later:?}"
    );
}

fn saga_prompt(story: &mut Story, batch: u64) -> String {
    let outputs = story
        .handle(Input::BatchEnd {
            id: MessageId(batch),
        })
        .unwrap();
    match outputs.as_slice() {
        [_, Output::ModelCall { prompt, .. }] => prompt.clone(),
        _ => panic!("expected a saga call, got {outputs:?}"),
    }
}

#[test]
fn a_small_moment_at_the_start_of_the_next_chapter_stays_out_of_the_saga_before_it() {
    let mut story = story_with("saga-moment-edge", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");
    let _ = close_narrator(&mut story, 1);
    play_fifty_minutes(&mut story, HOUR, "Westfall");
    new_chapter(&mut story, 5 * HOUR, "Duskwood", 91);
    meet(&mut story, 5 * HOUR, "Salma Saldean");

    dance_at(&mut story, 5 * HOUR, 12);
    let prompt = saga_prompt(&mut story, 2);

    assert!(!prompt.contains("/dance"), "{prompt}");
}

#[test]
fn the_saga_reads_the_entries_from_the_start_of_its_chapter_to_the_start_of_the_next() {
    let mut story = story_with("saga-entry-edge", &[]);
    meet(&mut story, HOUR, "Gryan Stoutmantle");
    add_entry(&mut story, HOUR, "At the start of chapter one.", None);
    play_fifty_minutes(&mut story, HOUR, "Westfall");
    new_chapter(&mut story, 5 * HOUR, "Duskwood", 91);
    meet(&mut story, 5 * HOUR, "Salma Saldean");
    add_entry(&mut story, 5 * HOUR, "At the start of chapter two.", None);

    let prompt = saga_prompt(&mut story, 2);

    let written =
        "What the player wrote in this chapter:\n<<<\n- At the start of chapter one.\n>>>";
    assert!(prompt.contains(written), "{prompt}");
}

#[test]
fn a_talk_at_the_byte_limits_of_the_npc_name_and_the_words_asks_the_model() {
    let mut story = story_with("talk-limits", &[]);
    let npc = "N".repeat(64);
    let words = "w".repeat(255);

    let at_the_limits = talk(&mut story, &npc, &words);
    let long_npc = talk(&mut story, &"N".repeat(65), "hi");

    assert!(
        matches!(one(at_the_limits.unwrap()), Some(Output::ModelCall { .. })),
        "the npc name and the words at their limits are taken"
    );
    assert!(matches!(long_npc, Err(StoryError::BadName)), "{long_npc:?}");
}

fn gossip(story: &mut Story, text: &str) -> Result<Vec<Output>, StoryError> {
    story.handle(Input::TextSeen {
        at: Tick(1),
        kind: timeways_story::seen::TextKind::Gossip,
        title: None,
        npc: None,
        zone: None,
        text: text.to_string(),
    })
}

#[test]
fn a_seen_text_of_2400_bytes_is_taken_and_one_byte_more_is_refused() {
    let mut story = story_with("seen-limit", &[]);

    let at_the_limit = gossip(&mut story, &"a".repeat(2400));
    let past_the_limit = gossip(&mut story, &"b".repeat(2401));

    assert!(at_the_limit.is_ok(), "{at_the_limit:?}");
    assert!(
        matches!(past_the_limit, Err(StoryError::BadSeenText)),
        "{past_the_limit:?}"
    );
}

#[test]
fn an_emote_of_24_letters_at_23_o_clock_is_taken() {
    let mut story = story_with("token-hour-limit", &[]);

    let result = story.handle(Input::EmoteDone {
        at: Tick(1),
        emote: "a".repeat(24),
        target: None,
        hour: Some(23),
    });

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn a_name_of_96_bytes_is_taken() {
    let mut story = story_with("name-limit", &[]);

    let result = story.handle(Input::NpcMet {
        at: Tick(1),
        name: "N".repeat(96),
        spot: None,
    });

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn a_refusal_tells_its_reason() {
    let mut story = story_with("refusal-text", &[]);
    level(&mut story, 1, 6);

    let error = story
        .handle(Input::LevelReached {
            at: Tick(1),
            level: 5,
        })
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "refused: level moves up, and 6 to 5 does not"
    );
}
