//! Which text of a chapter, a tale, or the summary shows (docs/plans/chapters.md 11). The
//! player can change what an entry says, never which events it holds. The rules work over
//! row ids and the kind of an edit, never over strings. The newest row by id stands.
//!
//! The functions walk by index: Aeneas translates no iterator adapter (lean/README.md).

/// What the text of an edit does with the text of the narrator.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditText {
    /// The narrator's text stays, and the player's paragraphs follow it.
    Keep,
    /// The player's paragraphs stand in place of the narrator's text.
    Replace,
    /// The narrator's text alone. A restore is this with no title.
    Narrator,
}

/// A row of `entry_edits`, as the rules read it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EditRow {
    pub row: u64,
    /// The row gives the player's title. Else the title of the code shows.
    pub has_title: bool,
    pub text: EditText,
}

/// What an entry shows, as rows. The story program reads the texts of these rows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Shown {
    /// The edit whose title shows, or None for the title of the code.
    pub title: Option<u64>,
    /// The narrator row whose text shows, or None for the plain list.
    pub narrator: Option<u64>,
    /// The edit whose paragraphs show, after the narrator's text or in its place.
    pub player: Option<u64>,
}

/// The largest row id. An index loop.
#[must_use]
pub fn newest_row(rows: &[u64]) -> Option<u64> {
    let mut newest = None;
    let mut index = 0;
    while index < rows.len() {
        let row = rows[index];
        let older = match newest {
            Some(best) => best > row,
            None => false,
        };
        if !older {
            newest = Some(row);
        }
        index += 1;
    }
    newest
}

/// The edit with the largest row id. An index loop.
#[must_use]
pub fn newest_edit(edits: &[EditRow]) -> Option<EditRow> {
    let mut newest: Option<EditRow> = None;
    let mut index = 0;
    while index < edits.len() {
        let edit = edits[index];
        let older = match newest {
            Some(best) => best.row > edit.row,
            None => false,
        };
        if !older {
            newest = Some(edit);
        }
        index += 1;
    }
    newest
}

/// What an entry shows, from its narrator rows and its edit rows.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn shown(narrator_rows: &[u64], edits: &[EditRow]) -> Shown {
    let narrator = newest_row(narrator_rows);
    let Some(edit) = newest_edit(edits) else {
        return Shown {
            title: None,
            narrator,
            player: None,
        };
    };
    let title = if edit.has_title { Some(edit.row) } else { None };
    match edit.text {
        EditText::Keep => Shown {
            title,
            narrator,
            player: Some(edit.row),
        },
        EditText::Replace => Shown {
            title,
            narrator: None,
            player: Some(edit.row),
        },
        EditText::Narrator => Shown {
            title,
            narrator,
            player: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEEP: EditRow = EditRow {
        row: 4,
        has_title: true,
        text: EditText::Keep,
    };
    const REPLACE: EditRow = EditRow {
        row: 5,
        has_title: false,
        text: EditText::Replace,
    };
    const RESTORE: EditRow = EditRow {
        row: 6,
        has_title: false,
        text: EditText::Narrator,
    };

    #[test]
    fn an_entry_with_no_edit_shows_the_newest_narrator_text() {
        let shown = shown(&[3, 9, 7], &[]);

        assert_eq!(
            shown,
            Shown {
                title: None,
                narrator: Some(9),
                player: None
            }
        );
    }

    #[test]
    fn a_saga_after_a_note_on_the_open_chapter_shows_above_it() {
        let before = shown(&[], &[KEEP]);

        let after = shown(&[1], &[KEEP]);

        assert_eq!(before.narrator, None);
        assert_eq!(after.narrator, Some(1));
        assert_eq!(after.player, Some(KEEP.row));
        assert_eq!(after.title, Some(KEEP.row));
    }

    #[test]
    fn a_saga_after_a_replace_does_not_show() {
        let shown = shown(&[1, 8], &[KEEP, REPLACE]);

        assert_eq!(shown.narrator, None);
        assert_eq!(shown.player, Some(REPLACE.row));
        assert_eq!(shown.title, None);
    }

    #[test]
    fn restore_shows_the_newest_narrator_text() {
        let shown = shown(&[1, 8], &[KEEP, REPLACE, RESTORE]);

        assert_eq!(
            shown,
            Shown {
                title: None,
                narrator: Some(8),
                player: None
            }
        );
    }

    #[test]
    fn the_newest_edit_by_row_stands_in_any_order() {
        let shown = shown(&[], &[RESTORE, KEEP]);

        assert_eq!(shown.player, None);
    }
}
