//! Plays a scenario into a world folder, and reads the world back as the journal shows it.

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different part of the harness"
)]

use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use timeways_dev::scenario::{Scenario, built_in};
use timeways_dev::seed::{Model, Report, play};
use timeways_story::pack::Pack;
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

pub const REALM: &str = "Dev Realm";
pub const NAME: &str = "Testpal";
/// A fixed start, so two plays of one scenario hold the same times. It is in the past, as
/// the clock check of the story program needs.
pub const START: u64 = 1_780_000_000;

pub fn folder(name: &str) -> PathBuf {
    static CASE: AtomicUsize = AtomicUsize::new(0);
    let case = CASE.fetch_add(1, Ordering::Relaxed);
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("dev-{name}-{case}"));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

pub fn scenario(name: &str) -> Scenario {
    Scenario::parse(built_in(name).unwrap()).unwrap()
}

/// Plays the built-in scenario into `folder` with no model.
pub fn seed(name: &str, folder: &Path) -> Report {
    seed_with(name, folder, None)
}

pub fn seed_with(name: &str, folder: &Path, model: Option<Model>) -> Report {
    let story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.to_path_buf()));
    play(
        &scenario(name).starting_at(START),
        REALM,
        NAME,
        story,
        model,
    )
}

/// Every page of the journal of the world in `folder`, with each list joined.
pub fn journal(folder: &Path) -> Map<String, Value> {
    let mut story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.to_path_buf()));
    let character = json!({"type": "character_entered", "realm": REALM, "name": NAME});
    let _ = serve::line(&mut story, character.to_string().into_bytes());
    let mut joined = Map::new();
    let mut page = 0;
    loop {
        let ask = json!({"type": "journal_asked", "id": 1, "page": page});
        let served = serve::line(&mut story, ask.to_string().into_bytes());
        let Value::Object(object) = serde_json::from_str(&served.lines[0]).unwrap() else {
            panic!("a journal that is no object");
        };
        let pages = object["pages"].as_u64().unwrap();
        for (key, value) in object {
            match (joined.get_mut(&key), value) {
                (Some(Value::Array(list)), Value::Array(more)) => list.extend(more),
                (_, value) => {
                    joined.insert(key, value);
                }
            }
        }
        page += 1;
        if page >= pages {
            return joined;
        }
    }
}

pub fn list(journal: &Map<String, Value>, key: &str) -> Vec<Value> {
    journal
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// The text of each value of `key` in each item of the list.
pub fn texts(items: &[Value], key: &str) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.get(key).and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}
