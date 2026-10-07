//! Passages of the lore pack from a wiki dump (GAMEPLAY.md 5.10). Every page is invented.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod wiki_dump;

use timeways_story::pack::{Link, Passage};
use timeways_story::pack_sources::{
    Outcome, PageReport, Sources, SourcesError, from_dump, paragraphs, subject_of,
};
use wiki_dump::{DumpPage, article, long, write_dump};

const INDEX: &str = "Test History";

fn sources(extra: &str) -> Sources {
    let text = format!(
        "[books]\nindex = \"{INDEX}\"\nchapters = [\"Chapter I: Dawn\", \"Chapter II: Noon\"]\n\
         title_suffix = \" (Test History)\"\n{extra}"
    );
    Sources::parse(&text).unwrap()
}

fn index(chapters: &str) -> String {
    format!("Intro.\n==Test History==\n{chapters}\n==Trivia==\n* [[Not a book]]\n")
}

fn book(words: &str) -> String {
    format!("{{{{Book|A title|content=\n{}\n}}}}", long(words))
}

fn texts(passages: &[Passage]) -> Vec<&str> {
    passages
        .iter()
        .map(|passage| passage.text.as_str())
        .collect()
}

fn read(title: &str, passages: usize, later: usize, game: usize) -> PageReport {
    PageReport {
        title: title.to_string(),
        outcome: Outcome::Read {
            passages,
            later,
            game,
        },
    }
}

#[test]
fn the_books_of_the_chosen_chapters_become_common_passages() {
    let index = index(
        "===Chapter I: Dawn===\n* [[The Dawn Book (Test History)]]\n\
         ===Chapter II: Noon ===\n* [[The Noon Book]]\n===Chapter III: Dusk===\n* [[The Dusk Book]]",
    );
    let dawn = book("The dawn came to Testvale.");
    let noon = book("The noon came to Mockshire.");
    let dusk = book("The dusk is later lore.");
    let dump = write_dump(
        "sources-books",
        &[
            article(INDEX, &index),
            article("The Dawn Book (Test History)", &dawn),
            article("The Noon Book", &noon),
            article("The Dusk Book", &dusk),
        ],
    );

    let built = from_dump(&dump, &sources("")).unwrap();

    assert_eq!(
        texts(&built.passages),
        [
            long("The dawn came to Testvale."),
            long("The noon came to Mockshire.")
        ]
    );
    assert_eq!(built.passages[0].source, "the book \"The Dawn Book\"");
    assert_eq!(built.passages[0].links, [Link::Common]);
    assert_eq!(
        built.report,
        [
            read("The Dawn Book (Test History)", 1, 0, 0),
            read("The Noon Book", 1, 0, 0)
        ]
    );
}

#[test]
fn a_book_behind_a_redirect_is_read_once() {
    let index = index(
        "===Chapter I: Dawn===\n* [[Old Dawn]]\n* [[The Dawn Book]]\n===Chapter II: Noon===\n",
    );
    let dawn = book("The dawn came to Testvale.");
    let dump = write_dump(
        "sources-redirect",
        &[
            article(INDEX, &index),
            article("Old Dawn", "#REDIRECT [[The Dawn Book]]"),
            article("The Dawn Book", &dawn),
        ],
    );

    let built = from_dump(&dump, &sources("")).unwrap();

    assert_eq!(texts(&built.passages), [long("The dawn came to Testvale.")]);
    assert_eq!(built.passages[0].source, "the book \"The Dawn Book\"");
}

#[test]
fn a_site_copy_of_a_book_is_skipped() {
    let index = index("===Chapter I: Dawn===\n* [[The Dawn Book]]\n===Chapter II: Noon===\n");
    let text = format!(
        "{{{{Book|The Dawn Book (site)|content=\n{}\n}}}}\n{{{{Book|The Dawn Book|content=\n{}\n}}}}",
        long("The website tells it this way."),
        long("The game tells it this way.")
    );
    let dump = write_dump(
        "sources-site",
        &[article(INDEX, &index), article("The Dawn Book", &text)],
    );

    let built = from_dump(&dump, &sources("")).unwrap();

    assert_eq!(
        texts(&built.passages),
        [long("The game tells it this way.")]
    );
}

#[test]
fn the_markup_of_a_book_is_stripped() {
    let index = index("===Chapter I: Dawn===\n* [[The Dawn Book]]\n===Chapter II: Noon===\n");
    let text = "{{Book|The Dawn Book|content=\n'''The [[Testvale Keep|keep]]''' fell{{Sic}}.<ref>Test</ref>[[File:Keep.jpg|thumb|A keep]] The rest of this invented line only makes it long enough to count as prose.\n}}";
    let dump = write_dump(
        "sources-markup",
        &[article(INDEX, &index), article("The Dawn Book", text)],
    );

    let built = from_dump(&dump, &sources("")).unwrap();

    assert_eq!(texts(&built.passages), [long("The keep fell.")]);
}

#[test]
fn a_page_without_a_book_is_reported_and_skipped() {
    let index = index("===Chapter I: Dawn===\n* [[The Dawn Book]]\n===Chapter II: Noon===\n");
    let dump = write_dump(
        "sources-no-book",
        &[
            article(INDEX, &index),
            article("The Dawn Book", &long("Only an article.")),
        ],
    );

    let built = from_dump(&dump, &sources("")).unwrap();

    assert!(built.passages.is_empty());
    assert_eq!(built.report[0].outcome, Outcome::NoBook);
}

#[test]
fn a_missing_chapter_is_reported() {
    let index = index("===Chapter I: Dawn===\n");
    let dump = write_dump("sources-no-chapter", &[article(INDEX, &index)]);

    let built = from_dump(&dump, &sources("")).unwrap();

    assert_eq!(built.missing_chapters, ["Chapter II: Noon"]);
}

#[test]
fn a_dump_without_the_index_page_is_an_error() {
    let dump = write_dump("sources-no-index", &[article("Testvale", "Hills.")]);

    let result = from_dump(&dump, &sources(""));

    assert!(
        matches!(result, Err(SourcesError::NoIndex(_))),
        "{result:?}"
    );
}

const TESTVALE: &str = r#"
[[pages]]
title = "Testvale"
lead = true
sections = ["History"]
places = ["Testvale"]
npcs = ["Keeper Stubbs"]

[[pages]]
title = "Test Folk"
sections = ["Origins"]
common = true

[[pages]]
title = "Nowhere"
sections = ["History"]
common = true

[later]
terms = ["Mock Expansion", 'Tyr\b']
"#;

fn testvale_dump(name: &str) -> std::path::PathBuf {
    let testvale = format!(
        "{}\n==History==\n{}\n* {}\n{}\n===Later days===\n{}\n==Trivia==\n{}\n",
        long("The lead of Testvale."),
        long("Testvale was founded by testers."),
        long("A list line is no prose."),
        "Too short.",
        long("A subsection is a section of its own."),
        long("Trivia stays out."),
    );
    let folk = format!(
        "Lead stays out.\n==Origins==\n{}\n{}\n{}\n",
        long("The folk came in the Mock Expansion."),
        long("The folk met Tyr and left."),
        long("The folk met Tyrande and stayed."),
    );
    let index = index("===Chapter I: Dawn===\n===Chapter II: Noon===\n");
    write_dump(
        name,
        &[
            article(INDEX, &index),
            article("Testvale", &testvale),
            DumpPage {
                title: "Test Folk",
                namespace: 0,
                text: &folk,
            },
        ],
    )
}

#[test]
fn only_the_listed_sections_of_a_page_go_in() {
    let dump = testvale_dump("sources-sections");

    let built = from_dump(&dump, &sources(TESTVALE)).unwrap();

    let testvale: Vec<&str> = built
        .passages
        .iter()
        .filter(|passage| passage.source == "the wiki page \"Testvale\"")
        .map(|passage| passage.text.as_str())
        .collect();
    assert_eq!(
        testvale,
        [
            long("The lead of Testvale."),
            long("Testvale was founded by testers.")
        ]
    );
}

#[test]
fn a_paragraph_with_a_later_term_is_dropped() {
    let dump = testvale_dump("sources-later");

    let built = from_dump(&dump, &sources(TESTVALE)).unwrap();

    let folk: Vec<&str> = built
        .passages
        .iter()
        .filter(|passage| passage.source == "the wiki page \"Test Folk\"")
        .map(|passage| passage.text.as_str())
        .collect();
    assert_eq!(folk, [long("The folk met Tyrande and stayed.")]);
}

#[test]
fn the_report_counts_the_paragraphs_that_a_later_term_dropped() {
    let dump = testvale_dump("sources-later-count");

    let built = from_dump(&dump, &sources(TESTVALE)).unwrap();

    assert_eq!(built.report[1], read("Test Folk", 1, 2, 0));
}

#[test]
fn a_paragraph_that_talks_about_the_game_is_dropped_and_counted() {
    let page = format!(
        "{}\n{}\n{}\n",
        long("Testvale lies under the old tower."),
        long("Testvale is a quest hub for testers."),
        long("Testers of the Mock Expansion quest hub came later."),
    );
    let index = index("===Chapter I: Dawn===\n===Chapter II: Noon===\n");
    let dump = write_dump(
        "sources-game",
        &[article(INDEX, &index), article("Testvale", &page)],
    );
    let list = "[[pages]]\ntitle = \"Testvale\"\nlead = true\nsections = []\n\
                places = [\"Testvale\"]\n[later]\nterms = [\"Mock Expansion\"]\n\
                [game]\nterms = [\"quest hub\"]\n";

    let built = from_dump(&dump, &sources(list)).unwrap();

    assert_eq!(
        texts(&built.passages),
        [long("Testvale lies under the old tower.")]
    );
    assert_eq!(built.report, [read("Testvale", 1, 1, 1)]);
}

/// The lead of "Brackenwall Village" tells of "the Pandaria campaign", and no later term
/// held it. A name that a narrator line may never say is no lore of 25 ADP either.
#[test]
fn a_paragraph_with_a_name_past_the_cutoff_is_dropped() {
    let page = format!(
        "{}\n{}\n",
        long("Testvale lies under the old tower."),
        long("Throughout the Pandaria campaign, the folk of Testvale stayed home."),
    );
    let index = index("===Chapter I: Dawn===\n===Chapter II: Noon===\n");
    let dump = write_dump(
        "sources-cutoff-name",
        &[article(INDEX, &index), article("Testvale", &page)],
    );
    let list = "[[pages]]\ntitle = \"Testvale\"\nlead = true\nsections = []\n\
                places = [\"Testvale\"]\n";

    let built = from_dump(&dump, &sources(list)).unwrap();

    assert_eq!(
        texts(&built.passages),
        [long("Testvale lies under the old tower.")]
    );
    assert_eq!(built.report, [read("Testvale", 1, 1, 0)]);
}

/// A paragraph past the limit of the bridge becomes several passages. A later term in one
/// piece drops every piece, because the rest of the paragraph tells the same later story.
#[test]
fn a_later_term_drops_every_piece_of_a_long_paragraph() {
    let later_story = "The folk then sailed to a land that only later players reach. ".repeat(80);
    let page = format!(
        "{}\nIn the Mock Expansion the folk changed. {later_story}\n",
        long("Testvale lies under the old tower."),
    );
    let index = index("===Chapter I: Dawn===\n===Chapter II: Noon===\n");
    let dump = write_dump(
        "sources-later-pieces",
        &[article(INDEX, &index), article("Testvale", &page)],
    );
    let list = "[[pages]]\ntitle = \"Testvale\"\nlead = true\nsections = []\n\
                places = [\"Testvale\"]\n[later]\nterms = [\"Mock Expansion\"]\n";

    let built = from_dump(&dump, &sources(list)).unwrap();

    assert_eq!(
        texts(&built.passages),
        [long("Testvale lies under the old tower.")]
    );
    assert_eq!(built.report, [read("Testvale", 1, 1, 0)]);
}

#[test]
fn a_listed_subsection_goes_in_only_under_a_parent_that_goes_in() {
    let page = format!(
        "Lead.\n==History==\n{}\n===Early days===\n{}\n==Quotes==\n===Early days===\n{}\n",
        long("Testvale was founded by testers."),
        long("The first testers slept in tents."),
        long("A quote under Quotes stays out."),
    );
    let index = index("===Chapter I: Dawn===\n===Chapter II: Noon===\n");
    let dump = write_dump(
        "sources-nested",
        &[article(INDEX, &index), article("Testvale", &page)],
    );
    let list = "[[pages]]\ntitle = \"Testvale\"\nsections = [\"History\", \"Early days\"]\n\
                places = [\"Testvale\"]\n";

    let built = from_dump(&dump, &sources(list)).unwrap();

    assert_eq!(
        texts(&built.passages),
        [
            long("Testvale was founded by testers."),
            long("The first testers slept in tents.")
        ]
    );
}

#[test]
fn a_page_with_a_place_links_to_it_and_a_common_page_is_common() {
    let dump = testvale_dump("sources-links");

    let built = from_dump(&dump, &sources(TESTVALE)).unwrap();

    let first = &built.passages[0];
    let last = &built.passages[built.passages.len() - 1];
    assert_eq!(
        first.links,
        [
            Link::Place("Testvale".to_string()),
            Link::Npc("Keeper Stubbs".to_string())
        ]
    );
    assert_eq!(last.links, [Link::Common]);
}

#[test]
fn a_missing_page_is_reported_and_skipped() {
    let dump = testvale_dump("sources-missing");

    let built = from_dump(&dump, &sources(TESTVALE)).unwrap();

    assert_eq!(
        built.report,
        [
            read("Testvale", 2, 0, 0),
            read("Test Folk", 1, 2, 0),
            PageReport {
                title: "Nowhere".to_string(),
                outcome: Outcome::Missing
            }
        ]
    );
}

#[test]
fn the_same_dump_gives_the_same_passages() {
    let dump = testvale_dump("sources-same");

    let first = from_dump(&dump, &sources(TESTVALE)).unwrap();
    let second = from_dump(&dump, &sources(TESTVALE)).unwrap();

    assert_eq!(first.passages, second.passages);
}

#[test]
fn a_broken_later_term_is_an_error() {
    let dump = testvale_dump("sources-bad-term");

    let result = from_dump(&dump, &sources("[later]\nterms = [\"(open\"]\n"));

    assert!(
        matches!(result, Err(SourcesError::BadTerm(_))),
        "{result:?}"
    );
}

#[test]
fn the_bundled_sources_read_and_their_later_terms_compile() {
    let bundled = Sources::bundled().unwrap();
    let dump = write_dump("sources-bundled", &[article(&bundled.books.index, "")]);

    let built = from_dump(&dump, &bundled).unwrap();

    assert!(built.passages.is_empty());
    assert_eq!(built.report.len(), bundled.pages.len());
}

/// Its passages pass no spoiler limit, so the pack would refuse them at the end of a long
/// build.
#[test]
fn a_page_with_no_place_no_npc_and_not_common_is_refused_when_the_list_is_read() {
    let text = format!(
        "[books]\nindex = \"{INDEX}\"\nchapters = []\n\n\
         [[pages]]\ntitle = \"Testvale\"\nsections = []\n"
    );

    let result = Sources::parse(&text);

    assert!(
        matches!(&result, Err(SourcesError::Unlinked(title)) if title == "Testvale"),
        "{result:?}"
    );
}

#[test]
fn every_bundled_page_has_a_place_an_npc_or_is_common() {
    let bundled = Sources::bundled().unwrap();

    for page in &bundled.pages {
        let linked = !page.places.is_empty() || !page.npcs.is_empty() || page.common;
        assert!(linked, "{}", page.title);
    }
}

#[test]
fn short_lines_and_list_lines_are_no_paragraphs() {
    let text = format!(
        "Too short.\n{}\n* {}\n; {}\n",
        long("A   line   of   prose."),
        long("A list line."),
        long("A term of a list.")
    );

    assert_eq!(paragraphs(&text), [long("A line of prose.")]);
}

/// A wiki page quotes the description of a dungeon as an indented line.
#[test]
fn an_indented_quote_is_a_paragraph() {
    let text = format!("::{}\n: {}\n", long("A quote."), long("Another quote."));

    assert_eq!(
        paragraphs(&text),
        [long("A quote."), long("Another quote.")]
    );
}

#[test]
fn a_page_is_about_the_link_that_its_title_names() {
    let deadmines = [Link::Place("The Deadmines".to_string())];
    let keep = [Link::Place("Shadowfang Keep".to_string())];

    assert_eq!(subject_of("Deadmines", &deadmines), "The Deadmines");
    assert_eq!(
        subject_of("Shadowfang Keep (Classic)", &keep),
        "Shadowfang Keep"
    );
}

#[test]
fn a_page_whose_title_names_no_link_is_about_its_own_title() {
    let deadmines = [Link::Place("The Deadmines".to_string())];
    let durotar = [Link::Place("Durotar".to_string())];

    assert_eq!(subject_of("Mr. Smite", &deadmines), "Mr. Smite");
    assert_eq!(subject_of("Test Folk", &[Link::Common]), "Test Folk");
    assert_eq!(
        subject_of("Test Folk (Classic)", &[Link::Common]),
        "Test Folk"
    );
    assert_eq!(subject_of("Darkspear tribe", &durotar), "Darkspear tribe");
}

#[test]
fn the_passages_of_a_page_keep_the_subject_of_the_page() {
    let dump = testvale_dump("sources-about");

    let built = from_dump(&dump, &sources(TESTVALE)).unwrap();

    let first = &built.passages[0];
    let last = &built.passages[built.passages.len() - 1];
    assert_eq!(first.about.as_deref(), Some("Testvale"));
    assert_eq!(last.about.as_deref(), Some("Test Folk"));
}
