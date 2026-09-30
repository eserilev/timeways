//! `/quest`, and the Quests page of the book (GAMEPLAY.md 3.4 and 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{Deed, Journal, pages};
use timeways_story::quest::{Status, Step, Tracked};
use timeways_story::story::Output;

const DAY: u64 = 1_790_000_000;

fn lantern(status: Status, steps_done: usize) -> Tracked {
    Tracked {
        number: 1,
        offered_at: Tick(DAY),
        giver: "Keeper Tessa".to_string(),
        title: "The Lost Lantern".to_string(),
        text: "Find the lantern.".to_string(),
        steps: vec![
            Step::Visit {
                place: "Mill Pond".to_string(),
            },
            Step::Meet {
                npc: "Farmer Bram".to_string(),
            },
        ],
        steps_done,
        status,
        done_at: (status == Status::Done).then_some(Tick(DAY)),
    }
}

/// The reply line of the first page of a journal with only this quest.
fn quest_reply(quest: Tracked) -> String {
    let journal = Journal {
        quests: vec![quest],
        ..Journal::default()
    };
    let page = pages(journal).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
    })
    .unwrap()
}

/// The page as text: one `style: text [button]` entry for each line.
fn lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('quests')) do
             local action = line.action and (' [' .. line.action.label .. ']') or ''
             table.insert(out, line.style .. ': ' .. line.text .. action)
         end
         return out",
    )
}

fn click(game: &Game, label: &str) {
    game.run(&format!(
        "for _, line in ipairs(ns.Journal.Lines('quests')) do
             if line.action and line.action.label == '{label}' then line.action.run() end
         end"
    ));
}

fn day(game: &Game) -> String {
    game.eval(&format!("date('%d %b %Y', {DAY})"))
}

#[test]
fn quest_asks_the_target_for_a_task_at_once() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Keeper Tessa' }
         wow.Slash('/quest', '')",
    );

    let asked = matches!(
        game.sent_inputs().as_slice(),
        [Input::QuestAsked { npc, .. }] if npc == "Keeper Tessa"
    );
    assert!(asked, "{:?}", game.sent_inputs());
    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: You ask Keeper Tessa for a task."]
    );
}

#[test]
fn quest_needs_an_npc_target() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Grimtusk', player = true }
         wow.Slash('/quest', '')",
    );

    assert!(game.sent().is_empty());
    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Who are you asking? Target someone first."]
    );
}

#[test]
fn quest_accept_and_decline_answer_the_offer() {
    let game = Game::new();

    game.run(
        "wow.Slash('/quest', 'accept')
         wow.Slash('/quest', 'DECLINE')
         wow.RunTickers()",
    );

    let inputs = game.sent_inputs();
    assert!(
        matches!(
            inputs.as_slice(),
            [
                Input::QuestAccepted { number: None, .. },
                Input::QuestDeclined { number: None, .. }
            ]
        ),
        "{inputs:?}"
    );
}

#[test]
fn an_unknown_word_shows_how_to_use_quest() {
    let game = Game::new();

    game.run("wow.Slash('/quest', 'abandon')");

    assert!(game.sent().is_empty());
    assert_eq!(
        game.printed(),
        [
            "|cffc8a064Timeways|r: Target someone and type /quest to ask for a task. Then /quest accept, or /quest decline."
        ]
    );
}

#[test]
fn accepting_sends_an_npc_that_you_met_in_this_session_again() {
    let game = Game::new();
    game.run(
        "wow.units.npc = { name = 'Farmer Bram' }
         wow.Fire('GOSSIP_SHOW')
         wow.Slash('/quest', 'accept')
         wow.Fire('GOSSIP_SHOW')
         wow.RunTickers()",
    );

    let meetings = game
        .sent_inputs()
        .into_iter()
        .filter(|input| matches!(input, Input::NpcMet { name, .. } if name == "Farmer Bram"))
        .count();

    assert_eq!(meetings, 2);
}

#[test]
fn an_offer_shows_with_its_buttons() {
    let game = Game::new();

    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    assert_eq!(
        lines(&game),
        [
            "heading: The Lost Lantern [Accept]".to_string(),
            format!("text: From Keeper Tessa, on {}.", day(&game)),
            "prose: Find the lantern.".to_string(),
            "entry: Visit Mill Pond.".to_string(),
            "entry: Speak with Farmer Bram.".to_string(),
            "note: An offer. Do you take it? [Decline]".to_string(),
        ]
    );
}

#[test]
fn the_buttons_of_an_offer_answer_it_by_its_number() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    click(&game, "Accept");
    click(&game, "Decline");
    game.run("wow.RunTickers()");

    let inputs = game.sent_inputs();
    assert!(
        matches!(
            inputs.as_slice(),
            [
                Input::QuestAccepted {
                    number: Some(1),
                    ..
                },
                Input::QuestDeclined {
                    number: Some(1),
                    ..
                }
            ]
        ),
        "{inputs:?}"
    );
}

#[test]
fn a_quest_in_progress_marks_its_steps_done() {
    let game = Game::new();

    game.reply(&quest_reply(lantern(Status::Accepted, 1)));

    let shown = lines(&game);
    assert_eq!(shown[0], "heading: The Lost Lantern");
    assert_eq!(shown[3], "entry: (done) Visit Mill Pond.");
    assert_eq!(shown[4], "entry: Speak with Farmer Bram.");
    assert_eq!(shown[5], "note: In progress. [Abandon]");
}

#[test]
fn a_done_quest_shows_the_day_it_ended() {
    let game = Game::new();

    game.reply(&quest_reply(lantern(Status::Done, 2)));

    let expected = format!("note: Done on {}.", day(&game));
    assert_eq!(lines(&game).last(), Some(&expected));
}

#[test]
fn a_broken_quest_shows_gaps_and_no_error() {
    let game = Game::new();

    game.reply(
        r#"{"type":"journal","page":0,"pages":1,"quests":[{"title":7,"steps":[{"goal":"fly"},"x"],"steps_done":"a"}, 5]}"#,
    );

    assert_eq!(
        lines(&game),
        [
            "heading: ?",
            "text: From ?, on an unknown day.",
            "entry: ?",
            "note: In progress."
        ]
    );
}

#[test]
fn with_no_quest_the_page_says_how_to_ask() {
    let game = Game::new();

    game.reply(r#"{"type":"journal","page":0,"pages":1}"#);

    assert_eq!(
        lines(&game),
        ["help: No one has asked you for a favor yet. Target someone, and type /quest."]
    );
}

#[test]
fn the_tabs_stay_inside_the_row_of_the_frame() {
    let game = Game::new();

    game.run("wow.Slash('/journal', '')");

    let right: f64 = game.eval(
        "local right = 0
         for _, widget in ipairs(wow.widgets) do
             if widget.template == 'UIPanelButtonTemplate' and widget.parent == TimewaysJournalFrame then
                 right = math.max(right, widget.point[4] + widget.width)
             end
         end
         return right",
    );
    assert!(right <= 351.0, "{right}");
}

#[test]
fn a_finished_quest_shows_as_a_deed() {
    let game = Game::new();
    let journal = Journal {
        deeds: vec![Deed::QuestDone {
            title: "The Lost Lantern".to_string(),
            at: Tick(DAY),
            place: None,
        }],
        ..Journal::default()
    };
    let page = pages(journal).remove(0);
    let reply = serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
    })
    .unwrap();

    game.reply(&reply);

    let first: String = game.eval("ns.Journal.Lines('deeds')[1].text");
    assert_eq!(first, "Finished the task The Lost Lantern");
}

#[test]
fn a_target_that_you_can_attack_has_no_tasks_to_give() {
    let game = Game::new();

    game.run(
        "wow.units.target = { name = 'Duskbat', hostile = true }
         wow.Slash('/quest', '')",
    );

    assert!(game.sent().is_empty());
    assert_eq!(
        game.printed(),
        ["|cffc8a064Timeways|r: Duskbat has no tasks to give."]
    );
}

#[test]
fn an_open_task_asks_first_and_then_abandons() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Accepted, 1)));

    click(&game, "Abandon");
    let asked_first = game.sent().is_empty();
    game.run("wow.AcceptPopup()");
    game.run("wow.RunTickers()");

    assert!(asked_first);
    let inputs = game.sent_inputs();
    assert!(
        matches!(
            inputs.as_slice(),
            [
                Input::QuestAbandoned { number: 1, .. },
                Input::JournalAsked { page: 0, .. }
            ]
        ),
        "{inputs:?}"
    );
}
