//! How sure the giver can be of each step of a player task: seen, witnessed, or not
//! confirmed (GAMEPLAY.md 4.7). The rules are pure functions of the claim of the doer and the
//! records of the giver.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

/// The task was sent at 1000. Ada gave it, and Corvin does it. `setup` fills `records`.
fn level(setup: &str, step: &str, claim: &str) -> String {
    let game = Game::new();
    game.eval(&format!(
        "local task = {{ giver = 'Ada-Stormrage', doer = 'Corvin-Stormrage', sentAt = 1000 }}
         local records = {{ party = {{}}, zones = {{}}, kills = {{}}, npcs = {{}}, near = {{}}, trades = {{}} }}
         {setup}
         return ns.TaskProof.Level({step}, {claim}, task, records)"
    ))
}

const KILL: &str = "{ kind = 'kill', target = 'Gregor Agamand', count = 1 }";
const PLACE: &str = "{ kind = 'place', target = 'Agamand Mills', count = 1 }";
const TALK: &str = "{ kind = 'npc', target = 'Innkeeper Renee', count = 1 }";
const LINEN: &str = "{ kind = 'item', target = 'Linen Cloth', count = 10 }";
const CLAIM: &str = "{ at = 2000, zone = 'Tirisfal Glades' }";

const IN_PARTY: &str = "records.party['Corvin-Stormrage'] = { { from = 1500 } }";
const IN_TIRISFAL: &str =
    "table.insert(records.zones, { at = 1200, zone = 'Tirisfal Glades', subzone = 'Brill' })";

#[test]
fn a_step_with_no_claim_is_missing() {
    assert_eq!(level("", KILL, "nil"), "missing");
}

#[test]
fn a_step_that_only_the_doer_saw_is_seen() {
    assert_eq!(level("", KILL, CLAIM), "seen");
}

#[test]
fn a_kill_that_the_giver_saw_in_the_party_is_witnessed() {
    let setup = format!(
        "{IN_PARTY}
         table.insert(records.kills, {{ at = 1990, doer = 'Corvin-Stormrage', target = 'Gregor Agamand' }})"
    );

    assert_eq!(level(&setup, KILL, CLAIM), "witnessed");
}

#[test]
fn fewer_kills_than_the_step_asks_are_not_witnessed() {
    let setup = "table.insert(records.kills, { at = 1990, doer = 'Corvin-Stormrage', target = 'Gregor Agamand' })";
    let three = "{ kind = 'kill', target = 'Gregor Agamand', count = 3 }";

    assert_eq!(level(setup, three, CLAIM), "seen");
}

#[test]
fn a_kill_before_the_task_was_sent_does_not_count() {
    let setup = "table.insert(records.kills, { at = 900, doer = 'Corvin-Stormrage', target = 'Gregor Agamand' })";

    assert_eq!(level(setup, KILL, CLAIM), "seen");
}

#[test]
fn a_kill_of_another_player_does_not_count_for_the_doer() {
    let setup = "table.insert(records.kills, { at = 1990, doer = 'Bram-Stormrage', target = 'Gregor Agamand' })";

    assert_eq!(level(setup, KILL, CLAIM), "seen");
}

#[test]
fn a_step_next_to_the_giver_that_the_giver_did_not_see_is_not_confirmed() {
    let near = "{ at = 2000, zone = 'Tirisfal Glades', near = true }";

    assert_eq!(level(IN_PARTY, KILL, near), "unconfirmed");
}

#[test]
fn a_step_in_a_party_in_the_same_zone_far_from_the_giver_is_seen() {
    let setup = format!("{IN_PARTY}\n{IN_TIRISFAL}");

    assert_eq!(level(&setup, KILL, CLAIM), "seen");
}

#[test]
fn a_level_kept_from_the_claim_stays_when_the_records_are_gone() {
    let game = Game::new();

    let kept: String = game.eval(&format!(
        "local task = {{ giver = 'Ada-Stormrage', doer = 'Corvin-Stormrage', sentAt = 1000 }}
         local records = {{ party = {{}}, zones = {{}}, kills = {{}}, npcs = {{}}, near = {{}}, trades = {{}} }}
         local claim = {{ at = 2000, zone = 'Tirisfal Glades', level = 'witnessed' }}
         return ns.TaskProof.Settled({PLACE}, claim, task, records)"
    ));

    assert_eq!(kept, "witnessed");
}

#[test]
fn a_witness_that_comes_after_the_claim_raises_its_level() {
    let setup = format!(
        "{IN_PARTY}
         table.insert(records.npcs, {{ at = 2100, name = 'Innkeeper Renee' }})"
    );
    let game = Game::new();

    let raised: String = game.eval(&format!(
        "local task = {{ giver = 'Ada-Stormrage', doer = 'Corvin-Stormrage', sentAt = 1000 }}
         local records = {{ party = {{}}, zones = {{}}, kills = {{}}, npcs = {{}}, near = {{}}, trades = {{}} }}
         {setup}
         local claim = {{ at = 2000, zone = 'Tirisfal Glades', level = 'seen' }}
         return ns.TaskProof.Settled({TALK}, claim, task, records)"
    ));

    assert_eq!(raised, "witnessed");
}

#[test]
fn a_party_that_ended_before_the_step_does_not_count() {
    let setup = "records.party['Corvin-Stormrage'] = { { from = 1500, to = 1900 } }
         table.insert(records.zones, { at = 1200, zone = 'Tirisfal Glades', subzone = 'Agamand Mills' })";

    assert_eq!(level(setup, PLACE, CLAIM), "seen");
}

#[test]
fn the_zone_of_the_giver_is_the_last_one_before_the_step() {
    let setup = format!(
        "{IN_PARTY}
         table.insert(records.zones, {{ at = 1200, zone = 'Tirisfal Glades', subzone = 'Agamand Mills' }})
         table.insert(records.zones, {{ at = 1800, zone = 'Silverpine Forest', subzone = '' }})
         table.insert(records.zones, {{ at = 2500, zone = 'Tirisfal Glades', subzone = 'Agamand Mills' }})"
    );

    assert_eq!(level(&setup, PLACE, CLAIM), "seen");
}

#[test]
fn a_place_where_the_giver_stood_in_the_party_is_witnessed() {
    let setup = format!(
        "{IN_PARTY}
         table.insert(records.zones, {{ at = 1950, zone = 'Tirisfal Glades', subzone = 'Agamand Mills' }})"
    );

    assert_eq!(level(&setup, PLACE, CLAIM), "witnessed");
}

#[test]
fn a_place_where_the_giver_stood_with_no_party_is_only_seen() {
    let setup = "table.insert(records.zones, { at = 1950, zone = 'Tirisfal Glades', subzone = 'Agamand Mills' })";

    assert_eq!(level(setup, PLACE, CLAIM), "seen");
}

#[test]
fn a_talk_with_an_npc_that_the_giver_saw_at_the_time_is_witnessed() {
    let setup = format!(
        "{IN_PARTY}
         table.insert(records.npcs, {{ at = 2100, name = 'Innkeeper Renee' }})"
    );

    assert_eq!(level(&setup, TALK, CLAIM), "witnessed");
}

#[test]
fn an_npc_that_the_giver_saw_long_before_is_no_witness() {
    let setup = format!(
        "{IN_PARTY}
         {IN_TIRISFAL}
         table.insert(records.npcs, {{ at = 1700, name = 'Innkeeper Renee' }})"
    );

    assert_eq!(level(&setup, TALK, CLAIM), "seen");
}

#[test]
fn a_meeting_with_the_giver_that_the_giver_saw_is_witnessed() {
    let meet = "{ kind = 'meet', target = 'Ada-Stormrage', count = 1 }";
    let setup = "table.insert(records.near, { at = 2005, name = 'Corvin-Stormrage' })";

    assert_eq!(level(setup, meet, CLAIM), "witnessed");
}

#[test]
fn a_meeting_with_another_player_is_never_witnessed() {
    let meet = "{ kind = 'meet', target = 'Bram-Stormrage', count = 1 }";
    let setup = "table.insert(records.near, { at = 2005, name = 'Corvin-Stormrage' })";

    assert_eq!(level(setup, meet, CLAIM), "seen");
}

#[test]
fn an_item_that_the_giver_got_in_a_trade_is_witnessed() {
    let setup = "table.insert(records.trades, { at = 1900, with = 'Corvin-Stormrage', got = { ['Linen Cloth'] = 6 }, gave = {}, money = 0 })
                 table.insert(records.trades, { at = 1950, with = 'Corvin-Stormrage', got = { ['Linen Cloth'] = 4 }, gave = {}, money = 0 })";

    assert_eq!(level(setup, LINEN, CLAIM), "witnessed");
}

#[test]
fn an_item_that_the_giver_never_got_is_not_confirmed_anywhere() {
    let setup = "table.insert(records.trades, { at = 1900, with = 'Corvin-Stormrage', got = { ['Linen Cloth'] = 9 }, gave = {}, money = 0 })";

    assert_eq!(level(setup, LINEN, CLAIM), "unconfirmed");
}

fn paid(setup: &str) -> bool {
    let game = Game::new();
    game.eval(&format!(
        "local task = {{ giver = 'Ada-Stormrage', doer = 'Corvin-Stormrage', sentAt = 1000 }}
         local records = {{ trades = {{}} }}
         {setup}
         return ns.TaskProof.Paid(task, records)"
    ))
}

#[test]
fn the_reward_is_paid_when_the_giver_gave_money_in_a_trade() {
    let setup = "table.insert(records.trades, { at = 2000, with = 'Corvin-Stormrage', got = {}, gave = {}, money = 50000 })";

    assert!(paid(setup));
}

#[test]
fn the_reward_is_paid_when_the_giver_gave_an_item_in_a_trade() {
    let setup = "table.insert(records.trades, { at = 2000, with = 'Corvin-Stormrage', got = {}, gave = { ['Silk Cloth'] = 5 }, money = 0 })";

    assert!(paid(setup));
}

#[test]
fn a_trade_where_the_giver_only_got_items_pays_nothing() {
    let setup = "table.insert(records.trades, { at = 2000, with = 'Corvin-Stormrage', got = { ['Linen Cloth'] = 10 }, gave = {}, money = 0 })";

    assert!(!paid(setup));
}

#[test]
fn a_trade_from_before_the_task_pays_nothing() {
    let setup = "table.insert(records.trades, { at = 900, with = 'Corvin-Stormrage', got = {}, gave = {}, money = 50000 })";

    assert!(!paid(setup));
}
