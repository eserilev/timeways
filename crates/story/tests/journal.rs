#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::MessageId;
use timeways_story::journal::{Deed, Journal, PAGE_BYTES, Person, Place, journal, pages};
use timeways_story::story::Output;

fn place(name: &str, within: Option<&str>, first_visit: u64) -> Place {
    Place {
        name: name.to_string(),
        within: within.map(str::to_string),
        first_visit: Tick(first_visit),
    }
}

fn level(from: Option<i64>, to: i64, at: u64, place: Option<&str>) -> Deed {
    Deed::Level {
        from,
        to,
        at: Tick(at),
        place: place.map(str::to_string),
    }
}

#[test]
fn a_new_character_has_an_empty_journal() {
    assert_eq!(journal(&Character::new()), Journal::default());
}

#[test]
fn places_come_in_the_order_of_the_first_visit_with_their_zone() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.enter_zone(Tick(20), "Westfall", None).unwrap();

    let places = journal(&character).places;

    let expected = [
        place("Elwynn Forest", None, 10),
        place("Goldshire", Some("Elwynn Forest"), 10),
        place("Westfall", None, 20),
    ];
    assert_eq!(places, expected);
}

#[test]
fn a_second_visit_keeps_the_time_of_the_first() {
    let mut character = Character::new();
    character.enter_zone(Tick(10), "Westfall", None).unwrap();
    character
        .enter_zone(Tick(20), "Elwynn Forest", None)
        .unwrap();
    character.enter_zone(Tick(30), "Westfall", None).unwrap();

    let places = journal(&character).places;

    assert_eq!(
        places,
        [
            place("Westfall", None, 10),
            place("Elwynn Forest", None, 20)
        ]
    );
}

#[test]
fn people_carry_the_place_where_you_met_them() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.meet_npc(Tick(15), "Innkeeper Farley").unwrap();
    character.meet_npc(Tick(16), "Marshal Dughan").unwrap();

    let people = journal(&character).people;

    let person = |name: &str, first_met| Person {
        name: name.to_string(),
        place: Some("Goldshire".to_string()),
        first_met: Tick(first_met),
    };
    assert_eq!(
        people,
        [person("Innkeeper Farley", 15), person("Marshal Dughan", 16)]
    );
}

#[test]
fn the_first_level_begins_the_deeds_and_each_level_up_follows_with_its_place() {
    let mut character = Character::new();
    character.reach_level(Tick(5), 12).unwrap();
    character.enter_zone(Tick(10), "Westfall", None).unwrap();
    character.reach_level(Tick(20), 13).unwrap();
    character.enter_zone(Tick(30), "Duskwood", None).unwrap();
    character.reach_level(Tick(40), 14).unwrap();

    let deeds = journal(&character).deeds;

    let expected = [
        level(None, 12, 5, None),
        level(Some(12), 13, 20, Some("Westfall")),
        level(Some(13), 14, 40, Some("Duskwood")),
    ];
    assert_eq!(deeds, expected);
}

#[test]
fn a_journal_serializes_with_a_kind_on_each_deed() {
    let mut character = Character::new();
    character.reach_level(Tick(5), 12).unwrap();

    let json = serde_json::to_value(journal(&character)).unwrap();

    let expected = serde_json::json!({
        "places": [],
        "people": [],
        "deeds": [{ "kind": "level", "from": null, "to": 12, "at": 5, "place": null }],
    });
    assert_eq!(json, expected);
}

fn big_journal() -> Journal {
    let mut character = Character::new();
    for n in 0..400 {
        let zone = format!("A zone with a long name, so that pages fill fast, number {n}");
        character
            .enter_zone(Tick(n), &zone, Some(&format!("{zone}, the subzone")))
            .unwrap();
        character
            .meet_npc(Tick(n), &format!("Someone met in zone {n}"))
            .unwrap();
    }
    character.reach_level(Tick(500), 60).unwrap();
    journal(&character)
}

#[test]
fn a_small_journal_is_one_page() {
    let mut character = Character::new();
    character.enter_zone(Tick(1), "Westfall", None).unwrap();

    let pages = pages(journal(&character));

    assert_eq!(pages.len(), 1);
    assert_eq!((pages[0].page, pages[0].pages), (0, 1));
}

#[test]
fn an_empty_journal_is_one_empty_page() {
    let pages = pages(Journal::default());

    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].journal, Journal::default());
}

#[test]
fn every_page_line_fits_in_one_reply() {
    let pages = pages(big_journal());

    assert!(pages.len() > 2, "{} pages", pages.len());
    for page in pages {
        let output = Output::Journal {
            id: MessageId(u64::MAX),
            page,
        };
        let line = serde_json::to_string(&output).unwrap();
        assert!(line.len() <= PAGE_BYTES, "{} bytes", line.len());
    }
}

#[test]
fn the_pages_joined_are_the_whole_journal_in_order() {
    let whole = big_journal();

    let mut joined = Journal::default();
    for page in pages(whole.clone()) {
        joined.places.extend(page.journal.places);
        joined.people.extend(page.journal.people);
        joined.deeds.extend(page.journal.deeds);
    }

    assert_eq!(joined, whole);
}
