//! The world of each character on disk: one SQLite file, with a table for each log
//! (GAMEPLAY.md 5.7). The state is never stored. A replay builds it at start.

use crate::character::Character;
use crate::flavor::{Flavor, Told};
use crate::hero::Change;
use crate::learned::{Read, Rumor};
use crate::quest::QuestChange;
use hourglass::{Event, EventId, Tick};
use rusqlite::{Connection, params};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// The limits of the bridge for the character line (Gnomish Relay SPEC.md 9.7).
const MAX_REALM_BYTES: usize = 64;
const MAX_NAME_BYTES: usize = 48;

const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("world of {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("world of {path}: {source}")]
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    #[error("world of {path}: it does not start with the founding of the character")]
    Foreign { path: PathBuf },
    #[error("a row does not serialize: {0}")]
    Json(#[from] serde_json::Error),
    #[error("character: {0}")]
    BadKey(&'static str),
}

/// Where the worlds live. `Memory` keeps nothing after the program stops.
pub enum Store {
    Memory,
    Folder(PathBuf),
}

/// One character on one realm, as the game names them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterKey {
    realm: String,
    name: String,
}

impl CharacterKey {
    /// # Errors
    ///
    /// Returns `BadKey` for an empty or too long name or realm, or one with a control
    /// character.
    pub fn new(realm: &str, name: &str) -> Result<CharacterKey, StoreError> {
        check_part(realm, MAX_REALM_BYTES, "the realm")?;
        check_part(name, MAX_NAME_BYTES, "the name")?;
        Ok(CharacterKey {
            realm: realm.to_string(),
            name: name.to_string(),
        })
    }

    /// The prefixes keep a name such as "Con" or "Aux" from naming a Windows device.
    fn relative_path(&self) -> PathBuf {
        let file = format!("c_{}.sqlite", safe_id(&self.name));
        Path::new("worlds")
            .join(format!("r_{}", safe_id(&self.realm)))
            .join(file)
    }
}

fn check_part(part: &str, max_bytes: usize, what: &'static str) -> Result<(), StoreError> {
    if part.is_empty() || part.len() > max_bytes || part.chars().any(char::is_control) {
        return Err(StoreError::BadKey(what));
    }
    Ok(())
}

/// A name from the game as a file name: ASCII letters and digits stay, and every other
/// byte becomes `_` and two hex digits. Two names never share an id, and no id holds a
/// `/`, a `.`, or a space.
#[must_use]
pub fn safe_id(text: &str) -> String {
    let mut id = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() {
            id.push(char::from(byte));
        } else {
            id.push('_');
            id.push(char::from(HEX[usize::from(byte >> 4)]));
            id.push(char::from(HEX[usize::from(byte & 0x0F)]));
        }
    }
    id
}

/// The tables of a world. Each row is one JSON value, in the order that it came.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Table {
    Events,
    Chapters,
    Flavor,
    Hero,
    Learned,
    Quests,
}

impl Table {
    pub const ALL: [Table; 6] = [
        Table::Events,
        Table::Chapters,
        Table::Flavor,
        Table::Hero,
        Table::Learned,
        Table::Quests,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Table::Events => "events",
            Table::Chapters => "chapters",
            Table::Flavor => "flavor",
            Table::Hero => "hero",
            Table::Learned => "learned",
            Table::Quests => "quests",
        }
    }
}

/// The new rows of each table since the last save.
pub type Rows = Vec<(Table, Vec<String>)>;

/// The SQLite file of one character.
pub struct Database {
    connection: Connection,
    path: PathBuf,
}

impl Database {
    fn open(path: &Path) -> Result<Database, StoreError> {
        let connection = Connection::open(path).map_err(|source| sqlite_error(path, source))?;
        let database = Database {
            connection,
            path: path.to_path_buf(),
        };
        for table in Table::ALL {
            let create = format!(
                "CREATE TABLE IF NOT EXISTS {} (position INTEGER PRIMARY KEY, body TEXT NOT NULL)",
                table.name()
            );
            database
                .connection
                .execute_batch(&create)
                .map_err(|source| database.error(source))?;
        }
        Ok(database)
    }

    /// Writes every row in one transaction, so a failed save writes nothing.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn save(&mut self, rows: &Rows) -> Result<(), StoreError> {
        let transaction = self
            .connection
            .transaction()
            .map_err(|source| sqlite_error(&self.path, source))?;
        for (table, bodies) in rows {
            let insert = format!("INSERT INTO {} (body) VALUES (?1)", table.name());
            for body in bodies {
                transaction
                    .execute(&insert, params![body])
                    .map_err(|source| sqlite_error(&self.path, source))?;
            }
        }
        transaction
            .commit()
            .map_err(|source| sqlite_error(&self.path, source))
    }

    /// The rows up to the first one that does not read, or that `accept` refuses at its
    /// place. Only another program writes such a row, so it and every row after it go.
    fn read<T: DeserializeOwned>(
        &self,
        table: Table,
        accept: impl Fn(&T, usize) -> bool,
    ) -> Result<Vec<T>, StoreError> {
        let rows = self.bodies(table).map_err(|source| self.error(source))?;
        let mut items = Vec::with_capacity(rows.len());
        for (position, body) in rows {
            match serde_json::from_str::<T>(&body) {
                Ok(item) if accept(&item, items.len()) => items.push(item),
                _ => {
                    self.cut(table, position)
                        .map_err(|source| self.error(source))?;
                    break;
                }
            }
        }
        Ok(items)
    }

    /// A body that is not text reads as an empty string, which is no JSON, so it ends the
    /// table like any other bad row.
    fn bodies(&self, table: Table) -> rusqlite::Result<Vec<(i64, String)>> {
        let select = format!(
            "SELECT position, body FROM {} ORDER BY position",
            table.name()
        );
        let mut statement = self.connection.prepare(&select)?;
        let rows =
            statement.query_map([], |row| Ok((row.get(0)?, row.get(1).unwrap_or_default())))?;
        rows.collect()
    }

    fn cut(&self, table: Table, from: i64) -> rusqlite::Result<()> {
        let delete = format!("DELETE FROM {} WHERE position >= ?1", table.name());
        self.connection.execute(&delete, params![from])?;
        Ok(())
    }

    fn error(&self, source: rusqlite::Error) -> StoreError {
        sqlite_error(&self.path, source)
    }
}

/// The saga of one chapter, as one row of the chronicle. A row from before footnotes has
/// none.
#[derive(Serialize, Deserialize)]
struct ChapterProse {
    began: Tick,
    text: String,
    #[serde(default)]
    footnotes: Vec<String>,
}

/// The saga of one chapter and its footnotes, as the player reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub text: String,
    pub footnotes: Vec<String>,
}

/// The saga of each chapter, by the tick that began the chapter. The world holds facts
/// only, so the words live in a table of their own.
#[derive(Debug, Default)]
pub struct Prose {
    chapters: BTreeMap<Tick, Written>,
    unsaved: Vec<String>,
}

impl Prose {
    #[must_use]
    pub fn get(&self, began: Tick) -> Option<&Written> {
        self.chapters.get(&began)
    }

    /// The sagas of the chapters that began before `began`.
    pub fn before(&self, began: Tick) -> impl Iterator<Item = &Written> {
        self.chapters.range(..began).map(|(_, written)| written)
    }

    /// The chapters that have a saga.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chapters.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chapters.is_empty()
    }

    /// # Errors
    ///
    /// Returns `Json` for a saga that does not serialize, and then keeps nothing.
    pub fn add(&mut self, began: Tick, written: Written) -> Result<(), StoreError> {
        let row = ChapterProse {
            began,
            text: written.text.clone(),
            footnotes: written.footnotes.clone(),
        };
        self.unsaved.push(serde_json::to_string(&row)?);
        self.chapters.insert(began, written);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<String> {
        std::mem::take(&mut self.unsaved)
    }
}

/// One row of the flavor table: a moment, or a telling of a kind of moment.
#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
enum FlavorLine {
    Moment(Flavor),
    Told(Told),
}

/// The flavor moments of a character and the tellings of them (GAMEPLAY.md 5.4.1). They are
/// small and silly, not facts, so they live in a table of their own.
#[derive(Debug, Default)]
pub struct FlavorLog {
    moments: Vec<Flavor>,
    told: Vec<Told>,
    unsaved: Vec<String>,
}

impl FlavorLog {
    #[must_use]
    pub fn moments(&self) -> &[Flavor] {
        &self.moments
    }

    #[must_use]
    pub fn told(&self) -> &[Told] {
        &self.told
    }

    /// # Errors
    ///
    /// Returns `Json` for a moment that does not serialize, and then keeps nothing.
    pub fn add_moment(&mut self, moment: Flavor) -> Result<(), StoreError> {
        self.unsaved
            .push(serde_json::to_string(&FlavorLine::Moment(moment.clone()))?);
        self.moments.push(moment);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns `Json` for a telling that does not serialize, and then keeps nothing.
    pub fn add_told(&mut self, told: Told) -> Result<(), StoreError> {
        self.unsaved
            .push(serde_json::to_string(&FlavorLine::Told(told.clone()))?);
        self.told.push(told);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<String> {
        std::mem::take(&mut self.unsaved)
    }
}

/// The changes of the story of the hero, oldest first (see `hero`). They are the player's
/// own words, not facts, so they live in a table of their own.
#[derive(Debug, Default)]
pub struct HeroLog {
    changes: Vec<Change>,
    unsaved: Vec<String>,
}

impl HeroLog {
    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns `Json` for a change that does not serialize, and then keeps nothing.
    pub fn add(&mut self, change: Change) -> Result<(), StoreError> {
        self.unsaved.push(serde_json::to_string(&change)?);
        self.changes.push(change);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<String> {
        std::mem::take(&mut self.unsaved)
    }
}

/// What the player read and heard, oldest first (see `learned`).
#[derive(Debug, Default)]
pub struct LearnedLog {
    read: Vec<Read>,
    rumors: Vec<Rumor>,
    unsaved: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
enum LearnedLine {
    Read(Read),
    Rumor(Rumor),
}

impl LearnedLog {
    #[must_use]
    pub fn read(&self) -> &[Read] {
        &self.read
    }

    #[must_use]
    pub fn rumors(&self) -> &[Rumor] {
        &self.rumors
    }

    /// # Errors
    ///
    /// Returns `Json` for a text that does not serialize, and then keeps nothing.
    pub fn add_read(&mut self, read: Read) -> Result<(), StoreError> {
        self.unsaved
            .push(serde_json::to_string(&LearnedLine::Read(read.clone()))?);
        self.read.push(read);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns `Json` for a rumor that does not serialize, and then keeps nothing.
    pub fn add_rumor(&mut self, rumor: Rumor) -> Result<(), StoreError> {
        self.unsaved
            .push(serde_json::to_string(&LearnedLine::Rumor(rumor.clone()))?);
        self.rumors.push(rumor);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<String> {
        std::mem::take(&mut self.unsaved)
    }
}

/// The changes of the side quests, oldest first (see `quest`).
#[derive(Debug, Default)]
pub struct QuestLog {
    changes: Vec<QuestChange>,
    unsaved: Vec<String>,
}

impl QuestLog {
    #[must_use]
    pub fn changes(&self) -> &[QuestChange] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns `Json` for a change that does not serialize, and then keeps nothing.
    pub fn add(&mut self, change: QuestChange) -> Result<(), StoreError> {
        self.unsaved.push(serde_json::to_string(&change)?);
        self.changes.push(change);
        Ok(())
    }

    pub fn take_unsaved(&mut self) -> Vec<String> {
        std::mem::take(&mut self.unsaved)
    }
}

/// What a character brings from the disk.
pub struct Opened {
    pub character: Character,
    /// None for the `Memory` store.
    pub database: Option<Database>,
    /// The events that the database holds. The next save starts after them.
    pub saved_events: usize,
    pub prose: Prose,
    pub flavor: FlavorLog,
    pub hero: HeroLog,
    pub learned: LearnedLog,
    pub quests: QuestLog,
}

impl Store {
    /// # Errors
    ///
    /// Returns the error of SQLite or of the file system, or `Foreign` for a world that
    /// another program wrote.
    pub fn open(&self, key: &CharacterKey) -> Result<Opened, StoreError> {
        let Store::Folder(folder) = self else {
            return Ok(Opened {
                character: Character::new(),
                database: None,
                saved_events: 0,
                prose: Prose::default(),
                flavor: FlavorLog::default(),
                hero: HeroLog::default(),
                learned: LearnedLog::default(),
                quests: QuestLog::default(),
            });
        };
        let path = folder.join(key.relative_path());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| io_error(&path, source))?;
        }
        let database = Database::open(&path)?;
        let events: Vec<Event> = database.read(Table::Events, |event: &Event, n| {
            event.id == EventId(n as u64)
        })?;
        let character = if events.is_empty() {
            Character::new()
        } else {
            Character::from_history(&events)
                .ok_or_else(|| StoreError::Foreign { path: path.clone() })?
        };
        let opened = Opened {
            character,
            saved_events: events.len(),
            prose: read_prose(&database)?,
            flavor: read_flavor(&database)?,
            hero: HeroLog {
                changes: database.read(Table::Hero, |_, _| true)?,
                unsaved: Vec::new(),
            },
            learned: read_learned(&database)?,
            quests: QuestLog {
                changes: database.read(Table::Quests, |_, _| true)?,
                unsaved: Vec::new(),
            },
            database: Some(database),
        };
        Ok(opened)
    }
}

fn read_prose(database: &Database) -> Result<Prose, StoreError> {
    let rows: Vec<ChapterProse> = database.read(Table::Chapters, |_, _| true)?;
    let chapters = rows
        .into_iter()
        .map(|row| {
            let written = Written {
                text: row.text,
                footnotes: row.footnotes,
            };
            (row.began, written)
        })
        .collect();
    Ok(Prose {
        chapters,
        unsaved: Vec::new(),
    })
}

fn read_flavor(database: &Database) -> Result<FlavorLog, StoreError> {
    let mut flavor = FlavorLog::default();
    for line in database.read(Table::Flavor, |_, _| true)? {
        match line {
            FlavorLine::Moment(moment) => flavor.moments.push(moment),
            FlavorLine::Told(told) => flavor.told.push(told),
        }
    }
    Ok(flavor)
}

fn read_learned(database: &Database) -> Result<LearnedLog, StoreError> {
    let mut learned = LearnedLog::default();
    for line in database.read(Table::Learned, |_, _| true)? {
        match line {
            LearnedLine::Read(read) => learned.read.push(read),
            LearnedLine::Rumor(rumor) => learned.rumors.push(rumor),
        }
    }
    Ok(learned)
}

fn io_error(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn sqlite_error(path: &Path, source: rusqlite::Error) -> StoreError {
    StoreError::Sqlite {
        path: path.to_path_buf(),
        source,
    }
}
