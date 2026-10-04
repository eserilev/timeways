//! Who the hero is, in the player's own words: a sheet of short fields, as a player of a
//! tabletop game writes before the first session, and entries of their own lore that they
//! add at any time. It is the hero's own story, never canon: `/lore` never reads it.

use crate::check::one_line;
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// The most that a text holds, in characters and in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limit {
    pub chars: usize,
    pub bytes: usize,
}

/// About 170 words: an entry, or a field that tells a story.
pub const LONG: Limit = Limit {
    chars: 1000,
    bytes: 1200,
};

/// One line, as roleplay addons show a birthplace, a motto, or what you do now.
pub const LINE: Limit = Limit {
    chars: 200,
    bytes: 240,
};

/// A few words, such as a name or an age.
pub const SHORT: Limit = Limit {
    chars: 100,
    bytes: 120,
};

/// A prompt takes this much of each text of the hero, so a long story keeps it short.
pub const PROMPT_TEXT_CHARS: usize = 300;

/// The fields of the sheet, in the order of the page: the six questions, then the roleplay
/// profile that roleplay addons share (GAMEPLAY.md 3.7.1).
pub const FIELDS: [&str; 12] = [
    "origin",
    "background",
    "goal",
    "bond",
    "flaw",
    "traits",
    "name",
    "title",
    "currently",
    "appearance",
    "age",
    "motto",
];

/// The name is the name in the game until the player sets one, and a title often holds it.
/// No model sees the name of a real player (GAMEPLAY.md 5.11).
const NOT_IN_PROMPTS: [&str; 2] = ["name", "title"];

/// None for a name that is no field.
#[must_use]
pub fn limit_of(field: &str) -> Option<Limit> {
    match field {
        "background" | "goal" | "bond" | "flaw" | "traits" | "appearance" => Some(LONG),
        "origin" | "currently" | "motto" => Some(LINE),
        "name" | "title" | "age" => Some(SHORT),
        _ => None,
    }
}

/// The entries that one prompt carries, newest first, so a long story keeps the prompt short.
pub const PROMPT_ENTRIES: usize = 5;

/// The texts of the entries that `keep` takes, newest first and cut short, for a prompt.
#[must_use]
pub fn newest_texts(entries: &[Entry], keep: impl Fn(&Entry) -> bool) -> Vec<&str> {
    let kept = entries.iter().rev().filter(|entry| keep(entry));
    kept.take(PROMPT_ENTRIES)
        .map(|entry| cut(&entry.text))
        .collect()
}

/// The first `PROMPT_TEXT_CHARS` characters of a text.
#[must_use]
pub fn cut(text: &str) -> &str {
    text.char_indices()
        .nth(PROMPT_TEXT_CHARS)
        .map_or(text, |(end, _)| &text[..end])
}

/// Every text of the hero, sheet and entries, one on each line. A check of a model answer
/// allows the later names in it, because the player wrote them first. A text holds no line
/// break, so each line is one text.
#[must_use]
pub fn player_text(hero: &Hero) -> String {
    let fields = hero.sheet.iter().map(|field| field.text.as_str());
    let entries = hero.entries.iter().map(|entry| entry.text.as_str());
    fields.chain(entries).collect::<Vec<_>>().join("\n")
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Hero {
    /// Each field that holds a text, in the order of `FIELDS`.
    pub sheet: Vec<Field>,
    /// The entries that stand, oldest first.
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Field {
    pub field: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Never reused, so a removal names one entry for good.
    pub number: u64,
    pub at: Tick,
    pub text: String,
    /// Where the hero stood when the player wrote it.
    pub place: Option<String>,
    /// The NPC that the player targeted, when the entry is about it.
    pub npc: Option<String>,
}

/// One change of the story of the hero. The file of the hero holds these lines, oldest
/// first, and never loses one: a new text of a field, or a removal, is a new line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Change {
    /// An empty text clears the field.
    Set {
        at: Tick,
        field: String,
        text: String,
    },
    Added(Entry),
    Removed {
        at: Tick,
        number: u64,
    },
}

/// True when the player set a field of the sheet in `[from, to)`.
#[must_use]
pub fn sheet_changed(changes: &[Change], from: Tick, to: Tick) -> bool {
    changes
        .iter()
        .any(|change| matches!(change, Change::Set { at, .. } if *at >= from && *at < to))
}

/// The hero as the changes leave it.
#[must_use]
pub fn hero(changes: &[Change]) -> Hero {
    let mut texts: Vec<Option<String>> = vec![None; FIELDS.len()];
    let mut entries: Vec<Entry> = Vec::new();
    for change in changes {
        match change {
            Change::Set { field, text, .. } => {
                if let Some(index) = FIELDS.iter().position(|name| name == field) {
                    texts[index] = Some(text.clone()).filter(|text| !text.is_empty());
                }
            }
            Change::Added(entry) => entries.push(entry.clone()),
            Change::Removed { number, .. } => entries.retain(|entry| entry.number != *number),
        }
    }
    let sheet = FIELDS
        .iter()
        .zip(texts)
        .filter_map(|(field, text)| {
            Some(Field {
                field: (*field).to_string(),
                text: text?,
            })
        })
        .collect();
    Hero { sheet, entries }
}

/// The number of the next entry.
#[must_use]
pub fn next_number(changes: &[Change]) -> u64 {
    let last = changes.iter().filter_map(|change| match change {
        Change::Added(entry) => Some(entry.number),
        _ => None,
    });
    last.max().map_or(1, |number| number + 1)
}

/// The text on one line, or the reason that it cannot stand. The reason goes back to the
/// player, because an edit has no reply of its own. The words are the player's own, so no
/// check reads them (GAMEPLAY.md 3.7).
///
/// # Errors
///
/// Returns the reason for a text that is empty, too long, or holds a control character.
pub fn checked_text(text: &str, limit: Limit) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err(EMPTY.to_string());
    }
    if text.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        return Err(ODD_CHARACTERS.to_string());
    }
    one_line(text, limit.chars, limit.bytes).ok_or_else(|| TOO_LONG.to_string())
}

const EMPTY: &str = "Couldn't save an empty note.";
const ODD_CHARACTERS: &str =
    "Couldn't save that: it has special characters. Take them out and try again.";
const TOO_LONG: &str = "Couldn't save that: it's too long. Try a shorter version.";

/// The heading of the story of the hero in a prompt. It keeps the player's words apart
/// from canon.
pub const OWN_WORDS: &str =
    "Who our hero is, in the player's own words. It is the hero's own story, not canon:";

/// "Who our hero is", for the prompts of the narrator: a line and a chapter. None for an empty
/// story.
#[must_use]
pub fn portrait(hero: &Hero) -> Option<String> {
    let shown = hero
        .sheet
        .iter()
        .filter(|field| !NOT_IN_PROMPTS.contains(&field.field.as_str()));
    let mut lines: Vec<String> = shown
        .map(|field| format!("- {}: {}", field.field, cut(&field.text)))
        .collect();
    let newest = newest_texts(&hero.entries, |_| true);
    lines.extend(
        newest
            .iter()
            .map(|text| format!("- Told by the player: {text}")),
    );
    (!lines.is_empty()).then(|| lines.join("\n"))
}
