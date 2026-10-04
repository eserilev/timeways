#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::hero::{Change, Hero, PROMPT_TEXT_CHARS, hero};
use timeways_story::hero_hook::{HOOK_EVERY, Hook, applies, hook};

fn set(field: &str, text: &str) -> Change {
    Change::Set {
        at: Tick(1),
        field: field.to_string(),
        text: text.to_string(),
    }
}

/// The field of the hook of each call, from the first call to the `calls`th.
fn hooked_fields(hero: &Hero, calls: u64) -> Vec<Option<String>> {
    (0..calls)
        .map(|count| hook(hero, count).map(|hook| hook.field.to_string()))
        .collect()
}

#[test]
fn the_third_call_gets_a_hook_and_the_two_before_it_do_not() {
    let hero = hero(&[set("goal", "Find my father.")]);

    let hooks: Vec<Option<Hook<'_>>> = (0..3).map(|count| hook(&hero, count)).collect();

    let goal = Hook {
        field: "goal",
        text: "Find my father.",
    };
    assert_eq!(hooks, [None, None, Some(goal)]);
}

#[test]
fn a_hook_comes_once_in_every_three_calls() {
    let applied: Vec<u64> = (0..12).filter(|count| applies(*count)).collect();

    assert_eq!(applied, [2, 5, 8, 11]);
    assert_eq!(HOOK_EVERY, 3);
}

#[test]
fn the_hook_rotates_through_the_filled_answers_in_the_order_of_the_sheet() {
    let hero = hero(&[
        set("traits", "Loud."),
        set("goal", "Find my father."),
        set("flaw", "Proud."),
    ]);

    let fields: Vec<String> = hooked_fields(&hero, 12).into_iter().flatten().collect();

    assert_eq!(fields, ["goal", "flaw", "traits", "goal"]);
}

#[test]
fn an_empty_sheet_gives_no_hook() {
    let hero = hero(&[]);

    assert!(hooked_fields(&hero, 9).iter().all(Option::is_none));
}

#[test]
fn origin_and_background_are_never_a_hook() {
    let hero = hero(&[
        set("origin", "Lakeshire."),
        set("background", "A farmer's son."),
        set("motto", "Onward."),
    ]);

    assert!(hooked_fields(&hero, 9).iter().all(Option::is_none));
}

#[test]
fn a_cleared_answer_is_never_a_hook() {
    let hero = hero(&[
        set("goal", "Find my father."),
        set("bond", "My sister."),
        set("goal", ""),
    ]);

    let fields: Vec<String> = hooked_fields(&hero, 9).into_iter().flatten().collect();

    assert_eq!(fields, ["bond", "bond", "bond"]);
}

#[test]
fn a_hook_takes_the_first_three_hundred_characters_of_its_answer() {
    let long = "é".repeat(PROMPT_TEXT_CHARS + 50);
    let hero = hero(&[set("goal", &long)]);

    let hook = hook(&hero, 2).unwrap();

    assert_eq!(hook.text.chars().count(), PROMPT_TEXT_CHARS);
}
