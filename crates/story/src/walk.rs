//! The walk from the history of a world to the steps of the chapter fold
//! (docs/plans/chapters.md 3). Each event becomes one step, and a rule change puts a rule
//! step before its first event. The walk reads only the events up to the step, so the steps
//! of a prefix are a prefix of the steps. It gives each key, foe, and zone a dense id in
//! the order of first use.

use crate::places::is_capital;
use crate::vocabulary::{
    BATTLEGROUND, BG_WON, CLASS_QUEST, DEATHS, DEFEATED, DUNGEON, FIRST_EPIC_ITEM,
    FIRST_EPIC_MOUNT, FIRST_MOUNT, GAME_QUEST_DONE, LEVEL, MARKED_BY, MET, PVP_RANK, QUALITY,
    QUEST_DONE, RAID, RESTING, TITLE, UPGRADED, VISITED, WORLD_BOSS,
};
use hourglass::{EntityId, EntityType, Event, EventId, EventKind, LOCATED_IN};
use std::collections::{HashMap, HashSet};
use timeways_rules::chapters::{Break, Key, Play, Step, Track};
use timeways_rules::weights::{KeyKind, RULE_ONE};

/// Every tenth level is a break.
pub const LEVEL_STEP: i64 = 10;

/// The zone of a step before the world knows where you stand. The fold reads no zone there.
pub const NO_ZONE: usize = usize::MAX;

/// One row of `chapter_rules`: from this event on, this rule cuts the chapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuleRow {
    pub rule: u8,
    pub from: EventId,
}

/// What a key is about. Each one gets a dense id at its first use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum KeyName {
    GameQuest(EntityId),
    SideQuest(EntityId),
    Subzone(EntityId),
    Level(i64),
    Talk(EntityId),
    Kill(EntityId),
    DeathBy(EntityId),
    DeathIn(EntityId),
    NoZoneDeath,
    Mark(EntityId),
    Title(EntityId),
    Mount,
    EpicMount,
    EpicItem,
    Upgrade { slot: i64, quality: i64 },
    Instance(EntityId),
    BgWin(EntityId),
    PvpRank(i64),
}

/// The state of the walk. It keeps what it saw of the world so far.
#[derive(Clone, Debug)]
pub struct Walk {
    you: EntityId,
    rules: Vec<RuleRow>,
    next_rule: usize,
    rule: u8,
    keys: HashMap<KeyName, usize>,
    foes: HashMap<EntityId, usize>,
    zones: HashMap<EntityId, usize>,
    /// The zone of each dense zone id.
    zone_ids: Vec<EntityId>,
    places: HashSet<EntityId>,
    /// The places with the name of a capital. Only a zone of them is a capital.
    capital_names: HashSet<EntityId>,
    parents: HashMap<EntityId, EntityId>,
    instances: HashSet<EntityId>,
    raids: HashSet<EntityId>,
    world_bosses: HashSet<EntityId>,
    class_quests: HashSet<EntityId>,
    qualities: HashMap<EntityId, i64>,
    here: Option<EntityId>,
    /// A step with a key happened in the zone since you came to it.
    stay_has_key: bool,
    /// The foe that killed you, from the event just before the count of deaths.
    killer: Option<EntityId>,
}

/// The steps of one event, each with the event that it stands for.
pub type Walked = Vec<(Step, EventId)>;

impl Walk {
    /// A walk for the character `you`. `rules` are the rows of `chapter_rules`, oldest first.
    #[must_use]
    pub fn new(you: EntityId, mut rules: Vec<RuleRow>) -> Walk {
        rules.sort_by_key(|row| row.from);
        Walk {
            you,
            rules,
            next_rule: 0,
            rule: RULE_ONE,
            keys: HashMap::new(),
            foes: HashMap::new(),
            zones: HashMap::new(),
            zone_ids: Vec::new(),
            places: HashSet::new(),
            capital_names: HashSet::new(),
            parents: HashMap::new(),
            instances: HashSet::new(),
            raids: HashSet::new(),
            world_bosses: HashSet::new(),
            class_quests: HashSet::new(),
            qualities: HashMap::new(),
            here: None,
            stay_has_key: false,
            killer: None,
        }
    }

    /// The zone of a dense zone id.
    #[must_use]
    pub fn zone_of_id(&self, id: usize) -> Option<EntityId> {
        self.zone_ids.get(id).copied()
    }

    /// The steps of one event: a rule step when a new rule starts here, then the event.
    pub fn step(&mut self, event: &Event) -> Walked {
        let mut steps = Vec::new();
        while let Some(row) = self.rules.get(self.next_rule).copied() {
            if row.from > event.id {
                break;
            }
            self.next_rule += 1;
            if row.rule != self.rule {
                self.rule = row.rule;
                steps.push((Step::Rule(row.rule), event.id));
            }
        }
        steps.push((Step::Play(self.play(event)), event.id));
        steps
    }

    fn play(&mut self, event: &Event) -> Play {
        let mut mark = None;
        self.learn(&event.kind, &mut mark);
        let keyed = self.key_of(&event.kind);
        let counts = keyed.map_or(Counts::Here, |(_, counts)| counts);
        let key = keyed.map(|(key, _)| key);
        let zone = self
            .place_of(&event.kind)
            .map_or_else(|| self.zone(), |place| Some(self.zone_around(place)));
        let track = match zone {
            Some(zone) if counts == Counts::Here && self.instances.contains(&zone) => {
                Track::Instance(self.zone_id(zone))
            }
            _ => Track::World,
        };
        if key.is_some() {
            self.stay_has_key = true;
        }
        Play {
            key,
            zone: zone.map_or(NO_ZONE, |zone| self.zone_id(zone)),
            track,
            mark,
            at: event.tick.0,
        }
    }

    /// The place that a move is about. The events of a move come before you stand in the
    /// new place, so a move out of an instance is a step of the place where you go: it
    /// opens no run of the instance that you leave.
    fn place_of(&self, kind: &EventKind) -> Option<EntityId> {
        match kind {
            EventKind::EntityCreated {
                id,
                entity_type: EntityType::Place,
                ..
            } => Some(*id),
            EventKind::FactStart {
                entity,
                name,
                linked_to: Some(place),
                ..
            } if *entity == self.you && name == VISITED => Some(*place),
            EventKind::FactStart { entity, name, .. }
                if name == LOCATED_IN && self.places.contains(entity) =>
            {
                Some(*entity)
            }
            _ => None,
        }
    }

    /// What the walk keeps of the world: places, where you stand, instances, and the
    /// marks of a capital that you leave and of a tenth level.
    fn learn(&mut self, kind: &EventKind, mark: &mut Option<Break>) {
        match kind {
            EventKind::EntityCreated {
                id,
                entity_type: EntityType::Place,
                name,
            } => {
                self.places.insert(*id);
                if is_capital(name) {
                    self.capital_names.insert(*id);
                }
            }
            EventKind::FactStart {
                entity,
                name,
                linked_to: Some(place),
                ..
            } if name == LOCATED_IN => self.locate(*entity, *place, mark),
            EventKind::FactStart { entity, name, .. } if is_instance_mark(name) => {
                self.instances.insert(*entity);
                if name == RAID {
                    self.raids.insert(*entity);
                }
            }
            EventKind::FactStart { entity, name, .. } if name == WORLD_BOSS => {
                self.world_bosses.insert(*entity);
            }
            EventKind::FactEnd { entity, name, .. } if *entity == self.you && name == RESTING => {
                *mark = Some(Break::Inn);
            }
            EventKind::FactStart { entity, name, .. } if name == CLASS_QUEST => {
                self.class_quests.insert(*entity);
            }
            EventKind::FactStart {
                entity,
                name,
                value: Some(quality),
                ..
            } if name == QUALITY => {
                self.qualities.insert(*entity, *quality);
            }
            EventKind::FactUpdate {
                entity, name, to, ..
            } if name == QUALITY => {
                self.qualities.insert(*entity, *to);
            }
            EventKind::FactUpdate {
                entity, name, to, ..
            } if *entity == self.you && name == LEVEL && to % LEVEL_STEP == 0 => {
                *mark = Some(Break::Level);
            }
            _ => {}
        }
    }

    fn locate(&mut self, entity: EntityId, place: EntityId, mark: &mut Option<Break>) {
        if entity != self.you {
            self.parents.insert(entity, place);
            return;
        }
        let before = self.zone();
        self.here = Some(place);
        let after = self.zone();
        if before == after {
            return;
        }
        self.stay_has_key = false;
        if before.is_some_and(|zone| self.is_capital(zone)) {
            *mark = Some(Break::Capital);
        }
    }

    fn is_capital(&self, zone: EntityId) -> bool {
        self.capital_names.contains(&zone)
    }

    /// The zone where you stand: the place around the place where you stand.
    fn zone(&self) -> Option<EntityId> {
        self.here.map(|place| self.zone_around(place))
    }

    fn zone_around(&self, place: EntityId) -> EntityId {
        let mut zone = place;
        // A cycle of places cannot come from the world, but the walk stops at any length.
        for _ in 0..self.parents.len() {
            match self.parents.get(&zone) {
                Some(parent) if self.places.contains(parent) => zone = *parent,
                _ => break,
            }
        }
        zone
    }

    fn zone_id(&mut self, zone: EntityId) -> usize {
        let next = self.zone_ids.len();
        *self.zones.entry(zone).or_insert_with(|| {
            self.zone_ids.push(zone);
            next
        })
    }

    fn key_id(&mut self, name: KeyName) -> usize {
        let next = self.keys.len();
        *self.keys.entry(name).or_insert(next)
    }

    fn foe_id(&mut self, foe: EntityId) -> usize {
        let next = self.foes.len();
        *self.foes.entry(foe).or_insert(next)
    }

    fn key(&mut self, name: KeyName, kind: KeyKind, foe: Option<usize>) -> Key {
        Key {
            id: self.key_id(name),
            kind,
            foe,
        }
    }

    /// The key of an event, and the track where it counts.
    fn key_of(&mut self, kind: &EventKind) -> Option<(Key, Counts)> {
        match kind {
            EventKind::FactStart {
                entity,
                name,
                linked_to,
                value,
            } => self.started_key(*entity, name, *linked_to, *value),
            EventKind::FactUpdate {
                entity,
                name,
                linked_to,
                to,
                ..
            } => self.updated_key(*entity, name, *linked_to, *to),
            _ => None,
        }
    }

    fn started_key(
        &mut self,
        entity: EntityId,
        name: &str,
        linked_to: Option<EntityId>,
        value: Option<i64>,
    ) -> Option<(Key, Counts)> {
        if is_instance_mark(name) {
            let kind = match name {
                RAID => KeyKind::Raid,
                BATTLEGROUND => KeyKind::Battleground,
                _ => KeyKind::Dungeon,
            };
            return Some((
                self.key(KeyName::Instance(entity), kind, None),
                Counts::Here,
            ));
        }
        if entity != self.you {
            self.note_killer(entity, name, linked_to);
            return None;
        }
        let key = match (name, linked_to) {
            (DEATHS, _) => self.death_key(),
            (DEFEATED, Some(foe)) => self.kill_key(foe),
            (FIRST_MOUNT, _) => {
                return Some((
                    self.key(KeyName::Mount, KeyKind::Mount, None),
                    Counts::InTheWorld,
                ));
            }
            (FIRST_EPIC_MOUNT, _) => {
                let key = self.key(KeyName::EpicMount, KeyKind::EpicMount, None);
                return Some((key, Counts::InTheWorld));
            }
            (FIRST_EPIC_ITEM, _) => self.key(KeyName::EpicItem, KeyKind::EpicItem, None),
            (UPGRADED, Some(item)) => self.upgrade_key(item, value?),
            (BG_WON, Some(zone)) => self.key(KeyName::BgWin(zone), KeyKind::BgWin, None),
            (_, Some(thing)) => self.thing_key(name, thing)?,
            _ => return None,
        };
        Some((key, Counts::Here))
    }

    fn updated_key(
        &mut self,
        entity: EntityId,
        name: &str,
        linked_to: Option<EntityId>,
        to: i64,
    ) -> Option<(Key, Counts)> {
        if entity != self.you {
            self.note_killer(entity, name, linked_to);
            return None;
        }
        let key = match (name, linked_to) {
            (LEVEL, _) => {
                return Some((
                    self.key(KeyName::Level(to), KeyKind::Level, None),
                    Counts::InTheWorld,
                ));
            }
            (DEATHS, _) => self.death_key(),
            (DEFEATED, Some(foe)) => self.kill_key(foe),
            (UPGRADED, Some(item)) => self.upgrade_key(item, to),
            (PVP_RANK, _) => self.key(KeyName::PvpRank(to), KeyKind::PvpRank, None),
            _ => return None,
        };
        Some((key, Counts::Here))
    }

    /// `die` counts a kill of you by the killer just before your count of deaths.
    fn note_killer(&mut self, entity: EntityId, name: &str, linked_to: Option<EntityId>) {
        if name == DEFEATED && linked_to == Some(self.you) {
            self.killer = Some(entity);
        }
    }

    /// A fact of yours about a thing or a place, made once in a life.
    fn thing_key(&mut self, name: &str, thing: EntityId) -> Option<Key> {
        let (key_name, kind) = match name {
            GAME_QUEST_DONE if self.class_quests.contains(&thing) => {
                (KeyName::GameQuest(thing), KeyKind::ClassQuest)
            }
            GAME_QUEST_DONE => (KeyName::GameQuest(thing), KeyKind::GameQuest),
            QUEST_DONE => (KeyName::SideQuest(thing), KeyKind::SideQuest),
            MET => (KeyName::Talk(thing), KeyKind::Talk),
            MARKED_BY => (KeyName::Mark(thing), KeyKind::Mark),
            TITLE => (KeyName::Title(thing), KeyKind::Title),
            VISITED => return self.subzone_key(thing),
            _ => return None,
        };
        Some(self.key(key_name, kind, None))
    }

    /// A subzone counts only on foot: after another key in its zone, in the same stay. So
    /// a subzone first seen from the air never counts.
    fn subzone_key(&mut self, place: EntityId) -> Option<Key> {
        let zone = self.parents.get(&place).copied()?;
        let in_this_stay = self.stay_has_key && self.zone() == Some(self.zone_around(zone));
        if !in_this_stay {
            return None;
        }
        Some(self.key(KeyName::Subzone(place), KeyKind::Subzone, None))
    }

    fn kill_key(&mut self, foe: EntityId) -> Key {
        let in_raid = self.zone().is_some_and(|zone| self.raids.contains(&zone));
        let kind = if in_raid || self.world_bosses.contains(&foe) {
            KeyKind::RaidKill
        } else {
            KeyKind::Kill
        };
        let id = self.foe_id(foe);
        self.key(KeyName::Kill(foe), kind, Some(id))
    }

    fn death_key(&mut self) -> Key {
        if let Some(foe) = self.killer.take() {
            let id = self.foe_id(foe);
            return self.key(KeyName::DeathBy(foe), KeyKind::Death, Some(id));
        }
        let name = self.zone().map_or(KeyName::NoZoneDeath, KeyName::DeathIn);
        self.key(name, KeyKind::Death, None)
    }

    /// One key for each slot and quality.
    fn upgrade_key(&mut self, item: EntityId, slot: i64) -> Key {
        let quality = self.qualities.get(&item).copied().unwrap_or_default();
        self.key(KeyName::Upgrade { slot, quality }, KeyKind::Upgrade, None)
    }
}

/// The fact that marks a zone as an instance of the game.
fn is_instance_mark(name: &str) -> bool {
    name == DUNGEON || name == RAID || name == BATTLEGROUND
}

/// Where a key counts. A level and a mount belong to the leveling story, so they count in
/// the open world, also inside an instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Counts {
    Here,
    InTheWorld,
}
