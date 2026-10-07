#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::Path;
use timeways_story::pack::{Dependency, Link, Origin, Pack, Passage};
use timeways_story::pack_coverage::{PlaceKind, ZoneLevels, coverage, leveling_path, text};
use timeways_story::pack_sources::Sources;

fn passage(about: Option<&str>, links: Vec<Link>) -> Passage {
    Passage {
        text: "The fields of Westfall lie fallow since the Second War, and the Defias hold them."
            .to_string(),
        source: "the wiki page \"Westfall\"".to_string(),
        links,
        origin: Origin::Pack,
        about: about.map(str::to_string),
        depends_on: None,
        setup_for: None,
    }
}

fn place(name: &str) -> Link {
    Link::Place(name.to_string())
}

fn pack_of(name: &str, passages: &[Passage]) -> Pack {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("coverage-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    Pack::write(&path, passages).unwrap();
    Pack::open(&path).unwrap()
}

#[test]
fn every_place_of_the_zone_levels_is_named_once() {
    let path = leveling_path(&ZoneLevels::bundled().unwrap());

    let mut names = BTreeSet::new();
    let doubled: Vec<&str> = path
        .iter()
        .map(|stop| stop.name.as_str())
        .filter(|name| !names.insert(*name))
        .collect();

    assert_eq!(doubled, Vec::<&str>::new());
}

#[test]
fn every_level_range_of_the_zone_levels_is_within_classic() {
    let table = ZoneLevels::bundled().unwrap();

    let broken: Vec<&str> = table
        .places
        .iter()
        .filter(|place| {
            let [low, high] = place.levels;
            low == 0 || low > high || high > 60
        })
        .map(|place| place.name.as_str())
        .collect();

    assert_eq!(broken, Vec::<&str>::new());
}

#[test]
fn every_dungeon_has_the_zone_outside_it_and_no_subzone() {
    let table = ZoneLevels::bundled().unwrap();

    let broken: Vec<&str> = table
        .places
        .iter()
        .filter(|place| {
            let dungeon = place.kind == PlaceKind::Dungeon;
            dungeon != place.within.is_some() || (dungeon && !place.subzones.is_empty())
        })
        .map(|place| place.name.as_str())
        .collect();

    assert_eq!(broken, Vec::<&str>::new());
}

#[test]
fn every_instance_of_the_pack_sources_is_on_the_path() {
    let path = leveling_path(&ZoneLevels::bundled().unwrap());
    let sources = Sources::bundled().unwrap();

    let missing: Vec<&String> = sources
        .instances
        .iter()
        .filter(|instance| !path.iter().any(|stop| &stop.name == *instance))
        .collect();

    assert_eq!(missing, Vec::<&String>::new());
}

#[test]
fn the_report_counts_the_passages_of_each_place_from_the_pack() {
    let pack = pack_of(
        "counts",
        &[
            passage(Some("Westfall"), vec![place("Westfall")]),
            passage(
                None,
                vec![
                    place("Westfall"),
                    Link::Npc("Gryan Stoutmantle".to_string()),
                ],
            ),
            Passage {
                depends_on: Some(Dependency::Foe("Edwin VanCleef".to_string())),
                ..passage(None, vec![place("The Deadmines")])
            },
            passage(Some("Edwin VanCleef"), vec![place("The Deadmines")]),
        ],
    );
    let path = leveling_path(&ZoneLevels::bundled().unwrap());
    let bosses = vec![(
        "Edwin VanCleef".to_string(),
        vec!["The Deadmines".to_string()],
    )];

    let report = coverage(&pack, &path, &bosses).unwrap();

    let of = |name: &str| report.iter().find(|place| place.stop.name == name).unwrap();
    let westfall = &of("Westfall").counts;
    let mines = &of("The Deadmines").counts;
    assert_eq!(
        (westfall.usable, westfall.own_page, westfall.later),
        (1, 1, 1)
    );
    assert_eq!((mines.usable, mines.outcomes, mines.bosses), (1, 1, 1));
    assert!(of("Duskwood").silent);
}

#[test]
fn the_text_report_marks_a_silent_place_and_lists_the_gaps() {
    let pack = pack_of("text", &[passage(None, vec![place("Westfall")])]);
    let path = leveling_path(&ZoneLevels::bundled().unwrap());

    let report = coverage(&pack, &path, &[]).unwrap();
    let shown = text(&report, 20);

    assert!(shown.contains("Duskwood  SILENT"));
    assert!(shown.contains("The 20 biggest gaps:"));
}

#[test]
fn the_json_report_names_each_place_with_its_counts() {
    let pack = pack_of("json", &[passage(None, vec![place("Westfall")])]);
    let path = leveling_path(&ZoneLevels::bundled().unwrap());

    let report = coverage(&pack, &path, &[]).unwrap();
    let json = serde_json::to_value(&report).unwrap();

    let westfall = json
        .as_array()
        .unwrap()
        .iter()
        .find(|place| place["name"] == "Westfall")
        .unwrap();
    assert_eq!(westfall["usable"], 1);
    assert_eq!(westfall["silent"], false);
    assert_eq!(westfall["levels"], serde_json::json!([10, 20]));
}
