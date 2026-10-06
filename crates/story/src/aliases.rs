//! Player names in the story program (GAMEPLAY.md 5.11). The addon marks each name of a
//! player that it knows: "{Ada} held the bridge". The story program gives the player an
//! ID, and a model reads "{P1} held the bridge", with a card. The player's own screen
//! gets the name back.
//!
//! The proved rules live in `timeways_rules::aliases`. This module cuts a text into their
//! pieces and joins the pieces again. A word here is a run of letters and digits, in any
//! script.

use crate::race_class::{Class, Race};
use serde::{Deserialize, Serialize};
use timeways_rules::aliases::{self as rules, Alias, Piece, PlayerId};

/// The limit of the bridge for the name of a character (Gnomish Relay SPEC.md 9.8).
pub const MAX_PLAYER_NAME_BYTES: usize = 48;

/// What a player's screen shows for an ID that the table does not hold. Only a damaged
/// file makes one.
const UNKNOWN_PLAYER: &str = "someone";

/// One row of the `aliases` table: a player, with the race and the class that the game
/// showed when the table learned the name. The row's position is the ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasRow {
    pub name: String,
    #[serde(default)]
    pub race: Option<Race>,
    #[serde(default)]
    pub class: Option<Class>,
}

impl AliasRow {
    #[must_use]
    pub fn named(name: &str) -> AliasRow {
        AliasRow {
            name: name.to_string(),
            race: None,
            class: None,
        }
    }

    /// What a model reads about the player: "{P7}: a Forsaken mage".
    #[must_use]
    pub fn card(&self, id: PlayerId) -> String {
        let what = match (self.race, self.class) {
            (Some(race), Some(class)) => format!("{} {}", race.word(), class.word()),
            (Some(race), None) => format!("{} player", race.word()),
            (None, Some(class)) => class.word().to_string(),
            (None, None) => "player".to_string(),
        };
        format!("{}: {} {what}", id_text(id), article(&what))
    }
}

fn article(word: &str) -> &'static str {
    let vowel = word
        .chars()
        .next()
        .is_some_and(|c| "aeiouAEIOU".contains(c));
    if vowel { "an" } else { "a" }
}

/// The name of a player without its realm: "Ada-Stormrage" is "Ada". A name holds only
/// letters, and no game name holds a digit. So "P7" is never a name.
#[must_use]
pub fn player_name(name: &str) -> Option<&str> {
    let short = name.split('-').next().unwrap_or_default();
    let letters = !short.is_empty() && short.chars().all(char::is_alphabetic);
    (letters && short.len() <= MAX_PLAYER_NAME_BYTES).then_some(short)
}

/// What a word must match: the name in lower case. "ÉLISE" and "élise" share a key.
#[must_use]
pub fn key_of(word: &str) -> String {
    word.to_lowercase()
}

/// The table entry of a name, or None for a text that is no name.
#[must_use]
pub fn alias_of(name: &str) -> Option<Alias> {
    let short = player_name(name)?;
    Some(Alias {
        key: key_of(short),
        shown: short.to_string(),
    })
}

/// The ID as a text shows it: `{P1}` for the first player.
#[must_use]
pub fn id_text(id: PlayerId) -> String {
    format!("{{P{}}}", id.0.saturating_add(1))
}

/// A text from the addon with its marks taken out, and the names that it marked.
#[derive(Debug, PartialEq, Eq)]
pub struct Marked {
    pub text: String,
    pub names: Vec<String>,
}

/// "{Ada} and {Bob-Stormrage} met" is "Ada and Bob-Stormrage met", with the names Ada and
/// Bob. Every other brace becomes a parenthesis, so a text that a player typed never holds
/// an ID.
#[must_use]
pub fn unmarked(text: &str) -> Marked {
    let mut plain = String::with_capacity(text.len());
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some((inner, after)) = mark_at(rest) {
            plain.push_str(inner);
            if let Some(name) = player_name(inner) {
                names.push(name.to_string());
            }
            rest = after;
            continue;
        }
        plain.push(match c {
            '{' => '(',
            '}' => ')',
            other => other,
        });
        rest = &rest[c.len_utf8()..];
    }
    Marked { text: plain, names }
}

/// The name in a mark at the start of the text, and the text after the mark.
fn mark_at(text: &str) -> Option<(&str, &str)> {
    let inside = text.strip_prefix('{')?;
    let end = inside.find('}')?;
    let inner = &inside[..end];
    player_name(inner)?;
    let realm_ok = inner
        .split_once('-')
        .is_none_or(|(_, realm)| is_realm(realm));
    realm_ok.then_some((inner, &inside[end + 1..]))
}

fn is_realm(realm: &str) -> bool {
    !realm.is_empty() && realm.chars().all(is_word_char)
}

/// Stricter than `TaskNames.lua`, which reads every byte past ASCII as a letter: a curly
/// apostrophe or a dash after a name ends the word, so "Ada’s" still loses its name.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// The text cut into words and the rest. A word with a realm, "Ada-Stormrage", is one
/// word when the table knows its name, as `TaskNames.lua` does it.
#[must_use]
pub fn text_pieces(text: &str, table: &[Alias]) -> Vec<Piece> {
    let runs = runs(text);
    let mut pieces = Vec::with_capacity(runs.len());
    let mut index = 0;
    while index < runs.len() {
        let (is_word, run) = runs[index];
        if !is_word {
            pieces.push(Piece::Text(run.to_string()));
            index += 1;
            continue;
        }
        let key = key_of(run);
        let realm = runs
            .get(index + 2)
            .filter(|_| runs[index + 1] == (false, "-"));
        if let Some((true, realm)) = realm
            && rules::find(table, &key).is_some()
        {
            pieces.push(Piece::Word {
                key,
                written: format!("{run}-{realm}"),
            });
            index += 3;
            continue;
        }
        pieces.push(Piece::Word {
            key,
            written: run.to_string(),
        });
        index += 1;
    }
    pieces
}

/// The runs of word letters and of other letters, in order, each with whether it is a word.
fn runs(text: &str) -> Vec<(bool, &str)> {
    let mut runs = Vec::new();
    let mut start = 0;
    let mut current: Option<bool> = None;
    for (at, c) in text.char_indices() {
        let is_word = is_word_char(c);
        if current.is_some_and(|word| word != is_word) {
            runs.push((!is_word, &text[start..at]));
            start = at;
        }
        current = Some(is_word);
    }
    if let Some(is_word) = current {
        runs.push((is_word, &text[start..]));
    }
    runs
}

/// The pieces joined again. An ID shows as `{P7}`.
#[must_use]
pub fn joined(pieces: &[Piece]) -> String {
    let mut text = String::new();
    for piece in pieces {
        match piece {
            Piece::Text(part) => text.push_str(part),
            Piece::Word { written, .. } => text.push_str(written),
            Piece::Player(id) => text.push_str(&id_text(*id)),
        }
    }
    text
}

/// The text for a model: each name that the table knows becomes its ID.
#[must_use]
pub fn without_names(table: &[Alias], text: &str) -> String {
    joined(&rules::to_ids(table, &text_pieces(text, table)))
}

/// A text that holds IDs, cut into IDs and the rest. Only `{P1}` and up are IDs, with no
/// leading zero, so the cut joins back to the same text.
#[must_use]
pub fn id_pieces(text: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some((id, after)) = id_at(rest) {
            if !plain.is_empty() {
                pieces.push(Piece::Text(std::mem::take(&mut plain)));
            }
            pieces.push(Piece::Player(id));
            rest = after;
            continue;
        }
        plain.push(c);
        rest = &rest[c.len_utf8()..];
    }
    if !plain.is_empty() {
        pieces.push(Piece::Text(plain));
    }
    pieces
}

fn id_at(text: &str) -> Option<(PlayerId, &str)> {
    let inside = text.strip_prefix("{P")?;
    let end = inside.find('}')?;
    let digits = &inside[..end];
    let canonical = !digits.is_empty()
        && !digits.starts_with('0')
        && digits.chars().all(|c| c.is_ascii_digit());
    let number: usize = digits.parse().ok().filter(|_| canonical)?;
    Some((PlayerId(number - 1), &inside[end + 1..]))
}

/// The IDs of a text, in order, once each.
#[must_use]
pub fn ids_in(text: &str) -> Vec<PlayerId> {
    let mut ids = Vec::new();
    for piece in id_pieces(text) {
        if let Piece::Player(id) = piece
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids
}

/// True when the text holds an ID. The journal shows a text of the narrator as it is, so
/// such a text would show `{P1}` to the player.
#[must_use]
pub fn holds_an_id(text: &str) -> bool {
    !ids_in(text).is_empty()
}

/// The text for the player's own screen: each ID becomes the name of its player.
#[must_use]
pub fn with_names(table: &[Alias], text: &str) -> String {
    let named = rules::to_names(table, &id_pieces(text));
    let unknown: Vec<Piece> = named
        .into_iter()
        .map(|piece| match piece {
            Piece::Player(_) => Piece::Text(UNKNOWN_PLAYER.to_string()),
            other => other,
        })
        .collect();
    joined(&unknown)
}

/// True when each ID of the text names a player of the table. A model cannot invent a
/// player.
#[must_use]
pub fn knows_every_id(table: &[Alias], text: &str) -> bool {
    ids_in(text).iter().all(|id| id.0 < table.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(names: &[&str]) -> Vec<Alias> {
        names.iter().filter_map(|name| alias_of(name)).collect()
    }

    #[test]
    fn a_mark_gives_its_name_and_leaves_the_name_in_the_text() {
        let marked = unmarked("{Ada} and {Bob-Stormrage} held {the} bridge {P7}");

        assert_eq!(marked.text, "Ada and Bob-Stormrage held the bridge (P7)");
        assert_eq!(marked.names, vec!["Ada", "Bob", "the"]);
    }

    #[test]
    fn a_known_name_becomes_its_id_in_any_case_and_with_its_realm() {
        let table = table(&["Ada", "Élise"]);

        let text = without_names(&table, "ADA-Stormrage met élise, not Adam or Bada.");

        assert_eq!(text, "{P1} met {P2}, not Adam or Bada.");
    }

    #[test]
    fn a_word_with_a_realm_whose_name_is_unknown_keeps_the_other_word() {
        let table = table(&["Ada"]);

        let text = without_names(&table, "well-known Bob-Ada");

        assert_eq!(text, "well-known Bob-{P1}");
    }

    #[test]
    fn the_player_reads_the_name_and_never_an_id() {
        let table = table(&["Ada"]);

        let text = with_names(&table, "{P1} and {P2} met {P01} at {P}");

        assert_eq!(text, "Ada and someone met {P01} at {P}");
    }

    #[test]
    fn a_name_with_a_digit_is_no_name() {
        assert_eq!(player_name("P7"), None);
        assert_eq!(player_name(""), None);
        assert_eq!(player_name("Ada-Stormrage"), Some("Ada"));
    }

    #[test]
    fn a_card_names_the_race_and_the_class_that_the_game_showed() {
        let row = AliasRow {
            name: "Ada".into(),
            race: Some(Race::Orc),
            class: Some(Class::Mage),
        };

        assert_eq!(row.card(PlayerId(6)), "{P7}: an orc mage");
        assert_eq!(AliasRow::named("Bob").card(PlayerId(0)), "{P1}: a player");
    }

    #[test]
    fn a_model_cannot_name_a_player_that_the_table_does_not_hold() {
        let table = table(&["Ada"]);

        assert!(knows_every_id(&table, "{P1} waits"));
        assert!(!knows_every_id(&table, "{P2} waits"));
    }
}
