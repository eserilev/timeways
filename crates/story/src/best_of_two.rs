//! The drafts of one chapter saga, until one is final: the best of two (GAMEPLAY.md 3.3).
//! A round lives in memory only. A restart drops it, and the chapter starts again.

use crate::chronicle::{self, Pick, Saga};
use crate::store::CharacterKey;
use hourglass::Tick;

/// The chapter that began at `began`, for this character only, and its drafts so far.
#[derive(Debug)]
pub struct Round {
    pub key: CharacterKey,
    pub began: Tick,
    /// The kinds of the small moments of the prompt, in their order.
    pub kinds: Vec<String>,
    number: usize,
    facts: String,
    second_prompt: String,
    answered: usize,
    passed: Vec<Saga>,
}

/// What the round needs after its last answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    /// A call with this prompt.
    Call(String),
    /// The saga for the chapter, or None when no draft passed the checks.
    Final(Option<Saga>),
}

impl Round {
    /// The round of a chapter whose first draft is out.
    #[must_use]
    pub fn new(
        key: CharacterKey,
        began: Tick,
        kinds: Vec<String>,
        number: usize,
        facts: String,
        second_prompt: String,
    ) -> Round {
        Round {
            key,
            began,
            kinds,
            number,
            facts,
            second_prompt,
            answered: 0,
            passed: Vec::new(),
        }
    }

    /// `saga` is None for a draft that failed or broke a rule.
    pub fn add_draft(&mut self, saga: Option<Saga>) {
        self.answered += 1;
        self.passed.extend(saga);
    }

    /// True when the next answer is the pick of the judge, and not a draft.
    #[must_use]
    pub fn is_judged(&self) -> bool {
        self.answered == 2
    }

    /// `tight` stops each extra call, so the first passing draft wins.
    #[must_use]
    pub fn next(&self, tight: bool) -> Next {
        let first = self.passed.first().cloned();
        if tight {
            return Next::Final(first);
        }
        if self.answered < 2 {
            return Next::Call(self.second_prompt.clone());
        }
        match self.passed.as_slice() {
            [one, two] => Next::Call(chronicle::judge_prompt(
                self.number,
                &self.facts,
                &one.text,
                &two.text,
            )),
            _ => Next::Final(first),
        }
    }

    /// The facts of the chapter, as its prompts show them.
    #[must_use]
    pub fn facts(&self) -> &str {
        &self.facts
    }

    /// The draft that the judge picked.
    #[must_use]
    pub fn picked(&self, pick: Pick) -> Option<Saga> {
        let index = match pick {
            Pick::First => 0,
            Pick::Second => 1,
        };
        self.passed.get(index).cloned()
    }
}
