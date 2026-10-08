//! Game names (docs/plans/lore-names-and-now.md 1): the bundled rows, and each compare of
//! a name that goes through them. The names are those of the game and of the wiki.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use hourglass::Tick;
use std::collections::BTreeSet;
use std::path::Path;
use timeways_story::character::Character;
use timeways_story::game_names::{ROWS, parse, rows, wiki_name};
use timeways_story::narrator_lore::{is_thin, lore_about};
use timeways_story::outcome_passages::{Npc, PageKind, Stance, with_known_bosses};
use timeways_story::pack::{Deed, Dependency, Link, Origin, Pack, Passage, SetupFor};
use timeways_story::seen::SeenIndex;
use timeways_story::spoiler::{outcome_allowed, setup_allowed};

#[test]
fn the_bundled_game_names_read() {
    assert!(ROWS.is_ok(), "{:?}", ROWS.as_ref().err());
    assert!(rows().len() >= 15);
}

#[test]
fn the_game_names_are_a_function() {
    let wiki: BTreeSet<&str> = rows().iter().map(|row| row.wiki.as_str()).collect();
    let mut seen = BTreeSet::new();

    for row in rows() {
        for game in &row.game {
            assert!(seen.insert(game.as_str()), "{game} sits in two rows");
            assert!(!wiki.contains(game.as_str()), "{game} is a wiki name too");
        }
    }
}

#[test]
fn no_row_holds_a_bare_surname() {
    for row in rows() {
        let wiki_words = row.wiki.split_whitespace().count();
        for game in &row.game {
            let one_word = game.split_whitespace().count() == 1;
            assert!(!one_word || wiki_words == 1 || is_whole_first_word(game, &row.wiki));
        }
    }
}

/// "Aku'mai" is the whole name of "Aku'mai the Devourer", not a surname.
fn is_whole_first_word(game: &str, wiki: &str) -> bool {
    wiki.split_whitespace().next() == Some(game)
}

#[test]
fn a_game_name_gives_its_wiki_name_and_any_other_name_stays() {
    assert_eq!(wiki_name("High Inquisitor Whitemane"), "Sally Whitemane");
    assert_eq!(wiki_name("Sally Whitemane"), "Sally Whitemane");
    assert_eq!(wiki_name("Whitemane"), "Whitemane");
    assert_eq!(wiki_name("Edwin VanCleef"), "Edwin VanCleef");
}

#[test]
fn a_file_with_an_unknown_field_is_refused() {
    assert!(parse("[[person]]\nwiki = \"A\"\ngame = [\"B\"]\nnick = \"C\"\n").is_err());
}

fn in_the_monastery() -> Character {
    let mut character = Character::new();
    character
        .enter_zone(Tick(1), "Scarlet Monastery", None)
        .unwrap();
    character
}

#[test]
fn a_kill_under_the_game_name_unlocks_a_passage_tagged_with_the_wiki_name() {
    let tag = Dependency::Foe("Sally Whitemane".to_string());
    let mut character = in_the_monastery();
    assert!(!outcome_allowed(&character, std::slice::from_ref(&tag)));

    character
        .defeat_npc(Tick(2), "High Inquisitor Whitemane")
        .unwrap();

    assert!(outcome_allowed(&character, std::slice::from_ref(&tag)));
}

#[test]
fn a_kill_under_the_game_name_makes_a_setup_tagged_with_the_wiki_name_stale() {
    let setup = SetupFor {
        deed: Deed::Foe("Sicco Thermaplugg".to_string()),
        instance: "Gnomeregan".to_string(),
    };
    let mut character = Character::new();
    assert!(setup_allowed(&character, Some(&setup)));

    character
        .defeat_npc(Tick(2), "Mekgineer Thermaplugg")
        .unwrap();

    assert!(!setup_allowed(&character, Some(&setup)));
}

#[test]
fn a_kill_of_moira_never_unlocks_a_tag_of_emperor_thaurissan() {
    let tag = Dependency::Foe("Emperor Dagran Thaurissan".to_string());
    let mut character = Character::new();

    character
        .defeat_npc(Tick(2), "Princess Moira Bronzebeard")
        .unwrap();

    assert!(!outcome_allowed(&character, std::slice::from_ref(&tag)));
}

#[test]
fn a_kill_of_the_emperor_unlocks_a_tag_under_his_page_title() {
    let tag = Dependency::Foe("Dagran Thaurissan".to_string());
    let mut character = Character::new();

    character
        .defeat_npc(Tick(2), "Emperor Dagran Thaurissan")
        .unwrap();

    assert!(outcome_allowed(&character, std::slice::from_ref(&tag)));
}

#[test]
fn meeting_moira_bronzebeard_never_meets_magni() {
    let mut character = Character::new();

    character
        .meet_npc(Tick(2), "Princess Moira Bronzebeard")
        .unwrap();

    assert!(!character.has_met("King Magni Bronzebeard"));
    assert!(!character.has_met("Magni Bronzebeard"));
    assert!(character.has_met("Moira Thaurissan"));
}

#[test]
fn meeting_an_npc_under_the_game_name_meets_the_wiki_name() {
    let mut character = Character::new();

    character
        .meet_npc(Tick(2), "King Magni Bronzebeard")
        .unwrap();

    assert!(character.knows_all(&[Link::Npc("Magni Bronzebeard".to_string())]));
}

fn whitemane_page() -> Passage {
    Passage {
        text: "Sally Whitemane was the High Inquisitor of the Scarlet Crusade.".to_string(),
        source: "the wiki page \"Sally Whitemane\"".to_string(),
        links: vec![Link::Place("Scarlet Monastery".to_string())],
        origin: Origin::Pack,
        about: Some("Sally Whitemane".to_string()),
        depends_on: Vec::new(),
        setup_for: None,
    }
}

#[test]
fn the_kill_of_high_inquisitor_whitemane_takes_the_page_of_sally_whitemane() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("game-names-whitemane.sqlite");
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, &[whitemane_page()]).unwrap();
    let pack = Pack::open(&path).unwrap();
    let seen = SeenIndex::new(&[]).unwrap();
    let character = in_the_monastery();

    let lore = lore_about(&pack, &seen, &character, "High Inquisitor Whitemane").unwrap();

    assert_eq!(lore, Some(whitemane_page()));
}

#[test]
fn the_page_of_the_wiki_name_is_no_thin_lore_for_the_game_name() {
    let subjects = ["High Inquisitor Whitemane".to_string()];

    assert!(!is_thin(&subjects, Some(&whitemane_page())));
}

#[test]
fn a_known_boss_under_its_page_title_is_a_foe_under_its_infobox_name() {
    let emperor = PageKind::Npc(Npc {
        name: "Emperor Dagran Thaurissan".to_string(),
        stance: Stance::NoFoe,
    });

    let known = with_known_bosses(emperor, &["Dagran Thaurissan".to_string()]);

    assert_eq!(
        known,
        PageKind::Npc(Npc {
            name: "Emperor Dagran Thaurissan".to_string(),
            stance: Stance::Foe,
        })
    );
}
