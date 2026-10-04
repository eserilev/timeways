//! The size of a prompt in tokens, so that each call fits a small local model (GAMEPLAY.md
//! 3.2.1). The estimate is rough on purpose. A prompt with a part of open length keeps as
//! much of that part as fits its budget.

use crate::{check, chronicle, narrator, quest, talk};

/// The context of a small local model. The prompt and the longest reply share it.
pub const CONTEXT_TOKENS: usize = 2048;

/// The chat template of a local model wraps the prompt in a few tokens of its own.
const TEMPLATE_TOKENS: usize = 100;

/// A rule of thumb for English text.
const CHARS_PER_TOKEN: usize = 4;

/// The keys, quotes, and marks of a JSON reply around its words.
const JSON_CHARS: usize = 100;

/// One step of a quest in JSON, with a long place name.
const STEP_CHARS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    NarratorLine,
    Chapter,
    /// The pick between two drafts of a chapter.
    Judge,
    Talk,
    Quest,
    Lore,
}

impl Call {
    /// The tokens that the prompt of this call can take, with room for its longest reply.
    #[must_use]
    pub fn prompt_budget(self) -> usize {
        CONTEXT_TOKENS - TEMPLATE_TOKENS - self.reply_tokens()
    }

    /// The longest reply that the checks let through.
    #[must_use]
    pub fn reply_tokens(self) -> usize {
        let chars = match self {
            Call::NarratorLine => narrator::MAX_LINE_CHARS,
            Call::Chapter => {
                chronicle::MAX_CHAPTER_CHARS
                    + chronicle::MAX_FOOTNOTES * chronicle::MAX_FOOTNOTE_CHARS
                    + JSON_CHARS
            }
            Call::Judge => JSON_CHARS,
            Call::Talk => talk::MAX_SAY_CHARS + JSON_CHARS,
            Call::Quest => {
                quest::MAX_TITLE_CHARS
                    + quest::MAX_TEXT_CHARS
                    + quest::MAX_STEPS * STEP_CHARS
                    + JSON_CHARS
            }
            Call::Lore => check::MAX_CHARS,
        };
        chars.div_ceil(CHARS_PER_TOKEN)
    }
}

#[must_use]
pub fn estimated_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_TOKEN)
}

/// The prompt `build(count)` with the largest count up to `most` that fits `budget`. When
/// no count fits, the prompt of count 0.
#[must_use]
pub fn largest_fit(most: usize, budget: usize, build: impl Fn(usize) -> String) -> String {
    for count in (1..=most).rev() {
        let prompt = build(count);
        if estimated_tokens(&prompt) <= budget {
            return prompt;
        }
    }
    build(0)
}
