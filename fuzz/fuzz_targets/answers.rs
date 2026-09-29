//! Random model answers and random player text. Every text that passes a check keeps its
//! limits, and the search never fails on syntax.

#![no_main]

#[path = "common.rs"]
mod common;

use libfuzzer_sys::fuzz_target;
use timeways_story::check::{check, later_names, names_after_cutoff, plain_text, same_words};
use timeways_story::quest::{self, Known, Step};
use timeways_story::seen::{SeenText, TextKind};
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

fn known(seen: &[SeenText]) -> Known<'_> {
    Known {
        giver: "Keeper Tessa",
        zones: vec!["Testvale"],
        subzones: vec!["Old Tower"],
        npcs: vec!["Keeper Tessa", "Farmer Bram"],
        seen,
    }
}

fn assert_quest(text: &str) {
    let seen = [SeenText {
        kind: TextKind::Quest,
        title: Some("Rats".to_string()),
        npc: None,
        zone: None,
        text: "Miller Oda needs help.".to_string(),
    }];
    let known = known(&seen);
    let Ok(offer) = quest::checked_quest(text, &known) else {
        return;
    };
    assert_plain(&offer.title, quest::MAX_TITLE_CHARS, quest::MAX_OFFER_BYTES);
    assert_plain(&offer.text, quest::MAX_TEXT_CHARS, quest::MAX_OFFER_BYTES);
    assert!((1..=quest::MAX_STEPS).contains(&offer.steps.len()));
    assert!(
        !same_words(&offer.title, "Rats"),
        "the title of a game quest: {}",
        offer.title
    );
    assert!(quest::offer_line(known.giver, &offer).len() <= quest::MAX_OFFER_BYTES);
    for step in &offer.steps {
        match step {
            Step::Visit { place } => assert!(["Testvale", "Old Tower"].contains(&place.as_str())),
            Step::Meet { npc } => assert_eq!(npc, "Farmer Bram"),
        }
    }
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
    assert_quest(&text);
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
