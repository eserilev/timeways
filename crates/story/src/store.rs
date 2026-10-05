//! The world of each character on disk: one SQLite file (GAMEPLAY.md 5.7). The state is
//! never stored. A replay builds it at start.

mod database;
pub mod graph;
mod logs;
mod shared;

pub use database::{
    CallEnd, CallRecord, Database, Line, NewCall, NewInput, NewRow, Next, Node, Origin, Outcome,
    PROMPTS_KEPT, Root, Table,
};
pub use logs::{
    AliasLog, FlavorLog, HeroLog, LearnedLog, Prose, QuestLog, RowLog, SagaSpan, StoryLog, Summary,
    SummaryLog, TaleText, Written, ZoneHistory,
};
pub use shared::Shared;

use crate::character::Character;
use crate::walk::RuleRow;
use hourglass::{Event, EventId};
use logs::{ChapterProse, FlavorLine, LearnedLine};
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
    #[error("world of {path}: another version of Timeways ({version}) wrote it")]
    OtherVersion { path: PathBuf, version: i64 },
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

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
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

/// The name that `safe_id` made this id from, or None for an id that it never makes.
#[must_use]
pub fn name_of_safe_id(id: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(id.len());
    let mut rest = id.as_bytes();
    while let Some((&first, after)) = rest.split_first() {
        if first != b'_' {
            bytes.push(first);
            rest = after;
            continue;
        }
        let hex = std::str::from_utf8(after.get(..2)?).ok()?;
        bytes.push(u8::from_str_radix(hex, 16).ok()?);
        rest = &after[2..];
    }
    String::from_utf8(bytes).ok()
}

/// What a character brings from the disk.
pub struct Opened {
    pub character: Character,
    pub database: Database,
    /// The events that the database holds. The next save starts after them.
    pub saved_events: usize,
    pub next: Next,
    pub prose: Prose,
    pub flavor: FlavorLog,
    pub hero: HeroLog,
    pub learned: LearnedLog,
    pub quests: QuestLog,
    pub stories: StoryLog,
    pub aliases: AliasLog,
    pub summaries: SummaryLog,
    pub rules: RowLog<RuleRow>,
    pub tales: RowLog<TaleText>,
    pub zone_histories: RowLog<ZoneHistory>,
}

/// Everything of `Opened` but the database, read in one transaction.
struct Read {
    character: Character,
    saved_events: usize,
    next: Next,
    prose: Prose,
    flavor: FlavorLog,
    hero: HeroLog,
    learned: LearnedLog,
    quests: QuestLog,
    stories: StoryLog,
    aliases: AliasLog,
    summaries: SummaryLog,
    rules: RowLog<RuleRow>,
    tales: RowLog<TaleText>,
    zone_histories: RowLog<ZoneHistory>,
}

impl Store {
    /// What all characters share: `timeways.sqlite` in the data folder.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite or of the file system, or `OtherVersion`.
    pub fn open_shared(&self) -> Result<Shared, StoreError> {
        match self {
            Store::Memory => Shared::in_memory(),
            Store::Folder(folder) => {
                fs::create_dir_all(folder).map_err(|source| io_error(folder, source))?;
                Shared::open(&folder.join("timeways.sqlite"))
            }
        }
    }

    /// # Errors
    ///
    /// Returns the error of SQLite or of the file system, `Foreign` for a world that
    /// another program wrote, or `OtherVersion`.
    pub fn open(&self, key: &CharacterKey) -> Result<Opened, StoreError> {
        let (database, path) = match self {
            Store::Memory => (Database::in_memory()?, PathBuf::new()),
            Store::Folder(folder) => {
                let path = folder.join(key.relative_path());
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|source| io_error(&path, source))?;
                }
                (Database::open(&path)?, path)
            }
        };
        let read = database.in_transaction(|database| read_all(database, path))?;
        Ok(Opened {
            character: read.character,
            database,
            saved_events: read.saved_events,
            next: read.next,
            prose: read.prose,
            flavor: read.flavor,
            hero: read.hero,
            learned: read.learned,
            quests: read.quests,
            stories: read.stories,
            aliases: read.aliases,
            summaries: read.summaries,
            rules: read.rules,
            tales: read.tales,
            zone_histories: read.zone_histories,
        })
    }
}

fn read_all(database: &Database, path: PathBuf) -> Result<Read, StoreError> {
    let events: Vec<Event> = database.read_and_repair(Table::Events, |event: &Event, n| {
        event.id == EventId(n as u64)
    })?;
    let character = if events.is_empty() {
        Character::new()
    } else {
        Character::from_history(&events).ok_or(StoreError::Foreign { path })?
    };
    let prose =
        Prose::from_rows(database.read_and_repair(Table::Chapters, |_: &ChapterProse, _| true)?);
    let flavor =
        FlavorLog::from_rows(database.read_and_repair(Table::Flavor, |_: &FlavorLine, _| true)?);
    let hero = HeroLog::from_rows(database.read_and_repair(Table::Hero, |_, _| true)?);
    let learned =
        LearnedLog::from_rows(database.read_and_repair(Table::Learned, |_: &LearnedLine, _| true)?);
    let quests = QuestLog::from_rows(database.read_and_repair(Table::Quests, |_, _| true)?);
    let stories = StoryLog::from_rows(database.read_and_repair(Table::Stories, |_, _| true)?);
    let seen = std::cell::RefCell::new(std::collections::BTreeSet::new());
    let aliases = AliasLog::from_rows(database.read_and_repair(Table::Aliases, |row, _| {
        logs::is_new_name(row, &mut seen.borrow_mut())
    })?);
    let summaries = SummaryLog::from_rows(database.read_and_repair(Table::Summaries, |_, _| true)?);
    let rules = RowLog::from_rows(database.read_and_repair(Table::ChapterRules, |_, _| true)?);
    let tales = RowLog::from_rows(database.read_and_repair(Table::Tales, |_, _| true)?);
    let zone_histories =
        RowLog::from_rows(database.read_and_repair(Table::ZoneHistories, |_, _| true)?);
    database.drop_broken_links()?;
    Ok(Read {
        character,
        saved_events: events.len(),
        next: database.next()?,
        prose,
        flavor,
        hero,
        learned,
        quests,
        stories,
        aliases,
        summaries,
        rules,
        tales,
        zone_histories,
    })
}

fn io_error(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}
