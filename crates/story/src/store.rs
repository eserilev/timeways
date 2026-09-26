//! The history of each character on disk: one JSON line for each event, append-only
//! (GAMEPLAY.md 5.7). The state is never stored. A replay builds it at start.

use crate::character::Character;
use hourglass::{Event, EventId, Tick};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
        append_lines(&self.path, events).map_err(|source| io_error(&self.path, source))?;
        self.len += events.len();
        Ok(())
    }
}

/// The words of the bard for one chapter, as one line of the chronicle file.
#[derive(Serialize, Deserialize)]
struct ChapterProse {
    began: Tick,
    text: String,
}

/// The words of the bard for each chapter, by the tick that began the chapter. The world
/// holds facts only, so the words live in a file of their own.
#[derive(Debug, Default)]
pub struct Prose {
    chapters: BTreeMap<Tick, String>,
    path: Option<PathBuf>,
}

impl Prose {
    #[must_use]
    pub fn get(&self, began: Tick) -> Option<&str> {
        self.chapters.get(&began).map(String::as_str)
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add(&mut self, began: Tick, text: String) -> Result<(), StoreError> {
        if let Some(path) = &self.path {
            let line = ChapterProse {
                began,
                text: text.clone(),
            };
            append_lines(path, &[line]).map_err(|source| io_error(path, source))?;
        }
        self.chapters.insert(began, text);
        Ok(())
    }
}

/// What a character brings from the disk.
pub struct Opened {
    pub character: Character,
    pub history: Option<HistoryFile>,
    pub prose: Prose,
}

impl Store {
    /// # Errors
    ///
    /// Returns an I/O error, or `Foreign` for a history that another program wrote.
    pub fn open(&self, key: &CharacterKey) -> Result<Opened, StoreError> {
        let Store::Folder(folder) = self else {
            let character = Character::new();
            return Ok(Opened {
                character,
                history: None,
                prose: Prose::default(),
            });
        };
        let path = folder.join(key.relative_path());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| io_error(&path, source))?;
        }
        let events: Vec<Event> =
            read_lines(&path, |event: &Event, n| event.id == EventId(n as u64))
                .map_err(|source| io_error(&path, source))?;
        let character = if events.is_empty() {
            Character::new()
        } else {
            Character::from_history(&events)
                .ok_or_else(|| StoreError::Foreign { path: path.clone() })?
        };
        let prose_path = path.with_extension("chronicle.jsonl");
        let lines: Vec<ChapterProse> =
            read_lines(&prose_path, |_, _| true).map_err(|source| io_error(&prose_path, source))?;
        let prose = Prose {
            chapters: lines
                .into_iter()
                .map(|line| (line.began, line.text))
                .collect(),
            path: Some(prose_path),
        };
        let history = HistoryFile {
            path,
            len: events.len(),
        };
        Ok(Opened {
            character,
            history: Some(history),
            prose,
        })
    }
}

fn io_error(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Appends one JSON line for each item. A failed write puts the file back to its length
/// before, because a torn line in the middle would end the file there at the next read.
fn append_lines<T: Serialize>(path: &Path, items: &[T]) -> io::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let mut text = String::new();
    for item in items {
        text.push_str(&serde_json::to_string(item).map_err(io::Error::from)?);
        text.push('\n');
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let before = file.metadata()?.len();
    let written = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_data());
    if let Err(error) = written {
        let _ = file.set_len(before);
        return Err(error);
    }
    Ok(())
}

/// The items up to the first line that does not read, or that `accept` refuses at its
/// position. A crash in the middle of a write leaves such a line at the end, so the file
/// is cut there, and new lines follow the good part.
fn read_lines<T: DeserializeOwned>(
    path: &Path,
    accept: impl Fn(&T, usize) -> bool,
) -> io::Result<Vec<T>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut items = Vec::new();
    let mut good_bytes = 0;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        match serde_json::from_str::<T>(line.trim_end()) {
            Ok(item) if read > 0 && line.ends_with('\n') && accept(&item, items.len()) => {
                items.push(item);
                good_bytes += read as u64;
            }
            _ => break,
        }
    }
    if good_bytes < fs::metadata(path)?.len() {
        OpenOptions::new()
            .write(true)
            .open(path)?
            .set_len(good_bytes)?;
    }
    Ok(items)
}
