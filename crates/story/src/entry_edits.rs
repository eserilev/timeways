//! The player's edits of a chapter, a tale, or the summary (docs/plans/chapters.md 11). The
//! player changes what an entry says, never which events it holds. An edit is a new row,
//! and the newest row of an entry stands. Which text shows is a proved rule
//! (`timeways_rules::entry_edits::shown`).

use crate::stories::is_good_story;
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use timeways_rules::entry_edits::{self as rules, EditRow, Shown};

/// What kind of entry an edit is about. A chapter and a tale can start at one event, so
/// the kind is part of the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Chapter,
    Tale,
    Summary,
}

/// The entry of an edit: its kind, and the first event of a chapter or a tale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryKey {
    pub kind: EntryKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<u64>,
}

/// What the text of an edit does with the text of the narrator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditText {
    /// The narrator's text stays, and the player's paragraphs follow it.
    Keep,
    /// The player's paragraphs stand in place of the narrator's text.
    Replace,
    /// The narrator's text alone. A restore is this with no title.
    Narrator,
}

/// One row of `entry_edits`. Each name of a player is its ID (GAMEPLAY.md 5.11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryEdit {
    pub entry: EntryKey,
    pub title: Option<String>,
    pub text: EditText,
    pub paragraphs: Vec<String>,
    pub at: Tick,
}

/// The standing edit of an entry, as the journal sends it. Each name is the player's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditView {
    pub entry: EntryKey,
    pub title: Option<String>,
    pub text: EditText,
    pub paragraphs: Vec<String>,
}

/// The reason of a refused edit, for the next journal. The addon checks the same limits
/// before Save, so only a damaged line meets it.
pub const REFUSED: &str = "Your edit didn't save. Shorten it a little and try again.";

/// The summary is one paragraph with no title, so it can stand as the History of the
/// Roleplay Profile.
const SUMMARY_PARAGRAPHS: usize = 1;

/// True for an edit that keeps the limits of a story (docs/plans/hero-stories.md 3.1): a
/// title of 60 letters, and 1 to 20 paragraphs of 1000 letters together. A narrator text
/// has no paragraphs. A summary has no title, and only its own paragraph or the narrator's.
#[must_use]
pub fn is_good_edit(edit: &EntryEdit) -> bool {
    let entry_fits = match edit.entry.kind {
        EntryKind::Summary => {
            edit.entry.first.is_none()
                && edit.title.is_none()
                && match edit.text {
                    EditText::Replace => edit.paragraphs.len() == SUMMARY_PARAGRAPHS,
                    EditText::Narrator => true,
                    EditText::Keep => false,
                }
        }
        EntryKind::Chapter | EntryKind::Tale => edit.entry.first.is_some(),
    };
    let text_fits = match edit.text {
        EditText::Narrator => {
            edit.paragraphs.is_empty() && edit.title.as_deref().is_none_or(title_only)
        }
        EditText::Keep | EditText::Replace => {
            is_good_story(edit.title.as_deref(), &edit.paragraphs)
        }
    };
    entry_fits && text_fits
}

/// A title alone goes through the limits of a story with one paragraph.
fn title_only(title: &str) -> bool {
    is_good_story(Some(title), &["x".to_string()])
}

/// True when the edit says what the edit that stands says. Such a row is not written.
#[must_use]
pub fn changes_nothing(edit: &EntryEdit, standing: Option<&EntryEdit>) -> bool {
    standing.is_some_and(|old| {
        old.title == edit.title && old.text == edit.text && old.paragraphs == edit.paragraphs
    })
}

fn rule_text(text: EditText) -> rules::EditText {
    match text {
        EditText::Keep => rules::EditText::Keep,
        EditText::Replace => rules::EditText::Replace,
        EditText::Narrator => rules::EditText::Narrator,
    }
}

/// What an entry shows, as rows: the edit rows of the entry with their row ids, and the
/// rows of its narrator texts.
#[must_use]
pub fn shown(narrator_rows: &[u64], edits: &[(u64, &EntryEdit)]) -> Shown {
    let rows: Vec<EditRow> = edits
        .iter()
        .map(|(row, edit)| EditRow {
            row: *row,
            has_title: edit.title.is_some(),
            text: rule_text(edit.text),
        })
        .collect();
    rules::shown(narrator_rows, &rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(kind: EntryKind, first: Option<u64>, text: EditText, paragraphs: &[&str]) -> EntryEdit {
        EntryEdit {
            entry: EntryKey { kind, first },
            title: None,
            text,
            paragraphs: paragraphs.iter().map(ToString::to_string).collect(),
            at: Tick(1),
        }
    }

    #[test]
    fn a_note_on_a_chapter_is_a_good_edit() {
        let note = edit(EntryKind::Chapter, Some(4), EditText::Keep, &["We sang."]);

        assert!(is_good_edit(&note));
    }

    #[test]
    fn a_summary_takes_one_paragraph_in_place_of_the_narrator_and_no_title() {
        let one = edit(EntryKind::Summary, None, EditText::Replace, &["Who I am."]);
        let two = edit(
            EntryKind::Summary,
            None,
            EditText::Replace,
            &["One.", "Two."],
        );
        let kept = edit(EntryKind::Summary, None, EditText::Keep, &["More."]);
        let titled = EntryEdit {
            title: Some("Me".to_string()),
            ..one.clone()
        };

        assert!(is_good_edit(&one));
        assert!(!is_good_edit(&two));
        assert!(!is_good_edit(&kept));
        assert!(!is_good_edit(&titled));
    }

    #[test]
    fn a_restore_has_no_paragraphs() {
        let restore = edit(EntryKind::Tale, Some(9), EditText::Narrator, &[]);
        let odd = edit(EntryKind::Tale, Some(9), EditText::Narrator, &["x"]);

        assert!(is_good_edit(&restore));
        assert!(!is_good_edit(&odd));
    }

    #[test]
    fn a_paragraph_with_a_bar_is_refused() {
        let bar = edit(EntryKind::Chapter, Some(4), EditText::Keep, &["a | b"]);

        assert!(!is_good_edit(&bar));
    }

    #[test]
    fn the_same_edit_again_changes_nothing() {
        let note = edit(EntryKind::Chapter, Some(4), EditText::Keep, &["We sang."]);

        assert!(changes_nothing(&note, Some(&note)));
        assert!(!changes_nothing(&note, None));
    }
}
