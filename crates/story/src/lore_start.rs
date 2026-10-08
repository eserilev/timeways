//! The lore pack at the start of the story program. The desktop app builds the pack after
//! setup, in the background, and starts the story program again when the pack is in place
//! (GAMEPLAY.md 5.10, relay SPEC 11.4).

use crate::pack::{FORMAT_VERSION, Pack, PackError};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A file in the story folder while the story program runs with no pack.
pub const BUILDING_MARK: &str = "lore-building";

/// The chat line at the first start with a pack, after a start with none.
pub const LORE_READY: &str = "Lore is ready.";

/// The pack to start with, the notice for the first answer, and a line for the log.
pub struct LoreStart {
    pub pack: Pack,
    pub notice: Option<String>,
    pub log: Option<String>,
}

/// With no file at `pack`, or with a pack of another format version, the program starts
/// with an empty pack and leaves the mark. It never writes, moves, or deletes the pack:
/// the desktop app does. The relay contract: the mark asks the desktop app for a new
/// pack, and the app compares the format of a pack with `timeways-pack format`. `folder`
/// is None for a run with no data folder, which keeps no mark.
///
/// # Errors
///
/// Returns the error of the pack, or of the mark in the folder. A pack of another format
/// is no error.
pub fn open_lore(pack: &Path, folder: Option<&Path>) -> Result<LoreStart, LoreStartError> {
    if !pack.exists() {
        return start_with_no_lore(folder, None);
    }
    let pack = match Pack::open(pack) {
        Err(PackError::Version { found }) => {
            let log = format!(
                "lore pack format {found}, this program reads {FORMAT_VERSION}: \
                 starting with no lore until it is rebuilt"
            );
            return start_with_no_lore(folder, Some(log));
        }
        opened => opened?,
    };
    let mark = folder.map(|folder| folder.join(BUILDING_MARK));
    let was_building = mark.as_ref().is_some_and(|mark| mark.exists());
    if let Some(mark) = mark.filter(|_| was_building) {
        fs::remove_file(mark)?;
    }
    Ok(LoreStart {
        pack,
        notice: was_building.then(|| LORE_READY.to_string()),
        log: None,
    })
}

fn start_with_no_lore(
    folder: Option<&Path>,
    log: Option<String>,
) -> Result<LoreStart, LoreStartError> {
    if let Some(folder) = folder {
        fs::create_dir_all(folder)?;
        fs::write(folder.join(BUILDING_MARK), "")?;
    }
    Ok(LoreStart {
        pack: Pack::empty()?,
        notice: None,
        log,
    })
}

#[derive(Debug, Error)]
pub enum LoreStartError {
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error("lore mark: {0}")]
    Mark(#[from] std::io::Error),
}
