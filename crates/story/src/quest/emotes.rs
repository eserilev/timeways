//! The emotes that an emote step can ask for, and the words that no slap step may name
//! (docs/plans/quest-variety.md 4.3 and 4.10).

use crate::check::{data_lines, words_of};

const QUEST_EMOTES: &str = include_str!("../../data/quest_emotes.txt");
const CRUEL_WORDS: &str = include_str!("../../data/cruel_words.txt");

/// The emote tokens of the list, in its order.
#[must_use]
pub fn quest_emotes() -> Vec<&'static str> {
    data_lines(QUEST_EMOTES).collect()
}

/// A word of the cruelty list in the name, as a whole word in any case.
#[must_use]
pub fn is_cruel_target(npc: &str) -> bool {
    let cruel: Vec<&str> = data_lines(CRUEL_WORDS).collect();
    words_of(npc)
        .iter()
        .any(|word| cruel.contains(&word.as_str()))
}
