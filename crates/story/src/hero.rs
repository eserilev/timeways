//! Who the hero is, in the player's own words: a sheet of short fields, as a player of a
//! tabletop game writes before the first session, and entries of their own lore that they
//! add at any time. It is the hero's own story, never canon: `/lore` never reads it.

use crate::check::plain_text;
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// About 50 words for a field or an entry.
pub const MAX_TEXT_CHARS: usize = 300;
const MAX_TEXT_BYTES: usize = 1200;

/// The fields of the sheet, in the order of the page.
pub const FIELDS: [&str; 6] = ["origin", "background", "goal", "bond", "flaw", "traits"];

/// The entries that one prompt carries, newest first, so a long story keeps the prompt short.
pub const PROMPT_ENTRIES: usize = 5;

/// The texts of the entries that `keep` takes, newest first, for a prompt.
#[must_use]
pub fn newest_texts(entries: &[Entry], keep: impl Fn(&Entry) -> bool) -> Vec<&str> {
    let kept = entries.iter().rev().filter(|entry| keep(entry));
    kept.take(PROMPT_ENTRIES)
        .map(|entry| entry.text.as_str())
        .collect()
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
/// player, because an edit has no reply of its own.
///
/// # Errors
///
/// Returns the reason for a text that is too long, holds a control character, or names
/// something from after the lore cutoff.
pub fn checked_text(text: &str) -> Result<String, String> {
    plain_text(text, MAX_TEXT_CHARS, MAX_TEXT_BYTES).ok_or_else(|| {
        format!(
            "Not saved: a text holds at most {MAX_TEXT_CHARS} characters, and no name from \
             after the year 25 ADP."
        )
    })
}

/// The heading of the story of the hero in a prompt. It keeps the player's words apart
/// from canon.
pub const OWN_WORDS: &str =
    "Who our hero is, in the player's own words. It is the hero's own story, not canon:";

/// "Who our hero is", for the prompts of the narrator: a line and a chapter. None for an empty
/// story.
#[must_use]
pub fn portrait(hero: &Hero) -> Option<String> {
    let mut lines: Vec<String> = hero
        .sheet
        .iter()
        .map(|field| format!("- {}: {}", field.field, field.text))
        .collect();
    let newest = hero.entries.iter().rev().take(PROMPT_ENTRIES);
    lines.extend(newest.map(|entry| format!("- Told by the player: {}", entry.text)));
    (!lines.is_empty()).then(|| lines.join("\n"))
}
