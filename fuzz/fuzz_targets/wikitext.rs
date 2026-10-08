//! Random bytes as a wiki dump and as the wikitext of a page. No input panics, the plain
//! text holds no markup marks, each paragraph fits the bridge, each section is a part
//! of its page, and the cut of game talk leaves no game sentence (GAMEPLAY.md 5.10). The
//! cites of a line and the infobox of a page never panic, a cited title is never empty,
//! and the tag of an outcome passage is a foe or a quest of its cites, or unresolved. The
//! tag of a setup passage is a foe or a quest of its cites in the one instance, and its
//! window is a part of the paragraph that tells no end.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;
use timeways_story::dump::{xml_scan, xml_texts};
use timeways_story::game_talk::{Cut, cut_game_talk};
use timeways_story::outcome_passages::{PageKind, Paragraph, dependencies, may_tell_an_end, page_kind};
use timeways_story::pack::{Deed, Dependency, Link};
use timeways_story::setup_passages::{Found, Instances, setup_for, tells_an_end, window};
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
    setup_of(text);
});

/// Each line is a paragraph of the page "Test Page (2)" in the instance "Testvault", and
/// the whole text is the page of every cited title.
fn setup_of(text: &str) {
    let kind = page_kind("Test Page (2)", text);
    let names: Vec<String> = match &kind {
        PageKind::Npc(npc) => vec![npc.name.clone()],
        PageKind::Quest(quest) => vec![quest.name.clone()],
        PageKind::Other => Vec::new(),
    };
    let instances = Instances {
        names: vec!["Testvault".to_string()],
        bosses: Default::default(),
    };
    let links = [Link::Place("Testvault".to_string())];
    for line in text.lines() {
        let cited = cites(line);
        let paragraph = plain(line);
        let found = Found {
            text: &paragraph,
            cites: &cited,
            page: "Test Page (2)",
            links: &links,
        };
        let Some(setup) = setup_for(&found, &instances, |_| Some(kind.clone())) else {
            continue;
        };
        assert_eq!(setup.instance, "Testvault");
        let (Deed::Foe(name) | Deed::Quest(name)) = &setup.deed;
        assert!(names.contains(name), "{name} is no cited page");
        let shown = window(&paragraph, &setup.deed).expect("a setup has a window");
        assert!(paragraph.contains(shown));
        assert!(!tells_an_end(shown), "{shown:?}");
    }
}

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
        if !may_tell_an_end(&paragraph) {
            continue;
        }
        let names: Vec<String> = match &kind {
            PageKind::Npc(npc) => vec![npc.name.clone()],
            PageKind::Quest(quest) => vec![quest.name.clone()],
            PageKind::Other => Vec::new(),
        };
        let found = Paragraph {
            text: &paragraph,
            cites: &cited,
            page: "Test Page (2)",
            bosses: &[],
            foes: &[],
        };
        let tags = dependencies(&found, |_| Some(kind.clone()));
        let mut seen = BTreeSet::new();
        for tag in &tags {
            assert!(seen.insert(format!("{tag:?}")), "{tag:?} twice in {tags:?}");
            match tag {
                Dependency::Foe(name) | Dependency::Quest(name) => {
                    assert!(names.contains(name), "{name} is no cited page");
                }
                Dependency::Unresolved => assert_eq!(tags.len(), 1, "{tags:?}"),
            }
        }
    }
}
