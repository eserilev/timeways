//! A clause that only tells that the hero arrived: "$N came in by the Great Forge"
//! (GAMEPLAY.md 3.2.1). The world is the main character, so the arrival of the hero is
//! never the news.
//!
//! The check reads the words right after a word for the hero. A people that comes
//! somewhere stays history: "The Scourge came to Lordaeron" passes, and so do "The trolls
//! came" (a plural) and "The troll Vol'jin came" (a name between).

use crate::check::words_of;
use crate::house::NAME_MARK;

/// The words for the hero in any text. A race, a class, or a title comes on top.
pub(crate) const HERO_WORDS: [&str; 4] = [NAME_MARK, "stranger", "newcomer", "hero"];

/// Small words that can stand between the hero and the verb: "$N has now come".
const BETWEEN: [&str; 10] = [
    "has", "had", "have", "now", "then", "finally", "also", "just", "first", "soon",
];

/// At most this many `BETWEEN` words stand between the hero and the verb.
const MOST_BETWEEN: usize = 2;

/// The verbs of an arrival, with the word after them where the verb alone is a deed.
const ARRIVALS: [&str; 33] = [
    "came",
    "come",
    "comes",
    "walked",
    "walks",
    "entered",
    "enters",
    "arrived",
    "arrives",
    "set foot",
    "sets foot",
    "stood at",
    "stands at",
    "stood before",
    "stood in",
    "climbed up",
    "climbed to",
    "went in",
    "went into",
    "went inside",
    "went down",
    "made it to",
    "stepped in",
    "stepped into",
    "rode in",
    "rode into",
    "crossed to",
    "crossed into",
    "found the way",
    "found a way",
    "found their way",
    "found his way",
    "found her way",
];

/// The first clause of `text` that tells only that the hero arrived, in lower case: "n
/// came" for "$N came". `also_hero` holds more words for the hero of this text, such as
/// "troll" or a title.
#[must_use]
pub fn arrival_in(text: &str, also_hero: &[String]) -> Option<String> {
    let words = words_of(text);
    let heroes: Vec<Vec<String>> = HERO_WORDS
        .iter()
        .copied()
        .chain(also_hero.iter().map(String::as_str))
        .map(words_of)
        .filter(|hero| !hero.is_empty())
        .collect();
    (0..words.len()).find_map(|at| heroes.iter().find_map(|hero| arrival_at(&words, at, hero)))
}

/// The arrival clause that starts at `at` with the words of `hero`, if any.
fn arrival_at(words: &[String], at: usize, hero: &[String]) -> Option<String> {
    if !starts_with(&words[at..], hero) {
        return None;
    }
    let after_hero = at + hero.len();
    let between = words[after_hero..]
        .iter()
        .take(MOST_BETWEEN)
        .take_while(|word| BETWEEN.contains(&word.as_str()))
        .count();
    let verb = after_hero + between;
    let arrival = ARRIVALS
        .iter()
        .map(|arrival| words_of(arrival))
        .find(|arrival| starts_with(&words[verb..], arrival))?;
    Some(words[at..verb + arrival.len()].join(" "))
}

fn starts_with(words: &[String], phrase: &[String]) -> bool {
    words.len() >= phrase.len() && words[..phrase.len()] == *phrase
}
