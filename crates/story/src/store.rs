//! The history of each character on disk: one JSON line for each event, append-only
//! (GAMEPLAY.md 5.7). The state is never stored. A replay builds it at start.

use crate::character::Character;
use hourglass::{Event, EventId};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// The limits of the bridge for the character line (Gnomish Relay SPEC.md 9.7).
const MAX_REALM_BYTES: usize = 64;
const MAX_NAME_BYTES: usize = 48;

const HEX: &[u8; 16] = b"0123456789ABCDEF";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("history of {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("history of {path}: it does not start with the founding of the character")]
    Foreign { path: PathBuf },
    #[error("character: {0}")]
    BadKey(&'static str),
}

/// Where the histories live. `Memory` keeps nothing after the program stops.
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
        let file = format!("c_{}.jsonl", safe_id(&self.name));
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

/// The file that the new events of one character go to.
pub struct HistoryFile {
    path: PathBuf,
    /// How many events the file holds. The next event to write has this position.
    len: usize,
}

impl HistoryFile {
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write. The file keeps the events that it held before,
    /// and the next call writes the same events again.
    pub fn append(&mut self, events: &[Event]) -> Result<(), StoreError> {
        if events.is_empty() {
            return Ok(());
        }
        let io_error = |source| StoreError::Io {
            path: self.path.clone(),
            source,
        };
        let mut text = String::new();
        for event in events {
            let line = serde_json::to_string(event).map_err(|error| io_error(error.into()))?;
            text.push_str(&line);
            text.push('\n');
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(io_error)?;
        let before = file.metadata().map_err(io_error)?.len();
        let written = file
            .write_all(text.as_bytes())
            .and_then(|()| file.sync_data());
        if let Err(error) = written {
            // A torn write in the middle of the file would end the history there at the
            // next load, so the file goes back to its last good length.
            let _ = file.set_len(before);
            return Err(io_error(error));
        }
        self.len += events.len();
        Ok(())
    }
}

impl Store {
    /// The character with the world of its history, and the file for its new events.
    ///
    /// # Errors
    ///
    /// Returns an I/O error, or `Foreign` for a file that another program wrote.
    pub fn open(&self, key: &CharacterKey) -> Result<(Character, Option<HistoryFile>), StoreError> {
        let Store::Folder(folder) = self else {
            return Ok((Character::new(), None));
        };
        let path = folder.join(key.relative_path());
        let io_error = |source| StoreError::Io {
            path: path.clone(),
            source,
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let events = read_history(&path).map_err(io_error)?;
        let character = if events.is_empty() {
            Character::new()
        } else {
            Character::from_history(&events)
                .ok_or_else(|| StoreError::Foreign { path: path.clone() })?
        };
        let file = HistoryFile {
            path,
            len: events.len(),
        };
        Ok((character, Some(file)))
    }
}

/// The events up to the first line that does not read. A crash in the middle of a write
/// leaves such a line at the end, so the file is cut there, and new events follow the
/// good part.
fn read_history(path: &Path) -> io::Result<Vec<Event>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut events = Vec::new();
    let mut good_bytes = 0;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        let expected = EventId(events.len() as u64);
        match serde_json::from_str::<Event>(line.trim_end()) {
            Ok(event) if read > 0 && line.ends_with('\n') && event.id == expected => {
                events.push(event);
                good_bytes += read as u64;
            }
            _ => break,
        }
    }
    let length = fs::metadata(path)?.len();
    if good_bytes < length {
        OpenOptions::new()
            .write(true)
            .open(path)?
            .set_len(good_bytes)?;
    }
    Ok(events)
}
