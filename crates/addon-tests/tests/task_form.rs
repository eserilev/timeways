//! The form to write a task for another player (GAMEPLAY.md 4.7): typed steps that a model
//! checks, a reward of money and items, and a task that you save and send now or later.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, ada_and_corvin, exchange, received_key};

/// The parchment of the Tasks page as text: one `style: text [button]` entry for each line.
fn lines(player: &Player) -> Vec<String> {
    player.eval(
        "local out = {}
         for _, line in ipairs(ns.Journal.Page('quests').lines) do
             local action = ''
             if line.action then
                 action = ' [' .. line.action.label .. (line.action.disabled and ' (off)' or '') .. ']'
             end
             table.insert(out, line.style .. ': ' .. (line.text or '') .. action)
         end
         return out",
    )
}

fn rows(player: &Player) -> Vec<String> {
    player.eval(
        "local out = {}
         for _, row in ipairs(ns.Journal.Page('quests').list) do
             local detail = row.detail and (' (' .. row.detail .. ')') or ''
             table.insert(out, row.style .. ': ' .. row.text .. detail)
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

fn footer(player: &Player) -> String {
    player.eval("ns.Journal.Page('quests').footer")
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
             local starts = (line.text or ''):sub(1, #'{text}') == '{text}'
             if starts and line.action and line.action.label == '{label}' then
                 line.action.run()
                 return
             end
         end
         error('no line {text} with {label}')"
    ));
}

/// Types into the box whose empty text is `placeholder`, as the player does.
fn type_in(player: &Player, placeholder: &str, text: &str) {
    player.run(&format!(
        "for _, line in ipairs(ns.Journal.Page('quests').lines) do
             if line.field and line.text == '{placeholder}' then
                 line.field.change('{text}')
                 return
             end
         end
         error('no box {placeholder}')"
    ));
}

/// Runs a function of the input of the first line with this style, such as `money.change`.
fn use_input(player: &Player, style: &str, call: &str) {
    player.run(&format!(
        "for _, line in ipairs(ns.Journal.Page('quests').lines) do
             if line.style == '{style}' then
                 local input = line.{style}
                 input.{call}
                 return
             end
         end
         error('no {style}')"
    ));
}

/// Writes the text into the editor that is open, and saves it.
fn write(player: &Player, text: &str) {
    player.run(&format!(
        "wow.MultiLineBox():SetText('{text}') ns.Editor.Save()"
    ));
}

/// Picks up a stack from the bags and drops it on a slot of the reward.
fn drop_item(player: &Player, id: u32, name: &str, count: u32) {
    player.run(&format!(
        "wow.cursor = {{ id = {id}, name = '{name}', count = {count} }}"
    ));
    use_input(player, "slots", "drop()");
}

fn form() -> (Player, Player) {
    let (ada, corvin) = ada_and_corvin();
    ada.run("ns.JournalFrame.Open('quests') ns.TaskForm.Open()");
    exchange(&ada, &corvin);
    (ada, corvin)
}

fn desktop_away(player: &Player) {
    player.run("ns.Messages.Bridge = function() return 'offline' end");
}

/// A step typed with the desktop away: it stays as written, at once.
fn add_written_step(player: &Player, text: &str) {
    type_in(player, "Add a step, like: kill 5 bats", text);
    click_line(player, "Add a step", "Add");
}

fn asked_ideas(player: &Player) -> Vec<String> {
    player
        .game
        .sent()
        .iter()
        .flat_map(|batch| batch.lines().map(str::to_string).collect::<Vec<_>>())
        .filter(|line| line.contains("draft_asked"))
        .collect()
}

fn draft_answer(steps: &str) -> String {
    format!(
        r#"{{"type":"draft_answer","id":1,"draft":{{"title":"Bats","text":"Go.","steps":{steps}}}}}"#
    )
}

// The page ----------------------------------------------------------------------------------

#[test]
fn the_form_asks_for_a_title_steps_a_reward_and_a_player() {
    let (ada, _corvin) = form();

    assert_eq!(
        lines(&ada),
        [
            "field: Title",
            "help: Description (optional) [Add]",
            "section: Steps",
            "entry: Turn in to you, face to face.",
            "field: Add a step, like: kill 5 bats [Add]",
            "section: Reward",
            "money: ",
            "slots: ",
            "help: You trade it to them at turn-in.",
            "section: Send to",
            "text: Corvin (guild) [Pick]",
            "help: Like chat, Blizzard can read what you send.",
        ]
    );
    assert_eq!(
        buttons(&ada),
        ["Help me write", "Cancel", "Save (off)", "Send (off)"]
    );
    assert_eq!(footer(&ada), "Add a title.");
}

#[test]
fn with_nobody_to_send_to_the_form_says_who_shows_up() {
    let ada = Player::new("Ada");
    ada.run("ns.JournalFrame.Open('quests') ns.TaskForm.Open()");

    assert!(
        lines(&ada)
            .contains(&"help: Party, guild, and friends with Timeways show up here.".to_string())
    );
}

#[test]
fn send_needs_a_title_a_step_and_a_player() {
    let (ada, _corvin) = form();
    desktop_away(&ada);

    type_in(&ada, "Title", "The Lost Lantern");
    let titled = footer(&ada);
    add_written_step(&ada, "Find my lantern");
    let stepped = footer(&ada);
    click_line(&ada, "Corvin", "Pick");

    assert_eq!(titled, "Add a step.");
    assert_eq!(stepped, "Pick who gets it.");
    assert_eq!(footer(&ada), "");
    assert_eq!(buttons(&ada), ["Help me write", "Cancel", "Save", "Send"]);
}

#[test]
fn a_title_too_long_in_bytes_keeps_what_the_player_typed() {
    let (ada, _corvin) = form();
    let title = format!("a{}", "\u{e9}".repeat(30));

    type_in(&ada, "Title", &title);

    assert!(lines(&ada).contains(&"hint: Too long. Try a shorter title.".to_string()));
    assert_eq!(ada.eval::<String>("ns.TaskForm.Draft().title"), title);
    assert_eq!(footer(&ada), "The title is too long.");
}

#[test]
fn the_description_is_optional_and_opens_in_the_editor() {
    let (ada, _corvin) = form();

    click_line(&ada, "Description", "Add");
    write(&ada, "Look around the mill.");

    assert!(lines(&ada).contains(&"prose: Look around the mill. [Edit]".to_string()));
}

// Steps --------------------------------------------------------------------------------------

#[test]
fn a_typed_step_goes_to_the_model_and_shows_as_a_step_the_game_checks() {
    let (ada, _corvin) = form();
    type_in(&ada, "Add a step, like: kill 5 bats", "kill 5 bats");

    click_line(&ada, "Add a step", "Add");
    let checking = lines(&ada);
    ada.game
        .reply(&draft_answer(r#"[{"goal":"kill","target":"5 Duskbat"}]"#));

    assert!(checking.contains(&"entry: kill 5 bats.".to_string()));
    assert!(checking.contains(&"hint: Checking...".to_string()));
    let ideas = asked_ideas(&ada);
    assert_eq!(ideas.len(), 1);
    assert!(ideas[0].contains("kill 5 bats"), "{ideas:?}");
    assert!(lines(&ada).contains(&"entry: Defeat Duskbat (5 times). [Remove]".to_string()));
}

#[test]
fn the_add_button_waits_while_a_step_is_checked() {
    let (ada, _corvin) = form();
    type_in(&ada, "Add a step, like: kill 5 bats", "kill 5 bats");

    click_line(&ada, "Add a step", "Add");

    assert!(lines(&ada).contains(&"field: Add a step, like: kill 5 bats [Add (off)]".to_string()));
    assert_eq!(buttons(&ada)[0], "Help me write (off)");
}

#[test]
fn a_step_that_the_model_cannot_check_stays_as_written() {
    let (ada, _corvin) = form();
    type_in(&ada, "Add a step, like: kill 5 bats", "find my lost ring");
    click_line(&ada, "Add a step", "Add");

    ada.game
        .reply(r#"{"type":"draft_answer","id":1,"draft":null}"#);

    let page = lines(&ada);
    assert!(page.contains(&"entry: find my lost ring. [Remove]".to_string()));
    assert!(page.contains(&"hint: You check this one at turn-in.".to_string()));
}

#[test]
fn with_the_desktop_away_a_typed_step_stays_as_written_at_once() {
    let (ada, _corvin) = form();
    desktop_away(&ada);

    add_written_step(&ada, "find my lost ring");

    assert!(asked_ideas(&ada).is_empty());
    assert!(lines(&ada).contains(&"entry: find my lost ring. [Remove]".to_string()));
}

#[test]
fn a_typed_step_marks_the_names_of_players_for_the_story_program() {
    let (ada, _corvin) = form();
    type_in(
        &ada,
        "Add a step, like: kill 5 bats",
        "help Corvin at the mill",
    );

    click_line(&ada, "Add a step", "Add");

    let ideas = asked_ideas(&ada);
    assert!(ideas[0].contains("help {Corvin} at the mill"), "{ideas:?}");
}

#[test]
fn a_step_too_long_keeps_what_the_player_typed() {
    let (ada, _corvin) = form();
    let step = "a".repeat(97);
    type_in(&ada, "Add a step, like: kill 5 bats", &step);

    click_line(&ada, "Add a step", "Add");

    assert!(lines(&ada).contains(&"hint: Too long. Try a shorter step.".to_string()));
    assert_eq!(ada.eval::<String>("ns.TaskForm.Draft().stepText"), step);
    assert!(asked_ideas(&ada).is_empty());
}

#[test]
fn a_task_holds_at_most_five_steps_and_the_turn_in() {
    let (ada, _corvin) = form();
    desktop_away(&ada);

    for n in 1..=5 {
        add_written_step(&ada, &format!("step {n}"));
    }

    let page = lines(&ada);
    assert_eq!(
        page.iter()
            .filter(|line| line.ends_with("[Remove]"))
            .count(),
        5
    );
    assert!(
        !page
            .iter()
            .any(|line| line.starts_with("field: Add a step"))
    );
}

#[test]
fn the_same_foe_from_two_typed_steps_becomes_one_step() {
    let (ada, _corvin) = form();
    for count in [3, 2] {
        type_in(&ada, "Add a step, like: kill 5 bats", "kill rats");
        click_line(&ada, "Add a step", "Add");
        ada.game.reply(&draft_answer(&format!(
            r#"[{{"goal":"kill","target":"{count} Rat"}}]"#
        )));
    }

    let page = lines(&ada);
    let steps: Vec<&String> = page
        .iter()
        .filter(|line| line.starts_with("entry: Defeat"))
        .collect();
    assert_eq!(steps, ["entry: Defeat Rat (5 times). [Remove]"]);
}

// Reward -------------------------------------------------------------------------------------

#[test]
fn the_reward_is_money_and_items_from_the_bags() {
    let (ada, corvin) = form();
    desktop_away(&ada);
    type_in(&ada, "Title", "The Lost Lantern");
    add_written_step(&ada, "find my lantern");
    click_line(&ada, "Corvin", "Pick");

    use_input(&ada, "money", "change('5', '20', '')");
    drop_item(&ada, 2589, "Linen Cloth", 10);
    drop_item(&ada, 2589, "Linen Cloth", 5);
    drop_item(&ada, 858, "Lesser Healing Potion", 1);
    click(&ada, "Send");
    exchange(&ada, &corvin);

    let reward: String =
        corvin.eval("local _, task = next(ns.TaskStore.Data().received) return task.reward");
    assert_eq!(
        reward,
        "5 gold 20 silver, 15 Linen Cloth, Lesser Healing Potion"
    );
    let cursor: bool = ada.eval("wow.cursor == nil");
    assert!(cursor, "the item goes back to the bags");
}

#[test]
fn a_right_click_takes_an_item_out_of_the_reward() {
    let (ada, _corvin) = form();
    drop_item(&ada, 2589, "Linen Cloth", 10);

    use_input(&ada, "slots", "remove(1)");

    assert_eq!(ada.eval::<usize>("#ns.TaskForm.Draft().items"), 0);
}

#[test]
fn a_reward_holds_at_most_six_items() {
    let (ada, _corvin) = form();

    for id in 1..=7 {
        drop_item(&ada, id, &format!("Gem {id}"), 1);
    }

    assert_eq!(ada.eval::<usize>("#ns.TaskForm.Draft().items"), 6);
    assert!(
        ada.printed()
            .iter()
            .any(|line| line.contains("That's all the reward a quest can hold."))
    );
}

#[test]
fn each_money_box_holds_what_its_coin_can() {
    let ada = Player::new("Ada");

    let copper: u64 = ada.eval("ns.TaskReward.Copper(100000, 150, -3)");
    let text: String = ada.eval("ns.TaskReward.Text(ns.TaskReward.Copper(0, 0, 7), {})");

    assert_eq!(copper, 99_999 * 10_000 + 99 * 100);
    assert_eq!(text, "7 copper");
}

// Save now, send later -----------------------------------------------------------------------

fn saved_lantern() -> (Player, Player) {
    let (ada, corvin) = form();
    desktop_away(&ada);
    type_in(&ada, "Title", "The Lost Lantern");
    add_written_step(&ada, "find my lantern");
    click(&ada, "Save");
    (ada, corvin)
}

#[test]
fn a_task_saves_without_a_player_and_waits_in_the_list() {
    let (ada, _corvin) = saved_lantern();

    assert!(rows(&ada).contains(&"item: The Lost Lantern (Not sent yet)".to_string()));
    assert_eq!(
        buttons(&ada),
        ["Help me write", "Delete", "Cancel", "Save", "Send (off)"]
    );
    let crumb: String = ada.eval("ns.Journal.Page('quests').crumb");
    assert_eq!(crumb, "The Lost Lantern");
}

#[test]
fn a_saved_task_goes_out_later_to_the_picked_player() {
    let (ada, corvin) = saved_lantern();
    click(&ada, "Cancel");
    let key: String = ada.eval("'draft:' .. next(ns.TaskStore.Data().drafts)");
    ada.run(&format!("ns.JournalFrame.Select('{key}')"));

    click_line(&ada, "Corvin", "Pick");
    click(&ada, "Send");
    exchange(&ada, &corvin);

    let title: String =
        corvin.eval("local _, task = next(ns.TaskStore.Data().received) return task.title");
    assert_eq!(title, "The Lost Lantern");
    let saved: bool = ada.eval("next(ns.TaskStore.Data().drafts) == nil");
    assert!(saved, "a sent task leaves the saved ones");
}

#[test]
fn cancel_leaves_a_saved_task_as_it_was() {
    let (ada, _corvin) = saved_lantern();
    type_in(&ada, "Title", "Another name");

    click(&ada, "Cancel");
    let key: String = ada.eval("'draft:' .. next(ns.TaskStore.Data().drafts)");
    ada.run(&format!("ns.JournalFrame.Select('{key}')"));

    assert_eq!(
        ada.eval::<String>("ns.TaskForm.Draft().title"),
        "The Lost Lantern"
    );
}

#[test]
fn delete_asks_first_and_removes_a_saved_task() {
    let (ada, _corvin) = saved_lantern();

    click(&ada, "Delete");
    let before: bool = ada.eval("next(ns.TaskStore.Data().drafts) ~= nil");
    ada.run("wow.AcceptPopup()");

    assert!(before);
    let after: bool = ada.eval("next(ns.TaskStore.Data().drafts) == nil");
    assert!(after);
}

#[test]
fn new_task_opens_an_empty_form_after_a_saved_one() {
    let (ada, _corvin) = saved_lantern();

    ada.run("ns.JournalFrame.Select('give')");

    assert_eq!(ada.eval::<String>("ns.TaskForm.Draft().title"), "");
}

// A step that the game can't check -----------------------------------------------------------

#[test]
fn the_doer_marks_a_written_step_done_and_the_giver_decides() {
    let (ada, corvin, id) =
        players::accepted_task("{ { kind = 'other', target = 'Find my lost ring', count = 1 } }");
    corvin.run(&format!(
        "ns.Journal.Select('quests', 'got:{}')",
        received_key(&id)
    ));

    let before = lines(&corvin);
    click_line(&corvin, "Find my lost ring", "Done");
    exchange(&ada, &corvin);
    ada.run(&format!("ns.Journal.Select('quests', 'gave:{id}')"));

    assert!(before.contains(&"entry: Find my lost ring. [Done]".to_string()));
    assert!(lines(&corvin).contains(&"entry: (done) Find my lost ring.".to_string()));
    let proof = lines(&ada);
    assert!(
        proof
            .iter()
            .any(|line| line.starts_with("hint: They say it's done. You decide.")),
        "{proof:?}"
    );
}

#[test]
fn a_task_with_no_description_reaches_the_doer() {
    let (ada, corvin) = ada_and_corvin();
    let id: String = ada.eval(
        "return ns.PlayerTasks.Give({ title = 'Quick one', text = '', reward = '',
             steps = { { kind = 'other', target = 'Wave at me', count = 1 } } }, 'Corvin-Stormrage')",
    );

    exchange(&ada, &corvin);
    corvin.run(&format!(
        "ns.Journal.Select('quests', 'got:{}')",
        received_key(&id)
    ));

    let page = lines(&corvin);
    assert_eq!(page[0], "heading: Quick one");
    assert!(!page.iter().any(|line| line.starts_with("prose:")));
}

// Help me write ------------------------------------------------------------------------------

#[test]
fn an_idea_too_long_in_bytes_keeps_what_the_player_typed() {
    let (ada, _corvin) = form();
    let idea = "\u{e9}".repeat(150);

    click(&ada, "Help me write");
    write(&ada, &idea);

    assert!(ada.eval::<bool>("ns.Editor.IsShown()"));
    assert_eq!(ada.eval::<String>("wow.MultiLineBox():GetText()"), idea);
    assert!(asked_ideas(&ada).is_empty());
}

#[test]
fn the_name_of_the_giver_from_the_model_shows_as_the_name() {
    let (ada, _corvin) = form();

    ada.game.reply(
        r#"{"type":"draft_answer","id":1,"draft":{"title":"$N needs help","text":"Help $N at the mill.","steps":[{"goal":"place","target":"Brill"}]}}"#,
    );

    let page = lines(&ada);
    assert!(
        page.contains(&"text: Ada needs help".to_string()),
        "{page:?}"
    );
    assert!(page.contains(&"prose: Help Ada at the mill.".to_string()));
}

#[test]
fn a_draft_with_another_code_of_the_model_is_dropped() {
    let (ada, _corvin) = form();

    ada.game.reply(
        r#"{"type":"draft_answer","id":1,"draft":{"title":"Help $G","text":"Go.","steps":[{"goal":"place","target":"Brill"}]}}"#,
    );

    assert!(!lines(&ada).contains(&"section: Suggestion [Use this]".to_string()));
}

#[test]
fn a_suggestion_of_the_model_changes_nothing_until_the_player_picks_it() {
    let (ada, _corvin) = form();
    click(&ada, "Help me write");
    write(&ada, "kill gregor");
    let asking = lines(&ada);

    ada.game.reply(
        r#"{"type":"draft_answer","id":1,"draft":{"title":"Trouble at the Mills","text":"Put Gregor to rest.","steps":[{"goal":"kill","target":"3 Gregor Agamand"}]}}"#,
    );
    let before: String = ada.eval("ns.TaskForm.Draft().title");
    click_line(&ada, "Suggestion", "Use this");

    assert!(asking.contains(&"hint: Writing...".to_string()));
    assert_eq!(before, "");
    let after = lines(&ada);
    assert_eq!(
        ada.eval::<String>("ns.TaskForm.Draft().title"),
        "Trouble at the Mills"
    );
    assert!(after.contains(&"entry: Defeat Gregor Agamand (3 times). [Remove]".to_string()));
    assert!(
        asked_ideas(&ada)
            .iter()
            .any(|line| line.contains(r#""idea":"kill gregor""#))
    );
}

#[test]
fn a_draft_that_breaks_a_rule_of_the_addon_is_dropped_whole() {
    let (ada, _corvin) = form();

    ada.game.reply(
        r#"{"type":"draft_answer","id":1,"draft":{"title":"Fine","text":"Also ||Hfine||h.","steps":[]}}"#,
    );

    let page = lines(&ada);
    assert!(!page.contains(&"section: Suggestion [Use this]".to_string()));
    assert!(page.contains(&"hint: Couldn't write that one. Try other words.".to_string()));
}

#[test]
fn no_draft_from_the_desktop_says_so() {
    let (ada, _corvin) = form();

    ada.game
        .reply(r#"{"type":"draft_answer","id":1,"draft":null}"#);

    assert!(lines(&ada).contains(&"hint: Couldn't write that one. Try other words.".to_string()));
}

// The widgets ------------------------------------------------------------------------------

/// The boxes of the journal that show, by the text that they show when empty.
fn shown_boxes(player: &Player) -> Vec<String> {
    player.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'EditBox' and widget.shown and widget.placeholder then
                 table.insert(out, widget.placeholder.text)
             end
         end
         return out",
    )
}

#[test]
fn the_form_draws_a_box_for_the_title_and_one_for_a_step() {
    let (ada, _corvin) = form();

    assert_eq!(
        shown_boxes(&ada),
        ["Title", "Add a step, like: kill 5 bats"]
    );
}

#[test]
fn typing_in_the_title_box_changes_the_task() {
    let (ada, _corvin) = form();

    ada.run(
        "for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'EditBox' and widget.placeholder and widget.placeholder.text == 'Title' then
                 widget:SetText('The Lost Lantern')
                 widget.scripts.OnTextChanged(widget, true)
             end
         end",
    );

    assert_eq!(
        ada.eval::<String>("ns.TaskForm.Draft().title"),
        "The Lost Lantern"
    );
}

#[test]
fn the_reward_shows_six_slots_and_the_money_boxes() {
    let (ada, _corvin) = form();
    drop_item(&ada, 2589, "Linen Cloth", 10);

    let slots: Vec<String> = ada.eval(
        "local out = {}
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'Button' and widget.shown and widget.icon then
                 table.insert(out, widget.count.text or '')
             end
         end
         return out",
    );
    let coins: usize = ada.eval(
        "local count = 0
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'EditBox' and widget.shown and widget.numeric then count = count + 1 end
         end
         return count",
    );

    assert_eq!(slots, ["10", "", "", "", "", ""]);
    assert_eq!(coins, 3);
}

#[test]
fn the_step_box_empties_after_add() {
    let (ada, _corvin) = form();
    desktop_away(&ada);
    let step_box = "local box
         for _, widget in ipairs(wow.widgets) do
             if widget.kind == 'EditBox' and widget.placeholder
                 and widget.placeholder.text == 'Add a step, like: kill 5 bats' then box = widget end
         end";

    ada.run(&format!(
        "{step_box}
         box:SetText('wave at me')
         box.scripts.OnTextChanged(box, true)
         box.scripts.OnEnterPressed(box)"
    ));

    let text: String = ada.eval(&format!("{step_box} return box:GetText()"));
    assert_eq!(text, "");
    assert!(lines(&ada).contains(&"entry: wave at me. [Remove]".to_string()));
}
