//! The world files of the data folder, their backups, and their snapshots. Each command
//! touches the files of one character only.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use timeways_story::store::{CharacterKey, StoreError, safe_id};

/// SQLite keeps the newest writes of a world in these files next to it.
const SIDE_FILES: [&str; 2] = ["-wal", "-shm"];

/// The snapshots live next to the worlds, in the data folder of the story program.
const SNAPSHOTS: &str = "dev-snapshots";

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("{0} exists. Add --replace to move it to a backup and write a new one.")]
    Exists(PathBuf),
    #[error("{0} does not exist")]
    Missing(PathBuf),
    #[error("a snapshot name holds only letters, digits, '-', and '_'")]
    BadSnapshotName,
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    #[error(transparent)]
    Key(#[from] StoreError),
}

/// Whether a command writes over a world that exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Existing {
    Keep,
    /// Move it to a backup first.
    Replace,
}

/// The world file of a character in the data folder of the story program.
///
/// # Errors
///
/// Returns `Key` for a realm or a name that the story program refuses.
pub fn world_file(data: &Path, realm: &str, name: &str) -> Result<PathBuf, WorldError> {
    Ok(data.join(CharacterKey::new(realm, name)?.relative_path()))
}

/// The file of a snapshot of a character.
///
/// # Errors
///
/// Returns `BadSnapshotName` for a name that is not a plain file name.
pub fn snapshot_file(
    data: &Path,
    realm: &str,
    name: &str,
    snapshot: &str,
) -> Result<PathBuf, WorldError> {
    let plain = !snapshot.is_empty()
        && snapshot
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !plain {
        return Err(WorldError::BadSnapshotName);
    }
    Ok(data
        .join(SNAPSHOTS)
        .join(format!("r_{}", safe_id(realm)))
        .join(format!("c_{}", safe_id(name)))
        .join(format!("{snapshot}.sqlite")))
}

/// Puts `new` in the place of the world `world`. A world that exists moves to
/// `<file>.bak-<seconds>` first, and only with `Existing::Replace`. Returns the backup.
///
/// # Errors
///
/// Returns `Exists` for a world that exists with `Existing::Keep`, and the error of the
/// file system.
pub fn put_world(
    new: &Path,
    world: &Path,
    existing: Existing,
) -> Result<Option<PathBuf>, WorldError> {
    let backup = if world.exists() {
        if existing == Existing::Keep {
            return Err(WorldError::Exists(world.to_path_buf()));
        }
        Some(back_up(world)?)
    } else {
        None
    };
    if let Some(parent) = world.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    // A copy, then a rename, so the story program never opens half a file.
    let part = with_suffix(world, ".new");
    fs::copy(new, &part).map_err(|source| io_error(&part, source))?;
    fs::rename(&part, world).map_err(|source| io_error(world, source))?;
    Ok(backup)
}

/// Moves the world and its side files to `<file>.bak-<seconds>`, so the backup opens as a
/// world of its own. A second backup in the same second gets `-2`, and so on: a backup
/// never takes the place of another.
fn back_up(world: &Path) -> Result<PathBuf, WorldError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let mut backup = with_suffix(world, &format!(".bak-{seconds}"));
    let mut count = 1;
    while backup.exists() {
        count += 1;
        backup = with_suffix(world, &format!(".bak-{seconds}-{count}"));
    }
    for side in SIDE_FILES {
        let from = with_suffix(world, side);
        if from.exists() {
            let to = with_suffix(&backup, side);
            fs::rename(&from, &to).map_err(|source| io_error(&from, source))?;
        }
    }
    fs::rename(world, &backup).map_err(|source| io_error(world, source))?;
    Ok(backup)
}

/// Saves the world as one file, with the newest writes in it, also while the story program
/// has it open.
///
/// # Errors
///
/// Returns `Missing` for a world that does not exist, `Exists` for a snapshot that does,
/// and the error of SQLite.
pub fn save_snapshot(world: &Path, snapshot: &Path) -> Result<(), WorldError> {
    if !world.exists() {
        return Err(WorldError::Missing(world.to_path_buf()));
    }
    if snapshot.exists() {
        return Err(WorldError::Exists(snapshot.to_path_buf()));
    }
    if let Some(parent) = snapshot.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    let sqlite = |source| WorldError::Sqlite {
        path: world.to_path_buf(),
        source,
    };
    let connection = rusqlite::Connection::open(world).map_err(sqlite)?;
    connection
        .execute("VACUUM INTO ?1", [snapshot.to_string_lossy()])
        .map_err(sqlite)?;
    Ok(())
}

/// Puts the snapshot in the place of the world. The world that exists moves to a backup.
///
/// # Errors
///
/// Returns `Missing` for a snapshot that does not exist, and the error of the file system.
pub fn restore_snapshot(snapshot: &Path, world: &Path) -> Result<Option<PathBuf>, WorldError> {
    if !snapshot.exists() {
        return Err(WorldError::Missing(snapshot.to_path_buf()));
    }
    put_world(snapshot, world, Existing::Replace)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn io_error(path: &Path, source: io::Error) -> WorldError {
    WorldError::Io {
        path: path.to_path_buf(),
        source,
    }
}
