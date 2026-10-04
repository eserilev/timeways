//! The steps of a player task that the addon sees in the game: a place, a talk, a kill, a
//! meeting, and a trade (GAMEPLAY.md 4.7). The doer's addon claims each step, and the giver's
//! addon keeps what it saw itself.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;
mod players;

use players::{Player, accepted_task, exchange, received_key};

const PLACE: &str = "{ { kind = 'place', target = 'Agamand Mills', count = 1 } }";
const TALK: &str = "{ { kind = 'npc', target = 'Innkeeper Renee', count = 1 } }";
const THREE_ZOMBIES: &str = "{ { kind = 'kill', target = 'Rattlecage Soldier', count = 3 } }";
const LINEN: &str = "{ { kind = 'item', target = 'Linen Cloth', count = 10 } }";
const FIND_BRAM: &str = "{ { kind = 'meet', target = 'Bram-Stormrage', count = 1 } }";

/// Whether the doer's addon claimed each step, in order.
fn claimed(corvin: &Player, id: &str) -> Vec<bool> {
    corvin.eval(&format!(
        "local task = ns.TaskStore.Data().received['{}']
         local out = {{}}
         for index in ipairs(task.steps) do out[index] = task.claims[index] ~= nil end
         return out",
        received_key(id)
    ))
}

/// Whether the giver's addon got the claim of each step, in order.
fn reported(ada: &Player, id: &str) -> Vec<bool> {
    ada.eval(&format!(
        "local task = ns.TaskStore.Data().given['{id}']
         local out = {{}}
         for index in ipairs(task.steps) do out[index] = task.claims[index] ~= nil end
         return out"
    ))
}

/// The level that the turn-in card shows: the one kept since the claim came.
fn settled(ada: &Player, id: &str, index: usize) -> String {
    ada.eval(&format!(
        "local task = ns.TaskStore.Data().given['{id}']
         local step = task.steps[{index}]
         return ns.TaskProof.Settled(step, task.claims[{index}], task, ns.TaskStore.Data())"
    ))
}

fn level(ada: &Player, id: &str, index: usize) -> String {
    ada.eval(&format!(
        "local task = ns.TaskStore.Data().given['{id}']
         local step = task.steps[{index}]
         return ns.TaskProof.Level(step, task.claims[{index}], task, ns.TaskStore.Data())"
    ))
}

/// A zombie that Corvin sees and then kills: its GUID first, then the kill.
fn kill_zombie(player: &Player, attacker: &str, n: u32) {
    player.run(&format!(
        "wow.units.nameplate1 = {{ name = 'Rattlecage Soldier', guid = 'Creature-{n}', hostile = true }}
         wow.Fire('NAME_PLATE_UNIT_ADDED', 'nameplate1')
         wow.Fire('PARTY_KILL', '{attacker}', 'Creature-{n}')"
    ));
}

#[test]
fn a_place_step_is_done_when_the_doer_walks_in() {
    let (ada, corvin, id) = accepted_task(PLACE);

    corvin.run("wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED')");
    exchange(&ada, &corvin);

    assert_eq!(claimed(&corvin, &id), [true]);
    assert_eq!(reported(&ada, &id), [true]);
}

#[test]
fn a_place_where_the_doer_stands_at_the_accept_counts_at_once() {
    let steps = "{ { kind = 'place', target = 'Brill', count = 1 } }";

    let (_ada, corvin, id) = accepted_task(steps);

    assert_eq!(claimed(&corvin, &id), [true]);
}

#[test]
fn a_talk_step_is_done_when_the_doer_talks_to_the_npc() {
    let (_ada, corvin, id) = accepted_task(TALK);

    corvin.run("wow.units.npc = { name = 'Innkeeper Renee' } wow.Fire('GOSSIP_SHOW')");

    assert_eq!(claimed(&corvin, &id), [true]);
}

#[test]
fn a_player_with_the_name_of_the_npc_never_counts_as_a_talk() {
    let (_ada, corvin, id) = accepted_task(TALK);

    corvin.run(
        "wow.units.npc = { name = 'Innkeeper Renee', player = true } wow.Fire('QUEST_DETAIL')",
    );

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn a_defeat_step_counts_each_kill_of_its_target() {
    let (_ada, corvin, id) = accepted_task(THREE_ZOMBIES);

    kill_zombie(&corvin, "Player-1-Corvin", 1);
    kill_zombie(&corvin, "Player-1-Corvin", 2);
    let after_two = claimed(&corvin, &id);
    kill_zombie(&corvin, "Player-1-Corvin", 3);

    assert_eq!(after_two, [false]);
    assert_eq!(claimed(&corvin, &id), [true]);
}

#[test]
fn a_kill_of_a_unit_that_the_addon_never_saw_counts_nothing() {
    let (_ada, corvin, id) =
        accepted_task("{ { kind = 'kill', target = 'Rattlecage Soldier', count = 1 } }");

    corvin.run("wow.Fire('PARTY_KILL', 'Player-1-Corvin', 'Creature-9')");

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn a_hidden_guid_is_never_read() {
    let (_ada, corvin, id) =
        accepted_task("{ { kind = 'kill', target = 'Rattlecage Soldier', count = 1 } }");

    corvin.run(
        "wow.secrets['Creature-1'] = true
         wow.units.target = { name = 'Rattlecage Soldier', guid = 'Creature-1', hostile = true }
         wow.Fire('PLAYER_TARGET_CHANGED')
         wow.Fire('PARTY_KILL', 'Player-1-Corvin', 'Creature-1')",
    );

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn a_find_a_player_step_is_done_next_to_the_player() {
    let (_ada, corvin, id) = accepted_task(FIND_BRAM);

    corvin.run(
        "wow.units.target = { name = 'Bram', player = true } wow.Fire('PLAYER_TARGET_CHANGED')",
    );
    let far = claimed(&corvin, &id);
    corvin.run("wow.units.target.near = true wow.Fire('PLAYER_TARGET_CHANGED')");

    assert_eq!(far, [false]);
    assert_eq!(claimed(&corvin, &id), [true]);
}

#[test]
fn a_bring_step_is_done_when_the_items_reach_the_giver_in_trades() {
    let (ada, corvin, id) = accepted_task(LINEN);

    corvin.run(
        "wow.Trade('Ada', { gave = { { name = 'Linen Cloth', count = 6 } }, got = {}, money = 0, moneyGot = 0 })",
    );
    let after_six = claimed(&corvin, &id);
    corvin.run(
        "wow.Trade('Ada', { gave = { { name = 'Linen Cloth', count = 4 } }, got = {}, money = 0, moneyGot = 0 })",
    );
    exchange(&ada, &corvin);

    assert_eq!(after_six, [false]);
    assert_eq!(claimed(&corvin, &id), [true]);
    assert_eq!(reported(&ada, &id), [true]);
}

#[test]
fn items_traded_to_another_player_count_nothing() {
    let (_ada, corvin, id) = accepted_task(LINEN);

    corvin.run(
        "wow.Trade('Bram', { gave = { { name = 'Linen Cloth', count = 10 } }, got = {}, money = 0, moneyGot = 0 })",
    );

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn a_trade_that_one_player_never_accepted_counts_nothing() {
    let (_ada, corvin, id) = accepted_task(LINEN);

    corvin.run(
        "wow.units.npc = { name = 'Ada', player = true }
         wow.trade = { gave = { { name = 'Linen Cloth', count = 10 } }, got = {}, money = 0, moneyGot = 0 }
         wow.Fire('TRADE_SHOW')
         wow.Fire('TRADE_ACCEPT_UPDATE', 1, 0)
         wow.Fire('TRADE_CLOSED')",
    );

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn a_change_after_both_accepted_ends_the_accept() {
    let (_ada, corvin, id) = accepted_task(LINEN);

    corvin.run(
        "wow.units.npc = { name = 'Ada', player = true }
         wow.trade = { gave = { { name = 'Linen Cloth', count = 10 } }, got = {}, money = 0, moneyGot = 0 }
         wow.Fire('TRADE_SHOW')
         wow.Fire('TRADE_ACCEPT_UPDATE', 1, 1)
         wow.Fire('TRADE_ACCEPT_UPDATE', 0, 0)
         wow.Fire('TRADE_CLOSED')",
    );

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn the_giver_keeps_the_stretches_of_party_time_with_the_doer() {
    let (ada, corvin, _id) = accepted_task(PLACE);

    ada.in_party_with(&corvin);
    ada.run("wow.now = wow.now + 100 wow.Fire('GROUP_ROSTER_UPDATE')");
    ada.leave_party();
    ada.run("wow.now = wow.now + 50 wow.Fire('GROUP_ROSTER_UPDATE')");

    let stretch: Vec<u64> = ada.eval(
        "local stretch = ns.TaskStore.Data().party['Corvin-Stormrage'][1]
         return { stretch.from, stretch.to }",
    );
    assert_eq!(stretch[1] - stretch[0], 50);
}

#[test]
fn the_giver_witnesses_a_kill_of_the_doer_in_the_party() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    ada.run("wow.Fire('GROUP_ROSTER_UPDATE') wow.units.party1.guid = 'Player-1-Corvin'");

    for n in 1..=3 {
        kill_zombie(&corvin, "Player-1-Corvin", n);
        kill_zombie(&ada, "Player-1-Corvin", n);
    }
    exchange(&ada, &corvin);

    assert_eq!(level(&ada, &id, 1), "witnessed");
}

#[test]
fn a_kill_next_to_the_giver_that_the_giver_never_saw_is_not_confirmed() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    ada.run("wow.units.party1.yards = 20 wow.Fire('GROUP_ROSTER_UPDATE')");

    for n in 1..=3 {
        kill_zombie(&corvin, "Player-1-Corvin", n);
    }
    exchange(&ada, &corvin);

    assert_eq!(level(&ada, &id, 1), "unconfirmed");
}

#[test]
fn a_kill_far_from_a_giver_with_no_party_is_seen() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    ada.run("wow.zone = 'Silverpine Forest' wow.Fire('ZONE_CHANGED_NEW_AREA')");

    for n in 1..=3 {
        kill_zombie(&corvin, "Player-1-Corvin", n);
    }
    exchange(&ada, &corvin);

    assert_eq!(level(&ada, &id, 1), "seen");
}

#[test]
fn the_giver_sees_the_items_and_the_reward_in_the_same_trade() {
    let (ada, corvin, id) = accepted_task(LINEN);

    ada.run(
        "wow.Trade('Corvin', { gave = {}, got = { { name = 'Linen Cloth', count = 10 } }, money = 50000, moneyGot = 0 })",
    );
    corvin.run(
        "wow.Trade('Ada', { gave = { { name = 'Linen Cloth', count = 10 } }, got = {}, money = 0, moneyGot = 50000 })",
    );
    exchange(&ada, &corvin);

    assert_eq!(level(&ada, &id, 1), "witnessed");
    let paid: bool = ada.eval(&format!(
        "ns.TaskProof.Paid(ns.TaskStore.Data().given['{id}'], ns.TaskStore.Data())"
    ));
    assert!(paid);
}

#[test]
fn a_trade_with_a_player_of_no_task_leaves_no_record() {
    let (ada, _corvin, _id) = accepted_task(LINEN);

    ada.run("wow.Trade('Bram', { gave = {}, got = {}, money = 100, moneyGot = 0 })");

    let trades: usize = ada.eval("#ns.TaskStore.Data().trades");
    assert_eq!(trades, 0);
}

#[test]
fn a_party_that_began_before_the_accept_counts_from_the_accept() {
    let (ada, corvin) = players::ada_and_corvin();
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    let id: String = ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'T', text = 'X.', reward = '', steps = {PLACE} }}, 'Corvin-Stormrage')"
    ));
    exchange(&ada, &corvin);

    corvin.run(&format!("ns.PlayerTasks.Accept('{}')", received_key(&id)));
    exchange(&ada, &corvin);

    let open: bool = ada.eval(
        "local stretch = ns.TaskStore.Data().party['Corvin-Stormrage'][1]
         return stretch.from ~= nil and stretch.to == nil",
    );
    assert!(open);
}

/// Ada and Corvin in one party, with Corvin 20 yards from Ada.
fn in_party_close(ada: &Player, corvin: &Player) {
    ada.in_party_with(corvin);
    corvin.in_party_with(ada);
    ada.run("wow.units.party1.yards = 20 wow.Fire('GROUP_ROSTER_UPDATE')");
}

/// Ada hears a step message of Corvin with the time and the zone that Corvin gives.
fn hear_step(ada: &Player, corvin: &Player, id: &str, index: u32, at: u64, zone: &str) {
    ada.hear_logged(
        "Timeways",
        &format!("1:1:1:1;step;{id};{index};{at};{zone}"),
        "WHISPER",
        &corvin.full_name(),
    );
}

#[test]
fn a_kill_in_the_party_far_from_the_giver_is_seen() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    ada.run("wow.Fire('GROUP_ROSTER_UPDATE') wow.Fire('ZONE_CHANGED')");

    for n in 1..=3 {
        kill_zombie(&corvin, "Player-1-Corvin", n);
    }
    exchange(&ada, &corvin);

    assert_eq!(level(&ada, &id, 1), "seen");
}

#[test]
fn a_doer_who_reports_another_time_and_zone_is_judged_by_the_clock_of_the_giver() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    in_party_close(&ada, &corvin);

    hear_step(&ada, &corvin, &id, 1, 1, "Elsewhere");

    assert_eq!(level(&ada, &id, 1), "unconfirmed");
    let zone: String = ada.eval(&format!("ns.TaskStore.Data().given['{id}'].claims[1].zone"));
    assert_eq!(zone, "Tirisfal Glades");
}

#[test]
fn a_kill_by_another_member_of_the_group_counts_for_the_doer_and_the_giver() {
    let (ada, corvin, id) =
        accepted_task("{ { kind = 'kill', target = 'Rattlecage Soldier', count = 1 } }");
    in_party_close(&ada, &corvin);
    corvin.in_party_with(&ada);

    kill_zombie(&ada, "Player-1-Bram", 1);
    kill_zombie(&corvin, "Player-1-Bram", 1);
    exchange(&ada, &corvin);

    assert_eq!(claimed(&corvin, &id), [true]);
    assert_eq!(level(&ada, &id, 1), "witnessed");
}

#[test]
fn a_place_that_the_doer_reached_while_the_giver_was_offline_is_never_witnessed() {
    let (ada, corvin, id) = accepted_task(PLACE);
    ada.in_party_with(&corvin);
    ada.run(
        "wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED') wow.Fire('GROUP_ROSTER_UPDATE')
         wow.Fire('PLAYER_LOGOUT')
         wow.now = wow.now + 600
         wow.Fire('PLAYER_ENTERING_WORLD')",
    );

    let at: u64 = ada.eval("wow.now - 300");
    ada.hear_logged(
        "Timeways",
        &format!("1:1:1:1;turnin;{id};1;1;{at};Tirisfal Glades"),
        "WHISPER",
        &corvin.full_name(),
    );

    assert_eq!(level(&ada, &id, 1), "seen");
}

#[test]
fn the_giver_knows_where_it_stood_when_it_gave_the_task() {
    let (ada, corvin) = players::ada_and_corvin();
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    ada.run("wow.subzone = 'Agamand Mills'");
    corvin.run("wow.subzone = 'Agamand Mills'");
    let id: String = ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'T', text = 'X.', reward = '', steps = {PLACE} }}, 'Corvin-Stormrage')"
    ));
    exchange(&ada, &corvin);

    corvin.run(&format!("ns.PlayerTasks.Accept('{}')", received_key(&id)));
    exchange(&ada, &corvin);

    assert_eq!(level(&ada, &id, 1), "witnessed");
}

#[test]
fn the_zone_history_keeps_only_real_changes() {
    let (ada, _corvin, _id) = accepted_task(PLACE);

    ada.run("for n = 1, 5 do wow.Fire('ZONE_CHANGED') end");
    ada.run("wow.subzone = 'Agamand Mills' wow.Fire('ZONE_CHANGED')");

    let zones: usize = ada.eval("#ns.TaskStore.Data().zones");
    assert_eq!(zones, 2);
}

#[test]
fn the_proof_of_a_step_stays_when_the_records_of_the_giver_are_gone() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    in_party_close(&ada, &corvin);
    for n in 1..=3 {
        kill_zombie(&corvin, "Player-1-Corvin", n);
        kill_zombie(&ada, "Player-1-Corvin", n);
    }
    exchange(&ada, &corvin);

    ada.run(
        "local data = ns.TaskStore.Data() for n = #data.kills, 1, -1 do data.kills[n] = nil end",
    );

    assert_eq!(settled(&ada, &id, 1), "witnessed");
}

#[test]
fn a_reward_paid_after_the_turn_in_counts() {
    let (ada, corvin, id) = accepted_task(PLACE);
    ada.run("wow.units.target = { name = 'Corvin', player = true, near = true }");
    ada.run(&format!("ns.PlayerTasks.Complete('{id}')"));
    exchange(&ada, &corvin);

    ada.run("wow.Trade('Corvin', { gave = {}, got = {}, money = 50000, moneyGot = 0 })");

    let paid: bool = ada.eval(&format!(
        "ns.TaskProof.Paid(ns.TaskStore.Data().given['{id}'], ns.TaskStore.Data())"
    ));
    assert!(paid);
}

#[test]
fn a_trade_that_the_game_completes_before_both_accepts_show_still_counts() {
    let (_ada, corvin, id) = accepted_task(LINEN);

    corvin.run(
        "wow.units.npc = { name = 'Ada', player = true }
         wow.trade = { gave = {}, got = {}, money = 0, moneyGot = 0 }
         wow.Fire('TRADE_SHOW')
         wow.trade.gave[1] = { name = 'Linen Cloth', count = 10 }
         wow.Fire('TRADE_PLAYER_ITEM_CHANGED', 1)
         wow.Fire('TRADE_ACCEPT_UPDATE', 1, 0)
         wow.trade = { gave = {}, got = {}, money = 0, moneyGot = 0 }
         wow.Fire('TRADE_CLOSED')
         wow.Fire('UI_INFO_MESSAGE', wow.TRADE_COMPLETE, 'Trade complete.')",
    );

    assert_eq!(claimed(&corvin, &id), [true]);
}

#[test]
fn a_trade_that_one_player_canceled_after_the_other_accepted_counts_nothing() {
    let (_ada, corvin, id) = accepted_task(LINEN);

    corvin.run(
        "wow.units.npc = { name = 'Ada', player = true }
         wow.trade = { gave = { { name = 'Linen Cloth', count = 10 } }, got = {}, money = 0, moneyGot = 0 }
         wow.Fire('TRADE_SHOW')
         wow.Fire('TRADE_ACCEPT_UPDATE', 1, 0)
         wow.Fire('TRADE_CLOSED')
         wow.Fire('UI_INFO_MESSAGE', 1, 'Trade cancelled.')",
    );

    assert_eq!(claimed(&corvin, &id), [false]);
}

#[test]
fn a_giver_holds_at_most_twenty_open_tasks() {
    let (ada, _corvin) = players::ada_and_corvin();

    let given: Vec<bool> = ada.eval(&format!(
        "local given = {{}}
         for n = 1, 21 do
             given[n] = ns.PlayerTasks.Give({{ title = 'T', text = 'X.', reward = '', steps = {PLACE} }}, 'Corvin-Stormrage') ~= nil
         end
         return given"
    ));

    assert!(given[..20].iter().all(|given| *given));
    assert!(!given[20]);
}

#[test]
fn party_time_stays_only_for_the_doers_of_kept_tasks() {
    let (ada, _corvin, _id) = accepted_task(PLACE);
    ada.run("ns.TaskStore.Data().party['Bram-Stormrage'] = { { from = 1 } }");

    ada.run("wow.Fire('GROUP_ROSTER_UPDATE')");

    let names: Vec<String> = ada.eval(
        "local names = {} for name in pairs(ns.TaskStore.Data().party) do table.insert(names, name) end return names",
    );
    assert_eq!(names, ["Corvin-Stormrage"]);
}
