//! The sentences of a narrator text, for the shape checks of the style guide
//! (docs/plans/narrator-style.md 3 and 10.1).

/// The marks that end a sentence.
const END_MARKS: [char; 3] = ['.', '!', '?'];

/// Marks that close a quote or an aside after the end mark: `finished "The Tome."`.
const CLOSERS: [char; 3] = ['"', '\'', ')'];

/// Words with a period that ends no sentence: "Mr. Smite" is one sentence.
const ABBREVIATIONS: [&str; 9] = ["mr", "mrs", "ms", "dr", "st", "jr", "sr", "lt", "sgt"];

/// The sentences of `text`, trimmed. A sentence ends at `.`, `!`, or `?` before a space or
/// the end of the text. A run of marks, such as "...", ends one sentence.
#[must_use]
pub fn sentences(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = 0;
    for (at, mark) in text.char_indices() {
        if at < start || !END_MARKS.contains(&mark) {
            continue;
        }
        let end = after_closers(text, at);
        let at_break = text[end..].chars().next().is_none_or(char::is_whitespace);
        if at_break && !is_abbreviation(&text[start..at], mark) {
            push_trimmed(&mut found, &text[start..end]);
            start = end;
        }
    }
    push_trimmed(&mut found, &text[start..]);
    found
}

/// The words of a sentence: runs between spaces that hold a letter or a digit. "$N" is
/// one word, and so is "Gath'Ilzogg".
#[must_use]
pub fn word_count(sentence: &str) -> usize {
    sentence
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count()
}

/// The byte after the end marks and the closers that start at `at`.
fn after_closers(text: &str, at: usize) -> usize {
    let closing = |c: &char| END_MARKS.contains(c) || CLOSERS.contains(c);
    let length: usize = text[at..]
        .chars()
        .take_while(closing)
        .map(char::len_utf8)
        .sum();
    at + length
}

fn is_abbreviation(before: &str, mark: char) -> bool {
    let last = before.rsplit(|c: char| !c.is_alphabetic()).next();
    mark == '.' && last.is_some_and(|word| ABBREVIATIONS.contains(&word.to_lowercase().as_str()))
}

fn push_trimmed<'a>(found: &mut Vec<&'a str>, sentence: &'a str) {
    let sentence = sentence.trim();
    if !sentence.is_empty() {
        found.push(sentence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_ends_at_a_mark_before_a_space() {
        assert_eq!(
            sentences("Hogger fell. The Riverpaw scattered! Who leads now?"),
            ["Hogger fell.", "The Riverpaw scattered!", "Who leads now?"]
        );
    }

    #[test]
    fn a_title_of_address_ends_no_sentence() {
        assert_eq!(
            sentences("Mr. Smite guards the ship. Dr. Weavil is gone."),
            ["Mr. Smite guards the ship.", "Dr. Weavil is gone."]
        );
    }

    #[test]
    fn a_closing_quote_stays_with_its_sentence() {
        assert_eq!(
            sentences("The dwarf finished \"The Tome.\" The order grows."),
            ["The dwarf finished \"The Tome.\"", "The order grows."]
        );
    }

    #[test]
    fn a_mark_inside_a_word_ends_no_sentence() {
        assert_eq!(
            sentences("It weighs 1.5 stone...and more"),
            ["It weighs 1.5 stone...and more"]
        );
        assert_eq!(sentences("Wait... Then go."), ["Wait...", "Then go."]);
    }

    #[test]
    fn a_word_is_a_run_with_a_letter_or_a_digit() {
        assert_eq!(word_count("Gath'Ilzogg fell to $N - at level 20."), 7);
        assert_eq!(word_count(""), 0);
    }
}
