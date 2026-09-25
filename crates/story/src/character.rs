//! The world of one character, fed by game events (GAMEPLAY.md 5.1 and 5.2).

use crate::record::{GameEvent, Record};
use crate::vocabulary::{self, LEVEL, MET, VISITED};
use hourglass::{EntityId, EntityType, EventKind, LOCATED_IN, Rejection, Tick, World};

/// Every reason at once, as Hourglass gives them.
pub type Refusal = Vec<Rejection>;

const YOU: &str = "you";

pub struct Character {
    world: World,
    you: EntityId,
}

impl Default for Character {
    fn default() -> Self {
        Self::new()
    }
}

impl Character {
    #[must_use]
    pub fn new() -> Self {
        let mut world = World::new(vocabulary::vocabulary());
        let you = world.next_entity_id();
        let kind = EventKind::EntityCreated {
            id: you,
            entity_type: EntityType::Person,
            name: YOU.to_string(),
        };
        // An empty world has nothing for the check to find.
        world.commit(Tick(0), kind);
        Character { world, you }
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub fn you(&self) -> EntityId {
        self.you
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass. The events before it stay in the world.
    pub fn apply(&mut self, record: &Record) -> Result<(), Refusal> {
        let at = record.at;
        match &record.event {
            GameEvent::ZoneEntered { zone, subzone } => self.enter(at, zone, subzone.as_deref()),
            GameEvent::NpcMet { name } => self.meet(at, name),
            GameEvent::LevelReached { level } => self.reach_level(at, i64::from(*level)),
        }
    }

    fn enter(&mut self, at: Tick, zone: &str, subzone: Option<&str>) -> Result<(), Refusal> {
        let zone_id = self.find_or_create(at, EntityType::Place, zone)?;
        self.start_once(at, self.you, VISITED, zone_id)?;
        // WoW gives the zone name as the subzone in some places, for example in capitals.
        let Some(subzone) = subzone.filter(|name| *name != zone) else {
            return self.settle(at, self.you, zone_id);
        };
        let subzone_id = self.find_or_create(at, EntityType::Place, subzone)?;
        self.settle(at, subzone_id, zone_id)?;
        self.start_once(at, self.you, VISITED, subzone_id)?;
        self.settle(at, self.you, subzone_id)
    }

    fn meet(&mut self, at: Tick, name: &str) -> Result<(), Refusal> {
        let npc = self.find_or_create(at, EntityType::Person, name)?;
        if let Some(here) = self.world.location_of(self.you) {
            self.settle(at, npc, here)?;
        }
        self.start_once(at, self.you, MET, npc)
    }

    fn reach_level(&mut self, at: Tick, level: i64) -> Result<(), Refusal> {
        let held = self.world.entity(self.you).and_then(|you| you.value(LEVEL));
        let kind = match held {
            Some(held) if held == level => return Ok(()),
            Some(held) => EventKind::FactUpdate {
                entity: self.you,
                name: LEVEL.to_string(),
                linked_to: None,
                from: held,
                to: level,
            },
            None => EventKind::FactStart {
                entity: self.you,
                name: LEVEL.to_string(),
                value: Some(level),
                linked_to: None,
            },
        };
        self.propose(at, kind)
    }

    fn find_or_create(
        &mut self,
        at: Tick,
        entity_type: EntityType,
        name: &str,
    ) -> Result<EntityId, Refusal> {
        if let Some(id) = self.find(entity_type, name) {
            return Ok(id);
        }
        let id = self.world.next_entity_id();
        let kind = EventKind::EntityCreated {
            id,
            entity_type,
            name: name.to_string(),
        };
        self.propose(at, kind)?;
        Ok(id)
    }

    fn find(&self, entity_type: EntityType, name: &str) -> Option<EntityId> {
        self.world
            .entities()
            .find(|entity| entity.entity_type == entity_type && entity.name == name)
            .map(|entity| entity.id)
    }

    fn settle(&mut self, at: Tick, entity: EntityId, place: EntityId) -> Result<(), Refusal> {
        if self.world.location_of(entity) == Some(place) {
            return Ok(());
        }
        self.start(at, entity, LOCATED_IN, place)
    }

    fn start_once(
        &mut self,
        at: Tick,
        entity: EntityId,
        name: &str,
        target: EntityId,
    ) -> Result<(), Refusal> {
        let held = self
            .world
            .entity(entity)
            .and_then(|e| e.fact(name, Some(target)));
        if held.is_some() {
            return Ok(());
        }
        self.start(at, entity, name, target)
    }

    fn start(
        &mut self,
        at: Tick,
        entity: EntityId,
        name: &str,
        target: EntityId,
    ) -> Result<(), Refusal> {
        let kind = EventKind::FactStart {
            entity,
            name: name.to_string(),
            value: None,
            linked_to: Some(target),
        };
        self.propose(at, kind)
    }

    fn propose(&mut self, at: Tick, kind: EventKind) -> Result<(), Refusal> {
        self.world.propose(at, kind).map(|_| ())
    }
}
