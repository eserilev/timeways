//! The checks of a text of the narrator about the chronicle: the saga check of
//! docs/plans/chapters.md 6 and 10, for a tale and for the history of a zone. Each refusal
//! says why, so a retry can name the rule.

use crate::arrival::arrival_in;
use crate::check::{
    COPIED_WORDS, banned_words_in, copies_a_sample, has_emoji, names_after_cutoff_except, one_line,
    slop_in,
};
use crate::grounding::ungrounded_names;
use crate::house::NAME_MARK;
use crate::prose::prose_faults;
use crate::samples;

/// The hero's name, `$N`, comes at most this often.
pub const MAX_NAMES: usize = 2;

/// What a text may be, and what it may not copy.
pub struct Limits<'a> {
    pub max_chars: usize,
    pub max_bytes: usize,
    /// The facts of the prompt: a slop word that they hold, such as a name, stays allowed.
    pub told: &'a str,
    /// The hero in the player's own words: a name past the cutoff there stays allowed.
    pub player_text: &'a str,
    /// Texts that the answer must not copy 8 words of: earlier texts and the player's.
    pub not_copied: &'a [&'a str],
    /// What the prompt gave the model (`grounding::given_text`): each name of the text is
    /// a word of it.
    pub given: &'a str,
}

/// The text as the player reads it, or the reasons why it breaks a rule.
///
/// # Errors
///
/// Returns each rule that the text breaks, in plain words for a retry.
pub fn checked(text: &str, limits: &Limits<'_>) -> Result<String, Vec<String>> {
    let Some(line) = one_line(text, limits.max_chars, limits.max_bytes) else {
        return Err(vec![format!(
            "It must be one paragraph of at most {} characters.",
            limits.max_chars
        )]);
    };
    let mut faults = Vec::new();
    for name in names_after_cutoff_except(&line, limits.player_text) {
        faults.push(format!(
            "It names {name}, which comes after the time of the story."
        ));
    }
    let given = format!("{}\n{}", limits.given, limits.player_text);
    for name in ungrounded_names(&line, &given) {
        faults.push(format!(
            "It names {name}, which no fact or lore of the prompt holds."
        ));
    }
    if has_emoji(&line) || !banned_words_in(&line).is_empty() {
        faults.push("It holds an emoji or a word that the voice never uses.".to_string());
    }
    for slop in slop_in(&line, limits.told) {
        faults.push(format!("It says \"{slop}\", which the facts do not hold."));
    }
    if let Some(arrival) = arrival_in(&line, &[]) {
        faults.push(format!("It only tells that the hero came: \"{arrival}\"."));
    }
    faults.extend(prose_faults(&line, &[]).iter().map(ToString::to_string));
    if line.matches(NAME_MARK).count() > MAX_NAMES {
        faults.push(format!("It names the hero more than {MAX_NAMES} times."));
    }
    if line.to_lowercase().contains("our hero") {
        faults.push("It says \"our hero\".".to_string());
    }
    if copies_a_sample(&line, &samples::every_sample()) {
        faults.push("It copies a sample.".to_string());
    }
    if copies_a_sample(&line, limits.not_copied) {
        faults.push(format!(
            "It copies {COPIED_WORDS} words in a row of an earlier text."
        ));
    }
    if faults.is_empty() {
        Ok(line)
    } else {
        Err(faults)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits<'a>(not_copied: &'a [&'a str]) -> Limits<'a> {
        Limits {
            max_chars: 400,
            max_bytes: 1600,
            told: "",
            player_text: "",
            not_copied,
            given: "Westfall Deadmines VanCleef",
        }
    }

    #[test]
    fn a_plain_paragraph_passes() {
        let text = "The miners of the Deadmines remember who ended VanCleef.";

        assert_eq!(checked(text, &limits(&[])).as_deref(), Ok(text));
    }

    #[test]
    fn an_arrival_of_the_hero_is_refused_with_its_clause() {
        let refused = checked("$N came to Westfall.", &limits(&[]));

        assert_eq!(
            refused,
            Err(vec![
                "It only tells that the hero came: \"n came\".".to_string()
            ])
        );
    }

    #[test]
    fn a_copy_of_an_earlier_text_is_refused() {
        let earlier = "the farmers of westfall still speak of the night the mill burned";

        let refused = checked(
            "The farmers of Westfall still speak of the night the mill burned down.",
            &limits(&[earlier]),
        );

        assert!(refused.is_err());
    }
}
