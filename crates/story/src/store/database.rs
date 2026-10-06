//! The SQLite file of one character: its rows, the input lines and model calls behind them,
//! and what each call read (GAMEPLAY.md 5.7).

use super::StoreError;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::time::Duration;
pub use timeways_rules::prompts::PROMPTS_KEPT;
use timeways_rules::prompts::oldest_prompt_kept;

/// A file of another version is refused, never changed. Nothing is live, so a new version
/// starts with new worlds.
const VERSION: i64 = 9;

/// Version 9 only added the column `shape` to `calls`, so a file of version 8 takes the
/// column and keeps its rows.
const ADD_SHAPE: &str = "ALTER TABLE calls ADD COLUMN shape TEXT; PRAGMA user_version = 9;";

/// WAL syncs the disk once for each line, and a reader such as `sqlite3` never blocks a
/// save.
pub(super) const PRAGMAS: &str =
    "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL";

/// Only another program that writes holds the lock, so a save gives up soon.
pub(super) const BUSY_WAIT: Duration = Duration::from_millis(500);

/// Each row table keeps one JSON value for each row, and the line or call that made it.
const ROW_TABLE: &str = "position INTEGER PRIMARY KEY, body TEXT NOT NULL,
    input INTEGER REFERENCES inputs (position), call INTEGER REFERENCES calls (position)";

const SCHEMA: &str = "
CREATE TABLE inputs (
    position INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    root TEXT NOT NULL,
    body TEXT
);
CREATE TABLE calls (
    position INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    input INTEGER REFERENCES inputs (position),
    call INTEGER REFERENCES calls (position),
    pack TEXT NOT NULL,
    prompt TEXT,
    answer TEXT,
    result TEXT NOT NULL,
    shape TEXT
);
CREATE TABLE reads (
    call INTEGER NOT NULL REFERENCES calls (position),
    tab TEXT NOT NULL,
    row INTEGER NOT NULL
);
CREATE INDEX reads_of_a_row ON reads (tab, row);
CREATE INDEX reads_of_a_call ON reads (call);
CREATE INDEX calls_of_a_kind ON calls (kind);
";

/// The tables of the world. Each row is one JSON value, and its position is its place in
/// the table, from 0. An event's position is its `EventId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Table {
    Events,
    Chapters,
    Flavor,
    Hero,
    Learned,
    Quests,
    Stories,
    /// The alias table (GAMEPLAY.md 5.11): the place of a row is the ID of its player.
    Aliases,
    /// Who the character has become, for the title page of the Chronicle. The newest row
    /// stands.
    Summaries,
    /// The rule epochs of the chapters (docs/plans/chapters.md 13): from which event each
    /// rule cuts.
    ChapterRules,
    /// The texts of the tales, each with the visit that it covers.
    Tales,
    /// "Your history here": one text for a zone, rewritten after a chapter.
    ZoneHistories,
    /// The player's edits of a chapter, a tale, or the summary. The newest row of an entry
    /// stands.
    EntryEdits,
}

impl Table {
    pub const ALL: [Table; 13] = [
        Table::Events,
        Table::Chapters,
        Table::Flavor,
        Table::Hero,
        Table::Learned,
        Table::Quests,
        Table::Stories,
        Table::Aliases,
        Table::Summaries,
        Table::ChapterRules,
        Table::Tales,
        Table::ZoneHistories,
        Table::EntryEdits,
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
            Table::Stories => "stories",
            Table::Aliases => "aliases",
            Table::Summaries => "summaries",
            Table::ChapterRules => "chapter_rules",
            Table::Tales => "tales",
            Table::ZoneHistories => "zone_histories",
            Table::EntryEdits => "entry_edits",
        }
    }

    #[must_use]
    pub fn named(name: &str) -> Option<Table> {
        Table::ALL.into_iter().find(|table| table.name() == name)
    }
}

/// One node of the graph of proof: a row, a model call, or an input line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node {
    Row(Table, u64),
    Call(u64),
    Input(u64),
}

/// What stands behind an input line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Root {
    /// A row whose line or call is gone. Only another program makes one.
    Lost,
    /// Another player said it.
    Shared,
    /// The player typed it, or clicked a button of Timeways.
    Player,
    /// The game said it.
    Game,
}

impl Root {
    fn name(self) -> &'static str {
        match self {
            Root::Lost => "lost",
            Root::Shared => "shared",
            Root::Player => "player",
            Root::Game => "game",
        }
    }

    fn named(name: &str) -> Root {
        [Root::Shared, Root::Player, Root::Game]
            .into_iter()
            .find(|root| root.name() == name)
            .unwrap_or(Root::Lost)
    }
}

/// How a model call ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Open,
    /// The answer passed its checks, and the story used it.
    Accepted,
    /// The answer broke a rule. An answer for a character that is not active ends
    /// nothing: its call stays open.
    Refused,
    Failed,
}

impl Outcome {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Open => "open",
            Outcome::Accepted => "accepted",
            Outcome::Refused => "refused",
            Outcome::Failed => "failed",
        }
    }
}

/// What made the rows and calls of a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Input(u64),
    Call(u64),
}

/// A new row, with its position in its table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewRow {
    pub position: u64,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewInput {
    pub position: u64,
    pub kind: String,
    pub root: Root,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCall {
    pub position: u64,
    pub kind: &'static str,
    pub pack: String,
    pub prompt: String,
    pub reads: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallEnd {
    pub position: u64,
    pub answer: Option<String>,
    pub outcome: Outcome,
    /// The main part of the shape of an accepted narrator line (docs/plans/narrator-templates.md
    /// 3.5).
    pub shape: Option<String>,
}

/// Everything that one line from the bridge saves, in one transaction.
#[derive(Debug, Default)]
pub struct Line {
    pub input: Option<NewInput>,
    /// None only for a line of a call of another character, which makes nothing here.
    pub origin: Option<Origin>,
    pub calls: Vec<NewCall>,
    pub ended: Vec<CallEnd>,
    pub rows: Vec<(Table, Vec<NewRow>)>,
}

impl Line {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.input.is_none()
            && self.calls.is_empty()
            && self.ended.is_empty()
            && self.rows.iter().all(|(_, rows)| rows.is_empty())
    }
}

/// The next free position of the tables that the story program adds to without reading
/// them back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Next {
    pub input: u64,
    pub call: u64,
}

pub struct Database {
    connection: Connection,
    path: PathBuf,
}

impl Database {
    /// # Errors
    ///
    /// Returns the error of SQLite, or `OtherVersion` for a file of another version.
    pub fn open(path: &Path) -> Result<Database, StoreError> {
        let connection = Connection::open(path).map_err(|source| sqlite_error(path, source))?;
        Database::start(connection, path)
    }

    /// For `Store::Memory`: every query works, and nothing stays after the program stops.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn in_memory() -> Result<Database, StoreError> {
        let path = Path::new(":memory:");
        let connection =
            Connection::open_in_memory().map_err(|source| sqlite_error(path, source))?;
        Database::start(connection, path)
    }

    fn start(connection: Connection, path: &Path) -> Result<Database, StoreError> {
        let database = Database {
            connection,
            path: path.to_path_buf(),
        };
        database
            .connection
            .execute_batch(PRAGMAS)
            .map_err(|source| database.error(source))?;
        database
            .connection
            .busy_timeout(BUSY_WAIT)
            .map_err(|source| database.error(source))?;
        database.create_or_check()?;
        Ok(database)
    }

    fn create_or_check(&self) -> Result<(), StoreError> {
        let version: i64 = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|source| self.error(source))?;
        if version == VERSION {
            return Ok(());
        }
        if version == VERSION - 1 {
            return self
                .connection
                .execute_batch(ADD_SHAPE)
                .map_err(|source| self.error(source));
        }
        let tables: i64 = self
            .connection
            .query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))
            .map_err(|source| self.error(source))?;
        if tables > 0 {
            return Err(StoreError::OtherVersion {
                path: self.path.clone(),
                version,
            });
        }
        let row_tables: String = Table::ALL
            .map(|table| format!("CREATE TABLE {} ({ROW_TABLE});\n", table.name()))
            .concat();
        let script =
            format!("BEGIN; {SCHEMA} {row_tables} PRAGMA user_version = {VERSION}; COMMIT;");
        self.connection
            .execute_batch(&script)
            .map_err(|source| self.error(source))
    }

    /// Writes the line in one transaction, so a failed save writes nothing.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn save(&mut self, line: &Line) -> Result<(), StoreError> {
        if line.is_empty() {
            return Ok(());
        }
        let path = self.path.clone();
        let transaction = self
            .connection
            .transaction()
            .map_err(|source| sqlite_error(&path, source))?;
        write_line(&transaction, line).map_err(|source| sqlite_error(&path, source))?;
        transaction
            .commit()
            .map_err(|source| sqlite_error(&path, source))
    }

    /// The rows up to the first one that does not read, that `accept` refuses, or that does
    /// not sit at its place. Only another program writes such a row, so it and every row
    /// after it go, with every read of them.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn read_and_repair<T: DeserializeOwned>(
        &self,
        table: Table,
        accept: impl Fn(&T, usize) -> bool,
    ) -> Result<Vec<T>, StoreError> {
        let rows = self.bodies(table).map_err(|source| self.error(source))?;
        let mut items = Vec::with_capacity(rows.len());
        for (position, body) in rows {
            let at_its_place = usize::try_from(position).is_ok_and(|p| p == items.len());
            match serde_json::from_str::<T>(&body) {
                Ok(item) if at_its_place && accept(&item, items.len()) => items.push(item),
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
        self.connection.execute(
            "DELETE FROM reads WHERE tab = ?1 AND row >= ?2",
            params![table.name(), from],
        )?;
        Ok(())
    }

    /// Another program can leave a link to a line or call that is gone. Such a link goes,
    /// so the row shows as lost, and every new write passes the check of SQLite.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn drop_broken_links(&self) -> Result<(), StoreError> {
        self.drop_links().map_err(|source| self.error(source))
    }

    /// A check first, so an open writes nothing while another program holds the lock.
    fn drop_links(&self) -> rusqlite::Result<()> {
        let mut check = self.connection.prepare("PRAGMA foreign_key_check")?;
        if !check.exists([])? {
            return Ok(());
        }
        // Both columns in one statement: `calls` points to itself, so SQLite checks the
        // whole row of a call that changes, and a fix of one column fails while the other
        // is still broken.
        for table in Table::ALL.map(Table::name).into_iter().chain(["calls"]) {
            let update = format!(
                "UPDATE {table} SET
                     input = CASE WHEN input IN (SELECT position FROM inputs) THEN input END,
                     call = CASE WHEN call IN (SELECT position FROM calls) THEN call END
                 WHERE input NOT IN (SELECT position FROM inputs)
                     OR call NOT IN (SELECT position FROM calls)"
            );
            self.connection.execute(&update, [])?;
        }
        self.connection.execute(
            "DELETE FROM reads WHERE call NOT IN (SELECT position FROM calls)",
            [],
        )?;
        Ok(())
    }

    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn next(&self) -> Result<Next, StoreError> {
        let next = |table: &str| -> rusqlite::Result<u64> {
            let select = format!("SELECT coalesce(max(position) + 1, 0) FROM {table}");
            let next: i64 = self.connection.query_row(&select, [], |row| row.get(0))?;
            Ok(u64::try_from(next).unwrap_or_default())
        };
        let found = next("inputs").and_then(|input| {
            Ok(Next {
                input,
                call: next("calls")?,
            })
        });
        found.map_err(|source| self.error(source))
    }

    /// Runs `act` in one transaction, so the cuts of an open land together or not at all.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite, or the error of `act`.
    pub fn in_transaction<T>(
        &self,
        act: impl FnOnce(&Database) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|source| self.error(source))?;
        let result = act(self)?;
        transaction.commit().map_err(|source| self.error(source))?;
        Ok(result)
    }

    /// The input line or the call that made a row or a call. An input has none.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn origin(&self, node: Node) -> Result<Option<Node>, StoreError> {
        let (table, position) = match node {
            Node::Input(_) => return Ok(None),
            Node::Call(position) => ("calls", position),
            Node::Row(table, position) => (table.name(), position),
        };
        let select = format!("SELECT input, call FROM {table} WHERE position = ?1");
        let found: Option<(Option<i64>, Option<i64>)> = self
            .connection
            .query_row(&select, params![as_sql(position)], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(|source| self.error(source))?;
        let Some((input, call)) = found else {
            return Ok(None);
        };
        let input = input.and_then(from_sql).map(Node::Input);
        Ok(input.or_else(|| call.and_then(from_sql).map(Node::Call)))
    }

    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn root_of_input(&self, position: u64) -> Result<Root, StoreError> {
        let root: Option<String> = self
            .connection
            .query_row(
                "SELECT root FROM inputs WHERE position = ?1",
                params![as_sql(position)],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.error(source))?;
        Ok(root.as_deref().map_or(Root::Lost, Root::named))
    }

    /// The kind of an input line, such as `npc_slapped`.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn input_kind(&self, position: u64) -> Result<Option<String>, StoreError> {
        self.connection
            .query_row(
                "SELECT kind FROM inputs WHERE position = ?1",
                params![as_sql(position)],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.error(source))
    }

    /// What a call read when it opened.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn reads_of(&self, call: u64) -> Result<Vec<Node>, StoreError> {
        let select = "SELECT tab, row FROM reads WHERE call = ?1 ORDER BY rowid";
        self.nodes(select, call)
    }

    /// The accepted calls that read this node, oldest first.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn accepted_readers_of(&self, node: Node) -> Result<Vec<u64>, StoreError> {
        let (tab, row) = match node {
            Node::Input(_) => return Ok(Vec::new()),
            Node::Call(position) => ("calls", position),
            Node::Row(table, position) => (table.name(), position),
        };
        let select =
            "SELECT DISTINCT reads.call FROM reads JOIN calls ON calls.position = reads.call
            WHERE reads.tab = ?1 AND reads.row = ?2 AND calls.result = 'accepted'
            ORDER BY reads.call";
        let mut statement = self
            .connection
            .prepare(select)
            .map_err(|source| self.error(source))?;
        let calls = statement
            .query_map(params![tab, as_sql(row)], |row| row.get::<_, i64>(0))
            .and_then(Iterator::collect::<rusqlite::Result<Vec<i64>>>)
            .map_err(|source| self.error(source))?;
        Ok(calls.into_iter().filter_map(from_sql).collect())
    }

    /// The rows of `calls` with one of these kinds, whatever their result.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn count_calls(&self, kinds: &[&str]) -> Result<u64, StoreError> {
        let marks = vec!["?"; kinds.len()].join(", ");
        let select = format!("SELECT count(*) FROM calls WHERE kind IN ({marks})");
        let found: i64 = self
            .connection
            .query_row(&select, params_from_iter(kinds), |row| row.get(0))
            .map_err(|source| self.error(source))?;
        Ok(u64::try_from(found).unwrap_or_default())
    }

    /// The shapes of the newest accepted calls that hold one, newest first.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn newest_shapes(&self, most: usize) -> Result<Vec<String>, StoreError> {
        let select = "SELECT shape FROM calls WHERE shape IS NOT NULL AND result = 'accepted' \
                      ORDER BY position DESC LIMIT ?1";
        let mut statement = self
            .connection
            .prepare_cached(select)
            .map_err(|source| self.error(source))?;
        let rows = statement
            .query_map(params![i64::try_from(most).unwrap_or(i64::MAX)], |row| {
                row.get(0)
            })
            .map_err(|source| self.error(source))?;
        rows.collect::<Result<Vec<String>, _>>()
            .map_err(|source| self.error(source))
    }

    /// The prompt, the answer, and how a call ended.
    ///
    /// # Errors
    ///
    /// Returns the error of SQLite.
    pub fn call(&self, position: u64) -> Result<Option<CallRecord>, StoreError> {
        let select = "SELECT kind, prompt, answer, result FROM calls WHERE position = ?1";
        self.connection
            .query_row(select, params![as_sql(position)], |row| {
                Ok(CallRecord {
                    kind: row.get(0)?,
                    prompt: row.get(1)?,
                    answer: row.get(2)?,
                    result: row.get(3)?,
                })
            })
            .optional()
            .map_err(|source| self.error(source))
    }

    fn nodes(&self, select: &str, key: u64) -> Result<Vec<Node>, StoreError> {
        let mut statement = self
            .connection
            .prepare(select)
            .map_err(|source| self.error(source))?;
        let rows = statement
            .query_map(params![as_sql(key)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .and_then(Iterator::collect::<rusqlite::Result<Vec<(String, i64)>>>)
            .map_err(|source| self.error(source))?;
        Ok(rows
            .into_iter()
            .filter_map(|(tab, row)| node(&tab, from_sql(row)?))
            .collect())
    }

    fn error(&self, source: rusqlite::Error) -> StoreError {
        sqlite_error(&self.path, source)
    }
}

/// A call as the database keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallRecord {
    pub kind: String,
    pub prompt: Option<String>,
    pub answer: Option<String>,
    pub result: String,
}

fn write_line(transaction: &rusqlite::Transaction<'_>, line: &Line) -> rusqlite::Result<()> {
    if let Some(input) = &line.input {
        transaction.execute(
            "INSERT INTO inputs (position, kind, root, body) VALUES (?1, ?2, ?3, ?4)",
            params![
                as_sql(input.position),
                input.kind,
                input.root.name(),
                input.body
            ],
        )?;
    }
    let (input, call) = match line.origin {
        Some(Origin::Input(position)) => (Some(as_sql(position)), None),
        Some(Origin::Call(position)) => (None, Some(as_sql(position))),
        None => (None, None),
    };
    for new in &line.calls {
        transaction.execute(
            "INSERT INTO calls (position, kind, input, call, pack, prompt, result)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open')",
            params![
                as_sql(new.position),
                new.kind,
                input,
                call,
                new.pack,
                new.prompt
            ],
        )?;
        for read in &new.reads {
            let (tab, row) = address(*read);
            transaction.execute(
                "INSERT INTO reads (call, tab, row) VALUES (?1, ?2, ?3)",
                params![as_sql(new.position), tab, as_sql(row)],
            )?;
        }
    }
    for end in &line.ended {
        transaction.execute(
            "UPDATE calls SET answer = ?2, result = ?3, shape = ?4 WHERE position = ?1",
            params![
                as_sql(end.position),
                end.answer,
                end.outcome.name(),
                end.shape
            ],
        )?;
    }
    for (table, rows) in &line.rows {
        let insert = format!(
            "INSERT INTO {} (position, body, input, call) VALUES (?1, ?2, ?3, ?4)",
            table.name()
        );
        for row in rows {
            transaction.execute(
                &insert,
                params![as_sql(row.position), row.body, input, call],
            )?;
        }
    }
    if let (Some(first), Some(newest)) = (line.calls.first(), line.calls.last()) {
        // The prompts that aged out before this line are cleared already.
        let aged_before = first.position.checked_sub(1).map_or(0, oldest_prompt_kept);
        transaction.execute(
            "UPDATE calls SET prompt = NULL WHERE position >= ?1 AND position < ?2",
            params![
                as_sql(aged_before),
                as_sql(oldest_prompt_kept(newest.position))
            ],
        )?;
    }
    Ok(())
}

fn address(node: Node) -> (&'static str, u64) {
    match node {
        Node::Row(table, row) => (table.name(), row),
        Node::Call(row) => ("calls", row),
        Node::Input(row) => ("inputs", row),
    }
}

fn node(tab: &str, row: u64) -> Option<Node> {
    match tab {
        "calls" => Some(Node::Call(row)),
        "inputs" => Some(Node::Input(row)),
        _ => Table::named(tab).map(|table| Node::Row(table, row)),
    }
}

/// SQLite keeps signed integers. No position comes near the edge.
fn as_sql(position: u64) -> i64 {
    i64::try_from(position).unwrap_or(i64::MAX)
}

fn from_sql(position: i64) -> Option<u64> {
    u64::try_from(position).ok()
}

pub(super) fn sqlite_error(path: &Path, source: rusqlite::Error) -> StoreError {
    StoreError::Sqlite {
        path: path.to_path_buf(),
        source,
    }
}
