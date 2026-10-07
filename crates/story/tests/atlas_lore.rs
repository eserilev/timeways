//! The lore of the Knowledge atlas (GAMEPLAY.md 3.6): the own page of each place and
//! person, behind the spoiler limit, and "There's more to learn here." for a place.

#![allow(clippy::unwrap_used)]

use hourglass::Tick;
use std::path::Path;
use timeways_story::atlas_lore::{LORE_BYTES, LORE_CHARS, LORE_SUBJECTS, Lore, clipped, lore_of};
use timeways_story::character::Character;
use timeways_story::journal::journal;
use timeways_story::pack::{Deed, Dependency, Link, Origin, Pack, Passage, SetupFor};
use timeways_story::serve;
use timeways_story::store::Store;
use timeways_story::story::Story;

fn pack_with(name: &str, passages: &[Passage]) -> Pack {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("atlas-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, passages).unwrap();
    Pack::open(&path).unwrap()
}

fn passage(text: &str, about: &str, links: Vec<Link>) -> Passage {
    Passage {
        text: text.to_string(),
        source: format!("the wiki page \"{about}\""),
        links,
        origin: Origin::Pack,
        about: Some(about.to_string()),
        depends_on: None,
        setup_for: None,
    }
}

fn place(name: &str) -> Link {
    Link::Place(name.to_string())
}

fn npc(name: &str) -> Link {
    Link::Npc(name.to_string())
}

/// A Forsaken at the Sepulcher of Silverpine Forest, who met Dalar Dawnweaver.
fn at_the_sepulcher() -> Character {
    let mut character = Character::new();
    character.reach_level(Tick(1), 12).unwrap();
    character
        .enter_zone(Tick(2), "Silverpine Forest", Some("The Sepulcher"))
        .unwrap();
    character.meet_npc(Tick(3), "Dalar Dawnweaver").unwrap();
    character
}

fn lore(pack: &Pack, character: &Character) -> Vec<Lore> {
    let journal = journal(character);
    lore_of(pack, character, &journal.places, &journal.people).unwrap()
}

fn about<'a>(lore: &'a [Lore], name: &str) -> Option<&'a Lore> {
    lore.iter().find(|entry| entry.about == name)
}

#[test]
fn a_place_shows_the_lead_of_its_own_page() {
    let lead = passage(
        "The Forsaken hold the Sepulcher, an old crypt in the forest.",
        "The Sepulcher",
        vec![place("The Sepulcher")],
    );
    let later = passage(
        "Deathstalkers go out from it.",
        "The Sepulcher",
        vec![place("The Sepulcher")],
    );
    let pack = pack_with("lead", &[lead, later]);

    let lore = lore(&pack, &at_the_sepulcher());

    let sepulcher = about(&lore, "The Sepulcher").unwrap();
    assert_eq!(
        sepulcher.text.as_deref(),
        Some("The Forsaken hold the Sepulcher, an old crypt in the forest.")
    );
    assert!(!sepulcher.more);
}

#[test]
fn a_person_shows_the_lead_of_their_own_page() {
    let dalar = passage(
        "Dalar Dawnweaver studies the worgen for the Forsaken.",
        "Dalar Dawnweaver",
        vec![npc("Dalar Dawnweaver")],
    );
    let pack = pack_with("person", &[dalar]);

    let lore = lore(&pack, &at_the_sepulcher());

    assert!(about(&lore, "Dalar Dawnweaver").unwrap().text.is_some());
}

#[test]
fn a_passage_about_someone_you_never_met_stays_hidden_and_the_place_has_more_to_learn() {
    let hidden = passage(
        "Arugal called the worgen into Silverpine.",
        "Silverpine Forest",
        vec![place("Silverpine Forest"), npc("Archmage Arugal")],
    );
    let pack = pack_with("hidden", &[hidden]);

    let lore = lore(&pack, &at_the_sepulcher());

    let forest = about(&lore, "Silverpine Forest").unwrap();
    assert_eq!(forest.text, None);
    assert!(forest.more);
}

#[test]
fn a_person_never_says_there_is_more_to_learn() {
    let hidden = passage(
        "Dalar Dawnweaver once served Arugal.",
        "Dalar Dawnweaver",
        vec![npc("Dalar Dawnweaver"), npc("Archmage Arugal")],
    );
    let pack = pack_with("person-more", &[hidden]);

    let lore = lore(&pack, &at_the_sepulcher());

    assert_eq!(about(&lore, "Dalar Dawnweaver"), None);
}

#[test]
fn a_deed_that_the_pack_tied_to_no_one_promises_nothing_more() {
    let mut never = passage(
        "Adventurers cleared the forest.",
        "Silverpine Forest",
        vec![place("Silverpine Forest")],
    );
    never.depends_on = Some(Dependency::Unresolved);
    let pack = pack_with("unresolved", &[never]);

    let lore = lore(&pack, &at_the_sepulcher());

    assert_eq!(about(&lore, "Silverpine Forest"), None);
}

#[test]
fn a_setup_of_a_deed_that_you_did_promises_nothing_more() {
    let mut character = at_the_sepulcher();
    character.defeat_npc(Tick(4), "Archmage Arugal").unwrap();
    let mut stale = passage(
        "Dalar wants Arugal dead.",
        "The Sepulcher",
        vec![place("The Sepulcher")],
    );
    stale.setup_for = Some(SetupFor {
        deed: Deed::Foe("Archmage Arugal".to_string()),
        instance: "The Sepulcher".to_string(),
    });
    let pack = pack_with("stale", &[stale]);

    let lore = lore(&pack, &character);

    assert_eq!(about(&lore, "The Sepulcher"), None);
}

#[test]
fn a_place_with_no_lore_has_no_entry() {
    let pack = pack_with("none", &[]);

    let lore = lore(&pack, &at_the_sepulcher());

    assert!(lore.is_empty());
}

#[test]
fn a_long_passage_is_cut_after_a_sentence_within_both_limits() {
    let sentence = "The Forsaken hold the crypt of the forest. ";
    let text = sentence.repeat(40);

    let cut = clipped(&text);

    assert!(cut.chars().count() <= LORE_CHARS);
    assert!(cut.len() <= LORE_BYTES);
    assert!(cut.ends_with("forest."), "{cut}");
}

#[test]
fn letters_outside_ascii_stop_the_text_at_its_bytes() {
    let text = "Ж".repeat(LORE_CHARS);

    let cut = clipped(&text);

    assert!(cut.len() <= LORE_BYTES);
    assert!(!cut.is_empty());
}

#[test]
fn a_passage_loses_its_line_breaks() {
    assert_eq!(clipped("One line.\nTwo\tlines."), "One line. Two lines.");
}

#[test]
fn zones_come_before_subzones_and_people_when_the_list_is_full() {
    let mut character = Character::new();
    let mut passages = Vec::new();
    for n in 0..LORE_SUBJECTS {
        let name = format!("Person {n}");
        character.meet_npc(Tick(1), &name).unwrap();
        passages.push(passage("A person of note.", &name, vec![npc(&name)]));
    }
    character
        .enter_zone(Tick(2), "Silverpine Forest", None)
        .unwrap();
    passages.push(passage(
        "A forest of pines.",
        "Silverpine Forest",
        vec![place("Silverpine Forest")],
    ));
    let pack = pack_with("full", &passages);

    let lore = lore(&pack, &character);

    assert_eq!(lore.len(), LORE_SUBJECTS);
    assert_eq!(lore[0].about, "Silverpine Forest");
}

#[test]
fn the_journal_carries_the_lore_of_each_place_that_you_visited() {
    let lead = passage(
        "The Forsaken hold the Sepulcher, an old crypt in the forest.",
        "The Sepulcher",
        vec![place("The Sepulcher")],
    );
    let mut story = Story::new(pack_with("journal", &[lead]), Store::Memory);
    let lines = [
        r#"{"type":"character_entered","realm":"Stormrage","name":"Ada"}"#,
        r#"{"type":"zone_entered","id":1,"at":1790000000,"zone":"Silverpine Forest","subzone":"The Sepulcher"}"#,
    ];
    for line in lines {
        let _ = serve::line(&mut story, line.as_bytes().to_vec());
    }

    let ask = r#"{"type":"journal_asked","id":2}"#;
    let page = serve::line(&mut story, ask.as_bytes().to_vec())
        .lines
        .remove(0);

    let page: serde_json::Value = serde_json::from_str(&page).unwrap();
    assert_eq!(page["lore"][0]["about"], "The Sepulcher");
    assert_eq!(
        page["lore"][0]["text"],
        "The Forsaken hold the Sepulcher, an old crypt in the forest."
    );
}
