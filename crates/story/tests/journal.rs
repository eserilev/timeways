#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::MessageId;
use timeways_story::journal::{Chapter, Deed, Journal, PAGE_BYTES, Person, Place, journal, pages};
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
        "chapters": [{
            "number": 1,
            "began": 5,
            "zones": [],
            "people": [],
            "deeds": [{ "kind": "level", "from": null, "to": 12, "at": 5, "place": null }],
            "left_out": 0,
        }],
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
        joined.chapters.extend(page.journal.chapters);
        joined.places.extend(page.journal.places);
        joined.people.extend(page.journal.people);
        joined.deeds.extend(page.journal.deeds);
    }

    assert_eq!(joined, whole);
}

#[test]
fn the_first_kill_and_each_echo_are_deeds_with_their_count_and_place() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Elwynn Forest", None)
        .unwrap();
    character.defeat_npc(Tick(20), "Hogger").unwrap();
    character.defeat_npc(Tick(30), "Hogger").unwrap();

    let deeds = journal(&character).deeds;

    let defeated = |times, at| Deed::Defeated {
        foe: "Hogger".to_string(),
        times,
        at: Tick(at),
        place: Some("Elwynn Forest".to_string()),
    };
    assert_eq!(deeds, [defeated(1, 20), defeated(2, 30)]);
}

#[test]
fn each_death_is_a_deed_with_its_killer_when_known() {
    let mut character = Character::new();
    character.enter_zone(Tick(10), "Westfall", None).unwrap();
    character.die(Tick(20), Some("Defias Pillager")).unwrap();
    character.die(Tick(30), None).unwrap();

    let deeds = journal(&character).deeds;

    let died = |killer: Option<&str>, at| Deed::Died {
        killer: killer.map(str::to_string),
        at: Tick(at),
        place: Some("Westfall".to_string()),
    };
    assert_eq!(deeds, [died(Some("Defias Pillager"), 20), died(None, 30)]);
}

#[test]
fn a_death_is_not_a_kill_of_yours() {
    let mut character = Character::new();

    character.die(Tick(1), Some("Hogger")).unwrap();

    let deeds = journal(&character).deeds;
    assert!(
        deeds
            .iter()
            .all(|deed| !matches!(deed, Deed::Defeated { .. })),
        "{deeds:?}"
    );
}

const HOUR: u64 = 3600;

#[test]
fn one_session_is_one_chapter_with_its_new_zones_people_and_deeds() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(HOUR), "Westfall", Some("Moonbrook"))
        .unwrap();
    character
        .meet_npc(Tick(HOUR + 60), "Gryan Stoutmantle")
        .unwrap();
    character
        .defeat_npc(Tick(HOUR + 120), "Mother Fang")
        .unwrap();

    let chapters = journal(&character).chapters;

    let kill = Deed::Defeated {
        foe: "Mother Fang".to_string(),
        times: 1,
        at: Tick(HOUR + 120),
        place: Some("Moonbrook".to_string()),
    };
    let expected = Chapter {
        number: 1,
        began: Tick(HOUR),
        zones: vec!["Westfall".to_string()],
        people: vec!["Gryan Stoutmantle".to_string()],
        deeds: vec![kill],
        left_out: 0,
    };
    assert_eq!(chapters, [expected]);
}

#[test]
fn a_long_pause_starts_a_new_chapter() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    character
        .enter_zone(Tick(HOUR + 20 * 60), "Duskwood", None)
        .unwrap();
    character
        .enter_zone(Tick(5 * HOUR), "Redridge Mountains", None)
        .unwrap();

    let chapters = journal(&character).chapters;

    let summary: Vec<(usize, Vec<String>)> = chapters
        .into_iter()
        .map(|chapter| (chapter.number, chapter.zones))
        .collect();
    let expected = vec![
        (1, vec!["Westfall".to_string(), "Duskwood".to_string()]),
        (2, vec!["Redridge Mountains".to_string()]),
    ];
    assert_eq!(summary, expected);
}

#[test]
fn a_session_with_nothing_new_has_no_chapter_and_leaves_no_gap_in_the_numbers() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    character
        .enter_zone(Tick(HOUR + 60), "Duskwood", None)
        .unwrap();
    character
        .enter_zone(Tick(5 * HOUR), "Westfall", None)
        .unwrap();
    character
        .enter_zone(Tick(9 * HOUR), "Redridge Mountains", None)
        .unwrap();

    let chapters = journal(&character).chapters;

    let numbers: Vec<usize> = chapters.iter().map(|chapter| chapter.number).collect();
    assert_eq!(numbers, [1, 2]);
    assert_eq!(chapters[1].zones, ["Redridge Mountains"]);
}

#[test]
fn a_chapter_keeps_thirty_entries_of_each_list_and_counts_the_rest() {
    let mut character = Character::new();
    for n in 0..35 {
        character
            .enter_zone(Tick(HOUR + n), &format!("Zone {n}"), None)
            .unwrap();
    }

    let chapters = journal(&character).chapters;

    assert_eq!(chapters[0].zones.len(), 30);
    assert_eq!(chapters[0].left_out, 5);
}
