//! The items that you put on (GAMEPLAY.md 5.4).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::{Input, SlotWas};

/// A fighter with a green sword of level 20 in the main hand (16) and nothing in a ring
/// slot (11), at login.
fn logged_in() -> Game {
    let game = Game::new();
    game.run(
        "wow.items['Old Sword'] = { quality = 2, level = 20 }
         wow.items['Cruel Barb'] = { quality = 3, level = 30 }
         wow.items['Shiny Sword'] = { quality = 3, level = 25 }
         wow.items['Barman Shanker'] = { quality = 4, level = 47 }
         wow.items['Green Ring'] = { quality = 2, level = 40 }
         wow.gear[16] = 'Old Sword'
         ns.Gear.Login()",
    );
    game
}

fn put_on(game: &Game, slot: u8, item: &str) {
    game.run(&format!(
        "wow.gear[{slot}] = '{item}'; wow.Fire('PLAYER_EQUIPMENT_CHANGED', {slot}, false)"
    ));
}

type Sent = (u8, String, u8, Option<u16>, Option<u16>, SlotWas);

fn sent(game: &Game) -> Vec<Sent> {
    game.run("wow.RunTickers()");
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::ItemEquipped {
                slot,
                item,
                quality,
                level,
                replaced,
                was,
                ..
            } => Some((slot, item, quality, level, replaced, was)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_rare_item_goes_with_the_level_of_what_it_replaced() {
    let game = logged_in();

    put_on(&game, 16, "Cruel Barb");

    assert_eq!(
        sent(&game),
        [(
            16,
            "Cruel Barb".to_string(),
            3,
            Some(30),
            Some(20),
            SlotWas::Worn
        )]
    );
}

#[test]
fn a_slot_empty_since_the_login_says_so() {
    let game = logged_in();

    put_on(&game, 11, "Barman Shanker");

    assert_eq!(
        sent(&game),
        [(
            11,
            "Barman Shanker".to_string(),
            4,
            Some(47),
            None,
            SlotWas::Empty
        )]
    );
}

#[test]
fn an_uncommon_item_or_the_same_item_again_sends_nothing() {
    let game = logged_in();

    put_on(&game, 11, "Green Ring");
    put_on(&game, 16, "Cruel Barb");
    put_on(&game, 16, "Cruel Barb");

    assert_eq!(sent(&game).len(), 1);
}

#[test]
fn a_slot_taken_off_and_filled_again_compares_with_its_last_item() {
    let game = logged_in();

    game.run("wow.gear[16] = nil; wow.Fire('PLAYER_EQUIPMENT_CHANGED', 16, true)");
    put_on(&game, 16, "Shiny Sword");

    assert_eq!(
        sent(&game),
        [(
            16,
            "Shiny Sword".to_string(),
            3,
            Some(25),
            Some(20),
            SlotWas::Worn
        )]
    );
}

#[test]
fn the_shirt_the_tabard_and_an_unknown_item_send_nothing() {
    let game = logged_in();

    put_on(&game, 4, "Cruel Barb");
    put_on(&game, 19, "Cruel Barb");
    put_on(&game, 16, "Not Loaded Yet");

    assert!(sent(&game).is_empty());
}

#[test]
fn a_hidden_value_or_a_broken_event_sends_nothing() {
    let game = logged_in();

    game.run("wow.secrets['Cruel Barb'] = true");
    put_on(&game, 16, "Cruel Barb");
    game.run("wow.Fire('PLAYER_EQUIPMENT_CHANGED', 'x', false)");
    game.run("wow.Fire('PLAYER_EQUIPMENT_CHANGED', 99, false)");

    assert!(sent(&game).is_empty());
}

#[test]
fn an_item_level_that_is_no_whole_number_goes_with_no_level() {
    let game = logged_in();
    game.run("wow.items['Odd Blade'] = { quality = 3, level = 30.5 }");

    put_on(&game, 16, "Odd Blade");

    assert_eq!(
        sent(&game),
        [(
            16,
            "Odd Blade".to_string(),
            3,
            None,
            Some(20),
            SlotWas::Worn
        )]
    );
}

/// The game sends the data of an item that it had none on.
fn data_comes(game: &Game, item: &str, quality: u8, level: u16) {
    game.run(&format!(
        "wow.items['{item}'] = {{ quality = {quality}, level = {level} }}
         wow.Fire('GET_ITEM_INFO_RECEIVED', 1, true)"
    ));
}

#[test]
fn an_item_with_no_data_yet_goes_out_once_the_data_comes() {
    let game = logged_in();
    put_on(&game, 16, "Late Blade");
    let before_the_data = sent(&game);

    data_comes(&game, "Late Blade", 4, 50);

    assert!(before_the_data.is_empty(), "{before_the_data:?}");
    assert_eq!(
        sent(&game),
        [(
            16,
            "Late Blade".to_string(),
            4,
            Some(50),
            Some(20),
            SlotWas::Worn
        )]
    );
}

#[test]
fn an_item_whose_data_never_comes_in_time_is_dropped() {
    let game = logged_in();
    put_on(&game, 16, "Late Blade");

    game.run("wow.now = wow.now + 61");
    data_comes(&game, "Late Blade", 4, 50);

    assert!(sent(&game).is_empty());
}

#[test]
fn a_slot_that_waited_too_long_still_sends_its_next_item() {
    let game = logged_in();
    put_on(&game, 16, "Late Blade");
    game.run("wow.now = wow.now + 61");

    data_comes(&game, "Late Blade", 4, 50);
    put_on(&game, 16, "Cruel Barb");

    assert_eq!(sent(&game).len(), 1);
}

#[test]
fn gear_with_no_data_at_login_gives_its_level_once_the_data_comes() {
    let game = Game::new();
    game.run(
        "wow.items['Cruel Barb'] = { quality = 3, level = 30 }
         wow.gear[16] = 'Old Sword'
         ns.Gear.Login()",
    );
    data_comes(&game, "Old Sword", 2, 20);

    put_on(&game, 16, "Cruel Barb");

    assert_eq!(sent(&game)[0].4, Some(20));
}

#[test]
fn an_equip_waits_for_the_data_of_the_item_that_it_replaced() {
    let game = Game::new();
    game.run(
        "wow.items['Cruel Barb'] = { quality = 3, level = 30 }
         wow.gear[16] = 'Old Sword'
         ns.Gear.Login()",
    );
    put_on(&game, 16, "Cruel Barb");

    data_comes(&game, "Old Sword", 2, 20);

    assert_eq!(sent(&game)[0].4, Some(20));
}

/// The game gives no quality, then a hidden level. The check of hidden values must not stop
/// at the first field that is nil.
#[test]
fn a_hidden_level_after_a_field_with_no_value_is_never_sent() {
    let game = logged_in();
    game.run("wow.items['Odd Ring'] = { level = 33 }; wow.secrets[33] = true");
    put_on(&game, 16, "Odd Ring");

    put_on(&game, 16, "Cruel Barb");

    assert_eq!(sent(&game)[0].4, None);
}
