//! The lore of Edwin VanCleef after his kill (GAMEPLAY.md 3.1 and 5.10), replayed from the
//! inputs of a real world: the player entered the Deadmines under the game name
//! "Deadmines", never visited Moonbrook, and killed VanCleef. The pages are invented, and
//! they are filed as the pack files them: his own page on the Deadmines, and the end of
//! the Brotherhood on Moonbrook.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::{Deed, Dependency, Link, Origin, Pack, Passage, SetupFor};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const REPLAY: &str = include_str!("replays/deadmines_kill.jsonl");

/// The line of the replay with the kill. The lines before it are the play before.
const KILL_LINE: usize = 10;

const HIS_PAGE: &str = "Edwin VanCleef led the Stonemasons Guild that rebuilt Stormwind City, \
                        until the House of Nobles refused to pay the masons for their work.";
const HIS_END: &str = "Edwin VanCleef held the Deadmines for years, until adventurers fought \
                       through the mine and killed him on the deck of his ship.";
const MOONBROOK_FREED: &str = "The farmers of Moonbrook went home again after adventurers \
                               killed Edwin VanCleef, the leader of the Defias Brotherhood.";
const MINE_HISTORY: &str = "The Deadmines were once the richest gold mine of Westfall, until \
                            the Defias Brotherhood took its tunnels for a secret shipyard.";
const STOUTMANTLE_ASKS: &str = "Gryan Stoutmantle sent adventurers into the Deadmines to kill \
                                Edwin VanCleef and to end the Defias Brotherhood for good.";

fn passage(text: &str, about: &str, place: &str) -> Passage {
    Passage {
        text: text.to_string(),
        source: format!("the wiki page \"{about}\""),
        links: vec![Link::Place(place.to_string())],
        origin: Origin::Pack,
        about: Some(about.to_string()),
        depends_on: Vec::new(),
        setup_for: None,
    }
}

fn after_his_kill(passage: Passage) -> Passage {
    Passage {
        depends_on: vec![Dependency::Foe("Edwin VanCleef".to_string())],
        ..passage
    }
}

fn pack_passages() -> Vec<Passage> {
    let setup = Passage {
        setup_for: Some(SetupFor {
            deed: Deed::Foe("Edwin VanCleef".to_string()),
            instance: "The Deadmines".to_string(),
        }),
        ..passage(STOUTMANTLE_ASKS, "Edwin VanCleef", "The Deadmines")
    };
    vec![
        passage(MINE_HISTORY, "The Deadmines", "The Deadmines"),
        passage(HIS_PAGE, "Edwin VanCleef", "The Deadmines"),
        setup,
        after_his_kill(passage(HIS_END, "Edwin VanCleef", "The Deadmines")),
        after_his_kill(passage(MOONBROOK_FREED, "Moonbrook", "Moonbrook")),
    ]
}

fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("deadmines-kill-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &pack_passages()).unwrap();
    Story::new(Pack::open(&path).unwrap(), Store::Memory)
}

fn inputs() -> Vec<Input> {
    REPLAY
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// Plays the inputs, and refuses each model call at once. Returns the prompts.
fn play(story: &mut Story, inputs: Vec<Input>) -> Vec<String> {
    let mut prompts = Vec::new();
    for input in inputs {
        let mut open = story.handle(input).unwrap();
        while let Some(output) = open.pop() {
            let Output::ModelCall { call, prompt } = output else {
                continue;
            };
            prompts.push(prompt);
            let refused = Input::ModelAnswered {
                call,
                text: String::new(),
            };
            open.extend(story.handle(refused).unwrap());
        }
    }
    prompts
}

fn lore_prompt(story: &mut Story) -> String {
    let asked = Input::LoreAsked {
        id: MessageId(99),
        question: "What happened to Edwin VanCleef?".to_string(),
        target: None,
    };
    story
        .handle(asked)
        .unwrap()
        .into_iter()
        .find_map(|output| match output {
            Output::ModelCall { prompt, .. } => Some(prompt),
            _ => None,
        })
        .unwrap_or_default()
}

fn narrator_prompt<'a>(prompts: &'a [String], moment: &str) -> Option<&'a String> {
    prompts.iter().find(|prompt| {
        prompt
            .split("The moment:")
            .last()
            .is_some_and(|part| part.contains(moment))
    })
}

fn before_the_kill() -> Vec<Input> {
    inputs().into_iter().take(KILL_LINE).collect()
}

fn the_kill() -> Vec<Input> {
    inputs().into_iter().skip(KILL_LINE).collect()
}

#[test]
fn the_entry_into_the_deadmines_under_its_game_name_gets_lore() {
    let mut story = story("entry");

    let prompts = play(&mut story, before_the_kill());

    let entry = narrator_prompt(&prompts, "entered the dungeon Deadmines").expect("no entry call");
    assert!(!entry.contains("The lore: none"), "{entry}");
    assert!(
        entry.contains("Gryan Stoutmantle sent adventurers"),
        "{entry}"
    );
}

#[test]
fn before_the_kill_lore_never_tells_his_end() {
    let mut story = story("before");
    play(&mut story, before_the_kill());

    let lore = lore_prompt(&mut story);

    assert!(lore.contains("Stonemasons Guild"), "{lore}");
    assert!(!lore.contains("killed him on the deck"), "{lore}");
    assert!(!lore.contains("went home again"), "{lore}");
}

#[test]
fn after_the_kill_lore_tells_his_own_page_and_his_end() {
    let mut story = story("after");
    play(&mut story, before_the_kill());
    play(&mut story, the_kill());

    let lore = lore_prompt(&mut story);

    assert!(lore.contains("Stonemasons Guild"), "{lore}");
    assert!(lore.contains("killed him on the deck"), "{lore}");
    assert!(lore.contains("went home again"), "{lore}");
    assert!(!lore.contains("Gryan Stoutmantle sent"), "{lore}");
}

#[test]
fn the_kill_of_vancleef_gets_a_narrator_moment_with_his_lore() {
    let mut story = story("narrator");
    play(&mut story, before_the_kill());

    let prompts = play(&mut story, the_kill());

    let kill = narrator_prompt(&prompts, "defeated Edwin VanCleef").expect("no call for the kill");
    assert!(!kill.contains("The lore: none"), "{kill}");
    assert!(
        kill.contains("Edwin VanCleef led the Stonemasons"),
        "{kill}"
    );
}

#[test]
fn a_kill_with_no_visit_still_tells_his_end_in_lore() {
    let mut story = story("dev-kill");
    let entered = inputs().into_iter().take(1);
    let kill = inputs().into_iter().skip(KILL_LINE);
    play(&mut story, entered.chain(kill).collect());

    let lore = lore_prompt(&mut story);

    assert!(lore.contains("killed him on the deck"), "{lore}");
    assert!(lore.contains("went home again"), "{lore}");
    assert!(!lore.contains("Stonemasons Guild"), "{lore}");
}
