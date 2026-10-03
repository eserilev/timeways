//! What all characters share, in `timeways.sqlite` next to the worlds (docs/plans/links.md
//! 2): the budget and the pace of the narrator. So a restart never lets the narrator speak
//! 3 times at once.

use super::StoreError;
use super::database::sqlite_error;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A file of another version is refused, never changed.
const VERSION: i64 = 1;

const SCHEMA: &str = "CREATE TABLE state (name TEXT PRIMARY KEY, body TEXT NOT NULL)";

/// Each value as one JSON row, by its name.
pub struct Shared {
    connection: Connection,
    path: PathBuf,
    /// The body of each value as the file holds it, so a save writes only a change.
    saved: BTreeMap<&'static str, String>,
}

impl Shared {
    /// # Errors
    ///
    /// Returns the error of SQLite, or `OtherVersion`.
    pub fn open(path: &Path) -> Result<Shared, StoreError> {
        let connection = Connection::open(path).map_err(|source| sqlite_error(path, source))?;
        Shared::start(connection, path)
    }

    /// For `Store::Memory`.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn in_memory() -> Result<Shared, StoreError> {
        let path = Path::new(":memory:");
        let connection =
            Connection::open_in_memory().map_err(|source| sqlite_error(path, source))?;
        Shared::start(connection, path)
    }

    fn start(connection: Connection, path: &Path) -> Result<Shared, StoreError> {
        let shared = Shared {
            connection,
            path: path.to_path_buf(),
            saved: BTreeMap::new(),
        };
        let version: i64 = shared
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|source| shared.error(source))?;
        if version == VERSION {
            return Ok(shared);
        }
        let tables: i64 = shared
            .connection
            .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))
            .map_err(|source| shared.error(source))?;
        if tables > 0 {
            return Err(StoreError::OtherVersion {
                path: shared.path.clone(),
                version,
            });
        }
        let script = format!("BEGIN; {SCHEMA}; PRAGMA user_version = {VERSION}; COMMIT;");
        shared
            .connection
            .execute_batch(&script)
            .map_err(|source| shared.error(source))?;
        Ok(shared)
    }

    /// The default for a value that the file does not hold, or that does not read.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn load<T: DeserializeOwned + Default>(
        &mut self,
        name: &'static str,
    ) -> Result<T, StoreError> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT body FROM state WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.error(source))?;
        let Some(body) = body else {
            return Ok(T::default());
        };
        let value = serde_json::from_str(&body).unwrap_or_default();
        self.saved.insert(name, body);
        Ok(value)
    }

    /// Writes the value only when it changed.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn save<T: Serialize>(&mut self, name: &'static str, value: &T) -> Result<(), StoreError> {
        let body = serde_json::to_string(value)?;
        if self.saved.get(name) == Some(&body) {
            return Ok(());
        }
        self.connection
            .execute(
                "INSERT INTO state (name, body) VALUES (?1, ?2)
                 ON CONFLICT (name) DO UPDATE SET body = excluded.body",
                params![name, body],
            )
            .map_err(|source| self.error(source))?;
        self.saved.insert(name, body);
        Ok(())
    }

    fn error(&self, source: rusqlite::Error) -> StoreError {
        sqlite_error(&self.path, source)
    }
}
