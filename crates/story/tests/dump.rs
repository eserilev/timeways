//! Pages from a MediaWiki XML export (GAMEPLAY.md 5.10).

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[allow(dead_code, reason = "each test file uses a part of the helpers")]
mod wiki_dump;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use timeways_story::dump::{DumpError, Page, pages, xml_scan, xml_texts};
use wiki_dump::{DumpPage, article, fresh, write_dump, xml};

fn wanted(titles: &[&str]) -> BTreeSet<String> {
    titles.iter().map(ToString::to_string).collect()
}

fn titles(titles: &[&str]) -> Vec<String> {
    titles.iter().map(ToString::to_string).collect()
}

#[test]
fn only_the_wanted_articles_come_back() {
    let dump = xml(&[
        article("Testvale", "Hills."),
        article("Mockshire", "An inn."),
    ]);

    let found = xml_texts(dump.as_bytes(), &wanted(&["Mockshire", "Nowhere"])).unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found["Mockshire"], "An inn.");
}

#[test]
fn a_page_in_another_namespace_is_not_the_article() {
    let dump = xml(&[
        DumpPage {
            title: "Testvale",
            namespace: 1,
            text: "A talk page.",
        },
        article("Testvale", "The article."),
    ]);

    let found = xml_texts(dump.as_bytes(), &wanted(&["Testvale"])).unwrap();

    assert_eq!(found["Testvale"], "The article.");
}

#[test]
fn xml_entities_are_decoded() {
    let dump = xml(&[article("Testvale", "Salt & <pepper> \"here\"")]).replace("here", "&#104;ere");

    let found = xml_texts(dump.as_bytes(), &wanted(&["Testvale"])).unwrap();

    assert_eq!(found["Testvale"], "Salt & <pepper> \"here\"");
}

#[test]
fn a_redirect_is_followed_one_step() {
    let dump = write_dump(
        "dump-redirect",
        &[
            article("Old Tower", "#REDIRECT [[Testvale Tower]]"),
            article("Testvale Tower", "#REDIRECT [[Loop Tower]]"),
            article("Loop Tower", "The end."),
        ],
    );

    let found = pages(&dump, &titles(&["Old Tower"])).unwrap();

    assert_eq!(
        found["Old Tower"],
        Page {
            title: "Testvale Tower".to_string(),
            text: "#REDIRECT [[Loop Tower]]".to_string()
        }
    );
}

#[test]
fn a_redirect_to_a_missing_page_gives_nothing() {
    let dump = write_dump(
        "dump-redirect-missing",
        &[article("Old Tower", "#REDIRECT [[Gone Tower]]")],
    );

    let found = pages(&dump, &titles(&["Old Tower"])).unwrap();

    assert!(found.is_empty());
}

#[test]
fn broken_xml_is_an_error_not_a_panic() {
    let dump = "<mediawiki><page><title>Testvale</title></ns></page></mediawiki>";

    let result = xml_texts(dump.as_bytes(), &wanted(&["Testvale"]));

    assert!(matches!(result, Err(DumpError::Xml(_))), "{result:?}");
}

#[test]
fn a_dump_cut_short_is_an_error() {
    let dump = xml(&[article("Testvale", "Hills.")]);
    let cut = &dump[..dump.find("</page>").unwrap()];

    let result = xml_texts(cut.as_bytes(), &wanted(&["Testvale"]));

    assert!(matches!(result, Err(DumpError::Truncated)), "{result:?}");
}

#[test]
fn a_7z_dump_reads_like_the_xml() {
    let folder = fresh("dump-7z-source");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("testwiki.xml"),
        xml(&[article("Testvale", "Hills.")]),
    )
    .unwrap();
    let archive = fresh("dump.7z");
    sevenz_rust2::compress_to_path(&folder, &archive).unwrap();

    let found = pages(&archive, &titles(&["Testvale"])).unwrap();

    assert_eq!(found["Testvale"].text, "Hills.");
}

#[test]
fn a_7z_without_xml_is_an_error() {
    let folder = fresh("dump-7z-empty");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("notes.txt"), "no dump here").unwrap();
    let archive = fresh("dump-empty.7z");
    sevenz_rust2::compress_to_path(&folder, &archive).unwrap();

    let result = pages(&archive, &titles(&["Testvale"]));

    assert!(matches!(result, Err(DumpError::NoXml)), "{result:?}");
}

/// So the builder knows where each book leads before its second read of the dump.
#[test]
fn a_scan_reads_to_the_end_and_keeps_every_redirect() {
    let dump = xml(&[
        article("Testvale", "Hills."),
        article("Old Tower", "#REDIRECT [[testvale Tower]]"),
        DumpPage {
            title: "Talk Tower",
            namespace: 1,
            text: "#REDIRECT [[Testvale]]",
        },
    ]);

    let scan = xml_scan(dump.as_bytes(), &wanted(&["Testvale"])).unwrap();

    assert_eq!(scan.texts["Testvale"], "Hills.");
    assert_eq!(
        scan.redirects,
        BTreeMap::from([("Old Tower".to_string(), "Testvale Tower".to_string())])
    );
}

#[test]
fn a_scan_lacks_the_targets_of_redirects_and_the_titles_it_did_not_want() {
    let dump = xml(&[
        article("Testvale", "Hills."),
        article("Old Tower", "#REDIRECT [[Testvale Tower]]"),
    ]);
    let scan = xml_scan(dump.as_bytes(), &wanted(&["Testvale"])).unwrap();

    let lacking = scan.lacking(&titles(&["Testvale", "Old Tower", "Mockshire"]));

    assert_eq!(lacking, wanted(&["Testvale Tower", "Mockshire"]));
}

#[test]
fn a_page_of_a_scan_follows_its_redirect_into_a_later_read() {
    let dump = xml(&[article("Old Tower", "#REDIRECT [[Testvale Tower]]")]);
    let scan = xml_scan(dump.as_bytes(), &wanted(&[])).unwrap();
    let later = BTreeMap::from([("Testvale Tower".to_string(), "Stones.".to_string())]);

    let page = scan.page("Old Tower", &later);

    let expected = Page {
        title: "Testvale Tower".to_string(),
        text: "Stones.".to_string(),
    };
    assert_eq!(page, Some(expected));
}
