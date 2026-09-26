#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::{Path, PathBuf};
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::seen::{MAX_SEEN_BYTES, TextKind};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story, StoryError};

fn folder(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("seen-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn story_in(folder: &Path, passages: &[Passage], files: Store) -> Story {
    let pack = folder.join("pack.sqlite");
    if !pack.exists() {
        Pack::write(&pack, passages).unwrap();
    }
    let mut story = Story::new(Pack::open(&pack).unwrap(), files);
    let character = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    story.handle(character).unwrap();
    story
}

fn story_with(name: &str, passages: &[Passage]) -> Story {
    story_in(&folder(name), passages, Store::Memory)
}

fn gossip(npc: &str, text: &str) -> Input {
    Input::TextSeen {
        at: Tick(1),
        kind: TextKind::Gossip,
        title: None,
        npc: Some(npc.to_string()),
        zone: Some("Elwynn Forest".to_string()),
        text: text.to_string(),
    }
}

fn see(story: &mut Story, npc: &str, text: &str) {
    assert_eq!(story.handle(gossip(npc, text)).unwrap(), []);
}

/// The sources of an answer, as a player with no model sees them.
fn sources(story: &mut Story, question: &str) -> Vec<String> {
    let input = Input::LoreAsked {
        id: MessageId(7),
        question: question.to_string(),
        target: None,
    };
    let mut outputs = story.handle(input).unwrap();
    if let Some(Output::ModelCall { call, .. }) = outputs.first() {
        outputs = story.handle(Input::ModelFailed { call: *call }).unwrap();
    }
    let Some(Output::LoreAnswer { answer, .. }) = outputs.pop() else {
        panic!("expected a lore answer, got {outputs:?}");
    };
    answer
        .passages
        .into_iter()
        .map(|passage| passage.source)
        .collect()
}

fn pack_passage(text: &str, source: &str) -> Passage {
    Passage {
        text: text.to_string(),
        source: source.to_string(),
        links: vec![Link::Place("Elwynn Forest".to_string())],
        origin: Origin::Pack,
    }
}

#[test]
fn a_text_seen_line_reads() {
    let line = r#"{"type":"text_seen","at":5,"kind":"quest","title":"Wanted: Hogger","text":"Well met, $N."}"#;

    let input: Input = serde_json::from_str(line).unwrap();

    let expected = Input::TextSeen {
        at: Tick(5),
        kind: TextKind::Quest,
        title: Some("Wanted: Hogger".to_string()),
        npc: None,
        zone: None,
        text: "Well met, $N.".to_string(),
    };
    assert_eq!(input, expected);
}

#[test]
fn gossip_that_you_saw_answers_a_question_with_an_empty_pack() {
    let mut story = story_with("empty-pack", &[]);
    see(
        &mut story,
        "Innkeeper Farley",
        "The gnolls of Hogger grow bold.",
    );

    let found = sources(&mut story, "who is hogger");

    assert_eq!(found, ["Innkeeper Farley, in Elwynn Forest"]);
}

#[test]
fn text_that_you_saw_passes_the_spoiler_limit() {
    let mut story = story_with("spoiler", &[]);

    see(&mut story, "Marshal Dughan", "The Defias hide in Westfall.");

    assert_eq!(sources(&mut story, "defias").len(), 1);
}

#[test]
fn text_that_you_read_comes_first_and_the_pack_fills_the_rest() {
    let pack = [
        pack_passage("Gnolls roam the forest.", "https://example.test/1"),
        pack_passage("Gnolls fear fire.", "https://example.test/2"),
    ];
    let mut story = story_with("turns", &pack);
    story
        .handle(Input::ZoneEntered {
            at: Tick(1),
            zone: "Elwynn Forest".to_string(),
            subzone: None,
        })
        .unwrap();
    see(&mut story, "Guard Thomas", "Gnolls took the farm.");
    see(&mut story, "Guard Thomas", "Gnolls stole our bread.");

    let found = sources(&mut story, "gnolls");

    assert_eq!(found.len(), 4);
    assert!(found[0].starts_with("Guard Thomas"));
    assert!(found[1].starts_with("Guard Thomas"));
    assert!(found[2].starts_with("https://"));
    assert!(found[3].starts_with("https://"));
}

#[test]
fn text_that_you_saw_stays_after_a_restart_and_is_written_once() {
    let folder = folder("restart");
    let mut story = story_in(&folder, &[], Store::Folder(folder.clone()));
    see(
        &mut story,
        "Innkeeper Farley",
        "Welcome to the Lion's Pride.",
    );
    see(
        &mut story,
        "Innkeeper Farley",
        "Welcome to the Lion's Pride.",
    );
    drop(story);

    let mut story = story_in(&folder, &[], Store::Folder(folder.clone()));
    see(
        &mut story,
        "Innkeeper Farley",
        "Welcome to the Lion's Pride.",
    );

    assert_eq!(sources(&mut story, "pride").len(), 1);
    let file = std::fs::read_dir(folder.join("worlds"))
        .unwrap()
        .flat_map(|realm| std::fs::read_dir(realm.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path())
        .find(|path| path.to_string_lossy().ends_with(".seen.jsonl"))
        .expect("a seen file");
    assert_eq!(std::fs::read_to_string(file).unwrap().lines().count(), 1);
}

#[test]
fn a_seen_text_keeps_its_line_breaks() {
    let mut story = story_with("line-breaks", &[]);

    let result = story.handle(gossip("Innkeeper Farley", "Welcome.\n\nSit down."));

    assert!(result.is_ok());
}

#[test]
fn a_seen_text_with_another_control_character_is_refused() {
    let mut story = story_with("control", &[]);

    let result = story.handle(gossip("Innkeeper Farley", "Welcome.\u{7}"));

    assert!(matches!(result, Err(StoryError::BadSeenText)));
}

#[test]
fn a_seen_text_that_is_too_long_or_empty_is_refused() {
    let mut story = story_with("too-long", &[]);

    let long = story.handle(gossip("Innkeeper Farley", &"a".repeat(MAX_SEEN_BYTES + 1)));
    let empty = story.handle(gossip("Innkeeper Farley", " \n "));

    assert!(matches!(long, Err(StoryError::BadSeenText)));
    assert!(matches!(empty, Err(StoryError::BadSeenText)));
}

#[test]
fn a_seen_text_with_a_bad_name_is_refused() {
    let mut story = story_with("bad-name", &[]);

    let result = story.handle(gossip("", "Welcome."));

    assert!(matches!(result, Err(StoryError::BadName)));
}

#[test]
fn a_seen_text_before_a_character_is_refused() {
    let pack = folder("no-character").join("pack.sqlite");
    Pack::write(&pack, &[]).unwrap();
    let mut story = Story::new(Pack::open(&pack).unwrap(), Store::Memory);

    let result = story.handle(gossip("Innkeeper Farley", "Welcome."));

    assert!(matches!(result, Err(StoryError::NoCharacter)));
}

#[test]
fn an_npc_that_you_talk_to_knows_what_it_told_you() {
    let mut story = story_with("talk", &[]);
    see(&mut story, "Innkeeper Farley", "The gnolls grow bold.");

    let outputs = story
        .handle(Input::TalkAsked {
            id: MessageId(3),
            at: Tick(2),
            npc: "Innkeeper Farley".to_string(),
            text: "any news about gnolls".to_string(),
        })
        .unwrap();

    let Some(Output::ModelCall { prompt, .. }) = outputs.first() else {
        panic!("expected a model call, got {outputs:?}");
    };
    assert!(prompt.contains("The gnolls grow bold."));
}
