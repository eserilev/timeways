//! AI off: the player chose no model at all (GAMEPLAY.md 5.6). The addon says so in its
//! `ai_set` line, and the journal tells the addon.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use fake_bridge::{FakeBridge, Reply};
use hourglass::Tick;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use timeways_story::ai::{Ai, AiWord};
use timeways_story::dev_mode::{DevMode, SETTINGS_FILE};
use timeways_story::input::{Input, MessageId};
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::race_class::{Class, Race};
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const CHARACTER: &str = r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#;
const HELLO_AI_OFF: &str = r#"{"type":"hello","protocol":1,"app":"timeways","ai":"off"}"#;

fn story() -> Story {
    Story::new(Pack::empty().unwrap(), Store::Memory)
}

fn send(story: &mut Story, line: &str) -> serve::Served {
    serve::line(story, line.as_bytes().to_vec())
}

fn journal(story: &mut Story) -> Value {
    send(story, CHARACTER);
    let served = send(story, r#"{"type":"journal_asked","id":9}"#);
    serde_json::from_str(&served.lines[0]).unwrap()
}

fn folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ai-{name}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn the_journal_says_ai_is_on_by_default() {
    assert_eq!(journal(&mut story())["ai"], json!("on"));
}

#[test]
fn the_journal_says_ai_is_off_after_a_hello_with_ai_off() {
    let mut story = story();

    send(&mut story, HELLO_AI_OFF);

    assert_eq!(journal(&mut story)["ai"], json!("off"));
}

#[test]
fn the_journal_with_ai_off_passes_the_checks_of_the_bridge() {
    let mut story = story();
    send(&mut story, HELLO_AI_OFF);
    let mut bridge = FakeBridge::new(story);

    let reply = bridge.batch(&format!("{CHARACTER}\n{}", r#"{"type":"journal_asked"}"#));

    let Reply::Done(text) = reply else {
        panic!("{reply:?}")
    };
    assert!(text.contains(r#""ai":"off""#), "{text}");
}

#[test]
fn a_journal_that_fails_still_says_ai_is_off() {
    let mut story = story();
    send(&mut story, HELLO_AI_OFF);

    // With no character yet, the request fails, and it gets the empty journal.
    let served = send(&mut story, r#"{"type":"journal_asked","id":9}"#);

    assert!(served.error.is_some());
    let empty: Value = serde_json::from_str(&served.lines[0]).unwrap();
    assert_eq!(empty["ai"], json!("off"));
}

#[test]
fn a_hello_with_an_unknown_ai_word_keeps_ai_on_and_gets_its_reply() {
    for word in [r#""local""#, r#""Off""#, "null", "1"] {
        let mut story = story();
        let hello = format!(r#"{{"type":"hello","protocol":1,"app":"timeways","ai":{word}}}"#);

        let served = send(&mut story, &hello);

        assert_eq!(
            served.lines,
            vec![r#"{"type":"hello","protocol":1}"#],
            "{word}"
        );
        assert_eq!(served.notes.len(), 1, "{word}");
        assert_eq!(story.ai(), Ai::On, "{word}");
    }
}

#[test]
fn only_ai_off_in_the_settings_turns_ai_off() {
    assert_eq!(Ai::of_settings("ai = \"off\"\n"), Ai::Off);
    for text in [
        "",
        "ai = \"on\"",
        "ai = false",
        "ai = \"of\"",
        "[x]\nai = \"off\"",
    ] {
        assert_eq!(Ai::of_settings(text), Ai::On, "{text}");
    }
}

#[test]
fn ai_off_in_the_settings_keeps_dev_mode_on() {
    let text = "dev = true\nai = \"off\"\n";

    assert_eq!(DevMode::of_settings(text), DevMode::On);
    assert_eq!(Ai::of_settings(text), Ai::Off);
}

#[test]
fn the_settings_file_in_the_story_folder_turns_ai_off() {
    let folder = folder("settings");
    std::fs::write(folder.join(SETTINGS_FILE), "ai = \"off\"\n").unwrap();

    assert_eq!(Ai::of_folder(&folder), Ai::Off);
    assert_eq!(Ai::of_folder(&folder.join("missing")), Ai::On);
}

#[test]
fn the_dev_switch_turns_ai_off_only_in_dev_mode() {
    let mut off_in_dev = story();
    off_in_dev.set_dev_mode(DevMode::On);
    off_in_dev.set_dev_ai(Ai::Off);
    let mut off_out_of_dev = story();
    off_out_of_dev.set_dev_ai(Ai::Off);

    assert_eq!(off_in_dev.ai(), Ai::Off);
    assert_eq!(off_out_of_dev.ai(), Ai::On);
}

#[test]
fn the_dev_switch_never_turns_ai_on() {
    let mut story = story();
    story.set_dev_mode(DevMode::On);
    send(&mut story, HELLO_AI_OFF);

    story.set_dev_ai(Ai::On);

    assert_eq!(story.ai(), Ai::Off);
}

// With AI off, each feature takes the path that it takes with no model, and asks none.

fn passage(text: &str, links: Vec<Link>, about: Option<&str>) -> Passage {
    Passage {
        text: text.to_string(),
        source: "https://example.test/1".to_string(),
        links,
        origin: Origin::Pack,
        about: about.map(str::to_string),
        depends_on: Vec::new(),
        setup_for: None,
    }
}

fn tower() -> Passage {
    let testvale = Link::Place("Testvale".to_string());
    passage("The tower of Testvale fell.", vec![testvale], None)
}

fn stormwind() -> Passage {
    let text =
        "King Barathen Wrynn scattered the gnolls of Elwynn, and his line rules Stormwind City.";
    passage(text, vec![Link::Common], Some("Stormwind City"))
}

/// A human paladin with the lore of Testvale and Stormwind, and AI as `ai`.
fn story_with_lore(name: &str, ai: AiWord) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ai-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[tower(), stormwind()]).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    story.handle(Input::Hello { ai }).unwrap();
    story
        .handle(Input::CharacterEntered {
            realm: "Testrealm".to_string(),
            name: "Tester".to_string(),
        })
        .unwrap();
    let described = Input::CharacterDescribed {
        at: Tick(1),
        race: Race::Human,
        class: Class::Paladin,
    };
    story.handle(described).unwrap();
    story
}

fn enter(story: &mut Story, at: u64, zone: &str) -> Vec<Output> {
    let input = Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: None,
        spot: None,
        hour: None,
        taxi: None,
    };
    story.handle(input).unwrap()
}

fn level(story: &mut Story, at: u64, level: u8) {
    story
        .handle(Input::LevelReached {
            at: Tick(at),
            level,
        })
        .unwrap();
}

fn batch_end(story: &mut Story, id: u64) -> Vec<Output> {
    story.handle(Input::BatchEnd { id: MessageId(id) }).unwrap()
}

fn quiet(id: u64) -> Output {
    Output::EventsSeen {
        id: MessageId(id),
        narrator: None,
        narrator_id: None,
        notice: None,
    }
}

fn has_model_call(outputs: &[Output]) -> bool {
    outputs
        .iter()
        .any(|output| matches!(output, Output::ModelCall { .. }))
}

#[test]
fn with_ai_off_a_lore_question_gets_its_passages_at_once() {
    let mut story = story_with_lore("lore", AiWord::Off);
    enter(&mut story, 10, "Testvale");

    let outputs = story
        .handle(Input::LoreAsked {
            id: MessageId(7),
            question: "Why did the tower fall?".to_string(),
            target: None,
        })
        .unwrap();

    let [Output::LoreAnswer { answer, .. }] = outputs.as_slice() else {
        panic!("{outputs:?}")
    };
    assert_eq!(answer.text, None);
    assert_eq!(answer.passages[0].text, "The tower of Testvale fell.");
}

#[test]
fn with_ai_off_a_talk_gets_no_words_and_still_meets_the_npc() {
    let mut story = story_with_lore("talk", AiWord::Off);

    let outputs = story
        .handle(Input::TalkAsked {
            id: MessageId(8),
            at: Tick(50),
            npc: "Innkeeper Farley".to_string(),
            text: "Any news?".to_string(),
        })
        .unwrap();

    let silent = Output::TalkAnswer {
        id: MessageId(8),
        npc: "Innkeeper Farley".to_string(),
        text: None,
        notice: None,
    };
    assert_eq!(outputs, vec![silent]);
    let journal = journal_page(&mut story);
    assert_eq!(journal["people"][0]["name"], json!("Innkeeper Farley"));
}

#[test]
fn with_ai_off_a_quest_request_asks_no_model() {
    let mut story = story_with_lore("quest", AiWord::Off);
    story
        .handle(Input::NpcMet {
            at: Tick(40),
            name: "Marshal Dughan".to_string(),
            spot: None,
        })
        .unwrap();
    story
        .handle(Input::QuestAsked {
            at: Tick(50),
            npc: "Marshal Dughan".to_string(),
        })
        .unwrap();

    let outputs = batch_end(&mut story, 3);

    assert_eq!(outputs, vec![quiet(3)]);
}

#[test]
fn with_ai_off_a_draft_request_gets_no_draft_at_once() {
    let mut story = story_with_lore("draft", AiWord::Off);

    let outputs = story
        .handle(Input::DraftAsked {
            id: MessageId(9),
            at: Tick(50),
            idea: "Bring me five linen cloths.".to_string(),
        })
        .unwrap();

    let empty = Output::DraftAnswer {
        id: MessageId(9),
        draft: None,
        notice: None,
    };
    assert_eq!(outputs, vec![empty]);
}

#[test]
fn with_ai_off_a_big_moment_gets_no_line() {
    let mut story = story_with_lore("moment", AiWord::Off);
    level(&mut story, 1, 19);
    level(&mut story, 2, 20);

    let outputs = batch_end(&mut story, 3);

    assert_eq!(outputs, vec![quiet(3)]);
}

#[test]
fn with_ai_off_the_narrator_spends_no_budget() {
    let mut story = story_with_lore("budget", AiWord::Off);
    for (batch, zone) in [
        (1, "Testvale"),
        (2, "Elwynn Forest"),
        (3, "Westfall"),
        (4, "Duskwood"),
    ] {
        enter(&mut story, batch * 10, zone);
        assert!(!has_model_call(&batch_end(&mut story, batch)));
    }

    story.handle(Input::Hello { ai: AiWord::On }).unwrap();
    enter(&mut story, 50, "Redridge Mountains");
    let outputs = batch_end(&mut story, 5);

    assert!(has_model_call(&outputs), "{outputs:?}");
}

#[test]
fn with_ai_off_the_moments_of_a_batch_never_wait_for_a_later_one() {
    let mut story = story_with_lore("moments-go", AiWord::Off);
    enter(&mut story, 10, "Testvale");
    batch_end(&mut story, 1);
    story.handle(Input::Hello { ai: AiWord::On }).unwrap();

    let outputs = batch_end(&mut story, 2);

    assert_eq!(outputs, vec![quiet(2)]);
}

fn journal_page(story: &mut Story) -> Value {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();
    serde_json::to_value(&outputs[0]).unwrap()
}

#[test]
fn a_bridge_with_ai_off_gets_no_model_call_for_a_question() {
    let story = story_with_lore("bridge", AiWord::On);
    let mut bridge = FakeBridge::new(story).with_ai_off();
    let question = r#"{"type":"lore_asked","at":10,"question":"Why did the tower fall?"}"#;

    let reply = bridge.batch(&format!("{CHARACTER}\n{question}"));

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
}

#[test]
fn with_ai_off_a_draft_with_no_character_fails_as_with_ai_on() {
    let ask = || Input::DraftAsked {
        id: MessageId(9),
        at: Tick(50),
        idea: "Bring me five linen cloths.".to_string(),
    };
    let mut on = story();
    let mut off = story();
    off.handle(Input::Hello { ai: AiWord::Off }).unwrap();

    assert!(on.handle(ask()).is_err());
    assert!(off.handle(ask()).is_err());
}

// The addon says AI off with its own line. The bridge does not send `ai`.

const ADDON_AI_OFF: &str = r#"{"type":"ai_set","ai":"off","id":4}"#;
const ADDON_AI_ON: &str = r#"{"type":"ai_set","ai":"on","id":5}"#;

#[test]
fn ai_is_on_before_the_addon_says_anything() {
    assert_eq!(story().ai(), Ai::On);
}

#[test]
fn the_addon_turns_ai_off_and_on_again_with_its_line() {
    let mut story = story();

    let off = send(&mut story, ADDON_AI_OFF);
    let after_off = story.ai();
    let on = send(&mut story, ADDON_AI_ON);

    assert_eq!((off.lines.len(), off.error), (0, None));
    assert_eq!(after_off, Ai::Off);
    assert_eq!((on.lines.len(), on.error), (0, None));
    assert_eq!(story.ai(), Ai::On);
}

#[test]
fn the_journal_says_ai_is_off_after_the_addon_turned_it_off() {
    let mut story = story();

    send(&mut story, ADDON_AI_OFF);

    assert_eq!(journal(&mut story)["ai"], json!("off"));
}

#[test]
fn an_unknown_ai_word_of_the_addon_keeps_ai_on() {
    let mut story = story();

    let served = send(&mut story, r#"{"type":"ai_set","ai":"maybe","id":4}"#);

    assert_eq!(served.error, None);
    assert_eq!(served.notes.len(), 1);
    assert_eq!(story.ai(), Ai::On);
}

#[test]
fn off_from_any_source_wins() {
    let mut hello_off = story();
    send(&mut hello_off, HELLO_AI_OFF);
    send(&mut hello_off, ADDON_AI_ON);
    let mut dev_off = story();
    dev_off.set_dev_mode(DevMode::On);
    dev_off.set_dev_ai(Ai::Off);
    send(&mut dev_off, ADDON_AI_ON);
    let mut addon_off = story();
    let hello_on = r#"{"type":"hello","protocol":1,"app":"timeways","ai":"on"}"#;
    send(&mut addon_off, hello_on);
    send(&mut addon_off, ADDON_AI_OFF);

    assert_eq!(hello_off.ai(), Ai::Off);
    assert_eq!(dev_off.ai(), Ai::Off);
    assert_eq!(addon_off.ai(), Ai::Off);
}

/// A batch whose big moment asks the narrator while AI is on.
fn level_twenty_batch(ai_line: &str) -> String {
    let events = [
        r#"{"type":"character_described","at":1,"race":"Human","class":"PALADIN"}"#,
        r#"{"type":"zone_entered","at":10,"zone":"Testvale","subzone":null}"#,
        r#"{"type":"level_reached","at":20,"level":19}"#,
        r#"{"type":"level_reached","at":30,"level":20}"#,
    ];
    format!("{CHARACTER}\n{ai_line}{}", events.join("\n"))
}

fn model_calls_of(batch: &str, name: &str) -> usize {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = calls.clone();
    let story = story_with_lore(name, AiWord::On);
    let mut bridge = FakeBridge::new(story).with_model(Box::new(move |_| {
        counted.set(counted.get() + 1);
        None
    }));

    let reply = bridge.batch(batch);

    assert!(matches!(reply, Reply::Done(_)), "{reply:?}");
    calls.get()
}

#[test]
fn the_first_batch_after_login_opens_no_model_call_when_the_addon_says_off() {
    let with_ai_on = model_calls_of(&level_twenty_batch(""), "first-batch-on");
    let ai_off = "{\"type\":\"ai_set\",\"ai\":\"off\"}\n";
    let with_ai_off = model_calls_of(&level_twenty_batch(ai_off), "first-batch-off");

    assert!(with_ai_on > 0);
    assert_eq!(with_ai_off, 0);
}
