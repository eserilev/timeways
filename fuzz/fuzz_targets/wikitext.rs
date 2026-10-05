//! Random bytes as a wiki dump and as the wikitext of a page. No input panics, the plain
//! text holds no markup marks, each paragraph fits the bridge, and each section is a part
//! of its page (GAMEPLAY.md 5.10).

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;
use timeways_story::dump::{xml_scan, xml_texts};
use timeways_story::pack_sources::paragraphs;
use timeways_story::passage_limits::MAX_PASSAGE_BYTES;
use timeways_story::wikitext::{book_content, listed_pages, plain, redirect_target, sections};

const MARKS: [&str; 5] = ["[[", "]]", "{{", "}}", "''"];

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
        }
    }
    if let Some(content) = book_content(text) {
        assert!(text.contains(content));
    }
    let _ = redirect_target(text);
    let _ = listed_pages(text);
});
