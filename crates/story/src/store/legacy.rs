//! The JSON files of a build before SQLite, read once into a new database (GAMEPLAY.md
//! 5.7). The files stay as they are.
// TODO: remove this module at 0.2.0. By then, every test world moved to SQLite.

use super::{Database, Rows, StoreError, Table, io_error};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Builds the database under a temporary name and renames it at the end, so a crash
/// leaves no half database that hides the old files. With no old files, it does nothing.
pub(super) fn import(database: &Path) -> Result<(), StoreError> {
    let mut rows = Rows::new();
    for table in Table::ALL {
        let file = old_file(database, table);
        let bodies = read_bodies(&file).map_err(|source| io_error(&file, source))?;
        rows.push((table, bodies));
    }
    if rows.iter().all(|(_, bodies)| bodies.is_empty()) {
        return Ok(());
    }
    let building = database.with_extension("sqlite.new");
    let _ = fs::remove_file(&building);
    Database::open(&building)?.save(&rows)?;
    fs::rename(&building, database).map_err(|source| io_error(database, source))
}

/// `c_Ada.sqlite` had `c_Ada.jsonl` for its events and `c_Ada.<table>.jsonl` for the rest.
fn old_file(database: &Path, table: Table) -> PathBuf {
    match table {
        Table::Events => database.with_extension("jsonl"),
        Table::Chapters => database.with_extension("chronicle.jsonl"),
        _ => database.with_extension(format!("{}.jsonl", table.name())),
    }
}

/// The lines up to the first one that is not whole JSON. A crash left such a line at the
/// end. The open of the database checks each row against its type.
fn read_bodies(file: &Path) -> io::Result<Vec<String>> {
    let bytes = match fs::read(file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut bodies = Vec::new();
    let mut lines = bytes.split_inclusive(|byte| *byte == b'\n');
    while let Some(line) = lines.next().filter(|line| line.ends_with(b"\n")) {
        let Ok(text) = std::str::from_utf8(line.trim_ascii_end()) else {
            break;
        };
        if serde_json::from_str::<serde_json::Value>(text).is_err() {
            break;
        }
        bodies.push(text.to_string());
    }
    Ok(bodies)
}
