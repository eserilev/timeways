//! A kill of a foe that is neither rare nor a boss (GAMEPLAY.md 5.13): it counts as a
//! defeat only when a tag of the pack names the foe.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::{Deed, Dependency, Link, Origin, Pack, Passage, SetupFor};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

fn story_with(name: &str, passages: &[Passage]) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("foe-kills-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, passages).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    story
        .handle(Input::CharacterEntered {
            realm: "Testrealm".to_string(),
            name: "Tester".to_string(),
        })
        .unwrap();
    story
}

fn passage(text: &str) -> Passage {
    Passage {
        text: text.to_string(),
        source: "the wiki page \"Mor'Ladim\"".to_string(),
        links: vec![Link::Place("Duskwood".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: None,
        setup_for: None,
    }
}

/// An outcome passage whose foe is an elite of the open world, not a rare.
fn morladim_falls() -> Passage {
    Passage {
        depends_on: Some(Dependency::Foe("Mor'Ladim".to_string())),
        ..passage(
            "Mor'Ladim haunted the Raven Hill Cemetery until adventurers slew him for Abercrombie.",
        )
    }
}

fn morladim_wanted() -> Passage {
    Passage {
        setup_for: Some(SetupFor {
            deed: Deed::Foe("Mor'Ladim".to_string()),
            instance: "Duskwood".to_string(),
        }),
        ..passage("Abercrombie sent adventurers to kill Mor'Ladim in the Raven Hill Cemetery.")
    }
}

fn enter_duskwood(story: &mut Story) {
    story
        .handle(Input::ZoneEntered {
            at: Tick(1),
            zone: "Duskwood".to_string(),
            subzone: None,
            spot: None,
            hour: None,
            taxi: None,
        })
        .unwrap();
}

fn kill(story: &mut Story, name: &str) {
    let input = Input::NpcKilled {
        at: Tick(2),
        name: name.to_string(),
    };
    story.handle(input).unwrap();
}

/// The text of the passages that a question about Mor'Ladim puts in the prompt.
fn lore_prompt(story: &mut Story) -> String {
    let input = Input::LoreAsked {
        id: MessageId(7),
        question: "what happened to Mor'Ladim?".to_string(),
        target: None,
    };
    story
        .handle(input)
        .unwrap()
        .into_iter()
        .find_map(|output| match output {
            Output::ModelCall { prompt, .. } => Some(prompt),
            _ => None,
        })
        .unwrap_or_default()
}

fn deeds(story: &mut Story) -> Vec<timeways_story::journal::Deed> {
    let input = Input::JournalAsked {
        id: MessageId(1),
        page: 0,
    };
    match story.handle(input).unwrap().into_iter().next() {
        Some(Output::Journal { page, .. }) => page.journal.deeds,
        other => panic!("expected a journal, got {other:?}"),
    }
}

#[test]
fn a_kill_of_a_foe_that_an_outcome_tag_names_unlocks_the_outcome() {
    let mut story = story_with("outcome", &[morladim_falls()]);
    enter_duskwood(&mut story);
    let before = lore_prompt(&mut story);

    kill(&mut story, "Mor'Ladim");
    let after = lore_prompt(&mut story);

    assert!(!before.contains("adventurers slew him"), "{before}");
    assert!(after.contains("adventurers slew him"), "{after}");
}

#[test]
fn a_kill_of_a_foe_that_a_setup_tag_names_makes_the_setup_stale() {
    let mut story = story_with("setup", &[morladim_wanted()]);
    enter_duskwood(&mut story);
    let before = lore_prompt(&mut story);

    kill(&mut story, "Mor'Ladim");
    let after = lore_prompt(&mut story);

    assert!(before.contains("sent adventurers to kill"), "{before}");
    assert!(!after.contains("sent adventurers to kill"), "{after}");
}

#[test]
fn a_kill_of_a_foe_that_no_tag_names_is_no_deed() {
    let mut story = story_with("untagged", &[morladim_falls()]);
    enter_duskwood(&mut story);

    kill(&mut story, "Skeletal Warrior");

    assert!(deeds(&mut story).is_empty());
}
