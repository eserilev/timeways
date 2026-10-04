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
use timeways_story::check::{Fault, check, later_names, mentions, without_citations};
use timeways_story::chronicle::{Pick, Saga};
use timeways_story::hero::{Field, LONG, checked_text, cut, limit_of};
use timeways_story::hero_hook::HOOK_FIELDS;
use timeways_story::house::fenced;
use timeways_story::input::{GameQuestKind, Input, MessageId, Reaction};
use timeways_story::journal::{Journal, journal, pages};
use timeways_story::npc_memory::{MAX_MEMORIES, MAX_MEMORY_CHARS, when};
use timeways_story::pace::{Pace, WINDOW_SECONDS};
use timeways_story::pack::Pack;
use timeways_story::passage_limits::{MAX_PASSAGE_BYTES, pieces};
use timeways_story::places::InstanceKind;
use timeways_story::quest::variety::{Recent, SHAPES_TO_AVOID, Shape, TITLES_TO_AVOID, main_words};
use timeways_story::quest::{
    AnyOrder, DAY_SECONDS, Known, MAX_KILLS, MAX_WAIT_DAYS, QuestChange, Status, Step, Tracked,
    checked_quest, quest_log,
};
use timeways_story::reply_size::{MAX_LINE, MAX_SLOT, Size};
use timeways_story::seen::TextKind;
use timeways_story::spot::{MAP_IDS, Spot, THOUSANDTHS, spot_of};
use timeways_story::store::{CharacterKey, Node, Store, Table, safe_id};
use timeways_story::story::{Output, Story};
use timeways_story::vocabulary::{DEATHS, DEFEATED, MET};
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
    /// The end of a batch. The model answers each call of it, and each call that follows,
    /// with these words: the narrator line, the drafts of a saga, and the pick.
    EndBatch(String),
    /// A story that a player of the party told, accepted, with its number in the addon.
    StoryAccept(u64, String),
    StoryRemove(u64),
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
        (0usize..FIELDS.len(), hero_text()).prop_map(|(field, text)| Play::HeroSet(field, text)),
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
        prop::sample::select(vec![
            "The road remembers you.".to_string(),
            r#"{"saga": "Our hero walked on.", "pick": 2}"#.to_string(),
            "not an answer".to_string(),
        ])
        .prop_map(Play::EndBatch),
        (0u64..4, "Zqstory [A-Za-z ]{1,60}")
            .prop_map(|(number, text)| Play::StoryAccept(number, text)),
        (0u64..4).prop_map(Play::StoryRemove),
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

/// A meet step, a kill step whose count sits often at the edges of its band, or a wait.
fn quest_step() -> impl Strategy<Value = Step> {
    let count = prop_oneof![Just(1u8), Just(MAX_KILLS), 1..=MAX_KILLS];
    prop_oneof![
        Just(Step::Meet {
            npc: "Farmer Bram".to_string(),
        }),
        count.prop_map(|count| Step::Kill {
            creature: "Duskbat".to_string(),
            count,
        }),
        (1..=MAX_WAIT_DAYS).prop_map(|days| Step::Wait { days }),
    ]
}

/// A time near the end of a wait that opened at the accept (2) or at a step done at 3: a
/// second before it, at it, or a second after it.
fn quest_time() -> impl Strategy<Value = Tick> {
    let edge = (2u64..=3, 0..=u64::from(MAX_WAIT_DAYS), 0u64..=2);
    edge.prop_map(|(base, days, delta)| Tick(base + days * DAY_SECONDS + delta - 1))
}

/// No span, or a span at the start, in the middle, or at the end of a quest of 4 steps.
/// A span past the steps of a shorter quest comes too, as in a damaged file.
fn any_order_span() -> impl Strategy<Value = Option<AnyOrder>> {
    prop_oneof![
        Just(None),
        Just(Some(AnyOrder { first: 0, last: 1 })),
        Just(Some(AnyOrder { first: 1, last: 2 })),
        Just(Some(AnyOrder { first: 2, last: 3 })),
        Just(Some(AnyOrder { first: 1, last: 3 })),
    ]
}

/// A step for the variety check: few targets, so shapes and targets repeat often.
fn variety_step() -> impl Strategy<Value = serde_json::Value> {
    prop::sample::select(vec![
        serde_json::json!({"goal": "visit", "place": "Old Tower"}),
        serde_json::json!({"goal": "visit", "place": "Testvale"}),
        serde_json::json!({"goal": "meet", "npc": "Farmer Bram"}),
        serde_json::json!({"goal": "talk", "npc": "Farmer Bram"}),
    ])
}

/// An offer of the log, with a shape of one or two visits and meetings.
fn recent_offer() -> impl Strategy<Value = Recent> {
    let step = prop::sample::select(vec![
        Step::Visit {
            place: "Old Tower".to_string(),
        },
        Step::Meet {
            npc: "Farmer Bram".to_string(),
        },
    ]);
    let title = prop::sample::select(vec!["Old Debts", "The Tower", "Rats", "A Task"]);
    (prop::collection::vec(step, 1..3), title).prop_map(|(steps, title)| Recent {
        number: 1,
        title: title.to_string(),
        shape: Shape::of(&steps, None),
        genre: None,
    })
}

/// Few numbers and short quests, so that most changes find their quest. Kills come often,
/// so a kill step can fill up.
fn quest_change() -> impl Strategy<Value = QuestChange> {
    let number = 1u64..4;
    let steps = prop::collection::vec(quest_step(), 0..5);
    prop_oneof![
        1 => (number.clone(), steps, any_order_span()).prop_map(move |(number, steps, any_order)| {
            QuestChange::Offered {
                number,
                at: Tick(1),
                giver: "Keeper Tessa".to_string(),
                title: "A Task".to_string(),
                text: "Go.".to_string(),
                steps,
                any_order,
                genre: None,
            }
        }),
        1 => number.clone().prop_map(|number| QuestChange::Accepted {
            number,
            at: Tick(2)
        }),
        1 => number.clone().prop_map(|number| QuestChange::Declined {
            number,
            at: Tick(2)
        }),
        1 => number.clone().prop_map(|number| QuestChange::Abandoned {
            number,
            at: Tick(2)
        }),
        2 => (number.clone(), 0usize..5, quest_time()).prop_map(|(number, step, at)| {
            QuestChange::StepDone { number, step, at }
        }),
        3 => (number, 0usize..2).prop_map(|(number, step)| QuestChange::Killed {
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

use timeways_story::hero::FIELDS;

fn input(play: &Play, at: Tick) -> Option<Input> {
    Some(match play.clone() {
        Play::Zone(zone, subzone, spot) => Input::ZoneEntered {
            at,
            zone,
            subzone,
            spot,
            hour: None,
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
        Play::EndBatch(_) => Input::BatchEnd { id: MessageId(4) },
        Play::StoryAccept(number, text) => Input::StoryAccepted { at, number, text },
        Play::StoryRemove(number) => Input::StoryRemoved { at, number },
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
    serde_json::json!({ "title": "A Task", "genre": "errand", "text": "I need you to go.", "steps": steps }).to_string()
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
    if let Play::EndBatch(text) = play {
        return answer_every_call(story, outputs, text);
    }
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
        // A refused answer gets a retry, and the model answers it the same way.
        Play::Quest(_, steps) => return answer_every_call(story, outputs, &quest_answer(steps)),
        _ => return Vec::new(),
    };
    let call = *call;
    story
        .handle(Input::ModelAnswered { call, text })
        .unwrap_or_default()
}

/// Answers each call of the outputs, and each call that the answers open, so no call stays
/// open after the step.
fn answer_every_call(story: &mut Story, mut outputs: Vec<Output>, text: &str) -> Vec<Output> {
    for _ in 0..8 {
        let calls: Vec<_> = outputs
            .iter()
            .filter_map(|output| match output {
                Output::ModelCall { call, .. } => Some(*call),
                _ => None,
            })
            .collect();
        if calls.is_empty() {
            break;
        }
        outputs = Vec::new();
        for call in calls {
            let text = text.to_string();
            outputs.extend(
                story
                    .handle(Input::ModelAnswered { call, text })
                    .unwrap_or_default(),
            );
        }
    }
    outputs
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
            // An NPC that talks to you in the game is a friend now (`befriend`).
            Play::Meet(npc, _) => {
                self.met.push(npc.clone());
                if let Some((reaction, _)) = self.seen.get_mut(npc) {
                    *reaction = Reaction::Friendly;
                }
            }
            Play::Slap(npc) | Play::Talk(npc, _) | Play::Quest(npc, _) => {
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

/// The giver of an offer that came back as the notice of its batch, or None.
fn offer_giver(outputs: &[Output]) -> Option<&str> {
    let Some(Output::EventsSeen {
        notice: Some(line), ..
    }) = outputs.first()
    else {
        return None;
    };
    line.split_once(" has a quest for you: ")
        .map(|(giver, _)| giver)
}

/// Every page of the journal as the bridge gets it.
fn world_file(folder: &Path) -> PathBuf {
    folder
        .join("worlds")
        .join("r_Stormrage")
        .join("c_Ada.sqlite")
}

fn rows_of(connection: &rusqlite::Connection, select: &str) -> Vec<Vec<Option<String>>> {
    let mut statement = connection.prepare(select).unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| {
            (0..columns)
                .map(|column| {
                    let value: rusqlite::types::Value = row.get(column)?;
                    Ok(match value {
                        rusqlite::types::Value::Null => None,
                        rusqlite::types::Value::Integer(n) => Some(n.to_string()),
                        rusqlite::types::Value::Text(text) => Some(text),
                        other => Some(format!("{other:?}")),
                    })
                })
                .collect()
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Every link of the world, but the prompts: a prompt holds samples picked by the number
/// of the call in the run, and a restart starts that number again.
fn links_of(folder: &Path) -> Vec<Vec<Option<String>>> {
    let connection = rusqlite::Connection::open(world_file(folder)).unwrap();
    let mut links = rows_of(
        &connection,
        "SELECT 'input', position, kind, root, body FROM inputs ORDER BY position",
    );
    links.extend(rows_of(
        &connection,
        "SELECT 'call', position, kind, input, call, answer, result FROM calls ORDER BY position",
    ));
    links.extend(rows_of(
        &connection,
        "SELECT 'read', call, tab, row FROM reads ORDER BY call, tab, row",
    ));
    for table in Table::ALL {
        let select = format!(
            "SELECT '{}', position, input, call FROM {} ORDER BY position",
            table.name(),
            table.name()
        );
        links.extend(rows_of(&connection, &select));
    }
    links
}

/// Each row and call of the world, as a node of the graph.
fn nodes_of(folder: &Path) -> Vec<Node> {
    let connection = rusqlite::Connection::open(world_file(folder)).unwrap();
    let mut nodes = Vec::new();
    for table in Table::ALL {
        let select = format!("SELECT position FROM {}", table.name());
        for row in rows_of(&connection, &select) {
            let position = row[0].as_deref().unwrap().parse().unwrap();
            nodes.push(Node::Row(table, position));
        }
    }
    for row in rows_of(&connection, "SELECT position FROM calls") {
        nodes.push(Node::Call(row[0].as_deref().unwrap().parse().unwrap()));
    }
    nodes
}

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
/// A text of about `len` bytes, from a pattern of words and sentence ends. A length at
/// the limit of the bridge comes often.
fn passage_text() -> impl Strategy<Value = String> {
    let len = prop_oneof![
        Just(MAX_PASSAGE_BYTES),
        Just(MAX_PASSAGE_BYTES + 1),
        Just(2 * MAX_PASSAGE_BYTES + 1),
        0..3 * MAX_PASSAGE_BYTES,
    ];
    (len, "[a-zé .!?\n]{1,60}").prop_map(|(len, pattern)| {
        let text = pattern.repeat(len / pattern.len() + 1);
        let end = (0..=len)
            .rev()
            .find(|end| text.is_char_boundary(*end))
            .unwrap_or(0);
        text[..end].to_string()
    })
}

/// With no spaces or invisible characters, and each look-alike angle as its ASCII angle.
fn as_a_model_reads_it(text: &str) -> String {
    let invisible = ['\u{200B}', '\u{2060}', '\u{FEFF}', '\u{00AD}', '\u{FE0F}'];
    let seen = text
        .chars()
        .filter(|c| !c.is_whitespace() && !invisible.contains(c));
    seen.map(|c| match c {
        '＜' => '<',
        '＞' | '﹥' | '›' | '»' | '〉' | '⟩' => '>',
        other => other,
    })
    .collect()
}

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
    fn every_link_points_to_a_row_that_exists_and_came_before_it(plays in prop::collection::vec(play(), 0..80)) {
        let folder = fresh("links-exist");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let connection = rusqlite::Connection::open(world_file(&folder)).unwrap();
        let broken: i64 = connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| row.get(0)).unwrap();
        prop_assert_eq!(broken, 0);
        for read in rows_of(&connection, "SELECT call, tab, row FROM reads") {
            let (call, tab, row) = (read[0].clone().unwrap(), read[1].clone().unwrap(), read[2].clone().unwrap());
            let table = if tab == "calls" { "calls".to_string() } else { tab.clone() };
            let found: i64 = connection
                .query_row(&format!("SELECT count(*) FROM {table} WHERE position = ?1"), [&row], |found| found.get(0))
                .unwrap();
            prop_assert_eq!(found, 1, "call {} reads {} {}, which does not exist", call, tab, row);
            if tab == "calls" {
                prop_assert!(row.parse::<i64>().unwrap() < call.parse::<i64>().unwrap());
            }
        }
        let later_parent: i64 = connection
            .query_row("SELECT count(*) FROM calls WHERE call IS NOT NULL AND call >= position", [], |row| row.get(0))
            .unwrap();
        prop_assert_eq!(later_parent, 0);
    }

    /// In this first version, no story reaches a model (GAMEPLAY.md 4.8), and each one rests
    /// on the word of another player.
    #[test]
    fn no_prompt_holds_a_story_and_each_story_rests_on_another_player(plays in prop::collection::vec(play(), 0..80)) {
        let folder = fresh("stories");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let connection = rusqlite::Connection::open(world_file(&folder)).unwrap();
        let in_prompts: i64 = connection
            .query_row("SELECT count(*) FROM calls WHERE prompt LIKE '%Zqstory%'", [], |row| row.get(0))
            .unwrap();
        prop_assert_eq!(in_prompts, 0);
        let key = CharacterKey::new("Stormrage", "Ada").unwrap();
        let database = Store::Folder(folder.clone()).open(&key).unwrap().database;
        for node in nodes_of(&folder).into_iter().filter(|node| matches!(node, Node::Row(Table::Stories, _))) {
            let proof = database.proof_of(node).unwrap();
            let shared = proof == std::collections::BTreeSet::from([timeways_story::store::Root::Shared]);
            let player = proof == std::collections::BTreeSet::from([timeways_story::store::Root::Player]);
            prop_assert!(shared || player, "{:?} rests on {:?}", node, proof);
        }
    }

    #[test]
    fn every_event_rests_on_a_line_or_a_call(plays in prop::collection::vec(play(), 0..80)) {
        let folder = fresh("events-rest");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let connection = rusqlite::Connection::open(world_file(&folder)).unwrap();
        let bare: i64 = connection
            .query_row("SELECT count(*) FROM events WHERE input IS NULL AND call IS NULL", [], |row| row.get(0))
            .unwrap();
        prop_assert_eq!(bare, 0);
    }

    #[test]
    fn the_proof_of_each_row_is_the_same_after_a_reopen(plays in prop::collection::vec(play(), 0..60)) {
        let folder = fresh("proof-reopen");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);
        let key = CharacterKey::new("Stormrage", "Ada").unwrap();
        let nodes = nodes_of(&folder);
        let proofs = |database: &timeways_story::store::Database| -> Vec<_> {
            nodes.iter().map(|node| database.proof_of(*node).unwrap()).collect()
        };
        let before = proofs(&Store::Folder(folder.clone()).open(&key).unwrap().database);

        let after = proofs(&Store::Folder(folder.clone()).open(&key).unwrap().database);

        prop_assert_eq!(before, after);
    }

    /// The memory of one run, such as a saga that failed in this run, can make the calls
    /// after a restart differ from one long run. A saved link never changes.
    #[test]
    fn a_restart_at_any_point_keeps_every_saved_link(
        plays in prop::collection::vec(play(), 0..60),
        restarts in prop::collection::vec(any::<prop::sample::Index>(), 0..4),
    ) {
        let folder = fresh("links-restarts");
        let mut cuts: Vec<usize> = restarts.iter().map(|index| index.index(plays.len() + 1)).collect();
        cuts.sort_unstable();
        let mut clock = 1_000;
        let mut restarted = story(&folder, Store::Folder(folder.clone()));
        let mut from = 0;
        let mut saved = Vec::new();
        for cut in cuts.into_iter().chain([plays.len()]) {
            run(&mut restarted, &plays[from..cut], &mut clock);
            from = cut;
            drop(restarted);
            saved.push(links_of(&folder));
            restarted = story(&folder, Store::Folder(folder.clone()));
        }
        run(&mut restarted, &plays[..plays.len().min(5)], &mut clock);
        drop(restarted);

        let last = links_of(&folder);
        for links in saved {
            for link in links {
                prop_assert!(last.contains(&link), "a restart lost {:?}", link);
            }
        }
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
            for list in ["chapters", "places", "people", "deeds", "learned", "quests", "stories"] {
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
    fn the_pages_joined_hold_the_whole_sheet_and_each_page_fits(
        texts in prop::collection::vec(hero_text(), FIELDS.len())
    ) {
        let mut whole = Journal::default();
        for (field, text) in FIELDS.iter().zip(texts) {
            let limit = limit_of(field).unwrap();
            let Ok(text) = checked_text(&text, limit) else { continue };
            whole.hero.sheet.push(Field { field: (*field).to_string(), text });
        }

        let pages = pages(whole.clone());

        let budget = Size { line: MAX_LINE, slot: MAX_SLOT };
        let joined: Vec<Field> = pages.iter().flat_map(|page| page.journal.hero.sheet.clone()).collect();
        prop_assert_eq!(joined, whole.hero.sheet);
        for page in &pages {
            prop_assert!(Size::of(page).fits(budget));
        }
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
        prop_assume!(text.chars().count() <= LONG.chars && text.len() <= LONG.bytes);

        prop_assert_eq!(checked_text(&text, LONG), Ok(text.clone()));
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
    fn no_offer_repeats_the_shape_of_the_last_two(
        recent in prop::collection::vec(recent_offer(), 0..4),
        steps in prop::collection::vec(variety_step(), 1..4),
        title in prop::sample::select(vec!["Old Debts", "The Tower", "Rats", "A Task"]),
    ) {
        let known = Known {
            giver: "Keeper Tessa",
            zones: vec!["Testvale"],
            subzones: vec!["Old Tower"],
            npcs: vec!["Farmer Bram"],
            recent: recent.clone(),
            ..Known::default()
        };
        let text = serde_json::json!({
            "title": title, "genre": "errand", "text": "I need you.", "steps": steps,
        });

        let Ok(quest) = checked_quest(&text.to_string(), &known) else {
            return Ok(());
        };

        let shape = Shape::of(&quest.steps, quest.any_order);
        prop_assert!(recent.iter().take(SHAPES_TO_AVOID).all(|old| old.shape != shape));
        let words = main_words(&quest.title);
        for old in recent.iter().take(TITLES_TO_AVOID) {
            prop_assert!(main_words(&old.title).iter().all(|word| !words.contains(word)));
        }
    }

    #[test]
    fn every_offer_in_play_differs_in_shape_from_the_two_before_it(
        plays in prop::collection::vec(target_play(), 0..150)
    ) {
        let folder = fresh("variety");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let key = CharacterKey::new("Stormrage", "Ada").unwrap();
        let opened = Store::Folder(folder).open(&key).unwrap();
        let shapes: Vec<Shape> = quest_log(opened.quests.changes())
            .iter()
            .map(|quest| Shape::of(&quest.steps, quest.any_order))
            .collect();
        for (n, shape) in shapes.iter().enumerate() {
            let before = &shapes[n.saturating_sub(SHAPES_TO_AVOID)..n];
            prop_assert!(!before.contains(shape), "{:?}", shapes);
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

    /// Mostly angle marks, look-alikes, spaces, and invisible characters, so that broken,
    /// joined, and hidden fence marks come often.
    #[test]
    fn no_input_can_close_its_fence(
        text in "[<>＜＞﹥›»〉⟩ \t\u{200B}\u{2060}\u{FEFF}\u{00AD}\u{FE0F}a]{0,24}",
    ) {
        let fenced = fenced(&text);

        let inside = as_a_model_reads_it(&fenced[4..fenced.len() - 4]);
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
            prop_assert!(quest.steps_done() <= quest.steps.len());
            let finished = !quest.steps.is_empty() && quest.steps_done() == quest.steps.len();
            prop_assert_eq!(quest.status == Status::Done, finished);
            prop_assert_eq!(quest.done_at.is_some(), finished);
        }
    }

    #[test]
    fn in_an_ordered_quest_no_step_is_done_before_the_step_before_it(
        changes in prop::collection::vec(quest_change(), 0..40),
    ) {
        for quest in quest_log(&changes).into_iter().filter(|quest| quest.any_order.is_none()) {
            let first_gap = quest.done.iter().position(Option::is_none).unwrap_or(quest.done.len());
            prop_assert!(quest.done[first_gap..].iter().all(Option::is_none), "{:?}", quest);
        }
    }

    #[test]
    fn an_any_order_set_is_done_only_when_all_its_steps_are_done(
        changes in prop::collection::vec(quest_change(), 0..40),
    ) {
        for quest in quest_log(&changes) {
            let Some(span) = quest.any_order else {
                continue;
            };
            let set_done = (span.first..=span.last).all(|step| quest.is_done(step));
            let later_done = (span.last + 1..quest.steps.len()).any(|step| quest.is_done(step));
            prop_assert!(set_done || !later_done, "{:?}", quest);
            let before_done = (0..span.first).all(|step| quest.is_done(step));
            let set_started = (span.first..=span.last).any(|step| quest.is_done(step));
            prop_assert!(before_done || !set_started, "{:?}", quest);
        }
    }

    #[test]
    fn a_wait_never_ends_early(changes in prop::collection::vec(quest_change(), 0..40)) {
        for quest in quest_log(&changes) {
            for (step, done) in quest.done.iter().enumerate() {
                let (Step::Wait { days }, Some(done)) = (&quest.steps[step], done) else {
                    continue;
                };
                let opened = quest.done[..step].iter().flatten().max().copied().or(quest.accepted_at);
                let ready = opened.unwrap().0 + u64::from(*days) * DAY_SECONDS;
                prop_assert!(done.0 >= ready, "{:?}", quest);
            }
        }
    }

    #[test]
    fn kills_count_only_for_an_open_kill_step_and_never_past_its_count(
        changes in prop::collection::vec(quest_change(), 0..60),
    ) {
        let mut before: Vec<Tracked> = Vec::new();
        for end in 0..=changes.len() {
            let quests = quest_log(&changes[..end]);

            for (index, quest) in quests.iter().enumerate() {
                let never_accepted = matches!(quest.status, Status::Offered | Status::Declined);
                for (step, kills) in quest.kills.iter().enumerate() {
                    let limit = match &quest.steps[step] {
                        Step::Kill { count, .. } if !never_accepted => *count,
                        _ => 0,
                    };
                    prop_assert!(*kills <= limit, "{:?}", quest);
                    let old = before.get(index).map_or(0, |old| old.kills[step]);
                    let counted = *kills > old;
                    let was_open = before.get(index).is_some_and(|old| old.is_open(step));
                    prop_assert!(!counted || was_open, "{:?}", quest);
                }
            }
            before = quests;
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
    fn every_piece_of_a_passage_fits_the_bridge_and_no_word_is_lost(text in passage_text()) {
        let cut = pieces(&text);

        for piece in &cut {
            prop_assert!(!piece.is_empty() && piece.len() <= MAX_PASSAGE_BYTES, "{}", piece.len());
        }
        let letters = |text: &str| text.split_whitespace().collect::<String>();
        prop_assert_eq!(letters(&cut.concat()), letters(&text));
        prop_assert_eq!(cut.len() == 1, text.trim().len() <= MAX_PASSAGE_BYTES && !text.trim().is_empty());
    }

    #[test]
    fn text_without_markup_stays_as_it_is(text in "[a-zA-Z0-9 .,;:?\n]{0,200}") {
        prop_assert_eq!(plain(&text), text);
    }
}

/// `play()`, and also waits at each edge of the age words, talks to Farley that name the
/// character one time in four, quests of Farley that the check lets through, and answers
/// of the sheet that a hook takes.
fn memory_play() -> impl Strategy<Value = Play> {
    let edge_wait = prop::sample::select(vec![HOUR, DAY, 2 * DAY, 7 * DAY, 30 * DAY, 365 * DAY])
        .prop_map(Play::Wait);
    let farley_says = prop_oneof![
        1 => Just("Well met, Ada.".to_string()),
        3 => "[A-Za-z ]{1,60}",
    ];
    let farley_talk = farley_says.prop_map(|say| Play::Talk(FARLEY.to_string(), say));
    let farley_quest = prop::sample::select(vec!["Goldshire", "Westfall"]).prop_map(|place| {
        Play::Quest(FARLEY.to_string(), vec![TaskStep::Visit(place.to_string())])
    });
    let hook_field = prop::sample::select(vec!["goal", "flaw"])
        .prop_map(|field| FIELDS.iter().position(|name| *name == field).unwrap());
    let hook_set = (hook_field, hero_text()).prop_map(|(field, text)| Play::HeroSet(field, text));
    prop_oneof![
        6 => play(),
        1 => edge_wait,
        2 => farley_talk,
        1 => farley_quest,
        3 => hook_set,
    ]
}

const FARLEY: &str = "Innkeeper Farley";
const HOUR: u64 = 3_600;
const DAY: u64 = 24 * HOUR;

/// A talk call as the database keeps it: its prompt, and the body of each row it read.
struct TalkCall {
    prompt: String,
    reads: Vec<(String, String)>,
}

fn talk_calls(folder: &Path) -> Vec<TalkCall> {
    let connection = rusqlite::Connection::open(world_file(folder)).unwrap();
    let calls = rows_of(
        &connection,
        "SELECT position, prompt FROM calls WHERE kind = 'talk' AND prompt IS NOT NULL",
    );
    calls
        .into_iter()
        .map(|call| {
            let position = call[0].clone().unwrap();
            let select = format!("SELECT tab, row FROM reads WHERE call = {position}");
            let reads = rows_of(&connection, &select)
                .into_iter()
                .filter_map(|read| {
                    let (tab, row) = (read[0].clone()?, read[1].clone()?);
                    let select = format!("SELECT body FROM {tab} WHERE position = {row}");
                    let body = rows_of(&connection, &select).first()?[0].clone()?;
                    Some((tab, body))
                })
                .collect();
            TalkCall {
                prompt: call[1].clone().unwrap(),
                reads,
            }
        })
        .collect()
}

/// The lines of the memory fence of a talk prompt, without their dashes.
fn memory_lines(prompt: &str) -> Vec<&str> {
    let Some((_, block)) = prompt.split_once("Each line is true:\n<<<\n") else {
        return Vec::new();
    };
    let (block, _) = block.split_once("\n>>>").unwrap();
    block
        .lines()
        .map(|line| line.strip_prefix("- ").unwrap())
        .collect()
}

fn npc_of(prompt: &str) -> &str {
    prompt
        .lines()
        .find_map(|line| line.strip_prefix("Name: "))
        .unwrap()
}

fn read_values<'a>(
    call: &'a TalkCall,
    tab: &'a str,
) -> impl Iterator<Item = serde_json::Value> + 'a {
    call.reads
        .iter()
        .filter(move |(read_tab, _)| read_tab == tab)
        .map(|(_, body)| serde_json::from_str(body).unwrap())
}

fn read_events(call: &TalkCall) -> impl Iterator<Item = hourglass::Event> + '_ {
    call.reads
        .iter()
        .filter(|(tab, _)| tab == "events")
        .map(|(_, body)| serde_json::from_str(body).unwrap())
}

/// Does a row that the call read say what the line says?
fn rests_on_a_read(line: &str, npc: &str, call: &TalkCall, character: &Character) -> bool {
    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some((_, quoted)) = line.split_once("you told the player \"") {
        let said = quoted.strip_suffix('"').unwrap();
        let said = said.strip_suffix("...").unwrap_or(said);
        return read_values(call, "learned").any(|rumor| {
            rumor["line"] == "rumor"
                && rumor["npc"] == npc
                && words(rumor["text"].as_str().unwrap()).starts_with(said)
        });
    }
    if let Some((_, quoted)) = line.split_once("your quest \"") {
        let (title, _) = quoted.rsplit_once("\". ").unwrap();
        return read_values(call, "quests").any(|offer| {
            offer["line"] == "offered" && offer["giver"] == npc && offer["title"] == title
        });
    }
    if line.ends_with("for the first time.") {
        return read_events(call).any(|event| match event.kind {
            hourglass::EventKind::FactStart {
                name,
                linked_to: Some(npc_id),
                ..
            } => {
                name == MET
                    && character
                        .world()
                        .entity(npc_id)
                        .is_some_and(|e| e.name == npc)
            }
            _ => false,
        });
    }
    read_events(call).any(
        |event| matches!(event.kind.fact_name(), Some(name) if name == DEFEATED || name == DEATHS),
    )
}

const HOOK_HEADING: &str = "Something the player wrote about their hero, as ";

/// The talk and quest calls, oldest first: the position and the prompt of each.
fn hook_calls(connection: &rusqlite::Connection) -> Vec<(String, String)> {
    let select = "SELECT position, prompt FROM calls WHERE kind IN ('talk', 'quest') \
                  ORDER BY position";
    rows_of(connection, select)
        .into_iter()
        .map(|call| (call[0].clone().unwrap(), call[1].clone().unwrap()))
        .collect()
}

/// The fenced words of the hook block of a prompt.
fn hook_text(prompt: &str) -> Option<&str> {
    let (_, block) = prompt.split_once(HOOK_HEADING)?;
    let (_, fenced) = block.split_once("<<<\n")?;
    Some(fenced.split_once("\n>>>")?.0)
}

/// The input line that opened a call, through the call that opened it when it has none.
fn input_of_call(connection: &rusqlite::Connection, call: &str) -> String {
    let select = format!("SELECT input, call FROM calls WHERE position = {call}");
    let row = rows_of(connection, &select).remove(0);
    match (&row[0], &row[1]) {
        (Some(input), _) => input.clone(),
        (None, Some(parent)) => input_of_call(connection, parent),
        (None, None) => panic!("call {call} rests on nothing"),
    }
}

/// The newest `Set` row of each hook field that the lines before `input` wrote.
fn newest_sets_before(connection: &rusqlite::Connection, input: &str) -> HashMap<String, String> {
    let select = format!("SELECT position, body FROM hero WHERE input < {input} ORDER BY position");
    let mut newest = HashMap::new();
    for row in rows_of(connection, &select) {
        let change: serde_json::Value = serde_json::from_str(row[1].as_deref().unwrap()).unwrap();
        if change["line"] == "set" && HOOK_FIELDS.contains(&change["field"].as_str().unwrap()) {
            newest.insert(
                change["field"].as_str().unwrap().to_string(),
                row[0].clone().unwrap(),
            );
        }
    }
    newest
}

/// A tick, with the ends of `u64` and the edges of each band of age words likely.
fn memory_tick() -> impl Strategy<Value = u64> {
    let edges: Vec<u64> = [
        HOUR,
        DAY,
        2 * DAY,
        7 * DAY,
        14 * DAY,
        30 * DAY,
        60 * DAY,
        365 * DAY,
    ]
    .into_iter()
    .flat_map(|edge| [edge - 1, edge, edge + 1])
    .chain([0, u64::MAX])
    .collect();
    prop_oneof![prop::sample::select(edges), any::<u64>(), 0..400 * DAY]
}

/// Every phrase of GAMEPLAY.md 3.5 for the age of a memory.
fn age_words() -> Vec<String> {
    let numbers = [
        "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven", "Twelve",
    ];
    let mut words: Vec<String> = [
        "Less than an hour ago",
        "Hours ago",
        "Yesterday",
        "A week ago",
        "A month ago",
        "Over a year ago",
    ]
    .map(String::from)
    .to_vec();
    for number in numbers {
        for unit in ["days", "weeks", "months"] {
            words.push(format!("{number} {unit} ago"));
        }
    }
    words
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn every_memory_in_a_talk_prompt_rests_on_a_row_that_the_call_read(
        plays in prop::collection::vec(memory_play(), 0..80),
    ) {
        let folder = fresh("memories-read");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let key = CharacterKey::new("Stormrage", "Ada").unwrap();
        let character = Store::Folder(folder.clone()).open(&key).unwrap().character;
        for call in talk_calls(&folder) {
            let npc = npc_of(&call.prompt);
            for line in memory_lines(&call.prompt) {
                prop_assert!(rests_on_a_read(line, npc, &call, &character), "{} rests on no read", line);
            }
        }
    }

    #[test]
    fn a_talk_prompt_holds_at_most_five_memories_and_never_the_name_of_the_character(
        plays in prop::collection::vec(memory_play(), 0..80),
    ) {
        let folder = fresh("memories-limits");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        for call in talk_calls(&folder) {
            let lines = memory_lines(&call.prompt);
            prop_assert!(lines.len() <= MAX_MEMORIES, "{:?}", lines);
            for line in lines {
                prop_assert!(line.chars().count() <= MAX_MEMORY_CHARS, "{}", line);
                let said = line.contains("you told the player");
                prop_assert!(!(said && mentions(line, "Ada")), "{}", line);
            }
        }
    }

    #[test]
    fn at_most_one_talk_or_quest_call_in_three_has_a_hook(
        plays in prop::collection::vec(memory_play(), 0..80),
    ) {
        let folder = fresh("hook-rhythm");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let connection = rusqlite::Connection::open(world_file(&folder)).unwrap();
        for (index, (_, prompt)) in hook_calls(&connection).iter().enumerate() {
            let hooked = prompt.contains(HOOK_HEADING);
            prop_assert!(!hooked || index % 3 == 2, "call {} of the count has a hook", index + 1);
        }
    }

    #[test]
    fn every_hook_is_the_words_of_the_player_and_its_row_is_read(
        plays in prop::collection::vec(memory_play(), 0..80),
    ) {
        let folder = fresh("hook-read");
        let mut clock = 1_000;
        let mut story = story(&folder, Store::Folder(folder.clone()));
        run(&mut story, &plays, &mut clock);
        drop(story);

        let connection = rusqlite::Connection::open(world_file(&folder)).unwrap();
        for (call, prompt) in hook_calls(&connection) {
            let Some(text) = hook_text(&prompt) else {
                continue;
            };
            let select = format!(
                "SELECT hero.position, hero.body FROM reads JOIN hero ON hero.position = reads.row \
                 WHERE reads.call = {call} AND reads.tab = 'hero'"
            );
            let newest = newest_sets_before(&connection, &input_of_call(&connection, &call));
            let read_sets = rows_of(&connection, &select).into_iter().filter_map(|row| {
                let change: serde_json::Value = serde_json::from_str(row[1].as_deref()?).ok()?;
                let field = change["field"].as_str()?.to_string();
                let words = change["line"] == "set" && cut(change["text"].as_str()?) == text;
                words.then(|| (field, row[0].clone()))
            });
            let source: Vec<(String, Option<String>)> = read_sets.collect();
            prop_assert!(!source.is_empty(), "call {} reads no row with its hook {:?}", call, text);
            let newest_read = source.iter().any(|(field, row)| newest.get(field) == row.as_ref());
            prop_assert!(newest_read, "call {} reads an old answer", call);
        }
    }

    #[test]
    fn the_age_of_a_memory_is_always_words(then in memory_tick(), now in memory_tick()) {
        let words = when(Tick(then), Tick(now));

        prop_assert!(!words.chars().any(|c| c.is_ascii_digit()), "{}", words);
        prop_assert!(age_words().contains(&words), "{}", words);
    }
}
