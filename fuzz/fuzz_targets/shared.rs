//! A `timeways.sqlite` that another program damaged: rows of any type and body, other
//! versions, extra tables, and no `state` table. Opening it is Ok, or it refuses another
//! version, or it gives the error of SQLite. A value that does not read loads as its
//! default, and a value saved after a load reads back the same.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use timeways_story::narrator::Budget;
use timeways_story::pace::Pace;
use timeways_story::store::{Shared, StoreError};

const BUDGET: &str = "budget";
const PACE: &str = "pace";

#[derive(Arbitrary, Debug)]
enum Name {
    Budget,
    Pace,
    Other(String),
}

impl Name {
    fn text(&self) -> &str {
        match self {
            Name::Budget => BUDGET,
            Name::Pace => PACE,
            Name::Other(name) => name,
        }
    }
}

/// The body of a row, as any program can write it.
#[derive(Arbitrary, Debug)]
enum Body {
    Text(String),
    /// Bytes that SQLite keeps as text, even when they are not UTF-8.
    RawText(Vec<u8>),
    Blob(Vec<u8>),
    Integer(i64),
    Real(f64),
    /// A body in the form of the values, with times that can be too big or below zero.
    Times {
        times: Vec<Option<i128>>,
        failed: Option<i128>,
    },
}

#[derive(Arbitrary, Debug)]
struct File {
    /// None keeps the version of this build.
    version: Option<i64>,
    drop_state: bool,
    extra_tables: Vec<String>,
    rows: Vec<(Name, Body)>,
}

fn path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "timeways-fuzz-shared-{}.sqlite",
        std::process::id()
    ))
}

fn times_json(times: &[Option<i128>], failed: Option<i128>) -> String {
    let time = |time: Option<i128>| time.map_or("null".to_string(), |time| time.to_string());
    let times: Vec<String> = times.iter().map(|t| time(*t)).collect();
    let times = times.join(",");
    format!(
        r#"{{"spoken":[{times}],"opened":[{times}],"failed":{}}}"#,
        time(failed)
    )
}

/// The write of another program can fail, as a NaN in a column that is NOT NULL does.
fn insert(connection: &Connection, name: &str, body: &Body) {
    let replace = "INSERT OR REPLACE INTO state (name, body) VALUES (?1, ?2)";
    let _ = match body {
        Body::Text(text) => connection.execute(replace, params![name, text]),
        Body::RawText(bytes) => connection.execute(
            "INSERT OR REPLACE INTO state (name, body) VALUES (?1, CAST(?2 AS TEXT))",
            params![name, bytes],
        ),
        Body::Blob(bytes) => connection.execute(replace, params![name, bytes]),
        Body::Integer(number) => connection.execute(replace, params![name, number]),
        Body::Real(number) => connection.execute(replace, params![name, number]),
        Body::Times { times, failed } => {
            connection.execute(replace, params![name, times_json(times, *failed)])
        }
    };
}

fn damage(file: &File) {
    let connection = Connection::open(path()).unwrap();
    for table in &file.extra_tables {
        let table = table.replace('"', "\"\"");
        let _ = connection.execute_batch(&format!("CREATE TABLE IF NOT EXISTS \"{table}\" (x)"));
    }
    for (name, body) in &file.rows {
        insert(&connection, name.text(), body);
    }
    if file.drop_state {
        connection.execute_batch("DROP TABLE state").unwrap();
    }
    if let Some(version) = file.version {
        connection
            .execute_batch(&format!("PRAGMA user_version = {version}"))
            .unwrap();
    }
}

fn has_state() -> bool {
    let connection = Connection::open(path()).unwrap();
    let tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name = 'state'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    tables == 1
}

/// The value that a body of the file gives: the default for a body that does not read.
fn expected<T: DeserializeOwned + Default + Serialize>(name: &str) -> String {
    let connection = Connection::open(path()).unwrap();
    let body: Option<Vec<u8>> = connection
        .query_row(
            "SELECT CAST(body AS BLOB) FROM state WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )
        .ok();
    let value: T = body
        .and_then(|body| String::from_utf8(body).ok())
        .and_then(|body| serde_json::from_str(&body).ok())
        .unwrap_or_default();
    serde_json::to_string(&value).unwrap()
}

/// A load gives what the file holds, and a save of it after the load reads back the same.
fn check_round_trip<T: DeserializeOwned + Default + Serialize>(name: &'static str) {
    let want = expected::<T>(name);
    let mut shared = Shared::open(&path()).unwrap();
    let loaded: T = shared.load(name).unwrap();
    assert_eq!(serde_json::to_string(&loaded).unwrap(), want);

    shared.save(name, &loaded).unwrap();
    drop(shared);
    let again: T = Shared::open(&path()).unwrap().load(name).unwrap();
    assert_eq!(serde_json::to_string(&again).unwrap(), want);
}

fuzz_target!(|file: File| {
    let _ = std::fs::remove_file(path());
    drop(Shared::open(&path()).unwrap());
    damage(&file);

    match Shared::open(&path()) {
        Ok(_) => {}
        Err(StoreError::OtherVersion { .. } | StoreError::Sqlite { .. }) => return,
        Err(error) => panic!("an odd error: {error}"),
    }
    if !has_state() {
        let mut shared = Shared::open(&path()).unwrap();
        let load = shared.load::<Budget>(BUDGET);
        assert!(matches!(load, Err(StoreError::Sqlite { .. })), "{load:?}");
        return;
    }
    check_round_trip::<Budget>(BUDGET);
    check_round_trip::<Pace>(PACE);
});
