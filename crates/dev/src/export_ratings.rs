//! `timeways-dev export-ratings`: the ratings of one character as a file that the player
//! can send (GAMEPLAY.md 3.2.2). The file holds the rated texts, their moments, the
//! ratings, the model, and the faults, and never the name of a real player.

use serde::Serialize;
use std::path::{Path, PathBuf};
use thiserror::Error;
use timeways_story::aliases::{AliasRow, alias_of};
use timeways_story::ratings::{ExportedRating, RatedLine, export};
use timeways_story::store::{RowLog, StoreError, Table};

/// The version of the file. A change of its shape takes the next one. Version 2 added
/// `reason`.
const FORMAT: u32 = 2;

/// The exports live next to the worlds, in the data folder of the story program.
const EXPORTS: &str = "exports";

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("{0} does not exist")]
    Missing(PathBuf),
    #[error("{path}: {source}")]
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("a row does not read: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// What the file holds.
#[derive(Debug, Serialize)]
pub struct Export {
    pub format: u32,
    pub ratings: Vec<ExportedRating>,
}

/// The ratings of the world file `world`, whose character is `own`. It only reads the
/// world, also while the story program has it open.
///
/// # Errors
///
/// Returns `Missing` for a world that does not exist, and the error of SQLite or of a row.
pub fn ratings_of(world: &Path, own: &str, model: &str) -> Result<Export, ExportError> {
    if !world.exists() {
        return Err(ExportError::Missing(world.to_path_buf()));
    }
    let sqlite = |source| ExportError::Sqlite {
        path: world.to_path_buf(),
        source,
    };
    let connection =
        rusqlite::Connection::open_with_flags(world, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(sqlite)?;
    let mut ratings = RowLog::default();
    for body in bodies(&connection, Table::Ratings).map_err(sqlite)? {
        ratings.add(serde_json::from_str::<RatedLine>(&body)?)?;
    }
    let mut players = Vec::new();
    for body in bodies(&connection, Table::Aliases).map_err(sqlite)? {
        let row: AliasRow = serde_json::from_str(&body)?;
        players.extend(alias_of(&row.name));
    }
    Ok(Export {
        format: FORMAT,
        ratings: export(&ratings, &players, own, model),
    })
}

/// The rows of a table, oldest first. A world of a version before the ratings has no
/// table of ratings, and so no ratings.
fn bodies(connection: &rusqlite::Connection, table: Table) -> rusqlite::Result<Vec<String>> {
    let query = format!("SELECT body FROM {} ORDER BY position", table.name());
    let Ok(mut statement) = connection.prepare(&query) else {
        return Ok(Vec::new());
    };
    let rows = statement.query_map([], |row| row.get(0))?;
    rows.collect()
}

/// Writes the export into `<data>/exports/ratings-<stamp>.json`, or into `out`. Returns
/// the file.
///
/// # Errors
///
/// Returns the error of the file system.
pub fn write_export(
    export: &Export,
    data: &Path,
    out: Option<&Path>,
    stamp: u64,
) -> Result<PathBuf, ExportError> {
    let file = match out {
        Some(out) => out.to_path_buf(),
        None => data.join(EXPORTS).join(format!("ratings-{stamp}.json")),
    };
    let io = |source| ExportError::Io {
        path: file.clone(),
        source,
    };
    if let Some(parent) = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    let text = serde_json::to_string_pretty(export)?;
    std::fs::write(&file, text + "\n").map_err(io)?;
    Ok(file)
}
