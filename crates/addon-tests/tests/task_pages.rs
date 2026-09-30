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

/// Writes the text into the editor that is open, and saves it.
fn write(player: &Player, text: &str) {
    player.run(&format!("wow.EditBox():SetText('{text}') ns.Editor.Save()"));
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
            "note: New task.",
            "text: From Ada, your guild.",
            "prose: Put Gregor to rest.",
            "entry: Go to Agamand Mills.",
            "entry: Bring 10 Linen Cloth to Ada.",
            "entry: Turn in to Ada, face to face.",
            "section: Rewards",
            "text: This goes into your Chronicle, with Ada's name.",
            "text: 5 gold. Promised by Ada, paid by trade.",
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
            "group: Tasks I gave",
            "item: Give a task (Write one for a friend)",
        ]
    );
    assert_eq!(
        rows(&ada),
        [
            "group: Tasks I gave",
            "item: Give a task (Write one for a friend)",
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

    corvin.run("wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED')");
    click(&corvin, "Turn in");
    exchange(&ada, &corvin);

    assert_eq!(buttons(&corvin), ["Give up", "Turn in (off)"]);
    assert_eq!(
        lines(&corvin)[1],
        "note: Waiting for Ada to check it. Stand next to them."
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
    assert_eq!(buttons(&ada), ["Cancel task"]);
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

    assert_eq!(far, ["Not yet", "Complete task (off)"]);
    assert!(
        far_line
            .contains(&"hint: Corvin isn't next to you. Target them and stand close.".to_string())
    );
    assert_eq!(buttons(&ada), ["Not yet", "Complete task"]);
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
    click(&ada, "Complete task");

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
fn a_finished_task_goes_into_the_chronicle_of_the_doer_with_both_names() {
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
    assert!(page.contains(&"section: For your Chronicle".to_string()));
    assert!(page.contains(
        &"prose: Corvin finished Trouble at Agamand Mills for Ada. They met face to face in Brill to turn it in."
            .to_string()
    ));
}

#[test]
fn a_declined_task_leaves_the_list_of_the_doer() {
    let (_ada, corvin, id) = offered();

    corvin.run(&format!("ns.PlayerTasks.Decline('{}')", received_key(&id)));

    assert_eq!(
        rows(&corvin),
        [
            "group: Tasks I gave",
            "item: Give a task (Write one for a friend)"
        ]
    );
}

// The form ---------------------------------------------------------------------------------

fn form() -> (Player, Player) {
    let (ada, corvin) = ada_and_corvin();
    ada.run("ns.JournalFrame.Open('quests') ns.TaskForm.Open()");
    exchange(&ada, &corvin);
    (ada, corvin)
}

#[test]
fn the_form_asks_for_a_name_a_text_steps_and_a_player() {
    let (ada, _corvin) = form();

    let page = lines(&ada);

    assert_eq!(page[0], "heading: Give a task");
    assert!(page.contains(&"section: Name your task [Edit]".to_string()));
    assert!(page.contains(&"text: Go to Brill, where you stand. [Add]".to_string()));
    assert!(page.contains(&"text: Corvin (your guild) [Pick]".to_string()));
    assert!(page.contains(&"hint: Name your task first.".to_string()));
    assert_eq!(buttons(&ada), ["Cancel", "Send (off)"]);
}

#[test]
fn a_step_comes_from_the_target() {
    let (ada, _corvin) = form();

    ada.run(
        "wow.units.target = { name = 'Gregor Agamand', hostile = true }
         wow.Fire('PLAYER_TARGET_CHANGED')",
    );
    click_line(&ada, "Defeat Gregor Agamand", "Add");
    click_line(&ada, "Defeat Gregor Agamand", "Add");
    ada.run(
        "wow.units.target = { name = 'Innkeeper Renee' }
         wow.Fire('PLAYER_TARGET_CHANGED')",
    );
    click_line(&ada, "Talk to Innkeeper Renee", "Add");

    let page = lines(&ada);
    assert!(page.contains(&"entry: Defeat Gregor Agamand (2 times). [Remove]".to_string()));
    assert!(page.contains(&"entry: Talk to Innkeeper Renee. [Remove]".to_string()));
}

#[test]
fn an_item_step_takes_a_count_and_a_name() {
    let (ada, _corvin) = form();

    click_line(&ada, "Bring an item", "Add");
    write(&ada, "10 Linen Cloth");

    assert!(lines(&ada).contains(&"entry: Bring 10 Linen Cloth to you. [Remove]".to_string()));
}

#[test]
fn an_item_count_past_the_limit_adds_no_step() {
    let (ada, _corvin) = form();

    click_line(&ada, "Bring an item", "Add");
    write(&ada, "999 Linen Cloth");

    assert!(!lines(&ada).iter().any(|line| line.contains("Linen Cloth")));
}

#[test]
fn a_task_holds_at_most_five_steps_and_the_turn_in() {
    let (ada, _corvin) = form();

    for n in 1..=6 {
        click_line(&ada, "Bring an item", "Add");
        write(&ada, &format!("1 Item {n}"));
    }

    let steps = lines(&ada)
        .iter()
        .filter(|line| line.ends_with("[Remove]"))
        .count();
    assert_eq!(steps, 5);
}

#[test]
fn a_written_task_goes_to_the_picked_player() {
    let (ada, corvin) = form();
    click_line(&ada, "Name your task", "Edit");
    write(&ada, "The |cffff0000Red|r Mill");
    click_line(&ada, "What should they do?", "Edit");
    write(&ada, "Look around the mill.");
    click_line(&ada, "Go to Brill", "Add");
    click_line(&ada, "Corvin", "Pick");

    click(&ada, "Send");
    exchange(&ada, &corvin);

    let title: String =
        corvin.eval("local _, entry = next(ns.TaskStore.Data().received) return entry.title");
    assert_eq!(title, "The  cffff0000Red r Mill");
    assert_eq!(lines(&ada)[0], "heading: The  cffff0000Red r Mill");
}

#[test]
fn cancel_throws_the_draft_away() {
    let (ada, _corvin) = form();
    click_line(&ada, "Name your task", "Edit");
    write(&ada, "A draft");

    click(&ada, "Cancel");
    ada.run("ns.TaskForm.Open()");

    assert!(lines(&ada).contains(&"hint: No name yet.".to_string()));
}

#[test]
fn the_help_to_write_stays_hidden_while_the_relay_has_no_draft_reply() {
    let (ada, _corvin) = form();

    assert!(
        !buttons(&ada)
            .iter()
            .any(|label| label.starts_with("Help me write this"))
    );
}

#[test]
fn an_idea_for_the_model_loses_the_names_of_players() {
    let (ada, _corvin) = form();

    let idea: String = ada.eval(
        "ns.TaskDraftHelp.WithoutNames('get corvin to kill Gregor for Ada, tell Corvin now')",
    );

    assert_eq!(idea, "get corvin to kill Gregor for $N, tell my friend now");
}

#[test]
fn a_suggestion_of_the_model_changes_nothing_until_the_player_picks_it() {
    let (ada, _corvin) = form();
    ada.run("ns.TaskDraftHelp.enabled = true");
    click(&ada, "Help me write this");
    write(&ada, "kill gregor");

    ada.game.reply(
        r#"{"type":"draft_answer","title":"Trouble at the Mills","text":"Put Gregor to rest.","steps":[{"kind":"kill","target":"Gregor Agamand","count":1}]}"#,
    );
    let before = lines(&ada);
    click_line(&ada, "Suggestion", "Use this");

    assert!(before.contains(&"hint: No name yet.".to_string()));
    assert!(before.contains(&"entry: Defeat Gregor Agamand.".to_string()));
    let after = lines(&ada);
    assert!(after.contains(&"text: Trouble at the Mills".to_string()));
    assert!(after.contains(&"entry: Defeat Gregor Agamand. [Remove]".to_string()));
    let asked = ada
        .game
        .sent()
        .iter()
        .any(|batch| batch.contains(r#""idea":"kill gregor""#));
    assert!(asked);
}

#[test]
fn a_suggestion_that_breaks_a_rule_is_dropped_whole() {
    let (ada, _corvin) = form();
    ada.run("ns.TaskDraftHelp.enabled = true");

    ada.game
        .reply(r#"{"type":"draft_answer","title":"Fine","text":"Also |Hfine|h.","steps":[]}"#);

    assert!(!lines(&ada).contains(&"section: Suggestion [Use this]".to_string()));
}

#[test]
fn a_click_on_give_a_task_in_the_book_opens_the_form() {
    let (ada, _corvin) = ada_and_corvin();
    ada.run("wow.Slash('/journal', '') ns.JournalFrame.Open('quests')");

    ada.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.title and widget.title.text == 'Give a task' then
                 widget:Click()
             end
         end",
    );

    let shown: Vec<String> =
        ada.eval("wow.ShownTexts(TimewaysJournalFrameScroll:GetScrollChild())");
    assert_eq!(shown[0], "Give a task");
    assert!(!ada.eval::<bool>("wow.Button('Send').enabled"));
}
