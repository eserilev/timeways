//! Side quests through the story program: the offer, the answer, and the progress
//! (GAMEPLAY.md 3.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::character::QUEST_TRUST;
use timeways_story::input::{CallId, Input, MessageId};
use timeways_story::journal::{Deed, Page};
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
    ask_from(story, GIVER, at)
}

/// `/quest` to this giver, and the end of its batch.
fn ask_from(story: &mut Story, giver: &str, at: u64) -> Output {
    let asked = Input::QuestAsked {
        at: Tick(at),
        npc: giver.to_string(),
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
    story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();

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
fn a_finished_quest_is_a_deed_with_its_title() {
    let mut story = story("deed");
    offer(&mut story, 5);
    story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();

    story.handle(zone(8, "Mill Pond")).unwrap();
    story.handle(meet(9, "Farmer Bram")).unwrap();

    let deeds = page(&mut story).journal.deeds;
    assert_eq!(
        deeds.last(),
        Some(&Deed::QuestDone {
            title: "The Lost Lantern".to_string(),
            at: Tick(9),
            place: Some("Mill Pond".to_string()),
        })
    );
}

#[test]
fn a_step_that_holds_at_accept_is_done_at_once() {
    let mut story = story("at-once");
    offer(&mut story, 5);
    story.handle(zone(6, "Mill Pond")).unwrap();

    story
        .handle(Input::QuestAccepted {
            at: Tick(7),
            number: None,
        })
        .unwrap();

    assert_eq!(quests(&mut story)[0].steps_done, 1);
}

#[test]
fn a_talk_counts_as_meeting_the_npc_of_a_step() {
    let mut story = story("talk");
    offer(&mut story, 5);
    story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();
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

    story
        .handle(Input::QuestDeclined {
            at: Tick(6),
            number: None,
        })
        .unwrap();

    assert!(quests(&mut story).is_empty());
}

#[test]
fn an_answer_with_no_offer_waiting_changes_nothing() {
    let mut story = story("no-offer");

    let outputs = story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();

    assert_eq!(outputs, []);
    assert!(quests(&mut story).is_empty());
}

#[test]
fn a_giver_waits_for_you_to_finish_its_open_quest() {
    let mut story = story("waits");
    offer(&mut story, 5);
    story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();

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

fn answer_with(story: &mut Story, call: CallId, title: &str) -> Option<String> {
    let text = OFFER.replace("The Lost Lantern", title);
    narrator(story.handle(Input::ModelAnswered { call, text }).unwrap())
}

fn accept(story: &mut Story, at: u64, number: Option<u64>) -> Vec<Output> {
    let accepted = Input::QuestAccepted {
        at: Tick(at),
        number,
    };
    story.handle(accepted).unwrap()
}

fn open_quests(story: &mut Story) -> usize {
    let quests = quests(story);
    quests
        .iter()
        .filter(|q| q.status == Status::Accepted)
        .count()
}

#[test]
fn two_offers_of_one_giver_never_make_two_open_quests() {
    let mut story = story("two-offers");
    let (first, _) = call_of(ask(&mut story, 5));
    let (second, _) = call_of(ask(&mut story, 6));
    answer_with(&mut story, first, "The Lost Lantern");
    accept(&mut story, 7, None);

    let line = answer_with(&mut story, second, "The Old Well");
    accept(&mut story, 8, None);

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa waits for you to finish \"The Lost Lantern\".")
    );
    assert_eq!(open_quests(&mut story), 1);
}

#[test]
fn an_accept_past_three_open_quests_is_refused_in_the_narrator_line() {
    let mut story = story("full-log");
    let givers = ["Keeper Tessa", "Innkeeper Pell", "Guard Rolf", "Smith Hana"];
    for (n, giver) in (0u64..).zip(givers) {
        story.handle(meet(10 + n, giver)).unwrap();
        let (call, _) = call_of(ask_from(&mut story, giver, 20 + n));
        answer_with(&mut story, call, &format!("Task {n}"));
    }

    for n in 1..=4 {
        accept(&mut story, 30 + n, Some(n));
    }
    let batch = story.handle(Input::BatchEnd { id: BATCH }).unwrap();

    assert_eq!(open_quests(&mut story), 3);
    assert_eq!(
        narrator(batch).as_deref(),
        Some("Your quest log is full. Finish a quest first.")
    );
}

#[test]
fn an_accept_with_a_number_takes_that_offer_and_not_the_newest() {
    let mut story = story("by-number");
    story.handle(meet(10, "Innkeeper Pell")).unwrap();
    offer(&mut story, 11);
    let (call, _) = call_of(ask_from(&mut story, "Innkeeper Pell", 12));
    answer_with(&mut story, call, "The Old Well");

    accept(&mut story, 13, Some(1));

    let quests = quests(&mut story);
    assert_eq!(quests[0].status, Status::Accepted);
    assert_eq!(quests[1].status, Status::Offered);
}

#[test]
fn a_new_offer_ends_only_the_waiting_offer_of_the_same_giver() {
    let mut story = story("same-giver");
    story.handle(meet(10, "Innkeeper Pell")).unwrap();
    offer(&mut story, 11);
    let (call, _) = call_of(ask_from(&mut story, "Innkeeper Pell", 12));
    answer_with(&mut story, call, "The Old Well");

    offer(&mut story, 13);

    let waiting: Vec<u64> = quests(&mut story)
        .iter()
        .filter(|q| q.status == Status::Offered)
        .map(|q| q.number)
        .collect();
    assert_eq!(waiting, [2, 3]);
}

#[test]
fn a_slap_counts_as_meeting_the_npc_of_a_step() {
    let mut story = story("slap");
    offer(&mut story, 5);
    accept(&mut story, 6, None);
    story.handle(zone(7, "Mill Pond")).unwrap();

    let slap = Input::NpcSlapped {
        at: Tick(8),
        name: "Farmer Bram".to_string(),
    };
    story.handle(slap).unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn an_offer_that_sends_you_to_the_npc_of_a_game_quest_that_you_read_shows_no_task() {
    let mut story = story("game-quest");
    let read = Input::TextSeen {
        at: Tick(5),
        kind: timeways_story::seen::TextKind::Quest,
        title: Some("Wheat for the Mill".to_string()),
        npc: None,
        zone: Some("Testvale".to_string()),
        text: "Speak with Farmer Bram about his wheat.".to_string(),
    };
    story.handle(read).unwrap();

    let line = offer(&mut story, 6);

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no task for you now.")
    );
}

fn abandon(story: &mut Story, at: u64, number: u64) {
    let abandoned = Input::QuestAbandoned {
        at: Tick(at),
        number,
    };
    assert_eq!(story.handle(abandoned).unwrap(), []);
}

#[test]
fn an_abandoned_quest_leaves_the_book_and_frees_its_giver() {
    let mut story = story("abandon");
    offer(&mut story, 5);
    accept(&mut story, 6, None);

    abandon(&mut story, 7, 1);

    assert!(quests(&mut story).is_empty());
    let (_, prompt) = call_of(ask(&mut story, 8));
    assert!(prompt.contains("You are Keeper Tessa"), "{prompt}");
}

#[test]
fn a_waiting_offer_can_be_abandoned_too() {
    let mut story = story("abandon-offer");
    offer(&mut story, 5);

    abandon(&mut story, 6, 1);

    assert!(quests(&mut story).is_empty());
}

#[test]
fn abandoning_a_finished_quest_or_an_unknown_number_changes_nothing() {
    let mut story = story("abandon-nothing");
    offer(&mut story, 5);
    accept(&mut story, 6, None);
    story.handle(zone(7, "Mill Pond")).unwrap();
    story.handle(meet(8, "Farmer Bram")).unwrap();

    abandon(&mut story, 9, 1);
    abandon(&mut story, 10, 42);

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}
