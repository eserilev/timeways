//! The lore pack at the start of the story program. The desktop app builds the pack after
//! setup, in the background, and starts the story program again when the pack is in place
//! (GAMEPLAY.md 5.10, relay SPEC 11.4).

use crate::pack::{Pack, PackError};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// A file in the story folder while the story program runs with no pack.
pub const BUILDING_MARK: &str = "lore-building";

/// The chat line at the first start with a pack, after a start with none.
pub const LORE_READY: &str = "Lore is ready.";

/// The pack to start with, and the notice for the first answer.
pub struct LoreStart {
    pub pack: Pack,
    pub notice: Option<String>,
}

/// With no file at `pack`, the program starts with an empty pack and leaves the mark. It
/// never writes the pack: the desktop app does. `folder` is None for a run with no data
/// folder, which keeps no mark.
///
/// # Errors
///
/// Returns the error of the pack, or of the mark in the folder.
pub fn open_lore(pack: &Path, folder: Option<&Path>) -> Result<LoreStart, LoreStartError> {
    let mark = folder.map(|folder| folder.join(BUILDING_MARK));
    if !pack.exists() {
        if let (Some(mark), Some(folder)) = (&mark, folder) {
            fs::create_dir_all(folder)?;
            fs::write(mark, "")?;
        }
        return Ok(LoreStart {
            pack: Pack::empty()?,
            notice: None,
        });
    }
    let pack = Pack::open(pack)?;
    let was_building = mark.as_ref().is_some_and(|mark| mark.exists());
    if let Some(mark) = mark.filter(|_| was_building) {
        fs::remove_file(mark)?;
    }
    Ok(LoreStart {
        pack,
        notice: was_building.then(|| LORE_READY.to_string()),
    })
}

#[derive(Debug, Error)]
pub enum LoreStartError {
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error("lore mark: {0}")]
    Mark(#[from] std::io::Error),
}
