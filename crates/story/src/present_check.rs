//! The present of a narrator history (docs/plans/lore-names-and-now.md 2.3 C). A clause in
//! the present tense tells what holds now. The lore of the prompt must hold it too, and it
//! never tells a foe that the player defeated as alive. The tense test is a heuristic: a
//! present verb outside its lists passes unchecked, and a false refusal costs one retry.

use crate::check::{content_words, mentions, words_of};
use crate::sentences::sentences;

/// Verbs that tell a present state.
const PRESENT_VERBS: [&str; 20] = [
    "is", "are", "has", "have", "holds", "hold", "rules", "remains", "remain", "lies", "lives",
    "live", "guards", "leads", "stands", "stand", "serves", "serve", "rests", "dwells",
];

/// Words that tell a past after "still" or before "now": "still remained", "was now".
const PAST_WORDS: [&str; 34] = [
    "was", "were", "had", "held", "stood", "lay", "ran", "rose", "fell", "took", "led", "kept",
    "knew", "grew", "came", "went", "did", "could", "swore", "built", "made", "sent", "fought",
    "found", "left", "brought", "taught", "lost", "won", "began", "became", "gave", "told",
    "drove",
];

/// Words in "-s" that are no verb.
const NOT_VERBS: [&str; 14] = [
    "its",
    "his",
    "hers",
    "this",
    "thus",
    "us",
    "always",
    "perhaps",
    "towards",
    "afterwards",
    "sometimes",
    "besides",
    "as",
    "was",
];

/// A word in "-s" after a name is a noun when one of these follows it: "the Gurubashi
/// trolls once ruled".
const AFTER_A_NOUN: [&str; 14] = [
    "once", "were", "was", "had", "have", "are", "is", "of", "and", "who", "that", "which", "from",
    "in",
];

/// Past words that start a state with no end: "began to roam" still holds now.
const STATE_STARTS: [&[&str]; 9] = [
    &["began", "to"],
    &["begun", "to"],
    &["took", "over"],
    &["take", "over"],
    &["taken", "over"],
    &["has", "since"],
    &["have", "since"],
    &["fell", "into", "the", "hands", "of"],
    &["fallen", "into", "the", "hands", "of"],
];

/// Words of a present clause that tell nothing about what holds: the marks of the tense.
const TENSE_WORDS: [&str; 6] = ["still", "now", "there", "here", "it", "they"];

/// The marks that end a clause inside a sentence.
const CLAUSE_MARKS: [char; 4] = [',', ';', ':', '\u{2014}'];

/// The quote marks of the lore. A sentence inside them is what someone said, not a fact.
const QUOTES: [char; 3] = ['"', '\u{201C}', '\u{201D}'];

/// A last word of a name names its owner alone when it has at least this many letters:
/// "VanCleef" for "Edwin VanCleef".
const NAME_WORD_CHARS: usize = 4;

/// Why a present clause is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentFault {
    /// The lore holds no present source for the clause.
    Unsourced(String),
    /// The clause tells a defeated foe as alive: the clause, then the foe.
    Defeated(String, String),
}

/// What a history is checked against: the lore of the prompt, the names of the moment,
/// and the foes that the player defeated.
#[derive(Clone, Copy, Debug)]
pub struct PresentGrounds<'a> {
    pub lore: &'a str,
    pub names: &'a [String],
    pub defeated: &'a [String],
}

/// The faults of each present clause of `history`. A clause needs a source in the lore: a
/// present sentence, or a past one that starts a state with no end, that shares one word
/// that tells something with it, past the names of the moment. A clause that names a
/// defeated foe is refused whatever the lore says.
#[must_use]
pub fn unsourced_present_in(history: &str, grounds: &PresentGrounds<'_>) -> Vec<PresentFault> {
    let source = source_words(grounds.lore);
    let names: Vec<String> = grounds
        .names
        .iter()
        .flat_map(|name| content_words(name))
        .collect();
    let mut faults = Vec::new();
    for clause in clauses(history) {
        if !is_present(clause) {
            continue;
        }
        if let Some(foe) = defeated_in(clause, grounds.defeated) {
            faults.push(PresentFault::Defeated(clause.to_string(), foe.to_string()));
            continue;
        }
        let telling = telling_words(clause, &names);
        if !telling.iter().any(|word| source.contains(word)) {
            faults.push(PresentFault::Unsourced(clause.to_string()));
        }
    }
    faults
}

/// True when a sentence of the text, outside quote marks, is in the present tense.
#[must_use]
pub fn has_present_sentence(text: &str) -> bool {
    sentences(&without_quotes(text))
        .into_iter()
        .any(|sentence| clauses(sentence).into_iter().any(is_present))
}

fn clauses(text: &str) -> Vec<&str> {
    sentences(text)
        .into_iter()
        .flat_map(|sentence| sentence.split(CLAUSE_MARKS))
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .collect()
}

/// A clause is present when it holds a present verb, a verb after "still", "now" after no
/// past word, or a verb in "-s" right after a name.
fn is_present(clause: &str) -> bool {
    let written: Vec<&str> = clause_words(clause);
    let lower: Vec<String> = written.iter().map(|word| word.to_lowercase()).collect();
    (0..lower.len()).any(|at| {
        present_verb_at(&lower, at)
            || still_verb_at(&lower, at)
            || now_at(&lower, at)
            || name_verb_at(&written, &lower, at)
    })
}

/// The words of a clause, with their case and their apostrophes.
fn clause_words(clause: &str) -> Vec<&str> {
    clause
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
        .filter(|word| !word.is_empty())
        .collect()
}

/// "to hold" is no present: the verb of an infinitive has no tense.
fn present_verb_at(lower: &[String], at: usize) -> bool {
    PRESENT_VERBS.contains(&lower[at].as_str()) && (at == 0 || lower[at - 1] != "to")
}

fn still_verb_at(lower: &[String], at: usize) -> bool {
    lower[at] == "still" && lower.get(at + 1).is_some_and(|next| !is_past(next))
}

fn now_at(lower: &[String], at: usize) -> bool {
    lower[at] == "now" && (at == 0 || !is_past(&lower[at - 1]))
}

fn name_verb_at(written: &[&str], lower: &[String], at: usize) -> bool {
    if at == 0 {
        return false;
    }
    let before = written[at - 1];
    let word = lower[at].as_str();
    let after_a_name = before.starts_with(char::is_uppercase)
        && !is_possessive(before)
        && !content_words(before).is_empty();
    let looks_like_a_verb = word.ends_with('s')
        && !word.ends_with("ss")
        && !word.ends_with("ous")
        && !is_possessive(word)
        && word.chars().count() > 3
        && !NOT_VERBS.contains(&word)
        && written[at].starts_with(char::is_lowercase);
    let noun_after = lower
        .get(at + 1)
        .is_some_and(|next| AFTER_A_NOUN.contains(&next.as_str()) || is_past(next));
    after_a_name && looks_like_a_verb && !noun_after
}

fn is_possessive(word: &str) -> bool {
    word.ends_with("'s") || word.ends_with("\u{2019}s") || word.ends_with('\'')
}

fn is_past(word: &str) -> bool {
    word.ends_with("ed") || PAST_WORDS.contains(&word)
}

/// The words of the lore that can source a present clause, each as its stem.
fn source_words(lore: &str) -> Vec<String> {
    let plain = without_quotes(lore);
    sentences(&plain)
        .into_iter()
        .filter(|sentence| is_source(sentence))
        .flat_map(content_words)
        .map(|word| stem(&word))
        .collect()
}

/// A source is a present sentence, a past one that starts a state with no end, or one
/// with no past word at all: "The ruins of their cities line the coast" has a present
/// verb that no list holds.
fn is_source(sentence: &str) -> bool {
    let lower = words_of(sentence);
    let starts_a_state = STATE_STARTS
        .iter()
        .any(|phrase| lower.windows(phrase.len()).any(|run| run == *phrase));
    let has_a_past = lower.iter().any(|word| is_past(word));
    starts_a_state || !has_a_past || clauses(sentence).into_iter().any(is_present)
}

/// The words of a clause that tell what holds: no mark of the tense, no present verb, and
/// no word of a name of the moment. Each as its stem.
fn telling_words(clause: &str, names: &[String]) -> Vec<String> {
    content_words(clause)
        .into_iter()
        .filter(|word| !TENSE_WORDS.contains(&word.as_str()))
        .filter(|word| !PRESENT_VERBS.contains(&word.as_str()))
        .filter(|word| !names.contains(word))
        .map(|word| stem(&word))
        .collect()
}

/// A word with no plural "s": "trolls" and "troll" share a stem.
fn stem(word: &str) -> String {
    word.strip_suffix('s')
        .filter(|rest| !rest.ends_with('s'))
        .unwrap_or(word)
        .to_string()
}

/// The text with each span in quote marks cut out.
fn without_quotes(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut inside = false;
    for c in text.chars() {
        if QUOTES.contains(&c) {
            inside = !inside;
            continue;
        }
        if !inside {
            kept.push(c);
        }
    }
    kept
}

/// The defeated foe that the clause names: the whole name, or the last word of a name of
/// two words or more, written with a capital: "VanCleef" for "Edwin VanCleef".
fn defeated_in<'a>(clause: &str, defeated: &'a [String]) -> Option<&'a str> {
    let written = clause_words(clause);
    defeated
        .iter()
        .find(|foe| mentions(clause, foe) || names_by_last_word(&written, foe))
        .map(String::as_str)
}

fn names_by_last_word(written: &[&str], foe: &str) -> bool {
    let parts: Vec<&str> = foe.split_whitespace().collect();
    let Some(last) = parts.last().filter(|_| parts.len() > 1) else {
        return false;
    };
    last.chars().count() >= NAME_WORD_CHARS
        && last.starts_with(char::is_uppercase)
        && written.iter().any(|word| word == last)
}
