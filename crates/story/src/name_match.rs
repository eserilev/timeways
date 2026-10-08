//! Where a sentence names a person of the pack (GAMEPLAY.md 5.10). The detectors of outcome
//! and setup passages read it. A long word of the name names its owner alone, so
//! "VanCleef" names "Edwin VanCleef". A capital word before it that is not of the name
//! names someone else: "Emperor Thaurissan" is not "Moira Thaurissan", and "Alexandros
//! Mograine" is not "Renault Mograine". Each name of the person counts (`game_names`).

use crate::game_names;
use std::ops::Range;

/// Words of a name that name no one alone: "Emperor Dagran Thaurissan" is "Thaurissan", not
/// "Emperor". The titles of the game names are here too: "Scarlet Crusaders" never names
/// "Scarlet Commander Mograine".
const TITLES: [&str; 32] = [
    "lord",
    "lady",
    "king",
    "queen",
    "prince",
    "princess",
    "baron",
    "captain",
    "emperor",
    "high",
    "grand",
    "general",
    "commander",
    "master",
    "elder",
    "chief",
    "chieftain",
    "warchief",
    "the",
    "of",
    "devourer",
    "herald",
    "scarlet",
    "inquisitor",
    "mekgineer",
    "tinker",
    "arch",
    "druid",
    "keeper",
    "archmage",
    "magistrate",
    "arcanist",
];

fn is_a_title(word: &str) -> bool {
    TITLES.contains(&word) && word != "the" && word != "of"
}

/// A word of a name names its owner alone when it has at least this many letters.
const NAME_WORD_CHARS: usize = 5;

/// The words of a sentence with their case, so "to Stormwind" is no verb. An apostrophe
/// stays inside its word: "Aku'mai", "VanCleef's".
#[must_use]
pub fn words_in_case(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
        .filter(|word| !word.is_empty())
        .collect()
}

/// The words of a sentence in lower case, with no possessive: "VanCleef's head" gives
/// "vancleef" and "head".
#[must_use]
pub fn lower_words(sentence: &str) -> Vec<String> {
    words_in_case(sentence)
        .iter()
        .map(|word| without_possessive(&word.to_lowercase()).to_string())
        .collect()
}

/// For each word, whether a comma, a colon, or a semicolon stands between it and the word
/// before: "the leader of the Brotherhood, Edwin VanCleef".
fn after_a_comma(sentence: &str) -> Vec<bool> {
    let mut marks = Vec::new();
    let mut pending = false;
    for piece in
        sentence.split_inclusive(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
    {
        let word = piece
            .trim_end_matches(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'));
        if !word.is_empty() {
            marks.push(pending);
            pending = false;
        }
        if piece[word.len()..].contains([',', ';', ':']) {
            pending = true;
        }
    }
    marks
}

fn without_possessive(word: &str) -> &str {
    word.strip_suffix("'s")
        .or_else(|| word.strip_suffix("\u{2019}s"))
        .or_else(|| word.strip_suffix('\''))
        .unwrap_or(word)
}

/// The words of a sentence in lower case, which of them start with a capital letter,
/// which of them were a possessive, and which of them come right after a comma, a colon,
/// or a semicolon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    pub lower: Vec<String>,
    pub capital: Vec<bool>,
    pub possessive: Vec<bool>,
    pub after_a_comma: Vec<bool>,
}

impl Words {
    #[must_use]
    pub fn of(sentence: &str) -> Words {
        let in_case = words_in_case(sentence);
        let capital = in_case
            .iter()
            .map(|word| word.starts_with(char::is_uppercase))
            .collect();
        let possessive = in_case
            .iter()
            .map(|word| without_possessive(word).len() < word.len())
            .collect();
        Words {
            lower: lower_words(sentence),
            capital,
            possessive,
            after_a_comma: after_a_comma(sentence),
        }
    }

    /// The words in lower case, with an empty word for each possessive: "the adventurers'
    /// hall" names no adventurers.
    #[must_use]
    pub fn without_possessives(&self) -> Vec<String> {
        self.lower
            .iter()
            .zip(&self.possessive)
            .map(|(word, possessive)| {
                if *possessive {
                    String::new()
                } else {
                    word.clone()
                }
            })
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lower.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lower.is_empty()
    }

    /// The words of `range` name the person under one of its names.
    #[must_use]
    pub fn names(&self, range: Range<usize>, name: &str) -> bool {
        self.name_at(range, name).is_some()
    }

    /// The index of the first word of `range` that names the person under one of its
    /// names. A range past the end is cut at the end.
    #[must_use]
    pub fn name_at(&self, range: Range<usize>, name: &str) -> Option<usize> {
        let range = range.start.min(self.len())..range.end.min(self.len());
        let wiki = game_names::wiki_name(name);
        let mut names = vec![wiki];
        names.extend(game_names::game_names_of(wiki));
        names
            .into_iter()
            .filter_map(|one| self.one_name_at(range.clone(), one))
            .min()
    }

    /// The word at `last` ends a name of the person: the last word of the whole name, or a
    /// long word of it on its own.
    #[must_use]
    pub fn name_ends_at(&self, last: usize, name: &str) -> bool {
        let wiki = game_names::wiki_name(name);
        let mut names = vec![wiki];
        names.extend(game_names::game_names_of(wiki));
        names.into_iter().any(|one| {
            let parts = lower_words(one);
            let fits = !parts.is_empty() && parts.len() <= last + 1 && last < self.len();
            let whole = fits && self.lower[last + 1 - parts.len()..=last] == parts[..];
            whole || self.one_name_at(last..last + 1, one) == Some(last)
        })
    }

    fn one_name_at(&self, range: Range<usize>, name: &str) -> Option<usize> {
        let parts = lower_words(name);
        let words = &self.lower[range.clone()];
        let whole = (!parts.is_empty())
            .then(|| {
                words
                    .windows(parts.len())
                    .position(|window| window == parts)
            })
            .flatten()
            .map(|at| range.start + at);
        let is_long = |word: &String| {
            word.chars().count() >= NAME_WORD_CHARS && !TITLES.contains(&word.as_str())
        };
        let single = range.clone().find(|at| {
            let word = &self.lower[*at];
            let alone = !game_names::is_shared(word) || self.own_title_before(*at, &parts);
            is_long(word) && parts.contains(word) && alone && !self.named_other(*at, &parts)
        });
        // A title of this name counts for where the name starts: "Emperor Thaurissan" starts
        // at "Emperor".
        let start = single.map(|at| {
            let own_title = at > range.start && self.own_title_before(at, &parts);
            if own_title { at - 1 } else { at }
        });
        whole.into_iter().chain(start).min()
    }

    /// The word before `at` is a title of the name: "Emperor Thaurissan". "the" and "of"
    /// are no titles here: "Avatar of Hakkar" starts at "Avatar".
    fn own_title_before(&self, at: usize, parts: &[String]) -> bool {
        at.checked_sub(1).is_some_and(|before| {
            let word = &self.lower[before];
            is_a_title(word) && parts.contains(word)
        })
    }

    /// The word before `at` is a title, or a capital word, that is no part of the name. The
    /// first word of a sentence has a capital letter anyway, so it counts only as a title.
    fn named_other(&self, at: usize, parts: &[String]) -> bool {
        let Some(before) = at.checked_sub(1) else {
            return false;
        };
        let word = &self.lower[before];
        let is_title = is_a_title(word);
        let is_capital = before > 0 && self.capital[before] && !self.after_a_comma[at];
        (is_title || is_capital) && !parts.contains(word)
    }
}
