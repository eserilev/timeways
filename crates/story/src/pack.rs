//! The lore pack: passages with their sources, in one SQLite file with a full-text index
//! (GAMEPLAY.md 5.10).

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;
use std::path::Path;
use thiserror::Error;

/// A pack of another version gets refused, never guessed at. Format 2 added `about`,
/// format 3 added `depends_on`, and format 4 added `setup_for`.
const FORMAT_VERSION: i64 = 4;

const SCHEMA: &str = "
    CREATE TABLE passage (
        id INTEGER PRIMARY KEY,
        text TEXT NOT NULL,
        source TEXT NOT NULL,
        about TEXT
    );
    CREATE INDEX passage_about ON passage (about);
    CREATE TABLE link (
        passage INTEGER NOT NULL REFERENCES passage (id),
        kind TEXT NOT NULL,
        name TEXT NOT NULL
    );
    CREATE TABLE depends_on (
        passage INTEGER PRIMARY KEY REFERENCES passage (id),
        kind TEXT NOT NULL,
        name TEXT NOT NULL
    );
    CREATE TABLE setup_for (
        passage INTEGER PRIMARY KEY REFERENCES passage (id),
        kind TEXT NOT NULL,
        name TEXT NOT NULL,
        instance TEXT NOT NULL
    );
    CREATE INDEX setups_of_an_instance ON setup_for (instance);
    CREATE VIRTUAL TABLE passage_index USING fts5 (text, content = 'passage', content_rowid = 'id');
";

const PLACE: &str = "place";
const NPC: &str = "npc";
const COMMON: &str = "common";
const FOE: &str = "foe";
const QUEST: &str = "quest";
const UNRESOLVED: &str = "unresolved";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Passage {
    pub text: String,
    pub source: String,
    /// What the passage is about. The spoiler limit shows it only when the world holds all of them.
    #[serde(skip)]
    pub links: Vec<Link>,
    #[serde(skip)]
    pub origin: Origin,
    /// The place or the person that the page of this passage is about: "The Deadmines" for
    /// the page "Deadmines", and None for the page "Mr. Smite" that links to it. The
    /// narrator takes the own page of a place first (GAMEPLAY.md 3.2).
    #[serde(skip)]
    pub about: Option<String>,
    /// The deed of adventurers that the passage tells, or None for no deed. The passage
    /// waits until the player did that deed (GAMEPLAY.md 5.10).
    #[serde(skip)]
    pub depends_on: Option<Dependency>,
    /// The deed that the passage sets up in a dungeon or a raid, or None for no setup. The
    /// passage goes stale once the player did that deed (GAMEPLAY.md 5.10).
    #[serde(skip)]
    pub setup_for: Option<SetupFor>,
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

/// What an outcome passage waits for: "adventurers killed Mr. Smite" waits for the
/// defeat of Mr. Smite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dependency {
    Foe(String),
    /// The title of a quest of the game.
    Quest(String),
    /// The builder tied the deed to no foe and no quest, so the passage never shows.
    Unresolved,
}

/// What a setup passage asks for, and where: "Gryan Stoutmantle sent adventurers to kill
/// VanCleef" sets up the defeat of Edwin VanCleef in the Deadmines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupFor {
    pub deed: Deed,
    /// The dungeon or the raid of the deed.
    pub instance: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Deed {
    Foe(String),
    /// The title of a quest of the game.
    Quest(String),
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
    #[error("lore pack: unknown dependency kind {kind}")]
    UnknownDependency { kind: String },
    #[error("lore pack: unknown setup kind {kind}")]
    UnknownSetup { kind: String },
}

pub struct Pack {
    connection: Connection,
    label: String,
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
        let passages: i64 =
            connection.query_row("SELECT count(*) FROM passage", [], |row| row.get(0))?;
        let label = format!("format {FORMAT_VERSION}, {passages} passages");
        Ok(Pack { connection, label })
    }

    /// A pack with no passage, in memory, while the desktop app builds the real one (relay
    /// SPEC 11.4). `/lore` then answers from the text that the player read.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite fails.
    pub fn empty() -> Result<Pack, PackError> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(SCHEMA)?;
        let label = "no pack yet".to_string();
        Ok(Pack { connection, label })
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

    /// Which pack a model call used. A passage id changes with each pack, so a call keeps
    /// this in its place.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
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
            "SELECT passage.id, passage.text, passage.source, passage.about
             FROM passage_index JOIN passage ON passage.id = passage_index.rowid
             WHERE passage_index MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = statement.query_map(params![query, limit], row_of)?;
        self.passages(rows)
    }

    /// The passages of the own page of `name`, in the order of the page, so the lead
    /// comes first.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite fails, or when the pack holds an unknown link kind.
    pub fn about(&self, name: &str, limit: u32) -> Result<Vec<Passage>, PackError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT id, text, source, about FROM passage WHERE about = ?1 ORDER BY id LIMIT ?2",
        )?;
        let rows = statement.query_map(params![name, limit], row_of)?;
        self.passages(rows)
    }

    /// The setup passages of a dungeon or a raid, in pack order.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite fails, or when the pack holds an unknown kind.
    pub fn setups_of(&self, instance: &str) -> Result<Vec<Passage>, PackError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT passage.id, passage.text, passage.source, passage.about
             FROM passage JOIN setup_for ON setup_for.passage = passage.id
             WHERE setup_for.instance = ?1 ORDER BY passage.id",
        )?;
        let rows = statement.query_map(params![instance], row_of)?;
        self.passages(rows)
    }

    /// True when an outcome or a setup passage waits for the defeat of this foe.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite fails.
    pub fn tags_foe(&self, name: &str) -> Result<bool, PackError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT EXISTS (SELECT 1 FROM depends_on WHERE kind = ?1 AND name = ?2)
                 OR EXISTS (SELECT 1 FROM setup_for WHERE kind = ?1 AND name = ?2)",
        )?;
        Ok(statement.query_row(params![FOE, name], |row| row.get(0))?)
    }

    /// Every passage of a place, in pack order: each one that links to it, and each setup
    /// of it.
    ///
    /// # Errors
    ///
    /// Returns an error when SQLite fails, or when the pack holds an unknown kind.
    pub fn of_place(&self, place: &str) -> Result<Vec<Passage>, PackError> {
        let mut statement = self.connection.prepare_cached(
            "SELECT id, text, source, about FROM passage
             WHERE id IN (SELECT passage FROM link WHERE kind = ?1 AND name = ?2)
                OR id IN (SELECT passage FROM setup_for WHERE instance = ?2)
             ORDER BY id",
        )?;
        let rows = statement.query_map(params![PLACE, place], row_of)?;
        self.passages(rows)
    }

    fn passages(
        &self,
        rows: impl Iterator<Item = rusqlite::Result<Row>>,
    ) -> Result<Vec<Passage>, PackError> {
        let mut passages = Vec::new();
        for row in rows {
            let (id, text, source, about) = row?;
            let links = self.links(id)?;
            let depends_on = self.depends_on(id)?;
            let setup_for = self.setup_for(id)?;
            passages.push(Passage {
                text,
                source,
                links,
                origin: Origin::Pack,
                about,
                depends_on,
                setup_for,
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

    fn depends_on(&self, passage: i64) -> Result<Option<Dependency>, PackError> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT kind, name FROM depends_on WHERE passage = ?1")?;
        let row = statement
            .query_row([passage], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        row.map(|(kind, name)| dependency(kind, name)).transpose()
    }

    fn setup_for(&self, passage: i64) -> Result<Option<SetupFor>, PackError> {
        let mut statement = self
            .connection
            .prepare_cached("SELECT kind, name, instance FROM setup_for WHERE passage = ?1")?;
        let row = statement
            .query_row([passage], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .optional()?;
        row.map(|(kind, name, instance)| setup(kind, name, instance))
            .transpose()
    }
}

/// The id, the text, the source, and the subject of a passage.
type Row = (i64, String, String, Option<String>);

fn row_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn insert(connection: &Connection, passage: &Passage) -> Result<(), PackError> {
    connection.execute(
        "INSERT INTO passage (text, source, about) VALUES (?1, ?2, ?3)",
        params![passage.text, passage.source, passage.about],
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
    if let Some(dependency) = &passage.depends_on {
        let (kind, name) = match dependency {
            Dependency::Foe(name) => (FOE, name.as_str()),
            Dependency::Quest(name) => (QUEST, name.as_str()),
            Dependency::Unresolved => (UNRESOLVED, ""),
        };
        connection.execute(
            "INSERT INTO depends_on (passage, kind, name) VALUES (?1, ?2, ?3)",
            params![id, kind, name],
        )?;
    }
    if let Some(setup) = &passage.setup_for {
        let (kind, name) = match &setup.deed {
            Deed::Foe(name) => (FOE, name.as_str()),
            Deed::Quest(name) => (QUEST, name.as_str()),
        };
        connection.execute(
            "INSERT INTO setup_for (passage, kind, name, instance) VALUES (?1, ?2, ?3, ?4)",
            params![id, kind, name, setup.instance],
        )?;
    }
    Ok(())
}

fn setup(kind: String, name: String, instance: String) -> Result<SetupFor, PackError> {
    let deed = match kind.as_str() {
        FOE => Deed::Foe(name),
        QUEST => Deed::Quest(name),
        _ => return Err(PackError::UnknownSetup { kind }),
    };
    Ok(SetupFor { deed, instance })
}

fn dependency(kind: String, name: String) -> Result<Dependency, PackError> {
    match kind.as_str() {
        FOE => Ok(Dependency::Foe(name)),
        QUEST => Ok(Dependency::Quest(name)),
        UNRESOLVED => Ok(Dependency::Unresolved),
        _ => Err(PackError::UnknownDependency { kind }),
    }
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
