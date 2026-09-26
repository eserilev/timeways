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
