//! The pages of player tasks in the Tasks section of the journal: the form, a task that you
//! got, and a task that you gave with its turn-in card (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, accepted_task, ada_and_corvin, exchange, received_key};

const TWO_STEPS: &str = "{ { kind = 'place', target = 'Agamand Mills', count = 1 },
                           { kind = 'item', target = 'Linen Cloth', count = 10 } }";

/// The parchment of the Tasks page as text: one `style: text` entry for each line.
fn lines(player: &Player) -> Vec<String> {
    player.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Page('quests').lines) do
             local action = line.action and (' [' .. line.action.label .. ']') or ''
             table.insert(out, line.style .. ': ' .. line.text .. action)
         end
         return out",
    )
}

/// The rows of the list, as `style: text (detail) [mark]`.
fn rows(player: &Player) -> Vec<String> {
    player.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('quests').list) do
             local detail = row.detail and (' (' .. row.detail .. ')') or ''
             local mark = row.mark and (' [' .. row.mark .. ']') or ''
             table.insert(out, row.style .. ': ' .. row.text .. detail .. mark)
         end
         return out",
    )
}

/// The buttons at the bottom, with `(off)` after a disabled one.
fn buttons(player: &Player) -> Vec<String> {
    player.eval(
        "local out = {}
         for _, button in ipairs(ns.Journal.Page('quests').buttons) do
             table.insert(out, button.label .. (button.disabled and ' (off)' or ''))
         end
         return out",
    )
}

fn click(player: &Player, label: &str) {
    player.run(&format!(
        "for _, button in ipairs(ns.Journal.Page('quests').buttons) do
             if button.label == '{label}' then button.run() end
         end"
    ));
}

/// Clicks the button on the line that starts with `text`.
fn click_line(player: &Player, text: &str, label: &str) {
    player.run(&format!(
        "for _, line in ipairs(ns.Journal.Page('quests').lines) do
             if line.text:sub(1, #'{text}') == '{text}' and line.action and line.action.label == '{label}' then
                 line.action.run()
                 return
             end
         end
         error('no line {text} with {label}')"
    ));
}

fn open(player: &Player, key: &str) {
    player.run(&format!("ns.Journal.Select('quests', '{key}')"));
}

fn offered() -> (Player, Player, String) {
    let (ada, corvin) = ada_and_corvin();
    let id: String = ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'Trouble at Agamand Mills', text = 'Put Gregor to rest.',
             reward = '5 gold', steps = {TWO_STEPS} }}, 'Corvin-Stormrage')"
    ));
    exchange(&ada, &corvin);
    (ada, corvin, id)
}

#[test]
fn the_doer_sees_an_offer_with_its_steps_its_rewards_and_three_answers() {
    let (_ada, corvin, id) = offered();

    open(&corvin, &format!("got:{}", received_key(&id)));

    assert_eq!(
        lines(&corvin),
        [
            "heading: Trouble at Agamand Mills",
            "note: New quest.",
            "text: From Ada, your guild.",
            "prose: Put Gregor to rest.",
            "entry: Go to Agamand Mills.",
            "entry: Bring 10 Linen Cloth to Ada.",
            "entry: Turn in to Ada, face to face.",
            "section: Rewards",
            "text: A line about it in your journal, with Ada's name.",
            "text: 5 gold. Ada pays it in a trade.",
            "help: To report abuse, open Support in the game menu.",
        ]
    );
    assert_eq!(buttons(&corvin), ["Block player", "Decline", "Accept"]);
}

#[test]
fn the_list_holds_tasks_from_players_and_the_tasks_that_you_gave() {
    let (ada, corvin, _id) = offered();

    assert_eq!(
        rows(&corvin),
        [
            "group: From players",
            "item: Trouble at Agamand Mills (From Ada) [New]",
            "group: Quests you wrote",
            "item: New quest",
        ]
    );
    assert_eq!(
        rows(&ada),
        [
            "group: Quests you wrote",
            "item: New quest",
            "item: Trouble at Agamand Mills (To Corvin. Reward: promised) [Waiting]",
        ]
    );
}

#[test]
fn a_task_from_a_player_opens_first_when_the_npcs_gave_none() {
    let (_ada, corvin, _id) = offered();

    let first = lines(&corvin);

    assert_eq!(first[0], "heading: Trouble at Agamand Mills");
}

#[test]
fn player_tasks_show_while_the_desktop_is_away() {
    let (_ada, corvin, id) = offered();

    open(&corvin, &format!("got:{}", received_key(&id)));

    let has_pages: bool =
        corvin.eval("ns.Journal.Page('deeds').lines[1].text:find('Loading') ~= nil");
    assert!(has_pages, "the desktop never answered in this test");
    assert_eq!(lines(&corvin)[0], "heading: Trouble at Agamand Mills");
}

#[test]
fn accept_on_the_page_takes_the_task() {
    let (ada, corvin, id) = offered();
    open(&corvin, &format!("got:{}", received_key(&id)));

    click(&corvin, "Accept");
    exchange(&ada, &corvin);

    assert_eq!(buttons(&corvin), ["Give up", "Turn in (off)"]);
    let status: String = ada.eval(&format!("ns.TaskStore.Data().given['{id}'].status"));
    assert_eq!(status, "accepted");
}

#[test]
fn turn_in_waits_until_every_step_is_done() {
    let (ada, corvin, id) =
        accepted_task("{ { kind = 'place', target = 'Agamand Mills', count = 1 } }");
    open(&corvin, &format!("got:{}", received_key(&id)));
    let before = buttons(&corvin);

    corvin.run("wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED')");
    click(&corvin, "Turn in");
    exchange(&ada, &corvin);

    assert_eq!(before, ["Give up", "Turn in (off)"]);
    assert_eq!(
        lines(&corvin)[1],
        "note: Waiting for Ada to check it. Stand next to them."
    );
}

#[test]
fn the_doer_can_ask_again_while_the_giver_has_not_answered() {
    let (ada, corvin, id) = accepted_task("{ { kind = 'place', target = 'Brill', count = 1 } }");
    open(&corvin, &format!("got:{}", received_key(&id)));

    click(&corvin, "Turn in");
    exchange(&ada, &corvin);

    assert_eq!(buttons(&corvin), ["Give up", "Turn in"]);
}

#[test]
fn a_finished_task_shows_as_done_in_the_list_of_the_doer() {
    let (ada, corvin, id) = accepted_task("{ { kind = 'place', target = 'Brill', count = 1 } }");
    corvin.run(&format!(
        "ns.PlayerTasks.AskTurnIn('{}')",
        received_key(&id)
    ));
    exchange(&ada, &corvin);

    ada.run("wow.units.target = { name = 'Corvin', player = true, near = true }");
    ada.run(&format!("ns.PlayerTasks.Complete('{id}')"));
    exchange(&ada, &corvin);

    assert!(
        rows(&corvin).contains(&"item: Trouble at Agamand Mills (From Ada) [Done]".to_string())
    );
}

#[test]
fn the_turn_in_lines_show_only_on_an_accepted_task() {
    let (ada, _corvin, id) = offered();

    open(&ada, &format!("gave:{id}"));

    assert!(
        !lines(&ada)
            .iter()
            .any(|line| line.contains("Turn in to you"))
    );
}

#[test]
fn the_doer_sees_the_count_of_a_step_as_it_grows() {
    let (_ada, corvin, id) = accepted_task(TWO_STEPS);
    open(&corvin, &format!("got:{}", received_key(&id)));

    corvin.run(
        "wow.Trade('Ada', { gave = { { name = 'Linen Cloth', count = 4 } }, got = {}, money = 0, moneyGot = 0 })",
    );

    assert!(lines(&corvin).contains(&"entry: (4 of 10) Bring 10 Linen Cloth to Ada.".to_string()));
}

#[test]
fn the_giver_sees_each_step_with_how_sure_it_is() {
    let (ada, corvin, id) = accepted_task(TWO_STEPS);
    corvin.run("wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED')");
    exchange(&ada, &corvin);
    open(&ada, &format!("gave:{id}"));

    let page = lines(&ada);

    assert_eq!(page[0], "heading: Trouble at Agamand Mills");
    assert_eq!(page[2], "section: What they did");
    assert_eq!(page[3], "entry: Go to Agamand Mills.");
    assert!(
        page[4].starts_with("hint: Seen: only their addon recorded it. "),
        "{}",
        page[4]
    );
    assert_eq!(page[5], "entry: Bring 10 Linen Cloth to you.");
    assert_eq!(page[6], "hint: Not done yet.");
    assert_eq!(buttons(&ada), ["Cancel quest"]);
}

#[test]
fn the_turn_in_card_offers_complete_only_face_to_face() {
    let (ada, corvin, id) =
        accepted_task("{ { kind = 'place', target = 'Agamand Mills', count = 1 } }");
    corvin.run("wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED')");
    corvin.run(&format!(
        "ns.PlayerTasks.AskTurnIn('{}')",
        received_key(&id)
    ));
    exchange(&ada, &corvin);
    open(&ada, &format!("gave:{id}"));

    let far = buttons(&ada);
    let far_line = lines(&ada);
    ada.run("wow.units.target = { name = 'Corvin', player = true, near = true }");

    assert_eq!(far, ["Not yet", "Complete quest (off)"]);
    assert!(
        far_line
            .contains(&"hint: Corvin isn't next to you. Target them and stand close.".to_string())
    );
    assert_eq!(buttons(&ada), ["Not yet", "Complete quest"]);
    assert!(lines(&ada).contains(&"hint: Face to face now.".to_string()));
}

#[test]
fn the_reward_line_follows_the_trade_window() {
    let (ada, corvin, id) = accepted_task("{ { kind = 'place', target = 'Brill', count = 1 } }");
    corvin.run(&format!(
        "ns.PlayerTasks.AskTurnIn('{}')",
        received_key(&id)
    ));
    exchange(&ada, &corvin);
    open(&ada, &format!("gave:{id}"));
    let promised = lines(&ada);

    ada.run(
        "wow.Trade('Corvin', { gave = {}, got = {}, money = 50000, moneyGot = 0 })
         wow.units.target = { name = 'Corvin', player = true, near = true }",
    );
    click(&ada, "Complete quest");

    assert!(promised.contains(&"text: Reward: promised. 5 gold.".to_string()));
    assert!(lines(&ada).contains(&"text: Reward: paid in trade. 5 gold.".to_string()));
}

#[test]
fn a_task_done_with_no_trade_shows_the_reward_as_not_paid() {
    let (ada, corvin, id) = accepted_task("{ { kind = 'place', target = 'Brill', count = 1 } }");
    corvin.run(&format!(
        "ns.PlayerTasks.AskTurnIn('{}')",
        received_key(&id)
    ));
    exchange(&ada, &corvin);

    ada.run("wow.units.target = { name = 'Corvin', player = true, near = true }");
    ada.run(&format!("ns.PlayerTasks.Complete('{id}')"));

    let detail: Vec<String> = rows(&ada);
    assert!(detail.contains(
        &"item: Trouble at Agamand Mills (To Corvin. Reward: not paid) [Done]".to_string()
    ));
}

#[test]
fn a_finished_task_leaves_a_line_in_the_journal_of_the_doer_with_both_names() {
    let (ada, corvin, id) = accepted_task("{ { kind = 'place', target = 'Brill', count = 1 } }");
    corvin.run(&format!(
        "ns.PlayerTasks.AskTurnIn('{}')",
        received_key(&id)
    ));
    exchange(&ada, &corvin);
    ada.run("wow.units.target = { name = 'Corvin', player = true, near = true }");
    ada.run(&format!("ns.PlayerTasks.Complete('{id}')"));
    exchange(&ada, &corvin);

    open(&corvin, &format!("got:{}", received_key(&id)));

    let page = lines(&corvin);
    assert!(page.contains(&"section: Your story".to_string()));
    assert!(page.contains(
        &"prose: Corvin finished Trouble at Agamand Mills for Ada. They met face to face in Brill to turn it in."
            .to_string()
    ));
}

#[test]
fn block_player_asks_first() {
    let (_ada, corvin, id) = offered();
    open(&corvin, &format!("got:{}", received_key(&id)));

    click(&corvin, "Block player");
    let before: bool = corvin.eval("next(ns.TaskStore.Data().blocked) ~= nil");
    corvin.run("wow.AcceptPopup()");

    assert!(!before);
    let blocked: bool = corvin.eval("ns.TaskStore.Data().blocked['Ada-Stormrage'] ~= nil");
    assert!(blocked);
}

#[test]
fn a_blocked_player_can_be_unblocked_from_the_list() {
    let (ada, corvin, id) = offered();
    corvin.run(&format!("ns.PlayerTasks.Block('{}')", received_key(&id)));
    exchange(&ada, &corvin);
    open(&corvin, "blocked");
    let page = lines(&corvin);

    click_line(&corvin, "Ada", "Unblock");
    ada.run("ns.PlayerTasks.Call()");
    exchange(&ada, &corvin);
    let again: Option<String> = ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'Again', text = 'Please.', reward = '',
             steps = {TWO_STEPS} }}, 'Corvin-Stormrage')"
    ));

    assert!(
        page.contains(&"text: Ada [Unblock]".to_string()),
        "{page:?}"
    );
    assert!(again.is_some());
    assert!(
        rows(&corvin)
            .iter()
            .all(|row| !row.contains("Blocked players"))
    );
}

#[test]
fn a_declined_task_leaves_the_list_of_the_doer() {
    let (_ada, corvin, id) = offered();

    corvin.run(&format!("ns.PlayerTasks.Decline('{}')", received_key(&id)));

    assert_eq!(
        rows(&corvin),
        ["group: Quests you wrote", "item: New quest"]
    );
}
