//! The world of one character, fed by game events (GAMEPLAY.md 5.1 and 5.2).

use crate::input::{GameQuestKind, Reaction};
use crate::places::InstanceKind;
use crate::spot::{self, Spot};
use crate::vocabulary::{
    self, ANIMAL, CLASS_QUEST, DEAD, DEATHS, DEFEATED, DUNGEON, GAME_QUEST_DONE, GAME_QUEST_TAKEN,
    HOSTILE, LEVEL, MAP_X, MAP_Y, MARK_OF, MARKED_BY, MET, ON_MAP, QUEST_ACCEPTED, QUEST_DONE,
    QUEST_OFFERED, RAID, SEEN, SLAPPED, TALLY, TITLE, TRUST, TRUSTS, VISITED,
};
use hourglass::{
    Entity, EntityId, EntityType, Event, EventHistory, EventKind, Fact, LOCATED_IN, Rejection,
    Tick, World,
};

/// Every reason at once, as Hourglass gives them.
pub type Refusal = Vec<Rejection>;

const YOU: &str = "you";

/// The trust that one slap costs.
const SLAP_TRUST: i64 = 10;

/// The trust that a finished side quest earns with its giver. The model never picks it.
pub const QUEST_TRUST: i64 = 10;

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

    /// Your slaps, of every NPC together.
    #[must_use]
    pub fn slaps(&self) -> i64 {
        self.world.entity(self.you).map_or(0, |you| {
            you.facts_named(SLAPPED).filter_map(|fact| fact.value).sum()
        })
    }

    /// A title is a thing that you hold for good (GAMEPLAY.md 5.4.1).
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn earn_title(&mut self, at: Tick, title: &str) -> Result<(), Refusal> {
        let thing = self.find_or_create(at, EntityType::Thing, title)?;
        self.start_once(at, self.you, TITLE, thing)
    }

    #[must_use]
    pub fn has_title(&self, title: &str) -> bool {
        self.holds_about(TITLE, title)
    }

    /// Does this NPC share a past with you: trust, a slap, or a kill either way? A plain
    /// meeting is no history, so it makes no callback (GAMEPLAY.md 5.4.1).
    #[must_use]
    pub fn has_history_with(&self, npc: &str) -> bool {
        let Some(id) = self.find(EntityType::Person, npc) else {
            return false;
        };
        let theirs = self.world.entity(id).is_some_and(|entity| {
            entity.fact(TRUSTS, Some(self.you)).is_some()
                || entity.fact(DEFEATED, Some(self.you)).is_some()
        });
        let yours = self.world.entity(self.you).is_some_and(|you| {
            you.fact(SLAPPED, Some(id)).is_some() || you.fact(DEFEATED, Some(id)).is_some()
        });
        theirs || yours
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

    /// The zones that you visited: the places in no other place.
    #[must_use]
    pub fn visited_zones(&self) -> Vec<&str> {
        self.visited(|place| place.location().is_none())
    }

    #[must_use]
    pub fn visited_subzones(&self) -> Vec<&str> {
        self.visited(|place| place.location().is_some())
    }

    fn visited(&self, keep: impl Fn(&Entity) -> bool) -> Vec<&str> {
        self.linked_by_you(&[VISITED])
            .into_iter()
            .filter(|place| keep(place))
            .map(|place| place.name.as_str())
            .collect()
    }

    /// The NPCs that a task can send you to meet: met or seen, alive, never hostile, and
    /// never a beast (GAMEPLAY.md 3.4). The newest first, as for each list of targets.
    #[must_use]
    pub fn npcs_to_meet(&self) -> Vec<&str> {
        let known = self.linked_by_you(&[MET, SEEN]);
        let mut names: Vec<&str> = Vec::new();
        for npc in known {
            let blocked = [DEAD, HOSTILE, ANIMAL]
                .iter()
                .any(|flag| holds_flag(npc, flag));
            if !blocked && !names.contains(&npc.name.as_str()) {
                names.push(npc.name.as_str());
            }
        }
        names
    }

    /// The creatures that a task can send you to kill: seen hostile, and alive.
    #[must_use]
    pub fn foes_seen(&self) -> Vec<&str> {
        self.linked_by_you(&[SEEN])
            .into_iter()
            .filter(|npc| holds_flag(npc, HOSTILE) && !holds_flag(npc, DEAD))
            .map(|npc| npc.name.as_str())
            .collect()
    }

    /// The rares and bosses that you defeated (5.13).
    #[must_use]
    pub fn foes_defeated(&self) -> Vec<&str> {
        self.linked_by_you(&[DEFEATED])
            .into_iter()
            .map(|foe| foe.name.as_str())
            .collect()
    }

    #[must_use]
    pub fn has_seen(&self, npc: &str) -> bool {
        self.holds_about(SEEN, npc)
    }

    /// A hostile NPC or a beast has no task to give, and nothing to say (GAMEPLAY.md 3.4).
    #[must_use]
    pub fn is_hostile_or_animal(&self, npc: &str) -> bool {
        self.find(EntityType::Person, npc)
            .and_then(|id| self.world.entity(id))
            .is_some_and(|entity| holds_flag(entity, HOSTILE) || holds_flag(entity, ANIMAL))
    }

    #[must_use]
    pub fn is_dead(&self, npc: &str) -> bool {
        self.find(EntityType::Person, npc)
            .and_then(|id| self.world.entity(id))
            .is_some_and(|entity| entity.fact(DEAD, None).is_some())
    }

    /// The targets of your facts of these names, the newest fact first.
    fn linked_by_you(&self, names: &[&str]) -> Vec<&Entity> {
        let Some(you) = self.world.entity(self.you) else {
            return Vec::new();
        };
        let mut facts: Vec<&Fact> = names
            .iter()
            .flat_map(|name| you.facts_named(name))
            .collect();
        facts.sort_by_key(|fact| std::cmp::Reverse(fact.opened));
        facts
            .into_iter()
            .filter_map(|fact| self.world.entity(fact.linked_to?))
            .collect()
    }

    /// The zone where an NPC lives: the outermost place around it.
    #[must_use]
    pub fn zone_of_npc(&self, npc: &str) -> Option<&str> {
        let id = self.find(EntityType::Person, npc)?;
        self.zone_around(self.world.location_of(id)?)
    }

    /// The zone of a place: the place itself for a zone. A subzone name that repeats across
    /// zones gives one of them.
    #[must_use]
    pub fn zone_of_place(&self, place: &str) -> Option<&str> {
        self.zone_around(self.find(EntityType::Place, place)?)
    }

    fn zone_around(&self, place: EntityId) -> Option<&str> {
        let zone = self.world.ancestry(place).last().copied().unwrap_or(place);
        Some(self.world.entity(zone)?.name.as_str())
    }

    /// A quest is a thing that its giver offers you (GAMEPLAY.md 3.4).
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn offer_quest(&mut self, at: Tick, giver: &str, quest: &str) -> Result<(), Refusal> {
        let giver = self.find_or_create(at, EntityType::Person, giver)?;
        let quest = self.find_or_create(at, EntityType::Thing, quest)?;
        self.start_once(at, giver, QUEST_OFFERED, quest)
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn accept_quest(&mut self, at: Tick, quest: &str) -> Result<(), Refusal> {
        let quest = self.find_or_create(at, EntityType::Thing, quest)?;
        self.start_once(at, self.you, QUEST_ACCEPTED, quest)
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn finish_quest(&mut self, at: Tick, giver: &str, quest: &str) -> Result<(), Refusal> {
        let quest = self.find_or_create(at, EntityType::Thing, quest)?;
        self.start_once(at, self.you, QUEST_DONE, quest)?;
        let giver = self.find_or_create(at, EntityType::Person, giver)?;
        self.change_trust(at, giver, QUEST_TRUST)
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn take_game_quest(
        &mut self,
        at: Tick,
        title: &str,
        kind: GameQuestKind,
    ) -> Result<(), Refusal> {
        let quest = self.game_quest(at, title, kind)?;
        self.start_once(at, self.you, GAME_QUEST_TAKEN, quest)
    }

    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn finish_game_quest(
        &mut self,
        at: Tick,
        title: &str,
        kind: GameQuestKind,
    ) -> Result<(), Refusal> {
        let quest = self.game_quest(at, title, kind)?;
        self.start_once(at, self.you, GAME_QUEST_DONE, quest)
    }

    /// A quest of the game left a lasting buff or debuff on you. Each mark counts once.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn take_quest_mark(&mut self, at: Tick, quest: &str, mark: &str) -> Result<(), Refusal> {
        let quest = self.find_or_create(at, EntityType::Thing, &game_quest_name(quest))?;
        let mark = self.find_or_create(at, EntityType::Thing, &mark_name(mark))?;
        self.start_once(at, mark, MARK_OF, quest)?;
        self.start_once(at, self.you, MARKED_BY, mark)
    }

    /// The class mark comes before any fact about the quest, so a moment of the same batch
    /// already sees it.
    fn game_quest(
        &mut self,
        at: Tick,
        title: &str,
        kind: GameQuestKind,
    ) -> Result<EntityId, Refusal> {
        let quest = self.find_or_create(at, EntityType::Thing, &game_quest_name(title))?;
        let marked = self
            .world
            .entity(quest)
            .is_some_and(|entity| entity.fact(CLASS_QUEST, None).is_some());
        if kind == GameQuestKind::Class && !marked {
            let flag = EventKind::FactStart {
                entity: quest,
                name: CLASS_QUEST.to_string(),
                value: None,
                linked_to: None,
            };
            self.propose(at, flag)?;
        }
        Ok(quest)
    }

    #[must_use]
    pub fn has_done_quest(&self, quest: &str) -> bool {
        self.holds_about(QUEST_DONE, quest)
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

    /// The zone is an instance of the game. The mark stays, because an instance stays one.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn mark_instance(
        &mut self,
        at: Tick,
        zone: &str,
        kind: InstanceKind,
    ) -> Result<(), Refusal> {
        let zone = self.place(at, zone, None)?;
        let name = match kind {
            InstanceKind::Dungeon => DUNGEON,
            InstanceKind::Raid => RAID,
        };
        if self
            .world
            .entity(zone)
            .is_some_and(|place| place.fact(name, None).is_some())
        {
            return Ok(());
        }
        let mark = EventKind::FactStart {
            entity: zone,
            name: name.to_string(),
            value: None,
            linked_to: None,
        };
        self.propose(at, mark)
    }

    /// The place where you stand keeps its first position, and so does each place around
    /// it. You mostly stand in a subzone, so a zone takes the first position in any of them.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn mark_here(&mut self, at: Tick, spot: Spot) -> Result<(), Refusal> {
        let Some(here) = self.world.location_of(self.you) else {
            return Ok(());
        };
        let mut places = vec![here];
        places.extend(self.world.ancestry(here));
        for place in places {
            self.mark_spot(at, place, spot)?;
        }
        Ok(())
    }

    /// An NPC keeps the position of the first meeting that had one.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn mark_npc(&mut self, at: Tick, npc: &str, spot: Spot) -> Result<(), Refusal> {
        let Some(npc) = self.find(EntityType::Person, npc) else {
            return Ok(());
        };
        self.mark_spot(at, npc, spot)
    }

    fn mark_spot(&mut self, at: Tick, entity: EntityId, spot: Spot) -> Result<(), Refusal> {
        if spot::spot_of(&self.world, entity).is_some() {
            return Ok(());
        }
        let facts = [
            (ON_MAP, i64::from(spot.map)),
            (MAP_X, i64::from(spot.x)),
            (MAP_Y, i64::from(spot.y)),
        ];
        for (name, value) in facts {
            let kind = EventKind::FactStart {
                entity,
                name: name.to_string(),
                value: Some(value),
                linked_to: None,
            };
            self.propose(at, kind)?;
        }
        Ok(())
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

    /// Seeing is not meeting. The last sighting says whether you can attack the NPC. The NPC
    /// lives where you saw it last.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn see_npc(
        &mut self,
        at: Tick,
        name: &str,
        reaction: Reaction,
        creature: Option<&str>,
    ) -> Result<(), Refusal> {
        let npc = self.find_or_create(at, EntityType::Person, name)?;
        if let Some(here) = self.world.location_of(self.you) {
            self.settle(at, npc, here)?;
        }
        self.start_once(at, self.you, SEEN, npc)?;
        self.set_hostile(at, npc, reaction)?;
        if creature.is_some_and(is_animal) {
            self.flag_once(at, npc, ANIMAL)?;
        }
        Ok(())
    }

    /// A talk in the game shows that the NPC is a friend now, so it is no longer hostile.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn befriend(&mut self, at: Tick, npc: &str) -> Result<(), Refusal> {
        let Some(npc) = self.find(EntityType::Person, npc) else {
            return Ok(());
        };
        self.set_hostile(at, npc, Reaction::Friendly)
    }

    fn set_hostile(&mut self, at: Tick, npc: EntityId, reaction: Reaction) -> Result<(), Refusal> {
        let held = self
            .world
            .entity(npc)
            .is_some_and(|entity| holds_flag(entity, HOSTILE));
        let kind = match (reaction, held) {
            (Reaction::Hostile, false) => EventKind::FactStart {
                entity: npc,
                name: HOSTILE.to_string(),
                value: None,
                linked_to: None,
            },
            (Reaction::Friendly, true) => EventKind::FactEnd {
                entity: npc,
                name: HOSTILE.to_string(),
                linked_to: None,
            },
            _ => return Ok(()),
        };
        self.propose(at, kind)
    }

    fn flag_once(&mut self, at: Tick, entity: EntityId, name: &str) -> Result<(), Refusal> {
        let held = self
            .world
            .entity(entity)
            .is_some_and(|holder| holds_flag(holder, name));
        if held {
            return Ok(());
        }
        let flag = EventKind::FactStart {
            entity,
            name: name.to_string(),
            value: None,
            linked_to: None,
        };
        self.propose(at, flag)
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
        let to = next_trust(held, by);
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

    /// Starts a tally at 1, or adds 1 to the tally that the slot holds, up to its top.
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
            // Hourglass refuses a count past its band, and the refusal loses the rest
            // of the event, such as the trust that a slap costs.
            Some(count) if count >= TALLY.max => return Ok(()),
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

fn holds_flag(entity: &Entity, flag: &str) -> bool {
    entity.fact(flag, None).is_some()
}

/// The creature types of the game that are animals. The addon sends the English name, in
/// lower case.
fn is_animal(creature: &str) -> bool {
    matches!(creature, "beast" | "critter")
}

/// The trust after a change of `by`, inside the band. An NPC with no trust yet starts at 0.
#[must_use]
pub fn next_trust(held: Option<i64>, by: i64) -> i64 {
    TRUST.clamp(held.unwrap_or(0).saturating_add(by))
}

const GAME_QUEST_PREFIX: &str = "game quest: ";

/// A quest of the game lives as a thing apart from the side quests and the titles.
fn game_quest_name(title: &str) -> String {
    format!("{GAME_QUEST_PREFIX}{title}")
}

/// The title of a quest of the game, from the name of its thing.
#[must_use]
pub fn title_of_game_quest(name: &str) -> Option<&str> {
    name.strip_prefix(GAME_QUEST_PREFIX)
}

const MARK_PREFIX: &str = "mark: ";

fn mark_name(mark: &str) -> String {
    format!("{MARK_PREFIX}{mark}")
}

/// The name of a buff or debuff, from the name of its thing.
#[must_use]
pub fn title_of_mark(name: &str) -> Option<&str> {
    name.strip_prefix(MARK_PREFIX)
}
