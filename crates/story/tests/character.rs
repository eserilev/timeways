#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::{EntityId, EntityType, Event, EventId, EventKind, Tick};
use timeways_story::character::Character;
use timeways_story::input::{GameQuestKind, Reaction};
use timeways_story::race_class::{Class, Race};
use timeways_story::vocabulary::{
    DEAD, GAME_QUEST_TAKEN, LEVEL, MET, QUEST_ACCEPTED, QUEST_OFFERED, VISITED,
};

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

fn trust_of(character: &Character, npc: &str) -> Option<i64> {
    let npc = person(character, npc);
    character
        .world()
        .entity(npc)
        .unwrap()
        .fact(timeways_story::vocabulary::TRUSTS, Some(character.you()))
        .and_then(|fact| fact.value)
}

#[test]
fn a_slap_costs_ten_trust_and_counts() {
    let mut character = Character::new();

    character.slap(Tick(1), "Innkeeper Farley").unwrap();
    character.slap(Tick(2), "Innkeeper Farley").unwrap();

    let farley = person(&character, "Innkeeper Farley");
    let you = character.world().entity(character.you()).unwrap();
    let slaps = you
        .fact(timeways_story::vocabulary::SLAPPED, Some(farley))
        .and_then(|fact| fact.value);
    assert_eq!(slaps, Some(2));
    assert_eq!(trust_of(&character, "Innkeeper Farley"), Some(-20));
}

#[test]
fn trust_stops_at_minus_one_hundred() {
    let mut character = Character::new();

    for at in 1..=12 {
        character.slap(Tick(at), "Innkeeper Farley").unwrap();
    }

    assert_eq!(trust_of(&character, "Innkeeper Farley"), Some(-100));
}

#[test]
fn trust_from_talk_stops_at_one_hundred() {
    let mut character = Character::new();

    for at in 1..=25 {
        character
            .adjust_trust(Tick(at), "Innkeeper Farley", 5)
            .unwrap();
    }

    assert_eq!(trust_of(&character, "Innkeeper Farley"), Some(100));
}

#[test]
fn a_death_past_the_cap_of_its_count_still_lands() {
    let mut character = Character::new();
    for at in 1..=1000 {
        character.die(Tick(at), None).unwrap();
    }

    let past_the_cap = character.die(Tick(1001), Some("Hogger"));

    assert_eq!(past_the_cap, Ok(()));
}

#[test]
fn a_slap_past_the_cap_of_its_count_still_costs_trust() {
    let mut character = Character::new();
    for at in 1..=1000 {
        character.slap(Tick(at), "Hogger").unwrap();
    }

    let past_the_cap = character.slap(Tick(1001), "Hogger");

    assert_eq!(past_the_cap, Ok(()));
    assert_eq!(character.slaps_of("Hogger"), Some(1000));
}

fn created(id: u32, entity_type: EntityType, name: &str) -> EventKind {
    EventKind::EntityCreated {
        id: EntityId(id),
        entity_type,
        name: name.to_string(),
    }
}

fn history_of(kinds: Vec<EventKind>) -> Vec<Event> {
    kinds
        .into_iter()
        .zip(0..)
        .map(|(kind, at)| Event {
            id: EventId(at),
            tick: Tick(at),
            kind,
        })
        .collect()
}

#[test]
fn you_are_the_entity_that_the_saved_history_founds() {
    let events = history_of(vec![created(3, EntityType::Person, "you")]);

    let character = Character::from_history(&events).unwrap();

    assert_eq!(character.you(), EntityId(3));
}

#[test]
fn an_earned_title_is_held() {
    let mut character = Character::new();

    character.earn_title(Tick(1), "Hogger Slayer").unwrap();

    assert!(character.has_title("Hogger Slayer"));
    assert!(!character.has_title("Kingslayer"));
}

#[test]
fn trust_of_gives_the_trust_that_the_npc_holds() {
    let mut character = Character::new();

    character
        .adjust_trust(Tick(1), "Innkeeper Farley", 7)
        .unwrap();

    assert_eq!(character.trust_of("Innkeeper Farley"), Some(7));
    assert_eq!(character.trust_of("Hogger"), None);
}

#[test]
fn visited_zones_leave_out_the_subzones() {
    let mut character = Character::new();

    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();

    assert_eq!(character.visited_zones(), ["Elwynn Forest"]);
}

#[test]
fn an_npc_with_the_dead_fact_is_dead() {
    let events = history_of(vec![
        created(0, EntityType::Person, "you"),
        created(1, EntityType::Person, "Hogger"),
        EventKind::FactStart {
            entity: EntityId(1),
            name: DEAD.to_string(),
            value: None,
            linked_to: None,
        },
    ]);

    let character = Character::from_history(&events).unwrap();

    assert!(character.is_dead("Hogger"));
}

#[test]
fn an_offered_quest_is_a_deed_of_its_giver() {
    let mut character = Character::new();

    character
        .offer_quest(Tick(1), "Marshal Dughan", "Wolves Across the Border")
        .unwrap();

    let dughan = person(&character, "Marshal Dughan");
    let quest = id_of(&character, EntityType::Thing, "Wolves Across the Border").unwrap();
    let giver = character.world().entity(dughan).unwrap();
    assert!(giver.fact(QUEST_OFFERED, Some(quest)).is_some());
}

#[test]
fn an_accepted_quest_is_your_deed() {
    let mut character = Character::new();

    character
        .accept_quest(Tick(1), "Wolves Across the Border")
        .unwrap();

    let quest = id_of(&character, EntityType::Thing, "Wolves Across the Border").unwrap();
    assert!(holds(&character, QUEST_ACCEPTED, quest));
}

#[test]
fn only_a_finished_quest_is_done() {
    let mut character = Character::new();
    character
        .accept_quest(Tick(1), "The Fargodeep Mine")
        .unwrap();

    character
        .finish_quest(Tick(2), "Marshal Dughan", "Wolves Across the Border")
        .unwrap();

    assert!(character.has_done_quest("Wolves Across the Border"));
    assert!(!character.has_done_quest("The Fargodeep Mine"));
}

#[test]
fn the_level_that_you_hold_adds_no_event() {
    let mut character = Character::new();
    character.reach_level(Tick(1), 5).unwrap();
    let before = character.world().history().len();

    let result = character.reach_level(Tick(2), 5);

    assert_eq!(result, Ok(()));
    assert_eq!(character.world().history().len(), before);
}

#[test]
fn a_change_of_trust_past_the_end_of_the_band_adds_no_event() {
    let mut character = Character::new();
    character
        .adjust_trust(Tick(1), "Innkeeper Farley", 100)
        .unwrap();
    let before = character.world().history().len();

    let result = character.adjust_trust(Tick(2), "Innkeeper Farley", 5);

    assert_eq!(result, Ok(()));
    assert_eq!(character.world().history().len(), before);
}

#[test]
fn a_taken_quest_of_the_game_is_held_under_its_own_name() {
    let mut character = Character::new();

    character
        .take_game_quest(Tick(1), "Rattling the Rattlecages", GameQuestKind::Normal)
        .unwrap();

    let quest = id_of(
        &character,
        EntityType::Thing,
        "game quest: Rattling the Rattlecages",
    )
    .unwrap();
    assert!(holds(&character, GAME_QUEST_TAKEN, quest));
}

fn see(character: &mut Character, name: &str, reaction: Reaction, creature: Option<&str>) {
    character
        .see_npc(Tick(1), name, reaction, creature)
        .unwrap();
}

#[test]
fn seeing_an_npc_is_not_meeting_it() {
    let mut character = Character::new();

    see(
        &mut character,
        "Keeper Tessa",
        Reaction::Friendly,
        Some("humanoid"),
    );

    assert!(character.has_seen("Keeper Tessa"));
    assert!(!character.has_met("Keeper Tessa"));
}

#[test]
fn a_friendly_npc_that_you_saw_is_someone_to_meet() {
    let mut character = Character::new();

    see(
        &mut character,
        "Keeper Tessa",
        Reaction::Friendly,
        Some("humanoid"),
    );

    assert_eq!(character.npcs_to_meet(), ["Keeper Tessa"]);
    assert!(character.foes_seen().is_empty());
}

#[test]
fn a_hostile_creature_is_a_foe_and_never_someone_to_meet() {
    let mut character = Character::new();

    see(&mut character, "Duskbat", Reaction::Hostile, Some("beast"));

    assert!(character.npcs_to_meet().is_empty());
    assert_eq!(character.foes_seen(), ["Duskbat"]);
}

#[test]
fn a_beast_is_never_someone_to_meet_even_a_friendly_one() {
    let mut character = Character::new();

    see(
        &mut character,
        "Old Mule",
        Reaction::Friendly,
        Some("beast"),
    );
    see(
        &mut character,
        "Barn Cat",
        Reaction::Friendly,
        Some("critter"),
    );

    assert!(character.npcs_to_meet().is_empty());
}

#[test]
fn an_npc_that_you_met_and_later_saw_hostile_is_no_one_to_meet() {
    let mut character = Character::new();
    character.meet_npc(Tick(1), "Guard Rolf").unwrap();

    see(
        &mut character,
        "Guard Rolf",
        Reaction::Hostile,
        Some("humanoid"),
    );

    assert!(character.npcs_to_meet().is_empty());
    assert_eq!(character.foes_seen(), ["Guard Rolf"]);
}

#[test]
fn a_foe_seen_friendly_later_is_someone_to_meet_again() {
    let mut character = Character::new();
    see(&mut character, "Guard Rolf", Reaction::Hostile, None);

    see(&mut character, "Guard Rolf", Reaction::Friendly, None);

    assert_eq!(character.npcs_to_meet(), ["Guard Rolf"]);
    assert!(character.foes_seen().is_empty());
}

#[test]
fn an_npc_both_met_and_seen_is_listed_once() {
    let mut character = Character::new();
    character.meet_npc(Tick(1), "Keeper Tessa").unwrap();

    see(&mut character, "Keeper Tessa", Reaction::Friendly, None);

    assert_eq!(character.npcs_to_meet(), ["Keeper Tessa"]);
}

#[test]
fn the_same_sighting_twice_adds_no_events() {
    let mut character = Character::new();
    see(&mut character, "Duskbat", Reaction::Hostile, Some("beast"));
    let events = character.world().history().len();

    see(&mut character, "Duskbat", Reaction::Hostile, Some("beast"));

    assert_eq!(character.world().history().len(), events);
}

#[test]
fn the_first_meeting_keeps_its_event_and_its_time() {
    let mut character = Character::new();
    character.meet_npc(Tick(5), "Innkeeper Farley").unwrap();

    character.meet_npc(Tick(9), "Innkeeper Farley").unwrap();

    let (event, at) = character.first_met("Innkeeper Farley").unwrap();
    assert_eq!(at, Tick(5));
    assert_eq!(
        character.world().history().get(event).unwrap().tick,
        Tick(5)
    );
}

#[test]
fn a_sighting_opens_no_first_meeting() {
    let mut character = Character::new();

    see(&mut character, "Keeper Tessa", Reaction::Friendly, None);

    assert_eq!(character.first_met("Keeper Tessa"), None);
}

/// You entered The Deadmines and defeated Edwin there, and defeated Hogger in Elwynn.
fn dungeon_run() -> Character {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Elwynn Forest", Some("Goldshire"))
        .unwrap();
    character.defeat_npc(Tick(2), "Hogger").unwrap();
    character
        .enter_zone(Tick(3), "The Deadmines", None)
        .unwrap();
    character
        .mark_instance(
            Tick(4),
            "The Deadmines",
            timeways_story::places::InstanceKind::Dungeon,
        )
        .unwrap();
    character.defeat_npc(Tick(5), "Edwin VanCleef").unwrap();
    character
}

#[test]
fn an_outdoor_zone_is_no_dungeon() {
    let character = dungeon_run();

    assert_eq!(character.dungeons_entered(), ["The Deadmines"]);
}

#[test]
fn a_rare_defeated_outside_a_dungeon_is_no_boss() {
    let character = dungeon_run();

    assert_eq!(character.bosses_defeated(), ["Edwin VanCleef"]);
}

#[test]
fn a_game_quest_turned_in_is_no_longer_open() {
    let mut character = Character::new();
    character
        .take_game_quest(Tick(1), "Wanted: Hogger", GameQuestKind::Normal)
        .unwrap();
    character
        .take_game_quest(Tick(2), "Red Linen Goods", GameQuestKind::Normal)
        .unwrap();

    character
        .finish_game_quest(Tick(3), "Wanted: Hogger", GameQuestKind::Normal)
        .unwrap();

    assert_eq!(character.game_quests_open(), ["Red Linen Goods"]);
    assert_eq!(character.game_quests_done(), ["Wanted: Hogger"]);
}

#[test]
fn the_race_and_the_class_stay_in_the_world() {
    let mut character = Character::new();

    character
        .describe(Tick(1), Race::NightElf, Class::Druid)
        .unwrap();
    let events: Vec<Event> = character.world().history().iter().cloned().collect();
    let replayed = Character::from_history(&events).unwrap();

    assert_eq!(replayed.race(), Some(Race::NightElf));
    assert_eq!(replayed.class(), Some(Class::Druid));
}

#[test]
fn a_second_description_at_the_next_login_adds_nothing() {
    let mut character = Character::new();
    character
        .describe(Tick(1), Race::Troll, Class::Shaman)
        .unwrap();
    let before = character.world().history().len();

    character
        .describe(Tick(2), Race::Troll, Class::Shaman)
        .unwrap();

    assert_eq!(character.world().history().len(), before);
}

#[test]
fn a_character_that_the_addon_never_described_has_no_race_and_no_class() {
    let character = Character::new();

    assert_eq!(character.race(), None);
    assert_eq!(character.class(), None);
}

#[test]
fn the_titles_come_oldest_first() {
    let mut character = Character::new();
    character.earn_title(Tick(1), "Bookworm").unwrap();
    character.earn_title(Tick(2), "Slap Happy").unwrap();

    assert_eq!(character.titles(), ["Bookworm", "Slap Happy"]);
}
