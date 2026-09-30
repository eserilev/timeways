//! Rules that hold for any play, checked against many random cases.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use proptest::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use timeways_story::chapters::{
    MIN_CHAPTER_PLAY_SECONDS, SESSION_GAP_SECONDS, chapter_starts, sessions,
};
use timeways_story::character::Character;
use timeways_story::check::{Fault, check, without_citations};
use timeways_story::house::fenced;
use timeways_story::input::{GameQuestKind, Input, MessageId};
use timeways_story::journal::{Journal, journal, pages};
use timeways_story::pack::Pack;
use timeways_story::places::InstanceKind;
use timeways_story::quest::{QuestChange, Status, Step, quest_log};
use timeways_story::reply_size::{MAX_LINE, MAX_SLOT, Size};
use timeways_story::seen::TextKind;
use timeways_story::store::{Store, safe_id};
use timeways_story::story::{Output, Story};

/// One step of play, as the addon sends it.
#[derive(Clone, Debug)]
enum Play {
    Zone(String, Option<String>),
    Meet(String),
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
    /// `/quest` to an NPC, and the steps that the model proposes: `true` visits.
    Quest(String, Vec<(bool, String)>),
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

/// Few names, so that the same NPC and the same place come back.
fn name() -> impl Strategy<Value = String> {
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

fn play() -> impl Strategy<Value = Play> {
    prop_oneof![
        (name(), prop::option::of(name())).prop_map(|(zone, subzone)| Play::Zone(zone, subzone)),
        name().prop_map(Play::Meet),
        name().prop_map(Play::Defeat),
        name().prop_map(Play::Slap),
        prop::option::of(name()).prop_map(Play::Die),
        (1u8..=60).prop_map(Play::Level),
        ("[a-z]{1,8}", 0u8..24).prop_map(|(emote, hour)| Play::Emote(emote, hour)),
        (0usize..6, "[A-Za-z ]{0,40}").prop_map(|(field, text)| Play::HeroSet(field, text)),
        "[A-Za-z ]{1,40}".prop_map(Play::HeroAdd),
        (1u64..6).prop_map(Play::HeroRemove),
        (
            prop::sample::select(vec![TextKind::Quest, TextKind::Gossip, TextKind::Book]),
            prop::option::of(name()),
            "[A-Za-z$ \n]{1,300}",
        )
            .prop_map(|(kind, title, text)| Play::Read(kind, title, text)),
        (name(), "[A-Za-z ]{1,60}").prop_map(|(npc, say)| Play::Talk(npc, say)),
        (name(), prop::collection::vec((any::<bool>(), name()), 0..5))
            .prop_map(|(npc, steps)| Play::Quest(npc, steps)),
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
        Play::Zone(zone, subzone) => Input::ZoneEntered { at, zone, subzone },
        Play::Meet(name) => Input::NpcMet { at, name },
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
        Play::Accept => Input::QuestAccepted { at, number: None },
        Play::Decline => Input::QuestDeclined { at, number: None },
        Play::Wait(_) => return None,
    })
}

/// The answer of a model to a quest call, with the steps of the play.
fn quest_answer(steps: &[(bool, String)]) -> String {
    let steps: Vec<serde_json::Value> = steps
        .iter()
        .map(|(visit, name)| match visit {
            true => serde_json::json!({ "goal": "visit", "place": name }),
            false => serde_json::json!({ "goal": "meet", "npc": name }),
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

/// Plays each step, and moves the clock. A refused step is part of the game. The model
/// answers a talk and a quest at once.
fn run(story: &mut Story, plays: &[Play], clock: &mut u64) {
    for play in plays {
        *clock += if let Play::Wait(seconds) = play {
            *seconds
        } else {
            1
        };
        let Some(input) = input(play, Tick(*clock)) else {
            continue;
        };
        let mut outputs = story.handle(input).unwrap_or_default();
        if let Play::Quest(..) = play {
            outputs = story
                .handle(Input::BatchEnd { id: MessageId(3) })
                .unwrap_or_default();
        }
        let Some(Output::ModelCall { call, .. }) = outputs.first() else {
            continue;
        };
        let text = match play {
            Play::Talk(_, say) => serde_json::json!({ "say": say, "trust": 1 }).to_string(),
            Play::Quest(_, steps) => quest_answer(steps),
            _ => continue,
        };
        let _ = story.handle(Input::ModelAnswered { call: *call, text });
    }
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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

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
                Play::Zone(zone, subzone) => character.enter_zone(at, zone, subzone.as_deref()),
                Play::Meet(name) => character.meet_npc(at, name),
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
}
