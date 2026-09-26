//! Random model answers and random player text. Every text that passes a check keeps its
//! limits, and the search never fails on syntax.

#![no_main]

#[path = "common.rs"]
#[expect(dead_code, reason = "this target checks no output lines")]
mod common;

use libfuzzer_sys::fuzz_target;
use timeways_story::check::{check, later_names, names_after_cutoff, plain_text};
use timeways_story::{bard, narrator, talk};

fn assert_plain(text: &str, max_chars: usize, max_bytes: usize) {
    assert!(
        text.chars().count() <= max_chars && text.len() <= max_bytes,
        "{text:?}"
    );
    assert!(!text.chars().any(char::is_control), "{text:?}");
    assert!(!text.is_empty() && text.trim() == text, "{text:?}");
    assert!(names_after_cutoff(text).is_empty(), "{text:?}");
}

fuzz_target!(|data: &[u8]| {
    let moments = data.first().map_or(0, |byte| usize::from(byte % 9));
    let text = String::from_utf8_lossy(data);

    if let Some(saga) = bard::checked_saga(&text, moments) {
        assert_plain(&saga.text, bard::MAX_CHAPTER_CHARS, bard::MAX_CHAPTER_BYTES);
        assert!(saga.footnotes.len() <= bard::MAX_FOOTNOTES);
        for (moment, footnote) in &saga.footnotes {
            assert!((1..=moments).contains(moment));
            assert_plain(footnote, bard::MAX_FOOTNOTE_CHARS, 1600);
        }
    }
    if let Some(answer) = talk::checked_answer(&text) {
        assert_plain(&answer.say, talk::MAX_SAY_CHARS, talk::MAX_SAY_BYTES);
        assert!((-talk::MAX_TRUST_CHANGE..=talk::MAX_TRUST_CHANGE).contains(&answer.trust_change));
    }
    if let Some(line) = narrator::checked_line(&text) {
        assert_plain(&line, narrator::MAX_LINE_CHARS, narrator::MAX_LINE_BYTES);
    }
    if let Some(line) = plain_text(&text, 50, 200) {
        assert_plain(&line, 50, 200);
    }
    let _ = check(&text, moments);
    let known: Vec<&str> = later_names().collect();
    assert!(
        names_after_cutoff(&text)
            .iter()
            .all(|name| known.contains(name))
    );
    common::pack().search(&text, 5).unwrap();
});
