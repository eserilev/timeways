//! Two clauses that make the hero the center of the world (docs/plans/narrator-style.md
//! 10.1): an order or a power that grows inside the hero ("Its power grows in the
//! druid"), and a people that knows the hero ("The Riverpaw remember the paladin").
//!
//! A race, a class, or a title of the hero names the hero only when it stands alone. A
//! name after it ("the paladin Uther"), a possessive ("the Forsaken's crypts"), or "of",
//! "who", or "that" after it makes it a person of the lore or a people.

use crate::arrival::HERO_WORDS;
use crate::check::words_of;
use crate::sentences::sentences;

/// The verbs of a power that grows or lives somewhere.
const GROWS: [&str; 17] = [
    "grow", "grows", "grew", "live", "lives", "lived", "burn", "burns", "burned", "stir", "stirs",
    "stirred", "rise", "rises", "rose", "sharper", "stronger",
];

/// At most this many words stand between the verb and the word of place: "lives on in".
const MOST_BETWEEN: usize = 2;

const INSIDE: [&str; 4] = ["in", "within", "inside", "through"];

/// The verbs of a people that knows or honors the hero.
const KNOWS: [&str; 20] = [
    "know",
    "knows",
    "knew",
    "remember",
    "remembers",
    "remembered",
    "recognize",
    "recognizes",
    "recognized",
    "fear",
    "fears",
    "feared",
    "greet",
    "greets",
    "greeted",
    "honor",
    "honors",
    "honored",
    "thank",
    "thanks",
];

/// Small words that stand before the hero: "in the druid".
const ARTICLES: [&str; 5] = ["the", "a", "an", "this", "that"];

/// Words after a race or a class that make it a people or a part of a name.
const NOT_THE_HERO: [&str; 4] = ["s", "of", "who", "that"];

/// The first clause of `text` where a power grows inside the hero, in lower case:
/// "grows in the druid". `also_hero` holds the race, the class, and the title words.
#[must_use]
pub fn inside_hero_in(text: &str, also_hero: &[String]) -> Option<String> {
    let heroes = Heroes::of(also_hero);
    sentences(text).into_iter().find_map(|sentence| {
        let words = Words::of(sentence);
        (0..words.lower.len()).find_map(|at| inside_at(&words, at, &heroes))
    })
}

/// The first clause of `text` where a people knows or honors the hero, in lower case:
/// "remember the paladin".
#[must_use]
pub fn recognition_in(text: &str, also_hero: &[String]) -> Option<String> {
    let heroes = Heroes::of(also_hero);
    sentences(text).into_iter().find_map(|sentence| {
        let words = Words::of(sentence);
        (0..words.lower.len()).find_map(|at| knows_at(&words, at, &heroes))
    })
}

fn inside_at(words: &Words, at: usize, heroes: &Heroes) -> Option<String> {
    if !GROWS.contains(&words.lower[at].as_str()) {
        return None;
    }
    (0..=MOST_BETWEEN).find_map(|between| {
        let place = at + 1 + between;
        let word = words.lower.get(place)?;
        if !INSIDE.contains(&word.as_str()) {
            return None;
        }
        let end = heroes.end_at(words, place + 1)?;
        Some(words.lower[at..end].join(" "))
    })
}

fn knows_at(words: &Words, at: usize, heroes: &Heroes) -> Option<String> {
    if !KNOWS.contains(&words.lower[at].as_str()) {
        return None;
    }
    let end = heroes.end_at(words, at + 1)?;
    Some(words.lower[at..end].join(" "))
}

/// The words of one sentence, as written and in lower case.
struct Words<'a> {
    written: Vec<&'a str>,
    lower: Vec<String>,
}

impl<'a> Words<'a> {
    fn of(sentence: &'a str) -> Words<'a> {
        let written: Vec<&str> = sentence
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        let lower = written.iter().map(|word| word.to_lowercase()).collect();
        Words { written, lower }
    }

    fn starts_with(&self, at: usize, phrase: &[String]) -> bool {
        self.lower.get(at..at + phrase.len()) == Some(phrase)
    }
}

/// The words for the hero: the words of every text, and the race, the class, and the
/// title of this hero.
struct Heroes {
    always: Vec<Vec<String>>,
    kinds: Vec<Vec<String>>,
}

impl Heroes {
    fn of(also_hero: &[String]) -> Heroes {
        Heroes {
            always: phrases(HERO_WORDS.iter().copied()),
            kinds: phrases(also_hero.iter().map(String::as_str)),
        }
    }

    /// The end of the hero that starts at `at`, after an article, if any.
    fn end_at(&self, words: &Words, at: usize) -> Option<usize> {
        let article = words
            .lower
            .get(at)
            .is_some_and(|word| ARTICLES.contains(&word.as_str()));
        let start = at + usize::from(article);
        let always = self
            .always
            .iter()
            .find(|hero| words.starts_with(start, hero));
        if let Some(hero) = always {
            return Some(start + hero.len());
        }
        let kind = self
            .kinds
            .iter()
            .find(|hero| words.starts_with(start, hero))?;
        let end = start + kind.len();
        stands_alone(words, end).then_some(end)
    }
}

fn phrases<'a>(words: impl Iterator<Item = &'a str>) -> Vec<Vec<String>> {
    words
        .map(words_of)
        .filter(|phrase| !phrase.is_empty())
        .collect()
}

/// True when the word at `end` makes no name, possessive, or people of the word before it.
fn stands_alone(words: &Words, end: usize) -> bool {
    let Some(next) = words.written.get(end) else {
        return true;
    };
    let name = next.chars().next().is_some_and(char::is_uppercase);
    !name && !NOT_THE_HERO.contains(&words.lower[end].as_str())
}
