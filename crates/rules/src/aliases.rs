//! The alias table (GAMEPLAY.md 5.11): each player gets an ID, and a text loses each name
//! that the table knows. A model sees `{P7}`, never the name of a real player.
//!
//! The story program cuts a text into pieces and folds each word to its key. These rules
//! never look inside a string: they only compare keys.

/// The ID of a player: the place of its name in the table, from 0. A text shows place 0 as
/// `P1`. The table only grows, so an ID is never reused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlayerId(pub usize);

/// One player of the table.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Alias {
    /// The name in lower case, with no realm. A word of a text matches it.
    pub key: String,
    /// The name as the game writes it, for the player's own screen.
    pub shown: String,
}

/// A piece of a text that a player or the game wrote. It has no case for an ID, so a text
/// for a model can hold an ID only where the swap put one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Plain {
    /// What is no word: spaces, signs, and braces.
    Text(String),
    /// A word as the text writes it, and its key.
    Word { key: String, written: String },
}

/// A piece of a text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Piece {
    /// What is no word: spaces, signs, and braces.
    Text(String),
    /// A word as the text writes it, and its key.
    Word {
        key: String,
        written: String,
    },
    Player(PlayerId),
}

/// The ID of the key, or None for a key that the table does not hold. An index loop:
/// Aeneas translates no iterator adapter (lean/README.md).
#[cfg_attr(charon, verify::start_from)]
#[allow(
    clippy::ptr_arg,
    reason = "the Lean model compares a String with a String"
)]
#[must_use]
pub fn find(table: &[Alias], key: &String) -> Option<PlayerId> {
    let mut place = 0;
    while place < table.len() {
        if table[place].key == *key {
            return Some(PlayerId(place));
        }
        place += 1;
    }
    None
}

/// The ID of the player. A name that the table does not hold yet goes at the end.
#[cfg_attr(charon, verify::start_from)]
pub fn learn(table: &mut Vec<Alias>, alias: Alias) -> PlayerId {
    if let Some(id) = find(table, &alias.key) {
        return id;
    }
    let id = PlayerId(table.len());
    table.push(alias);
    id
}

/// Learns each name of a line, in order. An index loop, as in `find`.
#[cfg_attr(charon, verify::start_from)]
pub fn learn_all(table: &mut Vec<Alias>, names: &[Alias]) {
    let mut index = 0;
    while index < names.len() {
        learn(table, names[index].clone());
        index += 1;
    }
}

/// A word whose key the table holds becomes the ID of its player. Every other piece stays.
fn to_id(table: &[Alias], piece: &Plain) -> Piece {
    match piece {
        Plain::Text(text) => Piece::Text(text.clone()),
        Plain::Word { key, written } => match find(table, key) {
            Some(id) => Piece::Player(id),
            None => Piece::Word {
                key: key.clone(),
                written: written.clone(),
            },
        },
    }
}

/// The text for a model: each known name becomes its ID. An index loop, as in `find`.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn to_ids(table: &[Alias], pieces: &[Plain]) -> Vec<Piece> {
    let mut swapped = Vec::new();
    let mut index = 0;
    while index < pieces.len() {
        swapped.push(to_id(table, &pieces[index]));
        index += 1;
    }
    swapped
}

/// An ID that the table holds becomes the name of its player. An unknown ID stays.
fn to_name(table: &[Alias], piece: &Piece) -> Piece {
    if let Piece::Player(id) = piece
        && id.0 < table.len()
    {
        let alias = &table[id.0];
        return Piece::Word {
            key: alias.key.clone(),
            written: alias.shown.clone(),
        };
    }
    piece.clone()
}

/// The text for the player's own screen: each ID becomes its name. An index loop, as in
/// `find`.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn to_names(table: &[Alias], pieces: &[Piece]) -> Vec<Piece> {
    let mut swapped = Vec::new();
    let mut index = 0;
    while index < pieces.len() {
        swapped.push(to_name(table, &pieces[index]));
        index += 1;
    }
    swapped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alias(name: &str) -> Alias {
        Alias {
            key: name.to_lowercase(),
            shown: name.to_string(),
        }
    }

    fn word(written: &str) -> Piece {
        Piece::Word {
            key: written.to_lowercase(),
            written: written.to_string(),
        }
    }

    fn plain_word(written: &str) -> Plain {
        Plain::Word {
            key: written.to_lowercase(),
            written: written.to_string(),
        }
    }

    #[test]
    fn the_first_player_gets_the_first_id_and_keeps_it() {
        let mut table = Vec::new();

        let first = learn(&mut table, alias("Ada"));
        let second = learn(&mut table, alias("Corvin"));
        let again = learn(&mut table, alias("ADA"));

        assert_eq!(
            (first, second, again),
            (PlayerId(0), PlayerId(1), PlayerId(0))
        );
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn a_name_learned_again_keeps_the_form_it_had_first() {
        let mut table = vec![alias("Ada")];

        learn_all(&mut table, &[alias("ADA"), alias("Corvin")]);

        assert_eq!(table, vec![alias("Ada"), alias("Corvin")]);
    }

    #[test]
    fn a_known_name_becomes_its_id_and_comes_back_in_the_form_of_the_table() {
        let table = vec![alias("Ada")];
        let pieces = vec![
            plain_word("ADA"),
            Plain::Text(" and ".into()),
            plain_word("Bob"),
        ];
        let rest = [Piece::Text(" and ".into()), word("Bob")];

        let for_a_model = to_ids(&table, &pieces);
        let for_the_player = to_names(&table, &for_a_model);

        assert_eq!(for_a_model[0], Piece::Player(PlayerId(0)));
        assert_eq!(for_a_model[1..], rest);
        assert_eq!(for_the_player[0], word("Ada"));
        assert_eq!(for_the_player[1..], rest);
    }

    #[test]
    fn an_id_that_the_table_does_not_hold_stays_an_id() {
        let table = vec![alias("Ada")];

        let swapped = to_names(&table, &[Piece::Player(PlayerId(1))]);

        assert_eq!(swapped, vec![Piece::Player(PlayerId(1))]);
    }
}
