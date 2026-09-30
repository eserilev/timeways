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
fn a_kill_in_the_party_that_the_giver_never_saw_is_not_confirmed() {
    let (ada, corvin, id) = accepted_task(THREE_ZOMBIES);
    ada.in_party_with(&corvin);
    corvin.in_party_with(&ada);
    ada.run("wow.Fire('GROUP_ROSTER_UPDATE') wow.Fire('ZONE_CHANGED')");

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
