//! The player's edits in the story program (docs/plans/chapters.md 11): a new row for each
//! edit that changes something, the text that each entry shows, and the telling that a
//! model reads. An edit is no event, so the chapters and the tales never see it.

use super::{Active, Output, Story, StoryError, aliases};
use crate::aliases::{Marked, unmarked, with_names};
use crate::entry_edits::{
    EditText, EditView, EntryEdit, EntryKey, EntryKind, REFUSED, changes_nothing, is_good_edit,
    shown,
};
use crate::house::first_chars;
use crate::store::{Node, Table};
use hourglass::Tick;
use timeways_rules::entry_edits::Shown;

/// A model reads at most this much of the player's telling.
pub const TELLING_CHARS: usize = 600;

impl Story {
    /// An edit that changes nothing writes no row. One that breaks a limit leaves its
    /// reason for the next journal, because the line has no reply.
    pub(super) fn edit_entry(
        &mut self,
        at: Tick,
        entry: EntryKey,
        title: Option<&str>,
        text: EditText,
        paragraphs: &[String],
    ) -> Result<Vec<Output>, StoryError> {
        let title = title.filter(|title| !title.is_empty()).map(unmarked);
        let marked: Vec<Marked> = paragraphs.iter().map(|text| unmarked(text)).collect();
        let plain = EntryEdit {
            entry,
            title: title.as_ref().map(|marked| marked.text.clone()),
            text,
            paragraphs: marked.iter().map(|marked| marked.text.clone()).collect(),
            at,
        };
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        if !is_good_edit(&plain) {
            active.edit_refused = Some(REFUSED.to_string());
            return Ok(Vec::new());
        }
        let names: Vec<String> = title
            .iter()
            .chain(&marked)
            .flat_map(|marked| marked.names.iter().cloned())
            .collect();
        aliases::learn_names(active, &names)?;
        let edit = EntryEdit {
            title: plain.title.map(|title| aliases::with_ids(active, &title)),
            paragraphs: plain
                .paragraphs
                .iter()
                .map(|text| aliases::with_ids(active, text))
                .collect(),
            ..plain
        };
        active.edit_refused = None;
        let standing = standing_edit(active, entry).map(|(_, edit)| edit);
        if !changes_nothing(&edit, standing) {
            active.entry_edits.add(edit)?;
        }
        Ok(Vec::new())
    }
}

/// The edits of one entry, with their rows, oldest first.
pub(super) fn edits_of(active: &Active, entry: EntryKey) -> Vec<(u64, &EntryEdit)> {
    active
        .entry_edits
        .with_rows()
        .filter(|(_, edit)| edit.entry == entry)
        .collect()
}

/// The newest edit of an entry: the one that stands.
pub(super) fn standing_edit(active: &Active, entry: EntryKey) -> Option<(u64, &EntryEdit)> {
    edits_of(active, entry).into_iter().last()
}

/// What an entry shows, from the rows of its narrator texts.
pub(super) fn shown_of(active: &Active, entry: EntryKey, narrator_rows: &[u64]) -> Shown {
    shown(narrator_rows, &edits_of(active, entry))
}

/// The edit of an entry whose words show, with its row: the one whose title or paragraphs
/// the proved rule shows. A narrator row never changes either (theorem 21), so none is
/// read. A restore shows no words of the player, so it gives none.
fn shown_edit(active: &Active, entry: EntryKey) -> Option<(u64, &EntryEdit)> {
    let edits = edits_of(active, entry);
    let shown = shown(&[], &edits);
    let row = shown.player.or(shown.title)?;
    edits.into_iter().find(|(id, _)| *id == row)
}

/// The edit of each entry that shows words of the player, with the names of the players.
pub(super) fn journal_edits(active: &Active) -> Vec<EditView> {
    let mut entries: Vec<EntryKey> = Vec::new();
    for edit in active.entry_edits.rows() {
        if !entries.contains(&edit.entry) {
            entries.push(edit.entry);
        }
    }
    let table = active.aliases.table();
    entries
        .into_iter()
        .filter_map(|entry| shown_edit(active, entry))
        .map(|(_, edit)| EditView {
            entry: edit.entry,
            title: edit.title.as_ref().map(|title| with_names(table, title)),
            text: edit.text,
            paragraphs: edit
                .paragraphs
                .iter()
                .map(|text| with_names(table, text))
                .collect(),
        })
        .collect()
}

/// The player's telling of an entry for a prompt, with the IDs of the players, cut short,
/// and the row behind it. None when the entry shows no words of the player. The swap runs
/// again, because the table can know a name now that it did not know at the edit.
pub(super) fn telling_of(active: &Active, entry: EntryKey) -> Option<(String, Node)> {
    let (row, edit) = shown_edit(active, entry)?;
    let mut parts: Vec<&str> = edit.title.iter().map(String::as_str).collect();
    parts.extend(edit.paragraphs.iter().map(String::as_str));
    let with_ids = aliases::with_ids(active, &parts.join("\n"));
    let telling = first_chars(&with_ids, TELLING_CHARS).to_string();
    Some((telling, Node::Row(Table::EntryEdits, row)))
}

/// The key of a chapter, a tale, or the summary.
pub(super) fn chapter_key(first: u64) -> EntryKey {
    EntryKey {
        kind: EntryKind::Chapter,
        first: Some(first),
    }
}

pub(super) fn tale_key(first: u64) -> EntryKey {
    EntryKey {
        kind: EntryKind::Tale,
        first: Some(first),
    }
}

pub(super) const SUMMARY_KEY: EntryKey = EntryKey {
    kind: EntryKind::Summary,
    first: None,
};
