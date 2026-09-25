#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::{EntityId, EntityType, Tick};
use timeways_story::character::Character;
use timeways_story::record::{GameEvent, Record};
use timeways_story::vocabulary::{LEVEL, MET, VISITED};

fn record(at: u64, event: GameEvent) -> Record {
    Record {
        at: Tick(at),
        event,
    }
}

fn zone(at: u64, zone: &str, subzone: Option<&str>) -> Record {
    let event = GameEvent::ZoneEntered {
        zone: zone.to_string(),
        subzone: subzone.map(str::to_string),
    };
    record(at, event)
}

fn npc(at: u64, name: &str) -> Record {
    record(
        at,
        GameEvent::NpcMet {
            name: name.to_string(),
        },
    )
}

fn level(at: u64, level: u8) -> Record {
    record(at, GameEvent::LevelReached { level })
}

fn id_of(character: &Character, entity_type: EntityType, name: &str) -> Option<EntityId> {
    character
        .world()
        .entities()
        .find(|entity| entity.entity_type == entity_type && entity.name == name)
        .map(|entity| entity.id)
}

fn place(character: &Character, name: &str) -> EntityId {
    id_of(character, EntityType::Place, name).expect("the place exists")
}

fn person(character: &Character, name: &str) -> EntityId {
    id_of(character, EntityType::Person, name).expect("the person exists")
}

fn holds(character: &Character, name: &str, target: EntityId) -> bool {
    let you = character
        .world()
        .entity(character.you())
        .expect("you exist");
    you.fact(name, Some(target)).is_some()
}

#[test]
fn entering_a_zone_marks_it_visited() {
    let mut character = Character::new();

    character.apply(&zone(1, "Elwynn Forest", None)).unwrap();

    let elwynn = place(&character, "Elwynn Forest");
    assert!(holds(&character, VISITED, elwynn));
    assert_eq!(character.world().location_of(character.you()), Some(elwynn));
}

#[test]
fn a_subzone_sits_inside_its_zone() {
    let mut character = Character::new();

    character
        .apply(&zone(1, "Elwynn Forest", Some("Goldshire")))
        .unwrap();

    let elwynn = place(&character, "Elwynn Forest");
    let goldshire = place(&character, "Goldshire");
    assert_eq!(character.world().location_of(goldshire), Some(elwynn));
}

#[test]
fn you_stand_in_the_subzone_and_visited_both_places() {
    let mut character = Character::new();

    character
        .apply(&zone(1, "Elwynn Forest", Some("Goldshire")))
        .unwrap();

    let elwynn = place(&character, "Elwynn Forest");
    let goldshire = place(&character, "Goldshire");
    assert!(holds(&character, VISITED, elwynn));
    assert!(holds(&character, VISITED, goldshire));
    assert_eq!(
        character.world().location_of(character.you()),
        Some(goldshire)
    );
}

#[test]
fn a_subzone_with_the_name_of_its_zone_is_the_zone() {
    let mut character = Character::new();

    let result = character.apply(&zone(1, "Stormwind City", Some("Stormwind City")));

    assert!(result.is_ok());
    let places = character
        .world()
        .entities()
        .filter(|e| e.entity_type == EntityType::Place);
    assert_eq!(places.count(), 1);
}

#[test]
fn entering_the_same_place_again_adds_no_events() {
    let mut character = Character::new();
    character
        .apply(&zone(1, "Elwynn Forest", Some("Goldshire")))
        .unwrap();
    let before = character.world().history().len();

    character
        .apply(&zone(2, "Elwynn Forest", Some("Goldshire")))
        .unwrap();

    assert_eq!(character.world().history().len(), before);
}

#[test]
fn a_zone_with_no_name_is_refused() {
    let mut character = Character::new();

    let result = character.apply(&zone(1, "", None));

    assert!(result.is_err());
}

#[test]
fn meeting_an_npc_marks_it_met() {
    let mut character = Character::new();

    character.apply(&npc(1, "Innkeeper Farley")).unwrap();

    let farley = person(&character, "Innkeeper Farley");
    assert!(holds(&character, MET, farley));
}

#[test]
fn an_npc_lives_where_you_met_it() {
    let mut character = Character::new();
    character
        .apply(&zone(1, "Elwynn Forest", Some("Goldshire")))
        .unwrap();

    character.apply(&npc(2, "Innkeeper Farley")).unwrap();

    let farley = person(&character, "Innkeeper Farley");
    let goldshire = place(&character, "Goldshire");
    assert_eq!(character.world().location_of(farley), Some(goldshire));
}

#[test]
fn meeting_an_npc_twice_keeps_one_entity() {
    let mut character = Character::new();
    character.apply(&npc(1, "Innkeeper Farley")).unwrap();
    let before = character.world().len();

    character.apply(&npc(2, "Innkeeper Farley")).unwrap();

    assert_eq!(character.world().len(), before);
}

#[test]
fn a_level_up_raises_the_level() {
    let mut character = Character::new();
    character.apply(&level(1, 5)).unwrap();

    character.apply(&level(2, 6)).unwrap();

    let you = character.world().entity(character.you()).unwrap();
    assert_eq!(you.value(LEVEL), Some(6));
}

#[test]
fn a_lower_level_is_refused() {
    let mut character = Character::new();
    character.apply(&level(1, 6)).unwrap();

    let result = character.apply(&level(2, 5));

    assert!(result.is_err());
}

#[test]
fn a_level_past_sixty_is_refused() {
    let mut character = Character::new();

    let result = character.apply(&level(1, 61));

    assert!(result.is_err());
}

#[test]
fn an_event_older_than_the_world_is_refused() {
    let mut character = Character::new();
    character.apply(&zone(10, "Elwynn Forest", None)).unwrap();

    let result = character.apply(&npc(9, "Innkeeper Farley"));

    assert!(result.is_err());
}
