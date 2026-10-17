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
use timeways_rules::aliases::{self as rules, Alias, Piece, Plain, PlayerId};

/// The limit of the bridge for the name of a character (Gnomish Relay SPEC.md 9.8).
pub const MAX_PLAYER_NAME_BYTES: usize = 48;

/// What a player's screen shows for an ID that the table does not hold. Only a damaged
/// file makes one.
const UNKNOWN_PLAYER: &str = "someone";

/// One row of the `aliases` table: a player, with the race and the class that the game
/// showed when the table learned the name. The row's position is the ID.
///
/// A surname is a row of its own, because rows only grow and the table often learns a
/// first name before its surname. A row of a world from before 2026-10-17 has no
/// `surname_of`, and reads as a first name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasRow {
    pub name: String,
    #[serde(default)]
    pub race: Option<Race>,
    #[serde(default)]
    pub class: Option<Class>,
    /// The first name whose surname this name is: "James" of "Leborc James".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surname_of: Option<String>,
}

impl AliasRow {
    #[must_use]
    pub fn named(name: &str) -> AliasRow {
        AliasRow {
            name: name.to_string(),
            race: None,
            class: None,
            surname_of: None,
        }
    }

    /// The row of the surname of `first`.
    #[must_use]
    pub fn surname(surname: &str, first: &str) -> AliasRow {
        AliasRow {
            surname_of: Some(first.to_string()),
            ..AliasRow::named(surname)
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

/// The table entry of a row. A surname has a key with spaces, "james of leborc", so it
/// never matches a word: the proved swap reads first names only, and the surname pass
/// (`without_surnames`) reads the surnames. One surname row stands for each pair, so two
/// players can share a surname, and a player can share the name of a surname.
#[must_use]
pub fn alias_of_row(row: &AliasRow) -> Option<Alias> {
    let Some(first) = &row.surname_of else {
        return alias_of(&row.name);
    };
    let surname = player_name(&row.name)?;
    let first = player_name(first)?;
    Some(Alias {
        key: format!("{} of {}", key_of(surname), key_of(first)),
        shown: surname.to_string(),
    })
}

/// The most players that one text of the hero names (GAMEPLAY.md 5.11).
pub const MAX_NAMES: usize = 32;

/// A part that `alias_of` refuses still names the character, so each word in it is hidden:
/// "E\u{301}lise" with its accent in a code point of its own, a digit, or a long part.
fn part_aliases(part: &str) -> Vec<Alias> {
    if let Some(alias) = alias_of(part) {
        return vec![alias];
    }
    let words = runs(part).into_iter().filter(|(is_word, _)| *is_word);
    words
        .map(|(_, word)| Alias {
            key: key_of(word),
            shown: word.to_string(),
        })
        .collect()
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
/// word when the table knows its name, as `TaskNames.lua` does it. A plain piece is never
/// an ID, so the swap is the only step that puts one in a text for a model.
#[must_use]
pub fn text_pieces(text: &str, table: &[Alias]) -> Vec<Plain> {
    let runs = runs(text);
    let mut pieces = Vec::with_capacity(runs.len());
    let mut index = 0;
    while index < runs.len() {
        let (is_word, run) = runs[index];
        if !is_word {
            pieces.push(Plain::Text(run.to_string()));
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
            pieces.push(Plain::Word {
                key,
                written: format!("{run}-{realm}"),
            });
            index += 3;
            continue;
        }
        pieces.push(Plain::Word {
            key,
            written: run.to_string(),
        });
        index += 1;
    }
    pieces
}

/// The words of a text: runs of letters and digits.
pub fn words_in(text: &str) -> impl Iterator<Item = &str> {
    runs(text)
        .into_iter()
        .filter_map(|(is_word, run)| is_word.then_some(run))
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

/// The plain pieces joined again.
#[must_use]
pub fn plain_joined(pieces: &[Plain]) -> String {
    let mut text = String::new();
    for piece in pieces {
        match piece {
            Plain::Text(part) => text.push_str(part),
            Plain::Word { written, .. } => text.push_str(written),
        }
    }
    text
}

/// The text for a model: each name that the table knows becomes its ID.
#[must_use]
pub fn without_names(table: &[Alias], text: &str) -> String {
    joined(&rules::to_ids(table, &text_pieces(text, table)))
}

/// When a surname of the character, alone, is hidden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surname {
    /// In any case. An export blanks too much rather than too little.
    Always,
    /// Only when it is written with a capital, anywhere, also first in a sentence. In lower
    /// case it is a word: "I will go" keeps its meaning for Kobee Will.
    AsWritten,
}

/// The text with the name of the character as `$N`. The first name always goes, in any
/// case. Hidden parts in a row, "Kobee Bean", become one `$N`. A part that is no plain
/// name, such as one with a digit, hides each word in it.
#[must_use]
pub fn without_own_name(text: &str, own: &str, surname: Surname) -> String {
    let mut parts = own.split_whitespace();
    let first = parts.next().map(part_aliases).unwrap_or_default();
    let surnames: Vec<Alias> = parts.flat_map(part_aliases).collect();
    let all: Vec<Alias> = first.iter().chain(&surnames).cloned().collect();
    let mut hidden = String::with_capacity(text.len());
    // The spaces after a `$N`: they go when another part of the name follows.
    let mut after_mark: Option<String> = None;
    for piece in text_pieces(text, &all) {
        let hides = match &piece {
            Plain::Word { key, .. } if rules::find(&first, key).is_some() => true,
            Plain::Word { key, written } if rules::find(&surnames, key).is_some() => {
                surname == Surname::Always || is_capitalized(written)
            }
            _ => false,
        };
        match piece {
            _ if hides => {
                if after_mark.is_none() {
                    hidden.push_str("$N");
                }
                after_mark = Some(String::new());
            }
            Plain::Text(text) if is_spaces(&text) && after_mark.is_some() => {
                after_mark.get_or_insert_default().push_str(&text);
            }
            Plain::Word { written: text, .. } | Plain::Text(text) => {
                hidden.push_str(&after_mark.take().unwrap_or_default());
                hidden.push_str(&text);
            }
        }
    }
    hidden.push_str(&after_mark.unwrap_or_default());
    hidden
}

fn is_spaces(text: &str) -> bool {
    text.chars().all(|c| c == ' ')
}

#[must_use]
pub fn is_capitalized(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase)
}

/// True when a word after `before` starts a sentence.
#[must_use]
pub fn starts_a_sentence(before: &str) -> bool {
    before
        .trim_end()
        .chars()
        .last()
        .is_none_or(|c| matches!(c, '.' | '!' | '?'))
}

/// The text with the surnames of other players hidden, after the proved swap of first
/// names. A surname right after the ID of its first name, with only spaces between, goes
/// with it, in any case: "{P1} James" is "{P1}". A surname alone with a capital becomes
/// the ID of its row: "the James family" is "the {P2} family". In lower case it is a word.
#[must_use]
pub fn without_surnames(rows: &[AliasRow], text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // The ID that the text ends with, and the spaces after it.
    let mut after: Option<(PlayerId, String)> = None;
    for piece in id_pieces(text) {
        match piece {
            Piece::Player(id) => {
                let surname = rows.get(id.0).map(|row| row.name.as_str());
                if after_its_first_name(rows, after.as_ref(), surname) {
                    after = None;
                    continue;
                }
                flush(&mut out, &mut after);
                out.push_str(&id_text(id));
                after = Some((id, String::new()));
            }
            Piece::Text(plain) => {
                for (is_word, run) in runs(&plain) {
                    surname_run(rows, &mut out, &mut after, is_word, run);
                }
            }
            Piece::Word { written, .. } => {
                flush(&mut out, &mut after);
                out.push_str(&written);
            }
        }
    }
    flush(&mut out, &mut after);
    out
}

fn surname_run(
    rows: &[AliasRow],
    out: &mut String,
    after: &mut Option<(PlayerId, String)>,
    is_word: bool,
    run: &str,
) {
    if !is_word {
        match after {
            Some((_, spaces)) if is_spaces(run) => spaces.push_str(run),
            _ => {
                flush(out, after);
                out.push_str(run);
            }
        }
        return;
    }
    if after_its_first_name(rows, after.as_ref(), Some(run)) {
        *after = None;
        return;
    }
    flush(out, after);
    match surname_id(rows, run).filter(|_| is_capitalized(run)) {
        Some(id) => out.push_str(&id_text(id)),
        None => out.push_str(run),
    }
}

fn flush(out: &mut String, after: &mut Option<(PlayerId, String)>) {
    if let Some((_, spaces)) = after.take() {
        out.push_str(&spaces);
    }
}

/// True when `word` is a surname of the player whose ID the text ends with, with only
/// spaces between.
fn after_its_first_name(
    rows: &[AliasRow],
    after: Option<&(PlayerId, String)>,
    word: Option<&str>,
) -> bool {
    let (Some((first, _)), Some(word)) = (after, word) else {
        return false;
    };
    let Some(first) = rows.get(first.0).filter(|row| row.surname_of.is_none()) else {
        return false;
    };
    let (first, word) = (key_of(&first.name), key_of(word));
    rows.iter().any(|row| {
        row.surname_of.as_deref().map(key_of).as_ref() == Some(&first) && key_of(&row.name) == word
    })
}

/// The ID of the first surname row of the word.
fn surname_id(rows: &[AliasRow], word: &str) -> Option<PlayerId> {
    let key = key_of(word);
    rows.iter()
        .position(|row| row.surname_of.is_some() && key_of(&row.name) == key)
        .map(PlayerId)
}

/// A text that the player typed, as a model reads it: a brace becomes a parenthesis, each
/// name of the table becomes its ID with its surname, and the name of the character
/// becomes `$N`. `table` holds the names of `rows`.
#[must_use]
pub fn typed_for_model(table: &[Alias], rows: &[AliasRow], own: &str, text: &str) -> String {
    let typed = unmarked(text).text;
    let with_ids = without_surnames(rows, &without_names(table, &typed));
    without_own_name(&with_ids, own, Surname::AsWritten)
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
            surname_of: None,
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

    /// Leborc James, P1 and P2, and Bram Will, P3 and P4.
    fn rows() -> Vec<AliasRow> {
        vec![
            AliasRow::named("Leborc"),
            AliasRow::surname("James", "Leborc"),
            AliasRow::named("Bram"),
            AliasRow::surname("Will", "Bram"),
        ]
    }

    #[test]
    fn a_surname_right_after_its_first_name_goes_with_it_in_any_case() {
        let text = without_surnames(&rows(), "{P1} James and {P1}  james and {P3} Will.");

        assert_eq!(text, "{P1} and {P1} and {P3}.");
    }

    #[test]
    fn a_surname_with_a_capital_alone_is_hidden_anywhere() {
        let text = without_surnames(&rows(), "James is my house. {P1}, James.\n{P1}\nJames");

        assert_eq!(text, "{P2} is my house. {P1}, {P2}.\n{P1}\n{P2}");
    }

    #[test]
    fn a_surname_in_lower_case_alone_is_a_word() {
        let text = without_surnames(&rows(), "I will carry the james stone. {P1}, will you?");

        assert_eq!(text, "I will carry the james stone. {P1}, will you?");
    }

    #[test]
    fn a_surname_after_another_first_name_is_hidden_only_with_a_capital() {
        let text = without_surnames(&rows(), "{P3} James and {P3} james");

        assert_eq!(text, "{P3} {P2} and {P3} james");
    }

    #[test]
    fn a_surname_row_never_matches_a_word_of_the_proved_swap() {
        let rows = rows();
        let table: Vec<Alias> = rows.iter().filter_map(alias_of_row).collect();

        let text = without_names(&table, "Leborc James met Bram Will");

        assert_eq!(text, "{P1} James met {P3} Will");
    }

    #[test]
    fn the_own_first_name_goes_in_any_case_and_a_surname_only_with_a_capital() {
        let own = "Kobee Will";

        let hidden = |text| without_own_name(text, own, Surname::AsWritten);

        assert_eq!(
            hidden("Kobee will avenge his father."),
            "$N will avenge his father."
        );
        assert_eq!(
            hidden("Will is my house. kobee Will rests."),
            "$N is my house. $N rests."
        );
        assert_eq!(hidden("the will clan"), "the will clan");
        assert_eq!(hidden("KOBEE  Will"), "$N");
        assert_eq!(hidden("Kobee,\nWill"), "$N,\n$N");
    }

    #[test]
    fn the_strict_own_name_pass_hides_each_part_in_any_case() {
        let hidden = without_own_name("I will go. kobee will.", "Kobee Will", Surname::Always);

        assert_eq!(hidden, "I $N go. $N.");
    }
}
