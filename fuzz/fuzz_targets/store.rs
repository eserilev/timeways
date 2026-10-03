//! Damaged rows in the database of a character. Opening it always works (GAMEPLAY.md 5.7).
//! Only a history from another program is refused. The first open cuts each row that does
//! not read, so a second open reads the database the same way.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use timeways_story::store::{CharacterKey, Opened, Store, StoreError, Table};

fn folder() -> PathBuf {
    std::env::temp_dir().join(format!("timeways-fuzz-store-{}", std::process::id()))
}

/// The counts of what each table gave.
fn counts(opened: &Opened) -> [usize; 8] {
    [
        opened.saved_events,
        opened.prose.len(),
        opened.flavor.moments().len(),
        opened.flavor.told().len(),
        opened.hero.changes().len(),
        opened.learned.read().len(),
        opened.learned.rumors().len(),
        opened.quests.changes().len(),
    ]
}

/// A body that is not UTF-8 goes in as a blob, as another program can write it.
fn insert(connection: &Connection, table: Table, body: &[u8]) {
    let insert = format!("INSERT INTO {} (body) VALUES (?1)", table.name());
    match std::str::from_utf8(body) {
        Ok(text) => connection.execute(&insert, params![text]),
        Err(_) => connection.execute(&insert, params![body]),
    }
    .unwrap();
}

fuzz_target!(|data: &[u8]| {
    let root = folder();
    let _ = std::fs::remove_dir_all(&root);
    let store = Store::Folder(root.clone());
    let key = CharacterKey::new("Stormrage", "Ada").unwrap();
    drop(store.open(&key).unwrap());
    let file = root.join("worlds").join("r_Stormrage").join("c_Ada.sqlite");
    let connection = Connection::open(file).unwrap();
    for (table, rows) in Table::ALL.into_iter().zip(data.split(|byte| *byte == 0xFF)) {
        for body in rows.split(|byte| *byte == b'\n') {
            insert(&connection, table, body);
        }
    }
    drop(connection);
    let first = match store.open(&key) {
        Ok(first) => first,
        Err(StoreError::Foreign { .. }) => return,
        Err(error) => panic!("a damaged row locked the character out: {error}"),
    };
    let second = store.open(&key).unwrap();
    assert_eq!(counts(&first), counts(&second));
});
