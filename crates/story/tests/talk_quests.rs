//! Work in a talk becomes a real quest (GAMEPLAY.md 3.4 and 3.5): the talk asks the NPC
//! for a quest, as `/quest` does, and the journal tells the talk window how it goes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::input::{CallId, Input, MessageId, Reaction};
use timeways_story::journal::{Page, TalkQuest, TalkQuestState};
use timeways_story::pack::Pack;
use timeways_story::quest::{QuestView, Status};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const GIVER: &str = "Keeper Tessa";
const BATCH: MessageId = MessageId(5);
const TALK: MessageId = MessageId(8);

const WORK: &str = r#"{"say": "Something is wrong at the pond.", "trust": 1, "work": true}"#;
const NO_WORK: &str = r#"{"say": "Nothing but rain.", "trust": 1}"#;

const OFFER: &str = r#"{"title": "The Lost Lantern", "genre": "errand", "text": "I lost my lantern. Find it.",
    "steps": [{"goal": "visit", "place": "Mill Pond"}, {"goal": "meet", "npc": "Farmer Bram"}]}"#;

/// A world where you visited Mill Pond and Old Tower, and met Farmer Bram and the giver.
fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("talk-quests-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    story.handle(enter("Tester")).unwrap();
    for input in [
        zone(1, "Mill Pond"),
        meet(2, "Farmer Bram"),
        zone(3, "Old Tower"),
        meet(4, GIVER),
    ] {
        story.handle(input).unwrap();
    }
    story
}

fn enter(name: &str) -> Input {
    Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: name.to_string(),
    }
}

fn zone(at: u64, subzone: &str) -> Input {
    Input::ZoneEntered {
        at: Tick(at),
        zone: "Testvale".to_string(),
        subzone: Some(subzone.to_string()),
        spot: None,
        hour: None,
        taxi: None,
    }
}

fn meet(at: u64, name: &str) -> Input {
    Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
        spot: None,
    }
}

fn calls(outputs: &[Output]) -> Vec<(CallId, String)> {
    outputs
        .iter()
        .filter_map(|output| match output {
            Output::ModelCall { call, prompt } => Some((*call, prompt.clone())),
            _ => None,
        })
        .collect()
}

/// `/talk` to this NPC: the call of the talk.
fn ask_talk(story: &mut Story, at: u64, npc: &str) -> CallId {
    let asked = Input::TalkAsked {
        id: TALK,
        at: Tick(at),
        npc: npc.to_string(),
        text: "Any work for me?".to_string(),
    };
    calls(&story.handle(asked).unwrap())[0].0
}

/// A talk to the giver whose answer is `answer`: the outputs of the answer.
fn talk(story: &mut Story, at: u64, answer: &str) -> Vec<Output> {
    let call = ask_talk(story, at, GIVER);
    answered(story, call, answer)
}

fn answered(story: &mut Story, call: CallId, text: &str) -> Vec<Output> {
    let text = text.to_string();
    story.handle(Input::ModelAnswered { call, text }).unwrap()
}

/// A talk with work, and the call of the quest that it asks for.
fn talk_with_work(story: &mut Story, at: u64) -> (CallId, String) {
    calls(&talk(story, at, WORK)).remove(0)
}

fn page(story: &mut Story) -> Page {
    let asked = Input::JournalAsked {
        id: MessageId(9),
        page: 0,
    };
    match story.handle(asked).unwrap().remove(0) {
        Output::Journal { page, .. } => *page,
        other => panic!("expected a journal page, got {other:?}"),
    }
}

fn talk_quest(story: &mut Story) -> Option<TalkQuest> {
    page(story).journal.talk_quest.map(|quest| *quest)
}

fn state(story: &mut Story) -> Option<TalkQuestState> {
    talk_quest(story).map(|quest| quest.state)
}

fn quests(story: &mut Story) -> Vec<QuestView> {
    page(story).journal.quests
}

fn refused(line: &str) -> TalkQuestState {
    TalkQuestState::Refused {
        line: line.to_string(),
    }
}

/// `/quest` to the giver, and the end of its batch.
fn ask_quest(story: &mut Story, at: u64) -> Vec<Output> {
    let asked = Input::QuestAsked {
        at: Tick(at),
        npc: GIVER.to_string(),
    };
    story.handle(asked).unwrap();
    story.handle(Input::BatchEnd { id: BATCH }).unwrap()
}

/// An offer from this giver with these steps, accepted.
fn accepted_quest(story: &mut Story, giver: &str, title: &str, steps: &str) {
    let at = 5;
    let asked = Input::QuestAsked {
        at: Tick(at),
        npc: giver.to_string(),
    };
    story.handle(asked).unwrap();
    let outputs = story.handle(Input::BatchEnd { id: BATCH }).unwrap();
    let (call, _) = calls(&outputs).remove(0);
    let text = format!(
        r#"{{"title": "{title}", "genre": "errand", "text": "I need your help.", "steps": {steps}}}"#
    );
    answered(story, call, &text);
    let accept = Input::QuestAccepted {
        at: Tick(at),
        number: None,
    };
    story.handle(accept).unwrap();
}

#[test]
fn a_talk_with_work_answers_first_and_then_asks_the_npc_for_a_quest() {
    let mut story = story("asks");

    let outputs = talk(&mut story, 10, WORK);

    let answer = Output::TalkAnswer {
        id: TALK,
        npc: GIVER.to_string(),
        text: Some("Something is wrong at the pond.".to_string()),
        notice: None,
    };
    assert_eq!(outputs[0], answer);
    let (_, prompt) = calls(&outputs).remove(0);
    assert!(prompt.contains("Name: Keeper Tessa"), "{prompt}");
    assert!(prompt.contains("- Mill Pond"), "{prompt}");
    assert!(
        prompt.contains(
            "You just said this to the player:\n<<<\nSomething is wrong at the pond.\n>>>"
        ),
        "{prompt}"
    );
}

#[test]
fn a_talk_with_no_work_asks_for_no_quest() {
    let mut story = story("no-work");

    let outputs = talk(&mut story, 10, NO_WORK);

    assert!(calls(&outputs).is_empty());
    assert_eq!(talk_quest(&mut story), None);
}

#[test]
fn the_journal_says_that_the_quest_of_a_talk_is_being_written() {
    let mut story = story("writing");

    talk_with_work(&mut story, 10);

    let expected = TalkQuest {
        npc: GIVER.to_string(),
        at: Tick(10),
        state: TalkQuestState::Writing,
    };
    assert_eq!(talk_quest(&mut story), Some(expected));
}

#[test]
fn the_offer_of_a_talk_is_a_real_quest_with_checked_steps() {
    let mut story = story("offer");
    let (call, _) = talk_with_work(&mut story, 10);

    answered(&mut story, call, OFFER);

    let offered = quests(&mut story);
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].giver, GIVER);
    assert_eq!(offered[0].status, Status::Offered);
    assert_eq!(offered[0].steps.len(), 2);
    let number = offered[0].number;
    assert_eq!(state(&mut story), Some(TalkQuestState::Offered { number }));
}

#[test]
fn the_offer_of_a_talk_sends_no_notice_because_the_window_shows_it() {
    let mut story = story("no-notice");
    let (call, _) = talk_with_work(&mut story, 10);

    let outputs = answered(&mut story, call, OFFER);

    assert_eq!(outputs, []);
    let Output::Journal { notice, .. } = story
        .handle(Input::JournalAsked {
            id: MessageId(9),
            page: 0,
        })
        .unwrap()
        .remove(0)
    else {
        panic!("expected a journal page");
    };
    assert_eq!(notice, None);
}

#[test]
fn a_talk_quest_that_breaks_the_check_twice_has_no_quest() {
    let mut story = story("broken");
    let (call, _) = talk_with_work(&mut story, 10);
    let bad = OFFER.replace("Farmer Bram", "Captain Vorn");

    let (retry, _) = calls(&answered(&mut story, call, &bad)).remove(0);
    assert_eq!(state(&mut story), Some(TalkQuestState::Writing));
    answered(&mut story, retry, &bad);

    assert_eq!(
        state(&mut story),
        Some(refused("Keeper Tessa has no quest for you now."))
    );
    assert!(quests(&mut story).is_empty());
}

#[test]
fn a_talk_quest_with_no_model_has_no_quest() {
    let mut story = story("no-model");
    let (call, _) = talk_with_work(&mut story, 10);

    let outputs = story.handle(Input::ModelFailed { call }).unwrap();

    assert_eq!(outputs, []);
    assert_eq!(
        state(&mut story),
        Some(refused("Keeper Tessa has no quest for you now."))
    );
}

#[test]
fn a_giver_with_an_open_quest_refuses_the_quest_of_a_talk_with_no_model_call() {
    let mut story = story("giver-busy");
    let steps = r#"[{"goal": "visit", "place": "Mill Pond"}]"#;
    accepted_quest(&mut story, GIVER, "The Lost Lantern", steps);

    let outputs = talk(&mut story, 10, WORK);

    assert!(calls(&outputs).is_empty());
    assert_eq!(
        state(&mut story),
        Some(refused(
            "Keeper Tessa is waiting for you to finish \"The Lost Lantern\"."
        ))
    );
}

#[test]
fn three_open_quests_refuse_the_quest_of_a_talk_and_the_words_stay() {
    let mut story = story("three-open");
    let offers = [
        (
            "Giver One",
            "Lost Lantern",
            r#"[{"goal": "visit", "place": "Mill Pond"}]"#,
        ),
        (
            "Giver Two",
            "Muddy Boots",
            r#"[{"goal": "meet", "npc": "Farmer Bram"}]"#,
        ),
        (
            "Giver Three",
            "Broken Bell",
            r#"[{"goal": "visit", "place": "Old Tower"}, {"goal": "visit", "place": "Mill Pond"}]"#,
        ),
    ];
    for (giver, title, steps) in offers {
        story.handle(meet(5, giver)).unwrap();
        accepted_quest(&mut story, giver, title, steps);
    }

    let outputs = talk(&mut story, 10, WORK);

    assert!(calls(&outputs).is_empty());
    assert!(matches!(
        &outputs[0],
        Output::TalkAnswer { text: Some(_), .. }
    ));
    assert_eq!(
        state(&mut story),
        Some(refused("You already have 3 quests. Finish one first."))
    );
}

#[test]
fn an_animal_has_no_quest_for_a_talk() {
    let mut story = story("animal");
    let seen = Input::NpcSeen {
        at: Tick(6),
        name: "Old Bessie".to_string(),
        reaction: Reaction::Friendly,
        creature: Some("beast".to_string()),
    };
    story.handle(seen).unwrap();
    let call = ask_talk(&mut story, 10, "Old Bessie");

    let outputs = answered(&mut story, call, WORK);

    assert!(calls(&outputs).is_empty());
    assert_eq!(
        state(&mut story),
        Some(refused("Old Bessie has no quest for you now."))
    );
}

#[test]
fn a_hostile_npc_has_no_quest_for_a_talk() {
    let mut story = story("hostile");
    let seen = Input::NpcSeen {
        at: Tick(6),
        name: "Grumpy Ned".to_string(),
        reaction: Reaction::Hostile,
        creature: Some("humanoid".to_string()),
    };
    story.handle(seen).unwrap();
    let call = ask_talk(&mut story, 10, "Grumpy Ned");

    let outputs = answered(&mut story, call, WORK);

    assert!(calls(&outputs).is_empty());
    assert_eq!(
        state(&mut story),
        Some(refused("Grumpy Ned has no quest for you now."))
    );
}

#[test]
fn a_talk_answer_for_another_character_asks_for_no_quest() {
    let mut story = story("other-character");
    let call = ask_talk(&mut story, 10, GIVER);
    story.handle(enter("Other")).unwrap();

    let outputs = answered(&mut story, call, WORK);

    assert!(calls(&outputs).is_empty());
    assert_eq!(talk_quest(&mut story), None);
}

#[test]
fn a_quest_command_while_a_talk_quest_is_written_asks_no_second_model() {
    let mut story = story("command-during-talk");
    let (call, _) = talk_with_work(&mut story, 10);

    let outputs = ask_quest(&mut story, 11);

    let quiet = Output::EventsSeen {
        id: BATCH,
        narrator: None,
        notice: None,
    };
    assert_eq!(outputs, [quiet]);
    answered(&mut story, call, OFFER);
    assert_eq!(quests(&mut story).len(), 1);
    assert!(matches!(
        state(&mut story),
        Some(TalkQuestState::Offered { .. })
    ));
}

#[test]
fn a_talk_with_work_while_a_quest_command_is_written_shows_that_offer() {
    let mut story = story("talk-during-command");
    let (call, _) = calls(&ask_quest(&mut story, 9)).remove(0);

    let outputs = talk(&mut story, 10, WORK);

    assert!(calls(&outputs).is_empty());
    assert_eq!(state(&mut story), Some(TalkQuestState::Writing));
    let offered = answered(&mut story, call, OFFER);
    let Some(Output::EventsSeen {
        notice: Some(line), ..
    }) = offered.first()
    else {
        panic!("expected the notice of the command, got {offered:?}");
    };
    assert!(
        line.starts_with("Keeper Tessa has a quest for you"),
        "{line}"
    );
    assert!(matches!(
        state(&mut story),
        Some(TalkQuestState::Offered { .. })
    ));
}

#[test]
fn a_newer_talk_with_work_replaces_the_state_of_the_older_one() {
    let mut story = story("newer");
    let (call, _) = talk_with_work(&mut story, 10);
    answered(&mut story, call, OFFER);
    story.handle(meet(11, "Farmer Bram")).unwrap();

    let farmer = ask_talk(&mut story, 12, "Farmer Bram");
    answered(&mut story, farmer, WORK);

    let quest = talk_quest(&mut story).unwrap();
    assert_eq!(
        (quest.npc.as_str(), quest.at, quest.state),
        ("Farmer Bram", Tick(12), TalkQuestState::Writing)
    );
}
