//! No invented names (GAMEPLAY.md 3.2.1): every proper name of a model text is a word of
//! what the prompt gave. The rule lives in `timeways_rules::grounding`, where Lean proves
//! it. This module finds the names, and turns words into ids.
//!
//! A name is a capital word that `data/common_names.txt` does not list. A capital word
//! alone at the start of a sentence is no name, because most such words are plain words.
//! The word of a name keeps its apostrophes and hyphens, so "Gath'Ilzogg" is one word. The
//! check ignores case, an "'s" at the end, and a plural "s". It reads each word alone, so
//! "VanCleef" passes when the prompt names "Edwin VanCleef", and "Captain Greenskin" when
//! it names "Greenskin".

use crate::check::{data_lines, names_after_cutoff};
use crate::prompt::RETRY_MARK;
use crate::samples::is_sample_heading;
use std::collections::HashMap;
use timeways_rules::grounding::{grounded, is_given};

const COMMON_NAMES: &str = include_str!("../data/common_names.txt");

const FENCE_OPEN: &str = "<<<\n";
const FENCE_CLOSE: &str = "\n>>>";

/// A word of a name shorter than this is a mark or an initial, such as "I" or the "N" of
/// `$N`.
const SHORTEST_NAME: usize = 2;

/// The names of `text` that `given` does not hold, as the text writes them: "Deuce
/// Waterman". `given` is what the prompt gave the model (`given_text`), and the player's
/// own text: a name that the player wrote first is no invention (GAMEPLAY.md 3.7).
#[must_use]
pub fn ungrounded_names(text: &str, given: &str) -> Vec<String> {
    let mut ids = Ids::default();
    let given_ids = ids.all(&given_keys(given));
    let names = names_in(text);
    let name_ids: Vec<Vec<u32>> = names.iter().map(|name| ids.all(&name.keys)).collect();
    if grounded(&name_ids.concat(), &given_ids) {
        return Vec::new();
    }
    let mut ungrounded: Vec<String> = Vec::new();
    for (name, keys) in names.iter().zip(&name_ids) {
        let new = keys.iter().any(|key| !is_given(*key, &given_ids));
        // A later name gets its own fault, so a retry reads one reason for it.
        let later = !names_after_cutoff(&name.text).is_empty();
        if new && !later && !ungrounded.contains(&name.text) {
            ungrounded.push(name.text.clone());
        }
    }
    ungrounded
}

/// What a prompt gave the model: the text between its fence marks, except the golden
/// samples. A sample names other places and people, so a name of a sample is invented
/// for this moment. A retry stops at the last answer, because the model wrote it.
#[must_use]
pub fn given_text(prompt: &str) -> String {
    let first = prompt.split(RETRY_MARK).next().unwrap_or_default();
    let mut given: Vec<&str> = Vec::new();
    let mut rest = first;
    while let Some(open) = rest.find(FENCE_OPEN) {
        let before = &rest[..open];
        let inside = &rest[open + FENCE_OPEN.len()..];
        let Some(close) = inside.find(FENCE_CLOSE) else {
            break;
        };
        let heading = before.trim_end().lines().last().unwrap_or_default();
        if !is_sample_heading(heading) {
            given.push(&inside[..close]);
        }
        rest = &inside[close + FENCE_CLOSE.len()..];
    }
    given.join("\n")
}

/// A run of capital words, such as "Deuce Waterman", and the keys of the words that the
/// check reads.
struct Name {
    text: String,
    keys: Vec<String>,
}

fn names_in(text: &str) -> Vec<Name> {
    let common = common_keys();
    let mut names = Vec::new();
    for run in capital_runs(&words(text)) {
        if run.len() == 1 && run[0].opens_sentence {
            continue;
        }
        let checked: Vec<(Word<'_>, Vec<String>)> = run
            .iter()
            .map(|word| (*word, name_keys(word, &common)))
            .filter(|(_, keys)| !keys.is_empty())
            .collect();
        let (Some((first, _)), Some((last, _))) = (checked.first(), checked.last()) else {
            continue;
        };
        names.push(Name {
            text: text[first.at..last.at + last.text.len()].to_string(),
            keys: checked.iter().flat_map(|(_, keys)| keys.clone()).collect(),
        });
    }
    names
}

/// The keys of a word of a name that the check reads: none for `$N` or a common word.
fn name_keys(word: &Word<'_>, common: &[String]) -> Vec<String> {
    if word.marked {
        return Vec::new();
    }
    name_parts(word.text)
        .into_iter()
        .map(key)
        .filter(|key| !common.contains(key))
        .collect()
}

/// The capital parts of a word that the check reads: a hyphen splits "Stormwind-born", and
/// an apostrophe keeps "Gath'Ilzogg" whole. A part of one letter, or with a digit, is none.
fn name_parts(word: &str) -> Vec<&str> {
    word.split('-')
        .filter(|part| part.chars().next().is_some_and(char::is_uppercase))
        .filter(|part| part.chars().count() >= SHORTEST_NAME)
        .filter(|part| !part.chars().any(|c| c.is_ascii_digit()))
        .collect()
}

/// Every word of the given text, and each part of a word with an apostrophe or a hyphen,
/// as keys. The case of a given word does not matter.
fn given_keys(given: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for word in words(given) {
        keys.push(key(word.text));
        let parts = word.text.split(['-', '\'', '\u{2019}']);
        keys.extend(parts.filter(|part| !part.is_empty()).map(key));
    }
    keys
}

fn common_keys() -> Vec<String> {
    data_lines(COMMON_NAMES).map(key).collect()
}

/// A word in lower case, with one apostrophe for every kind, and with no "'s", no
/// apostrophe at the end, and no plural "s".
fn key(word: &str) -> String {
    let lower = word.to_lowercase().replace('\u{2019}', "'");
    let bare = lower
        .strip_suffix("'s")
        .unwrap_or(&lower)
        .trim_end_matches('\'');
    match bare.strip_suffix('s') {
        Some(singular) if singular.chars().count() >= 3 => singular.to_string(),
        _ => bare.to_string(),
    }
}

/// A word of a text, and where it stands.
#[derive(Clone, Copy)]
struct Word<'a> {
    text: &'a str,
    /// The byte where the word starts in its text.
    at: usize,
    /// The first word of the text, or the first after a mark that ends a sentence.
    opens_sentence: bool,
    /// Only spaces stand between this word and the one before it.
    joined: bool,
    /// A `$` stands right before it, as in `$N`.
    marked: bool,
}

/// The marks that end a sentence or open a quote, so the word after them opens one.
fn ends_a_sentence(c: char) -> bool {
    matches!(
        c,
        '.' | '!' | '?' | ':' | ';' | '"' | '\u{201c}' | '\u{201d}' | '\n'
    )
}

/// The words of a text: runs of letters and digits, with an apostrophe or a hyphen inside.
fn words(text: &str) -> Vec<Word<'_>> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut found = Vec::new();
    let mut gap_start = 0;
    let mut index = 0;
    while index < chars.len() {
        if !chars[index].1.is_alphanumeric() {
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && is_inside_a_word(&chars, index) {
            index += 1;
        }
        let begin = chars[start].0;
        let end = chars.get(index).map_or(text.len(), |(at, _)| *at);
        let gap = &text[gap_start..begin];
        found.push(Word {
            text: &text[begin..end],
            at: begin,
            opens_sentence: found.is_empty() || gap.chars().any(ends_a_sentence),
            joined: !found.is_empty() && gap.chars().all(|c| c == ' '),
            marked: gap.ends_with('$'),
        });
        gap_start = end;
    }
    found
}

/// A letter or a digit, or an apostrophe or a hyphen between two of them.
fn is_inside_a_word(chars: &[(usize, char)], index: usize) -> bool {
    let c = chars[index].1;
    if c.is_alphanumeric() {
        return true;
    }
    let joins = matches!(c, '\'' | '\u{2019}' | '-');
    let before = index > 0 && chars[index - 1].1.is_alphanumeric();
    let after = chars
        .get(index + 1)
        .is_some_and(|(_, c)| c.is_alphanumeric());
    joins && before && after
}

/// The runs of capital words with only spaces between them.
fn capital_runs<'a>(words: &[Word<'a>]) -> Vec<Vec<Word<'a>>> {
    let mut runs: Vec<Vec<Word<'a>>> = Vec::new();
    let mut joined_to_last = false;
    for word in words {
        let capital = word.text.chars().next().is_some_and(char::is_uppercase);
        if !capital {
            joined_to_last = false;
            continue;
        }
        match runs.last_mut() {
            Some(run) if joined_to_last && word.joined && !word.opens_sentence => run.push(*word),
            _ => runs.push(vec![*word]),
        }
        joined_to_last = true;
    }
    runs
}

/// One id for each distinct key.
#[derive(Default)]
struct Ids {
    known: HashMap<String, u32>,
}

impl Ids {
    fn all(&mut self, keys: &[String]) -> Vec<u32> {
        keys.iter().map(|key| self.id(key)).collect()
    }

    fn id(&mut self, key: &str) -> u32 {
        let next = u32::try_from(self.known.len()).unwrap_or(u32::MAX);
        *self.known.entry(key.to_string()).or_insert(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_that_the_prompt_never_gave_is_ungrounded() {
        let names = ungrounded_names(
            "The Defias hold the mine, and Deuce Waterman still holds The Deadmines.",
            "The Defias Brotherhood holds the Deadmines.",
        );

        assert_eq!(names, ["Deuce Waterman"]);
    }

    #[test]
    fn a_lone_capital_word_at_the_start_of_a_sentence_is_no_name() {
        assert_eq!(
            ungrounded_names("Bandits hold the fields.", ""),
            [] as [&str; 0]
        );
    }

    #[test]
    fn the_hero_mark_is_no_name() {
        assert_eq!(
            ungrounded_names("Hogger is dead, and $N did it.", "Hogger"),
            [] as [&str; 0]
        );
    }

    #[test]
    fn a_key_drops_the_possessive_and_the_plural() {
        assert_eq!(key("VanCleef's"), "vancleef");
        assert_eq!(key("Murlocs"), "murloc");
        assert_eq!(key("Miners\u{2019}"), "miner");
    }
}
