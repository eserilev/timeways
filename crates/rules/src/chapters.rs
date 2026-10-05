//! The chapters and tales of the Chronicle (docs/plans/chapters.md). The story program walks
//! the history into steps. This fold decides every chapter, every visit of an instance, and
//! the gain of each step. A model only writes their text. Lean proves the laws of section 8
//! (lean/Timeways/Chapters.lean).
//!
//! Aeneas translates no closure, no iterator adapter, and no map. So the fold walks by
//! index, keeps each record in a `Vec` at the index of its dense id, and copies a slot to a
//! local before it reads it (lean/README.md). A slot or an id out of range from a damaged
//! walk counts as nothing, so the fold never panics.

use crate::weights::{
    CAP_MAX, DEATHS_COUNTED, KeyKind, REVENGE, RULE_ONE, death_weight, limits, weight,
};

/// A gap this long between two events is a break.
pub const AWAY_SECONDS: u64 = 8 * 3600;

/// A step this long after you left an instance closes the visit. So a wipe and a corpse run
/// stay one run.
pub const RUN_GAP_SECONDS: u64 = 30 * 60;

/// What a step is about. `foe` names the foe of a kill or of a death, when one is known.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    pub id: usize,
    pub kind: KeyKind,
    pub foe: Option<usize>,
}

/// Where a step counts: the open world, or one instance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    World,
    Instance(usize),
}

/// A natural pause. The order is the order of the title when two wait at one cut.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Break {
    Return,
    NewZone,
    Level,
    Capital,
    Inn,
    Away,
}

/// One event of the history, as the fold reads it. `at` is in seconds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Play {
    pub key: Option<Key>,
    pub zone: usize,
    pub track: Track,
    /// A break that the walk saw: a tenth level, or leaving a capital or an inn.
    pub mark: Option<Break>,
    pub at: u64,
}

/// One step of the fold: an event, or the change to a new rule (section 13).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Play(Play),
    Rule(u8),
}

/// Why a chapter began. The story program makes its title from it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Opening {
    First,
    Break(Break),
    /// The chapter before reached the most weight: "Tirisfal Glades, continued".
    Max,
    Rule,
}

/// Why a chapter closed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Close {
    Break,
    Max,
    Rule,
}

/// A chapter. `first` is the index of its first step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Chapter {
    pub first: usize,
    pub weight: u16,
    pub opening: Opening,
    /// The zone of its title, once a step of the open world gave it weight.
    pub zone: Option<usize>,
    pub rule: u8,
}

/// A closed chapter holds its steps from `chapter.first` to `last`, both in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClosedChapter {
    pub chapter: Chapter,
    pub last: usize,
    pub close: Close,
}

/// What the fold knows of a key so far.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyRecord {
    pub seen: bool,
    /// Deaths with no known foe, up to `DEATHS_COUNTED`.
    pub deaths: u8,
    /// All that the key added so far. Never more than `CAP_MAX`.
    pub gain: u16,
}

/// What the fold knows of a foe so far. One record serves both tracks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FoeRecord {
    pub beaten: bool,
    /// Deaths to this foe, up to `DEATHS_COUNTED`.
    pub deaths: u8,
}

/// What the fold knows of a zone so far.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ZoneRecord {
    pub settled: bool,
    /// The index of the newest chapter in which the zone gained open world weight.
    pub last_chapter: usize,
}

/// The one tale of an instance. `first` is the index of its first step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tale {
    pub instance: usize,
    pub first: usize,
    /// The sum of the gains of its closed visits.
    pub weight: u32,
    /// The number of its closed visits.
    pub runs: u32,
}

/// One run of an instance: its steps from `first` to `last`, both in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Visit {
    /// The index of its tale.
    pub tale: usize,
    pub first: usize,
    pub last: usize,
    /// The time of its last step.
    pub left_at: u64,
    pub gain: u32,
}

/// What one step gained, and where.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Gain {
    pub amount: u16,
    pub track: Track,
    /// The step was the first kill of a foe that had killed you.
    pub revenge: bool,
}

/// The state of the fold. The story program keeps it in memory and folds each new line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fold {
    pub keys: Vec<KeyRecord>,
    pub foes: Vec<FoeRecord>,
    pub zones: Vec<ZoneRecord>,
    pub closed: Vec<ClosedChapter>,
    pub open: Chapter,
    pub pending: Option<Break>,
    pub tales: Vec<Tale>,
    /// The closed visits, in the order they closed.
    pub visits: Vec<Visit>,
    pub visit: Option<Visit>,
    pub last_at: Option<u64>,
    /// The gain of each step so far, by index. Its length is the number of steps.
    pub gains: Vec<Gain>,
}

const UNSEEN: KeyRecord = KeyRecord {
    seen: false,
    deaths: 0,
    gain: 0,
};

const UNBEATEN: FoeRecord = FoeRecord {
    beaten: false,
    deaths: 0,
};

const UNSETTLED: ZoneRecord = ZoneRecord {
    settled: false,
    last_chapter: 0,
};

/// The fold before any step.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn start() -> Fold {
    Fold {
        keys: Vec::new(),
        foes: Vec::new(),
        zones: Vec::new(),
        closed: Vec::new(),
        open: Chapter {
            first: 0,
            weight: 0,
            opening: Opening::First,
            zone: None,
            rule: RULE_ONE,
        },
        pending: None,
        tales: Vec::new(),
        visits: Vec::new(),
        visit: None,
        last_at: None,
        gains: Vec::new(),
    }
}

/// The fold of a whole log.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn chapters(steps: &[Step]) -> Fold {
    let mut fold = start();
    advance(&mut fold, steps);
    fold
}

/// Folds more steps onto a fold. An index loop.
#[cfg_attr(charon, verify::start_from)]
pub fn advance(fold: &mut Fold, steps: &[Step]) {
    let mut index = 0;
    while index < steps.len() {
        apply(fold, steps[index]);
        index += 1;
    }
}

fn apply(fold: &mut Fold, step: Step) {
    match step {
        Step::Rule(rule) => apply_rule(fold, rule),
        Step::Play(play) => apply_play(fold, play),
    }
}

/// A rule change closes the open chapter, even below the least weight. The rule step opens
/// the next one. All else carries over.
fn apply_rule(fold: &mut Fold, rule: u8) {
    let here = fold.gains.len();
    if fold.open.first < here {
        close_chapter(fold, here - 1, Close::Rule);
        fold.open = Chapter {
            first: here,
            weight: 0,
            opening: Opening::Rule,
            zone: None,
            rule,
        };
    } else {
        fold.open.rule = rule;
    }
    fold.pending = None;
    fold.gains.push(Gain {
        amount: 0,
        track: Track::World,
        revenge: false,
    });
}

fn apply_play(fold: &mut Fold, play: Play) {
    let here = fold.gains.len();
    if let Some(last) = fold.last_at
        && play.at >= last.saturating_add(AWAY_SECONDS)
    {
        wait_for_cut(fold, Break::Away);
    }
    if let Some(mark) = play.mark {
        wait_for_cut(fold, mark);
    }
    match play.track {
        Track::World => leave_instance(fold, play.at),
        Track::Instance(instance) => enter_instance(fold, instance, here, play.at),
    }
    let (amount, revenge) = gain_of(fold, play.key);
    match play.track {
        Track::World => add_to_chapter(fold, play.zone, amount, here),
        Track::Instance(_) => add_to_visit(fold, amount),
    }
    fold.gains.push(Gain {
        amount,
        track: play.track,
        revenge,
    });
    fold.last_at = Some(play.at);
}

/// The break of the first place in the order of titles waits.
fn wait_for_cut(fold: &mut Fold, new: Break) {
    let pending = fold.pending;
    let first = match pending {
        Some(old) => {
            if title_rank(old) <= title_rank(new) {
                old
            } else {
                new
            }
        }
        None => new,
    };
    fold.pending = Some(first);
}

fn title_rank(cut: Break) -> u8 {
    match cut {
        Break::Return => 0,
        Break::NewZone => 1,
        Break::Level => 2,
        Break::Capital => 3,
        Break::Inn => 4,
        Break::Away => 5,
    }
}

fn close_chapter(fold: &mut Fold, last: usize, close: Close) {
    let chapter = fold.open;
    fold.closed.push(ClosedChapter {
        chapter,
        last,
        close,
    });
}

/// A step of the open world with gain: a cut at a waiting break, the weight, and a close
/// at the most weight.
fn add_to_chapter(fold: &mut Fold, zone: usize, amount: u16, here: usize) {
    if amount == 0 {
        return;
    }
    if let Some(cut) = zone_break(fold, zone) {
        wait_for_cut(fold, cut);
    }
    let rule = fold.open.rule;
    let bounds = limits(rule);
    let pending = fold.pending;
    if let Some(cut) = pending {
        fold.pending = None;
        if fold.open.weight >= bounds.min && fold.open.first < here {
            close_chapter(fold, here - 1, Close::Break);
            fold.open = Chapter {
                first: here,
                weight: 0,
                opening: Opening::Break(cut),
                zone: Some(zone),
                rule,
            };
        }
    }
    fold.open.weight = fold.open.weight.saturating_add(amount);
    if fold.open.zone.is_none() {
        fold.open.zone = Some(zone);
    }
    settle(fold, zone);
    if fold.open.weight >= bounds.max {
        close_chapter(fold, here, Close::Max);
        fold.open = Chapter {
            first: here + 1,
            weight: 0,
            opening: Opening::Max,
            zone: Some(zone),
            rule,
        };
    }
}

/// Settling in a new zone, or a return to a zone that had no weight in the open chapter
/// and the one before it.
fn zone_break(fold: &mut Fold, zone: usize) -> Option<Break> {
    if !has_slot(fold.zones.len(), zone) {
        return None;
    }
    if zone == fold.zones.len() {
        fold.zones.push(UNSETTLED);
    }
    let record = fold.zones[zone];
    if !record.settled {
        return Some(Break::NewZone);
    }
    if record.last_chapter.saturating_add(1) < fold.closed.len() {
        return Some(Break::Return);
    }
    None
}

fn settle(fold: &mut Fold, zone: usize) {
    if zone < fold.zones.len() {
        fold.zones[zone] = ZoneRecord {
            settled: true,
            last_chapter: fold.closed.len(),
        };
    }
}

/// True when `id` names a slot of a `Vec` of `length`, or the next one. A `Vec` never
/// holds `usize::MAX` items, and the check lets the proof see that a push never fails.
fn has_slot(length: usize, id: usize) -> bool {
    id < length || (id == length && length < usize::MAX)
}

/// The gain of a key, and whether it was revenge. It updates the records of the key and
/// its foe. An id out of range gains nothing.
fn gain_of(fold: &mut Fold, key: Option<Key>) -> (u16, bool) {
    let Some(key) = key else {
        return (0, false);
    };
    if !has_slot(fold.keys.len(), key.id) {
        return (0, false);
    }
    if key.id == fold.keys.len() {
        fold.keys.push(UNSEEN);
    }
    let mut record = fold.keys[key.id];
    let rule = fold.open.rule;
    let (raw, revenge) = match key.kind {
        KeyKind::Death => (death_gain(fold, &mut record, key.foe, rule), false),
        KeyKind::Kill | KeyKind::RaidKill => kill_gain(fold, record, key, rule),
        _ => (first_time_gain(record, key.kind, rule), false),
    };
    let room = CAP_MAX.saturating_sub(record.gain);
    let amount = if raw < room { raw } else { room };
    record.seen = true;
    record.gain = record.gain.saturating_add(amount);
    fold.keys[key.id] = record;
    (amount, revenge)
}

fn first_time_gain(record: KeyRecord, kind: KeyKind, rule: u8) -> u16 {
    if record.seen { 0 } else { weight(rule, kind) }
}

/// A first kill weighs its weight, and its revenge when the foe killed you before. Any kill
/// beats the foe.
fn kill_gain(fold: &mut Fold, record: KeyRecord, key: Key, rule: u8) -> (u16, bool) {
    let base = first_time_gain(record, key.kind, rule);
    let Some(foe) = key.foe else {
        return (base, false);
    };
    if !has_slot(fold.foes.len(), foe) {
        return (base, false);
    }
    if foe == fold.foes.len() {
        fold.foes.push(UNBEATEN);
    }
    let before = fold.foes[foe];
    fold.foes[foe] = FoeRecord {
        beaten: true,
        deaths: before.deaths,
    };
    if record.seen || before.beaten || before.deaths == 0 {
        return (base, false);
    }
    (base + REVENGE, true)
}

/// A death weighs less each time, and nothing once you beat its foe. A death with no known
/// foe counts on its key. A foe out of range weighs nothing.
fn death_gain(fold: &mut Fold, record: &mut KeyRecord, foe: Option<usize>, rule: u8) -> u16 {
    let Some(foe) = foe else {
        let gain = death_weight(rule, record.deaths);
        record.deaths = one_more_death(record.deaths);
        return gain;
    };
    if !has_slot(fold.foes.len(), foe) {
        return 0;
    }
    if foe == fold.foes.len() {
        fold.foes.push(UNBEATEN);
    }
    let before = fold.foes[foe];
    fold.foes[foe] = FoeRecord {
        beaten: before.beaten,
        deaths: one_more_death(before.deaths),
    };
    if before.beaten {
        0
    } else {
        death_weight(rule, before.deaths)
    }
}

fn one_more_death(deaths: u8) -> u8 {
    if deaths < DEATHS_COUNTED {
        deaths + 1
    } else {
        deaths
    }
}

/// A step of the open world closes the open visit once you were away from it long enough.
fn leave_instance(fold: &mut Fold, at: u64) {
    if let Some(visit) = fold.visit
        && at >= visit.left_at.saturating_add(RUN_GAP_SECONDS)
    {
        close_visit(fold, visit);
    }
}

/// A step in an instance resumes its open visit, or closes the open visit and opens a new
/// one. The first step of an instance opens its tale.
fn enter_instance(fold: &mut Fold, instance: usize, here: usize, at: u64) {
    let tale = tale_of(fold, instance, here);
    if let Some(visit) = fold.visit {
        let resumed = visit.tale == tale && at < visit.left_at.saturating_add(RUN_GAP_SECONDS);
        if resumed {
            fold.visit = Some(Visit {
                last: here,
                left_at: at,
                ..visit
            });
            return;
        }
        close_visit(fold, visit);
    }
    fold.visit = Some(Visit {
        tale,
        first: here,
        last: here,
        left_at: at,
        gain: 0,
    });
}

/// The index of the tale of an instance. It opens the tale when the instance has none. An
/// index loop: an instance id needs no density, and a life holds few instances.
fn tale_of(fold: &mut Fold, instance: usize, here: usize) -> usize {
    let mut index = 0;
    while index < fold.tales.len() {
        if fold.tales[index].instance == instance {
            return index;
        }
        index += 1;
    }
    fold.tales.push(Tale {
        instance,
        first: here,
        weight: 0,
        runs: 0,
    });
    index
}

/// The visit joins its tale: the tale gains its runs and its weight.
fn close_visit(fold: &mut Fold, visit: Visit) {
    fold.visits.push(visit);
    fold.visit = None;
    if visit.tale < fold.tales.len() {
        let tale = fold.tales[visit.tale];
        fold.tales[visit.tale] = Tale {
            weight: tale.weight.saturating_add(visit.gain),
            runs: tale.runs.saturating_add(1),
            ..tale
        };
    }
}

// Aeneas translates no `From`, so the gain widens with `as`.
#[allow(clippy::cast_lossless)]
fn add_to_visit(fold: &mut Fold, amount: u16) {
    if let Some(visit) = fold.visit {
        fold.visit = Some(Visit {
            gain: visit.gain.saturating_add(amount as u32),
            ..visit
        });
    }
}

#[cfg(test)]
mod tests;
