//! Random bytes as a wiki dump and as the wikitext of a page. No input panics, the plain
//! text holds no markup marks, each paragraph fits the bridge, each section is a part
//! of its page, and the cut of game talk leaves no game sentence (GAMEPLAY.md 5.10). The
//! cites of a line and the infobox of a page never panic, a cited title is never empty,
//! and the tag of an outcome passage is a foe or a quest of its cites, or unresolved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;
use timeways_story::dump::{xml_scan, xml_texts};
use timeways_story::game_talk::{Cut, cut_game_talk};
use timeways_story::outcome_passages::{PageKind, dependency, is_outcome, page_kind};
use timeways_story::pack::Dependency;
use timeways_story::pack_sources::paragraphs;
use timeways_story::passage_limits::MAX_PASSAGE_BYTES;
use timeways_story::wikitext::{
    book_content, cites, listed_pages, plain, redirect_target, sections,
};

const MARKS: [&str; 5] = ["[[", "]]", "{{", "}}", "''"];

/// A sentence with the word "game" counts as game talk.
fn says_game(sentence: &str) -> bool {
    sentence.contains("game")
}

fuzz_target!(|data: &[u8]| {
    let wanted: BTreeSet<String> = ["A".to_string(), "B".to_string()].into();
    let _ = xml_texts(data, &wanted);
    let _ = xml_scan(data, &wanted);

    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let cleaned = plain(text);
    for mark in MARKS {
        assert!(!cleaned.contains(mark), "{mark} in {cleaned:?}");
    }
    for section in sections(text) {
        assert!(text.contains(section.body));
        assert_eq!(section.heading.is_none(), section.level == 0);
        for paragraph in paragraphs(&plain(section.body)) {
            assert!(paragraph.len() <= MAX_PASSAGE_BYTES, "{}", paragraph.len());
            if let Cut::Trimmed { text, .. } = cut_game_talk(&paragraph, says_game) {
                assert!(!text.contains("game"), "{text:?}");
            }
        }
    }
    if let Some(content) = book_content(text) {
        assert!(text.contains(content));
    }
    let _ = redirect_target(text);
    let _ = listed_pages(text);
    outcome_of(text);
});

/// Each line is a paragraph, and the whole text is the page of every cited title.
fn outcome_of(text: &str) {
    let kind = page_kind("Test Page (2)", text);
    for line in text.lines() {
        let cited = cites(line);
        assert!(
            cited
                .links
                .iter()
                .chain(&cited.refs)
                .all(|title| !title.is_empty())
        );
        let paragraph = plain(line);
        if !is_outcome(&paragraph) {
            continue;
        }
        let names: Vec<String> = match &kind {
            PageKind::Npc(npc) => vec![npc.name.clone()],
            PageKind::Quest(quest) => vec![quest.name.clone()],
            PageKind::Other => Vec::new(),
        };
        match dependency(&paragraph, &cited, "Test Page (2)", |_| Some(kind.clone())) {
            Dependency::Foe(name) | Dependency::Quest(name) => {
                assert!(names.contains(&name), "{name} is no cited page");
            }
            Dependency::Unresolved => {}
        }
    }
}
