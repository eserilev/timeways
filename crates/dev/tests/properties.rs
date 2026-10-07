//! Rules of the scenarios that hold for any start time. They live here and not in
//! `crates/story/tests/properties.rs`, because the story crate does not know the
//! scenarios.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{NAME, REALM, folder};
use proptest::prelude::*;
use std::path::Path;
use timeways_dev::scenario::BUILT_IN;
use timeways_dev::seed::play;
use timeways_story::pack::Pack;
use timeways_story::store::{Store, Table};
use timeways_story::story::Story;

/// Every row of every table of the world, as text.
fn rows(folder: &Path) -> Vec<String> {
    let world = timeways_dev::worlds::world_file(folder, REALM, NAME).unwrap();
    let connection = rusqlite::Connection::open(world).unwrap();
    let mut names: Vec<String> = Table::ALL
        .iter()
        .map(|table| table.name().to_string())
        .collect();
    names.extend(["inputs", "calls", "reads"].map(String::from));
    let mut all = Vec::new();
    for name in names {
        let mut statement = connection
            .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        let found = statement
            .query_map([], |row| {
                let cells: Vec<String> = (0..columns)
                    .map(|n| format!("{:?}", row.get_ref(n).unwrap()))
                    .collect();
                Ok(format!("{name}: {}", cells.join(" | ")))
            })
            .unwrap();
        all.extend(found.map(Result::unwrap));
    }
    all
}

fn seeded_rows(name: &str, start: u64, text: &str) -> Vec<String> {
    let folder = folder(name);
    let story = Story::new(Pack::empty().unwrap(), Store::Folder(folder.clone()));
    let scenario = timeways_dev::scenario::Scenario::parse(text).unwrap();
    let report = play(&scenario.starting_at(start), REALM, NAME, story, None);
    assert!(report.is_clean(), "{report:#?}");
    rows(&folder)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(12))]

    /// A scenario is data, so two plays of it build the same world, row for row.
    #[test]
    fn every_scenario_replays_to_the_same_world_twice(
        index in 0..BUILT_IN.len(),
        start in prop_oneof![Just(1_700_000_000_u64), 1_600_000_000_u64..1_780_000_000],
    ) {
        let (name, _, text) = BUILT_IN[index];

        let first = seeded_rows(name, start, text);
        let second = seeded_rows(name, start, text);

        prop_assert!(!first.is_empty());
        prop_assert_eq!(first, second);
    }
}

/// A frame rate or a latency: the edges come often, as do equal values.
fn bench_value() -> impl Strategy<Value = f64> {
    prop_oneof![Just(0.0), Just(1.0), Just(60.0), Just(1e6), 0.0..300.0_f64]
}

proptest! {
    /// The numbers of a bench report keep their order for any list of values.
    #[test]
    fn the_numbers_of_a_spread_keep_their_order(
        values in prop::collection::vec(bench_value(), 0..40),
    ) {
        let Some(spread) = timeways_dev::stats::spread(&values) else {
            prop_assert!(values.is_empty());
            return Ok(());
        };

        prop_assert_eq!(spread.count, values.len());
        prop_assert!(spread.min <= spread.p5 && spread.p5 <= spread.median);
        prop_assert!(spread.median <= spread.p95 && spread.p95 <= spread.max);
        prop_assert!(spread.min - 1e-9 <= spread.mean && spread.mean <= spread.max + 1e-9);
    }
}
