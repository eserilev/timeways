//! Where the player stood on the map at a first visit or a meeting (GAMEPLAY.md 3.6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use serde_json::{Value, json};
use std::path::Path;
use timeways_story::character::Character;
use timeways_story::input::{Input, MessageId};
use timeways_story::journal::journal;
use timeways_story::pack::Pack;
use timeways_story::spot::{Spot, spot_of};
use timeways_story::store::Store;
use timeways_story::story::{Output, Story};

fn spot(map: i64, x: i64, y: i64) -> Spot {
    serde_json::from_value(json!({"map": map, "x": x, "y": y})).unwrap()
}

fn story(name: &str) -> Story {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("spots-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[]).unwrap();
    let mut story = Story::new(Pack::open(&path).unwrap(), Store::Memory);
    story
        .handle(Input::CharacterEntered {
            realm: "Testrealm".to_string(),
            name: "Tester".to_string(),
        })
        .unwrap();
    story
}

/// The first page of the journal, as the JSON line that the bridge gets.
fn first_page(story: &mut Story) -> Value {
    let outputs = story
        .handle(Input::JournalAsked {
            id: MessageId(1),
            page: 0,
        })
        .unwrap();
    let Some(Output::Journal { .. }) = outputs.first() else {
        panic!("expected a journal, got {outputs:?}");
    };
    serde_json::to_value(&outputs[0]).unwrap()
}

fn place_spot(character: &Character, name: &str) -> Option<Spot> {
    let places = journal(character).places;
    places.into_iter().find(|place| place.name == name)?.spot
}

fn person_spot(character: &Character, name: &str) -> Option<Spot> {
    let people = journal(character).people;
    people.into_iter().find(|person| person.name == name)?.spot
}

#[test]
fn a_spot_reads_from_whole_thousandths() {
    let line =
        r#"{"type":"npc_met","at":100,"name":"Keeper Tessa","spot":{"map":1420,"x":0,"y":1000}}"#;

    let input: Input = serde_json::from_str(line).unwrap();

    let expected = Input::NpcMet {
        at: Tick(100),
        name: "Keeper Tessa".to_string(),
        spot: Some(spot(1420, 0, 1000)),
    };
    assert_eq!(input, expected);
}

#[test]
fn a_spot_off_the_map_counts_as_no_spot_and_the_visit_still_counts() {
    let lines = [
        r#"{"type":"zone_entered","at":100,"zone":"Testvale","spot":{"map":1420,"x":1001,"y":5}}"#,
        r#"{"type":"zone_entered","at":100,"zone":"Testvale","spot":{"map":0,"x":1,"y":5}}"#,
        r#"{"type":"zone_entered","at":100,"zone":"Testvale","spot":{"map":1420,"x":0.5,"y":5}}"#,
        r#"{"type":"zone_entered","at":100,"zone":"Testvale","spot":"here"}"#,
        r#"{"type":"zone_entered","at":100,"zone":"Testvale","spot":null}"#,
    ];

    let inputs: Vec<Input> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    let expected = Input::ZoneEntered {
        at: Tick(100),
        zone: "Testvale".to_string(),
        subzone: None,
        spot: None,
        hour: None,
    };
    for input in inputs {
        assert_eq!(input, expected);
    }
}

#[test]
fn a_first_visit_with_a_spot_marks_the_subzone_and_its_zone() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Testvale", Some("Old Mill"))
        .unwrap();

    character.mark_here(Tick(10), spot(1420, 250, 750)).unwrap();

    assert_eq!(
        place_spot(&character, "Old Mill"),
        Some(spot(1420, 250, 750))
    );
    assert_eq!(
        place_spot(&character, "Testvale"),
        Some(spot(1420, 250, 750))
    );
}

#[test]
fn a_zone_keeps_its_spot_when_you_visit_another_subzone() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Testvale", Some("Old Mill"))
        .unwrap();
    character.mark_here(Tick(10), spot(1420, 250, 750)).unwrap();

    character
        .enter_zone(Tick(20), "Testvale", Some("Old Tower"))
        .unwrap();
    character.mark_here(Tick(20), spot(1420, 600, 100)).unwrap();

    assert_eq!(
        place_spot(&character, "Old Tower"),
        Some(spot(1420, 600, 100))
    );
    assert_eq!(
        place_spot(&character, "Testvale"),
        Some(spot(1420, 250, 750))
    );
}

#[test]
fn a_place_keeps_the_spot_of_its_first_visit() {
    let mut character = Character::new();
    character.enter_zone(Tick(10), "Testvale", None).unwrap();
    character.mark_here(Tick(10), spot(1420, 100, 100)).unwrap();

    character.enter_zone(Tick(20), "Testvale", None).unwrap();
    character.mark_here(Tick(20), spot(1420, 900, 900)).unwrap();

    assert_eq!(
        place_spot(&character, "Testvale"),
        Some(spot(1420, 100, 100))
    );
}

#[test]
fn an_npc_keeps_the_spot_of_the_first_meeting_that_had_one() {
    let mut character = Character::new();
    character.meet_npc(Tick(10), "Keeper Tessa").unwrap();
    let before = person_spot(&character, "Keeper Tessa");

    character
        .mark_npc(Tick(20), "Keeper Tessa", spot(1420, 400, 600))
        .unwrap();
    character
        .mark_npc(Tick(30), "Keeper Tessa", spot(1421, 1, 1))
        .unwrap();

    assert_eq!(before, None);
    assert_eq!(
        person_spot(&character, "Keeper Tessa"),
        Some(spot(1420, 400, 600))
    );
}

#[test]
fn a_spot_for_an_npc_that_you_never_met_adds_nobody() {
    let mut character = Character::new();

    character
        .mark_npc(Tick(10), "Keeper Tessa", spot(1420, 400, 600))
        .unwrap();

    assert!(journal(&character).people.is_empty());
}

#[test]
fn a_spot_stays_after_a_replay_of_the_history() {
    let mut character = Character::new();
    character.enter_zone(Tick(10), "Testvale", None).unwrap();
    character.mark_here(Tick(10), spot(1420, 1, 999)).unwrap();

    let events: Vec<_> = character.world().history().iter().cloned().collect();
    let replayed = Character::from_history(&events).unwrap();

    let place = replayed.world().location_of(replayed.you()).unwrap();
    assert_eq!(spot_of(replayed.world(), place), Some(spot(1420, 1, 999)));
}

#[test]
fn the_journal_carries_the_spots_of_places_and_people() {
    let mut story = story("journal");
    let zone = r#"{"type":"zone_entered","at":1790000000,"zone":"Testvale","subzone":"Old Mill","spot":{"map":1420,"x":500,"y":250}}"#;
    let npc = r#"{"type":"npc_met","at":1790000001,"name":"Keeper Tessa","spot":{"map":1420,"x":510,"y":260}}"#;

    story.handle(serde_json::from_str(zone).unwrap()).unwrap();
    story.handle(serde_json::from_str(npc).unwrap()).unwrap();
    let page = first_page(&mut story);

    assert_eq!(
        page["places"][1]["spot"],
        json!({"map": 1420, "x": 500, "y": 250})
    );
    assert_eq!(
        page["people"][0]["spot"],
        json!({"map": 1420, "x": 510, "y": 260})
    );
}

#[test]
fn a_place_with_no_spot_sends_no_spot() {
    let mut story = story("no-spot");
    let zone = r#"{"type":"zone_entered","at":1790000000,"zone":"Testvale"}"#;

    story.handle(serde_json::from_str(zone).unwrap()).unwrap();
    let page = first_page(&mut story);

    assert!(page["places"][0].get("spot").is_none(), "{page}");
}
