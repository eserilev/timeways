use super::*;

const MINUTE: u64 = 60;

/// Builds a log of steps with dense ids, one minute apart unless told otherwise.
struct Log {
    steps: Vec<Step>,
    at: u64,
    next_key: usize,
}

impl Log {
    fn new() -> Log {
        Log {
            steps: Vec::new(),
            at: 1_000_000,
            next_key: 0,
        }
    }

    fn new_key(&mut self, kind: KeyKind, foe: Option<usize>) -> Key {
        let key = Key {
            id: self.next_key,
            kind,
            foe,
        };
        self.next_key += 1;
        key
    }

    fn play(&mut self, key: Option<Key>, zone: usize, track: Track, mark: Option<Break>) {
        self.at += MINUTE;
        self.steps.push(Step::Play(Play {
            key,
            zone,
            track,
            mark,
            at: self.at,
        }));
    }

    /// A new quest of the game in the open world: weight 1.
    fn quest(&mut self, zone: usize) {
        let key = self.new_key(KeyKind::GameQuest, None);
        self.play(Some(key), zone, Track::World, None);
    }

    /// New quests in the open world up to this much weight.
    fn quests(&mut self, zone: usize, count: usize) {
        for _ in 0..count {
            self.quest(zone);
        }
    }

    fn again(&mut self, key: Key, zone: usize, track: Track) {
        self.play(Some(key), zone, track, None);
    }

    fn fly_over(&mut self, zone: usize) {
        self.play(None, zone, Track::World, None);
    }

    fn wait(&mut self, seconds: u64) {
        self.at += seconds;
    }

    fn fold(&self) -> Fold {
        chapters(&self.steps)
    }
}

const ELWYNN: usize = 0;
const WESTFALL: usize = 1;
const DUSKWOOD: usize = 2;
const STORMWIND: usize = 3;
const DEADMINES: usize = 4;
const MOLTEN_CORE: usize = 5;

fn gains(fold: &Fold) -> Vec<u16> {
    fold.gains.iter().map(|gain| gain.amount).collect()
}

#[test]
fn an_empty_log_has_no_closed_chapter_and_no_tale() {
    let fold = chapters(&[]);

    assert!(fold.closed.is_empty());
    assert!(fold.tales.is_empty());
    assert_eq!(fold.open.first, 0);
}

#[test]
fn a_flight_across_four_zones_makes_no_chapter() {
    let mut log = Log::new();
    log.quests(ELWYNN, 20);

    for zone in [WESTFALL, DUSKWOOD, STORMWIND, ELWYNN] {
        log.fly_over(zone);
    }
    let fold = log.fold();

    assert!(fold.closed.is_empty());
    assert_eq!(fold.open.weight, 20);
    assert_eq!(fold.pending, None);
}

#[test]
fn a_chapter_closes_at_a_new_zone_once_it_has_the_least_weight() {
    let mut log = Log::new();
    log.quests(ELWYNN, 15);

    log.quest(WESTFALL);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 1);
    assert_eq!(fold.closed[0].chapter.weight, 15);
    assert_eq!(fold.closed[0].last, 14);
    assert_eq!(fold.closed[0].close, Close::Break);
    assert_eq!(fold.open.first, 15);
    assert_eq!(fold.open.opening, Opening::Break(Break::NewZone));
    assert_eq!(fold.open.zone, Some(WESTFALL));
}

#[test]
fn a_break_below_the_least_weight_is_spent() {
    let mut log = Log::new();
    log.quests(ELWYNN, 14);

    log.quest(WESTFALL);
    log.quest(ELWYNN);
    let fold = log.fold();

    assert!(fold.closed.is_empty());
    assert_eq!(fold.open.weight, 16);
    assert_eq!(fold.pending, None);
}

#[test]
fn a_long_grind_in_one_zone_still_breaks_into_chapters() {
    let mut log = Log::new();

    log.quests(ELWYNN, 100);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 2);
    for closed in &fold.closed {
        assert_eq!(closed.chapter.weight, 40);
        assert_eq!(closed.close, Close::Max);
    }
    assert_eq!(fold.closed[1].chapter.opening, Opening::Max);
    assert_eq!(fold.closed[1].chapter.zone, Some(ELWYNN));
    assert_eq!(fold.open.weight, 20);
}

#[test]
fn the_same_thing_done_again_never_adds_weight() {
    let mut log = Log::new();
    let hogger = log.new_key(KeyKind::Kill, Some(0));
    log.again(hogger, ELWYNN, Track::World);

    for _ in 0..100 {
        log.again(hogger, ELWYNN, Track::World);
    }
    let fold = log.fold();

    assert_eq!(fold.open.weight, 3);
    assert_eq!(gains(&fold).iter().filter(|gain| **gain > 0).count(), 1);
}

#[test]
fn a_tenth_level_waits_for_the_next_step_with_weight() {
    let mut log = Log::new();
    log.quests(ELWYNN, 20);
    let level = log.new_key(KeyKind::Level, None);
    log.play(None, ELWYNN, Track::World, Some(Break::Level));
    log.fly_over(ELWYNN);

    log.again(level, ELWYNN, Track::World);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 1);
    assert_eq!(fold.closed[0].last, 21);
    assert_eq!(fold.open.opening, Opening::Break(Break::Level));
}

#[test]
fn eight_hours_away_is_a_break() {
    let mut log = Log::new();
    log.quests(ELWYNN, 20);

    log.wait(AWAY_SECONDS);
    log.quest(ELWYNN);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 1);
    assert_eq!(fold.open.opening, Opening::Break(Break::Away));
}

#[test]
fn a_return_needs_no_weight_in_the_open_chapter_and_the_one_before() {
    let mut log = Log::new();
    log.quests(ELWYNN, 15);
    log.quests(WESTFALL, 15);
    log.quests(DUSKWOOD, 15);

    log.quest(ELWYNN);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 3);
    assert_eq!(fold.open.opening, Opening::Break(Break::Return));
    assert_eq!(fold.open.zone, Some(ELWYNN));
}

#[test]
fn a_zone_of_the_chapter_before_is_no_return() {
    let mut log = Log::new();
    log.quests(ELWYNN, 15);
    log.quests(WESTFALL, 15);

    log.quest(ELWYNN);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 1);
    assert_eq!(fold.open.weight, 16);
}

#[test]
fn a_return_comes_first_in_the_title_when_two_breaks_wait() {
    let mut log = Log::new();
    log.quests(ELWYNN, 15);
    log.quests(WESTFALL, 15);
    log.quests(DUSKWOOD, 15);
    log.play(None, STORMWIND, Track::World, Some(Break::Capital));

    log.quest(ELWYNN);
    let fold = log.fold();

    assert_eq!(fold.open.opening, Opening::Break(Break::Return));
}

#[test]
fn deaths_to_one_foe_weigh_two_then_one_then_nothing() {
    let mut log = Log::new();
    let death = log.new_key(KeyKind::Death, Some(0));

    for _ in 0..100 {
        log.again(death, WESTFALL, Track::World);
    }
    let fold = log.fold();

    assert_eq!(gains(&fold)[..4], [2, 1, 0, 0]);
    assert_eq!(fold.open.weight, 3);
}

#[test]
fn deaths_with_no_known_killer_weigh_two_then_one_then_nothing() {
    let mut log = Log::new();
    let death = log.new_key(KeyKind::Death, None);

    for _ in 0..4 {
        log.again(death, WESTFALL, Track::World);
    }
    let fold = log.fold();

    assert_eq!(gains(&fold), [2, 1, 0, 0]);
}

#[test]
fn a_death_to_a_beaten_foe_weighs_nothing() {
    let mut log = Log::new();
    let kill = log.new_key(KeyKind::Kill, Some(0));
    let death = log.new_key(KeyKind::Death, Some(0));
    log.again(kill, WESTFALL, Track::World);

    log.again(death, WESTFALL, Track::World);
    let fold = log.fold();

    assert_eq!(gains(&fold), [3, 0]);
}

#[test]
fn revenge_outside_an_instance_counts_for_the_chapter() {
    let mut log = Log::new();
    let entry = log.new_key(KeyKind::Dungeon, None);
    let death = log.new_key(KeyKind::Death, Some(0));
    let kill = log.new_key(KeyKind::Kill, Some(0));
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));
    log.again(death, DEADMINES, Track::Instance(DEADMINES));

    log.again(kill, WESTFALL, Track::World);
    let fold = log.fold();

    assert_eq!(gains(&fold), [3, 2, 5]);
    assert!(fold.gains[2].revenge);
    assert_eq!(fold.open.weight, 5);
}

#[test]
fn a_kill_with_no_death_to_its_foe_before_it_adds_no_revenge() {
    let mut log = Log::new();
    let other_death = log.new_key(KeyKind::Death, Some(0));
    let kill = log.new_key(KeyKind::Kill, Some(1));
    let death = log.new_key(KeyKind::Death, Some(1));
    log.again(other_death, WESTFALL, Track::World);

    log.again(kill, WESTFALL, Track::World);
    log.again(death, WESTFALL, Track::World);
    let fold = log.fold();

    assert_eq!(gains(&fold), [2, 3, 0]);
    assert!(fold.gains.iter().all(|gain| !gain.revenge));
}

#[test]
fn revenge_counts_once() {
    let mut log = Log::new();
    let death = log.new_key(KeyKind::Death, Some(0));
    let kill = log.new_key(KeyKind::RaidKill, Some(0));
    let other_kill = log.new_key(KeyKind::Kill, Some(0));
    log.again(death, WESTFALL, Track::World);

    log.again(kill, WESTFALL, Track::World);
    log.again(other_kill, WESTFALL, Track::World);
    let fold = log.fold();

    assert_eq!(gains(&fold), [2, 7, 3]);
    assert_eq!(fold.gains.iter().filter(|gain| gain.revenge).count(), 1);
}

#[test]
fn a_raid_night_does_not_cut_the_open_chapter() {
    let mut log = Log::new();
    log.quests(ELWYNN, 20);
    let entry = log.new_key(KeyKind::Raid, None);
    log.again(entry, MOLTEN_CORE, Track::Instance(MOLTEN_CORE));
    for foe in 0..10 {
        let boss = log.new_key(KeyKind::RaidKill, Some(foe));
        log.again(boss, MOLTEN_CORE, Track::Instance(MOLTEN_CORE));
    }

    log.wait(RUN_GAP_SECONDS);
    log.quest(ELWYNN);
    let fold = log.fold();

    assert!(fold.closed.is_empty());
    assert_eq!(fold.open.weight, 21);
    assert_eq!(fold.tales.len(), 1);
    assert_eq!(fold.tales[0].weight, 55);
    assert_eq!(fold.tales[0].runs, 1);
}

#[test]
fn a_wipe_and_a_corpse_run_stay_one_run() {
    let mut log = Log::new();
    let entry = log.new_key(KeyKind::Dungeon, None);
    let death = log.new_key(KeyKind::Death, Some(0));
    let boss = log.new_key(KeyKind::Kill, Some(0));
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));
    log.again(death, DEADMINES, Track::Instance(DEADMINES));
    log.fly_over(WESTFALL);
    log.wait(RUN_GAP_SECONDS - 3 * MINUTE);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));

    log.again(boss, DEADMINES, Track::Instance(DEADMINES));
    let fold = log.fold();

    assert!(fold.visits.is_empty());
    let visit = fold.visit.expect("the visit is open");
    assert_eq!((visit.first, visit.last), (0, 4));
    assert_eq!(visit.gain, 3 + 2 + 5);
}

#[test]
fn a_step_a_run_gap_after_you_left_closes_the_visit() {
    let mut log = Log::new();
    let entry = log.new_key(KeyKind::Dungeon, None);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));

    log.wait(RUN_GAP_SECONDS - MINUTE);
    log.fly_over(WESTFALL);
    let fold = log.fold();

    assert_eq!(fold.visits.len(), 1);
    assert_eq!(fold.visit, None);
    assert_eq!(fold.tales[0].runs, 1);
    assert_eq!(fold.tales[0].weight, 3);
}

#[test]
fn a_step_just_short_of_a_run_gap_keeps_the_visit_open() {
    let mut log = Log::new();
    let entry = log.new_key(KeyKind::Dungeon, None);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));

    log.wait(RUN_GAP_SECONDS - MINUTE - 1);
    log.fly_over(WESTFALL);
    let fold = log.fold();

    assert!(fold.visits.is_empty());
    assert!(fold.visit.is_some());
}

#[test]
fn a_second_dungeon_run_with_nothing_new_changes_only_its_count() {
    let mut log = Log::new();
    let entry = log.new_key(KeyKind::Dungeon, None);
    let boss = log.new_key(KeyKind::Kill, Some(0));
    for _ in 0..2 {
        log.again(entry, DEADMINES, Track::Instance(DEADMINES));
        log.again(boss, DEADMINES, Track::Instance(DEADMINES));
        log.wait(RUN_GAP_SECONDS);
        log.fly_over(WESTFALL);
    }

    let fold = log.fold();

    assert_eq!(fold.tales.len(), 1);
    assert_eq!(fold.tales[0].runs, 2);
    assert_eq!(fold.tales[0].weight, 6);
    assert_eq!(fold.visits[1].gain, 0);
}

#[test]
fn another_instance_closes_the_open_visit() {
    let mut log = Log::new();
    let deadmines = log.new_key(KeyKind::Dungeon, None);
    let core = log.new_key(KeyKind::Raid, None);
    log.again(deadmines, DEADMINES, Track::Instance(DEADMINES));

    log.again(core, MOLTEN_CORE, Track::Instance(MOLTEN_CORE));
    let fold = log.fold();

    assert_eq!(fold.tales.len(), 2);
    assert_eq!(fold.visits.len(), 1);
    assert_eq!(fold.visits[0].tale, 0);
    assert_eq!(fold.visit.map(|visit| visit.tale), Some(1));
}

#[test]
fn a_level_inside_a_dungeon_counts_for_the_chapter() {
    let mut log = Log::new();
    let entry = log.new_key(KeyKind::Dungeon, None);
    let level = log.new_key(KeyKind::Level, None);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));

    log.again(level, DEADMINES, Track::World);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));
    let fold = log.fold();

    assert_eq!(fold.open.weight, 2);
    assert!(fold.visits.is_empty());
    assert_eq!(fold.visit.map(|visit| visit.gain), Some(3));
}

#[test]
fn an_instance_step_adds_nothing_to_the_chapter_and_closes_nothing() {
    let mut log = Log::new();
    log.quests(ELWYNN, 20);
    log.play(None, ELWYNN, Track::World, Some(Break::Capital));

    let entry = log.new_key(KeyKind::Dungeon, None);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));
    let fold = log.fold();

    assert!(fold.closed.is_empty());
    assert_eq!(fold.open.weight, 20);
    assert_eq!(fold.pending, Some(Break::Capital));
}

#[test]
fn a_rule_change_closes_the_open_chapter_below_the_least_weight() {
    let mut log = Log::new();
    log.quests(ELWYNN, 3);

    log.steps.push(Step::Rule(2));
    log.quest(ELWYNN);
    let fold = log.fold();

    assert_eq!(fold.closed.len(), 1);
    assert_eq!(fold.closed[0].close, Close::Rule);
    assert_eq!(fold.closed[0].chapter.weight, 3);
    assert_eq!(fold.open.first, 3);
    assert_eq!(fold.open.rule, 2);
    assert_eq!(fold.open.opening, Opening::Rule);
}

#[test]
fn a_rule_change_keeps_the_seen_keys() {
    let mut log = Log::new();
    let hogger = log.new_key(KeyKind::Kill, Some(0));
    log.again(hogger, ELWYNN, Track::World);

    log.steps.push(Step::Rule(2));
    log.again(hogger, ELWYNN, Track::World);
    let fold = log.fold();

    assert_eq!(gains(&fold), [3, 0, 0]);
}

#[test]
fn a_rule_change_at_the_start_closes_no_chapter() {
    let fold = chapters(&[Step::Rule(2)]);

    assert!(fold.closed.is_empty());
    assert_eq!(fold.open.rule, 2);
    assert_eq!(fold.gains.len(), 1);
}

#[test]
fn an_id_out_of_range_gains_nothing() {
    let key = Key {
        id: 5,
        kind: KeyKind::Raid,
        foe: None,
    };
    let play = Play {
        key: Some(key),
        zone: 7,
        track: Track::World,
        mark: None,
        at: 0,
    };

    let fold = chapters(&[Step::Play(play)]);

    assert_eq!(gains(&fold), [0]);
    assert!(fold.keys.is_empty());
    assert!(fold.zones.is_empty());
    assert_eq!(fold.open.zone, None);
}

#[test]
fn a_zone_flown_over_takes_its_id_and_the_next_zone_still_settles() {
    let mut log = Log::new();
    log.fly_over(ELWYNN);
    log.quests(WESTFALL, 15);

    log.quest(DUSKWOOD);
    let fold = log.fold();

    assert_eq!(fold.zones.len(), 3);
    assert_eq!(fold.closed.len(), 1);
    assert_eq!(fold.open.opening, Opening::Break(Break::NewZone));
}

#[test]
fn a_death_to_a_foe_out_of_range_weighs_nothing() {
    let key = Key {
        id: 0,
        kind: KeyKind::Death,
        foe: Some(9),
    };
    let play = Play {
        key: Some(key),
        zone: 0,
        track: Track::World,
        mark: None,
        at: 0,
    };

    let fold = chapters(&[Step::Play(play)]);

    assert_eq!(gains(&fold), [0]);
}

#[test]
fn one_key_never_adds_more_than_the_cap() {
    let mut log = Log::new();
    let raid = log.new_key(KeyKind::RaidKill, None);
    log.again(raid, ELWYNN, Track::World);
    let death = Key {
        kind: KeyKind::Death,
        ..raid
    };

    log.again(death, ELWYNN, Track::World);
    log.again(death, ELWYNN, Track::World);
    let fold = log.fold();

    assert_eq!(gains(&fold), [5, 2, 0]);
    assert_eq!(fold.keys[raid.id].gain, CAP_MAX);
}

#[test]
fn folding_one_line_at_a_time_gives_the_same_fold() {
    let mut log = Log::new();
    log.quests(ELWYNN, 30);
    let entry = log.new_key(KeyKind::Dungeon, None);
    log.again(entry, DEADMINES, Track::Instance(DEADMINES));
    log.quests(WESTFALL, 30);

    let mut fold = start();
    for step in &log.steps {
        advance(&mut fold, std::slice::from_ref(step));
    }

    assert_eq!(fold, log.fold());
}
