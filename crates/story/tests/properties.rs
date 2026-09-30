//! Rules that hold for any play, checked against many random cases.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use proptest::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use timeways_story::best_of_two::{Next, Round};
use timeways_story::chapters::{
    MIN_CHAPTER_PLAY_SECONDS, SESSION_GAP_SECONDS, chapter_starts, sessions,
};
use timeways_story::character::Character;
use timeways_story::check::{Fault, check, later_names, without_citations};
use timeways_story::chronicle::{Pick, Saga};
use timeways_story::hero::{MAX_TEXT_BYTES, MAX_TEXT_CHARS, checked_text};
use timeways_story::house::fenced;
use timeways_story::input::{GameQuestKind, Input, MessageId, Reaction};
use timeways_story::journal::{Journal, journal, pages};
use timeways_story::pace::{Pace, WINDOW_SECONDS};
use timeways_story::pack::Pack;
use timeways_story::places::InstanceKind;
use timeways_story::quest::{MAX_KILLS, QuestChange, Status, Step, quest_log};
use timeways_story::reply_size::{MAX_LINE, MAX_SLOT, Size};
use timeways_story::seen::TextKind;
use timeways_story::spot::{MAP_IDS, Spot, THOUSANDTHS, spot_of};
use timeways_story::store::{CharacterKey, Store, safe_id};
use timeways_story::story::{Output, Story};
use timeways_story::wikitext::plain;

/// One step of play, as the addon sends it.
#[derive(Clone, Debug)]
enum Play {
    Zone(String, Option<String>, Option<Spot>),
    Meet(String, Option<Spot>),
    Defeat(String),
    Slap(String),
    Die(Option<String>),
    Level(u8),
    Emote(String, u8),
    HeroSet(usize, String),
    HeroAdd(String),
    HeroRemove(u64),
    Read(TextKind, Option<String>, String),
    /// A talk, with the words of the model: a rumor.
    Talk(String, String),
    /// `/quest` to an NPC, and the steps that the model proposes.
    Quest(String, Vec<TaskStep>),
    /// A hover or a target: the NPC, whether you can attack it, and its creature type.
    See(String, Reaction, Option<String>),
    /// A kill for a kill step.
    Kill(String),
    Accept,
    Decline,
    /// A lasting buff or debuff of a quest of the game.
    Mark(String, String),
    /// A zone that the game calls an instance.
    Instance(String, InstanceKind),
    /// A quest of the game: taken, or turned in.
    GameQuest(String, GameQuestKind, bool),
    Wait(u64),
}

/// A step as a model proposes it. The count of a kill sits often at its edges.
#[derive(Clone, Debug)]
enum TaskStep {
    Visit(String),
    Meet(String),
    Kill(String, u8),
}

fn task_step(names: impl Strategy<Value = String> + Clone) -> impl Strategy<Value = TaskStep> {
    let count = prop_oneof![
        Just(0u8),
        Just(1),
        Just(MAX_KILLS),
        Just(MAX_KILLS + 1),
        0u8..=12
    ];
    prop_oneof![
        names.clone().prop_map(TaskStep::Visit),
        names.clone().prop_map(TaskStep::Meet),
        (names, count).prop_map(|(creature, count)| TaskStep::Kill(creature, count)),
    ]
}

fn sighting(names: impl Strategy<Value = String>) -> impl Strategy<Value = Play> {
    (
        names,
        prop::sample::select(vec![Reaction::Hostile, Reaction::Friendly]),
        prop::option::of(prop::sample::select(vec!["beast", "critter", "humanoid"])),
    )
        .prop_map(|(name, reaction, creature)| {
            Play::See(name, reaction, creature.map(String::from))
        })
}

/// Four names for every role, so that a task often names what the player knows.
fn target_name() -> impl Strategy<Value = String> + Clone {
    prop::sample::select(vec!["Mill Pond", "Keeper Tessa", "Duskbat", "Farmer Bram"])
        .prop_map(String::from)
}

/// The plays that change what a task can name.
fn target_play() -> impl Strategy<Value = Play> {
    let names = target_name;
    prop_oneof![
        (names(), prop::option::of(names())).prop_map(|(zone, sub)| Play::Zone(zone, sub, None)),
        names().prop_map(|name| Play::Meet(name, None)),
        names().prop_map(Play::Slap),
        sighting(names()),
        (names(), prop::collection::vec(task_step(names()), 1..3))
            .prop_map(|(npc, steps)| Play::Quest(npc, steps)),
        Just(Play::Accept),
        Just(Play::Decline),
        names().prop_map(Play::Kill),
    ]
}

/// Few names, so that the same NPC and the same place come back.
fn name() -> impl Strategy<Value = String> + Clone {
    prop_oneof![
        prop::sample::select(vec![
            "Goldshire",
            "Westfall",
            "Hogger",
            "Innkeeper Farley",
            "Cow"
        ])
        .prop_map(String::from),
        "[A-Za-z' ]{1,12}",
    ]
}

/// Mostly short, and often near the limits: long, outside ASCII, or full of quotes.
fn hero_text() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Za-z ]{0,40}",
        "[a\"|\u{e9}\u{10348}]{250,320}",
        "[A-Za-z\"]{900,1000}",
    ]
}

fn play() -> impl Strategy<Value = Play> {
    prop_oneof![
        (name(), prop::option::of(name()), prop::option::of(spot()))
            .prop_map(|(zone, subzone, spot)| Play::Zone(zone, subzone, spot)),
        (name(), prop::option::of(spot())).prop_map(|(name, spot)| Play::Meet(name, spot)),
        name().prop_map(Play::Defeat),
        name().prop_map(Play::Slap),
        prop::option::of(name()).prop_map(Play::Die),
        (1u8..=60).prop_map(Play::Level),
        ("[a-z]{1,8}", 0u8..24).prop_map(|(emote, hour)| Play::Emote(emote, hour)),
        (0usize..6, hero_text()).prop_map(|(field, text)| Play::HeroSet(field, text)),
        "[A-Za-z ]{1,40}".prop_map(Play::HeroAdd),
        (1u64..6).prop_map(Play::HeroRemove),
        (
            prop::sample::select(vec![TextKind::Quest, TextKind::Gossip, TextKind::Book]),
            prop::option::of(name()),
            "[A-Za-z$ \n]{1,300}",
        )
            .prop_map(|(kind, title, text)| Play::Read(kind, title, text)),
        (name(), "[A-Za-z ]{1,60}").prop_map(|(npc, say)| Play::Talk(npc, say)),
        (name(), prop::collection::vec(task_step(name()), 0..5))
            .prop_map(|(npc, steps)| Play::Quest(npc, steps)),
        sighting(name()),
        name().prop_map(Play::Kill),
        Just(Play::Accept),
        Just(Play::Decline),
        (name(), name()).prop_map(|(quest, mark)| Play::Mark(quest, mark)),
        (
            name(),
            prop::sample::select(vec![InstanceKind::Dungeon, InstanceKind::Raid]),
        )
            .prop_map(|(zone, kind)| Play::Instance(zone, kind)),
        (
            name(),
            prop::sample::select(vec![GameQuestKind::Normal, GameQuestKind::Class]),
            any::<bool>(),
        )
            .prop_map(|(title, kind, done)| Play::GameQuest(title, kind, done)),
        (0u64..20_000).prop_map(Play::Wait),
    ]
}

/// A position on a map. The ends of each band come often, because a uniform draw almost
/// never reaches them.
fn spot() -> impl Strategy<Value = Spot> {
    let map = prop_oneof![Just(MAP_IDS.min), Just(MAP_IDS.max), 1400i64..1460];
    let edge = || prop_oneof![Just(THOUSANDTHS.min), Just(THOUSANDTHS.max), 0i64..=1000];
    (map, edge(), edge()).prop_map(|(map, x, y)| {
        serde_json::from_value(serde_json::json!({ "map": map, "x": x, "y": y })).unwrap()
    })
}

/// Few numbers and short quests, so that most changes find their quest.
fn quest_change() -> impl Strategy<Value = QuestChange> {
    let number = 1u64..4;
    let step = Step::Meet {
        npc: "Farmer Bram".to_string(),
    };
    prop_oneof![
        (number.clone(), 0usize..4).prop_map(move |(number, steps)| QuestChange::Offered {
            number,
            at: Tick(1),
            giver: "Keeper Tessa".to_string(),
            title: "A Task".to_string(),
            text: "Go.".to_string(),
            steps: vec![step.clone(); steps],
        }),
        number.clone().prop_map(|number| QuestChange::Accepted {
            number,
            at: Tick(2)
        }),
        number.clone().prop_map(|number| QuestChange::Declined {
            number,
            at: Tick(2)
        }),
        number.clone().prop_map(|number| QuestChange::Abandoned {
            number,
            at: Tick(2)
        }),
        (number, 0usize..4).prop_map(|(number, step)| QuestChange::StepDone {
            number,
            step,
            at: Tick(3),
        }),
    ]
}

/// Steps between two events of play. Most sit at the edges of the session gap, where a
/// uniform draw almost never lands.
fn play_step() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(SESSION_GAP_SECONDS - 1),
        Just(SESSION_GAP_SECONDS),
        Just(SESSION_GAP_SECONDS + 1),
        Just(MIN_CHAPTER_PLAY_SECONDS),
        Just(MIN_CHAPTER_PLAY_SECONDS - 1),
        0..SESSION_GAP_SECONDS + 60,
        0..10 * SESSION_GAP_SECONDS,
    ]
}

/// The seconds of play between two ticks, counted the slow way: one second at a time
/// that lies inside a session.
fn played_slowly(sessions: &[(Tick, Tick)], from: Tick, to: Tick) -> u64 {
    (from.0..to.0)
        .filter(|second| {
            sessions
                .iter()
                .any(|(began, ended)| began.0 <= *second && *second < ended.0)
        })
        .count() as u64
}

/// The edges of `i64` come often, because a uniform draw almost never reaches them.
fn change_of_trust() -> impl Strategy<Value = i64> {
    prop_oneof![Just(i64::MAX), Just(i64::MIN), -10i64..=10, any::<i64>()]
}

const FIELDS: [&str; 6] = ["origin", "background", "goal", "bond", "flaw", "traits"];

fn input(play: &Play, at: Tick) -> Option<Input> {
    Some(match play.clone() {
        Play::Zone(zone, subzone, spot) => Input::ZoneEntered {
            at,
            zone,
            subzone,
            spot,
        },
        Play::Meet(name, spot) => Input::NpcMet { at, name, spot },
        Play::Defeat(name) => Input::NpcDefeated { at, name },
        Play::Slap(name) => Input::NpcSlapped { at, name },
        Play::Mark(quest, mark) => Input::QuestMarked { at, quest, mark },
        Play::Instance(zone, kind) => Input::InstanceEntered { at, zone, kind },
        Play::GameQuest(title, kind, false) => Input::GameQuestAccepted { at, title, kind },
        Play::GameQuest(title, kind, true) => Input::GameQuestDone { at, title, kind },
        Play::Die(killer) => Input::Died {
            at,
            killer,
            cause: None,
            killer_level: None,
            hour: None,
        },
        Play::Level(level) => Input::LevelReached { at, level },
        Play::Emote(emote, hour) => Input::EmoteDone {
            at,
            emote,
            target: None,
            hour: Some(hour),
        },
        Play::HeroSet(field, text) => Input::HeroSet {
            at,
            field: FIELDS[field].to_string(),
            text,
        },
        Play::HeroAdd(text) => Input::HeroAdded {
            at,
            text,
            npc: None,
        },
        Play::HeroRemove(number) => Input::HeroRemoved { at, number },
        Play::Read(kind, title, text) => Input::TextSeen {
            at,
            kind,
            title,
            npc: None,
            zone: Some("Goldshire".to_string()),
            text,
        },
        Play::Talk(npc, _) => Input::TalkAsked {
            id: MessageId(2),
            at,
            npc,
            text: "any news".to_string(),
        },
        Play::Quest(npc, _) => Input::QuestAsked { at, npc },
        Play::See(name, reaction, creature) => Input::NpcSeen {
            at,
            name,
            reaction,
            creature,
        },
        Play::Kill(name) => Input::NpcKilled { at, name },
        Play::Accept => Input::QuestAccepted { at, number: None },
        Play::Decline => Input::QuestDeclined { at, number: None },
        Play::Wait(_) => return None,
    })
}

/// The answer of a model to a quest call, with the steps of the play.
fn quest_answer(steps: &[TaskStep]) -> String {
    let steps: Vec<serde_json::Value> = steps
        .iter()
        .map(|step| match step {
            TaskStep::Visit(place) => serde_json::json!({ "goal": "visit", "place": place }),
            TaskStep::Meet(npc) => serde_json::json!({ "goal": "meet", "npc": npc }),
            TaskStep::Kill(creature, count) => {
                serde_json::json!({ "goal": "kill", "creature": creature, "count": count })
            }
        })
        .collect();
    serde_json::json!({ "title": "A Task", "text": "Go.", "steps": steps }).to_string()
}

fn fresh(name: &str) -> PathBuf {
    static CASE: AtomicUsize = AtomicUsize::new(0);
    let case = CASE.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("property-{name}-{case}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn story(folder: &Path, files: Store) -> Story {
    let pack = folder.join("pack.sqlite");
    if !pack.exists() {
        Pack::write(&pack, &[]).unwrap();
    }
    let mut story = Story::new(Pack::open(&pack).unwrap(), files);
    story
        .handle(Input::CharacterEntered {
            realm: "Stormrage".to_string(),
            name: "Ada".to_string(),
        })
        .unwrap();
    story
}

/// Plays each step, and moves the clock. A refused step is part of the game.
fn run(story: &mut Story, plays: &[Play], clock: &mut u64) {
    for play in plays {
        play_once(story, play, clock);
    }
}

/// Plays one step. The model answers a talk and a quest at once, and the outputs of its
/// answer come back.
fn play_once(story: &mut Story, play: &Play, clock: &mut u64) -> Vec<Output> {
    *clock += if let Play::Wait(seconds) = play {
        *seconds
    } else {
        1
    };
    let Some(input) = input(play, Tick(*clock)) else {
        return Vec::new();
    };
    let mut outputs = story.handle(input).unwrap_or_default();
    if let Play::Quest(..) = play {
        outputs = story
            .handle(Input::BatchEnd { id: MessageId(3) })
            .unwrap_or_default();
    }
    let Some(Output::ModelCall { call, .. }) = outputs.first() else {
        return Vec::new();
    };
    let text = match play {
        Play::Talk(_, say) => serde_json::json!({ "say": say, "trust": 1 }).to_string(),
        Play::Quest(_, steps) => quest_answer(steps),
        _ => return Vec::new(),
    };
    let call = *call;
    story
        .handle(Input::ModelAnswered { call, text })
        .unwrap_or_default()
}

/// The rules of 3.4 for the targets of a task, stated again apart from the story, from
/// what the plays tell: the places that you visited, the NPCs that you met, and the last
/// sighting of each NPC.
#[derive(Default)]
struct Targets {
    places: Vec<String>,
    met: Vec<String>,
    /// The last reaction of each NPC that you saw, and whether a sighting found an animal.
    seen: HashMap<String, (Reaction, bool)>,
    /// The targets of the newest offer.
    last: Vec<String>,
}

impl Targets {
    fn watch(&mut self, play: &Play) {
        match play {
            Play::Zone(zone, subzone, _) => {
                self.places.push(zone.clone());
                self.places.extend(subzone.clone());
            }
            Play::Meet(npc, _) | Play::Slap(npc) | Play::Talk(npc, _) | Play::Quest(npc, _) => {
                self.met.push(npc.clone());
            }
            Play::See(name, reaction, creature) => {
                let animal = matches!(creature.as_deref(), Some("beast" | "critter"));
                let was_animal = self.seen.get(name).is_some_and(|(_, animal)| *animal);
                self.seen
                    .insert(name.clone(), (*reaction, animal || was_animal));
            }
            _ => {}
        }
    }

    fn allows(&self, giver: &str, step: &TaskStep) -> bool {
        if self.last.contains(&target(step).to_string()) {
            return false;
        }
        match step {
            TaskStep::Visit(place) => self.places.contains(place),
            TaskStep::Meet(npc) => {
                let sighting = self.seen.get(npc);
                let known = self.met.contains(npc) || sighting.is_some();
                let friendly = sighting
                    .is_none_or(|(reaction, animal)| *reaction == Reaction::Friendly && !animal);
                npc != giver && known && friendly
            }
            TaskStep::Kill(creature, count) => {
                let seen = self.seen.get(creature);
                let hostile = seen.is_some_and(|(reaction, _)| *reaction == Reaction::Hostile);
                hostile && (1..=MAX_KILLS).contains(count)
            }
        }
    }
}

fn target(step: &TaskStep) -> &str {
    match step {
        TaskStep::Visit(name) | TaskStep::Meet(name) | TaskStep::Kill(name, _) => name,
    }
}

/// The giver of an offer that came back as the narrator line, or None.
fn offer_giver(outputs: &[Output]) -> Option<&str> {
    let Some(Output::EventsSeen {
        narrator: Some(line),
        ..
    }) = outputs.first()
    else {
        return None;
    };
    line.split_once(" has a task for you: ")
        .map(|(giver, _)| giver)
}

/// Every page of the journal as the bridge gets it.
fn journal_lines(story: &mut Story) -> Vec<String> {
    let mut lines = Vec::new();
    let mut page = 0;
    loop {
        let outputs = story
            .handle(Input::JournalAsked {
                id: MessageId(1),
                page,
            })
            .unwrap();
        let Some(Output::Journal {
            page: journal_page, ..
        }) = outputs.first()
        else {
            panic!("expected a journal, got {outputs:?}");
        };
        lines.push(serde_json::to_string(&outputs[0]).unwrap());
        page += 1;
        if page >= journal_page.pages {
            return lines;
        }
    }
}

/// One round of the best of two. `drafts` are the texts of the drafts that pass the checks,
/// and `tight` is the state of the window after each draft. Gives the calls and the saga.
fn play_round(drafts: &[Option<String>; 2], tight: [bool; 2], pick: Pick) -> (usize, Option<Saga>) {
    let key = CharacterKey::new("Stormrage", "Ada").unwrap();
    let mut round = Round::new(key, Tick(1), Vec::new(), 1, String::new(), String::new());
    let saga = |text: &Option<String>| {
        text.as_ref().map(|text| Saga {
            text: text.clone(),
            footnotes: Vec::new(),
        })
    };
    let mut calls = 1;
    round.add_draft(saga(&drafts[0]));
    for tight in tight {
        match round.next(tight) {
            Next::Final(saga) => return (calls, saga),
            Next::Call(_) if round.is_judged() => return (calls + 1, round.picked(pick)),
            Next::Call(_) => {
                calls += 1;
                round.add_draft(saga(&drafts[1]));
            }
        }
    }
    unreachable!("a round ends after its second draft")
}

fn pick() -> impl Strategy<Value = Pick> {
    prop_oneof![Just(Pick::First), Just(Pick::Second)]
}

/// A step of time for the pace: often at the edge of the window.
fn pace_step() -> impl Strategy<Value = u64> {
    prop_oneof![
        0..3u64,
        WINDOW_SECONDS - 2..WINDOW_SECONDS + 2,
        0..3 * WINDOW_SECONDS,
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn a_chapter_saga_costs_at_most_three_calls_and_keeps_a_passing_draft(
        drafts in prop::array::uniform2(prop::option::of("[a-z]{1,8}")),
        tight in prop::array::uniform2(any::<bool>()),
        pick in pick(),
    ) {
        let (calls, saga) = play_round(&drafts, tight, pick);

        prop_assert!(calls <= 3);
        if tight[0] {
            prop_assert_eq!(calls, 1);
        }
        let asked = &drafts[..calls.min(2)];
        let passed: Vec<&String> = asked.iter().flatten().collect();
        prop_assert_eq!(saga.is_some(), !passed.is_empty());
        if let Some(saga) = saga {
            prop_assert!(passed.contains(&&saga.text), "{:?} not in {:?}", saga, passed);
        }
    }

    /// The pace against a plain count of every call and failure.
    #[test]
    fn the_window_is_tight_after_a_recent_failure_or_five_recent_calls(
        steps in prop::collection::vec((any::<bool>(), pace_step()), 0..30),
        last in pace_step(),
    ) {
        let mut pace = Pace::default();
        let (mut opened, mut failed) = (Vec::new(), Vec::new());
        let mut now = 0;
        for (is_failure, step) in steps {
            now += step;
            if is_failure {
                pace.failed(Tick(now));
                failed.push(now);
            } else {
                pace.opened(Tick(now));
                opened.push(now);
            }
        }
        now += last;

        let recent = |times: &[u64]| times.iter().filter(|&&at| now - at < WINDOW_SECONDS).count();
        prop_assert_eq!(pace.is_tight(Tick(now)), recent(&failed) > 0 || recent(&opened) >= 5);
    }

    #[test]
    fn a_world_on_disk_reads_back_as_the_same_journal(plays in prop::collection::vec(play(), 0..60)) {
        let folder = fresh("round-trip");
        let mut clock = 1_000;
        let mut first = story(&folder, Store::Folder(folder.clone()));
        run(&mut first, &plays, &mut clock);
        journal_lines(&mut first);
        let before = journal_lines(&mut first);
        drop(first);

        let mut second = story(&folder, Store::Folder(folder.clone()));
        let after = journal_lines(&mut second);

        prop_assert_eq!(before, after);
    }

    #[test]
    fn a_restart_at_any_point_leaves_the_same_story(
        plays in prop::collection::vec(play(), 0..60),
        restarts in prop::collection::vec(any::<prop::sample::Index>(), 0..4),
    ) {
        let folder = fresh("restarts");
        let mut cuts: Vec<usize> = restarts.iter().map(|index| index.index(plays.len() + 1)).collect();
        cuts.sort_unstable();
        let mut clock = 1_000;
        let mut restarted = story(&folder, Store::Folder(folder.clone()));
        let mut from = 0;
        for cut in cuts.into_iter().chain([plays.len()]) {
            run(&mut restarted, &plays[from..cut], &mut clock);
            from = cut;
            drop(restarted);
            restarted = story(&folder, Store::Folder(folder.clone()));
        }
        let mut clock = 1_000;
        let mut whole = story(&fresh("restarts-memory"), Store::Memory);
        run(&mut whole, &plays, &mut clock);

        // The reason for a refused edit shows once and is not kept, so a first read clears it.
        journal_lines(&mut restarted);
        journal_lines(&mut whole);
        prop_assert_eq!(journal_lines(&mut restarted), journal_lines(&mut whole));
    }

    #[test]
    fn every_page_keeps_the_limits_of_the_bridge(plays in prop::collection::vec(play(), 0..120)) {
        let folder = fresh("pages");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Memory);
        run(&mut story, &plays, &mut clock);

        for line in journal_lines(&mut story) {
            let value: serde_json::Value = serde_json::from_str(&line).unwrap();
            let limit = Size { line: MAX_LINE, slot: MAX_SLOT };
            prop_assert!(Size::of_json(line.as_bytes()).fits(limit));
            for list in ["chapters", "places", "people", "deeds", "learned", "quests"] {
                prop_assert!(value[list].as_array().unwrap().len() <= 200);
            }
        }
    }

    #[test]
    fn the_pages_joined_are_the_whole_journal(plays in prop::collection::vec(play(), 0..300)) {
        let mut character = Character::new();
        let mut clock = 1_000;
        for play in &plays {
            clock += if let Play::Wait(seconds) = play { *seconds } else { 1 };
            let at = Tick(clock);
            let _ = match play {
                Play::Zone(zone, subzone, _) => character.enter_zone(at, zone, subzone.as_deref()),
                Play::Meet(name, _) => character.meet_npc(at, name),
                Play::Defeat(name) => character.defeat_npc(at, name),
                Play::Slap(name) => character.slap(at, name),
                Play::Die(killer) => character.die(at, killer.as_deref()),
                Play::Level(level) => character.reach_level(at, *level),
                _ => Ok(()),
            };
        }
        let whole = journal(&character);

        let mut joined = Journal::default();
        for page in pages(whole.clone()) {
            joined.chapters.extend(page.journal.chapters);
            joined.places.extend(page.journal.places);
            joined.people.extend(page.journal.people);
            joined.deeds.extend(page.journal.deeds);
        }

        prop_assert_eq!(joined, whole);
    }

    #[test]
    fn the_player_may_write_any_words_in_their_own_text(
        words in prop::collection::vec(
            prop_oneof![
                prop::sample::select(later_names().collect::<Vec<_>>()).prop_map(String::from),
                "[A-Za-z,.'!?]{1,12}",
            ],
            1..40,
        )
    ) {
        let text = words.join(" ");
        prop_assume!(text.chars().count() <= MAX_TEXT_CHARS && text.len() <= MAX_TEXT_BYTES);

        prop_assert_eq!(checked_text(&text), Ok(text.clone()));
    }

    #[test]
    fn an_offered_task_only_names_allowed_targets(
        plays in prop::collection::vec(target_play(), 0..150)
    ) {
        let folder = fresh("targets");
        let mut story = story(&folder, Store::Memory);
        let mut targets = Targets::default();
        let mut clock = 1_000;

        for play in &plays {
            targets.watch(play);
            let outputs = play_once(&mut story, play, &mut clock);
            let (Some(giver), Play::Quest(_, steps)) = (offer_giver(&outputs), play) else {
                continue;
            };
            for step in steps {
                prop_assert!(targets.allows(giver, step), "{step:?} from {giver}");
            }
            targets.last = steps.iter().map(|step| target(step).to_string()).collect();
        }
    }

    #[test]
    fn two_names_never_share_a_file_name(a in ".{0,16}", b in ".{0,16}") {
        prop_assume!(a != b);
        prop_assert_ne!(safe_id(&a), safe_id(&b));
        prop_assert!(safe_id(&a).bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'));
    }

    /// Mostly brackets, digits, and commas, so that broken and nested citations come often.
    #[test]
    fn an_answer_without_citations_cites_nothing(text in "[\\[\\]0-9, a.]{0,30}") {
        let plain = without_citations(&text);

        let faults = check(&plain, 0);
        prop_assert!(
            !faults.iter().any(|fault| matches!(fault, Fault::UnknownCitation { .. })),
            "{:?} became {:?}", text, plain
        );
    }

    /// Mostly angle marks, so that broken and joined fence marks come often.
    #[test]
    fn no_input_can_close_its_fence(text in "[<> a]{0,24}") {
        let fenced = fenced(&text);

        let inside = &fenced[4..fenced.len() - 4];
        prop_assert!(!inside.contains("<<<") && !inside.contains(">>>"), "{:?}", fenced);
        prop_assert_eq!(fenced.matches(">>>").count(), 1);
    }

    #[test]
    fn a_place_or_an_npc_never_moves_from_its_first_spot(plays in prop::collection::vec(play(), 0..120)) {
        let mut character = Character::new();
        let mut first = std::collections::BTreeMap::new();
        for (n, play) in plays.iter().enumerate() {
            let at = Tick(1_000 + n as u64);
            let _ = match play {
                Play::Zone(zone, subzone, spot) => character
                    .enter_zone(at, zone, subzone.as_deref())
                    .and_then(|()| spot.map_or(Ok(()), |spot| character.mark_here(at, spot))),
                Play::Meet(name, spot) => character
                    .meet_npc(at, name)
                    .and_then(|()| spot.map_or(Ok(()), |spot| character.mark_npc(at, name, spot))),
                _ => Ok(()),
            };
            let world = character.world();
            for entity in world.entities() {
                if let Some(spot) = spot_of(world, entity.id) {
                    prop_assert_eq!(*first.entry(entity.id).or_insert(spot), spot);
                }
            }
        }
    }

    #[test]
    fn trust_stays_between_minus_one_hundred_and_one_hundred(changes in prop::collection::vec(change_of_trust(), 0..40)) {
        let mut character = Character::new();
        for (at, by) in changes.iter().enumerate() {
            character.adjust_trust(Tick(at as u64 + 1), "Innkeeper Farley", *by).unwrap();
            let trust = character.trust_of("Innkeeper Farley").unwrap_or(0);
            prop_assert!((-100..=100).contains(&trust), "{}", trust);
        }
    }

    #[test]
    fn a_chapter_begins_only_at_a_milestone_after_enough_play(
        steps in prop::collection::vec(play_step(), 1..12),
        picks in prop::collection::vec(any::<prop::sample::Index>(), 0..6),
    ) {
        let mut ticks = vec![Tick(1)];
        for step in &steps {
            ticks.push(Tick(ticks[ticks.len() - 1].0 + step));
        }
        let milestones: Vec<Tick> = picks.iter().map(|pick| *pick.get(&ticks)).collect();
        let sessions = sessions(ticks.iter().copied());

        let starts = chapter_starts(&sessions, &milestones);

        prop_assert_eq!(starts[0], Tick(1));
        for pair in starts.windows(2) {
            prop_assert!(milestones.contains(&pair[1]));
            prop_assert!(played_slowly(&sessions, pair[0], pair[1]) >= MIN_CHAPTER_PLAY_SECONDS);
        }
        for milestone in milestones.iter().filter(|milestone| !starts.contains(milestone)) {
            let began = starts.iter().rev().find(|start| *start <= milestone).unwrap();
            prop_assert!(played_slowly(&sessions, *began, *milestone) < MIN_CHAPTER_PLAY_SECONDS);
        }
    }

    #[test]
    fn a_quest_log_never_skips_a_step_and_each_giver_holds_at_most_one_offer(
        changes in prop::collection::vec(quest_change(), 0..40),
    ) {
        let quests = quest_log(&changes);

        let offers = quests.iter().filter(|quest| quest.status == Status::Offered).count();
        prop_assert!(offers <= 1);
        for quest in &quests {
            prop_assert!(quest.steps_done <= quest.steps.len());
            let finished = !quest.steps.is_empty() && quest.steps_done == quest.steps.len();
            prop_assert_eq!(quest.status == Status::Done, finished);
            prop_assert_eq!(quest.done_at.is_some(), finished);
        }
    }

    /// The input is mostly markup characters, so broken and nested markup comes often.
    #[test]
    fn plain_text_from_any_wikitext_holds_no_markup_marks(
        text in "([\\[\\]{}'|<>/=!-]|ref|File:|http://| |a|\n){0,120}",
    ) {
        let cleaned = plain(&text);

        for mark in ["[[", "]]", "{{", "}}", "''"] {
            prop_assert!(!cleaned.contains(mark), "{} in {:?}", mark, cleaned);
        }
    }

    #[test]
    fn text_without_markup_stays_as_it_is(text in "[a-zA-Z0-9 .,;:?\n]{0,200}") {
        prop_assert_eq!(plain(&text), text);
    }
}
