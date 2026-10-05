//! The world of one character, fed by game events (GAMEPLAY.md 5.1 and 5.2).

use crate::gear::{Quality, item_name};
use crate::input::{GameQuestKind, Reaction};
use crate::mounts::mount_name;
use crate::pack::Link;
use crate::places::InstanceKind;
use crate::race_class::{Class, Race};
use crate::spot::{self, Spot};
use crate::vocabulary::{
    self, ANIMAL, CLASS, CLASS_QUEST, DEAD, DEATHS, DEFEATED, DUNGEON, FIRST_EPIC_ITEM,
    FIRST_EPIC_MOUNT, FIRST_MOUNT, GAME_QUEST_DONE, GAME_QUEST_TAKEN, HOSTILE, LEVEL, MAP_X, MAP_Y,
    MARK_OF, MARKED_BY, MET, ON_MAP, QUALITY, QUEST_ACCEPTED, QUEST_DONE, QUEST_OFFERED, RACE,
    RAID, SEEN, SLAPPED, TALLY, TITLE, TRUSTS, UPGRADED, VISITED,
};
use crate::vocabulary::{BATTLEGROUND, BG_WON, PVP_RANK, RESTING, WORLD_BOSS};
use hourglass::{
    Entity, EntityId, EntityType, Event, EventHistory, EventId, EventKind, Fact, LOCATED_IN,
    Rejection, Tick, World,
};

/// Whether you rest at an inn or in a city. The JSON of the addon has no booleans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resting {
    Yes,
    No,
}

/// Every reason at once, as Hourglass gives them.
pub type Refusal = Vec<Rejection>;

const YOU: &str = "you";

/// The trust that one slap costs.
pub const SLAP_TRUST: i64 = 10;

/// The trust that a finished side quest earns with its giver. The model never picks it.
pub const QUEST_TRUST: i64 = 10;

/// An item that the hero put on, after the checks of the story program.
#[derive(Clone, Copy, Debug)]
pub struct Item<'a> {
    pub name: &'a str,
    /// The inventory slot of the game, from 1 to 19.
    pub slot: u8,
    pub quality: Quality,
}

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

    /// The spoiler limit (GAMEPLAY.md 3.1): every place of the links is visited, and every
    /// NPC met.
    #[must_use]
    pub fn knows_all(&self, links: &[Link]) -> bool {
        links.iter().all(|link| match link {
            Link::Place(name) => self.has_visited(name),
            Link::Npc(name) => self.has_met(name),
            Link::Common => true,
        })
    }

    #[must_use]
    pub fn has_met(&self, npc: &str) -> bool {
        self.holds_about(MET, npc)
    }

    /// The event that opened the `met` fact of this NPC, and its time. Seeing is not
    /// meeting, so a sighting gives None.
    #[must_use]
    pub fn first_met(&self, npc: &str) -> Option<(EventId, Tick)> {
        let id = self.world.find(EntityType::Person, npc)?;
        let met = self.world.entity(self.you)?.fact(MET, Some(id))?;
        Some((met.opened, self.world.history().get(met.opened)?.tick))
    }

    /// The events behind what the world holds about each thing of this name: the event
    /// that made it, and the event that opened each fact that it holds or that points to
    /// it.
    #[must_use]
    pub fn events_about(&self, name: &str) -> Vec<EventId> {
        let ids = self.world.named(name);
        let mut events: Vec<EventId> = ids
            .iter()
            .filter_map(|id| self.world.entity(*id))
            .map(|entity| entity.created)
            .collect();
        for id in ids {
            let held = self
                .world
                .entity(*id)
                .into_iter()
                .flat_map(|entity| &entity.facts);
            events.extend(held.map(|fact| fact.opened));
            let pointing = self.world.facts_linked_to(*id);
            events.extend(pointing.into_iter().map(|(_, fact)| fact.opened));
        }
        events
    }

    /// The place of an NPC: where you met or fought it last.
    #[must_use]
    pub fn place_of(&self, npc: &str) -> Option<&str> {
        let id = self.world.find(EntityType::Person, npc)?;
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

    /// The titles that you earned, oldest first.
    #[must_use]
    pub fn titles(&self) -> Vec<&str> {
        self.names_held(TITLE)
    }

    /// The race and the class stay once known. The addon sends them at each login.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn describe(&mut self, at: Tick, race: Race, class: Class) -> Result<(), Refusal> {
        let race = self.find_or_create(at, EntityType::Thing, race.word())?;
        self.start_once(at, self.you, RACE, race)?;
        let class = self.find_or_create(at, EntityType::Thing, class.word())?;
        self.start_once(at, self.you, CLASS, class)
    }

    #[must_use]
    pub fn race(&self) -> Option<Race> {
        self.names_held(RACE).into_iter().find_map(Race::from_word)
    }

    #[must_use]
    pub fn class(&self) -> Option<Class> {
        self.names_held(CLASS)
            .into_iter()
            .find_map(Class::from_word)
    }

    /// Does this NPC share a past with you: trust, a slap, or a kill either way? A plain
    /// meeting is no history, so it makes no callback (GAMEPLAY.md 5.4.1).
    #[must_use]
    pub fn has_history_with(&self, npc: &str) -> bool {
        let Some(id) = self.world.find(EntityType::Person, npc) else {
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
        let id = self.world.find(EntityType::Person, npc)?;
        self.world.entity(id)?.fact(TRUSTS, Some(self.you))?.value
    }

    /// The event of the newest change of the trust of an NPC, and whether it went up.
    #[must_use]
    pub fn last_trust_change(&self, npc: &str) -> Option<(EventId, bool)> {
        let id = self.world.find(EntityType::Person, npc)?;
        let opened = self.world.entity(id)?.fact(TRUSTS, Some(self.you))?.opened;
        let up = match &self.world.history().get(opened)?.kind {
            EventKind::FactUpdate { from, to, .. } => to > from,
            EventKind::FactStart { value, .. } => value.is_some_and(|value| value > 0),
            _ => return None,
        };
        Some((opened, up))
    }

    #[must_use]
    pub fn slaps_of(&self, npc: &str) -> Option<i64> {
        let id = self.world.find(EntityType::Person, npc)?;
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

    /// The dungeons and raids that you entered: the places that you visited with the mark of
    /// an instance, the newest first (docs/plans/quest-variety.md 4.7).
    #[must_use]
    pub fn dungeons_entered(&self) -> Vec<&str> {
        self.visited(is_instance)
    }

    /// The rares and bosses that you defeated in a dungeon or a raid: the place of the foe
    /// is in a zone with the mark of an instance.
    #[must_use]
    pub fn bosses_defeated(&self) -> Vec<&str> {
        let in_instance = |foe: &&Entity| {
            let zone = self
                .world
                .location_of(foe.id)
                .map(|place| self.world.ancestry(place).last().copied().unwrap_or(place));
            zone.and_then(|zone| self.world.entity(zone))
                .is_some_and(is_instance)
        };
        self.linked_by_you(&[DEFEATED])
            .into_iter()
            .filter(in_instance)
            .map(|foe| foe.name.as_str())
            .collect()
    }

    /// The titles of the quests of the game that you took and did not turn in.
    #[must_use]
    pub fn game_quests_open(&self) -> Vec<&str> {
        let done = self.game_quests_done();
        self.game_quest_titles(GAME_QUEST_TAKEN)
            .into_iter()
            .filter(|title| !done.contains(title))
            .collect()
    }

    /// The titles of the quests of the game that you turned in.
    #[must_use]
    pub fn game_quests_done(&self) -> Vec<&str> {
        self.game_quest_titles(GAME_QUEST_DONE)
    }

    fn game_quest_titles(&self, fact: &str) -> Vec<&str> {
        self.linked_by_you(&[fact])
            .into_iter()
            .filter_map(|quest| title_of_game_quest(&quest.name))
            .collect()
    }

    /// The event that opened your level fact, when you have one.
    #[must_use]
    pub fn level_event(&self) -> Option<EventId> {
        self.world
            .entity(self.you)?
            .fact(LEVEL, None)
            .map(|fact| fact.opened)
    }

    #[must_use]
    pub fn has_seen(&self, npc: &str) -> bool {
        self.holds_about(SEEN, npc)
    }

    /// A hostile NPC or a beast has no task to give, and nothing to say (GAMEPLAY.md 3.4).
    #[must_use]
    pub fn is_hostile_or_animal(&self, npc: &str) -> bool {
        self.world
            .find(EntityType::Person, npc)
            .and_then(|id| self.world.entity(id))
            .is_some_and(|entity| holds_flag(entity, HOSTILE) || holds_flag(entity, ANIMAL))
    }

    #[must_use]
    pub fn is_dead(&self, npc: &str) -> bool {
        self.world
            .find(EntityType::Person, npc)
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
        let id = self.world.find(EntityType::Person, npc)?;
        self.zone_around(self.world.location_of(id)?)
    }

    /// The zone of a place: the place itself for a zone. A subzone name that repeats across
    /// zones gives one of them.
    #[must_use]
    pub fn zone_of_place(&self, place: &str) -> Option<&str> {
        self.zone_around(self.world.find(EntityType::Place, place)?)
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

    /// The first mount and the first epic mount count once in a life. Any other ride adds
    /// nothing, so a mount that is no first never becomes a thing of the world.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn ride_mount(&mut self, at: Tick, mount: &str, epic: bool) -> Result<(), Refusal> {
        let first = !self.you_hold(FIRST_MOUNT);
        let first_epic = epic && !self.you_hold(FIRST_EPIC_MOUNT);
        if !first && !first_epic {
            return Ok(());
        }
        let mount = self.find_or_create(at, EntityType::Thing, &mount_name(mount))?;
        if first {
            self.start(at, self.you, FIRST_MOUNT, mount)?;
        }
        if first_epic {
            self.start(at, self.you, FIRST_EPIC_MOUNT, mount)?;
        }
        Ok(())
    }

    /// The first epic item counts once in a life, and a big upgrade once for each slot
    /// and quality. The item keeps its quality, so a later upgrade can compare.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn put_on(&mut self, at: Tick, item: &Item<'_>, upgrade: bool) -> Result<(), Refusal> {
        let first_epic = item.quality.is_epic() && !self.you_hold(FIRST_EPIC_ITEM);
        let new_upgrade = upgrade && !self.has_upgraded(item.slot, self.quality_of(item));
        if !first_epic && !new_upgrade {
            return Ok(());
        }
        let thing = self.find_or_create(at, EntityType::Thing, &item_name(item.name))?;
        self.set_quality(at, thing, item.quality)?;
        if first_epic {
            self.start(at, self.you, FIRST_EPIC_ITEM, thing)?;
        }
        let held = self
            .world
            .entity(self.you)
            .and_then(|you| you.fact(UPGRADED, Some(thing)));
        if new_upgrade && held.is_none() {
            let slot = EventKind::FactStart {
                entity: self.you,
                name: UPGRADED.to_string(),
                value: Some(i64::from(item.slot)),
                linked_to: Some(thing),
            };
            self.propose(at, slot)?;
        }
        Ok(())
    }

    /// The quality that the world keeps for the item: the first one that came. An item
    /// of the game never changes its quality, so only a damaged line differs.
    fn quality_of(&self, item: &Item<'_>) -> i64 {
        self.world
            .find(EntityType::Thing, &item_name(item.name))
            .and_then(|thing| self.world.entity(thing)?.value(QUALITY))
            .unwrap_or_else(|| i64::from(item.quality.number()))
    }

    /// An earlier big upgrade of this slot with an item of this quality number.
    fn has_upgraded(&self, slot: u8, quality: i64) -> bool {
        let Some(you) = self.world.entity(self.you) else {
            return false;
        };
        you.facts_named(UPGRADED)
            .filter(|fact| fact.value == Some(i64::from(slot)))
            .filter_map(|fact| self.world.entity(fact.linked_to?))
            .any(|item| item.value(QUALITY) == Some(quality))
    }

    fn set_quality(&mut self, at: Tick, item: EntityId, quality: Quality) -> Result<(), Refusal> {
        let held = self.world.entity(item).and_then(|item| item.value(QUALITY));
        if held.is_some() {
            return Ok(());
        }
        let kind = EventKind::FactStart {
            entity: item,
            name: QUALITY.to_string(),
            value: Some(i64::from(quality.number())),
            linked_to: None,
        };
        self.propose(at, kind)
    }

    fn you_hold(&self, fact: &str) -> bool {
        self.world.entity(self.you).is_some_and(|you| you.has(fact))
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

    /// A flight over a zone and its subzone: you stand there, and visited nothing. So a
    /// subzone first seen from the air counts later, on foot (docs/plans/chapters.md 4).
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn fly_through(
        &mut self,
        at: Tick,
        zone: &str,
        subzone: Option<&str>,
    ) -> Result<(), Refusal> {
        let zone_id = self.place(at, zone, None)?;
        let Some(subzone) = subzone.filter(|name| *name != zone) else {
            return self.settle(at, self.you, zone_id);
        };
        let subzone_id = self.place(at, subzone, Some(zone_id))?;
        self.settle(at, self.you, subzone_id)
    }

    /// You won a battle in this battleground. Only the first win stays a fact.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn win_battleground(&mut self, at: Tick, zone: &str) -> Result<(), Refusal> {
        let zone = self.place(at, zone, None)?;
        self.start_once(at, self.you, BG_WON, zone)
    }

    /// A new rank in battle against players. A rank at or below the one held changes nothing, so the end of a
    /// season never lowers it.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn reach_pvp_rank(&mut self, at: Tick, rank: u8) -> Result<(), Refusal> {
        let rank = i64::from(rank);
        let held = self
            .world
            .entity(self.you)
            .and_then(|you| you.fact(PVP_RANK, None))
            .and_then(|fact| fact.value);
        let kind = match held {
            Some(held) if held >= rank => return Ok(()),
            Some(held) => EventKind::FactUpdate {
                entity: self.you,
                name: PVP_RANK.to_string(),
                linked_to: None,
                from: held,
                to: rank,
            },
            None => EventKind::FactStart {
                entity: self.you,
                name: PVP_RANK.to_string(),
                value: Some(rank),
                linked_to: None,
            },
        };
        self.propose(at, kind)
    }

    /// You rest at an inn or in a city, or you left it.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn rest(&mut self, at: Tick, resting: Resting) -> Result<(), Refusal> {
        let held = self
            .world
            .entity(self.you)
            .is_some_and(|you| holds_flag(you, RESTING));
        let kind = match (resting, held) {
            (Resting::Yes, false) => EventKind::FactStart {
                entity: self.you,
                name: RESTING.to_string(),
                value: None,
                linked_to: None,
            },
            (Resting::No, true) => EventKind::FactEnd {
                entity: self.you,
                name: RESTING.to_string(),
                linked_to: None,
            },
            _ => return Ok(()),
        };
        self.propose(at, kind)
    }

    /// The foe is a world boss. The mark stays.
    ///
    /// # Errors
    ///
    /// Returns the first refusal of Hourglass.
    pub fn mark_world_boss(&mut self, at: Tick, name: &str) -> Result<(), Refusal> {
        let foe = self.find_or_create(at, EntityType::Person, name)?;
        self.flag_once(at, foe, WORLD_BOSS)
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
            InstanceKind::Battleground => BATTLEGROUND,
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
        let Some(npc) = self.world.find(EntityType::Person, npc) else {
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
        let Some(npc) = self.world.find(EntityType::Person, npc) else {
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
        if let Some(id) = self.world.find(entity_type, name) {
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

    /// The names of what your facts of this name link to, oldest first.
    fn names_held(&self, fact: &str) -> Vec<&str> {
        let Some(you) = self.world.entity(self.you) else {
            return Vec::new();
        };
        you.facts_named(fact)
            .filter_map(|fact| fact.linked_to)
            .filter_map(|target| self.world.entity(target))
            .map(|entity| entity.name.as_str())
            .collect()
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
        let found = self.world.named(name).iter().copied().find(|id| {
            self.world.type_of(*id) == Some(EntityType::Place)
                && self.world.location_of(*id) == within
        });
        if let Some(place) = found {
            return Ok(place);
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

/// A place with the mark of a dungeon or a raid.
fn is_instance(place: &Entity) -> bool {
    holds_flag(place, DUNGEON) || holds_flag(place, RAID)
}

/// The creature types of the game that are animals. The addon sends the English name, in
/// lower case.
fn is_animal(creature: &str) -> bool {
    matches!(creature, "beast" | "critter")
}

/// Lean proves it (lean/README.md).
pub use timeways_rules::trust::next_trust;

const GAME_QUEST_PREFIX: &str = "game quest: ";

/// A quest of the game lives as a thing apart from the side quests and the titles.
#[must_use]
pub fn game_quest_name(title: &str) -> String {
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
