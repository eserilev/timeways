//! The cut of game talk from a paragraph of a wiki page (GAMEPLAY.md 5.10). A game term
//! drops only its own sentence, when the rest still reads on its own.

use crate::sentences::sentences;

/// A cut paragraph keeps at least this many sentences.
const MIN_KEPT_SENTENCES: usize = 2;

/// Words that point back to the sentence before them. A sentence that opens with one
/// cannot follow a cut sentence: "He leads them now." needs the sentence that named him.
const BACK_WORDS: [&str; 46] = [
    "this",
    "these",
    "that",
    "those",
    "it",
    "its",
    "they",
    "their",
    "them",
    "he",
    "his",
    "him",
    "she",
    "her",
    "here",
    "there",
    "however",
    "but",
    "and",
    "or",
    "also",
    "instead",
    "then",
    "thus",
    "therefore",
    "hence",
    "so",
    "afterward",
    "afterwards",
    "later",
    "such",
    "both",
    "meanwhile",
    "still",
    "yet",
    "nevertheless",
    "additionally",
    "furthermore",
    "moreover",
    "again",
    "similarly",
    "likewise",
    "otherwise",
    "consequently",
    "eventually",
    "finally",
];

/// Openings of more than one word that point back.
const BACK_PHRASES: [&str; 9] = [
    "as a result",
    "in the meantime",
    "in addition",
    "in turn",
    "the latter",
    "the former",
    "after this",
    "after that",
    "despite this",
];

/// Words that point back from early in a sentence: "Recently, however, the Horde..." or
/// "One such sentence...".
const EARLY_BACK_WORDS: [&str; 4] = ["however", "though", "instead", "such"];

/// How early an early back word stands: within the first words of the sentence.
const EARLY_WORDS: usize = 3;

/// What the cut leaves of a paragraph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cut {
    /// No sentence talks about the game.
    Whole,
    /// The paragraph without its game sentences, and the number of sentences that went.
    Trimmed { text: String, dropped: usize },
    /// Too little is left, or a kept sentence points back to a cut one.
    Dropped,
}

/// The paragraph without each sentence for which `is_game` holds.
#[must_use]
pub fn cut_game_talk(paragraph: &str, is_game: impl Fn(&str) -> bool) -> Cut {
    let all = sentences(paragraph);
    let mut kept = Vec::new();
    let mut after_cut = false;
    for sentence in &all {
        if is_game(sentence) {
            after_cut = true;
            continue;
        }
        if after_cut && points_back(sentence) {
            return Cut::Dropped;
        }
        kept.push(*sentence);
        after_cut = false;
    }
    let dropped = all.len() - kept.len();
    if dropped == 0 {
        return Cut::Whole;
    }
    if kept.len() < MIN_KEPT_SENTENCES {
        return Cut::Dropped;
    }
    Cut::Trimmed {
        text: kept.join(" "),
        dropped,
    }
}

fn points_back(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let opens_with = |phrase: &str| {
        let phrase: Vec<&str> = phrase.split(' ').collect();
        words.starts_with(&phrase)
    };
    let early = words.iter().take(EARLY_WORDS);
    BACK_WORDS
        .iter()
        .chain(&BACK_PHRASES)
        .any(|start| opens_with(start))
        || early
            .into_iter()
            .any(|word| EARLY_BACK_WORDS.contains(word))
}
