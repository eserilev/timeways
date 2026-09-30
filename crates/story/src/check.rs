//! The checks on a model answer before it shows (GAMEPLAY.md 3.1, 3.2.1, and 5.9, layer 4).

use crate::samples;
use std::fmt;

const LATER_NAMES: &str = include_str!("../data/later_names.txt");
const BANNED_WORDS: &str = include_str!("../data/banned_words.txt");

/// About 150 words. The prompt asks for 80, so this catches only a runaway answer.
pub const MAX_CHARS: usize = 1000;

/// The words that mark an answer that no passage supports.
const NO_SOURCE: [&str; 2] = ["nobody knows", "legend says"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    Empty,
    TooLong { chars: usize },
    NoCitation,
    UnknownCitation { number: usize },
    LaterName { name: String },
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::Empty => write!(f, "The answer is empty."),
            Fault::TooLong { chars } => {
                write!(
                    f,
                    "The answer has {chars} characters. The limit is {MAX_CHARS}."
                )
            }
            Fault::NoCitation => write!(
                f,
                "The answer cites no passage. Cite one, or say \"Nobody knows\" or \"Legend says\"."
            ),
            Fault::UnknownCitation { number } => write!(f, "No passage has the number [{number}]."),
            Fault::LaterName { name } => {
                write!(f, "\"{name}\" is from after the year 25 ADP. Leave it out.")
            }
        }
    }
}

/// Every fault at once, so one retry fixes all of them.
#[must_use]
pub fn check(answer: &str, passage_count: usize) -> Vec<Fault> {
    let answer = answer.trim();
    if answer.is_empty() {
        return vec![Fault::Empty];
    }
    let mut faults = Vec::new();
    let chars = answer.chars().count();
    if chars > MAX_CHARS {
        faults.push(Fault::TooLong { chars });
    }
    let cited = citations(answer);
    if cited.is_empty() && !admits_no_source(answer) {
        faults.push(Fault::NoCitation);
    }
    for number in cited {
        if number == 0 || number > passage_count {
            faults.push(Fault::UnknownCitation { number });
        }
    }
    for name in names_after_cutoff(answer) {
        faults.push(Fault::LaterName {
            name: name.to_string(),
        });
    }
    faults
}

/// The names of the cutoff list, as the data file holds them.
pub fn later_names() -> impl Iterator<Item = &'static str> {
    data_lines(LATER_NAMES)
}

/// The words and phrases that break the voice of the story, as the data file holds them.
pub fn banned_words() -> impl Iterator<Item = &'static str> {
    data_lines(BANNED_WORDS)
}

/// The lines of a data file, without its comments and blank lines.
pub(crate) fn data_lines(file: &'static str) -> impl Iterator<Item = &'static str> {
    file.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// This many words in a row of a golden sample make a copy of it.
pub const COPIED_WORDS: usize = 8;

/// A plain text (see `plain_text`) that is in voice, and copies no golden sample. A later
/// name that `player_text` holds is allowed: the player wrote it first (GAMEPLAY.md 3.7).
#[must_use]
pub fn voice_text(
    text: &str,
    max_chars: usize,
    max_bytes: usize,
    player_text: &str,
) -> Option<String> {
    let line = one_line(text, max_chars, max_bytes)?;
    if !names_after_cutoff_except(&line, player_text).is_empty() {
        return None;
    }
    let own_words = !copies_a_sample(&line, &samples::every_sample());
    (in_voice(&line) && own_words).then_some(line)
}

/// True when the text holds `COPIED_WORDS` words in a row of one sample, in any case.
#[must_use]
pub fn copies_a_sample(text: &str, samples: &[&str]) -> bool {
    let words = words_of(text);
    samples
        .iter()
        .any(|sample| shares_a_phrase(&words, &words_of(sample)))
}

fn shares_a_phrase(words: &[String], sample: &[String]) -> bool {
    sample
        .windows(COPIED_WORDS)
        .any(|phrase| words.windows(COPIED_WORDS).any(|window| window == phrase))
}

/// True when a text of the narrator or of an NPC has no emoji and no banned word
/// (GAMEPLAY.md 3.2.1). Player text never gets this check: it is the player's own voice.
#[must_use]
pub fn in_voice(text: &str) -> bool {
    !text.chars().any(is_emoji) && banned_words_in(text).is_empty()
}

/// The banned words that the text holds as whole words, in any case.
#[must_use]
pub fn banned_words_in(text: &str) -> Vec<&'static str> {
    let words = words_of(text);
    banned_words()
        .filter(|banned| contains_phrase(&words, banned))
        .collect()
}

/// The blocks of pictographs and dingbats, and the joiners that build an emoji from them.
fn is_emoji(c: char) -> bool {
    matches!(
        c,
        '\u{2600}'..='\u{27BF}' | '\u{1F000}'..='\u{1FAFF}' | '\u{FE0F}' | '\u{200D}'
    )
}

/// The proper names of an answer that its prompt never names: a capital word that does
/// not start a sentence. The caller only logs them, because the test is rough.
#[must_use]
pub fn names_in_no_fact(answer: &str, prompt: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for sentence in answer.split(['.', '!', '?', ':', ';', '"']) {
        for word in words(sentence).skip(1) {
            let new = !names.iter().any(|name| name == word);
            if is_name(word) && new && !mentions(prompt, word) {
                names.push(word.to_string());
            }
        }
    }
    names
}

fn is_name(word: &str) -> bool {
    word.chars().next().is_some_and(char::is_uppercase) && word.chars().count() > 1
}

/// The text on one line, or None when it is empty, longer than `max_chars` or `max_bytes`,
/// holds a control character, or names something from after the cutoff. For the short
/// texts of a model. The bridge limits bytes, and a character outside ASCII takes up to 4.
#[must_use]
pub fn plain_text(text: &str, max_chars: usize, max_bytes: usize) -> Option<String> {
    let line = one_line(text, max_chars, max_bytes)?;
    names_after_cutoff(&line).is_empty().then_some(line)
}

/// The text on one line, or None when it is empty, longer than `max_chars` or `max_bytes`,
/// or holds a control character. The words are free: the player's own text gets only this.
#[must_use]
pub fn one_line(text: &str, max_chars: usize, max_bytes: usize) -> Option<String> {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let too_long = line.chars().count() > max_chars || line.len() > max_bytes;
    if line.is_empty() || too_long || line.chars().any(char::is_control) {
        return None;
    }
    Some(line)
}

/// Models often wrap JSON in a code fence or a sentence, so the object is the text from
/// the first `{` to the last `}`.
#[must_use]
pub fn json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (start < end).then(|| &text[start..=end])
}

/// The names of the cutoff list that the text holds, in any case. The last word of a name
/// also counts at the start of a longer word, so "Pandarian" counts as "Pandaria".
#[must_use]
pub fn names_after_cutoff(answer: &str) -> Vec<&'static str> {
    let words = words_of(answer);
    later_names()
        .filter(|name| starts_a_phrase(&words, name))
        .collect()
}

/// The names of the cutoff list that the answer holds and `player_text` does not.
#[must_use]
pub fn names_after_cutoff_except(answer: &str, player_text: &str) -> Vec<&'static str> {
    let allowed = names_after_cutoff(player_text);
    names_after_cutoff(answer)
        .into_iter()
        .filter(|name| !allowed.contains(name))
        .collect()
}

fn starts_a_phrase(words: &[String], name: &str) -> bool {
    let phrase = words_of(name);
    let Some((last, first)) = phrase.split_last() else {
        return false;
    };
    words.windows(phrase.len()).any(|window| {
        window[..first.len()] == *first && window[first.len()].starts_with(last.as_str())
    })
}

/// Do the two texts hold the same words, in any case and with any marks between them?
#[must_use]
pub fn same_words(a: &str, b: &str) -> bool {
    words_of(a) == words_of(b)
}

/// Does the text hold the name as whole words, in any case?
#[must_use]
pub fn mentions(text: &str, name: &str) -> bool {
    contains_phrase(&words_of(text), name)
}

fn contains_phrase(words: &[String], name: &str) -> bool {
    let phrase = words_of(name);
    !phrase.is_empty()
        && words
            .windows(phrase.len())
            .any(|window| window == phrase.as_slice())
}

fn words_of(text: &str) -> Vec<String> {
    words(text).map(str::to_lowercase).collect()
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

fn admits_no_source(answer: &str) -> bool {
    let answer = answer.to_lowercase();
    NO_SOURCE.iter().any(|marker| answer.contains(marker))
}

/// The numbers of each `[n]` and `[n, m]` in the text, in order. Brackets with anything
/// else inside are not citations.
fn citations(text: &str) -> Vec<usize> {
    let mut numbers = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find(']') else {
            break;
        };
        numbers.extend(citation(&rest[..close]).unwrap_or_default());
        rest = &rest[close + 1..];
    }
    numbers
}

fn citation(inside: &str) -> Option<Vec<usize>> {
    inside
        .split(',')
        .map(|part| part.trim().parse().ok())
        .collect()
}

/// The citations serve the check. The player reads the answer without them. A removal can
/// join two halves into a new citation, so it repeats until none is left.
#[must_use]
pub fn without_citations(text: &str) -> String {
    let mut text = text.to_string();
    loop {
        let removed = without_citations_once(&text);
        if removed == text {
            return text;
        }
        text = removed;
    }
}

fn without_citations_once(text: &str) -> String {
    let mut kept = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let close = rest[open..].find(']').map(|close| open + close);
        let Some(close) = close.filter(|&close| citation(&rest[open + 1..close]).is_some()) else {
            kept.push_str(&rest[..=open]);
            rest = &rest[open + 1..];
            continue;
        };
        kept.push_str(rest[..open].trim_end());
        rest = &rest[close + 1..];
    }
    kept.push_str(rest);
    kept.trim().to_string()
}
