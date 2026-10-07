//! The limits of the bridge for one passage (GAMEPLAY.md 5.5 and 5.10).

#![allow(clippy::unwrap_used)]

use timeways_story::pack::{Link, Origin, Passage};
use timeways_story::passage_limits::{
    LimitFault, MAX_PASSAGE_BYTES, MAX_SOURCE_BYTES, fault, fitted, pieces,
};

fn passage(text: &str, source: &str) -> Passage {
    Passage {
        text: text.to_string(),
        source: source.to_string(),
        links: vec![Link::Common],
        origin: Origin::Pack,
        about: None,
        depends_on: None,
    }
}

#[test]
fn a_text_of_exactly_the_limit_stays_one_piece() {
    let text = "a".repeat(MAX_PASSAGE_BYTES);

    assert_eq!(pieces(&text), [text]);
}

#[test]
fn a_text_one_byte_over_the_limit_is_cut_after_its_last_sentence() {
    let first = format!("{}.", "a".repeat(3000));
    let text = format!("{first} {}", "b".repeat(MAX_PASSAGE_BYTES - first.len()));

    assert_eq!(text.len(), MAX_PASSAGE_BYTES + 1);
    assert_eq!(pieces(&text), [first, "b".repeat(MAX_PASSAGE_BYTES - 3001)]);
}

#[test]
fn a_long_text_with_no_sentence_end_is_cut_at_a_space() {
    let text = format!("{} {}", "a".repeat(4000), "b".repeat(200));

    assert_eq!(pieces(&text), ["a".repeat(4000), "b".repeat(200)]);
}

#[test]
fn a_long_word_is_cut_between_two_characters() {
    let text = "é".repeat(MAX_PASSAGE_BYTES);

    let cut = pieces(&text);

    assert!(cut.iter().all(|piece| piece.len() <= MAX_PASSAGE_BYTES));
    assert_eq!(cut.concat(), text);
}

#[test]
fn a_dot_inside_a_word_is_no_end_of_a_sentence() {
    let text = format!("{}.b {}", "a".repeat(4090), "c".repeat(10));

    assert_eq!(
        pieces(&text),
        [format!("{}.b", "a".repeat(4090)), "c".repeat(10)]
    );
}

#[test]
fn a_source_over_its_limit_or_with_a_control_character_breaks_a_limit() {
    let long = "s".repeat(MAX_SOURCE_BYTES + 1);

    assert_eq!(fault(&passage("text", &"s".repeat(MAX_SOURCE_BYTES))), None);
    assert_eq!(
        fault(&passage("text", &long)),
        Some(LimitFault::LongSource(MAX_SOURCE_BYTES + 1))
    );
    assert_eq!(
        fault(&passage("text", "a\nb")),
        Some(LimitFault::ControlInSource)
    );
}

#[test]
fn a_text_keeps_line_breaks_and_tabs_but_no_other_control_character() {
    assert_eq!(fault(&passage("a\nb\tc", "s")), None);
    assert_eq!(
        fault(&passage("a\rb", "s")),
        Some(LimitFault::ControlInText)
    );
}

#[test]
fn a_fitted_passage_keeps_the_first_piece_of_a_long_text() {
    let text = format!("{}. {}", "a".repeat(3000), "b".repeat(3000));

    let fitted = fitted(passage(&text, "s")).unwrap();

    assert_eq!(fitted.text, format!("{}.", "a".repeat(3000)));
}

#[test]
fn a_passage_with_a_bad_source_is_dropped() {
    assert_eq!(fitted(passage("text", "a\u{0}b")), None);
}
