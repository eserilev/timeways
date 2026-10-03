//! One `/lore` answer from a model: a prompt, a check, one retry, and a fallback to the
//! passages alone (GAMEPLAY.md 3.1, 5.6, and 5.9). No I/O: the caller carries each prompt
//! to the model, and each answer back.

use crate::check::{check, without_citations};
use crate::pack::Passage;
use crate::prompt::{self, Context};
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Answer {
    /// The words of the model, without its citations. None shows the passages alone.
    pub text: Option<String>,
    pub passages: Vec<Passage>,
}

#[derive(Debug)]
pub struct LoreCall {
    prompt: String,
    passages: Vec<Passage>,
    attempt: Attempt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attempt {
    First,
    Retry,
}

#[derive(Debug)]
pub enum Next {
    Ask(LoreCall),
    Done(Answer),
}

impl LoreCall {
    #[must_use]
    pub fn new(question: &str, context: &Context<'_>, passages: Vec<Passage>) -> LoreCall {
        let prompt = prompt::lore(question, context, &passages);
        LoreCall {
            prompt,
            passages,
            attempt: Attempt::First,
        }
    }

    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    #[must_use]
    pub fn passages(&self) -> &[Passage] {
        &self.passages
    }

    /// A second bad answer is dropped, and the passages show alone.
    #[must_use]
    pub fn answered(self, text: &str) -> Next {
        let faults = check(text, self.passages.len());
        if faults.is_empty() {
            let text = Some(without_citations(text.trim()));
            return Next::Done(Answer {
                text,
                passages: self.passages,
            });
        }
        match self.attempt {
            Attempt::First => Next::Ask(LoreCall {
                prompt: prompt::retry(&self.prompt, text, &faults),
                passages: self.passages,
                attempt: Attempt::Retry,
            }),
            Attempt::Retry => Next::Done(self.failed()),
        }
    }

    #[must_use]
    pub fn failed(self) -> Answer {
        Answer {
            text: None,
            passages: self.passages,
        }
    }
}
