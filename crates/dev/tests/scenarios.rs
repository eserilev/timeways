//! The built-in scenarios build the worlds that they promise (TESTING.md, "Dev mode").

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{NAME, REALM, START, folder, journal, list, seed, texts};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use timeways_dev::scenario::{BUILT_IN, Scenario};
use timeways_story::moments::{KINDS, Moment};
use timeways_story::narrator::Who;
use timeways_story::narrator_review::{Sources, reviews};
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::{CharacterKey, Store};
use timeways_story::story::Story;

/// For a person who writes a scenario: `SCENARIO=<name> cargo test -p timeways-dev --test
/// scenarios -- --ignored --nocapture` prints what its world holds.
#[test]
#[ignore = "prints a world for a person"]
fn print_the_world_of_a_scenario() {
    let name = std::env::var("SCENARIO").unwrap_or_else(|_| "fresh".to_string());
    let folder = folder("print");
    let report = seed(&name, &folder);
    println!("{report:#?}");
    let journal = journal(&folder);
    for key in [
        "chapters", "tales", "deeds", "quests", "learned", "stories", "edits",
    ] {
        println!("== {key}");
        for item in list(&journal, key) {
            println!("{item}");
        }
    }
    println!("== summary {:?}", journal.get("summary"));
}

fn numbers(items: &[Value], key: &str) -> Vec<u64> {
    items
        .iter()
        .filter_map(|item| item.get(key).and_then(Value::as_u64))
        .collect()
}

fn with_kind<'a>(items: &'a [Value], kind: &str) -> Vec<&'a Value> {
    items.iter().filter(|item| item["kind"] == kind).collect()
}

#[test]
fn every_built_in_scenario_plays_with_no_refused_line() {
    for (name, _, _) in BUILT_IN {
        let report = seed(name, &folder(name));

        assert!(report.is_clean(), "{name}: {report:#?}");
    }
}

#[test]
fn every_built_in_scenario_reads_as_a_scenario() {
    for (name, about, text) in BUILT_IN {
        let scenario = Scenario::parse(text).unwrap();

        assert!(!scenario.batches.is_empty(), "{name}");
        assert!(about.len() <= 90, "{name}: one short line about it");
    }
}

#[test]
fn the_fresh_scenario_is_one_open_chapter() {
    let folder = folder("fresh");

    seed("fresh", &folder);

    let chapters = list(&journal(&folder), "chapters");
    assert_eq!(texts(&chapters, "state"), ["open"]);
}

#[test]
fn the_level_30_paladin_has_about_ten_chapters_two_tales_a_revenge_and_a_mount() {
    let folder = folder("paladin");

    seed("level-30-paladin", &folder);

    let journal = journal(&folder);
    let chapters = list(&journal, "chapters");
    assert!((8..=12).contains(&chapters.len()), "{}", chapters.len());
    let tales = list(&journal, "tales");
    assert_eq!(texts(&tales, "kind"), ["dungeon", "dungeon"]);
    let deeds = list(&journal, "deeds");
    let deaths = with_kind(&deeds, "died");
    let killers: Vec<_> = deaths
        .iter()
        .filter(|deed| deed["killer"] == "Mor'Ladim")
        .collect();
    assert_eq!(killers.len(), 2);
    assert!(
        with_kind(&deeds, "defeated")
            .iter()
            .any(|deed| deed["foe"] == "Mor'Ladim")
    );
    assert_eq!(with_kind(&deeds, "mounted").len(), 1);
    assert_eq!(with_kind(&deeds, "class_quest_done").len(), 1);
    let quests = list(&journal, "quests");
    assert_eq!(texts(&quests, "status"), ["done", "accepted"]);
}

#[test]
fn the_raider_60_has_tales_with_run_counts_an_epic_mount_and_an_epic_item() {
    let folder = folder("raider");

    seed("raider-60", &folder);

    let journal = journal(&folder);
    let tales = list(&journal, "tales");
    assert_eq!(texts(&tales, "kind"), ["dungeon", "raid"]);
    assert_eq!(numbers(&tales, "runs"), [4, 3]);
    let deeds = list(&journal, "deeds");
    assert!(
        with_kind(&deeds, "mounted")
            .iter()
            .any(|deed| deed["epic"] == true)
    );
    assert_eq!(with_kind(&deeds, "epic_item").len(), 1);
    assert_eq!(with_kind(&deeds, "upgraded").len(), 1);
    assert!(
        with_kind(&deeds, "defeated")
            .iter()
            .any(|deed| deed["foe"] == "Azuregos")
    );
}

#[test]
fn the_story_inbox_holds_two_accepted_stories_and_no_removed_one() {
    let folder = folder("inbox");

    seed("story-inbox", &folder);

    let stories = list(&journal(&folder), "stories");
    assert_eq!(numbers(&stories, "number"), [1, 2]);
    let text = stories[0]["paragraphs"][0].as_str().unwrap();
    assert!(text.starts_with("Kobee held the bridge"), "{text}");
}

#[test]
fn the_edits_scenario_holds_edits_of_chapters_a_tale_and_the_title_page() {
    let folder = folder("edits");

    seed("edits", &folder);

    let edits = list(&journal(&folder), "edits");
    let kinds: Vec<_> = edits
        .iter()
        .map(|edit| edit["entry"]["kind"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(kinds, ["chapter", "chapter", "tale", "summary"]);
    assert_eq!(
        texts(&edits, "text"),
        ["keep", "replace", "replace", "replace"]
    );
}

#[test]
fn the_side_quests_cover_every_state_and_every_kind_of_step() {
    let folder = folder("side-quests");

    seed("side-quests", &folder);

    let quests = list(&journal(&folder), "quests");
    assert_eq!(
        texts(&quests, "status"),
        ["done", "done", "accepted", "accepted", "offered"]
    );
    assert_eq!(quests[3]["hidden_steps"], 1);
    let scenario = common::scenario("side-quests");
    let goals: BTreeSet<String> = scenario
        .batches
        .iter()
        .flat_map(|batch| &batch.answers)
        .flat_map(|answer| goals_of(&serde_json::from_str(&answer.text).unwrap()))
        .collect();
    let every_goal = [
        "any_order",
        "carry",
        "defeat",
        "emote",
        "enter",
        "game_quest",
        "kill",
        "level",
        "meet",
        "slap",
        "talk",
        "visit",
        "visit_at",
        "wait",
    ];
    assert_eq!(goals, every_goal.map(String::from).into());
}

fn goals_of(answer: &Value) -> Vec<String> {
    let mut goals = Vec::new();
    for step in answer["steps"].as_array().unwrap() {
        goals.push(step["goal"].as_str().unwrap().to_string());
        if step.get("steps").is_some() {
            goals.extend(goals_of(step));
        }
    }
    goals
}

#[test]
fn the_outcome_lore_warrior_left_the_deadmines_before_vancleef() {
    let folder = folder("outcome-lore");

    seed("outcome-lore", &folder);

    let deeds = list(&journal(&folder), "deeds");
    let foes: Vec<&Value> = with_kind(&deeds, "defeated")
        .into_iter()
        .map(|deed| &deed["foe"])
        .collect();
    assert!(foes.contains(&&Value::from("Mr. Smite")), "{foes:?}");
    assert!(!foes.contains(&&Value::from("Edwin VanCleef")), "{foes:?}");
}

#[test]
fn the_dungeon_setups_warrior_stands_in_westfall_and_never_entered_the_deadmines() {
    let folder = folder("dungeon-setups");

    seed("dungeon-setups", &folder);

    let journal = journal(&folder);
    assert!(list(&journal, "tales").is_empty(), "{journal:?}");
    let deeds = list(&journal, "deeds");
    assert!(with_kind(&deeds, "defeated").is_empty(), "{deeds:?}");
}

#[test]
fn the_prologue_35_paladin_gets_a_prologue_as_chapter_0() {
    let folder = folder("prologue-35");
    let prologue = r#"{"prologue": "Booty Bay stands on the coast of Stranglethorn Vale, and its goblins trade with every ship that comes into its harbor."}"#;
    let model: timeways_dev::seed::Model = Box::new(move |prompt: &str| {
        prompt
            .contains("Write the prologue of the chronicle")
            .then(|| prologue.to_string())
    });

    let report = timeways_dev::seed::play(
        &common::scenario("prologue-35").starting_at(common::START),
        REALM,
        NAME,
        timeways_story::story::Story::new(Pack::empty().unwrap(), Store::Folder(folder.clone())),
        Some(model),
    );

    assert!(report.is_clean(), "{report:#?}");
    let journal = journal(&folder);
    let chapters = list(&journal, "chapters");
    assert_eq!(chapters[0]["opened_by"], "prologue", "{chapters:?}");
    assert!(journal.get("past").is_none(), "{journal:?}");
}

#[test]
fn the_flavor_and_hero_scenario_earns_every_joke_title_of_the_horde() {
    let folder = folder("flavor");

    seed("flavor-and-hero", &folder);

    let journal = journal(&folder);
    let deeds = list(&journal, "deeds");
    let titles: BTreeSet<_> = with_kind(&deeds, "titled")
        .iter()
        .map(|deed| deed["title"].as_str().unwrap().to_string())
        .collect();
    let earned = [
        "Bookworm",
        "Friend of Gravity",
        "Lava Enthusiast",
        "Slap Happy",
        "Student of the Deep",
        "The Humbled",
    ];
    assert_eq!(titles, earned.map(String::from).into());
    let sheet = journal["hero"]["sheet"].as_array().unwrap();
    assert_eq!(sheet.len(), 11);
    assert_eq!(with_kind(&deeds, "won_battle").len(), 1);
}

/// The name of each kind of narrator moment. A new kind fails to compile here until it
/// gets a name, and then fails `every_kind_of_narrator_moment_comes_in_a_scenario` until
/// `moments::KINDS` holds it and a scenario makes it.
fn kind_of(moment: &Moment) -> &'static str {
    match moment {
        Moment::Flavor { .. } => "flavor",
        Moment::Titled { .. } => "titled",
        Moment::FirstKill { .. } => "first_kill",
        Moment::Revenge { .. } => "revenge",
        Moment::SlainAgain { .. } => "slain_again",
        Moment::Slapped { .. } => "slapped",
        Moment::LevelUp { .. } => "level_up",
        Moment::ClassQuestDone { .. } => "class_quest_done",
        Moment::QuestDone { .. } => "quest_done",
        Moment::NewZone { .. } => "new_zone",
        Moment::FirstInstance { .. } => "first_instance",
        Moment::InstanceAgain { .. } => "instance_again",
        Moment::QuestMarked { .. } => "quest_marked",
        Moment::FirstCapital { .. } => "first_capital",
        Moment::FirstMount { .. } => "first_mount",
        Moment::FirstEpicMount { .. } => "first_epic_mount",
        Moment::FirstEpicItem { .. } => "first_epic_item",
        Moment::BigUpgrade { .. } => "big_upgrade",
    }
}

#[test]
fn every_kind_of_narrator_moment_comes_in_a_scenario() {
    let mut found = BTreeSet::new();
    for (name, _, _) in BUILT_IN {
        let folder = folder(name);
        seed(name, &folder);
        let key = CharacterKey::new(REALM, NAME).unwrap();
        let opened = Store::Folder(folder).open(&key).unwrap();
        let events: Vec<_> = opened.character.world().history().iter().cloned().collect();
        let pack = Pack::empty().unwrap();
        let sources = Sources {
            pack: &pack,
            reads: &[],
            fallback: &Who::default(),
        };
        for review in reviews(&events, &sources).unwrap() {
            let kind = kind_of(&review.moment);
            assert!(KINDS.contains(&kind), "moments::KINDS lacks {kind}");
            found.insert(kind);
        }
    }
    // A flavor moment comes from the score of an emote or a book, never from an event
    // of the world, so the review finds none. The emotes and books of
    // flavor-and-hero make it.
    found.insert("flavor");

    let missing: Vec<_> = KINDS.iter().filter(|kind| !found.contains(*kind)).collect();
    assert!(missing.is_empty(), "no scenario makes {missing:?}");
}

#[test]
fn the_ratings_scenario_rates_two_narrator_lines_and_exports_them_with_no_name() {
    let folder = folder("ratings");

    let report = seed("ratings", &folder);

    assert!(report.is_clean(), "{report:#?}");
    assert_eq!(report.narrator.len(), 2, "{report:#?}");
    let world = timeways_dev::worlds::world_file(&folder, REALM, NAME).unwrap();
    let export = timeways_dev::export_ratings::ratings_of(&world, NAME, "none").unwrap();
    let value = serde_json::to_value(&export).unwrap();
    let ratings: Vec<(&str, &str)> = value["ratings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rating| {
            (
                rating["moment"].as_str().unwrap(),
                rating["rating"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(ratings, [("new_zone", "up"), ("new_zone", "down")]);
    assert!(export.ratings[0].text.starts_with("Duskwood"));
    assert!(
        export.ratings[1]
            .text
            .starts_with("The farmers of Westfall")
    );
    assert_eq!(value["ratings"][0].get("reason"), None);
    assert_eq!(value["ratings"][1]["reason"], "boring");
    assert!(!value.to_string().contains(NAME), "{value}");
}

/// The thumbs of the Chronicle show only on a page with a story of the narrator.
#[test]
fn the_first_chapter_of_the_ratings_scenario_has_a_story_to_rate() {
    let folder = folder("ratings-chapter");

    seed("ratings", &folder);

    let chapters = list(&journal(&folder), "chapters");
    let prose = chapters[0]["prose"].as_str().unwrap_or_default();
    assert!(prose.starts_with("Marshal McBride"), "{chapters:?}");
}

#[test]
fn a_rating_of_a_chapter_keeps_the_story_with_the_mark_of_the_hero() {
    let folder = folder("ratings-of-a-chapter");
    seed("ratings", &folder);
    let first = list(&journal(&folder), "chapters")[0]["first"].clone();
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.clone()));
    let character = json!({"type": "character_entered", "realm": REALM, "name": NAME});
    let rated = json!({"type": "line_rated", "at": START + 9000, "rated": "chapter",
        "first": first, "rating": "up"});

    for line in [character, rated] {
        let served = serve::line(&mut story, line.to_string().into_bytes());
        assert_eq!(served.error, None);
    }

    drop(story);
    let world = timeways_dev::worlds::world_file(&folder, REALM, NAME).unwrap();
    let export = timeways_dev::export_ratings::ratings_of(&world, NAME, "none").unwrap();
    let chapter = export.ratings.last().unwrap();
    assert_eq!(chapter.moment, "chapter");
    assert!(chapter.text.contains("$N took up"), "{}", chapter.text);
}

#[test]
fn a_rated_chapter_shows_its_rating_in_the_journal_after_a_restart() {
    let folder = folder("ratings-thumbs");
    seed("ratings", &folder);
    let first = list(&journal(&folder), "chapters")[0]["first"].clone();
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.clone()));
    let character = json!({"type": "character_entered", "realm": REALM, "name": NAME});
    let rated = json!({"type": "line_rated", "at": START + 9000, "rated": "chapter",
        "first": first, "rating": "down", "reason": "too_long"});
    for line in [character, rated] {
        let served = serve::line(&mut story, line.to_string().into_bytes());
        assert_eq!(served.error, None);
    }
    drop(story);

    let chapters = list(&journal(&folder), "chapters");

    assert_eq!(chapters[0]["rating"], "down", "{chapters:?}");
    assert_eq!(chapters[1].get("rating"), None, "{chapters:?}");
}
