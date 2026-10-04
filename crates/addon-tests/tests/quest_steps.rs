//! The open steps of your side quests that the addon watches (docs/plans/quest-variety.md
//! 10.2).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::Input;

/// A journal with one quest in progress and these steps.
fn journal(steps: &str) -> String {
    format!(
        r#"{{"type":"journal","page":0,"pages":1,"quests":[{{"number":1,"status":"accepted","steps":[{steps}]}}]}}"#
    )
}

const BATS: &str = r#"{"goal":"kill","creature":"Duskbat","count":2,"kills":0"#;
const RATS: &str = r#"{"goal":"kill","creature":"Mill Rat","count":2,"kills":0"#;

fn step(goal: &str, state: &str) -> String {
    format!(r#"{goal},"state":"{state}"}}"#)
}

fn hunted(game: &Game) -> Vec<String> {
    game.eval(
        "local out = {}
         for creature in pairs(ns.QuestSteps.Hunted()) do table.insert(out, creature) end
         table.sort(out)
         return out",
    )
}

/// A batch of events goes out, and the desktop answers it.
fn batch(game: &Game) {
    game.run(
        "wow.zone, wow.subzone = 'Duskwood', 'Mill Pond'
         wow.Fire('ZONE_CHANGED')
         wow.RunTickers()",
    );
}

fn journal_asks(game: &Game) -> usize {
    game.sent_inputs()
        .iter()
        .filter(|input| matches!(input, Input::JournalAsked { .. }))
        .count()
}

#[test]
fn every_open_kill_step_hunts_its_creature() {
    let game = Game::new();

    game.reply(&journal(&format!(
        "{},{}",
        step(BATS, "open"),
        step(RATS, "open")
    )));

    assert_eq!(hunted(&game), ["Duskbat", "Mill Rat"]);
}

#[test]
fn a_kill_step_that_is_not_open_hunts_nothing() {
    let game = Game::new();

    game.reply(&journal(&format!(
        "{},{}",
        step(BATS, "done"),
        step(RATS, "later")
    )));

    assert!(hunted(&game).is_empty());
}

#[test]
fn a_later_kill_step_makes_the_addon_ask_for_the_journal_after_a_batch() {
    let game = Game::new();
    let visit = r#"{"goal":"visit","place":"Mill Pond""#;
    game.reply(&journal(&format!(
        "{},{}",
        step(visit, "open"),
        step(BATS, "later")
    )));

    batch(&game);

    assert_eq!(journal_asks(&game), 1);
}
