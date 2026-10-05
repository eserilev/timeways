#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use timeways_story::pack::{Link, Origin, Pack, PackError, Passage};

fn fresh_path(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pack-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    path
}

fn passage(text: &str, source: &str, links: Vec<Link>) -> Passage {
    Passage {
        text: text.to_string(),
        source: source.to_string(),
        links,
        origin: Origin::Pack,
        about: None,
    }
}

fn place(name: &str) -> Link {
    Link::Place(name.to_string())
}

fn pack_of(name: &str, passages: &[Passage]) -> Pack {
    let path = fresh_path(name);
    Pack::write(&path, passages).unwrap();
    Pack::open(&path).unwrap()
}

#[test]
fn a_written_passage_comes_back_with_its_source_and_links() {
    let tower = passage(
        "The tower of Testvale fell to test goblins.",
        "https://example.test/1",
        vec![place("Testvale"), Link::Npc("Keeper Stubbs".to_string())],
    );
    let pack = pack_of("round-trip", std::slice::from_ref(&tower));

    let found = pack.search("tower", 10).unwrap();

    assert_eq!(found, [tower]);
}

#[test]
fn a_common_passage_comes_back_common() {
    let history = passage(
        "The testers came from the sea.",
        "https://example.test/1",
        vec![Link::Common],
    );
    let pack = pack_of("common", std::slice::from_ref(&history));

    let found = pack.search("testers", 10).unwrap();

    assert_eq!(found, [history]);
}

#[test]
fn the_better_match_comes_first() {
    let passages = [
        passage(
            "Mockshire has a mill.",
            "https://example.test/1",
            vec![place("Mockshire")],
        ),
        passage(
            "The tower of Testvale is a ruined tower.",
            "https://example.test/2",
            vec![place("Testvale")],
        ),
    ];
    let pack = pack_of("ranking", &passages);

    let found = pack.search("ruined tower", 10).unwrap();

    assert_eq!(found[0].source, "https://example.test/2");
}

#[test]
fn a_search_returns_at_most_the_limit() {
    let passages: Vec<Passage> = (0..5)
        .map(|n| {
            passage(
                "A tower.",
                &format!("https://example.test/{n}"),
                vec![place("Testvale")],
            )
        })
        .collect();
    let pack = pack_of("limit", &passages);

    let found = pack.search("tower", 3).unwrap();

    assert_eq!(found.len(), 3);
}

#[test]
fn index_syntax_in_a_question_counts_as_plain_words() {
    let tower = passage(
        "The tower fell.",
        "https://example.test/1",
        vec![place("Testvale")],
    );
    let pack = pack_of("syntax", &[tower]);

    let found = pack.search(r#"tower" NEAR( * -fell OR AND ^"#, 10).unwrap();

    assert_eq!(found.len(), 1);
}

#[test]
fn a_question_with_no_words_finds_nothing() {
    let tower = passage(
        "The tower fell.",
        "https://example.test/1",
        vec![place("Testvale")],
    );
    let pack = pack_of("no-words", &[tower]);

    let found = pack.search("?!", 10).unwrap();

    assert!(found.is_empty());
}

#[test]
fn a_passage_with_no_links_is_refused() {
    let unlinked = passage("Legend says much.", "https://example.test/1", Vec::new());

    let result = Pack::write(&fresh_path("unlinked"), &[unlinked]);

    assert!(matches!(result, Err(PackError::Unlinked { url }) if url == "https://example.test/1"));
}

#[test]
fn a_file_of_another_format_version_is_refused() {
    let path = fresh_path("version");
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 99)
        .unwrap();

    let result = Pack::open(&path);

    assert!(matches!(result, Err(PackError::Version { found: 99 })));
}

#[test]
fn a_missing_file_is_refused() {
    let result = Pack::open(&fresh_path("missing"));

    assert!(matches!(result, Err(PackError::Sqlite(_))));
}

fn about(passage: Passage, name: &str) -> Passage {
    Passage {
        about: Some(name.to_string()),
        ..passage
    }
}

#[test]
fn a_passage_keeps_the_subject_of_its_page() {
    let mine = about(
        passage(
            "The Deadmines lie beneath Moonbrook.",
            "the wiki page \"Deadmines\"",
            vec![place("The Deadmines")],
        ),
        "The Deadmines",
    );
    let pack = pack_of("about-round-trip", std::slice::from_ref(&mine));

    let found = pack.search("Moonbrook", 10).unwrap();

    assert_eq!(found, [mine]);
}

#[test]
fn the_own_page_of_a_subject_comes_in_page_order() {
    let lead = about(
        passage("The lead of the mine.", "a", vec![place("The Deadmines")]),
        "The Deadmines",
    );
    let boss = passage(
        "A boss of the mine, linked to it.",
        "b",
        vec![place("The Deadmines")],
    );
    let history = about(
        passage(
            "The history of the mine.",
            "a",
            vec![place("The Deadmines")],
        ),
        "The Deadmines",
    );
    let pack = pack_of("about-order", &[lead.clone(), boss, history.clone()]);

    let found = pack.about("The Deadmines", 10).unwrap();

    assert_eq!(found, [lead, history]);
}

#[test]
fn a_pack_of_format_one_is_refused() {
    let path = fresh_path("format-one");
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 1)
        .unwrap();

    let result = Pack::open(&path);

    assert!(matches!(result, Err(PackError::Version { found: 1 })));
}
