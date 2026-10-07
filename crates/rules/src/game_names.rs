//! Game names (docs/plans/lore-names-and-now.md 1). The pack names a person as the wiki
//! does, and the game names the same person in its own way: "Sally Whitemane" is "High
//! Inquisitor Whitemane". A row says that two exact names are one person. Nothing else
//! matches: no title is stripped, and no surname is enough. The story program gives each
//! name an id, so the rules read ids. Lean proves their laws
//! (lean/Timeways/GameNames.lean).
//!
//! The functions walk by index: Aeneas translates no iterator adapter (lean/README.md).

/// One game name of a person, and the wiki name of that person, as ids. The rows are a
/// function: no game name sits in two rows, and no game name is the wiki name of a row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NameRow {
    pub game: u32,
    pub wiki: u32,
}

/// The wiki name of the person that `id` names: the wiki name of its row, or `id` itself
/// when no row holds it as a game name. An index loop.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn person(rows: &[NameRow], id: u32) -> u32 {
    let mut index = 0;
    while index < rows.len() {
        if rows[index].game == id {
            return rows[index].wiki;
        }
        index += 1;
    }
    id
}

/// True when a name of `ids` names the same person as `id`. An index loop.
#[must_use]
pub fn holds_person(rows: &[NameRow], ids: &[u32], id: u32) -> bool {
    let wanted = person(rows, id);
    let mut index = 0;
    while index < ids.len() {
        if person(rows, ids[index]) == wanted {
            return true;
        }
        index += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITEMANE: [NameRow; 1] = [NameRow { game: 1, wiki: 2 }];

    #[test]
    fn a_game_name_gives_the_wiki_name_of_its_row() {
        assert_eq!(person(&WHITEMANE, 1), 2);
    }

    #[test]
    fn a_name_of_no_row_stays_as_it_is() {
        assert_eq!(person(&WHITEMANE, 2), 2);
        assert_eq!(person(&WHITEMANE, 7), 7);
    }

    #[test]
    fn either_name_of_a_row_is_held_by_the_other() {
        assert!(holds_person(&WHITEMANE, &[1], 2));
        assert!(holds_person(&WHITEMANE, &[2], 1));
        assert!(!holds_person(&WHITEMANE, &[7], 2));
    }
}
