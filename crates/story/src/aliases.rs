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

/// The most players that one text of the hero names (GAMEPLAY.md 5.11). A text with more
/// is refused, because a name past the list would reach a model unlearned.
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

/// "{Ada} and {Bob-Stormrage} met" is "Ada and Bob met", with the names "Ada" and
/// "Bob-Stormrage". On WoW Forever the part after the dash is the surname. The text holds
/// the first name alone, as the player typed it, so the limits of the addon hold. Every
/// other brace becomes a parenthesis, so a text that a player typed never holds an ID.
#[must_use]
pub fn unmarked(text: &str) -> Marked {
    read_marks(text, |inner| first_name(inner).to_string())
}

/// The text with each mark as the ID of the first name of its player, and every other brace
/// as a parenthesis. A mark comes from the addon only, so it names a player for certain,
/// also one whose first name is yours: run it before the own pass. Learn the names first.
#[must_use]
pub fn marks_as_ids(table: &[Alias], text: &str) -> String {
    let id_of = |inner: &str| {
        let first = first_name(inner);
        rules::find(table, &key_of(first)).map_or_else(|| first.to_string(), id_text)
    };
    read_marks(text, id_of).text
}

fn first_name(inner: &str) -> &str {
    player_name(inner).unwrap_or(inner)
}

/// The text with each mark as `mark` writes it, and the inner of each mark. The addon marks
/// whole words only, so a brace pair that touches a word character, "Leborc{James}", was
/// typed, and is no mark.
fn read_marks(text: &str, mut mark: impl FnMut(&str) -> String) -> Marked {
    let mut plain = String::with_capacity(text.len());
    let mut names = Vec::new();
    let mut rest = text;
    let mut before: Option<char> = None;
    while let Some(c) = rest.chars().next() {
        let found = mark_at(rest).filter(|(_, after)| {
            let touches = |c: Option<char>| c.is_some_and(is_word_char);
            !touches(before) && !touches(after.chars().next())
        });
        if let Some((inner, after)) = found {
            plain.push_str(&mark(inner));
            names.push(inner.to_string());
            before = Some('}');
            rest = after;
            continue;
        }
        plain.push(match c {
            '{' => '(',
            '}' => ')',
            other => other,
        });
        before = Some(c);
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

/// True for a character of a word: an ASCII letter or digit, or a code point in a range of
/// `LETTERS`. Every other code point is a sign: a wrong split hides too much, never too
/// little. The addon holds the same rule (`TaskNames.IsWordChar`), and the addon test
/// `the_addon_and_the_desktop_agree_on_every_letter` checks every code point of the BMP.
#[must_use]
pub fn is_word_char(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphanumeric();
    }
    in_ranges(u32::from(c), &LETTERS)
}

/// True for a character that shows nothing, and goes before a match: "Cor\u{200B}vin" is
/// Corvin. The addon drops the same ones (`TaskNames.Visible`).
#[must_use]
pub fn is_hidden(c: char) -> bool {
    in_ranges(u32::from(c), &HIDDEN)
}

fn in_ranges(code: u32, ranges: &[(u32, u32)]) -> bool {
    ranges
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&code))
}

/// The letters of the scripts of the game's names, and of Greek, Cyrillic, and East Asian
/// text. A filler of Hangul looks like a letter and shows nothing, so it is a sign.
const LETTERS: [(u32, u32); 30] = [
    (0x00AA, 0x00AA), // ª
    (0x00B5, 0x00B5), // µ
    (0x00BA, 0x00BA), // º
    (0x00C0, 0x00D6), // Latin-1 letters, less × and ÷
    (0x00D8, 0x00F6),
    (0x00F8, 0x00FF),
    (0x0100, 0x024F), // Latin Extended-A and B
    (0x0370, 0x0374), // Greek
    (0x0376, 0x0377),
    (0x037A, 0x037D),
    (0x037F, 0x037F),
    (0x0386, 0x0386),
    (0x0388, 0x038A),
    (0x038C, 0x038C),
    (0x038E, 0x03A1),
    (0x03A3, 0x03F5),
    (0x03F7, 0x03FF),
    (0x0400, 0x0481), // Cyrillic
    (0x048A, 0x04FF),
    (0x1100, 0x115E), // Hangul Jamo, less the fillers
    (0x1161, 0x11FF),
    (0x1E00, 0x1EFF), // Latin Extended Additional
    (0x3041, 0x3096), // Hiragana
    (0x309D, 0x309F),
    (0x30A1, 0x30FA), // Katakana
    (0x30FC, 0x30FF),
    (0x3131, 0x3163), // Hangul compatibility Jamo, less the filler
    (0x3165, 0x318E),
    (0x4E00, 0x9FFF), // CJK
    (0xAC00, 0xD7A3), // Hangul syllables
];

const HIDDEN: [(u32, u32); 7] = [
    (0x00AD, 0x00AD), // the soft hyphen
    (0x034F, 0x034F), // the combining grapheme joiner
    (0x180E, 0x180E), // the Mongolian vowel separator
    (0x200B, 0x200F), // zero-width spaces and joiners, and direction marks
    (0x2060, 0x2064), // the word joiner and invisible operators
    (0xFE00, 0xFE0F), // variation selectors
    (0xFEFF, 0xFEFF), // the zero-width no-break space
];

/// The text for a model, as names match it: with no character that shows nothing, as the
/// addon reads it (`TaskNames.Visible`). A combining mark stays, and is a sign: "Ada\u{301}"
/// holds the name Ada. The player's own screen keeps the typed text.
#[must_use]
pub fn for_matching(text: &str) -> String {
    text.chars().filter(|c| !is_hidden(*c)).collect()
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

/// The text with each name that the table knows as its ID. For a model, clean the text
/// with `for_matching` first.
#[must_use]
pub fn without_names(table: &[Alias], text: &str) -> String {
    joined(&rules::to_ids(table, &text_pieces(text, table)))
}

/// When a surname is hidden: one of the character, alone, or one of another player.
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
    let text = for_matching(text);
    let text = text.as_str();
    let mut parts = own.split_whitespace();
    let first = parts.next().map(part_aliases).unwrap_or_default();
    let surnames: Vec<Alias> = parts.flat_map(part_aliases).collect();
    let mut hidden = String::with_capacity(text.len());
    // The spaces after a `$N`: they go when another part of the name follows.
    let mut after_mark: Option<String> = None;
    // Only a first name takes a realm: "will-Kobee" is two words for Kobee Will.
    for piece in text_pieces(text, &first) {
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

/// Spaces between two parts of a name: any run of plain and no-break spaces.
fn is_spaces(text: &str) -> bool {
    text.chars().all(|c| c == ' ' || c == '\u{A0}')
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
/// names. A surname with a capital right after the ID of its first name, with only spaces
/// between, goes with it: "{P1} James" is "{P1}". A surname that the swap made an ID goes
/// there in any case. A surname alone with a capital becomes the ID of its row: "the James
/// family" is "the {P2} family". With `Surname::AsWritten`, a surname in lower case is a
/// word: "{P1} will help". With `Surname::Always`, as in an export, it goes in any case.
#[must_use]
pub fn without_surnames(rows: &[AliasRow], text: &str, rule: Surname) -> String {
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
                    surname_run(rows, rule, &mut out, &mut after, (is_word, run));
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
    rule: Surname,
    out: &mut String,
    after: &mut Option<(PlayerId, String)>,
    (is_word, run): (bool, &str),
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
    let as_a_name = rule == Surname::Always || is_capitalized(run);
    if as_a_name && after_its_first_name(rows, after.as_ref(), Some(run)) {
        *after = None;
        return;
    }
    flush(out, after);
    match surname_id(rows, run).filter(|_| as_a_name) {
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

/// A text that the player typed, as a model reads it: a brace becomes a parenthesis, the
/// name of the character becomes `$N`, and each name of the table becomes its ID with its
/// surname. `table` holds the names of `rows`.
#[must_use]
pub fn typed_for_model(table: &[Alias], rows: &[AliasRow], own: &str, text: &str) -> String {
    let typed = unmarked(&for_matching(text)).text;
    // `$N` matches no name, so the own name goes first and keeps its meaning.
    let without_own = without_own_name(&typed, own, Surname::AsWritten);
    without_surnames(
        rows,
        &without_names(table, &without_own),
        Surname::AsWritten,
    )
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

        assert_eq!(marked.text, "Ada and Bob held the bridge (P7)");
        assert_eq!(marked.names, vec!["Ada", "Bob-Stormrage", "the"]);
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
    fn a_surname_right_after_its_first_name_goes_with_it_only_with_a_capital() {
        let text = without_surnames(
            &rows(),
            "{P1} James and {P1}  JAMES and {P3} will help.",
            Surname::AsWritten,
        );

        assert_eq!(text, "{P1} and {P1} and {P3} will help.");
    }

    #[test]
    fn a_surname_that_the_swap_made_an_id_goes_after_its_first_name_in_any_case() {
        let mut rows = rows();
        rows.push(AliasRow::named("James"));

        let text = without_surnames(&rows, "{P1} {P5} and {P5}", Surname::AsWritten);

        assert_eq!(text, "{P1} and {P5}");
    }

    #[test]
    fn an_own_surname_in_lower_case_never_joins_a_first_name_as_its_realm() {
        let hidden = |text| without_own_name(text, "Kobee Will", Surname::AsWritten);

        assert_eq!(
            hidden("will-Kobee rides, will-kobee too"),
            "will-$N rides, will-$N too"
        );
        assert_eq!(hidden("Will-Kobee and Kobee-Will"), "$N-$N and $N");
    }

    #[test]
    fn the_own_name_goes_before_the_surnames_of_other_players() {
        let rows = rows();
        let table: Vec<Alias> = rows.iter().filter_map(alias_of_row).collect();

        let kobee = typed_for_model(&table, &rows, "Kobee Will", "Kobee Will rests.");
        let james = typed_for_model(&table, &rows, "James", "James rests.");

        assert_eq!(kobee, "$N rests.");
        assert_eq!(james, "$N rests.");
    }

    #[test]
    fn a_surname_with_a_capital_alone_is_hidden_anywhere() {
        let text = without_surnames(
            &rows(),
            "James is my house. {P1}, James.\n{P1}\nJames",
            Surname::AsWritten,
        );

        assert_eq!(text, "{P2} is my house. {P1}, {P2}.\n{P1}\n{P2}");
    }

    #[test]
    fn a_surname_in_lower_case_alone_is_a_word() {
        let text = without_surnames(
            &rows(),
            "I will carry the james stone. {P1}, will you?",
            Surname::AsWritten,
        );

        assert_eq!(text, "I will carry the james stone. {P1}, will you?");
    }

    #[test]
    fn a_surname_after_another_first_name_is_hidden_only_with_a_capital() {
        let text = without_surnames(&rows(), "{P3} James and {P3} james", Surname::AsWritten);

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

    #[test]
    fn a_brace_pair_that_touches_a_word_is_no_mark() {
        let marked = unmarked("Leborc{James} and {Ada}x and x{Ada}, {Ada}.");

        assert_eq!(marked.text, "Leborc(James) and (Ada)x and x(Ada), Ada.");
        assert_eq!(marked.names, vec!["Ada"]);
    }

    #[test]
    fn a_sign_outside_the_letter_ranges_ends_a_word() {
        let table = table(&["Ada"]);

        let text = without_names(
            &table,
            "Ada\u{1DC0} Ada\u{1680}went Ada\u{3164} Ada฿ Ada\u{E000}",
        );

        assert_eq!(
            text,
            "{P1}\u{1DC0} {P1}\u{1680}went {P1}\u{3164} {P1}฿ {P1}\u{E000}"
        );
    }

    /// Leborc James and Bram Will, with Corvin, for the own name Kobee Will.
    fn for_model(text: &str) -> String {
        let mut rows = rows();
        rows.push(AliasRow::named("Corvin"));
        let table: Vec<Alias> = rows.iter().filter_map(alias_of_row).collect();
        typed_for_model(&table, &rows, "Kobee Will", text)
    }

    #[test]
    fn a_combining_mark_after_a_surname_never_makes_it_a_new_word() {
        assert_eq!(for_model("Will\u{301} said"), "$N\u{301} said");
        assert_eq!(for_model("Kobee Will\u{301}"), "$N\u{301}");
        assert_eq!(for_model("James\u{301} Leborc"), "{P2}\u{301} {P1}");
        assert_eq!(
            for_model("the James\u{301} family"),
            "the {P2}\u{301} family"
        );
        assert_eq!(for_model("Leborc James\u{301}"), "{P1}\u{301}");
    }

    #[test]
    fn a_compatibility_sign_or_a_mark_after_a_name_never_joins_it() {
        let text = for_model("Corvin\u{212A} Corvin\u{2126} Corvin\u{301}x Corvin\u{335}x");

        assert_eq!(text, "{P5}\u{212A} {P5}\u{2126} {P5}\u{301}x {P5}\u{335}x");
    }

    #[test]
    fn a_mark_after_the_own_name_token_keeps_the_token() {
        assert!(for_model("$N\u{301}x").starts_with("$N"));
    }

    #[test]
    fn a_character_that_shows_nothing_never_hides_a_brace_that_touches_a_word() {
        let text = for_model("{Corvin}\u{200B}x");

        assert_eq!(text, "({P5})x");
    }

    #[test]
    fn a_name_matches_after_the_characters_that_show_nothing_go() {
        let table = table(&["Ada", "Corvin"]);

        let text = without_names(
            &table,
            &for_matching("A\u{200B}da, Co\u{AD}rvin, Corv\u{FE0F}in"),
        );

        assert_eq!(text, "{P1}, {P2}, {P2}");
    }

    #[test]
    fn the_own_name_matches_after_the_characters_that_show_nothing_go() {
        let hidden = without_own_name("Él\u{200D}ise and ÉLISE", "Élise", Surname::AsWritten);

        assert_eq!(hidden, "$N and $N");
    }

    /// A known gap (GAMEPLAY.md 5.11): no side composes accents, so a name typed in
    /// decomposed form, mostly from a paste, is no word of the name.
    #[test]
    fn a_combining_mark_ends_a_name_and_a_decomposed_name_stays_a_known_gap() {
        let table = table(&["Ada", "Élise", "Elise"]);

        let text = without_names(&table, "Ada\u{301}, Élise, Elise, E\u{301}lise");

        assert_eq!(text, "{P1}\u{301}, {P2}, {P3}, E\u{301}lise");
    }

    #[test]
    fn a_surname_after_its_first_name_goes_across_any_run_of_spaces() {
        let text = without_surnames(&rows(), "{P1}  James, {P1}\u{A0}James", Surname::AsWritten);

        assert_eq!(text, "{P1}, {P1}");
    }

    #[test]
    fn the_own_whole_name_is_one_mark_across_a_no_break_space() {
        let hidden = without_own_name("Kobee\u{A0}Bean rests", "Kobee Bean", Surname::AsWritten);

        assert_eq!(hidden, "$N rests");
    }

    #[test]
    fn a_mark_becomes_the_id_of_its_player_and_its_text_holds_the_first_name() {
        let table = table(&["Kobee", "Ada"]);

        let marked = unmarked("{Kobee-Will} and {Ada}");
        let ids = marks_as_ids(&table, "{Kobee-Will} and {Ada} and {P1}");

        assert_eq!(marked.text, "Kobee and Ada");
        assert_eq!(marked.names, vec!["Kobee-Will", "Ada"]);
        assert_eq!(ids, "{P1} and {P2} and (P1)");
    }
}
