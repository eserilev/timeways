//! The past of the character before Timeways saw it (GAMEPLAY.md 3.3, the prologue): the
//! addon reads it once, when the journal asks for it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use timeways_story::input::Input;
use timeways_story::past::{Gear, Past, Profession, Standing};

const WANTED: &str = r#"{"type":"journal","page":0,"pages":1,"past":"wanted"}"#;
const NOT_WANTED: &str = r#"{"type":"journal","page":0,"pages":1}"#;

/// A human paladin at level 35 in Stranglethorn Vale, with a long past.
fn veteran() -> Game {
    let game = Game::new();
    game.run(
        "wow.units.player = { name = 'Ada', level = 35 }
         wow.zone = 'Stranglethorn Vale'
         wow.questsDone = { [100] = 'Wanted: Hogger', [300] = 'The Defias Brotherhood', [200] = 'Red Linen Goods' }
         wow.maps = {
             [1429] = { name = 'Elwynn Forest', type = Enum.UIMapType.Zone, explored = { 1, 2 } },
             [1436] = { name = 'Westfall', type = Enum.UIMapType.Zone, explored = { 1 } },
             [1434] = { name = 'Stranglethorn Vale', type = Enum.UIMapType.Zone, explored = { 1 } },
             [1418] = { name = 'Badlands', type = Enum.UIMapType.Zone, explored = {} },
             [1415] = { name = 'Eastern Kingdoms', type = Enum.UIMapType.Continent, explored = { 1 } },
         }
         wow.factions = {
             { name = 'Alliance', isHeader = true, reaction = 4 },
             { name = 'Stormwind', reaction = 6 },
             { name = 'Ironforge', reaction = 5 },
             { name = 'Booty Bay', reaction = 4 },
             { name = 'Bloodsail Buccaneers', reaction = 2 },
         }
         wow.skills = {
             { name = 'Professions', isHeader = true, rank = 0, skillLineCategoryID = 11 },
             { name = 'Mining', isHeader = false, rank = 150, skillLineCategoryID = 11 },
             { name = 'Fishing', isHeader = false, rank = 75, skillLineCategoryID = 9 },
             { name = 'Swords', isHeader = false, rank = 175, skillLineCategoryID = 6 },
         }
         wow.mounts = {
             { name = 'Brown Horse', collected = true },
             { name = 'Swift Palomino', collected = false },
             { name = 'Black Stallion', collected = true, hidden = true },
         }
         wow.items['Blackened Defias Belt'] = { quality = 3, level = 25 }
         wow.items['Thunderbrew Shirt'] = { quality = 3, level = 1 }
         wow.gear[6] = 'Blackened Defias Belt'
         wow.gear[4] = 'Thunderbrew Shirt'",
    );
    game
}

/// Runs the one-shot timers of the addon, as the clock passes them.
fn run_timers(game: &Game) {
    game.run(
        "local timers = wow.after
         wow.after = {}
         for _, timer in ipairs(timers) do timer.callback() end",
    );
}

fn pasts(game: &Game) -> Vec<Past> {
    game.run("ns.Outbox.Flush()");
    game.sent_inputs()
        .into_iter()
        .filter_map(|input| match input {
            Input::PastRead(past) => Some(past),
            _ => None,
        })
        .collect()
}

#[test]
fn a_journal_that_wants_the_past_gets_one_past_line() {
    let game = veteran();

    game.reply(WANTED);
    game.run("wow.Fire('TIME_PLAYED_MSG', 400000, 3000)");
    run_timers(&game);

    let pasts = pasts(&game);
    assert_eq!(pasts.len(), 1);
    let past = &pasts[0];
    assert_eq!(past.level, 35);
    assert_eq!(past.zone.as_deref(), Some("Stranglethorn Vale"));
    assert_eq!(past.played, Some(400_000));
    assert_eq!(past.quests, 3);
    assert_eq!(
        past.quest_titles,
        [
            "The Defias Brotherhood",
            "Red Linen Goods",
            "Wanted: Hogger"
        ]
    );
    assert_eq!(
        past.zones,
        ["Elwynn Forest", "Stranglethorn Vale", "Westfall"]
    );
    assert_eq!(past.mounts, ["Brown Horse"]);
    assert_eq!(past.gear, Gear { rare: 1, epic: 0 });
}

#[test]
fn the_factions_furthest_from_neutral_come_first() {
    let game = veteran();

    game.reply(WANTED);
    run_timers(&game);

    let factions = &pasts(&game)[0].factions;
    let standing = |name: &str, standing: u8| Standing {
        name: name.to_string(),
        standing,
    };
    assert_eq!(
        *factions,
        [
            standing("Bloodsail Buccaneers", 2),
            standing("Stormwind", 6),
            standing("Ironforge", 5),
        ]
    );
}

#[test]
fn only_professions_and_secondary_skills_go() {
    let game = veteran();

    game.reply(WANTED);
    run_timers(&game);

    let profession = |name: &str, rank: u16| Profession {
        name: name.to_string(),
        rank,
    };
    assert_eq!(
        pasts(&game)[0].professions,
        [profession("Mining", 150), profession("Fishing", 75)]
    );
}

#[test]
fn the_past_is_asked_for_once_in_a_session() {
    let game = veteran();

    game.reply(WANTED);
    game.reply(WANTED);
    run_timers(&game);

    assert_eq!(pasts(&game).len(), 1);
    assert_eq!(game.eval::<u32>("wow.playedAsks"), 1);
}

#[test]
fn a_journal_that_wants_no_past_reads_nothing() {
    let game = veteran();

    game.reply(NOT_WANTED);
    run_timers(&game);

    assert!(pasts(&game).is_empty());
    assert_eq!(game.eval::<u32>("wow.playedAsks"), 0);
}

#[test]
fn a_hidden_value_never_goes_out() {
    let game = veteran();
    game.run(
        "wow.secrets['Stranglethorn Vale'] = true
         wow.secrets['Stormwind'] = true
         wow.zone = wow.Hidden('Stranglethorn Vale')
         wow.factions[2].name = wow.Hidden('Stormwind')",
    );

    game.reply(WANTED);
    run_timers(&game);

    let past = &pasts(&game)[0];
    assert_eq!(past.zone, None);
    assert!(
        past.factions
            .iter()
            .all(|faction| faction.name != "Stormwind")
    );
}

#[test]
fn a_new_character_sends_no_empty_list() {
    let game = Game::new();
    game.run("wow.units.player = { name = 'Ada', level = 1 }; wow.zone = 'Elwynn Forest'");

    game.reply(WANTED);
    run_timers(&game);

    let past = &pasts(&game)[0];
    assert_eq!(past.level, 1);
    assert_eq!(past.quests, 0);
    assert!(past.zones.is_empty());
}
