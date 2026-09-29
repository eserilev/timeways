//! Side quests through the story program: the offer, the answer, and the progress
//! (GAMEPLAY.md 3.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::character::QUEST_TRUST;
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::journal::Page;
use timeways_story::pack::Pack;
use timeways_story::quest::{Status, Tracked};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

const GIVER: &str = "Keeper Tessa";
const BATCH: MessageId = MessageId(5);

const OFFER: &str = r#"{"title": "The Lost Lantern", "text": "Find the lantern.",
    "steps": [{"goal": "visit", "place": "Mill Pond"}, {"goal": "meet", "npc": "Farmer Bram"}]}"#;

fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("quests-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    let inputs = [
        Input::CharacterEntered {
            realm: "Testrealm".to_string(),
            name: "Tester".to_string(),
        },
        zone(1, "Mill Pond"),
        meet(2, "Farmer Bram"),
        zone(3, "Old Tower"),
        meet(4, GIVER),
    ];
    for input in inputs {
        story.handle(input).unwrap();
    }
    story
}

fn zone(at: u64, subzone: &str) -> Input {
    Input::ZoneEntered {
        at: Tick(at),
        zone: "Testvale".to_string(),
        subzone: Some(subzone.to_string()),
    }
}

fn meet(at: u64, name: &str) -> Input {
    Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
    }
}

/// `/quest` to the giver, and the end of its batch.
fn ask(story: &mut Story, at: u64) -> Output {
    let asked = Input::QuestAsked {
        at: Tick(at),
        npc: GIVER.to_string(),
    };
    assert_eq!(story.handle(asked).unwrap(), []);
    story
        .handle(Input::BatchEnd { id: BATCH })
        .unwrap()
        .remove(0)
}

fn call_of(output: Output) -> (CallId, String) {
    match output {
        Output::ModelCall { call, prompt } => (call, prompt),
        other => panic!("expected a model call, got {other:?}"),
    }
}

fn narrator(outputs: Vec<Output>) -> Option<String> {
    match outputs.into_iter().next() {
        Some(Output::EventsSeen { id, narrator }) => {
            assert_eq!(id, BATCH);
            narrator
        }
        other => panic!("expected events_seen, got {other:?}"),
    }
}

fn offer(story: &mut Story, at: u64) -> Option<String> {
    let (call, _) = call_of(ask(story, at));
    let answer = Input::ModelAnswered {
        call,
        text: OFFER.to_string(),
    };
    narrator(story.handle(answer).unwrap())
}

fn page(story: &mut Story) -> Page {
    match story
        .handle(Input::JournalAsked {
            id: MessageId(9),
            page: 0,
        })
        .unwrap()
        .remove(0)
    {
        Output::Journal { page, .. } => page,
        other => panic!("expected a journal page, got {other:?}"),
    }
}

fn quests(story: &mut Story) -> Vec<Tracked> {
    page(story).journal.quests
}

fn trust_of_giver(story: &mut Story) -> Option<i64> {
    let people = page(story).journal.people;
    people
        .into_iter()
        .find(|person| person.name == GIVER)?
        .trust
}

#[test]
fn a_quest_request_takes_the_place_of_the_narrator_call() {
    let mut story = story("request");

    let (_, prompt) = call_of(ask(&mut story, 5));

    assert!(prompt.contains("You are Keeper Tessa"), "{prompt}");
    assert!(prompt.contains("- Farmer Bram"), "{prompt}");
    assert!(prompt.contains("- Mill Pond"), "{prompt}");
}

#[test]
fn a_checked_offer_comes_back_as_the_narrator_line() {
    let mut story = story("offer");

    let line = offer(&mut story, 5);

    assert_eq!(
        line.as_deref(),
        Some(
            "Keeper Tessa has a task for you: The Lost Lantern. Find the lantern. Type /quest accept."
        )
    );
    let offered = quests(&mut story);
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].status, Status::Offered);
}

#[test]
fn with_no_model_the_giver_has_no_task() {
    let mut story = story("no-model");
    let (call, _) = call_of(ask(&mut story, 5));

    let line = narrator(story.handle(Input::ModelFailed { call }).unwrap());

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no task for you now.")
    );
    assert!(quests(&mut story).is_empty());
}

#[test]
fn an_offer_that_breaks_a_rule_shows_no_task_and_stays_out_of_the_log() {
    let mut story = story("broken");
    let (call, _) = call_of(ask(&mut story, 5));
    let text = OFFER.replace("Farmer Bram", "Captain Vorn");

    let line = narrator(story.handle(Input::ModelAnswered { call, text }).unwrap());

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no task for you now.")
    );
    assert!(quests(&mut story).is_empty());
}

#[test]
fn a_quest_ends_when_its_steps_happen_in_order_and_the_giver_trusts_you_more() {
    let mut story = story("done");
    offer(&mut story, 5);
    story.handle(Input::QuestAccepted { at: Tick(6) }).unwrap();

    story.handle(meet(7, "Farmer Bram")).unwrap();
    assert_eq!(quests(&mut story)[0].steps_done, 0);
    story.handle(zone(8, "Mill Pond")).unwrap();
    story.handle(meet(9, "Farmer Bram")).unwrap();

    let done = quests(&mut story).remove(0);
    assert_eq!(done.status, Status::Done);
    assert_eq!(done.done_at, Some(Tick(9)));
    assert_eq!(trust_of_giver(&mut story), Some(QUEST_TRUST));
}

#[test]
fn a_step_that_holds_at_accept_is_done_at_once() {
    let mut story = story("at-once");
    offer(&mut story, 5);
    story.handle(zone(6, "Mill Pond")).unwrap();

    story.handle(Input::QuestAccepted { at: Tick(7) }).unwrap();

    assert_eq!(quests(&mut story)[0].steps_done, 1);
}

#[test]
fn a_talk_counts_as_meeting_the_npc_of_a_step() {
    let mut story = story("talk");
    offer(&mut story, 5);
    story.handle(Input::QuestAccepted { at: Tick(6) }).unwrap();
    story.handle(zone(7, "Mill Pond")).unwrap();

    let talk = Input::TalkAsked {
        id: MessageId(8),
        at: Tick(8),
        npc: "Farmer Bram".to_string(),
        text: "Hello there.".to_string(),
    };
    story.handle(talk).unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_declined_offer_leaves_the_journal() {
    let mut story = story("declined");
    offer(&mut story, 5);

    story.handle(Input::QuestDeclined { at: Tick(6) }).unwrap();

    assert!(quests(&mut story).is_empty());
}

#[test]
fn an_answer_with_no_offer_waiting_changes_nothing() {
    let mut story = story("no-offer");

    let outputs = story.handle(Input::QuestAccepted { at: Tick(6) }).unwrap();

    assert_eq!(outputs, []);
    assert!(quests(&mut story).is_empty());
}

#[test]
fn a_giver_waits_for_you_to_finish_its_open_quest() {
    let mut story = story("waits");
    offer(&mut story, 5);
    story.handle(Input::QuestAccepted { at: Tick(6) }).unwrap();

    let output = ask(&mut story, 7);

    assert_eq!(
        output,
        Output::EventsSeen {
            id: BATCH,
            narrator: Some(
                "Keeper Tessa waits for you to finish \"The Lost Lantern\".".to_string()
            ),
        }
    );
}

#[test]
fn a_quest_request_meets_its_giver() {
    let mut story = story("meets");
    let asked = Input::QuestAsked {
        at: Tick(5),
        npc: "Innkeeper Pell".to_string(),
    };

    story.handle(asked).unwrap();

    let people = page(&mut story).journal.people;
    assert!(people.iter().any(|person| person.name == "Innkeeper Pell"));
}
