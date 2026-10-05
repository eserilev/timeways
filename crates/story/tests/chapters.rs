//! The walk and the book of the chapters (docs/plans/chapters.md 3 to 6, and 13), through
//! the world of a character.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::{EventId, Tick};
use timeways_rules::chapters::{Opening, Track};
use timeways_rules::weights::RULE_ONE;
use timeways_story::chapters::{Book, CURRENT_RULE, SpanState, new_epoch};
use timeways_story::character::{Character, Resting};
use timeways_story::input::GameQuestKind;
use timeways_story::journal::{Deed, EntryState, OpenedBy, journal};
use timeways_story::places::{InstanceKind, PlaceKind};
use timeways_story::walk::RuleRow;

const HOUR: u64 = 3600;

/// `count` quests of the game turned in, one minute apart from `at`: a weight of `count`.
fn quests(character: &mut Character, at: u64, count: u64, prefix: &str) {
    for n in 0..count {
        let title = format!("{prefix} {n}");
        character
            .finish_game_quest(Tick(at + n * 60), &title, GameQuestKind::Normal)
            .unwrap();
    }
}

fn titles(character: &Character) -> Vec<Option<String>> {
    journal(character)
        .chapters
        .into_iter()
        .map(|chapter| chapter.title)
        .collect()
}

#[test]
fn a_flight_across_four_zones_makes_no_chapter() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    quests(&mut character, HOUR, 20, "Westfall");

    for (n, zone) in [
        "Duskwood",
        "Redridge Mountains",
        "Elwynn Forest",
        "Westfall",
    ]
    .into_iter()
    .enumerate()
    {
        character
            .enter_zone(Tick(2 * HOUR + n as u64 * 60), zone, Some("Flight Path"))
            .unwrap();
    }

    assert_eq!(titles(&character), [Some("Westfall".to_string())]);
}

#[test]
fn a_new_zone_with_weight_after_the_least_weight_starts_a_chapter() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    quests(&mut character, HOUR, 15, "Westfall");

    character
        .enter_zone(Tick(2 * HOUR), "Duskwood", None)
        .unwrap();
    quests(&mut character, 2 * HOUR, 1, "Duskwood");
    let chapters = journal(&character).chapters;

    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].state, EntryState::Closed);
    assert_eq!(chapters[1].title.as_deref(), Some("Duskwood"));
    assert_eq!(chapters[1].opened_by, OpenedBy::NewZone);
    assert_eq!(chapters[1].state, EntryState::Open);
}

#[test]
fn a_subzone_seen_on_arrival_counts_nothing_and_one_after_a_quest_counts_one() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(HOUR), "Westfall", Some("Sentinel Hill"))
        .unwrap();
    quests(&mut character, HOUR, 1, "Westfall");

    character
        .enter_zone(Tick(HOUR + 600), "Westfall", Some("Moonbrook"))
        .unwrap();
    let book = Book::of(&character);

    assert_eq!(book.fold().open.weight, 2);
}

#[test]
fn a_dungeon_entry_opens_a_tale_that_follows_its_chapter() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    quests(&mut character, HOUR, 3, "Westfall");

    character
        .enter_zone(Tick(2 * HOUR), "The Deadmines", None)
        .unwrap();
    character
        .mark_instance(Tick(2 * HOUR), "The Deadmines", InstanceKind::Dungeon)
        .unwrap();
    character
        .defeat_npc(Tick(2 * HOUR + 60), "Edwin VanCleef")
        .unwrap();
    let journal = journal(&character);

    assert_eq!(journal.tales.len(), 1);
    let tale = &journal.tales[0];
    assert_eq!(tale.instance, "The Deadmines");
    assert_eq!(tale.chapter, journal.chapters[0].first);
    assert_eq!(tale.deeds.len(), 1);
    let in_chapter = &journal.chapters[0].deeds;
    assert!(
        !in_chapter
            .iter()
            .any(|deed| matches!(deed, Deed::Defeated { .. }))
    );
}

#[test]
fn a_level_inside_a_dungeon_counts_for_the_chapter() {
    let mut character = Character::new();
    character.reach_level(Tick(HOUR), 17).unwrap();
    character
        .enter_zone(Tick(HOUR), "The Deadmines", None)
        .unwrap();
    character
        .mark_instance(Tick(HOUR), "The Deadmines", InstanceKind::Dungeon)
        .unwrap();

    character.reach_level(Tick(HOUR + 60), 18).unwrap();
    let book = Book::of(&character);
    let level = book.fold().gains.last().copied().unwrap();

    assert_eq!(level.track, Track::World);
    assert_eq!(level.amount, 2);
    assert_eq!(book.fold().open.weight, 2);
}

#[test]
fn leaving_a_capital_waits_and_cuts_at_the_next_step_with_weight() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(HOUR), "Stormwind City", None)
        .unwrap();
    quests(&mut character, HOUR, 15, "Stormwind");
    character
        .enter_zone(Tick(2 * HOUR), "Elwynn Forest", None)
        .unwrap();
    character
        .enter_zone(Tick(2 * HOUR + 60), "Stormwind City", None)
        .unwrap();

    quests(&mut character, 2 * HOUR + 120, 1, "Back");
    let book = Book::of(&character);

    assert_eq!(book.fold().closed.len(), 1);
    assert_eq!(
        book.fold().open.opening,
        Opening::Break(timeways_rules::chapters::Break::Capital)
    );
}

#[test]
fn a_world_with_no_rule_row_gets_the_first_rule_from_its_first_event() {
    let row = new_epoch(&[], EventId(40), CURRENT_RULE);

    assert_eq!(
        row,
        Some(RuleRow {
            rule: CURRENT_RULE,
            from: EventId(0)
        })
    );
}

#[test]
fn a_world_of_the_current_rule_needs_no_new_row() {
    let rows = [RuleRow {
        rule: CURRENT_RULE,
        from: EventId(0),
    }];

    assert_eq!(new_epoch(&rows, EventId(40), CURRENT_RULE), None);
}

#[test]
fn a_world_of_an_older_rule_gets_the_new_rule_from_its_next_event() {
    let rows = [RuleRow {
        rule: RULE_ONE,
        from: EventId(0),
    }];

    let row = new_epoch(&rows, EventId(40), RULE_ONE + 1);

    assert_eq!(
        row,
        Some(RuleRow {
            rule: RULE_ONE + 1,
            from: EventId(40)
        })
    );
}

#[test]
fn a_new_rule_closes_the_open_chapter_and_keeps_the_ones_before() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    quests(&mut character, HOUR, 5, "Westfall");
    let from = EventId(character.world().history().len() as u64);
    quests(&mut character, 2 * HOUR, 5, "Later");
    let rules = vec![
        RuleRow {
            rule: RULE_ONE,
            from: EventId(0),
        },
        RuleRow {
            rule: RULE_ONE + 1,
            from,
        },
    ];

    let mut book = Book::new(character.you(), rules);
    book.catch_up(character.world().history());
    let chapters = book.chapters();

    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].state, SpanState::Closed);
    assert_eq!(chapters[0].weight, 5);
    assert_eq!(chapters[1].first, from);
    assert_eq!(chapters[1].opening, Opening::Rule);
}

#[test]
fn folding_one_event_at_a_time_gives_the_same_book_as_the_whole_history() {
    let mut whole = Character::new();
    let mut book = Book::new(whole.you(), Vec::new());
    whole.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    for n in 0..40 {
        quests(&mut whole, HOUR + n * 60, 1, &format!("Quest {n}"));
        book.catch_up(whole.world().history());
    }

    let once = Book::of(&whole);

    assert_eq!(book.chapters(), once.chapters());
    assert_eq!(book.fold(), once.fold());
}

/// docs/plans/chapters.md 14, step 3: the time to fold a world of 50,000 events when it
/// opens. Run it with `--ignored --nocapture` in a release build to see the time.
#[test]
#[ignore = "a measurement, not a law"]
fn a_world_of_fifty_thousand_events_folds_in_well_under_a_second() {
    let mut character = Character::new();
    let mut at = HOUR;
    while character.world().history().len() < 50_000 {
        let zone = format!("Zone {}", at / HOUR % 40);
        character
            .enter_zone(Tick(at), &zone, Some(&format!("Spot {}", at % 97)))
            .unwrap();
        character
            .finish_game_quest(Tick(at), &format!("Quest {at}"), GameQuestKind::Normal)
            .unwrap();
        at += 60;
    }

    let started = std::time::Instant::now();
    let book = Book::of(&character);
    let folded = started.elapsed();
    let journal = timeways_story::journal::journal_of(&character, &book);
    let built = started.elapsed();

    eprintln!(
        "{} events, {} chapters: fold {folded:?}, fold and journal {built:?}",
        character.world().history().len(),
        journal.chapters.len()
    );
    assert!(folded.as_secs() < 1);
}

#[test]
fn a_battleground_opens_a_tale_and_its_first_win_adds_weight_once() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(HOUR), "Warsong Gulch", None)
        .unwrap();
    character
        .mark_instance(Tick(HOUR), "Warsong Gulch", InstanceKind::Battleground)
        .unwrap();

    character
        .win_battleground(Tick(HOUR + 600), "Warsong Gulch")
        .unwrap();
    character
        .win_battleground(Tick(HOUR + 1200), "Warsong Gulch")
        .unwrap();
    let book = Book::of(&character);

    let visit = book.fold().visit.expect("the run is open");
    assert_eq!(visit.gain, 3 + 3);
    assert_eq!(book.fold().open.weight, 0);
    let journal = journal(&character);
    assert_eq!(journal.tales[0].kind, PlaceKind::Battleground);
    assert!(matches!(
        journal.tales[0].deeds[..],
        [Deed::WonBattle { .. }]
    ));
}

#[test]
fn a_world_boss_weighs_as_a_raid_boss() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Azshara", None).unwrap();

    character.mark_world_boss(Tick(HOUR), "Azuregos").unwrap();
    character.defeat_npc(Tick(HOUR), "Azuregos").unwrap();
    let book = Book::of(&character);

    assert_eq!(book.fold().open.weight, 5);
}

#[test]
fn leaving_an_inn_is_a_break() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    quests(&mut character, HOUR, 15, "Westfall");
    character.rest(Tick(2 * HOUR), Resting::Yes).unwrap();
    character.rest(Tick(2 * HOUR + 60), Resting::No).unwrap();

    quests(&mut character, 2 * HOUR + 120, 1, "After");
    let chapters = journal(&character).chapters;

    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[1].opened_by, OpenedBy::Inn);
}

#[test]
fn a_subzone_seen_from_a_flight_counts_later_on_foot() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Duskwood", None).unwrap();
    quests(&mut character, HOUR, 1, "Duskwood");
    character
        .fly_through(Tick(HOUR + 60), "Duskwood", Some("Darkshire"))
        .unwrap();
    let flown = Book::of(&character).fold().open.weight;

    character
        .enter_zone(Tick(HOUR + 120), "Duskwood", Some("Darkshire"))
        .unwrap();
    let walked = Book::of(&character).fold().open.weight;

    assert_eq!(flown, 1);
    assert_eq!(walked, 2);
}

#[test]
fn a_new_pvp_rank_weighs_two_and_the_rank_at_the_first_login_nothing() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Ashenvale", None).unwrap();
    character.reach_pvp_rank(Tick(HOUR), 2).unwrap();

    character.reach_pvp_rank(Tick(HOUR + 60), 3).unwrap();
    character.reach_pvp_rank(Tick(HOUR + 120), 1).unwrap();
    let book = Book::of(&character);

    assert_eq!(book.fold().open.weight, 2);
    let ranks: Vec<Deed> = journal(&character)
        .deeds
        .into_iter()
        .filter(|deed| matches!(deed, Deed::PvpRank { .. }))
        .collect();
    assert_eq!(ranks.len(), 1);
}
