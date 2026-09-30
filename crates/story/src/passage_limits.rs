//! The limits of the bridge for one passage of a lore answer (Gnomish Relay
//! `story_lines.rs`). The bridge refuses a reply with a passage past them. The batch then
//! gets no answer, and the bridge stops the story program as hung.

use crate::pack::Passage;
use thiserror::Error;

pub const MAX_PASSAGE_BYTES: usize = 4096;
pub const MAX_SOURCE_BYTES: usize = 512;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LimitFault {
    #[error("the text has {0} bytes, and the bridge takes at most {MAX_PASSAGE_BYTES}")]
    LongText(usize),
    #[error("the text holds a control character other than a line break or a tab")]
    ControlInText,
    #[error("the source has {0} bytes, and the bridge takes at most {MAX_SOURCE_BYTES}")]
    LongSource(usize),
    #[error("the source holds a control character")]
    ControlInSource,
}

/// The first limit that the passage breaks.
#[must_use]
pub fn fault(passage: &Passage) -> Option<LimitFault> {
    source_fault(&passage.source).or_else(|| text_fault(&passage.text))
}

fn source_fault(source: &str) -> Option<LimitFault> {
    if source.len() > MAX_SOURCE_BYTES {
        return Some(LimitFault::LongSource(source.len()));
    }
    source
        .chars()
        .any(char::is_control)
        .then_some(LimitFault::ControlInSource)
}

fn text_fault(text: &str) -> Option<LimitFault> {
    if text.len() > MAX_PASSAGE_BYTES {
        return Some(LimitFault::LongText(text.len()));
    }
    let control = text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t');
    control.then_some(LimitFault::ControlInText)
}

/// The passage as the bridge takes it: a long text keeps only its first piece. None when
/// the source or a control character breaks a limit.
#[must_use]
pub fn fitted(mut passage: Passage) -> Option<Passage> {
    if let Some(first) = pieces(&passage.text).into_iter().next() {
        passage.text = first;
    }
    fault(&passage).is_none().then_some(passage)
}

/// The text in pieces of at most `MAX_PASSAGE_BYTES`. A cut goes after the end of a sentence,
/// else at a space, else between two characters.
#[must_use]
pub fn pieces(text: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut rest = text.trim();
    while rest.len() > MAX_PASSAGE_BYTES {
        let cut = cut_point(rest);
        pieces.push(rest[..cut].trim_end().to_string());
        rest = rest[cut..].trim_start();
    }
    if !rest.is_empty() {
        pieces.push(rest.to_string());
    }
    pieces
}

/// Above zero, because `text` starts with no space.
fn cut_point(text: &str) -> usize {
    let head = &text[..char_floor(text, MAX_PASSAGE_BYTES)];
    sentence_end(head)
        .or_else(|| head.rfind(char::is_whitespace).filter(|at| *at > 0))
        .unwrap_or(head.len())
}

fn char_floor(text: &str, at: usize) -> usize {
    (0..=at)
        .rev()
        .find(|at| text.is_char_boundary(*at))
        .unwrap_or(0)
}

/// The byte after the last `.`, `!`, or `?` that a space follows.
fn sentence_end(head: &str) -> Option<usize> {
    head.rmatch_indices(['.', '!', '?'])
        .map(|(at, mark)| at + mark.len())
        .find(|end| head[*end..].starts_with(char::is_whitespace))
}
