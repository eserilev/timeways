#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::{EntityId, EntityType, Tick};
use timeways_story::character::Character;
use timeways_story::vocabulary::{LEVEL, MET, VISITED};

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

    character
        .enter_zone(Tick(1), "Elwynn Forest", None)
        .unwrap();

    let elwynn = place(&character, "Elwynn Forest");
    assert!(holds(&character, VISITED, elwynn));
    assert_eq!(character.world().location_of(character.you()), Some(elwynn));
}

#[test]
fn a_subzone_sits_inside_its_zone() {
    let mut character = Character::new();

    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();

    let elwynn = place(&character, "Elwynn Forest");
    let goldshire = place(&character, "Goldshire");
    assert_eq!(character.world().location_of(goldshire), Some(elwynn));
}

#[test]
fn you_stand_in_the_subzone_and_visited_both_places() {
    let mut character = Character::new();

    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
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

    let result = character.enter_zone(Tick(1), "Stormwind City", Some("Stormwind City"));

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
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    let before = character.world().history().len();

    character
        .enter_zone(Tick(2), "Elwynn Forest", Some("Goldshire"))
        .unwrap();

    assert_eq!(character.world().history().len(), before);
}

#[test]
fn a_zone_with_no_name_is_refused() {
    let mut character = Character::new();

    let result = character.enter_zone(Tick(1), "", None);

    assert!(result.is_err());
}

#[test]
fn meeting_an_npc_marks_it_met() {
    let mut character = Character::new();

    character.meet_npc(Tick(1), "Innkeeper Farley").unwrap();

    let farley = person(&character, "Innkeeper Farley");
    assert!(holds(&character, MET, farley));
}

#[test]
fn an_npc_lives_where_you_met_it() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();

    character.meet_npc(Tick(2), "Innkeeper Farley").unwrap();

    let farley = person(&character, "Innkeeper Farley");
    let goldshire = place(&character, "Goldshire");
    assert_eq!(character.world().location_of(farley), Some(goldshire));
}

#[test]
fn meeting_an_npc_twice_keeps_one_entity() {
    let mut character = Character::new();
    character.meet_npc(Tick(1), "Innkeeper Farley").unwrap();
    let before = character.world().len();

    character.meet_npc(Tick(2), "Innkeeper Farley").unwrap();

    assert_eq!(character.world().len(), before);
}

#[test]
fn a_level_up_raises_the_level() {
    let mut character = Character::new();
    character.reach_level(Tick(1), 5).unwrap();

    character.reach_level(Tick(2), 6).unwrap();

    let you = character.world().entity(character.you()).unwrap();
    assert_eq!(you.value(LEVEL), Some(6));
}

#[test]
fn a_lower_level_is_refused() {
    let mut character = Character::new();
    character.reach_level(Tick(1), 6).unwrap();

    let result = character.reach_level(Tick(2), 5);

    assert!(result.is_err());
}

#[test]
fn a_level_past_sixty_is_refused() {
    let mut character = Character::new();

    let result = character.reach_level(Tick(1), 61);

    assert!(result.is_err());
}

#[test]
fn an_event_older_than_the_world_is_refused() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(10), "Elwynn Forest", None)
        .unwrap();

    let result = character.meet_npc(Tick(9), "Innkeeper Farley");

    assert!(result.is_err());
}

#[test]
fn a_place_you_never_entered_is_not_visited() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Elwynn Forest", None)
        .unwrap();

    assert!(character.has_visited("Elwynn Forest"));
    assert!(!character.has_visited("Westfall"));
}

#[test]
fn an_npc_you_never_met_is_not_met() {
    let mut character = Character::new();
    character.meet_npc(Tick(1), "Innkeeper Farley").unwrap();

    assert!(character.has_met("Innkeeper Farley"));
    assert!(!character.has_met("Hogger"));
}

#[test]
fn place_names_go_from_where_you_stand_outward() {
    let mut character = Character::new();

    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();

    assert_eq!(character.place_names(), ["Goldshire", "Elwynn Forest"]);
}

#[test]
fn place_names_are_empty_before_the_first_zone() {
    let character = Character::new();

    assert!(character.place_names().is_empty());
}

fn defeated_count(character: &Character, foe: &str) -> Option<i64> {
    let foe = person(character, foe);
    let you = character.world().entity(character.you()).unwrap();
    you.fact(timeways_story::vocabulary::DEFEATED, Some(foe))
        .and_then(|fact| fact.value)
}

#[test]
fn a_first_kill_counts_one() {
    let mut character = Character::new();

    character.defeat_npc(Tick(1), "Hogger").unwrap();

    assert_eq!(defeated_count(&character, "Hogger"), Some(1));
}

#[test]
fn each_later_kill_adds_one_and_the_foe_stays_alive() {
    let mut character = Character::new();
    character.defeat_npc(Tick(1), "Hogger").unwrap();

    character.defeat_npc(Tick(2), "Hogger").unwrap();
    character.defeat_npc(Tick(3), "Hogger").unwrap();

    let hogger = person(&character, "Hogger");
    assert_eq!(defeated_count(&character, "Hogger"), Some(3));
    assert!(!character.world().entity(hogger).unwrap().gone());
}

#[test]
fn a_foe_lives_where_you_fought_it() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Forest's Edge"))
        .unwrap();

    character.defeat_npc(Tick(2), "Hogger").unwrap();

    let hogger = person(&character, "Hogger");
    let edge = place(&character, "Forest's Edge");
    assert_eq!(character.world().location_of(hogger), Some(edge));
}

fn deaths(character: &Character) -> Option<i64> {
    character
        .world()
        .entity(character.you())
        .unwrap()
        .value(timeways_story::vocabulary::DEATHS)
}

#[test]
fn a_death_to_an_npc_is_a_deed_of_the_npc() {
    let mut character = Character::new();

    character.die(Tick(1), Some("Murloc Forager")).unwrap();
    character.die(Tick(2), Some("Murloc Forager")).unwrap();

    let murloc = person(&character, "Murloc Forager");
    let kills = character
        .world()
        .entity(murloc)
        .unwrap()
        .fact(timeways_story::vocabulary::DEFEATED, Some(character.you()))
        .and_then(|fact| fact.value);
    assert_eq!(kills, Some(2));
    assert_eq!(deaths(&character), Some(2));
}

#[test]
fn a_death_with_no_known_killer_still_counts() {
    let mut character = Character::new();

    character.die(Tick(1), None).unwrap();

    assert_eq!(deaths(&character), Some(1));
    assert_eq!(character.world().len(), 1);
}

#[test]
fn a_subzone_name_in_two_zones_is_two_places() {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Westfall", Some("The Great Sea"))
        .unwrap();

    character
        .enter_zone(Tick(2), "Stranglethorn Vale", Some("The Great Sea"))
        .unwrap();

    let seas: Vec<_> = character
        .world()
        .entities()
        .filter(|entity| entity.name == "The Great Sea")
        .map(|entity| {
            character
                .world()
                .entity(entity.location().unwrap())
                .unwrap()
                .name
                .clone()
        })
        .collect();
    assert_eq!(seas, ["Westfall", "Stranglethorn Vale"]);
    assert!(character.has_visited("The Great Sea"));
}
