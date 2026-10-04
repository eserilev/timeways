//! Side quests through the story program: the offer, the answer, and the progress
//! (GAMEPLAY.md 3.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use std::time::Duration;
use timeways_story::character::{QUEST_TRUST, SLAP_TRUST};
use timeways_story::input::{CallId, Input, MessageId, Reaction};
use timeways_story::journal::{Deed, Page};
use timeways_story::pack::Pack;
use timeways_story::quest::{AnyOrder, DAY_SECONDS, QuestView, Status, StepState, StepView};
use timeways_story::store::Store;
use timeways_story::story::why::{TrustCause, TrustWhy};
use timeways_story::story::{Output, Story};

const GIVER: &str = "Keeper Tessa";
const BATCH: MessageId = MessageId(5);

const OFFER: &str = r#"{"title": "The Lost Lantern", "genre": "errand", "text": "I lost my lantern. Find it.",
    "steps": [{"goal": "visit", "place": "Mill Pond"}, {"goal": "meet", "npc": "Farmer Bram"}]}"#;

fn story(name: &str) -> Story {
    story_in(name, Store::Memory)
}

/// The story program with an empty pack and this store, and the character of the tests.
fn opened(name: &str, files: Store) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("quests-{name}.sqlite"));
    if !path.exists() {
        Pack::write(&path, &[]).unwrap();
    }
    let mut story = Story::new(Pack::open(&path).unwrap(), files);
    let entered = Input::CharacterEntered {
        realm: "Testrealm".to_string(),
        name: "Tester".to_string(),
    };
    story.handle(entered).unwrap();
    story
}

/// A world where you visited Mill Pond and Old Tower, met Farmer Bram and the giver, and
/// stand in Old Tower.
fn story_in(name: &str, files: Store) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("quests-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let mut story = opened(name, files);
    let inputs = [
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
        spot: None,
        hour: None,
    }
}

fn meet(at: u64, name: &str) -> Input {
    Input::NpcMet {
        at: Tick(at),
        name: name.to_string(),
        spot: None,
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

/// The line of Timeways that answers a batch. The narrator says nothing about tasks.
fn notice(outputs: Vec<Output>) -> Option<String> {
    match outputs.into_iter().next() {
        Some(Output::EventsSeen {
            id,
            narrator: None,
            notice,
        }) => {
            assert_eq!(id, BATCH);
            notice
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
    notice(story.handle(answer).unwrap())
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

fn quests(story: &mut Story) -> Vec<QuestView> {
    page(story).journal.quests
}

fn steps_done(quest: &QuestView) -> usize {
    let done = |step: &&StepView| step.state == StepState::Done;
    quest.steps.iter().filter(done).count()
}

/// The kills so far of the first kill step of the first quest.
fn kills(story: &mut Story) -> Option<u8> {
    quests(story)[0].steps.iter().find_map(|step| step.kills)
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

    assert!(prompt.contains("Name: Keeper Tessa"), "{prompt}");
    assert!(prompt.contains("- Farmer Bram"), "{prompt}");
    assert!(prompt.contains("- Mill Pond"), "{prompt}");
}

#[test]
fn a_checked_offer_comes_back_as_a_notice_of_timeways() {
    let mut story = story("offer");

    let line = offer(&mut story, 5);

    assert_eq!(
        line.as_deref(),
        Some(
            "Keeper Tessa has a quest for you: The Lost Lantern. I lost my lantern. Find it. Type /quest accept."
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

    let line = notice(story.handle(Input::ModelFailed { call }).unwrap());

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no quest for you now.")
    );
    assert!(quests(&mut story).is_empty());
}

#[test]
fn an_offer_that_breaks_a_rule_shows_no_task_and_stays_out_of_the_log() {
    let mut story = story("broken");
    let (call, _) = call_of(ask(&mut story, 5));
    let text = OFFER.replace("Farmer Bram", "Captain Vorn");

    let line = refused_twice(&mut story, call, &text);

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no quest for you now.")
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
    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
    story.handle(zone(8, "Mill Pond")).unwrap();
    story.handle(meet(9, "Farmer Bram")).unwrap();

    let done = quests(&mut story).remove(0);
    assert_eq!(done.status, Status::Done);
    assert_eq!(done.done_at, Some(Tick(9)));
    assert_eq!(trust_of_giver(&mut story), Some(QUEST_TRUST));
}

#[test]
fn the_journal_shows_each_step_as_done_open_or_later() {
    let mut story = story("states");
    offer(&mut story, 5);
    accept(&mut story, 6, None);

    story.handle(zone(7, "Mill Pond")).unwrap();

    let states: Vec<StepState> = quests(&mut story)[0]
        .steps
        .iter()
        .map(|step| step.state)
        .collect();
    assert_eq!(states, [StepState::Done, StepState::Open]);
}

#[test]
fn an_offer_shows_its_steps_as_later() {
    let mut story = story("offer-states");

    offer(&mut story, 5);

    let quest = quests(&mut story).remove(0);
    assert!(
        quest
            .steps
            .iter()
            .all(|step| step.state == StepState::Later)
    );
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

    assert_eq!(steps_done(&quests(&mut story)[0]), 1);
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
            narrator: None,
            notice: Some(
                "Keeper Tessa is waiting for you to finish \"The Lost Lantern\".".to_string()
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

const VISIT_POND: &str = r#"{"goal": "visit", "place": "Mill Pond"}"#;
const MEET_BRAM: &str = r#"{"goal": "meet", "npc": "Farmer Bram"}"#;
const VISIT_TOWER: &str = r#"{"goal": "visit", "place": "Old Tower"}"#;

/// An offer of one step. Two tasks in a row never share a target, so the tests pick the
/// step.
fn answer_with(story: &mut Story, call: CallId, title: &str, step: &str) -> Option<String> {
    let text = answer_text(title, step);
    notice(story.handle(Input::ModelAnswered { call, text }).unwrap())
}

/// A model answer of an offer with these steps.
fn answer_text(title: &str, step: &str) -> String {
    format!(
        r#"{{"title": "{title}", "genre": "errand", "text": "I need you to go and look.", "steps": [{step}]}}"#
    )
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
    answer_with(&mut story, first, "The Lost Lantern", VISIT_POND);
    accept(&mut story, 7, None);

    let line = answer_with(&mut story, second, "The Old Well", MEET_BRAM);
    accept(&mut story, 8, None);

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa is waiting for you to finish \"The Lost Lantern\".")
    );
    assert_eq!(open_quests(&mut story), 1);
}

/// The steps of four quests in a row: no two share a shape or a target with the quest before.
fn four_shapes() -> [String; 4] {
    [
        VISIT_POND.to_string(),
        MEET_BRAM.to_string(),
        format!("{VISIT_TOWER}, {VISIT_POND}"),
        TALK_BRAM.to_string(),
    ]
}

#[test]
fn an_accept_past_three_open_quests_is_refused_in_a_notice() {
    let mut story = story("full-log");
    let givers = ["Keeper Tessa", "Innkeeper Pell", "Guard Rolf", "Smith Hana"];
    for ((n, giver), steps) in (0u64..).zip(givers).zip(four_shapes()) {
        story.handle(meet(10 + n, giver)).unwrap();
        let (call, _) = call_of(ask_from(&mut story, giver, 20 + n));
        answer_with(&mut story, call, &format!("Task {n}"), &steps);
    }

    for n in 1..=4 {
        accept(&mut story, 30 + n, Some(n));
    }
    let batch = story.handle(Input::BatchEnd { id: BATCH }).unwrap();

    assert_eq!(open_quests(&mut story), 3);
    assert_eq!(
        notice(batch).as_deref(),
        Some("You already have 3 quests. Finish one first.")
    );
}

#[test]
fn an_accept_with_a_number_takes_that_offer_and_not_the_newest() {
    let mut story = story("by-number");
    story.handle(meet(10, "Innkeeper Pell")).unwrap();
    offer(&mut story, 11);
    let (call, _) = call_of(ask_from(&mut story, "Innkeeper Pell", 12));
    answer_with(&mut story, call, "The Old Well", VISIT_TOWER);

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
    answer_with(&mut story, call, "The Old Well", VISIT_TOWER);

    let (call, _) = call_of(ask(&mut story, 13));
    answer_with(&mut story, call, "A Broken Wheel", TALK_BRAM);

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

    let (call, _) = call_of(ask(&mut story, 6));
    let line = refused_twice(&mut story, call, OFFER);

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no quest for you now.")
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
    assert!(prompt.contains("Name: Keeper Tessa"), "{prompt}");
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

fn sight(story: &mut Story, at: u64, name: &str, reaction: Reaction, creature: &str) {
    let seen = Input::NpcSeen {
        at: Tick(at),
        name: name.to_string(),
        reaction,
        creature: Some(creature.to_string()),
    };
    assert_eq!(story.handle(seen).unwrap(), []);
}

fn kill(story: &mut Story, at: u64, name: &str) {
    let killed = Input::NpcKilled {
        at: Tick(at),
        name: name.to_string(),
    };
    story.handle(killed).unwrap();
}

const KILL_BATS: &str = r#"{"goal": "kill", "creature": "Duskbat", "count": 2}"#;

/// A hunt of 2 Duskbats, accepted. The player saw the bats first.
fn bat_hunt(name: &str) -> Story {
    let mut story = story(name);
    sight(&mut story, 5, "Duskbat", Reaction::Hostile, "beast");
    let (call, _) = call_of(ask(&mut story, 6));
    answer_with(&mut story, call, "Bats in the Belfry", KILL_BATS);
    accept(&mut story, 7, None);
    story
}

#[test]
fn the_prompt_names_seen_foes_to_hunt_and_never_a_foe_to_meet() {
    let mut story = story("prompt-foes");
    sight(&mut story, 5, "Duskbat", Reaction::Hostile, "beast");
    sight(&mut story, 5, "Tanner Ilsa", Reaction::Friendly, "humanoid");

    let (_, prompt) = call_of(ask(&mut story, 6));

    let hunt = prompt
        .split("Creatures that the player can hunt:")
        .nth(1)
        .unwrap();
    let meet = prompt
        .split("People that the player can meet:")
        .nth(1)
        .unwrap();
    let meet = meet.split("Creatures").next().unwrap();
    assert!(hunt.starts_with("\n<<<\n- Duskbat\n>>>"), "{prompt}");
    assert!(meet.contains("- Tanner Ilsa"), "{prompt}");
    assert!(!meet.contains("Duskbat"), "{prompt}");
}

#[test]
fn a_hunt_ends_after_its_kills_and_the_giver_trusts_you_more() {
    let mut story = bat_hunt("hunt");

    kill(&mut story, 8, "Duskbat");
    assert_eq!(kills(&mut story), Some(1));
    kill(&mut story, 9, "Duskbat");

    let done = quests(&mut story).remove(0);
    assert_eq!(done.status, Status::Done);
    assert_eq!(trust_of_giver(&mut story), Some(QUEST_TRUST));
}

#[test]
fn a_kill_of_another_creature_counts_nothing() {
    let mut story = bat_hunt("other-kill");

    kill(&mut story, 8, "Mill Rat");

    assert_eq!(kills(&mut story), Some(0));
}

#[test]
fn a_kill_before_the_hunt_is_accepted_counts_nothing() {
    let mut story = story("early-kill");
    sight(&mut story, 5, "Duskbat", Reaction::Hostile, "beast");
    let (call, _) = call_of(ask(&mut story, 6));
    answer_with(&mut story, call, "Bats in the Belfry", KILL_BATS);

    kill(&mut story, 7, "Duskbat");
    accept(&mut story, 8, None);

    assert_eq!(kills(&mut story), Some(0));
}

#[test]
fn the_next_task_never_names_a_target_of_the_last_one() {
    let mut story = bat_hunt("last-task");
    kill(&mut story, 8, "Duskbat");
    kill(&mut story, 9, "Duskbat");
    story.handle(meet(10, "Innkeeper Pell")).unwrap();

    let (call, prompt) = call_of(ask_from(&mut story, "Innkeeper Pell", 11));
    let text = answer_text("More Bats", KILL_BATS);
    let line = refused_twice(&mut story, call, &text);

    assert!(!prompt.contains("- Duskbat"), "{prompt}");
    assert_eq!(
        line.as_deref(),
        Some("Innkeeper Pell has no quest for you now.")
    );
}

#[test]
fn a_task_finished_after_the_clock_went_back_still_earns_its_trust() {
    let mut story = bat_hunt("clock-back");
    kill(&mut story, 20, "Duskbat");

    kill(&mut story, 15, "Duskbat");

    let done = quests(&mut story).remove(0);
    assert_eq!(done.status, Status::Done);
    assert_eq!(trust_of_giver(&mut story), Some(QUEST_TRUST));
}

/// The lines of one list of the prompt, from its heading to the end of its fence.
fn prompt_list<'a>(prompt: &'a str, heading: &str) -> Vec<&'a str> {
    let list = prompt.split(heading).nth(1).unwrap();
    let list = list.split(">>>").next().unwrap();
    list.lines()
        .filter_map(|line| line.strip_prefix("- "))
        .collect()
}

#[test]
fn the_prompt_lists_the_newest_foes() {
    let mut story = story("newest-foes");
    for n in 1..=25 {
        sight(
            &mut story,
            10 + n,
            &format!("Duskbat {n}"),
            Reaction::Hostile,
            "beast",
        );
    }

    let (_, prompt) = call_of(ask(&mut story, 40));

    let foes = prompt_list(&prompt, "Creatures that the player can hunt:");
    assert_eq!(foes.len(), 20);
    assert_eq!(foes[0], "Duskbat 25");
    assert!(!foes.contains(&"Duskbat 5"), "{foes:?}");
}

#[test]
fn the_prompt_lists_the_newest_people() {
    let mut story = story("newest-people");
    for n in 1..=25 {
        story
            .handle(meet(10 + n, &format!("Farmhand {n}")))
            .unwrap();
    }

    let (_, prompt) = call_of(ask(&mut story, 40));

    let people = prompt_list(&prompt, "People that the player can meet:");
    assert_eq!(people.len(), 20);
    assert_eq!(people[0], "Farmhand 25");
    assert!(!people.contains(&"Farmer Bram"), "{people:?}");
}

#[test]
fn the_prompt_lists_the_newest_places() {
    let mut story = story("newest-places");
    for n in 1..=25 {
        story.handle(zone(10 + n, &format!("Field {n}"))).unwrap();
    }

    let (_, prompt) = call_of(ask(&mut story, 40));

    let places = prompt_list(&prompt, "Places that the player can visit:");
    assert_eq!(places.len(), 20);
    assert!(places.contains(&"Field 25"), "{places:?}");
    assert!(!places.contains(&"Mill Pond"), "{places:?}");
}

#[test]
fn the_prompt_lists_the_people_and_places_of_the_zone_of_the_giver_first() {
    let mut story = story("giver-zone-first");
    let far = Input::ZoneEntered {
        at: Tick(10),
        zone: "Farvale".to_string(),
        subzone: Some("Far Farm".to_string()),
        spot: None,
        hour: None,
    };
    story.handle(far).unwrap();
    story.handle(meet(11, "Farmer Fen")).unwrap();
    story.handle(zone(12, "Old Tower")).unwrap();

    let (_, prompt) = call_of(ask(&mut story, 40));

    let people = prompt_list(&prompt, "People that the player can meet:");
    let places = prompt_list(&prompt, "Places that the player can visit:");
    assert_eq!(people, ["Farmer Bram", "Farmer Fen"]);
    assert_eq!(places[0], "Testvale");
    assert_eq!(places.last(), Some(&"Far Farm"));
}

/// `/quest` to this giver and the end of its batch, when no model call goes out.
fn refused_ask(story: &mut Story, giver: &str, at: u64) -> Option<String> {
    let asked = Input::QuestAsked {
        at: Tick(at),
        npc: giver.to_string(),
    };
    assert_eq!(story.handle(asked).unwrap(), []);
    notice(story.handle(Input::BatchEnd { id: BATCH }).unwrap())
}

fn has_met(story: &mut Story, name: &str) -> bool {
    let people = page(story).journal.people;
    people.iter().any(|person| person.name == name)
}

#[test]
fn a_hostile_npc_gives_no_task_and_asking_does_not_meet_it() {
    let mut story = story("hostile-giver");
    sight(&mut story, 5, "Murloc Scout", Reaction::Hostile, "humanoid");

    let line = refused_ask(&mut story, "Murloc Scout", 6);

    assert_eq!(
        line.as_deref(),
        Some("Murloc Scout has no quest for you now.")
    );
    assert!(!has_met(&mut story, "Murloc Scout"));
}

#[test]
fn a_beast_gives_no_task() {
    let mut story = story("beast-giver");
    sight(&mut story, 5, "Old Hound", Reaction::Friendly, "beast");

    let line = refused_ask(&mut story, "Old Hound", 6);

    assert_eq!(line.as_deref(), Some("Old Hound has no quest for you now."));
    assert!(!has_met(&mut story, "Old Hound"));
}

#[test]
fn an_npc_that_you_talk_to_in_the_game_is_no_longer_a_foe() {
    let mut story = story("met-foe");
    sight(&mut story, 5, "Guard Rolf", Reaction::Hostile, "humanoid");

    story.handle(meet(6, "Guard Rolf")).unwrap();

    let (_, prompt) = call_of(ask(&mut story, 7));
    let people = prompt_list(&prompt, "People that the player can meet:");
    let foes = prompt_list(&prompt, "Creatures that the player can hunt:");
    assert!(people.contains(&"Guard Rolf"), "{people:?}");
    assert!(!foes.contains(&"Guard Rolf"), "{foes:?}");
}

/// The bridge drops an `events_seen` after its deadline, so a slow offer waits for the next
/// answer. The Tasks page shows it at once.
#[test]
fn an_offer_after_the_deadline_of_its_batch_comes_with_the_next_answer() {
    let mut story = story("late-offer");
    story.set_events_deadline(Duration::ZERO);
    let (call, _) = call_of(ask(&mut story, 5));

    let late = story
        .handle(Input::ModelAnswered {
            call,
            text: OFFER.to_string(),
        })
        .unwrap();
    let next = story.handle(Input::BatchEnd { id: BATCH }).unwrap();

    assert_eq!(late, []);
    assert_eq!(
        notice(next).as_deref(),
        Some(
            "Keeper Tessa has a quest for you: The Lost Lantern. I lost my lantern. Find it. Type /quest accept."
        )
    );
    assert_eq!(quests(&mut story)[0].status, Status::Offered);
}

#[test]
fn trust_why_names_a_finished_quest() {
    let mut story = story("why-quest");
    offer(&mut story, 5);
    story
        .handle(Input::QuestAccepted {
            at: Tick(6),
            number: None,
        })
        .unwrap();
    story.handle(zone(8, "Mill Pond")).unwrap();
    story.handle(meet(9, "Farmer Bram")).unwrap();

    let people = page(&mut story).journal.people;

    let giver = people.iter().find(|person| person.name == GIVER).unwrap();
    let why = TrustWhy {
        by: TrustCause::Quest,
        up: true,
        at: Tick(9),
    };
    assert_eq!(giver.trust_why, Some(why));
}

#[test]
fn asking_for_a_quest_moves_no_meet_step() {
    let mut story = story("ask-no-meet");
    let (call, _) = call_of(ask(&mut story, 5));
    answer_with(&mut story, call, "Word for Bram", MEET_BRAM);
    accept(&mut story, 6, None);

    story
        .handle(Input::QuestAsked {
            at: Tick(7),
            npc: "Farmer Bram".to_string(),
        })
        .unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn a_talk_remembers_the_quest_that_the_npc_gave() {
    let mut story = story("talk-remembers-quest");
    offer(&mut story, 5);
    let asked = Input::TalkAsked {
        id: MessageId(8),
        at: Tick(6),
        npc: GIVER.to_string(),
        text: "about that lantern".to_string(),
    };

    let (_, prompt) = call_of(story.handle(asked).unwrap().remove(0));

    assert!(
        prompt.contains(
            "you gave the player your quest \"The Lost Lantern\". They have not answered yet."
        ),
        "{prompt}"
    );
}

const TALK_BRAM: &str = r#"{"goal": "talk", "npc": "Farmer Bram", "about": "the missing cask"}"#;

/// An accepted quest with one step: ask Farmer Bram about the missing cask.
fn talk_quest(name: &str) -> Story {
    let mut story = story(name);
    let (call, _) = call_of(ask(&mut story, 5));
    answer_with(&mut story, call, "A Word with Bram", TALK_BRAM);
    accept(&mut story, 6, None);
    story
}

/// `/talk` to the NPC: the prompt of the call. The call then fails, so its slot is free.
fn talk_prompt(story: &mut Story, at: u64, npc: &str) -> String {
    let talk = Input::TalkAsked {
        id: MessageId(8),
        at: Tick(at),
        npc: npc.to_string(),
        text: "Hello there.".to_string(),
    };
    let (call, prompt) = call_of(story.handle(talk).unwrap().remove(0));
    story.handle(Input::ModelFailed { call }).unwrap();
    prompt
}

#[test]
fn a_talk_does_a_talk_step() {
    let mut story = talk_quest("talk-step");

    talk_prompt(&mut story, 7, "Farmer Bram");

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_gossip_window_never_does_a_talk_step() {
    let mut story = talk_quest("gossip-no-talk");

    story.handle(meet(7, "Farmer Bram")).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn a_slap_never_does_a_talk_step() {
    let mut story = talk_quest("slap-no-talk");
    let slap = Input::NpcSlapped {
        at: Tick(7),
        name: "Farmer Bram".to_string(),
    };

    story.handle(slap).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn a_talk_to_the_npc_of_a_talk_step_tells_the_npc_about_the_quest() {
    let mut story = talk_quest("talk-line");

    let prompt = talk_prompt(&mut story, 7, "Farmer Bram");

    assert!(prompt.contains("a quest of another person"), "{prompt}");
    assert!(prompt.contains("Giver: Keeper Tessa"), "{prompt}");
    assert!(prompt.contains("Quest: A Word with Bram"), "{prompt}");
    assert!(prompt.contains("Topic: the missing cask"), "{prompt}");
}

#[test]
fn the_quest_line_stays_in_the_later_turns_of_the_conversation() {
    let mut story = talk_quest("talk-line-stays");
    talk_prompt(&mut story, 7, "Farmer Bram");

    let later = talk_prompt(&mut story, 7 + 9 * 60, "Farmer Bram");
    let after_ten_minutes = talk_prompt(&mut story, 7 + 10 * 60, "Farmer Bram");

    assert!(later.contains("Quest: A Word with Bram"), "{later}");
    assert!(
        !after_ten_minutes.contains("Quest: A Word with Bram"),
        "{after_ten_minutes}"
    );
}

#[test]
fn a_talk_to_another_npc_hears_nothing_of_the_quest() {
    let mut story = talk_quest("talk-other");

    let prompt = talk_prompt(&mut story, 7, "Miller Oda");

    assert!(!prompt.contains("a quest of another person"), "{prompt}");
}

/// A quest of three steps: visit Mill Pond, come back after a day, and tell the giver
/// with /talk. The visit is done at 7, so the wait ends at 7 plus a day.
fn wait_quest(story: &mut Story) -> u64 {
    let steps = format!(
        r#"{VISIT_POND}, {{"goal": "wait", "days": 1}}, {{"goal": "talk", "npc": "{GIVER}"}}"#
    );
    let (call, _) = call_of(ask(story, 5));
    answer_with(story, call, "Come Back Tomorrow", &steps);
    accept(story, 6, None);
    story.handle(zone(7, "Mill Pond")).unwrap();
    7 + DAY_SECONDS
}

#[test]
fn a_wait_ends_with_the_first_line_after_its_time() {
    let mut story = story("wait-ends");
    let ready = wait_quest(&mut story);

    story.handle(zone(ready - 1, "Old Tower")).unwrap();
    let before = steps_done(&quests(&mut story)[0]);
    story.handle(zone(ready, "Mill Pond")).unwrap();

    assert_eq!(before, 1);
    assert_eq!(steps_done(&quests(&mut story)[0]), 2);
}

#[test]
fn a_wait_and_the_talk_after_it_end_on_one_line() {
    let mut story = story("wait-and-talk");
    let ready = wait_quest(&mut story);

    talk_prompt(&mut story, ready + 60, GIVER);

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_wait_survives_a_restart_of_the_story_program() {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("quests-wait-restart");
    let _ = std::fs::remove_dir_all(&folder);
    let mut story = story_in("wait-restart", Store::Folder(folder.clone()));
    let ready = wait_quest(&mut story);
    drop(story);

    let mut story = opened("wait-restart", Store::Folder(folder));
    story.handle(zone(ready, "Old Tower")).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 2);
}

#[test]
fn a_wait_never_ends_on_a_clock_that_went_back() {
    let mut story = story("wait-clock-back");
    wait_quest(&mut story);

    story.handle(zone(2, "Old Tower")).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 1);
}

#[test]
fn the_journal_shows_when_an_open_wait_is_over() {
    let mut story = story("wait-journal");
    let ready = wait_quest(&mut story);

    let wait = quests(&mut story).remove(0).steps.remove(1);

    assert_eq!(wait.state, StepState::Open);
    assert_eq!(wait.ready_at, Some(Tick(ready)));
}

#[test]
fn talks_and_quest_offers_share_one_count_of_calls() {
    let mut story = story("hook-count");
    let goal = Input::HeroSet {
        at: Tick(5),
        field: "goal".to_string(),
        text: "Find my father.".to_string(),
    };
    story.handle(goal).unwrap();
    for _ in 0..2 {
        let asked = Input::TalkAsked {
            id: MessageId(8),
            at: Tick(6),
            npc: GIVER.to_string(),
            text: "hello".to_string(),
        };
        let (call, prompt) = call_of(story.handle(asked).unwrap().remove(0));
        story.handle(Input::ModelFailed { call }).unwrap();
        assert!(!prompt.contains("Find my father."), "{prompt}");
    }

    let (_, prompt) = call_of(ask(&mut story, 7));

    assert!(prompt.contains("<<<\nFind my father.\n>>>"), "{prompt}");
}

#[test]
fn the_steps_of_a_set_end_in_the_order_that_you_do_them() {
    let mut story = story("set-order");
    let steps = format!(r#"{{"goal": "any_order", "steps": [{VISIT_POND}, {MEET_BRAM}]}}"#);
    let (call, _) = call_of(ask(&mut story, 5));
    answer_with(&mut story, call, "Odd Jobs", &steps);
    accept(&mut story, 6, None);

    story.handle(meet(7, "Farmer Bram")).unwrap();
    let half = quests(&mut story).remove(0);
    story.handle(zone(8, "Mill Pond")).unwrap();

    let states: Vec<StepState> = half.steps.iter().map(|step| step.state).collect();
    assert_eq!(states, [StepState::Open, StepState::Done]);
    assert_eq!(half.any_order, Some(AnyOrder { first: 0, last: 1 }));
    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

/// An accepted quest of one step: bring 10 Linen Cloth to Farmer Bram. You are level 12.
fn cloth_quest(name: &str) -> Story {
    let mut story = story(name);
    story
        .handle(Input::LevelReached {
            at: Tick(5),
            level: 12,
        })
        .unwrap();
    let step = r#"{"goal": "carry", "item": "Linen Cloth", "count": 10, "npc": "Farmer Bram"}"#;
    let (call, _) = call_of(ask(&mut story, 5));
    answer_with(&mut story, call, "Cloth for Bram", step);
    accept(&mut story, 6, None);
    story
}

fn items_held(at: u64, npc: &str, count: u16) -> Input {
    Input::ItemsHeld {
        at: Tick(at),
        npc: npc.to_string(),
        item: "Linen Cloth".to_string(),
        count,
    }
}

#[test]
fn items_held_at_its_npc_does_a_carry_step() {
    let mut story = cloth_quest("carry-done");

    story.handle(items_held(7, "Farmer Bram", 12)).unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn items_held_below_the_count_leaves_the_carry_step_open() {
    let mut story = cloth_quest("carry-short");

    story.handle(items_held(7, "Farmer Bram", 6)).unwrap();
    story.handle(items_held(8, "Miller Oda", 20)).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn the_prompt_lists_the_goods_of_your_level() {
    let mut story = story("carry-prompt");
    story
        .handle(Input::LevelReached {
            at: Tick(5),
            level: 12,
        })
        .unwrap();

    let (_, prompt) = call_of(ask(&mut story, 6));

    let goods = prompt_list(&prompt, "Goods that the player can bring:");
    assert!(goods.contains(&"Linen Cloth"), "{prompt}");
    assert!(!goods.contains(&"Runecloth"), "{prompt}");
    assert!(prompt.contains(r#""goal": "carry""#), "{prompt}");
}

/// The model gives the same bad answer to the first call and to its retry: the line of
/// Timeways for the batch.
fn refused_twice(story: &mut Story, call: CallId, text: &str) -> Option<String> {
    let first = story
        .handle(Input::ModelAnswered {
            call,
            text: text.to_string(),
        })
        .unwrap();
    let (retry, _) = call_of(first.into_iter().next().unwrap());
    notice(
        story
            .handle(Input::ModelAnswered {
                call: retry,
                text: text.to_string(),
            })
            .unwrap(),
    )
}

#[test]
fn a_refused_first_answer_asks_once_more_with_the_reason() {
    let mut story = story("retry");
    let (call, first_prompt) = call_of(ask(&mut story, 5));
    let bad = OFFER.replace("Farmer Bram", "Captain Vorn");

    let outputs = story
        .handle(Input::ModelAnswered {
            call,
            text: bad.clone(),
        })
        .unwrap();

    let (retry, prompt) = call_of(outputs.into_iter().next().unwrap());
    assert!(prompt.starts_with(&first_prompt), "{prompt}");
    assert!(prompt.contains("Your last answer was:"), "{prompt}");
    assert!(prompt.contains("Captain Vorn"), "{prompt}");
    assert!(prompt.contains("you never met or saw"), "{prompt}");
    let good = story
        .handle(Input::ModelAnswered {
            call: retry,
            text: OFFER.to_string(),
        })
        .unwrap();
    assert!(notice(good).unwrap().contains("has a quest for you"));
}

#[test]
fn a_second_refused_answer_gives_no_quest() {
    let mut story = story("retry-twice");
    let (call, _) = call_of(ask(&mut story, 5));

    let line = refused_twice(&mut story, call, "no quest here");

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no quest for you now.")
    );
    assert!(quests(&mut story).is_empty());
}

#[test]
fn a_failed_first_call_asks_no_retry() {
    let mut story = story("retry-failed");
    let (call, _) = call_of(ask(&mut story, 5));

    let outputs = story.handle(Input::ModelFailed { call }).unwrap();

    assert_eq!(
        notice(outputs).as_deref(),
        Some("Keeper Tessa has no quest for you now.")
    );
}

#[test]
fn a_refusal_of_the_limits_asks_no_model() {
    let mut story = story("limits-no-model");
    offer(&mut story, 5);
    accept(&mut story, 6, None);

    let output = ask(&mut story, 7);

    assert!(!matches!(output, Output::ModelCall { .. }), "{output:?}");
}

#[test]
fn a_retry_does_not_count_for_the_hero_hook() {
    let mut story = story("retry-hook");
    let goal = Input::HeroSet {
        at: Tick(5),
        field: "goal".to_string(),
        text: "Find my father.".to_string(),
    };
    story.handle(goal).unwrap();
    let (call, _) = call_of(ask(&mut story, 6));
    refused_twice(&mut story, call, "no quest here");
    talk_prompt(&mut story, 7, "Farmer Bram");

    let (_, prompt) = call_of(ask(&mut story, 8));

    assert!(prompt.contains("<<<\nFind my father.\n>>>"), "{prompt}");
}

/// An accepted quest of one step, of this genre.
fn one_step_quest(name: &str, genre: &str, step: &str) -> Story {
    let mut story = story(name);
    let (call, _) = call_of(ask(&mut story, 5));
    let text = answer_text("Odd Manners", step).replace("errand", genre);
    let line = notice(story.handle(Input::ModelAnswered { call, text }).unwrap());
    assert!(line.unwrap().contains("has a quest for you"));
    accept(&mut story, 6, None);
    story
}

fn emote_at(at: u64, emote: &str, target: Option<&str>) -> Input {
    Input::EmoteDone {
        at: Tick(at),
        emote: emote.to_string(),
        target: target.map(str::to_string),
        hour: Some(12),
    }
}

const BOW_TO_BRAM: &str = r#"{"goal": "emote", "emote": "bow", "npc": "Farmer Bram"}"#;

#[test]
fn an_emote_at_the_npc_of_the_step_does_it() {
    let mut story = one_step_quest("emote-npc", "errand", BOW_TO_BRAM);

    story
        .handle(emote_at(7, "bow", Some("Farmer Bram")))
        .unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn an_emote_at_another_npc_does_nothing() {
    let mut story = one_step_quest("emote-other", "errand", BOW_TO_BRAM);

    story
        .handle(emote_at(7, "bow", Some("Miller Oda")))
        .unwrap();
    story
        .handle(emote_at(8, "wave", Some("Farmer Bram")))
        .unwrap();
    story.handle(emote_at(9, "bow", None)).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn an_emote_in_the_place_of_the_step_does_it() {
    let step = r#"{"goal": "emote", "emote": "dance", "place": "Mill Pond"}"#;
    let mut story = one_step_quest("emote-place", "comic", step);

    story.handle(emote_at(7, "dance", None)).unwrap();
    let elsewhere = steps_done(&quests(&mut story)[0]);
    story.handle(zone(8, "Mill Pond")).unwrap();
    story
        .handle(emote_at(9, "dance", Some("Farmer Bram")))
        .unwrap();

    assert_eq!(elsewhere, 0);
    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_slap_step_costs_trust_as_any_slap() {
    let step = r#"{"goal": "slap", "npc": "Farmer Bram"}"#;
    let mut story = one_step_quest("slap-step", "comic", step);
    let slap = Input::NpcSlapped {
        at: Tick(7),
        name: "Farmer Bram".to_string(),
    };

    story.handle(slap).unwrap();

    let quest = quests(&mut story).remove(0);
    assert_eq!(quest.status, Status::Done);
    assert!(quest.has_slap);
    let bram = page(&mut story)
        .journal
        .people
        .into_iter()
        .find(|person| person.name == "Farmer Bram")
        .unwrap();
    assert_eq!(bram.trust, Some(-SLAP_TRUST));
    assert_eq!(bram.slapped, Some(1));
}

/// An accepted quest of one step: be at Mill Pond at night. You stand in Old Tower.
fn night_quest(name: &str) -> Story {
    let step = r#"{"goal": "visit_at", "place": "Mill Pond", "time": "night"}"#;
    one_step_quest(name, "mystery", step)
}

fn zone_at_hour(at: u64, subzone: &str, hour: Option<u8>) -> Input {
    Input::ZoneEntered {
        at: Tick(at),
        zone: "Testvale".to_string(),
        subzone: Some(subzone.to_string()),
        spot: None,
        hour,
    }
}

#[test]
fn a_zone_at_the_right_hour_does_a_time_step() {
    let mut story = night_quest("night-zone");

    story
        .handle(zone_at_hour(7, "Mill Pond", Some(23)))
        .unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_zone_at_the_wrong_hour_leaves_the_time_step_open() {
    let mut story = night_quest("noon-zone");

    story
        .handle(zone_at_hour(7, "Mill Pond", Some(12)))
        .unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn an_hour_change_while_you_stand_in_the_place_does_the_time_step() {
    let mut story = night_quest("night-falls");
    story
        .handle(zone_at_hour(7, "Mill Pond", Some(20)))
        .unwrap();

    story
        .handle(Input::HourChanged {
            at: Tick(8),
            hour: 21,
        })
        .unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_line_with_no_hour_never_does_a_time_step() {
    let mut story = night_quest("no-hour");

    story.handle(zone_at_hour(7, "Mill Pond", None)).unwrap();
    story.handle(meet(8, "Farmer Bram")).unwrap();

    assert_eq!(steps_done(&quests(&mut story)[0]), 0);
}

#[test]
fn an_hour_past_twenty_three_is_refused() {
    let mut story = story("hour-24");

    let changed = story.handle(Input::HourChanged {
        at: Tick(7),
        hour: 24,
    });
    let zone = story.handle(zone_at_hour(8, "Mill Pond", Some(24)));

    assert!(changed.is_err() && zone.is_err());
}

fn level(at: u64, level: u8) -> Input {
    Input::LevelReached {
        at: Tick(at),
        level,
    }
}

#[test]
fn a_level_reached_before_the_step_opens_does_it_at_once() {
    let mut story = story("level-early");
    story.handle(level(5, 10)).unwrap();
    let steps = format!(r#"{VISIT_POND}, {{"goal": "level", "level": 11}}"#);
    quest_with(&mut story, 5, "Growing Up", &steps);
    story.handle(level(7, 11)).unwrap();

    story.handle(zone(8, "Mill Pond")).unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

/// You entered The Deadmines and defeated Edwin there, and stand in Old Tower again.
fn after_a_dungeon(name: &str) -> Story {
    let mut story = story(name);
    story.handle(zone_in("The Deadmines", 5)).unwrap();
    let mark = Input::InstanceEntered {
        at: Tick(5),
        zone: "The Deadmines".to_string(),
        kind: timeways_story::places::InstanceKind::Dungeon,
    };
    story.handle(mark).unwrap();
    let boss = Input::NpcDefeated {
        at: Tick(6),
        name: "Edwin VanCleef".to_string(),
    };
    story.handle(boss).unwrap();
    story.handle(zone(7, "Old Tower")).unwrap();
    story
}

fn zone_in(zone: &str, at: u64) -> Input {
    Input::ZoneEntered {
        at: Tick(at),
        zone: zone.to_string(),
        subzone: None,
        spot: None,
        hour: None,
    }
}

fn quest_with(story: &mut Story, at: u64, title: &str, steps: &str) {
    let (call, _) = call_of(ask(story, at));
    let line = answer_with(story, call, title, steps);
    assert!(line.unwrap().contains("has a quest for you"));
    accept(story, at + 1, None);
}

#[test]
fn standing_in_the_dungeon_when_the_step_opens_does_it_at_once() {
    let mut story = after_a_dungeon("dungeon-at-once");
    story.handle(zone_in("The Deadmines", 8)).unwrap();

    quest_with(
        &mut story,
        9,
        "Into the Dark",
        r#"{"goal": "enter", "dungeon": "The Deadmines"}"#,
    );

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_boss_defeated_does_a_defeat_step() {
    let mut story = after_a_dungeon("boss-again");
    quest_with(
        &mut story,
        8,
        "Old Scores",
        r#"{"goal": "defeat", "boss": "Edwin VanCleef"}"#,
    );

    let boss = Input::NpcDefeated {
        at: Tick(20),
        name: "Edwin VanCleef".to_string(),
    };
    story.handle(boss).unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_game_quest_turned_in_before_the_step_opens_does_it_at_once() {
    let mut story = story("game-quest-early");
    let taken = Input::GameQuestAccepted {
        at: Tick(5),
        title: "Wanted: Hogger".to_string(),
        kind: timeways_story::input::GameQuestKind::Normal,
    };
    story.handle(taken).unwrap();
    let steps = format!(r#"{VISIT_POND}, {{"goal": "game_quest", "title": "Wanted: Hogger"}}"#);
    quest_with(&mut story, 6, "Help the Guard", &steps);
    let done = Input::GameQuestDone {
        at: Tick(8),
        title: "Wanted: Hogger".to_string(),
        kind: timeways_story::input::GameQuestKind::Normal,
    };
    story.handle(done).unwrap();

    story.handle(zone(9, "Mill Pond")).unwrap();

    assert_eq!(quests(&mut story)[0].status, Status::Done);
}

#[test]
fn a_mystery_shows_only_its_done_and_open_steps_in_the_journal() {
    let steps = format!("{VISIT_POND}, {MEET_BRAM}, {VISIT_TOWER}");
    let mut story = story("mystery");
    let (call, _) = call_of(ask(&mut story, 5));
    let text = answer_text("A Cask Gone Missing", &steps).replace("errand", "mystery");
    story.handle(Input::ModelAnswered { call, text }).unwrap();
    let offered = quests(&mut story).remove(0);
    accept(&mut story, 6, None);

    story.handle(zone(7, "Mill Pond")).unwrap();

    let quest = quests(&mut story).remove(0);
    assert_eq!(offered.steps.len(), 1);
    assert_eq!(offered.hidden_steps, 2);
    assert_eq!(quest.steps.len(), 2);
    assert_eq!(quest.hidden_steps, 1);
    let hidden = |step: &StepView| step.step.target() == Some("Old Tower");
    assert!(!quest.steps.iter().any(hidden));
}

#[test]
fn the_journal_leaves_out_the_genre() {
    let mut story = story("no-genre-in-book");
    offer(&mut story, 5);

    let line = serde_json::to_string(&page(&mut story)).unwrap();

    assert!(!line.contains("errand"), "{line}");
}

#[test]
fn a_giver_that_turns_hostile_while_the_model_thinks_gives_no_offer() {
    let mut story = story("hostile-while-thinking");
    let (call, _) = call_of(ask(&mut story, 5));
    sight(&mut story, 6, GIVER, Reaction::Hostile, "humanoid");

    let line = answer_with(&mut story, call, "The Lost Lantern", VISIT_POND);

    assert_eq!(
        line.as_deref(),
        Some("Keeper Tessa has no quest for you now.")
    );
    assert!(quests(&mut story).is_empty());
}

#[test]
fn an_offer_of_a_giver_that_turned_hostile_cannot_be_accepted() {
    let mut story = story("hostile-before-accept");
    let (call, _) = call_of(ask(&mut story, 5));
    answer_with(&mut story, call, "The Lost Lantern", VISIT_POND);
    sight(&mut story, 6, GIVER, Reaction::Hostile, "humanoid");

    accept(&mut story, 7, None);

    assert_eq!(open_quests(&mut story), 0);
}

#[test]
fn a_mystery_that_names_a_hidden_step_gets_a_retry_with_the_reason() {
    let mut story = story("mystery-retry");
    let (call, _) = call_of(ask(&mut story, 5));
    let text = answer_text("Lights on the Pond", &format!("{VISIT_POND}, {MEET_BRAM}"))
        .replace("errand", "mystery")
        .replace("go and look", "go to the pond, then ask Farmer Bram");

    let outputs = story.handle(Input::ModelAnswered { call, text }).unwrap();

    let (_, prompt) = call_of(outputs.into_iter().next().unwrap());
    assert!(
        prompt.contains("the text of a mystery names \"Farmer Bram\""),
        "{prompt}"
    );
}
