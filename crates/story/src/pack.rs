//! The lore pack: passages with their sources, in one SQLite file with a full-text index
//! (GAMEPLAY.md 5.10).

use rusqlite::{Connection, OpenFlags, params};
use serde::Serialize;
use std::path::Path;
use thiserror::Error;

/// A pack of another version gets refused, never guessed at.
const FORMAT_VERSION: i64 = 1;

const SCHEMA: &str = "
    CREATE TABLE passage (id INTEGER PRIMARY KEY, text TEXT NOT NULL, source TEXT NOT NULL);
    CREATE TABLE link (
        passage INTEGER NOT NULL REFERENCES passage (id),
        kind TEXT NOT NULL,
        name TEXT NOT NULL
    );
    CREATE VIRTUAL TABLE passage_index USING fts5 (text, content = 'passage', content_rowid = 'id');
";

const PLACE: &str = "place";
const NPC: &str = "npc";
const COMMON: &str = "common";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Passage {
    pub text: String,
    pub source: String,
    /// What the passage is about. The spoiler limit shows it only when the world holds all of them.
    #[serde(skip)]
    pub links: Vec<Link>,
    #[serde(skip)]
    pub origin: Origin,
}

/// Where a passage comes from. Text that the player read is their own lore (GAMEPLAY.md 3.1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Pack,
    Read,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    Place(String),
    Npc(String),
    /// Known to everyone in 25 ADP, such as a history book of the game (GAMEPLAY.md 3.1.1).
    Common,
}

#[derive(Debug, Error)]
pub enum PackError {
    #[error("lore pack: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("lore pack: format version {found}, and this program reads {FORMAT_VERSION}")]
    Version { found: i64 },
    /// A passage with no links passes every spoiler check, so it leaks.
    #[error("lore pack: the passage from {url} has no links")]
    Unlinked { url: String },
    #[error("lore pack: unknown link kind {kind}")]
    UnknownLink { kind: String },
}

pub struct Pack {
    connection: Connection,
}

impl Pack {
    /// # Errors
    ///
    /// Returns an error when the file is missing, is not a pack, or has another format version.
    pub fn open(path: &Path) -> Result<Pack, PackError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection = Connection::open_with_flags(path, flags)?;
        let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if found != FORMAT_VERSION {
            return Err(PackError::Version { found });
        }
        Ok(Pack { connection })
    }

    /// # Errors
    ///
    /// Returns an error when a passage has no links, or when SQLite fails. Nothing is
    /// written then.
    pub fn write(path: &Path, passages: &[Passage]) -> Result<(), PackError> {
        if let Some(unlinked) = passages.iter().find(|passage| passage.links.is_empty()) {
            return Err(PackError::Unlinked {
                url: unlinked.source.clone(),
            });
        }
        let mut connection = Connection::open(path)?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(SCHEMA)?;
        for passage in passages {
            insert(&transaction, passage)?;
        }
        transaction.pragma_update(None, "user_version", FORMAT_VERSION)?;
        transaction.commit()?;
        Ok(())
    }

    /// The best passages for the words of `text`, best first. Syntax of the index in
    /// `text` counts as plain words.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite fails, or when the pack holds an unknown link kind.
    pub fn search(&self, text: &str, limit: u32) -> Result<Vec<Passage>, PackError> {
        let Some(query) = match_query(text) else {
            return Ok(Vec::new());
        };
        let mut statement = self.connection.prepare_cached(
            "SELECT passage.id, passage.text, passage.source
             FROM passage_index JOIN passage ON passage.id = passage_index.rowid
             WHERE passage_index MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = statement.query_map(params![query, limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get(1)?, row.get(2)?))
        })?;
        let mut passages = Vec::new();
        for row in rows {
            let (id, text, source) = row?;
            let links = self.links(id)?;
            passages.push(Passage {
                text,
                source,
                links,
                origin: Origin::Pack,
            });
        }
        Ok(passages)
    }

    fn links(&self, passage: i64) -> Result<Vec<Link>, PackError> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT kind, name FROM link WHERE passage = ?1")?;
        let rows = statement.query_map([passage], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut links = Vec::new();
        for row in rows {
            let (kind, name): (String, String) = row?;
            links.push(link(kind, name)?);
        }
        Ok(links)
    }
}

fn insert(connection: &Connection, passage: &Passage) -> Result<(), PackError> {
    connection.execute(
        "INSERT INTO passage (text, source) VALUES (?1, ?2)",
        params![passage.text, passage.source],
    )?;
    let id = connection.last_insert_rowid();
    connection.execute(
        "INSERT INTO passage_index (rowid, text) VALUES (?1, ?2)",
        params![id, passage.text],
    )?;
    for link in &passage.links {
        let (kind, name) = match link {
            Link::Place(name) => (PLACE, name.as_str()),
            Link::Npc(name) => (NPC, name.as_str()),
            Link::Common => (COMMON, ""),
        };
        connection.execute(
            "INSERT INTO link (passage, kind, name) VALUES (?1, ?2, ?3)",
            params![id, kind, name],
        )?;
    }
    Ok(())
}

fn link(kind: String, name: String) -> Result<Link, PackError> {
    match kind.as_str() {
        PLACE => Ok(Link::Place(name)),
        NPC => Ok(Link::Npc(name)),
        COMMON => Ok(Link::Common),
        _ => Err(PackError::UnknownLink { kind }),
    }
}

/// Any word matches. Each word is quoted, so a player who types `NEAR(` or `"` asks a
/// plain question. The split matches the `unicode61` tokenizer of the index.
pub(crate) fn match_query(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{word}\""))
        .collect();
    if words.is_empty() {
        return None;
    }
    Some(words.join(" OR "))
}
