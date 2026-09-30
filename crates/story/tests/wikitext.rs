//! The markup of a wiki page goes, and its words stay (GAMEPLAY.md 5.10).

use timeways_story::wikitext::{
    Section, book_content, listed_pages, plain, redirect_target, sections,
};

#[test]
fn a_link_keeps_its_label_or_its_page() {
    let text = "The [[Testvale Tower|tower]] stands in [[Testvale]].";

    assert_eq!(plain(text), "The tower stands in Testvale.");
}

#[test]
fn a_template_goes_with_its_nested_templates() {
    let text = "Before {{Quote|a {{Sic|b {{Deep}}}} c}}after.";

    assert_eq!(plain(text), "Before after.");
}

#[test]
fn a_reference_goes_with_its_words() {
    let text =
        "Stubbs lit the lamp.<ref name=\"a\">Test book, page 3</ref> Then<ref name=\"a\" /> slept.";

    assert_eq!(plain(text), "Stubbs lit the lamp. Then slept.");
}

#[test]
fn an_html_tag_goes_and_its_words_stay() {
    let text = "The <span class=\"x\">keeper</span> waited.<br/>";

    assert_eq!(plain(text), "The keeper waited.");
}

#[test]
fn bold_and_italic_marks_go_but_an_apostrophe_stays() {
    let text = "'''Keeper Stubbs''' kept ''the'' keeper's lamp.";

    assert_eq!(plain(text), "Keeper Stubbs kept the keeper's lamp.");
}

#[test]
fn a_picture_goes_with_its_caption() {
    let text =
        "Hills.[[File:Testvale.jpg|thumb|The [[Testvale]] hills.]] More hills.[[Category:Tests]]";

    assert_eq!(plain(text), "Hills. More hills.");
}

#[test]
fn an_external_link_keeps_its_label_and_other_brackets_stay() {
    let text = "See [https://example.test/page the test page] [sic].";

    assert_eq!(plain(text), "See the test page [sic].");
}

#[test]
fn a_comment_and_a_table_go() {
    let text = "One<!-- a hidden note --> two.\n{| class=\"wikitable\"\n| cell\n|}\nThree.";

    assert_eq!(plain(text), "One two.\n\nThree.");
}

#[test]
fn broken_markup_leaves_no_marks() {
    let text = "[[Open {{never closed '' ]] }} [[{{}}]] {}}{ '''";

    let cleaned = plain(text);

    for mark in ["[[", "]]", "{{", "}}", "''"] {
        assert!(!cleaned.contains(mark), "{mark} in {cleaned:?}");
    }
}

#[test]
fn a_heading_splits_the_sections_at_any_level() {
    let text = "Lead line.\n== History ==\nOld times.\n=== Early days===\nEarly.\n";

    let found = sections(text);

    assert_eq!(
        found,
        [
            Section {
                heading: None,
                body: "Lead line.\n"
            },
            Section {
                heading: Some("History"),
                body: "Old times.\n"
            },
            Section {
                heading: Some("Early days"),
                body: "Early.\n"
            },
        ]
    );
}

#[test]
fn a_line_with_uneven_marks_is_no_heading() {
    let text = "Lead.\n== Uneven ===\nStill the lead.\n";

    let found = sections(text);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].heading, None);
}

#[test]
fn a_redirect_gives_its_target_page() {
    assert_eq!(
        redirect_target("  #redirect [[Testvale_Tower#Top|the tower]]"),
        Some("Testvale Tower".to_string())
    );
    assert_eq!(redirect_target("Testvale is a [[place]]."), None);
}

/// MediaWiki makes the first letter of a title upper case, so `[[night elf]]` is the page
/// "Night elf".
#[test]
fn a_link_title_gets_an_upper_case_first_letter() {
    assert_eq!(
        redirect_target("#REDIRECT [[night elf]]"),
        Some("Night elf".to_string())
    );
    assert_eq!(listed_pages("* [[élite guard]]\n"), ["Élite guard"]);
}

#[test]
fn listed_pages_come_in_order_without_labels() {
    let text = "Intro [[Not listed]]\n* [[Book One (Test)|Book One]]\n** [[:Book Two]]\n*No link\n";

    assert_eq!(listed_pages(text), ["Book One (Test)", "Book Two"]);
}

#[test]
fn the_game_book_comes_before_the_site_copy() {
    let text = "{{Book|The Tower (site)|content=Site words.}}\n{{Book|The Tower|author=x|content=Game {{Sic}} words.}}";

    assert_eq!(book_content(text), Some("Game {{Sic}} words."));
}

#[test]
fn a_page_with_only_a_site_copy_gives_that_copy() {
    let text = "{{Book|The Tower (site)|content=Site words.}}";

    assert_eq!(book_content(text), Some("Site words."));
}

#[test]
fn a_page_without_a_book_gives_nothing() {
    assert_eq!(book_content("Just an article about [[Testvale]]."), None);
}

#[test]
fn a_named_argument_after_the_content_stays_out_of_the_book() {
    let text = "{{Book|The Tower|content=Words of {{Sic|x}} and [[Page|label]].|author=Stubbs}}";

    assert_eq!(
        book_content(text),
        Some("Words of {{Sic|x}} and [[Page|label]].")
    );
}

#[test]
fn a_book_call_can_break_its_lines() {
    let text = "{{Book\n|The Tower\n|content=Words.\n|author=Stubbs\n}}";

    assert_eq!(book_content(text), Some("Words."));
}

#[test]
fn a_template_whose_name_only_starts_with_book_is_no_book() {
    let text = "{{Bookshelf|content=Shelf words.}}{{Book|The Tower|content=Words.}}";

    assert_eq!(book_content(text), Some("Words."));
    assert_eq!(book_content("{{Booklist|content=Shelf words.}}"), None);
}
