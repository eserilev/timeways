//! The world of one character, fed by game events (GAMEPLAY.md 5.1 and 5.2).

use crate::vocabulary::{self, DEATHS, DEFEATED, LEVEL, MET, SLAPPED, TRUST, TRUSTS, VISITED};
use hourglass::{
    EntityId, EntityType, Event, EventHistory, EventKind, LOCATED_IN, Rejection, Tick, World,
};

/// Every reason at once, as Hourglass gives them.
pub type Refusal = Vec<Rejection>;

const YOU: &str = "you";

/// The trust that one slap costs.
const SLAP_TRUST: i64 = 10;

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

    /// The character of a saved history, or None when the history does not start with
    /// the founding of the character.
    #[must_use]
    pub fn from_history(events: &[Event]) -> Option<Self> {
        let first = events.first()?;
        let EventKind::EntityCreated {
            id: you,
            entity_type: EntityType::Person,
            name,
        } = &first.kind
        else {
            return None;
        };
        if name != YOU {
            return None;
        }
        let mut history = EventHistory::new();
        for event in events {
            history.push(event.tick, event.kind.clone());
        }
        let world = World::replay(vocabulary::vocabulary(), &history);
        Some(Character { world, you: *you })
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub fn you(&self) -> EntityId {
        self.you
    }

    #[must_use]
    pub fn level(&self) -> Option<i64> {
        self.world.entity(self.you).and_then(|you| you.value(LEVEL))
    }

    /// Two places can share a name, so any place with this name counts.
    #[must_use]
    pub fn has_visited(&self, place: &str) -> bool {
        self.holds_about(VISITED, place)
    }

    #[must_use]
    pub fn has_met(&self, npc: &str) -> bool {
        self.holds_about(MET, npc)
    }

    /// The place of an NPC: where you met or fought it last.
    #[must_use]
    pub fn place_of(&self, npc: &str) -> Option<&str> {
        let id = self.find(EntityType::Person, npc)?;
        let place = self.world.location_of(id)?;
        Some(self.world.entity(place)?.name.as_str())
    }

    #[must_use]
    pub fn trust_of(&self, npc: &str) -> Option<i64> {
        let id = self.find(EntityType::Person, npc)?;
        self.world.entity(id)?.fact(TRUSTS, Some(self.you))?.value
    }

    #[must_use]
    pub fn slaps_of(&self, npc: &str) -> Option<i64> {
        let id = self.find(EntityType::Person, npc)?;
        self.world.entity(self.you)?.fact(SLAPPED, Some(id))?.value
    }

    /// Where you stand, then each place around it: the subzone, then the zone.
    #[must_use]
    pub fn place_names(&self) -> Vec<&str> {
        let Some(here) = self.world.location_of(self.you) else {
            return Vec::new();
        };
        let mut places = vec![here];
        places.extend(self.world.ancestry(here));
        places
            .into_iter()
            .filter_map(|id| self.world.entity(id))
            .map(|place| place.name.as_str())
            .collect()
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass. The events before it stay in the world,
    /// because each Hourglass event stands alone.
    pub fn enter_zone(
        &mut self,
        at: Tick,
        zone: &str,
        subzone: Option<&str>,
    ) -> Result<(), Refusal> {
        let zone_id = self.place(at, zone, None)?;
        self.start_once(at, self.you, VISITED, zone_id)?;
        // WoW gives the zone name as the subzone in some places, for example in capitals.
        let Some(subzone) = subzone.filter(|name| *name != zone) else {
            return self.settle(at, self.you, zone_id);
        };
        let subzone_id = self.place(at, subzone, Some(zone_id))?;
        self.start_once(at, self.you, VISITED, subzone_id)?;
        self.settle(at, self.you, subzone_id)
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn meet_npc(&mut self, at: Tick, name: &str) -> Result<(), Refusal> {
        let npc = self.find_or_create(at, EntityType::Person, name)?;
        if let Some(here) = self.world.location_of(self.you) {
            self.settle(at, npc, here)?;
        }
        self.start_once(at, self.you, MET, npc)
    }

    /// A kill is a deed of the killer, and the target stays alive, because the game brings
    /// it back (GAMEPLAY.md 5.13). The foe lives where you fought it last.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn defeat_npc(&mut self, at: Tick, name: &str) -> Result<(), Refusal> {
        let foe = self.find_or_create(at, EntityType::Person, name)?;
        if let Some(here) = self.world.location_of(self.you) {
            self.settle(at, foe, here)?;
        }
        self.count_up(at, self.you, DEFEATED, Some(foe))
    }

    /// A slap has consequences: the NPC counts it, and trusts you less (GAMEPLAY.md 5.4.1).
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn slap(&mut self, at: Tick, name: &str) -> Result<(), Refusal> {
        let npc = self.find_or_create(at, EntityType::Person, name)?;
        if let Some(here) = self.world.location_of(self.you) {
            self.settle(at, npc, here)?;
        }
        // A slap is a meeting too, so the NPC shows on the People page.
        self.start_once(at, self.you, MET, npc)?;
        self.count_up(at, self.you, SLAPPED, Some(npc))?;
        self.change_trust(at, npc, -SLAP_TRUST)
    }

    /// A change of trust that a talk proposed (GAMEPLAY.md 3.5). It stops at the ends of
    /// the band.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn adjust_trust(&mut self, at: Tick, npc: &str, by: i64) -> Result<(), Refusal> {
        let id = self.find_or_create(at, EntityType::Person, npc)?;
        self.change_trust(at, id, by)
    }

    /// Your death is a deed of the killer, when the addon knows one (GAMEPLAY.md 5.13).
    /// The count of deaths comes last, so the journal finds the killer just before it.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn die(&mut self, at: Tick, killer: Option<&str>) -> Result<(), Refusal> {
        if let Some(killer) = killer {
            let foe = self.find_or_create(at, EntityType::Person, killer)?;
            if let Some(here) = self.world.location_of(self.you) {
                self.settle(at, foe, here)?;
            }
            self.count_up(at, foe, DEFEATED, Some(self.you))?;
        }
        self.count_up(at, self.you, DEATHS, None)
    }

    /// # Errors
    ///
    /// Returns the refusal of Hourglass, for example for a level lower than the one held.
    pub fn reach_level(&mut self, at: Tick, level: u8) -> Result<(), Refusal> {
        let level = i64::from(level);
        let kind = match self.level() {
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

    fn holds_about(&self, fact: &str, name: &str) -> bool {
        let Some(you) = self.world.entity(self.you) else {
            return false;
        };
        you.facts_named(fact)
            .filter_map(|fact| fact.linked_to)
            .any(|target| {
                self.world
                    .entity(target)
                    .is_some_and(|entity| entity.name == name)
            })
    }

    /// A zone is a place in no other place. A subzone is found by its name and its zone,
    /// because some subzone names repeat across zones, such as "The Great Sea".
    fn place(
        &mut self,
        at: Tick,
        name: &str,
        within: Option<EntityId>,
    ) -> Result<EntityId, Refusal> {
        let found = self.world.entities().find(|entity| {
            entity.entity_type == EntityType::Place
                && entity.name == name
                && entity.location() == within
        });
        if let Some(place) = found {
            return Ok(place.id);
        }
        let id = self.world.next_entity_id();
        let kind = EventKind::EntityCreated {
            id,
            entity_type: EntityType::Place,
            name: name.to_string(),
        };
        self.propose(at, kind)?;
        if let Some(zone) = within {
            self.settle(at, id, zone)?;
        }
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

    /// Trust starts at 0, and stops at the ends of its band.
    fn change_trust(&mut self, at: Tick, npc: EntityId, by: i64) -> Result<(), Refusal> {
        let held = self
            .world
            .entity(npc)
            .and_then(|entity| entity.fact(TRUSTS, Some(self.you)))
            .and_then(|fact| fact.value);
        let to = TRUST.clamp(held.unwrap_or(0) + by);
        let kind = match held {
            Some(from) if from == to => return Ok(()),
            Some(from) => EventKind::FactUpdate {
                entity: npc,
                name: TRUSTS.to_string(),
                linked_to: Some(self.you),
                from,
                to,
            },
            None => EventKind::FactStart {
                entity: npc,
                name: TRUSTS.to_string(),
                value: Some(to),
                linked_to: Some(self.you),
            },
        };
        self.propose(at, kind)
    }

    /// Starts a tally at 1, or adds 1 to the tally that the slot holds.
    fn count_up(
        &mut self,
        at: Tick,
        holder: EntityId,
        name: &str,
        target: Option<EntityId>,
    ) -> Result<(), Refusal> {
        let held = self
            .world
            .entity(holder)
            .and_then(|entity| entity.fact(name, target))
            .and_then(|fact| fact.value);
        let kind = match held {
            Some(count) => EventKind::FactUpdate {
                entity: holder,
                name: name.to_string(),
                linked_to: target,
                from: count,
                to: count + 1,
            },
            None => EventKind::FactStart {
                entity: holder,
                name: name.to_string(),
                value: Some(1),
                linked_to: target,
            },
        };
        self.propose(at, kind)
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
