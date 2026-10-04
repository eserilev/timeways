//! `/quest`, and the Quests page of the book (GAMEPLAY.md 3.4 and 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Game, step_views};
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{Deed, Journal, pages};
use timeways_story::quest::{QuestView, Status, Step};
use timeways_story::story::Output;

const DAY: u64 = 1_790_000_000;

fn lantern(status: Status, steps_done: usize) -> QuestView {
    QuestView {
        number: 1,
        offered_at: Tick(DAY),
        giver: "Keeper Tessa".to_string(),
        title: "The Lost Lantern".to_string(),
        text: "Find the lantern.".to_string(),
        steps: step_views(
            vec![
                Step::Visit {
                    place: "Mill Pond".to_string(),
                },
                Step::Meet {
                    npc: "Farmer Bram".to_string(),
                },
            ],
            status,
            steps_done,
        ),
        status,
        done_at: (status == Status::Done).then_some(Tick(DAY)),
    }
}

/// The reply line of the first page of a journal with only this quest.
fn quest_reply(quest: QuestView) -> String {
    let journal = Journal {
        quests: vec![quest],
        ..Journal::default()
    };
    let page = pages(journal).remove(0);
    serde_json::to_string(&Output::Journal {
        id: MessageId(1),
        page,
        notice: None,
    })
    .unwrap()
}

/// The parchment of the open task as text: one `style: text` entry for each line.
fn lines(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Lines('quests')) do
             table.insert(out, line.style .. ': ' .. line.text)
         end
         return out",
    )
}

/// The labels of the buttons at the bottom of the Tasks page.
fn buttons(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, button in ipairs(ns.Journal.Page('quests').buttons) do
             table.insert(out, button.label)
         end
         return out",
    )
}

fn click(game: &Game, label: &str) {
    game.run(&format!(
        "for _, button in ipairs(ns.Journal.Page('quests').buttons) do
             if button.label == '{label}' then button.run() end
         end"
    ));
}

/// The lines of the rewards, at the end of each task of Keeper Tessa.
const REWARDS: [&str; 3] = [
    "section: Rewards",
    "text: Keeper Tessa trusts you more.",
    "text: An entry in your chronicle.",
];

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
        ["|cffc8a064Timeways|r: You ask Keeper Tessa for a quest."]
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
        ["|cffc8a064Timeways|r: Target someone to ask first."]
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
                Input::JournalAsked { page: 0, .. },
                Input::QuestDeclined { number: None, .. },
                Input::JournalAsked { page: 0, .. }
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
            "|cffc8a064Timeways|r: Target someone and type /quest to ask for a quest. Then /quest accept or /quest decline."
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

    let mut expected = vec![
        "heading: The Lost Lantern".to_string(),
        "note: New offer.".to_string(),
        format!("text: From Keeper Tessa, on {}.", day(&game)),
        "prose: Find the lantern.".to_string(),
        "entry: Visit Mill Pond.".to_string(),
        "entry: Speak with Farmer Bram.".to_string(),
    ];
    expected.extend(REWARDS.map(str::to_string));
    assert_eq!(lines(&game), expected);
    assert_eq!(buttons(&game), ["Accept", "Decline"]);
}

#[test]
fn accept_answers_its_offer_by_number_and_asks_for_the_journal() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    click(&game, "Accept");
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
                Input::JournalAsked { page: 0, .. }
            ]
        ),
        "{inputs:?}"
    );
}

#[test]
fn decline_answers_its_offer_by_number_and_asks_for_the_journal() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    click(&game, "Decline");
    game.run("wow.RunTickers()");

    let inputs = game.sent_inputs();
    assert!(
        matches!(
            inputs.as_slice(),
            [
                Input::QuestDeclined {
                    number: Some(1),
                    ..
                },
                Input::JournalAsked { page: 0, .. }
            ]
        ),
        "{inputs:?}"
    );
}

#[test]
fn an_accepted_offer_shows_as_saving_at_once() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    click(&game, "Accept");

    let shown = lines(&game);
    assert_eq!(shown[0], "heading: The Lost Lantern");
    assert_eq!(shown[1], "hint: Saving...");
    assert!(buttons(&game).is_empty());
}

#[test]
fn a_declined_offer_leaves_the_page_at_once() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    click(&game, "Decline");

    let shown = lines(&game);
    assert!(
        !shown.iter().any(|line| line.contains("The Lost Lantern")),
        "{shown:?}"
    );
}

#[test]
fn an_abandoned_task_leaves_the_page_at_once() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Accepted, 1)));

    click(&game, "Abandon");
    game.run("wow.AcceptPopup()");

    let shown = lines(&game);
    assert!(
        !shown.iter().any(|line| line.contains("The Lost Lantern")),
        "{shown:?}"
    );
}

#[test]
fn the_journal_replaces_the_answer_that_waits() {
    let game = Game::new();
    game.reply(&quest_reply(lantern(Status::Offered, 0)));
    click(&game, "Accept");

    game.reply(&quest_reply(lantern(Status::Accepted, 0)));

    assert_eq!(lines(&game)[1], "note: In progress.");
    assert_eq!(buttons(&game), ["Abandon"]);
}

#[test]
fn a_quest_in_progress_marks_its_steps_done() {
    let game = Game::new();

    game.reply(&quest_reply(lantern(Status::Accepted, 1)));

    let shown = lines(&game);
    assert_eq!(shown[1], "note: In progress.");
    assert_eq!(shown[4], "entry: Visit Mill Pond. (Complete)");
    assert_eq!(shown[5], "entry: Speak with Farmer Bram.");
}

#[test]
fn a_done_quest_shows_the_day_it_ended() {
    let game = Game::new();

    game.reply(&quest_reply(lantern(Status::Done, 2)));

    let expected = format!("note: Done on {}.", day(&game));
    assert_eq!(lines(&game)[1], expected);
    assert!(buttons(&game).is_empty());
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
            "note: In progress.",
            "text: From ?, on an unknown day.",
            "entry: ?",
            "section: Rewards",
            "text: ? trusts you more.",
            "text: An entry in your chronicle.",
        ]
    );
}

#[test]
fn with_no_quest_the_page_says_how_to_ask() {
    let game = Game::new();

    game.reply(r#"{"type":"journal","page":0,"pages":1}"#);

    assert_eq!(
        lines(&game),
        ["help: No quests yet. Target someone and type /quest to ask for one."]
    );
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
        notice: None,
    })
    .unwrap();

    game.reply(&reply);

    let first: String = game.eval("ns.Journal.Lines('deeds')[1].text");
    assert_eq!(first, "Finished the quest The Lost Lantern");
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
        ["|cffc8a064Timeways|r: Duskbat has no quests to give."]
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

/// A journal with a task in each state, in an order that the list changes.
const THREE_TASKS: &str = concat!(
    r#"{"type":"journal","page":0,"pages":1,"quests":["#,
    r#"{"number":1,"giver":"Keeper Tessa","title":"Old Bones","status":"done","steps":[]},"#,
    r#"{"number":2,"giver":"Farmer Bram","title":"The Mill","status":"accepted","#,
    r#""steps":[{"goal":"visit","place":"Mill Pond","state":"done"},"#,
    r#"{"goal":"meet","npc":"Keeper Tessa","state":"open"}]},"#,
    r#"{"number":3,"giver":"Executor Arren","title":"Old Names","status":"offered","steps":[]}]}"#,
);

/// The rows of the list of the Tasks page, as `style: text (detail) [mark]`.
fn task_rows(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('quests').list) do
             local detail = row.detail and (' (' .. row.detail .. ')') or ''
             local mark = row.mark and (' [' .. row.mark .. ']') or ''
             table.insert(out, row.style .. ': ' .. row.text .. detail .. mark)
         end
         return out",
    )
}

#[test]
fn the_task_list_groups_offers_then_tasks_in_progress_then_done_ones() {
    let game = Game::new();

    game.reply(THREE_TASKS);

    assert_eq!(
        task_rows(&game),
        [
            "group: Offered",
            "item: Old Names (Executor Arren) [New]",
            "group: In progress",
            "item: The Mill (Farmer Bram) [1 of 2]",
            "group: Done",
            "item: Old Bones (Keeper Tessa) [Done]",
            "group: Quests you wrote",
            "item: New quest",
        ]
    );
}

#[test]
fn the_tasks_page_opens_on_the_first_task_of_the_list() {
    let game = Game::new();

    game.reply(THREE_TASKS);

    assert_eq!(lines(&game)[0], "heading: Old Names");
    assert_eq!(buttons(&game), ["Accept", "Decline"]);
}

#[test]
fn a_click_on_a_task_in_the_list_opens_it() {
    let game = Game::new();
    game.run("wow.Slash('/journal', '')");
    game.reply(THREE_TASKS);
    game.run("ns.JournalFrame.Open('quests')");

    game.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.text == 'The Mill' and widget.shown then
                 widget.parent:Click()
             end
         end",
    );

    assert_eq!(lines(&game)[0], "heading: The Mill");
    assert_eq!(buttons(&game), ["Abandon"]);
}

#[test]
fn an_accepted_task_stays_open_when_it_moves_to_another_group() {
    let game = Game::new();
    game.reply(THREE_TASKS);
    game.run("ns.Journal.Select('quests', 3)");

    click(&game, "Accept");
    game.reply(&THREE_TASKS.replace(r#""status":"offered""#, r#""status":"accepted""#));

    assert_eq!(lines(&game)[0], "heading: Old Names");
    assert_eq!(task_rows(&game)[0], "group: In progress");
}

#[test]
fn a_declined_task_leaves_the_list_and_the_next_task_opens() {
    let game = Game::new();
    game.reply(THREE_TASKS);

    click(&game, "Decline");

    assert_eq!(task_rows(&game)[0], "group: In progress");
    assert_eq!(lines(&game)[0], "heading: The Mill");
}

#[test]
fn a_task_belongs_to_the_zone_of_its_giver() {
    let game = Game::new();

    game.reply(concat!(
        r#"{"type":"journal","page":0,"pages":1,"#,
        r#""places":[{"name":"Tirisfal Glades","first_visit":1790000000},"#,
        r#"{"name":"Brill","within":"Tirisfal Glades","first_visit":1790000000}],"#,
        r#""people":[{"name":"Keeper Tessa","place":"Brill","first_met":1790000000}],"#,
        r#""quests":[{"number":1,"giver":"Keeper Tessa","title":"The Mill","status":"offered","steps":[]}]}"#,
    ));

    let zone: String = game.eval("ns.Journal.Page('quests').zone");
    assert_eq!(zone, "Tirisfal Glades");
}

fn bat_hunt(steps_done: usize, kills: u8) -> QuestView {
    let mut quest = lantern(Status::Accepted, steps_done);
    let steps = vec![
        quest.steps[0].step.clone(),
        Step::Kill {
            creature: "Duskbat".to_string(),
            count: 6,
        },
    ];
    quest.steps = step_views(steps, Status::Accepted, steps_done);
    if steps_done < 2 {
        quest.steps[1].kills = Some(kills);
    }
    quest
}

#[test]
fn a_kill_step_shows_its_kills_as_the_game_does() {
    let game = Game::new();

    game.reply(&quest_reply(bat_hunt(1, 2)));

    let shown = lines(&game);
    assert_eq!(shown[4], "entry: Visit Mill Pond. (Complete)");
    assert_eq!(shown[5], "entry: Duskbat slain: 2/6");
}

#[test]
fn a_done_kill_step_shows_all_its_kills() {
    let game = Game::new();

    game.reply(&quest_reply(bat_hunt(2, 0)));

    assert_eq!(lines(&game)[5], "entry: Duskbat slain: 6/6 (Complete)");
}

#[test]
fn a_kill_step_that_waits_shows_no_kills_yet_and_shows_faded() {
    let game = Game::new();

    game.reply(&quest_reply(bat_hunt(0, 0)));

    assert_eq!(lines(&game)[5], "later: Duskbat slain: 0/6");
}

#[test]
fn the_steps_of_an_offer_show_as_plain_steps() {
    let game = Game::new();

    game.reply(&quest_reply(lantern(Status::Offered, 0)));

    assert_eq!(lines(&game)[4], "entry: Visit Mill Pond.");
}
