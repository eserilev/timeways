#![allow(clippy::unwrap_used)]

use hourglass::{Event, Tick};
use std::path::Path;
use std::process::Command;
use timeways_story::character::Character;
use timeways_story::input::Input;
use timeways_story::learned::Read;
use timeways_story::moments::Moment;
use timeways_story::narrator::Who;
use timeways_story::narrator_review::{Sources, reviews};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::race_class::{Class, Race};
use timeways_story::seen::{SeenText, TextKind};
use timeways_story::store::Store;
use timeways_story::story::Story;

fn pack_with(name: &str, passages: &[Passage]) -> Pack {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("review-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, passages).unwrap();
    Pack::open(&path).unwrap()
}

fn westfall() -> Passage {
    Passage {
        text: "The Defias Brotherhood holds Westfall.".to_string(),
        source: "the wiki page \"Westfall\"".to_string(),
        links: vec![Link::Place("Westfall".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: None,
        setup_for: None,
    }
}

fn events_of(character: &Character) -> Vec<Event> {
    character.world().history().iter().cloned().collect()
}

#[test]
fn each_big_moment_of_a_history_gets_a_prompt_and_a_plain_level_up_none() {
    let mut character = Character::new();
    character.reach_level(Tick(1), 18).unwrap();
    character.enter_zone(Tick(2), "Westfall", None).unwrap();
    character.reach_level(Tick(3), 19).unwrap();
    character.reach_level(Tick(4), 20).unwrap();
    let pack = pack_with("moments", &[westfall()]);
    let sources = Sources {
        pack: &pack,
        reads: &[],
        fallback: &Who::default(),
    };

    let found = reviews(&events_of(&character), &sources).unwrap();

    let moments: Vec<&Moment> = found.iter().map(|review| &review.moment).collect();
    assert_eq!(
        moments,
        [
            &Moment::NewZone {
                zone: "Westfall".to_string()
            },
            &Moment::LevelUp {
                level: 20,
                zone: Some("Westfall".to_string())
            },
        ]
    );
    assert!(
        found[0]
            .prompt
            .contains("The lore:\n<<<\nThe Defias Brotherhood holds Westfall.\n>>>"),
        "{}",
        found[0].prompt
    );
}

#[test]
fn a_review_reads_only_the_text_that_was_read_before_the_moment() {
    let mut character = Character::new();
    character.enter_zone(Tick(5), "Duskwood", None).unwrap();
    let read_later = Read {
        at: Tick(9),
        text: SeenText {
            kind: TextKind::Gossip,
            title: None,
            npc: None,
            zone: Some("Duskwood".to_string()),
            text: "Duskwood was called Brightwood once.".to_string(),
        },
    };
    let pack = pack_with("reads", &[]);
    let sources = Sources {
        pack: &pack,
        reads: &[read_later],
        fallback: &Who::default(),
    };

    let found = reviews(&events_of(&character), &sources).unwrap();

    assert!(
        found[0].prompt.contains("The lore: none"),
        "{}",
        found[0].prompt
    );
}

#[test]
fn a_world_from_before_the_race_and_the_class_takes_them_from_the_review() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Tirisfal Glades", None)
        .unwrap();
    let pack = pack_with("fallback", &[]);
    let fallback = Who {
        race: Some(Race::Forsaken),
        class: Some(Class::Warlock),
        titles: Vec::new(),
    };
    let sources = Sources {
        pack: &pack,
        reads: &[],
        fallback: &fallback,
    };

    let found = reviews(&events_of(&character), &sources).unwrap();

    let (setup, _) = found[0].templated.as_ref().unwrap();
    assert_eq!(setup.who, fallback);
    assert!(
        found[0].prompt.contains("Answer with JSON only"),
        "{}",
        found[0].prompt
    );
}

fn mine_passage(text: &str, about: Option<&str>) -> Passage {
    Passage {
        text: text.to_string(),
        source: "the wiki page".to_string(),
        links: vec![Link::Place("The Deadmines".to_string())],
        origin: Origin::Pack,
        about: about.map(str::to_string),
        depends_on: None,
        setup_for: None,
    }
}

#[test]
fn the_own_page_of_a_place_wins_over_a_boss_page_linked_to_it() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "The Deadmines", None)
        .unwrap();
    let boss = mine_passage(
        "Mr. Smite of The Deadmines serves in The Deadmines, the deep Deadmines.",
        None,
    );
    let history = mine_passage(
        "The Miners' League dug this mine first.",
        Some("The Deadmines"),
    );
    let pack = pack_with("own-page", &[boss, history]);
    let sources = Sources {
        pack: &pack,
        reads: &[],
        fallback: &Who::default(),
    };

    let found = reviews(&events_of(&character), &sources).unwrap();

    assert!(
        found[0]
            .prompt
            .contains("The lore:\n<<<\nThe Miners' League dug this mine first.\n>>>"),
        "{}",
        found[0].prompt
    );
}

/// A world on disk, as the story program saves it, with one new zone.
fn saved_world(folder: &Path) -> std::path::PathBuf {
    let _ = std::fs::remove_dir_all(folder);
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.to_path_buf()));
    let lines = [
        Input::CharacterEntered {
            realm: "Classic Beta".to_string(),
            name: "Kobee".to_string(),
        },
        Input::ZoneEntered {
            at: Tick(1_700_000_000),
            zone: "Westfall".to_string(),
            subzone: None,
            spot: None,
            hour: None,
            taxi: None,
        },
    ];
    for line in lines {
        story.handle(line).unwrap();
    }
    folder.join("worlds/r_Classic_20Beta/c_Kobee.sqlite")
}

#[test]
fn the_review_tool_prints_the_prompt_of_each_moment_and_leaves_the_world_as_it_was() {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("review-tool");
    let world = saved_world(&folder);
    let before = std::fs::read(&world).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_timeways-narrator-review"))
        .arg(&world)
        .args(["--race", "Human", "--class", "PALADIN"])
        .output()
        .unwrap();

    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{output:?}");
    assert!(
        printed.contains("=== 1 of 1, at 1700000000: The player arrived in Westfall."),
        "{printed}"
    );
    assert!(printed.contains("Answer with JSON only"), "{printed}");
    assert_eq!(std::fs::read(&world).unwrap(), before);
}

#[test]
fn the_review_tool_with_a_model_prints_the_line_that_the_player_sees() {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("review-tool-model");
    let world = saved_world(&folder);

    let output = Command::new(env!("CARGO_BIN_EXE_timeways-narrator-review"))
        .arg(&world)
        .args([
            "--model",
            "echo '{\"lore\": \"Westfall was farmland once, before the Defias.\"}'",
        ])
        .output()
        .unwrap();

    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(
        printed.contains("Shown: Westfall was farmland once, before the Defias."),
        "{printed}"
    );
}

#[test]
fn the_review_tool_shows_the_retry_of_a_refused_line() {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("review-tool-retry");
    let world = saved_world(&folder);

    let output = Command::new(env!("CARGO_BIN_EXE_timeways-narrator-review"))
        .arg(&world)
        .args(["--model", "echo '{\"lore\": \"Our hero moved on.\"}'"])
        .output()
        .unwrap();

    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("Refused: \"our hero\""), "{printed}");
    assert!(
        printed.contains("Retry: {\"lore\": \"Our hero moved on.\"}"),
        "{printed}"
    );
    assert!(printed.contains("Shown: (silence:"), "{printed}");
}
