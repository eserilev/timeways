//! Plays the bench moments through the real story program, behind the checks of the bridge,
//! into a scratch world, and times each model call (TESTING.md, "Testing the local model and
//! the frame rate"). It never opens a world of a real character.

use crate::model_runner::{Asked, Runner};
use crate::scenario::Scenario;
use crate::seed::{NO_DEADLINE, batch_text};
use fake_bridge::{FakeBridge, Reply};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;
use thiserror::Error;
use timeways_story::pack::Pack;
use timeways_story::store::{CharacterKey, Store, StoreError};
use timeways_story::story::Story;

/// The scratch character. No player of the game has this realm.
pub const REALM: &str = "Benchrealm";
pub const NAME: &str = "Bencher";

/// The bench moments that ship with the tool, each with one line about it. A version never
/// changes once it is out, so results compare over time.
pub const SETS: &[(&str, &str, &str)] = &[(
    "v1",
    "A human warrior from 18 to 20: arrival, deed, tenth level, dungeon setup, boss, tale, \
     revenge, chapter end (saga, summary, zone history), talk, and /lore.",
    include_str!("../bench/moments-v1.jsonl"),
)];

#[derive(Debug, Error)]
pub enum BenchError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("the scratch world: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// One model call of a measured batch.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// The number of the batch in the scenario.
    pub batch: usize,
    pub moment: String,
    pub kind: String,
    pub prompt: String,
    pub asked: Asked,
}

/// A call as the world keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub kind: String,
    pub prompt: Option<String>,
    pub accepted: bool,
}

/// What a play of the moments gave.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Played {
    pub calls: Vec<Call>,
    /// Each measured batch, by its number: its moment.
    pub moments: BTreeMap<usize, String>,
    /// The text that the player reads in the reply of a measured batch: the narrator line,
    /// the words of a talk, or a `/lore` answer.
    pub replies: BTreeMap<usize, String>,
    pub rows: Vec<Row>,
    /// The error of each batch that the bridge answered with one.
    pub failed_batches: Vec<String>,
}

#[derive(Default)]
struct Recorder {
    /// The measured batch that plays now.
    moment: Option<(usize, String)>,
    calls: Vec<Call>,
    /// Fixed answers of the scenario, by kind of call.
    answers: BTreeMap<String, VecDeque<String>>,
}

/// Plays `scenario` into a new world in `folder`. A call of a measured batch goes to the
/// runner until `deadline`, and fails after it. A call of any other batch fails at once.
///
/// # Errors
///
/// Returns the error of the scratch world.
pub fn play_bench(
    scenario: &Scenario,
    folder: &Path,
    pack: Pack,
    runner: &Runner,
    deadline: Option<Instant>,
) -> Result<Played, BenchError> {
    let mut story = Story::new(pack, Store::Folder(folder.to_path_buf()));
    story.set_events_deadline(NO_DEADLINE);
    let recorder = Rc::new(RefCell::new(Recorder::default()));
    let model = model_of(Rc::clone(&recorder), runner.clone(), deadline);
    let mut bridge = FakeBridge::new(story).with_model_by_kind(model);
    let character = json!({"type": "character_entered", "realm": REALM, "name": NAME});
    let mut played = Played::default();
    for (number, batch) in scenario.batches.iter().enumerate() {
        {
            let mut recorder = recorder.borrow_mut();
            recorder.moment = batch.moment.clone().map(|moment| (number, moment));
            for answer in &batch.answers {
                let queue = recorder.answers.entry(answer.kind.clone()).or_default();
                queue.push_back(answer.text.clone());
            }
        }
        let reply = bridge.batch(&batch_text(&character, batch));
        if let Reply::Error(error) = &reply {
            played.failed_batches.push(error.clone());
        }
        let Some(moment) = &batch.moment else {
            continue;
        };
        played.moments.insert(number, moment.clone());
        if let Some(text) = read_text(&reply) {
            played.replies.insert(number, text);
        }
    }
    drop(bridge);
    played.calls = std::mem::take(&mut recorder.borrow_mut().calls);
    played.rows = rows_of(&folder.join(CharacterKey::new(REALM, NAME)?.relative_path()))?;
    Ok(played)
}

fn model_of(
    recorder: Rc<RefCell<Recorder>>,
    runner: Runner,
    deadline: Option<Instant>,
) -> fake_bridge::ModelByKind {
    Box::new(move |kind, prompt| {
        let mut recorder = recorder.borrow_mut();
        let fixed = recorder.answers.get_mut(kind).and_then(VecDeque::pop_front);
        if fixed.is_some() {
            return fixed;
        }
        let (batch, moment) = recorder.moment.clone()?;
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return None;
        }
        let asked = runner.ask(prompt);
        let answer = asked.answer.clone();
        recorder.calls.push(Call {
            batch,
            moment,
            kind: kind.to_string(),
            prompt: prompt.to_string(),
            asked,
        });
        answer
    })
}

/// The narrator line of an `events_seen`, or the text of a talk or `/lore` answer.
fn read_text(reply: &Reply) -> Option<String> {
    let Reply::Done(text) = reply else {
        return None;
    };
    let value: Value = serde_json::from_str(text).ok()?;
    ["narrator", "text"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .map(str::to_string)
}

fn rows_of(world: &Path) -> Result<Vec<Row>, rusqlite::Error> {
    let connection = Connection::open_with_flags(world, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement =
        connection.prepare("SELECT kind, prompt, result FROM calls ORDER BY position")?;
    let rows = statement.query_map([], |row| {
        Ok(Row {
            kind: row.get(0)?,
            prompt: row.get(1)?,
            accepted: row.get::<_, String>(2)? == "accepted",
        })
    })?;
    rows.collect()
}
