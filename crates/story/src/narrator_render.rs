//! The text of a built narrator line: the tokens of the rules with the values of the
//! moment (docs/plans/narrator-templates.md 3.7). The rules never see a string, so the
//! render has property tests and no proof.

use crate::narrator_templates::{MARKS, Slot, Templates};
use std::collections::HashMap;
use timeways_rules::narrator_shapes::Token;

/// The values of one line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Values {
    pub lore: String,
    /// The naming of the turn: `$N`, "the paladin", "the Bookworm". None on an unnamed turn.
    pub hero: Option<String>,
    pub slots: HashMap<Slot, String>,
}

/// The marks that end a sentence.
const ENDS: [&str; 3] = [".", "!", "?"];

/// The line of `tokens`, one space between words, none before a mark, and a capital at
/// the start of each sentence. A token with no value renders as nothing: the rules build
/// no such line (`every_slot_has_a_value`).
#[must_use]
pub fn render(templates: &Templates, tokens: &[Token], values: &Values) -> String {
    let mut line = String::new();
    let mut sentence_starts = true;
    for token in tokens {
        let piece = match token {
            Token::Word(id) => templates.word(*id).to_string(),
            Token::Lore => with_end(values.lore.trim()),
            Token::Hero => values.hero.clone().unwrap_or_default(),
            Token::Slot(id) => Slot::of_id(*id)
                .and_then(|slot| values.slots.get(&slot).cloned())
                .unwrap_or_default(),
        };
        if piece.is_empty() {
            continue;
        }
        if !line.is_empty() && !MARKS.contains(&piece.as_str()) {
            line.push(' ');
        }
        let piece = if sentence_starts {
            capitalized(&piece)
        } else {
            piece
        };
        sentence_starts = ENDS.iter().any(|end| piece.ends_with(end));
        line.push_str(&piece);
    }
    line
}

/// The lore with a mark at its end, so the deed starts a sentence of its own.
fn with_end(lore: &str) -> String {
    if lore.is_empty() || ENDS.iter().any(|end| lore.ends_with(end)) {
        return lore.to_string();
    }
    format!("{lore}.")
}

fn capitalized(piece: &str) -> String {
    let mut chars = piece.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => first.to_uppercase().chain(chars).collect(),
        _ => piece.to_string(),
    }
}

/// "once", "twice", "three times" to "ten times", then "11 times".
#[must_use]
pub fn count_words(count: i64) -> String {
    match count {
        1 => "once".to_string(),
        2 => "twice".to_string(),
        3..=10 => format!("{} times", number_word(count)),
        _ => format!("{count} times"),
    }
}

/// "two" to "ten", then digits.
#[must_use]
pub fn count_number(count: i64) -> String {
    match count {
        1..=10 => number_word(count).to_string(),
        _ => count.to_string(),
    }
}

/// "second" to "tenth", then "11th", "21st", "22nd", "101st", "111th".
#[must_use]
pub fn ordinal(count: i64) -> String {
    const WORDS: [&str; 10] = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
        "tenth",
    ];
    let word = usize::try_from(count)
        .ok()
        .and_then(|count| count.checked_sub(1))
        .and_then(|index| WORDS.get(index));
    if let Some(word) = word {
        return (*word).to_string();
    }
    let suffix = match (count % 100, count % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{count}{suffix}")
}

fn number_word(count: i64) -> &'static str {
    const WORDS: [&str; 10] = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    usize::try_from(count)
        .ok()
        .and_then(|count| count.checked_sub(1))
        .and_then(|index| WORDS.get(index))
        .copied()
        .unwrap_or("")
}

/// "a Gray Ram" or "an Ivory Raptor". A name of the list of the data takes "a" before
/// a vowel: "a Unicorn".
#[must_use]
pub fn with_article(templates: &Templates, name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|first| "AEIOUaeiou".contains(first));
    let says_a = templates
        .article_a
        .iter()
        .any(|start| name.starts_with(start.as_str()));
    let article = if vowel && !says_a { "an" } else { "a" };
    format!("{article} {name}")
}

/// A title of a quest or of the journal, in double quotes.
#[must_use]
pub fn quoted(title: &str) -> String {
    format!("\"{title}\"")
}
