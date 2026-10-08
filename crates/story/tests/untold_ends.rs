//! The end of a boss never reaches `/lore` before the kill (GAMEPLAY.md 5.10). The builder
//! tags every passage that tells an end, and the one gate of the spoiler limit keeps it out.
//! The sentences come from the wiki pages of the pack. The rest of each dump is invented.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod wiki_dump;

use hourglass::Tick;
use std::path::Path;
use timeways_story::input::{FoeKind, Input, MessageId};
use timeways_story::pack::{Dependency, Pack, Passage};
use timeways_story::pack_sources::{Sources, from_dump};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};
use wiki_dump::{article, long, write_dump};

const ARUGAL_FALLS: &str = "Arugal was eventually defeated and beheaded by agents of the Horde.";

const VANCLEEF_FALLS: &str = "VanCleef was considered one of the greatest threats to the \
                              kingdom of Stormwind until he was killed by Alliance forces.";

const SOURCES: &str = r#"
[books]
index = "History of Warcraft"
chapters = []

[[pages]]
title = "Shadowfang Keep"
lead = true
sections = []
places = ["Shadowfang Keep"]
foes = ["Archmage Arugal"]

[[pages]]
title = "Edwin VanCleef"
lead = true
sections = []
places = ["The Deadmines"]
"#;

/// The passages that the builder makes of the two pages.
fn built(name: &str) -> Vec<Passage> {
    let vancleef = format!(
        "{{{{Npcbox\n| name = Edwin VanCleef\n| faction = Neutral\n}}}}\n{}\n",
        long(VANCLEEF_FALLS)
    );
    let dump = write_dump(
        name,
        &[
            article("History of Warcraft", ""),
            article("Shadowfang Keep", &long(ARUGAL_FALLS)),
            article("Edwin VanCleef", &vancleef),
        ],
    );
    from_dump(&dump, &Sources::parse(SOURCES).unwrap())
        .unwrap()
        .passages
}

fn story_of(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("untold-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &built(name)).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    story
        .handle(Input::CharacterEntered {
            realm: "Testrealm".to_string(),
            name: "Tester".to_string(),
        })
        .unwrap();
    story
}

fn enter(story: &mut Story, at: u64, zone: &str) {
    story
        .handle(Input::ZoneEntered {
            at: Tick(at),
            zone: zone.to_string(),
            subzone: None,
            spot: None,
            hour: None,
            taxi: None,
        })
        .unwrap();
}

fn defeat(story: &mut Story, at: u64, name: &str) {
    story
        .handle(Input::NpcDefeated {
            at: Tick(at),
            name: name.to_string(),
            kind: Some(FoeKind::Boss),
        })
        .unwrap();
}

/// The prompt of the `/lore` call, or an empty text when no call goes out.
fn lore_prompt(story: &mut Story, question: &str) -> String {
    let outputs = story
        .handle(Input::LoreAsked {
            id: MessageId(7),
            question: question.to_string(),
            target: None,
        })
        .unwrap();
    outputs
        .into_iter()
        .find_map(|output| match output {
            Output::ModelCall { prompt, .. } => Some(prompt),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn the_builder_ties_the_end_of_arugal_and_of_vancleef_to_their_kills() {
    let passages = built("untold-tags");

    let tag_of = |words: &str| {
        passages
            .iter()
            .find(|passage| passage.text.contains(words))
            .map(|passage| passage.depends_on.clone())
    };

    assert_eq!(
        tag_of("beheaded"),
        Some(vec![Dependency::Foe("Archmage Arugal".to_string())])
    );
    assert_eq!(
        tag_of("Alliance forces"),
        Some(vec![Dependency::Foe("Edwin VanCleef".to_string())])
    );
}

#[test]
fn lore_tells_the_end_of_arugal_only_after_his_kill() {
    let mut story = story_of("arugal");
    enter(&mut story, 1, "Shadowfang Keep");

    let before = lore_prompt(&mut story, "what happened to Arugal?");
    defeat(&mut story, 2, "Archmage Arugal");
    let after = lore_prompt(&mut story, "what happened to Arugal?");

    assert!(!before.contains("beheaded"), "{before}");
    assert!(after.contains("beheaded"), "{after}");
}

#[test]
fn lore_tells_the_end_of_vancleef_only_after_his_kill() {
    let mut story = story_of("vancleef");
    enter(&mut story, 1, "The Deadmines");

    let before = lore_prompt(&mut story, "what happened to VanCleef?");
    defeat(&mut story, 2, "Edwin VanCleef");
    let after = lore_prompt(&mut story, "what happened to VanCleef?");

    assert!(!before.contains("Alliance forces"), "{before}");
    assert!(after.contains("Alliance forces"), "{after}");
}
