//! `/twdev`: each command fakes a moment of play through the real code, and only while the
//! desktop says that dev mode is on (TESTING.md, "Dev mode").

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use serde_json::Value;
use timeways_story::dev_mode::DevMode;
use timeways_story::input::Input;
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

const OFF: &str = "|cffc8a064Timeways|r: Dev mode is off.";

/// The first page of a journal of the real story program, in this dev mode.
fn journal_line(mode: DevMode) -> String {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_dev_mode(mode);
    let character = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
    let _ = serve::line(&mut story, character.as_bytes().to_vec());
    let ask = r#"{"type":"journal_asked","id":1}"#;
    serve::line(&mut story, ask.as_bytes().to_vec())
        .lines
        .remove(0)
}

/// A game whose desktop said, in its journal, that dev mode is on.
fn dev_game() -> Game {
    let game = Game::new();
    game.reply(&journal_line(DevMode::On));
    game
}

fn twdev(game: &Game, message: &str) {
    let slash: mlua::Function = game.eval("return function(m) wow.Slash('/twdev', m) end");
    slash.call::<()>(message).unwrap();
}

/// Each line of each batch that went to the desktop, as JSON.
fn sent_lines(game: &Game) -> Vec<Value> {
    game.run("wow.RunTickers()");
    game.sent()
        .iter()
        .flat_map(|batch| {
            batch
                .lines()
                .skip(1)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .map(|line| serde_json::from_str(&line).unwrap())
        .collect()
}

fn types(lines: &[Value]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line["type"].as_str().unwrap().to_string())
        .collect()
}

/// The lines of one command, after the outbox flushed.
fn lines_of(command: &str) -> Vec<Value> {
    let game = dev_game();
    twdev(&game, command);
    sent_lines(&game)
}

#[test]
fn twdev_says_dev_mode_is_off_and_sends_nothing_before_the_desktop_says_on() {
    let game = Game::new();

    twdev(&game, "level 10");

    assert_eq!(game.printed().last().map(String::as_str), Some(OFF));
    assert!(sent_lines(&game).is_empty());
}

#[test]
fn a_journal_without_the_dev_mark_turns_dev_mode_off() {
    let game = dev_game();
    game.reply(&journal_line(DevMode::Off));

    twdev(&game, "level 10");

    assert_eq!(game.printed().last().map(String::as_str), Some(OFF));
    assert!(sent_lines(&game).is_empty());
}

#[test]
fn help_lists_every_command_with_dev_mode_on() {
    let game = dev_game();

    twdev(&game, "help");

    let printed = game.printed().join("\n");
    for command in [
        "level",
        "mount",
        "kill",
        "death",
        "zone",
        "dungeon",
        "dungeon-again",
        "item",
        "chapter-end",
        "bg-win",
        "rest",
        "taxi",
        "peer",
        "inbox",
        "fps",
    ] {
        assert!(printed.contains(&format!("/twdev {command}")), "{command}");
    }
}

#[test]
fn every_fake_event_line_carries_the_dev_mark_and_the_shape_of_the_story_program() {
    let game = dev_game();
    for command in EVERY_COMMAND {
        twdev(&game, command);
    }

    for line in sent_lines(&game) {
        let reply = matches!(
            line["type"].as_str(),
            Some("talk_asked" | "journal_asked" | "lore_asked" | "draft_asked")
        );
        assert_eq!(
            line.get("dev") == Some(&Value::Bool(true)),
            !reply,
            "{line}"
        );
    }
    // `sent_inputs` reads each line as the story program does, so a wrong shape fails.
    assert!(!game.sent_inputs().is_empty());
    for batch in game.sent() {
        assert_eq!(fake_bridge::dropped_lines_of(&batch), Some(0), "{batch}");
    }
}

#[test]
fn level_reaches_a_level() {
    let lines = lines_of("level 30");

    assert_eq!(types(&lines), ["level_reached"]);
    assert_eq!(lines[0]["level"], 30);
}

#[test]
fn mount_rides_a_mount_and_epic_rides_a_swift_one() {
    assert_eq!(lines_of("mount Brown Horse")[0]["speed"], 160);
    let epic = lines_of("mount Swift Brown Wolf epic");
    assert_eq!(epic[0]["mount"], "Swift Brown Wolf");
    assert_eq!(epic[0]["speed"], 200);
}

#[test]
fn kill_defeats_a_rare_or_a_boss_and_with_no_kind_kills_for_a_quest_step() {
    let boss = lines_of("kill Edwin VanCleef boss");
    assert_eq!(types(&boss), ["npc_defeated"]);
    assert_eq!(boss[0]["name"], "Edwin VanCleef");
    assert_eq!(boss[0]["kind"], "boss");
    assert_eq!(lines_of("kill Azuregos worldboss")[0]["kind"], "worldboss");
    assert_eq!(types(&lines_of("kill Defias Pillager")), ["npc_killed"]);
}

#[test]
fn death_dies_to_an_npc_and_fall_drown_and_lava_to_the_world() {
    assert_eq!(lines_of("death Mor'Ladim")[0]["killer"], "Mor'Ladim");
    assert_eq!(lines_of("fall")[0]["cause"], "falling");
    assert_eq!(lines_of("drown")[0]["cause"], "drowning");
    assert_eq!(lines_of("lava")[0]["cause"], "lava");
}

#[test]
fn zone_walks_into_a_zone_and_a_subzone() {
    let lines = lines_of("zone Westfall / Moonbrook");

    assert_eq!(lines[0]["zone"], "Westfall");
    assert_eq!(lines[0]["subzone"], "Moonbrook");
    assert_eq!(lines[0].get("taxi"), None);
}

#[test]
fn dungeon_and_raid_enter_an_instance_of_their_kind() {
    let dungeon = lines_of("dungeon The Deadmines");
    assert_eq!(types(&dungeon), ["zone_entered", "instance_entered"]);
    assert_eq!(dungeon[1]["kind"], "party");
    assert_eq!(lines_of("raid Blackrock Spire")[1]["kind"], "raid");
}

#[test]
fn dungeon_again_steps_out_to_where_you_stand_and_enters_again() {
    let lines = lines_of("dungeon-again The Deadmines");

    assert_eq!(
        types(&lines),
        ["zone_entered", "zone_entered", "instance_entered"]
    );
    assert_ne!(lines[0]["zone"], "The Deadmines");
    assert_eq!(lines[1]["zone"], "The Deadmines");
    assert_eq!(lines[2]["kind"], "party");
}

#[test]
fn item_puts_on_an_epic_item_far_above_the_one_before() {
    let lines = lines_of("item Ironfoe epic");

    assert_eq!(lines[0]["item"], "Ironfoe");
    assert_eq!(lines[0]["quality"], 4);
    let gain = lines[0]["level"].as_i64().unwrap() - lines[0]["replaced"].as_i64().unwrap();
    assert!(gain >= 10, "{gain}");
}

#[test]
fn bg_win_wins_a_battle_in_a_battleground() {
    let lines = lines_of("bg-win");

    assert_eq!(
        types(&lines),
        ["zone_entered", "instance_entered", "bg_won"]
    );
    assert_eq!(lines[1]["kind"], "pvp");
}

#[test]
fn rest_rests_at_an_inn_and_walks_out() {
    let lines = lines_of("rest");

    assert_eq!(lines[0]["resting"], "yes");
    assert_eq!(lines[1]["resting"], "no");
}

#[test]
fn taxi_flies_over_two_zones_and_lands_in_a_third() {
    let lines = lines_of("taxi");

    let taxi: Vec<_> = lines.iter().map(|line| line.get("taxi").cloned()).collect();
    assert_eq!(
        taxi,
        [Some(Value::from("yes")), Some(Value::from("yes")), None]
    );
}

/// Plays the lines that the game sent into a real story program in dev mode, and gives
/// the chapters of its journal.
fn chapters_after(game: &Game) -> Vec<Value> {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Memory);
    story.set_dev_mode(DevMode::On);
    for batch in game.sent() {
        for line in batch.lines() {
            let served = serve::line(&mut story, line.as_bytes().to_vec());
            assert_eq!(served.error, None, "{line}");
        }
    }
    let ask = r#"{"type":"journal_asked","id":9}"#;
    let journal: Value =
        serde_json::from_str(&serve::line(&mut story, ask.as_bytes().to_vec()).lines[0]).unwrap();
    journal["chapters"].as_array().cloned().unwrap_or_default()
}

#[test]
fn chapter_end_closes_the_open_chapter_and_opens_one_in_a_new_zone() {
    let game = dev_game();
    twdev(&game, "zone Westfall / Sentinel Hill");

    twdev(&game, "chapter-end Duskwood");
    game.run("wow.RunTickers() wow.RunTickers()");

    let chapters = chapters_after(&game);
    assert_eq!(chapters.len(), 2, "{chapters:?}");
    assert_eq!(chapters[0]["state"], "closed");
    assert_eq!(chapters[1]["title"], "Duskwood");
}

#[test]
fn talk_asks_the_npc_as_if_you_targeted_it() {
    let inputs = {
        let game = dev_game();
        twdev(&game, "talk Innkeeper Farley / Any news?");
        game.run("wow.RunTickers()");
        game.sent_inputs()
    };

    assert!(inputs.iter().any(|input| matches!(input,
        Input::TalkAsked { npc, text, .. } if npc == "Innkeeper Farley" && text == "Any news?")));
}

#[test]
fn quest_asks_the_npc_for_a_quest_and_the_target_is_real_again_after() {
    let game = dev_game();

    twdev(&game, "quest Gryan Stoutmantle");

    let lines = sent_lines(&game);
    assert_eq!(types(&lines), ["quest_asked"]);
    assert_eq!(lines[0]["npc"], "Gryan Stoutmantle");
    let target: Option<String> = game.eval("ns.Units.NpcName('target')");
    assert_eq!(target, None);
}

#[test]
fn slap_slaps_the_npc_as_if_you_targeted_it() {
    let lines = lines_of("slap Farmer Saldean");

    assert_eq!(types(&lines), ["emote_done", "npc_slapped"]);
    assert_eq!(lines[1]["name"], "Farmer Saldean");
}

#[test]
fn emote_does_an_emote_at_an_npc_or_in_the_place() {
    assert_eq!(lines_of("emote dance")[0]["emote"], "dance");
    assert_eq!(
        lines_of("emote bow / Marshal Dughan")[0]["target"],
        "Marshal Dughan"
    );
}

#[test]
fn meet_and_gossip_meet_the_npc_and_read_its_text() {
    assert_eq!(types(&lines_of("meet Marshal Dughan")), ["npc_met"]);
    let gossip = lines_of("gossip Innkeeper Farley / Rest a while.");
    assert_eq!(types(&gossip), ["npc_met", "text_seen"]);
    assert_eq!(gossip[1]["kind"], "gossip");
    assert_eq!(gossip[1]["npc"], "Innkeeper Farley");
}

#[test]
fn quest_text_reads_a_quest_with_the_name_of_the_character_as_a_mark() {
    let lines = lines_of("quest-text Gryan Stoutmantle / The People's Militia");

    assert_eq!(lines[1]["kind"], "quest");
    assert_eq!(lines[1]["title"], "The People's Militia");
    assert!(lines[1]["text"].as_str().unwrap().contains("$N"));
}

#[test]
fn book_reads_a_book() {
    let lines = lines_of("book The Kingdom of Stormwind");

    assert_eq!(lines[0]["kind"], "book");
    assert_eq!(lines[0]["title"], "The Kingdom of Stormwind");
}

#[test]
fn seen_sees_an_npc_hostile_unless_friendly() {
    let wolf = lines_of("seen Prowler beast");
    assert_eq!(wolf[0]["reaction"], "hostile");
    assert_eq!(wolf[0]["creature"], "beast");
    assert_eq!(lines_of("seen Thor friendly")[0]["reaction"], "friendly");
}

#[test]
fn the_quest_step_commands_send_their_lines() {
    assert_eq!(
        types(&lines_of("game-quest The Tome of Divinity class")),
        ["game_quest_accepted", "game_quest_done"]
    );
    assert_eq!(lines_of("game-quest Westfall Stew")[0]["kind"], "normal");
    assert_eq!(
        lines_of("mark Call of Earth / Earth Sapta")[0]["mark"],
        "Earth Sapta"
    );
    let carry = lines_of("carry 6 Linen Cloth / Farmer Saldean");
    assert_eq!(carry[0]["count"], 6);
    assert_eq!(carry[0]["npc"], "Farmer Saldean");
    assert_eq!(lines_of("hour 22")[0]["hour"], 22);
    assert_eq!(lines_of("pvp-rank 3")[0]["rank"], 3);
}

#[test]
fn a_command_with_bad_words_says_its_usage_and_sends_nothing() {
    let game = dev_game();

    twdev(&game, "level ninety");
    twdev(&game, "item Ironfoe");

    assert!(sent_lines(&game).is_empty());
    assert!(game.printed().iter().any(|line| line.contains("usage")));
}

#[test]
fn welcome_opens_the_setup_window_for_a_reason() {
    let game = dev_game();

    twdev(&game, "welcome offline");

    assert!(game.eval::<bool>("ns.Welcome.IsShown()"));
}

/// Each command of the help, with words, for the tests of every command.
const EVERY_COMMAND: [&str; 37] = [
    "level 12",
    "zone Westfall / Moonbrook",
    "taxi",
    "dungeon The Deadmines",
    "dungeon-again The Deadmines",
    "raid Blackrock Spire",
    "bg-win",
    "pvp-rank 2",
    "rest",
    "mount Brown Horse",
    "kill Hogger rare",
    "kill Prowler",
    "seen Prowler beast",
    "death Hogger",
    "fall",
    "drown",
    "lava",
    "item Lionheart Helm epic",
    "game-quest Westfall Stew",
    "mark Call of Earth / Earth Sapta",
    "chapter-end",
    "hour 22",
    "meet Thor",
    "gossip Thor / Fly safe.",
    "quest-text Thor / Flight to Stormwind",
    "book The Kingdom of Stormwind",
    "talk Thor",
    "quest Thor",
    "slap Thor",
    "emote dance",
    "carry 3 Linen Cloth / Thor",
    "journal",
    "welcome setup",
    "peer Kobee quest",
    "inbox",
    "fps start bench",
    "fps stop",
];

#[test]
fn every_command_of_the_help_has_a_test() {
    let game = dev_game();
    let usages: Vec<String> = game.eval("ns.Dev.Usages()");
    let tested: Vec<&str> = EVERY_COMMAND
        .iter()
        .chain(PLAYER_COMMANDS.iter())
        .map(|command| command.split(' ').next().unwrap())
        .collect();

    for usage in usages {
        let name = usage.split(' ').next().unwrap().trim_end_matches(':');
        assert!(tested.contains(&name), "no test runs /twdev {name}");
    }
}

/// The commands of fake players that the tests of `dev_peer.rs` run.
const PLAYER_COMMANDS: [&str; 4] = ["msp", "remember", "note", "tooltip"];

/// The lines of the protocol that no play makes: the bridge and the model make them.
const PROTOCOL_LINES: [&str; 5] = [
    "hello",
    "character_entered",
    "batch_end",
    "model_answered",
    "model_failed",
];

/// Every type of input line of the story program, as serde names them in its error.
fn every_input_type() -> Vec<String> {
    let error = serde_json::from_str::<Input>(r#"{"type":"no_such_line"}"#)
        .unwrap_err()
        .to_string();
    let expected = error.split("expected one of ").nth(1).unwrap();
    expected
        .split(", ")
        .map(|name| name.split('`').nth(1).unwrap().to_string())
        .collect()
}

#[test]
fn every_input_line_has_a_dev_command_or_a_scenario() {
    let game = dev_game();
    for command in EVERY_COMMAND {
        twdev(&game, command);
    }
    let mut covered: Vec<String> = types(&sent_lines(&game));
    for (_, _, text) in timeways_dev::scenario::BUILT_IN {
        let scenario = timeways_dev::scenario::Scenario::parse(text).unwrap();
        for line in scenario.batches.iter().flat_map(|batch| &batch.lines) {
            covered.push(line["type"].as_str().unwrap().to_string());
        }
    }

    let every = every_input_type();
    assert!(every.len() > 40, "{every:?}");
    let missing: Vec<&String> = every
        .iter()
        .filter(|kind| !PROTOCOL_LINES.contains(&kind.as_str()) && !covered.contains(kind))
        .collect();
    assert!(
        missing.is_empty(),
        "no /twdev command and no scenario makes {missing:?}"
    );
}

/// The scenario that fills each section of the journal, and the list of the journal that it
/// fills. The Stories tab also shows the stories that wait, which `/twdev inbox` brings.
const SECTION_SCENARIOS: [(&str, &str, &str); 6] = [
    ("hero", "flavor-and-hero", "hero"),
    ("chapters", "level-30-paladin", "chapters"),
    ("stories", "story-inbox", "stories"),
    ("deeds", "raider-60", "deeds"),
    ("learned", "level-30-paladin", "learned"),
    ("quests", "side-quests", "quests"),
];

/// Every page of the journal of a world built from this scenario, with each list joined.
fn journal_of_scenario(name: &str) -> Value {
    let folder =
        std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-section-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    let text = timeways_dev::scenario::built_in(name).unwrap();
    let scenario = timeways_dev::scenario::Scenario::parse(text).unwrap();
    let story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.clone()));
    let start = scenario.starting_at(1_780_000_000);
    let report = timeways_dev::seed::play(&start, "R", "N", story, None);
    assert!(report.is_clean(), "{report:#?}");
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder));
    let character = r#"{"type":"character_entered","realm":"R","name":"N"}"#;
    let _ = serve::line(&mut story, character.as_bytes().to_vec());
    let mut joined = serde_json::Map::new();
    for page in 0.. {
        let ask = format!(r#"{{"type":"journal_asked","id":1,"page":{page}}}"#);
        let line = serve::line(&mut story, ask.into_bytes()).lines.remove(0);
        let Value::Object(object) = serde_json::from_str(&line).unwrap() else {
            panic!("{line}");
        };
        let pages = object["pages"].as_u64().unwrap();
        for (key, value) in object {
            match (joined.get_mut(&key), value) {
                (Some(Value::Array(list)), Value::Array(more)) => list.extend(more),
                (_, value) => {
                    joined.insert(key, value);
                }
            }
        }
        if page + 1 >= pages {
            break;
        }
    }
    Value::Object(joined)
}

#[test]
fn every_section_of_the_journal_has_a_scenario_that_fills_it() {
    let sections: Vec<String> = dev_game().eval("ns.Journal.SECTIONS");

    for section in sections {
        let (_, scenario, list) = SECTION_SCENARIOS
            .iter()
            .find(|(name, _, _)| *name == section)
            .unwrap_or_else(|| panic!("no scenario fills the section {section}"));
        let journal = journal_of_scenario(scenario);
        let filled = match &journal[list] {
            Value::Array(items) => !items.is_empty(),
            Value::Object(hero) => hero["sheet"]
                .as_array()
                .is_some_and(|sheet| !sheet.is_empty()),
            _ => false,
        };
        assert!(filled, "{section}: {}", journal[list]);
    }
}
