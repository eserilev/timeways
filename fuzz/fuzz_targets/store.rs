//! A database of a character that another program damaged: bad rows, rows out of place,
//! links to nothing, and cycles of calls. Opening it always works (GAMEPLAY.md 5.7). Only
//! a history from another program is refused. The first open cuts each bad row and drops
//! each broken link, so a second open reads the same. Every proof query ends.
//!
//! The next part of the input damages the outcome tags of a lore pack (GAMEPLAY.md 5.10):
//! a search never panics, an unknown kind is an error, and a tag that a fresh character
//! did not earn never passes the gate. The last part damages the setup tags the same way:
//! a fresh character did no deed, so every setup passes its gate.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use timeways_story::character::Character;
use timeways_story::pack::{Link, Origin, Pack, PackError, Passage};
use timeways_story::spoiler::{outcome_allowed, setup_allowed};
use timeways_story::store::{CharacterKey, Node, Opened, Store, StoreError, Table};

const TAG_KINDS: [&str; 4] = ["foe", "quest", "unresolved", "rumor"];

const SETUP_KINDS: [&str; 3] = ["foe", "quest", "rumor"];

fn folder() -> PathBuf {
    std::env::temp_dir().join(format!("timeways-fuzz-store-{}", std::process::id()))
}

/// The counts of what each table gave.
fn counts(opened: &Opened) -> [usize; 13] {
    [
        opened.saved_events,
        opened.prose.len(),
        opened.flavor.moments().len(),
        opened.flavor.told().len(),
        opened.hero.changes().len(),
        opened.learned.read().len(),
        opened.learned.rumors().len(),
        opened.quests.changes().len(),
        opened.stories.changes().len(),
        opened.rules.rows().len(),
        opened.tales.rows().len(),
        opened.zone_histories.rows().len(),
        opened.entry_edits.rows().len(),
    ]
}

/// A small number from a byte, or NULL for a high byte: links to rows that are there, to
/// rows that are not, and to nothing.
fn link(byte: u8) -> Option<i64> {
    (byte < 0xC0).then_some(i64::from(byte % 8))
}

/// The first byte of a row picks its place and its links. A body that is not UTF-8 goes in
/// as a blob, as another program can write it.
fn insert_row(connection: &Connection, table: Table, position: i64, line: &[u8]) {
    let (head, body) = line.split_first().unwrap_or((&0, &[]));
    let position = if *head & 1 == 0 {
        position
    } else {
        i64::from(*head)
    };
    let insert = format!(
        "INSERT OR IGNORE INTO {} (position, body, input, call) VALUES (?1, ?2, ?3, ?4)",
        table.name()
    );
    let (input, call) = (link(head.rotate_left(2)), link(head.rotate_left(5)));
    match std::str::from_utf8(body) {
        Ok(text) => connection.execute(&insert, params![position, text, input, call]),
        Err(_) => connection.execute(&insert, params![position, body, input, call]),
    }
    .unwrap();
}

fn insert_input(connection: &Connection, line: &[u8]) {
    let root =
        ["game", "player", "shared", "other"][usize::from(line.first().copied().unwrap_or(0) % 4)];
    connection
        .execute(
            "INSERT INTO inputs (kind, root, body) VALUES ('fuzz', ?1, ?2)",
            params![root, String::from_utf8_lossy(line)],
        )
        .unwrap();
}

fn insert_call(connection: &Connection, line: &[u8]) {
    let byte = |index: usize| line.get(index).copied().unwrap_or(0xFF);
    connection
        .execute(
            "INSERT INTO calls (kind, input, call, pack, result) VALUES ('fuzz', ?1, ?2, '', ?3)",
            params![
                link(byte(0)),
                link(byte(1)),
                ["open", "accepted", "refused"][usize::from(byte(2) % 3)]
            ],
        )
        .unwrap();
}

fn insert_read(connection: &Connection, line: &[u8]) {
    let byte = |index: usize| line.get(index).copied().unwrap_or(0);
    let tabs = [
        "events", "chapters", "flavor", "hero", "learned", "quests", "stories", "calls", "nowhere",
    ];
    connection
        .execute(
            "INSERT INTO reads (call, tab, row) VALUES (?1, ?2, ?3)",
            params![
                i64::from(byte(0) % 8),
                tabs[usize::from(byte(1)) % tabs.len()],
                i64::from(byte(2) % 8)
            ],
        )
        .unwrap();
}

/// Every row and call of the world, as a node of the graph.
fn nodes(connection: &Connection) -> Vec<Node> {
    let mut nodes = Vec::new();
    let tables = Table::ALL.map(|table| (table.name(), Some(table)));
    for (name, table) in tables.into_iter().chain([("calls", None)]) {
        let mut statement = connection
            .prepare(&format!("SELECT position FROM {name}"))
            .unwrap();
        let positions = statement.query_map([], |row| row.get::<_, i64>(0)).unwrap();
        for position in positions
            .map(Result::unwrap)
            .filter_map(|p| u64::try_from(p).ok())
        {
            nodes.push(table.map_or(Node::Call(position), |table| Node::Row(table, position)));
        }
    }
    nodes
}

/// A pack of two passages, then one tag for each line: a passage id, a kind, and a name.
/// The id 3 points to no passage.
fn damaged_pack(lines: &[u8]) {
    let path = folder().join("pack.sqlite");
    let passage = |text: &str| Passage {
        text: text.to_string(),
        source: "s".to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: None,
        depends_on: None,
        setup_for: None,
    };
    Pack::write(
        &path,
        &[passage("The ooze fell."), passage("The ooze rose.")],
    )
    .unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    for line in lines.split(|byte| *byte == b'\n') {
        let byte = |index: usize| line.get(index).copied().unwrap_or(0);
        let name = String::from_utf8_lossy(line.get(2..).unwrap_or_default());
        connection
            .execute(
                "INSERT OR IGNORE INTO depends_on (passage, kind, name) VALUES (?1, ?2, ?3)",
                params![
                    i64::from(byte(0) % 4),
                    TAG_KINDS[usize::from(byte(1)) % TAG_KINDS.len()],
                    name
                ],
            )
            .unwrap();
    }
    drop(connection);
    let pack = Pack::open(&path).unwrap();
    match pack.search("ooze", 5) {
        Ok(found) => {
            let fresh = Character::new();
            for passage in found {
                let earned = outcome_allowed(&fresh, passage.depends_on.as_ref());
                assert_eq!(earned, passage.depends_on.is_none(), "{passage:?}");
            }
        }
        Err(PackError::UnknownDependency { kind }) => assert_eq!(kind, "rumor"),
        Err(error) => panic!("a damaged tag broke the pack: {error}"),
    }
}

/// A pack of two passages, then one setup tag for each line: a passage id, a kind, and a
/// name that is also the instance. The id 3 points to no passage.
fn damaged_setups(lines: &[u8]) {
    let path = folder().join("setups.sqlite");
    let passage = |text: &str| Passage {
        text: text.to_string(),
        source: "s".to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: None,
        depends_on: None,
        setup_for: None,
    };
    Pack::write(
        &path,
        &[passage("The marshal wants the ooze dead."), passage("The ooze plots.")],
    )
    .unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    for line in lines.split(|byte| *byte == b'\n') {
        let byte = |index: usize| line.get(index).copied().unwrap_or(0);
        let name = String::from_utf8_lossy(line.get(2..).unwrap_or_default());
        connection
            .execute(
                "INSERT OR IGNORE INTO setup_for (passage, kind, name, instance) \
                 VALUES (?1, ?2, ?3, ?3)",
                params![
                    i64::from(byte(0) % 4),
                    SETUP_KINDS[usize::from(byte(1)) % SETUP_KINDS.len()],
                    name
                ],
            )
            .unwrap();
    }
    drop(connection);
    let pack = Pack::open(&path).unwrap();
    match pack.search("ooze", 5) {
        Ok(found) => {
            let fresh = Character::new();
            for passage in found {
                assert!(setup_allowed(&fresh, passage.setup_for.as_ref()), "{passage:?}");
            }
        }
        Err(PackError::UnknownSetup { kind }) => assert_eq!(kind, "rumor"),
        Err(error) => panic!("a damaged setup tag broke the pack: {error}"),
    }
    for passage in pack.setups_of("ooze").unwrap_or_default() {
        assert!(passage.setup_for.is_some(), "{passage:?}");
    }
}

fuzz_target!(|data: &[u8]| {
    let root = folder();
    let _ = std::fs::remove_dir_all(&root);
    let store = Store::Folder(root.clone());
    let key = CharacterKey::new("Stormrage", "Ada").unwrap();
    drop(store.open(&key).unwrap());
    let file = root.join("worlds").join("r_Stormrage").join("c_Ada.sqlite");
    let connection = Connection::open(&file).unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    let mut parts = data.split(|byte| *byte == 0xFF);
    for table in Table::ALL {
        let lines = parts
            .next()
            .unwrap_or_default()
            .split(|byte| *byte == b'\n');
        for (position, line) in (0..).zip(lines) {
            insert_row(&connection, table, position, line);
        }
    }
    let inserters: [fn(&Connection, &[u8]); 3] = [insert_input, insert_call, insert_read];
    for insert in inserters {
        for line in parts
            .next()
            .unwrap_or_default()
            .split(|byte| *byte == b'\n')
        {
            insert(&connection, line);
        }
    }
    drop(connection);
    damaged_pack(parts.next().unwrap_or_default());
    damaged_setups(parts.next().unwrap_or_default());

    let first = match store.open(&key) {
        Ok(first) => first,
        Err(StoreError::Foreign { .. }) => return,
        Err(error) => panic!("a damaged row locked the character out: {error}"),
    };
    let second = store.open(&key).unwrap();
    assert_eq!(counts(&first), counts(&second));
    let connection = Connection::open(&file).unwrap();
    for node in nodes(&connection) {
        let proof = second.database.proof_of(node).unwrap();
        assert!(!proof.is_empty(), "{node:?} has no root");
        second.database.source_of(node).unwrap();
        second.database.uses_of(node).unwrap();
    }
});
