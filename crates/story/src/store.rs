//! The history of each character on disk: one JSON line for each event, append-only
//! (GAMEPLAY.md 5.7). The state is never stored. A replay builds it at start.

use crate::character::Character;
use crate::flavor::{Flavor, Told};
use crate::hero::Change;
use crate::learned::{Read, Rumor};
use crate::quest::QuestChange;
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

/// The words of the bard for one chapter, as one line of the chronicle file. A line from
/// before footnotes has none.
#[derive(Serialize, Deserialize)]
struct ChapterProse {
    began: Tick,
    text: String,
    #[serde(default)]
    footnotes: Vec<String>,
}

/// The saga of one chapter and its footnotes, as the player reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub text: String,
    pub footnotes: Vec<String>,
}

/// The words of the bard for each chapter, by the tick that began the chapter. The world
/// holds facts only, so the words live in a file of their own.
#[derive(Debug, Default)]
pub struct Prose {
    chapters: BTreeMap<Tick, Written>,
    path: Option<PathBuf>,
}

impl Prose {
    #[must_use]
    pub fn get(&self, began: Tick) -> Option<&Written> {
        self.chapters.get(&began)
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add(&mut self, began: Tick, written: Written) -> Result<(), StoreError> {
        if let Some(path) = &self.path {
            let line = ChapterProse {
                began,
                text: written.text.clone(),
                footnotes: written.footnotes.clone(),
            };
            append_lines(path, &[line]).map_err(|source| io_error(path, source))?;
        }
        self.chapters.insert(began, written);
        Ok(())
    }
}

/// One line of the flavor file: a moment, or a telling of a kind of moment.
#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
enum FlavorLine {
    Moment(Flavor),
    Told(Told),
}

/// The flavor moments of a character and the tellings of them (GAMEPLAY.md 5.4.1). They are
/// small and silly, not facts, so they live in a file of their own next to the history.
#[derive(Debug, Default)]
pub struct FlavorLog {
    moments: Vec<Flavor>,
    told: Vec<Told>,
    path: Option<PathBuf>,
}

impl FlavorLog {
    #[must_use]
    pub fn moments(&self) -> &[Flavor] {
        &self.moments
    }

    #[must_use]
    pub fn told(&self) -> &[Told] {
        &self.told
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add_moment(&mut self, moment: Flavor) -> Result<(), StoreError> {
        self.write(&FlavorLine::Moment(moment.clone()))?;
        self.moments.push(moment);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add_told(&mut self, told: Told) -> Result<(), StoreError> {
        self.write(&FlavorLine::Told(told.clone()))?;
        self.told.push(told);
        Ok(())
    }

    fn write(&self, line: &FlavorLine) -> Result<(), StoreError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        append_lines(path, std::slice::from_ref(line)).map_err(|source| io_error(path, source))
    }
}

/// The changes of the story of the hero, oldest first (see `hero`). They are the player's
/// own words, not facts, so they live in a file of their own next to the history.
#[derive(Debug, Default)]
pub struct HeroLog {
    changes: Vec<Change>,
    path: Option<PathBuf>,
}

impl HeroLog {
    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add(&mut self, change: Change) -> Result<(), StoreError> {
        if let Some(path) = &self.path {
            append_lines(path, std::slice::from_ref(&change))
                .map_err(|source| io_error(path, source))?;
        }
        self.changes.push(change);
        Ok(())
    }
}

/// What the player read and heard, oldest first (see `learned`).
#[derive(Debug, Default)]
pub struct LearnedLog {
    read: Vec<Read>,
    rumors: Vec<Rumor>,
    path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
enum LearnedLine {
    Read(Read),
    Rumor(Rumor),
}

impl LearnedLog {
    #[must_use]
    pub fn read(&self) -> &[Read] {
        &self.read
    }

    #[must_use]
    pub fn rumors(&self) -> &[Rumor] {
        &self.rumors
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add_read(&mut self, read: Read) -> Result<(), StoreError> {
        self.write(LearnedLine::Read(read.clone()))?;
        self.read.push(read);
        Ok(())
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add_rumor(&mut self, rumor: Rumor) -> Result<(), StoreError> {
        self.write(LearnedLine::Rumor(rumor.clone()))?;
        self.rumors.push(rumor);
        Ok(())
    }

    fn write(&self, line: LearnedLine) -> Result<(), StoreError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        append_lines(path, &[line]).map_err(|source| io_error(path, source))
    }
}

/// The changes of the side quests, oldest first (see `quest`).
#[derive(Debug, Default)]
pub struct QuestLog {
    changes: Vec<QuestChange>,
    path: Option<PathBuf>,
}

impl QuestLog {
    #[must_use]
    pub fn changes(&self) -> &[QuestChange] {
        &self.changes
    }

    /// # Errors
    ///
    /// Returns the I/O error of the write, and then keeps nothing.
    pub fn add(&mut self, change: QuestChange) -> Result<(), StoreError> {
        if let Some(path) = &self.path {
            append_lines(path, std::slice::from_ref(&change))
                .map_err(|source| io_error(path, source))?;
        }
        self.changes.push(change);
        Ok(())
    }
}

/// What a character brings from the disk.
pub struct Opened {
    pub character: Character,
    pub history: Option<HistoryFile>,
    pub prose: Prose,
    pub flavor: FlavorLog,
    pub hero: HeroLog,
    pub learned: LearnedLog,
    pub quests: QuestLog,
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
                flavor: FlavorLog::default(),
                hero: HeroLog::default(),
                learned: LearnedLog::default(),
                quests: QuestLog::default(),
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
                .map(|line| {
                    let written = Written {
                        text: line.text,
                        footnotes: line.footnotes,
                    };
                    (line.began, written)
                })
                .collect(),
            path: Some(prose_path),
        };
        let flavor_path = path.with_extension("flavor.jsonl");
        let lines: Vec<FlavorLine> = read_lines(&flavor_path, |_, _| true)
            .map_err(|source| io_error(&flavor_path, source))?;
        let mut flavor = FlavorLog {
            path: Some(flavor_path),
            ..FlavorLog::default()
        };
        for line in lines {
            match line {
                FlavorLine::Moment(moment) => flavor.moments.push(moment),
                FlavorLine::Told(told) => flavor.told.push(told),
            }
        }
        let hero_path = path.with_extension("hero.jsonl");
        let changes: Vec<Change> =
            read_lines(&hero_path, |_, _| true).map_err(|source| io_error(&hero_path, source))?;
        let hero = HeroLog {
            changes,
            path: Some(hero_path),
        };
        let learned_path = path.with_extension("learned.jsonl");
        let lines: Vec<LearnedLine> = read_lines(&learned_path, |_, _| true)
            .map_err(|source| io_error(&learned_path, source))?;
        let mut learned = LearnedLog {
            path: Some(learned_path),
            ..LearnedLog::default()
        };
        for line in lines {
            match line {
                LearnedLine::Read(read) => learned.read.push(read),
                LearnedLine::Rumor(rumor) => learned.rumors.push(rumor),
            }
        }
        let quest_path = path.with_extension("quests.jsonl");
        let changes: Vec<QuestChange> =
            read_lines(&quest_path, |_, _| true).map_err(|source| io_error(&quest_path, source))?;
        let quests = QuestLog {
            changes,
            path: Some(quest_path),
        };
        let history = HistoryFile {
            path,
            len: events.len(),
        };
        Ok(Opened {
            character,
            history: Some(history),
            prose,
            flavor,
            hero,
            learned,
            quests,
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
    // Bytes, not a `String`: a crash can cut a line inside a character, and that line is
    // damage to cut, not an error.
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        match serde_json::from_slice::<T>(line.trim_ascii_end()) {
            Ok(item) if read > 0 && line.ends_with(b"\n") && accept(&item, items.len()) => {
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
