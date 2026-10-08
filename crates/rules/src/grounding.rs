//! No invented names (GAMEPLAY.md 3.2.1): every proper name of a model text is a name of
//! what the model was given. The story program finds the names, and gives each one an id,
//! so the rule reads ids, never strings. Lean proves its laws
//! (lean/Timeways/Grounding.lean).
//!
//! The functions walk by index: Aeneas translates no iterator adapter (lean/README.md).

use crate::thin_lore::holds;

/// True when each id of `answer` is in `given`. `answer` holds the ids of the names of the
/// model text, and `given` the ids of every word that the prompt gave. An index loop.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn grounded(answer: &[u32], given: &[u32]) -> bool {
    let mut index = 0;
    while index < answer.len() {
        if !holds(given, answer[index]) {
            return false;
        }
        index += 1;
    }
    true
}

/// True when the prompt gave the name. The story program names each ungrounded name of a
/// refused text with it.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn is_given(name: u32, given: &[u32]) -> bool {
    holds(given, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_whose_names_are_all_given_is_grounded() {
        assert!(grounded(&[2, 1], &[1, 2, 3]));
    }

    #[test]
    fn a_text_with_one_name_that_was_not_given_is_not_grounded() {
        assert!(!grounded(&[1, 4], &[1, 2, 3]));
    }

    #[test]
    fn a_text_with_no_names_is_grounded() {
        assert!(grounded(&[], &[]));
    }

    #[test]
    fn a_name_of_an_empty_prompt_is_never_given() {
        assert!(!is_given(1, &[]));
    }
}
