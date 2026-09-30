use hourglass::{
    Contradiction, EntityId, EntityType, EventId, EventKind, Malformed, Rejection, Tick, World,
    vocabulary_sound,
};
use timeways_story::vocabulary::{
    CLASS_QUEST, DEAD, DEATHS, DEFEATED, DUNGEON, GAME_QUEST_DONE, GAME_QUEST_TAKEN, KNOWS_LORE,
    LEADER_OF, LEVEL, MAP_X, MAP_Y, MARK_OF, MARKED_BY, MEMBER_OF, MET, NEMESIS, ON_MAP,
    QUEST_ACCEPTED, QUEST_DONE, QUEST_OFFERED, RAID, SLAPPED, TITLE, TRUSTS, VISITED, vocabulary,
};

const NOW: Tick = Tick(1);

struct Cast {
    world: World,
    you: EntityId,
    innkeeper: EntityId,
    goldshire: EntityId,
    guild: EntityId,
}

fn cast() -> Cast {
    let mut world = World::new(vocabulary());
    let you = create(&mut world, EntityType::Person, "you");
    let innkeeper = create(&mut world, EntityType::Person, "Innkeeper Farley");
    let goldshire = create(&mut world, EntityType::Place, "Goldshire");
    let guild = create(&mut world, EntityType::Faction, "your guild");
    Cast {
        world,
        you,
        innkeeper,
        goldshire,
        guild,
    }
}

fn create(world: &mut World, entity_type: EntityType, name: &str) -> EntityId {
    let id = world.next_entity_id();
    let kind = EventKind::EntityCreated {
        id,
        entity_type,
        name: name.to_string(),
    };
    assert!(world.propose(NOW, kind).is_ok());
    id
}

fn start(
    world: &mut World,
    entity: EntityId,
    name: &str,
    value: Option<i64>,
    linked_to: Option<EntityId>,
) -> Result<EventId, Vec<Rejection>> {
    let kind = EventKind::FactStart {
        entity,
        name: name.to_string(),
        value,
        linked_to,
    };
    world.propose(NOW, kind)
}

fn update(
    world: &mut World,
    entity: EntityId,
    name: &str,
    linked_to: Option<EntityId>,
    (from, to): (i64, i64),
) -> Result<EventId, Vec<Rejection>> {
    let kind = EventKind::FactUpdate {
        entity,
        name: name.to_string(),
        linked_to,
        from,
        to,
    };
    world.propose(NOW, kind)
}

fn end(
    world: &mut World,
    entity: EntityId,
    name: &str,
    linked_to: Option<EntityId>,
) -> Result<EventId, Vec<Rejection>> {
    let kind = EventKind::FactEnd {
        entity,
        name: name.to_string(),
        linked_to,
    };
    world.propose(NOW, kind)
}

fn refused_as_backward(result: Result<EventId, Vec<Rejection>>) -> bool {
    result.is_err_and(|faults| {
        faults.iter().any(|fault| {
            matches!(
                fault,
                Rejection::Malformed(Malformed::Backward { .. })
                    | Rejection::Contradiction(Contradiction::Backward { .. })
            )
        })
    })
}

fn refused_as_out_of_band(result: Result<EventId, Vec<Rejection>>) -> bool {
    result.is_err_and(|faults| {
        faults
            .iter()
            .any(|fault| matches!(fault, Rejection::Malformed(Malformed::OutOfBand { .. })))
    })
}

fn refused_as_wrong_type(result: Result<EventId, Vec<Rejection>>) -> bool {
    result.is_err_and(|faults| {
        faults.iter().any(|fault| {
            matches!(
                fault,
                Rejection::Malformed(Malformed::TypeNotAllowed { .. })
            )
        })
    })
}

#[test]
fn vocabulary_is_sound() {
    assert_eq!(vocabulary_sound(&vocabulary()), Vec::new());
}

#[test]
fn vocabulary_holds_every_name_of_the_spec_and_located_in() {
    let names = [
        MET,
        TRUSTS,
        VISITED,
        KNOWS_LORE,
        DEAD,
        DEATHS,
        DEFEATED,
        NEMESIS,
        QUEST_OFFERED,
        QUEST_ACCEPTED,
        QUEST_DONE,
        GAME_QUEST_TAKEN,
        GAME_QUEST_DONE,
        CLASS_QUEST,
        DUNGEON,
        RAID,
        MARKED_BY,
        MARK_OF,
        LEVEL,
        SLAPPED,
        TITLE,
        MEMBER_OF,
        LEADER_OF,
        ON_MAP,
        MAP_X,
        MAP_Y,
        hourglass::LOCATED_IN,
    ];

    let vocabulary = vocabulary();

    assert_eq!(vocabulary.len(), names.len());
    for name in names {
        assert!(vocabulary.holds(name), "{name} is missing");
    }
}

#[test]
fn level_never_falls() {
    let mut cast = cast();
    start(&mut cast.world, cast.you, LEVEL, Some(12), None).unwrap();

    let result = update(&mut cast.world, cast.you, LEVEL, None, (12, 11));

    assert!(refused_as_backward(result));
}

#[test]
fn level_stays_inside_one_to_sixty() {
    let mut cast = cast();

    let result = start(&mut cast.world, cast.you, LEVEL, Some(61), None);

    assert!(refused_as_out_of_band(result));
}

#[test]
fn the_dead_stay_dead() {
    let mut cast = cast();
    start(&mut cast.world, cast.innkeeper, DEAD, None, None).unwrap();

    let result = end(&mut cast.world, cast.innkeeper, DEAD, None);

    assert!(refused_as_backward(result));
}

#[test]
fn a_visit_never_ends() {
    let mut cast = cast();
    start(
        &mut cast.world,
        cast.you,
        VISITED,
        None,
        Some(cast.goldshire),
    )
    .unwrap();

    let result = end(&mut cast.world, cast.you, VISITED, Some(cast.goldshire));

    assert!(refused_as_backward(result));
}

#[test]
fn only_a_place_can_be_visited() {
    let mut cast = cast();

    let result = start(
        &mut cast.world,
        cast.you,
        VISITED,
        None,
        Some(cast.innkeeper),
    );

    assert!(refused_as_wrong_type(result));
}

#[test]
fn trust_falls_after_a_slap() {
    let mut cast = cast();
    start(
        &mut cast.world,
        cast.innkeeper,
        TRUSTS,
        Some(20),
        Some(cast.you),
    )
    .unwrap();

    let result = update(
        &mut cast.world,
        cast.innkeeper,
        TRUSTS,
        Some(cast.you),
        (20, -30),
    );

    assert!(result.is_ok());
}

#[test]
fn trust_stays_inside_minus_one_hundred_to_one_hundred() {
    let mut cast = cast();

    let result = start(
        &mut cast.world,
        cast.innkeeper,
        TRUSTS,
        Some(-101),
        Some(cast.you),
    );

    assert!(refused_as_out_of_band(result));
}

#[test]
fn a_slap_count_never_falls() {
    let mut cast = cast();
    start(
        &mut cast.world,
        cast.you,
        SLAPPED,
        Some(3),
        Some(cast.innkeeper),
    )
    .unwrap();

    let result = update(
        &mut cast.world,
        cast.you,
        SLAPPED,
        Some(cast.innkeeper),
        (3, 2),
    );

    assert!(refused_as_backward(result));
}

#[test]
fn a_guild_can_defeat_a_person() {
    let mut cast = cast();

    let result = start(
        &mut cast.world,
        cast.guild,
        DEFEATED,
        Some(1),
        Some(cast.innkeeper),
    );

    assert!(result.is_ok());
}

#[test]
fn a_place_cannot_defeat_anyone() {
    let mut cast = cast();

    let result = start(
        &mut cast.world,
        cast.goldshire,
        DEFEATED,
        Some(1),
        Some(cast.you),
    );

    assert!(refused_as_wrong_type(result));
}

#[test]
fn a_feud_can_end() {
    let mut cast = cast();
    start(
        &mut cast.world,
        cast.innkeeper,
        NEMESIS,
        Some(7),
        Some(cast.you),
    )
    .unwrap();

    let result = end(&mut cast.world, cast.innkeeper, NEMESIS, Some(cast.you));

    assert!(result.is_ok());
}

#[test]
fn a_member_can_leave_the_guild() {
    let mut cast = cast();
    start(&mut cast.world, cast.you, MEMBER_OF, None, Some(cast.guild)).unwrap();

    let result = end(&mut cast.world, cast.you, MEMBER_OF, Some(cast.guild));

    assert!(result.is_ok());
}

#[test]
fn a_point_stays_on_its_map() {
    let mut cast = cast();

    let inside = start(&mut cast.world, cast.goldshire, MAP_X, Some(1000), None);
    let past = start(&mut cast.world, cast.innkeeper, MAP_Y, Some(1001), None);
    let no_map = start(&mut cast.world, cast.innkeeper, ON_MAP, Some(0), None);

    assert!(inside.is_ok());
    assert!(past.is_err());
    assert!(no_map.is_err());
}
