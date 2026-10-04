//! `/quest`, and the Quests page of the book (GAMEPLAY.md 3.4 and 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Game, step_views};
use hourglass::Tick;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::{Deed, Journal, pages};
use timeways_story::quest::{AnyOrder, QuestView, Status, Step, StepState, TimeOfDay};
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
        any_order: None,
        has_slap: false,
        hidden_steps: 0,
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

/// The line of the second step of the lantern quest, with this step in its place.
fn second_step_line(step: Step) -> String {
    let game = Game::new();
    let mut quest = lantern(Status::Accepted, 1);
    quest.steps[1].step = step;
    game.reply(&quest_reply(quest));
    lines(&game).remove(5)
}

#[test]
fn a_talk_step_names_the_command_to_use() {
    let plain = Step::Talk {
        npc: "Farmer Bram".to_string(),
        about: None,
    };
    let topic = Step::Talk {
        npc: "Farmer Bram".to_string(),
        about: Some("the missing cask".to_string()),
    };

    assert_eq!(
        second_step_line(plain),
        "entry: Talk to Farmer Bram (/talk)."
    );
    assert_eq!(
        second_step_line(topic),
        "entry: Ask Farmer Bram about the missing cask (/talk)."
    );
}

/// The lantern quest with a wait of 2 days as its second step, open while `left` seconds
/// remain.
fn wait_line(state: StepState, left: i64) -> String {
    let game = Game::new();
    let mut quest = lantern(Status::Accepted, 1);
    quest.steps[1].step = Step::Wait { days: 2 };
    quest.steps[1].state = state;
    let now = i64::try_from(DAY).unwrap();
    quest.steps[1].ready_at =
        (state == StepState::Open).then(|| Tick(u64::try_from(now + left).unwrap()));
    game.reply(&quest_reply(quest));
    lines(&game).remove(5)
}

#[test]
fn an_open_wait_shows_the_time_left_in_days_and_hours() {
    assert_eq!(
        wait_line(StepState::Open, 30 * 3600),
        "entry: Wait 2 days: 1 day left."
    );
    assert_eq!(
        wait_line(StepState::Open, 5 * 3600 + 59),
        "entry: Wait 2 days: 5 hours left."
    );
    assert_eq!(
        wait_line(StepState::Open, 3599),
        "entry: Wait 2 days: less than an hour left."
    );
}

#[test]
fn a_wait_whose_time_passed_shows_complete() {
    assert_eq!(
        wait_line(StepState::Open, 0),
        "entry: Wait 2 days. (Complete)"
    );
    assert_eq!(
        wait_line(StepState::Done, 0),
        "entry: Wait 2 days. (Complete)"
    );
}

#[test]
fn a_wait_that_is_not_open_yet_shows_faded() {
    assert_eq!(wait_line(StepState::Later, 0), "later: Wait 2 days.");
}

#[test]
fn an_any_order_set_shows_under_in_any_order() {
    let game = Game::new();
    let mut quest = lantern(Status::Accepted, 0);
    quest.steps[1].state = StepState::Open;
    quest.any_order = Some(AnyOrder { first: 0, last: 1 });

    game.reply(&quest_reply(quest));

    let shown = lines(&game);
    assert_eq!(shown[4], "text: In any order:");
    assert_eq!(shown[5], "entry: Visit Mill Pond.");
    assert_eq!(shown[6], "entry: Speak with Farmer Bram.");
}

fn cloth_step() -> Step {
    Step::Carry {
        item: "Linen Cloth".to_string(),
        count: 10,
        npc: "Farmer Bram".to_string(),
    }
}

#[test]
fn a_carry_step_shows_the_count_in_your_bags() {
    let game = Game::new();
    game.run("wow.bags['Linen Cloth'] = 6");
    let mut quest = lantern(Status::Accepted, 1);
    quest.steps[1].step = cloth_step();

    game.reply(&quest_reply(quest));

    assert_eq!(
        lines(&game)[5],
        "entry: Bring Linen Cloth to Farmer Bram: 6/10"
    );
}

#[test]
fn the_page_draws_again_when_the_bags_change() {
    let game = Game::new();
    let mut quest = lantern(Status::Accepted, 1);
    quest.steps[1].step = cloth_step();
    game.reply(&quest_reply(quest));
    game.run("ns.JournalFrame.Open('quests')");

    game.run("wow.bags['Linen Cloth'] = 12; wow.Fire('BAG_UPDATE_DELAYED')");

    let shown: Vec<String> = game.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'FontString' and widget.shown and widget.text then
                 table.insert(out, widget.text)
             end
         end
         return out",
    );
    assert!(
        shown
            .iter()
            .any(|text| text == "Bring Linen Cloth to Farmer Bram: 10/10"),
        "{shown:?}"
    );
}

#[test]
fn an_emote_or_a_slap_step_names_the_command_to_type() {
    let bow = Step::Emote {
        emote: "bow".to_string(),
        npc: Some("Sister Ada".to_string()),
        place: None,
    };
    let dance = Step::Emote {
        emote: "dance".to_string(),
        npc: None,
        place: Some("Goldshire".to_string()),
    };
    let slap = Step::Slap {
        npc: "Farmer Bram".to_string(),
    };

    assert_eq!(second_step_line(bow), "entry: Use /bow on Sister Ada.");
    assert_eq!(second_step_line(dance), "entry: Use /dance in Goldshire.");
    assert_eq!(second_step_line(slap), "entry: Use /slap on Farmer Bram.");
}

#[test]
fn a_quest_with_a_slap_says_the_npc_will_like_you_less() {
    let game = Game::new();
    let mut quest = lantern(Status::Offered, 0);
    quest.steps[1].step = Step::Slap {
        npc: "Farmer Bram".to_string(),
    };
    quest.has_slap = true;

    game.reply(&quest_reply(quest));

    let shown = lines(&game);
    let rewards = shown
        .iter()
        .position(|line| line == "section: Rewards")
        .unwrap();
    assert_eq!(shown[rewards + 2], "text: Farmer Bram will like you less.");
}

#[test]
fn a_time_step_names_its_hours() {
    let night = Step::VisitAt {
        place: "Old Mill".to_string(),
        time: TimeOfDay::Night,
    };

    assert_eq!(
        second_step_line(night),
        "entry: Visit Old Mill at night (9 PM to 5 AM)."
    );
}

#[test]
fn a_level_a_dungeon_a_boss_and_a_game_quest_show_their_lines() {
    let enter = Step::Enter {
        dungeon: "The Deadmines".to_string(),
    };
    let defeat = Step::Defeat {
        boss: "Edwin VanCleef".to_string(),
    };
    let turn_in = Step::GameQuest {
        title: "The Defias Brotherhood".to_string(),
    };

    assert_eq!(
        second_step_line(Step::Level { level: 14 }),
        "entry: Reach level 14."
    );
    assert_eq!(second_step_line(enter), "entry: Enter The Deadmines.");
    assert_eq!(second_step_line(defeat), "entry: Edwin VanCleef slain: 0/1");
    assert_eq!(
        second_step_line(turn_in),
        "entry: Complete \"The Defias Brotherhood\"."
    );
}

#[test]
fn a_mystery_shows_more_to_come_and_counts_only_its_done_steps() {
    let game = Game::new();
    let mut quest = lantern(Status::Accepted, 1);
    quest.hidden_steps = 2;

    game.reply(&quest_reply(quest));

    let shown = lines(&game);
    assert_eq!(shown[6], "later: More to come.");
    let mark: String = game.eval("ns.Journal.Page('quests').list[2].mark");
    assert_eq!(mark, "1 done");
}
