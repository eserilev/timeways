#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use timeways_story::character::Character;
use timeways_story::input::{GameQuestKind, MessageId};
use timeways_story::journal::{Chapter, Deed, Journal, Person, Place, journal, pages};
use timeways_story::learned::{Read, learned};
use timeways_story::places::{InstanceKind, PlaceKind};
use timeways_story::reply_size::MAX_LINE;
use timeways_story::seen::{MAX_SEEN_BYTES, SeenText, TextKind};
use timeways_story::story::{MAX_NAME_BYTES, Output};

fn place(name: &str, within: Option<&str>, first_visit: u64) -> Place {
    Place {
        name: name.to_string(),
        kind: PlaceKind::Zone,
        within: within.map(str::to_string),
        first_visit: Tick(first_visit),
        spot: None,
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
        trust: None,
        slapped: None,
        spot: None,
        trust_why: None,
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
        "hero": { "sheet": [], "entries": [] },
        "hero_refused": null,
        "chapters": [{
            "number": 1,
            "began": 5,
            "ended": 5,
            "zones": [],
            "people": [],
            "deeds": [{ "kind": "level", "from": null, "to": 12, "at": 5, "place": null }],
            "left_out": 0,
            "prose": null,
            "footnotes": [],
        }],
        "places": [],
        "people": [],
        "deeds": [{ "kind": "level", "from": null, "to": 12, "at": 5, "place": null }],
        "learned": [],
        "quests": [],
        "stories": [],
        "talk_quest": null,
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
            notice: None,
        };
        let line = serde_json::to_string(&output).unwrap();
        assert!(line.len() <= MAX_LINE, "{} bytes", line.len());
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
        joined.learned.extend(page.journal.learned);
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
        ended: Tick(HOUR + 120),
        zones: vec!["Westfall".to_string()],
        people: vec!["Gryan Stoutmantle".to_string()],
        deeds: vec![kill],
        left_out: 0,
        prose: None,
        footnotes: Vec::new(),
    };
    assert_eq!(chapters, [expected]);
}

/// The zones of each chapter, in order.
fn chapter_zones(character: &Character) -> Vec<Vec<String>> {
    journal(character)
        .chapters
        .into_iter()
        .map(|chapter| chapter.zones)
        .collect()
}

/// A character at level 8 who plays in Westfall from the first hour, with an event each
/// 10 minutes for `minutes`.
fn played_in_westfall(minutes: u64) -> Character {
    let mut character = Character::new();
    character.reach_level(Tick(HOUR), 8).unwrap();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    for n in 1..=minutes / 10 {
        character
            .enter_zone(Tick(HOUR + n * 600), "Westfall", Some(&format!("Farm {n}")))
            .unwrap();
    }
    character
}

#[test]
fn a_new_zone_after_enough_play_starts_a_chapter() {
    let mut character = played_in_westfall(50);

    character
        .enter_zone(Tick(HOUR + 50 * 60), "Duskwood", None)
        .unwrap();

    assert_eq!(chapter_zones(&character), [["Westfall"], ["Duskwood"]]);
}

#[test]
fn a_new_zone_soon_after_the_chapter_began_joins_it() {
    let mut character = played_in_westfall(20);

    character
        .enter_zone(Tick(HOUR + 20 * 60), "Duskwood", None)
        .unwrap();

    assert_eq!(chapter_zones(&character), [["Westfall", "Duskwood"]]);
}

#[test]
fn a_pause_alone_starts_no_chapter_and_time_away_is_no_play() {
    let mut character = played_in_westfall(20);

    character
        .enter_zone(Tick(9 * HOUR), "Duskwood", None)
        .unwrap();

    assert_eq!(chapter_zones(&character), [["Westfall", "Duskwood"]]);
}

#[test]
fn every_tenth_level_starts_a_chapter_and_other_levels_do_not() {
    let mut character = played_in_westfall(50);

    character.reach_level(Tick(HOUR + 50 * 60), 9).unwrap();
    character.reach_level(Tick(HOUR + 100 * 60), 10).unwrap();

    let chapters = journal(&character).chapters;
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[1].began, Tick(HOUR + 100 * 60));
}

#[test]
fn the_first_kill_of_a_rare_starts_a_chapter_and_its_echo_does_not() {
    let mut character = played_in_westfall(50);

    character
        .defeat_npc(Tick(HOUR + 50 * 60), "Mother Fang")
        .unwrap();
    character
        .defeat_npc(Tick(HOUR + 200 * 60), "Mother Fang")
        .unwrap();

    let starts: Vec<Tick> = journal(&character)
        .chapters
        .iter()
        .map(|chapter| chapter.began)
        .collect();
    assert_eq!(starts, [Tick(HOUR), Tick(HOUR + 50 * 60)]);
}

#[test]
fn a_chapter_keeps_twenty_entries_of_each_list_and_counts_the_rest() {
    let mut character = Character::new();
    for n in 0..25 {
        character
            .enter_zone(Tick(HOUR + n), &format!("Zone {n}"), None)
            .unwrap();
    }

    let chapters = journal(&character).chapters;

    assert_eq!(chapters[0].zones.len(), 20);
    assert_eq!(chapters[0].left_out, 5);
}

#[test]
fn a_slapped_npc_shows_the_slaps_and_its_lost_trust() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.meet_npc(Tick(11), "Innkeeper Farley").unwrap();
    character.slap(Tick(12), "Innkeeper Farley").unwrap();
    character.slap(Tick(13), "Innkeeper Farley").unwrap();

    let people = journal(&character).people;

    assert_eq!((people[0].slapped, people[0].trust), (Some(2), Some(-20)));
}

#[test]
fn a_slapped_npc_that_you_never_talked_to_is_on_the_people_page() {
    let mut character = Character::new();

    character.slap(Tick(12), "Stormwind City Guard").unwrap();

    let people = journal(&character).people;
    assert_eq!(people.len(), 1);
    assert_eq!((people[0].slapped, people[0].trust), (Some(1), Some(-10)));
}

#[test]
fn the_largest_chapter_still_fits_on_one_page() {
    let mut character = Character::new();
    let name = |kind: &str, n: u64| format!("{kind}{n:02}{}", "\"".repeat(92));
    for n in 0..25 {
        character
            .enter_zone(Tick(HOUR + n), &name("Z", n), None)
            .unwrap();
        character.meet_npc(Tick(HOUR + n), &name("P", n)).unwrap();
        character.defeat_npc(Tick(HOUR + n), &name("F", n)).unwrap();
    }
    let mut journal = journal(&character);
    journal.chapters[0].prose = Some("\"".repeat(600));
    journal.chapters[0].footnotes = vec!["\"".repeat(200); 3];
    journal.hero.sheet = timeways_story::hero::FIELDS
        .iter()
        .map(|field| timeways_story::hero::Field {
            field: (*field).to_string(),
            text: "\u{10348}".repeat(300),
        })
        .collect();
    journal.hero_refused = Some("r".repeat(200));

    let sizes: Vec<usize> = pages(journal)
        .into_iter()
        .map(|page| {
            serde_json::to_string(&Output::Journal {
                id: MessageId(u64::MAX),
                page,
                notice: None,
            })
            .unwrap()
            .len()
        })
        .collect();

    assert!(sizes.iter().all(|size| *size <= MAX_LINE), "{sizes:?}");
}

#[test]
fn no_page_holds_more_than_two_hundred_items_in_one_list() {
    let mut character = Character::new();
    for n in 0..450 {
        character
            .enter_zone(Tick(HOUR + n), &format!("Z{n}"), None)
            .unwrap();
    }

    let pages = pages(journal(&character));

    assert!(pages.iter().all(|page| page.journal.places.len() <= 200));
    let places: usize = pages.iter().map(|page| page.journal.places.len()).sum();
    assert_eq!(places, 450);
}

#[test]
fn a_long_list_of_what_you_learned_fits_on_pages_and_keeps_its_order() {
    let longest_name = "n".repeat(MAX_NAME_BYTES);
    let read: Vec<Read> = (0..500)
        .map(|n| Read {
            at: Tick(n),
            text: SeenText {
                kind: TextKind::Book,
                title: Some(longest_name.clone()),
                npc: Some(longest_name.clone()),
                zone: Some(longest_name.clone()),
                text: "\u{7a0}".repeat(MAX_SEEN_BYTES / 2),
            },
        })
        .collect();
    let whole = Journal {
        learned: learned(&read, &[]),
        ..Journal::default()
    };

    let pages = pages(whole.clone());

    let mut joined = Vec::new();
    for page in pages {
        assert!(page.journal.learned.len() <= 200);
        let output = Output::Journal {
            id: MessageId(u64::MAX),
            page,
            notice: None,
        };
        let line = serde_json::to_string(&output).unwrap();
        assert!(line.len() <= MAX_LINE, "{} bytes", line.len());
        let Output::Journal { page, .. } = output else {
            unreachable!()
        };
        joined.extend(page.journal.learned);
    }
    assert_eq!(joined, whole.learned);
}

/// Trust is a fact that an NPC holds about you too, but no kill.
#[test]
fn a_death_with_no_killer_names_no_one_after_an_npc_starts_to_trust_you() {
    let mut character = Character::new();
    character
        .adjust_trust(Tick(1), "Innkeeper Farley", 2)
        .unwrap();

    character.die(Tick(2), None).unwrap();

    let deeds = journal(&character).deeds;
    assert!(
        matches!(deeds.last(), Some(Deed::Died { killer: None, .. })),
        "{deeds:?}"
    );
}

#[test]
fn a_turned_in_quest_of_the_game_is_a_deed_and_a_class_quest_is_its_own_kind() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(HOUR), "Tirisfal Glades", Some("Deathknell"))
        .unwrap();
    character
        .take_game_quest(
            Tick(HOUR),
            "Rattling the Rattlecages",
            GameQuestKind::Normal,
        )
        .unwrap();
    character
        .finish_game_quest(
            Tick(HOUR + 60),
            "Rattling the Rattlecages",
            GameQuestKind::Normal,
        )
        .unwrap();
    character
        .finish_game_quest(
            Tick(HOUR + 120),
            "Rediscovering the Light",
            GameQuestKind::Class,
        )
        .unwrap();

    let deeds = journal(&character).deeds;

    let place = Some("Deathknell".to_string());
    assert_eq!(
        deeds,
        [
            Deed::GameQuestDone {
                title: "Rattling the Rattlecages".to_string(),
                at: Tick(HOUR + 60),
                place: place.clone(),
            },
            Deed::ClassQuestDone {
                title: "Rediscovering the Light".to_string(),
                at: Tick(HOUR + 120),
                place,
            },
        ]
    );
}

#[test]
fn a_finished_class_quest_starts_a_chapter_and_a_plain_quest_does_not() {
    let mut character = played_in_westfall(50);

    character
        .finish_game_quest(
            Tick(HOUR + 50 * 60),
            "The Defias Brotherhood",
            GameQuestKind::Normal,
        )
        .unwrap();
    character
        .finish_game_quest(
            Tick(HOUR + 51 * 60),
            "The Tome of Valor",
            GameQuestKind::Class,
        )
        .unwrap();

    let starts: Vec<Tick> = journal(&character)
        .chapters
        .iter()
        .map(|chapter| chapter.began)
        .collect();
    assert_eq!(starts, [Tick(HOUR), Tick(HOUR + 51 * 60)]);
}

#[test]
fn a_quest_taken_again_after_a_turn_in_adds_no_second_deed() {
    let mut character = Character::new();

    for at in [HOUR, HOUR + 60] {
        character
            .finish_game_quest(Tick(at), "Rediscovering the Light", GameQuestKind::Class)
            .unwrap();
    }

    assert_eq!(journal(&character).deeds.len(), 1);
}

/// The kind of each place, by name.
fn kinds(character: &Character) -> Vec<(String, PlaceKind)> {
    journal(character)
        .places
        .into_iter()
        .map(|place| (place.name, place.kind))
        .collect()
}

#[test]
fn an_instance_and_a_capital_have_their_kind_and_a_zone_is_plain() {
    let mut character = Character::new();
    character.enter_zone(Tick(HOUR), "Westfall", None).unwrap();
    character
        .enter_zone(Tick(HOUR), "The Deadmines", None)
        .unwrap();
    character
        .mark_instance(Tick(HOUR), "The Deadmines", InstanceKind::Dungeon)
        .unwrap();
    character
        .enter_zone(Tick(HOUR), "Molten Core", None)
        .unwrap();
    character
        .mark_instance(Tick(HOUR), "Molten Core", InstanceKind::Raid)
        .unwrap();
    character
        .enter_zone(Tick(HOUR), "Undercity", Some("Trade Quarter"))
        .unwrap();

    let expected = [
        ("Westfall", PlaceKind::Zone),
        ("The Deadmines", PlaceKind::Dungeon),
        ("Molten Core", PlaceKind::Raid),
        ("Undercity", PlaceKind::Capital),
        ("Trade Quarter", PlaceKind::Zone),
    ]
    .map(|(name, kind)| (name.to_string(), kind));
    assert_eq!(kinds(&character), expected);
}

#[test]
fn a_subzone_with_the_name_of_a_capital_is_no_capital() {
    let mut character = Character::new();

    character
        .enter_zone(Tick(HOUR), "Tirisfal Glades", Some("Undercity"))
        .unwrap();

    assert_eq!(
        kinds(&character)[1],
        ("Undercity".to_string(), PlaceKind::Zone)
    );
}

#[test]
fn a_second_mark_of_an_instance_adds_no_event() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(HOUR), "The Deadmines", None)
        .unwrap();
    character
        .mark_instance(Tick(HOUR), "The Deadmines", InstanceKind::Dungeon)
        .unwrap();
    let before = character.world().history().len();

    character
        .mark_instance(Tick(HOUR + 60), "The Deadmines", InstanceKind::Dungeon)
        .unwrap();

    assert_eq!(character.world().history().len(), before);
}

#[test]
fn a_quest_mark_is_a_deed_with_its_quest_once_and_no_milestone() {
    let mut character = played_in_westfall(50);

    for at in [HOUR + 50 * 60, HOUR + 51 * 60] {
        character
            .take_quest_mark(Tick(at), "Rediscovering the Light", "Touched by the Light")
            .unwrap();
    }

    let journal = journal(&character);
    let marks: Vec<&Deed> = journal
        .deeds
        .iter()
        .filter(|deed| matches!(deed, Deed::QuestMarked { .. }))
        .collect();
    let expected = Deed::QuestMarked {
        mark: "Touched by the Light".to_string(),
        quest: "Rediscovering the Light".to_string(),
        at: Tick(HOUR + 50 * 60),
        place: Some("Farm 5".to_string()),
    };
    assert_eq!(marks, [&expected]);
    assert_eq!(journal.chapters.len(), 1);
}

#[test]
fn a_sheet_full_of_quotes_spreads_over_pages_in_its_order() {
    let mut whole = Journal::default();
    whole.hero.sheet = timeways_story::hero::FIELDS
        .iter()
        .map(|field| timeways_story::hero::Field {
            field: (*field).to_string(),
            text: "\"".repeat(1000),
        })
        .collect();

    let pages = pages(whole.clone());

    assert!(pages.len() > 1, "{}", pages.len());
    let joined: Vec<_> = pages
        .iter()
        .flat_map(|page| page.journal.hero.sheet.clone())
        .collect();
    assert_eq!(joined, whole.hero.sheet);
}
