//! Side quests through the story program: the offer, the answer, and the progress
//! (GAMEPLAY.md 3.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::path::Path;
use std::time::Duration;
use timeways_story::character::QUEST_TRUST;
use timeways_story::input::{CallId, Input, MessageId, Reaction};
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
        spot: None,
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
            "Keeper Tessa has a quest for you: The Lost Lantern. Find the lantern. Type /quest accept."
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

    let line = notice(story.handle(Input::ModelAnswered { call, text }).unwrap());

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
    let text = format!(r#"{{"title": "{title}", "text": "Go and look.", "steps": [{step}]}}"#);
    notice(story.handle(Input::ModelAnswered { call, text }).unwrap())
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

#[test]
fn an_accept_past_three_open_quests_is_refused_in_a_notice() {
    let mut story = story("full-log");
    let givers = ["Keeper Tessa", "Innkeeper Pell", "Guard Rolf", "Smith Hana"];
    for (n, giver) in (0u64..).zip(givers) {
        story.handle(meet(10 + n, giver)).unwrap();
        let (call, _) = call_of(ask_from(&mut story, giver, 20 + n));
        let step = if n % 2 == 0 { VISIT_POND } else { MEET_BRAM };
        answer_with(&mut story, call, &format!("Task {n}"), step);
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
    assert_eq!(quests(&mut story)[0].kills, 1);
    kill(&mut story, 9, "Duskbat");

    let done = quests(&mut story).remove(0);
    assert_eq!(done.status, Status::Done);
    assert_eq!(trust_of_giver(&mut story), Some(QUEST_TRUST));
}

#[test]
fn a_kill_of_another_creature_counts_nothing() {
    let mut story = bat_hunt("other-kill");

    kill(&mut story, 8, "Mill Rat");

    assert_eq!(quests(&mut story)[0].kills, 0);
}

#[test]
fn a_kill_before_the_hunt_is_accepted_counts_nothing() {
    let mut story = story("early-kill");
    sight(&mut story, 5, "Duskbat", Reaction::Hostile, "beast");
    let (call, _) = call_of(ask(&mut story, 6));
    answer_with(&mut story, call, "Bats in the Belfry", KILL_BATS);

    kill(&mut story, 7, "Duskbat");
    accept(&mut story, 8, None);

    assert_eq!(quests(&mut story)[0].kills, 0);
}

#[test]
fn the_next_task_never_names_a_target_of_the_last_one() {
    let mut story = bat_hunt("last-task");
    kill(&mut story, 8, "Duskbat");
    kill(&mut story, 9, "Duskbat");
    story.handle(meet(10, "Innkeeper Pell")).unwrap();

    let (call, prompt) = call_of(ask_from(&mut story, "Innkeeper Pell", 11));
    let line = answer_with(&mut story, call, "More Bats", KILL_BATS);

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
            "Keeper Tessa has a quest for you: The Lost Lantern. Find the lantern. Type /quest accept."
        )
    );
    assert_eq!(quests(&mut story)[0].status, Status::Offered);
}
