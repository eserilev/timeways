#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use timeways_story::pack::{Deed, Dependency, Link, Origin, Pack, PackError, Passage, SetupFor};

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
        depends_on: Vec::new(),
        setup_for: None,
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

#[test]
fn a_pack_of_format_two_is_refused_because_it_has_no_outcome_tags() {
    let path = fresh_path("format-two");
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 2)
        .unwrap();

    let result = Pack::open(&path);

    assert!(matches!(result, Err(PackError::Version { found: 2 })));
}

#[test]
fn an_outcome_passage_comes_back_with_what_it_depends_on() {
    let tags = [
        Dependency::Foe("Edwin VanCleef".to_string()),
        Dependency::Quest("Red Linen Goods".to_string()),
        Dependency::Unresolved,
    ];
    let passages: Vec<Passage> = tags
        .iter()
        .map(|tag| Passage {
            depends_on: vec![tag.clone()],
            ..passage(
                "The adventurers killed the test ooze.",
                "https://example.test/1",
                vec![place("Testvale")],
            )
        })
        .collect();
    let pack = pack_of("outcomes", &passages);

    let found = pack.search("ooze", 10).unwrap();

    assert_eq!(found, passages);
}

#[test]
fn an_unknown_kind_of_dependency_is_an_error() {
    let path = fresh_path("unknown-dependency");
    let tower = passage(
        "The tower of Testvale fell.",
        "https://example.test/1",
        vec![place("Testvale")],
    );
    Pack::write(&path, &[tower]).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO depends_on (passage, kind, name) VALUES (1, 'rumor', 'x')",
            [],
        )
        .unwrap();
    let pack = Pack::open(&path).unwrap();

    let result = pack.search("tower", 10);

    assert!(matches!(
        result,
        Err(PackError::UnknownDependency { ref kind }) if kind == "rumor"
    ));
}

#[test]
fn a_pack_of_format_three_is_refused_because_it_has_no_setup_tags() {
    let path = fresh_path("format-three");
    rusqlite::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", 3)
        .unwrap();

    let result = Pack::open(&path);

    assert!(matches!(result, Err(PackError::Version { found: 3 })));
}

fn kingpin_setup(deed: Deed) -> SetupFor {
    SetupFor {
        deed,
        instance: "The Testmines".to_string(),
    }
}

#[test]
fn a_setup_passage_comes_back_with_its_deed_and_its_instance() {
    let tags = [
        Deed::Foe("Test Kingpin".to_string()),
        Deed::Quest("Into the Testmines".to_string()),
    ];
    let passages: Vec<Passage> = tags
        .iter()
        .map(|deed| Passage {
            setup_for: Some(kingpin_setup(deed.clone())),
            ..passage(
                "The marshal sent adventurers to kill the kingpin.",
                "https://example.test/1",
                vec![place("Testvale")],
            )
        })
        .collect();
    let pack = pack_of("setups", &passages);

    let found = pack.search("kingpin", 10).unwrap();

    assert_eq!(found, passages);
}

#[test]
fn the_passages_of_an_instance_are_its_links_and_its_setups_in_pack_order() {
    let lead = passage(
        "The Testmines were dug by miners.",
        "https://example.test/1",
        vec![place("The Testmines")],
    );
    let elsewhere = passage(
        "Testvale has a mill.",
        "https://example.test/2",
        vec![place("Testvale")],
    );
    let setup = Passage {
        setup_for: Some(kingpin_setup(Deed::Foe("Test Kingpin".to_string()))),
        ..passage(
            "The marshal sent adventurers to kill the kingpin.",
            "https://example.test/3",
            vec![place("Testvale")],
        )
    };
    let pack = pack_of(
        "instance-passages",
        &[lead.clone(), elsewhere, setup.clone()],
    );

    let all = pack.of_place("The Testmines").unwrap();
    let setups = pack.setups_of("The Testmines").unwrap();

    assert_eq!(all, vec![lead, setup.clone()]);
    assert_eq!(setups, vec![setup]);
}

#[test]
fn an_unknown_kind_of_setup_is_an_error() {
    let path = fresh_path("unknown-setup");
    let tower = passage(
        "The tower of Testvale fell.",
        "https://example.test/1",
        vec![place("Testvale")],
    );
    Pack::write(&path, &[tower]).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO setup_for (passage, kind, name, instance) VALUES (1, 'rumor', 'x', 'y')",
            [],
        )
        .unwrap();
    let pack = Pack::open(&path).unwrap();

    let result = pack.search("tower", 10);

    assert!(matches!(
        result,
        Err(PackError::UnknownSetup { ref kind }) if kind == "rumor"
    ));
}
