//! The checks of the style guide that hold for every text of the narrator: a line, a saga,
//! a tale, the summary, and a zone history (docs/plans/narrator-style.md 10.1). Each fault
//! holds the words that broke the rule, so a retry can name them.

use crate::check::{mentions, words_of};
use crate::group_words::group_word_hero_in;
use crate::inside_hero::{inside_hero_in, recognition_in};
use crate::sentences::{sentences, word_count};
use std::fmt;
use std::ops::RangeInclusive;

/// A sentence with fewer words is a fragment, such as "Level 10." or "Gone."
pub const FEWEST_WORDS: usize = 4;

/// The guide asks for 25 words at most. Names of several words get a margin.
pub const MOST_WORDS: usize = 30;

/// The characters between "not" and "but" in the pivot "not X, but Y", from the regex
/// `not [^.!?]{3,60} but` of the antislop lists.
const PIVOT_GAP: RangeInclusive<usize> = 3..=60;

/// The verbs of a sentence that cites its source: "Renferrel spoke of the plague".
const CITES: [&str; 4] = ["spoke of", "speaks of", "told of", "tells of"];

/// The words for a number after "Level" at the start of a text: "Level ten came". A
/// saga tells every level up to 60, and "fifty-five" starts with "fifty".
const NUMBER_WORDS: [&str; 27] = [
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "first",
    "tenth",
    "twentieth",
];

/// The longest start of a sentence that a fault shows.
const SHOWN_WORDS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProseFault {
    /// An order or a power grows inside the hero: "grows in the druid".
    InsideHero(String),
    /// A people knows, fears, or honors the hero.
    Recognition(String),
    /// "Someone spoke of X. In Y, its Z does W."
    SourceShape(String),
    Fragment(String),
    LongSentence(String),
    /// "Not X, but Y."
    Pivot(String),
    /// The text opens with "Level 10" or "Level ten".
    LevelOpener,
    /// The hero is named by a word that names a group of the text: "the Forsaken has".
    GroupWordForHero(String),
}

impl fmt::Display for ProseFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProseFault::InsideHero(clause) => write!(
                f,
                "\"{clause}\" puts a power inside the hero. Nothing grows or lives inside \
                 the hero. Tell what the order or the people does."
            ),
            ProseFault::Recognition(clause) => write!(
                f,
                "\"{clause}\" has the world know the hero. Leave it out, and tell what changed \
                 in the world."
            ),
            ProseFault::SourceShape(sentence) => write!(
                f,
                "\"{sentence}\" follows a sentence about who spoke of the lore. Tell the \
                 history itself, and never who told it."
            ),
            ProseFault::Fragment(sentence) => write!(
                f,
                "\"{sentence}\" is a fragment. Write whole sentences of 12 to 25 words."
            ),
            ProseFault::LongSentence(start) => write!(
                f,
                "The sentence \"{start}...\" has more than {MOST_WORDS} words. Keep each \
                 sentence to 25 words."
            ),
            ProseFault::Pivot(phrase) => write!(
                f,
                "\"{phrase}\" is a \"not X, but Y\" turn. Say what is true, plainly."
            ),
            ProseFault::LevelOpener => write!(
                f,
                "The text opens with the level. Open with the history of the place, the foe, \
                 or the people."
            ),
            ProseFault::GroupWordForHero(naming) => write!(
                f,
                "\"{naming}\" names the hero by a word that also names a people or a group. \
                 Name the hero $N, or leave the hero out."
            ),
        }
    }
}

/// Every fault of `text`, in order. `hero_words` holds the race, the class, and the title
/// of the hero, past `$N` and "hero".
#[must_use]
pub fn prose_faults(text: &str, hero_words: &[String]) -> Vec<ProseFault> {
    let sentences = sentences(text);
    let mut faults: Vec<ProseFault> = Vec::new();
    faults.extend(inside_hero_in(text, hero_words).map(ProseFault::InsideHero));
    faults.extend(recognition_in(text, hero_words).map(ProseFault::Recognition));
    faults.extend(source_shape_in(&sentences).map(ProseFault::SourceShape));
    faults.extend(fragment_in(&sentences).map(ProseFault::Fragment));
    faults.extend(long_sentence_in(&sentences).map(ProseFault::LongSentence));
    faults.extend(
        sentences
            .iter()
            .find_map(|sentence| pivot_in(sentence))
            .map(ProseFault::Pivot),
    );
    if opens_with_a_level(text) {
        faults.push(ProseFault::LevelOpener);
    }
    faults.extend(group_word_hero_in(text, hero_words).map(ProseFault::GroupWordForHero));
    faults
}

fn fragment_in(sentences: &[&str]) -> Option<String> {
    let fragment = sentences
        .iter()
        .find(|sentence| word_count(sentence) < FEWEST_WORDS)?;
    Some((*fragment).to_string())
}

/// The first words of the first sentence over `MOST_WORDS`.
fn long_sentence_in(sentences: &[&str]) -> Option<String> {
    let long = sentences
        .iter()
        .find(|sentence| word_count(sentence) > MOST_WORDS)?;
    let start: Vec<&str> = long.split_whitespace().take(SHOWN_WORDS).collect();
    Some(start.join(" "))
}

/// The sentence after one that cites a source, when it opens "In <Name>, its".
fn source_shape_in(sentences: &[&str]) -> Option<String> {
    let cites = |sentence: &str| CITES.iter().any(|verb| mentions(sentence, verb));
    let pair = sentences
        .windows(2)
        .find(|pair| cites(pair[0]) && opens_in_a_place_with_its(pair[1]))?;
    Some(pair[1].to_string())
}

/// "In Undercity, its" or "In the Redridge Mountains, its": a name of at most 4 words.
fn opens_in_a_place_with_its(sentence: &str) -> bool {
    let Some(rest) = sentence.strip_prefix("In ") else {
        return false;
    };
    let Some((place, after)) = rest.split_once(", ") else {
        return false;
    };
    let named = place
        .split_whitespace()
        .any(|word| word.starts_with(char::is_uppercase));
    let short = place.split_whitespace().count() <= 4;
    named && short && words_of(after).first().is_some_and(|word| word == "its")
}

/// The words from "not" to "but" in one sentence, with 3 to 60 characters between them.
fn pivot_in(sentence: &str) -> Option<String> {
    let buts = whole_word_starts(sentence, "but");
    whole_word_starts(sentence, "not")
        .into_iter()
        .find_map(|not| {
            let after_not = not + "not".len();
            let but = buts.iter().copied().find(|&but| {
                but > after_not
                    && PIVOT_GAP.contains(&sentence[after_not..but].trim().chars().count())
            })?;
            Some(sentence[not..but + "but".len()].to_string())
        })
}

/// The byte offsets where `word`, in ASCII, starts as a whole word of `text`, in any case.
fn whole_word_starts(text: &str, word: &str) -> Vec<usize> {
    let is_word_char = |c: char| c.is_alphanumeric() || c == '\'';
    text.char_indices()
        .map(|(at, _)| at)
        .filter(|&at| {
            let same = text
                .get(at..at + word.len())
                .is_some_and(|found| found.eq_ignore_ascii_case(word));
            let before = text[..at].chars().next_back();
            let after = text
                .get(at + word.len()..)
                .and_then(|rest| rest.chars().next());
            same && !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
        })
        .collect()
}

/// True when the text opens with "Level" and a number: "Level 10." or "Level ten came".
fn opens_with_a_level(text: &str) -> bool {
    let words = words_of(text);
    let [first, second, ..] = words.as_slice() else {
        return false;
    };
    let number =
        second.chars().all(|c| c.is_ascii_digit()) || NUMBER_WORDS.contains(&second.as_str());
    first == "level" && number
}
