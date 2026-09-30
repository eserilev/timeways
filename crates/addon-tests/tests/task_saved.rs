//! The saved state of player tasks: any addon can write it, so the addon checks each task and
//! record when it first reads the file (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;

const GOOD_TASK: &str = "{ id = 'a1', giver = 'Ada-Stormrage', doer = 'Corvin-Stormrage',
    title = 'The Mill', text = 'Go look.', reward = '', status = 'accepted', sentAt = 100,
    steps = { { kind = 'place', target = 'Brill', count = 1 } },
    claims = { { at = 200, zone = 'Tirisfal Glades' }, { at = 300, zone = 'Extra' } } }";

/// The keys of the tasks that you got, after the addon reads this saved table.
fn kept(saved: &str) -> Vec<String> {
    let game = Game::new();
    game.eval(&format!(
        "TimewaysTasks = {saved}
         local keys = {{}}
         for key in pairs(ns.TaskStore.Data().received) do table.insert(keys, key) end
         table.sort(keys)
         return keys"
    ))
}

#[test]
fn a_good_task_stays_with_only_the_claims_of_its_steps() {
    let game = Game::new();

    let claims: usize = game.eval(&format!(
        "TimewaysTasks = {{ received = {{ ['Ada-Stormrage/a1'] = {GOOD_TASK} }} }}
         local claims = 0
         for _ in pairs(ns.TaskStore.Data().received['Ada-Stormrage/a1'].claims) do claims = claims + 1 end
         return claims"
    ));

    assert_eq!(claims, 1);
}

#[test]
fn a_task_with_a_broken_field_is_dropped() {
    let broken = [
        "title = 'A |cffff0000red|r title'",
        "title = 42",
        "status = 'stolen'",
        "steps = {}",
        "steps = { { kind = 'fly', target = 'Brill', count = 1 } }",
        "steps = { { kind = 'kill', target = 'Gregor', count = 0.5 } }",
        "sentAt = 'yesterday'",
        "giver = string.rep('a', 65)",
        "id = 'a/1'",
    ];
    for field in broken {
        let saved = format!(
            "{{ received = {{ ['Ada-Stormrage/a1'] = {GOOD_TASK}, ['Ada-Stormrage/b2'] = {GOOD_TASK} }} }}
             TimewaysTasks.received['Ada-Stormrage/b2'].{field}"
        );
        assert_eq!(kept(&saved), ["Ada-Stormrage/a1"], "{field}");
    }
}

#[test]
fn a_record_with_a_broken_field_is_dropped() {
    let game = Game::new();

    let counts: Vec<usize> = game.eval(
        "TimewaysTasks = {
             kills = { { at = 1, doer = 'Corvin-Stormrage', target = 'Gregor' }, { at = 'x' }, 'kill' },
             trades = { { at = 1, with = 'Corvin-Stormrage', gave = { ['Linen Cloth'] = 'ten' }, got = {}, money = 0, moneyGot = 0 } },
             party = { ['Corvin-Stormrage'] = { { from = 1 }, { from = 'x' } }, [5] = {} },
         }
         local data = ns.TaskStore.Data()
         local names = 0
         for _ in pairs(data.party) do names = names + 1 end
         return { #data.kills, #data.trades, #data.party['Corvin-Stormrage'], names }",
    );

    assert_eq!(counts, [1, 0, 1, 1]);
}

#[test]
fn a_saved_value_that_is_no_table_starts_the_state_afresh() {
    let game = Game::new();

    let given: usize = game.eval(
        "TimewaysTasks = { given = 'all of them', nextId = -3 }
         local data = ns.TaskStore.Data()
         return #ns.TaskStore.List(data.given) + data.nextId",
    );

    assert_eq!(given, 0);
}

#[test]
fn only_the_newest_hundred_records_of_a_kind_stay() {
    let game = Game::new();

    let kept: Vec<u32> = game.eval(
        "for n = 1, 150 do
             ns.TaskStore.Record('npcs', { at = n, name = 'Innkeeper Renee' })
         end
         local npcs = ns.TaskStore.Data().npcs
         return { #npcs, npcs[1].at }",
    );

    assert_eq!(kept, [100, 51]);
}
