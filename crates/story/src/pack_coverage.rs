//! The coverage report of a lore pack (GAMEPLAY.md 5.10): for each place of the leveling
//! path, the passages that a first arrival can tell, the gated ones, and the places where
//! the narrator stays silent. A person reads it to find the gaps of `pack_sources.toml`.

use crate::pack::{Dependency, Link, Pack, PackError, Passage};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use thiserror::Error;

const BUNDLED: &str = include_str!("../data/zone_levels.toml");

/// A place has more passages than this only in a pack far larger than any of today.
const MOST_PASSAGES: u32 = 1000;

#[derive(Debug, Error)]
pub enum CoverageError {
    #[error("zone levels: {0}")]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Pack(#[from] PackError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    Zone,
    Capital,
    Subzone,
    Dungeon,
}

/// One place of `zone_levels.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeveledPlace {
    pub name: String,
    pub kind: PlaceKind,
    pub levels: [u8; 2],
    #[serde(default)]
    pub subzones: Vec<String>,
    /// The zone outside a dungeon.
    #[serde(default)]
    pub within: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneLevels {
    pub places: Vec<LeveledPlace>,
}

impl ZoneLevels {
    /// # Errors
    ///
    /// Returns an error when the bundled table is broken. A test reads it, so it is not.
    pub fn bundled() -> Result<ZoneLevels, CoverageError> {
        Ok(toml::from_str(BUNDLED)?)
    }
}

/// A place on the path, in the order of the report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stop {
    pub name: String,
    pub kind: PlaceKind,
    pub levels: [u8; 2],
    /// The zone of a subzone, or the zone outside a dungeon.
    pub within: Option<String>,
}

/// The places by their lowest level, then their highest, with each subzone right after
/// its zone. A stable sort keeps the order of the table within one range.
#[must_use]
pub fn leveling_path(table: &ZoneLevels) -> Vec<Stop> {
    let mut places: Vec<&LeveledPlace> = table.places.iter().collect();
    places.sort_by_key(|place| place.levels);
    let mut path = Vec::new();
    for place in places {
        path.push(Stop {
            name: place.name.clone(),
            kind: place.kind,
            levels: place.levels,
            within: place.within.clone(),
        });
        path.extend(place.subzones.iter().map(|subzone| Stop {
            name: subzone.clone(),
            kind: PlaceKind::Subzone,
            levels: place.levels,
            within: Some(place.name.clone()),
        }));
    }
    path
}

/// What a passage of a place is for a player who just arrived there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// The spoiler limit lets it through at once.
    Usable,
    /// It waits for a deed: a foe defeated or a quest done.
    Outcome,
    /// It tells a deed of no known doer, so it never shows.
    Never,
    /// It waits for a meeting with an NPC, or for a visit of a place later on the path.
    Later,
}

/// `stop` is the index of the place on the path. A place of the path up to it counts as
/// visited, and so does the zone around it.
#[must_use]
pub fn reach_of(passage: &Passage, path: &[Stop], stop: usize) -> Reach {
    if passage.depends_on.contains(&Dependency::Unresolved) {
        return Reach::Never;
    }
    if !passage.depends_on.is_empty() {
        return Reach::Outcome;
    }
    let known = |link: &Link| match link {
        Link::Common => true,
        Link::Npc(_) => false,
        Link::Place(name) => is_visited(name, path, stop),
    };
    if passage.links.iter().all(known) {
        Reach::Usable
    } else {
        Reach::Later
    }
}

fn is_visited(name: &str, path: &[Stop], stop: usize) -> bool {
    let Some(here) = path.get(stop) else {
        return false;
    };
    if here.within.as_deref() == Some(name) {
        return true;
    }
    path.iter()
        .position(|place| place.name == name)
        .is_some_and(|found| found <= stop)
}

/// The counts of one place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub usable: usize,
    /// The usable passages of the own page of the place, which an arrival takes first.
    pub own_page: usize,
    pub setups: usize,
    pub outcomes: usize,
    pub never: usize,
    pub later: usize,
    /// The passages of the own pages of the bosses of a dungeon.
    pub bosses: usize,
}

impl Counts {
    /// The passages that wait for a deed, or that a deed makes stale.
    #[must_use]
    pub fn gated(&self) -> usize {
        self.outcomes + self.setups + self.later
    }

    /// No passage that a first arrival can tell: the narrator is silent there.
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.usable == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlaceCoverage {
    #[serde(flatten)]
    pub stop: Stop,
    #[serde(flatten)]
    pub counts: Counts,
    pub silent: bool,
}

/// The counts of the passages of one place of the path.
#[must_use]
pub fn counts_of(passages: &[Passage], path: &[Stop], stop: usize) -> Counts {
    let name = path.get(stop).map(|place| place.name.as_str());
    let mut counts = Counts::default();
    for passage in passages {
        if passage.setup_for.is_some() {
            counts.setups += 1;
        }
        match reach_of(passage, path, stop) {
            Reach::Usable => {
                counts.usable += 1;
                if passage.about.as_deref() == name {
                    counts.own_page += 1;
                }
            }
            Reach::Outcome => counts.outcomes += 1,
            Reach::Never => counts.never += 1,
            Reach::Later => counts.later += 1,
        }
    }
    counts
}

/// `bosses` holds each known boss with its places (`pack_sources::boss_places`).
///
/// # Errors
///
/// Returns the error of the pack.
pub fn coverage(
    pack: &Pack,
    path: &[Stop],
    bosses: &[(String, Vec<String>)],
) -> Result<Vec<PlaceCoverage>, CoverageError> {
    let mut report = Vec::new();
    for (stop, place) in path.iter().enumerate() {
        let passages = pack.of_place(&place.name)?;
        let mut counts = counts_of(&passages, path, stop);
        for (boss, _) in bosses
            .iter()
            .filter(|(_, places)| places.contains(&place.name))
        {
            counts.bosses += pack.about(boss, MOST_PASSAGES)?.len();
        }
        report.push(PlaceCoverage {
            stop: place.clone(),
            silent: counts.is_silent(),
            counts,
        });
    }
    Ok(report)
}

/// The `count` places with the fewest usable passages, the silent ones first, each group in
/// the order of the path.
#[must_use]
pub fn gaps(report: &[PlaceCoverage], count: usize) -> Vec<&PlaceCoverage> {
    let mut gaps: Vec<&PlaceCoverage> = report.iter().collect();
    gaps.sort_by_key(|place| place.counts.usable);
    gaps.truncate(count);
    gaps
}

const HEADER: &str = "levels  kind      usable  own  setups  outcomes  never  later  bosses  place";

/// One line for each place, in the order of the path, and the gaps at the end.
#[must_use]
pub fn text(report: &[PlaceCoverage], gap_count: usize) -> String {
    let mut text = format!("{HEADER}\n");
    for place in report {
        text.push_str(&line(place));
    }
    let silent = report.iter().filter(|place| place.silent).count();
    let _ = writeln!(
        text,
        "\n{} places, {silent} silent at arrival.\n\nThe {gap_count} biggest gaps:\n{HEADER}",
        report.len()
    );
    for place in gaps(report, gap_count) {
        text.push_str(&line(place));
    }
    text
}

fn line(place: &PlaceCoverage) -> String {
    let [low, high] = place.stop.levels;
    let kind = match place.stop.kind {
        PlaceKind::Zone => "zone",
        PlaceKind::Capital => "capital",
        PlaceKind::Subzone => "subzone",
        PlaceKind::Dungeon => "dungeon",
    };
    let counts = &place.counts;
    let name = match (&place.stop.kind, &place.stop.within) {
        (PlaceKind::Subzone, Some(zone)) => format!("  {} ({zone})", place.stop.name),
        _ => place.stop.name.clone(),
    };
    let silent = if place.silent { "  SILENT" } else { "" };
    format!(
        "{low:>2}-{high:<2}   {kind:<8}  {:>6}  {:>3}  {:>6}  {:>8}  {:>5}  {:>5}  {:>6}  {name}{silent}\n",
        counts.usable,
        counts.own_page,
        counts.setups,
        counts.outcomes,
        counts.never,
        counts.later,
        counts.bosses,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{Deed, Origin, SetupFor};

    fn passage(links: Vec<Link>) -> Passage {
        Passage {
            text: "text".to_string(),
            source: "the wiki page \"Westfall\"".to_string(),
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

    fn path() -> Vec<Stop> {
        let table: ZoneLevels = toml::from_str(
            r#"
            [[places]]
            name = "Westfall"
            kind = "zone"
            levels = [10, 20]
            subzones = ["Moonbrook"]

            [[places]]
            name = "Elwynn Forest"
            kind = "zone"
            levels = [1, 10]

            [[places]]
            name = "The Deadmines"
            kind = "dungeon"
            levels = [17, 26]
            within = "Westfall"

            [[places]]
            name = "Duskwood"
            kind = "zone"
            levels = [18, 30]
            "#,
        )
        .unwrap();
        leveling_path(&table)
    }

    fn stop_of(path: &[Stop], name: &str) -> usize {
        path.iter().position(|stop| stop.name == name).unwrap()
    }

    #[test]
    fn the_path_goes_by_level_with_each_subzone_after_its_zone() {
        let names: Vec<String> = path().into_iter().map(|stop| stop.name).collect();

        assert_eq!(
            names,
            [
                "Elwynn Forest",
                "Westfall",
                "Moonbrook",
                "The Deadmines",
                "Duskwood"
            ]
        );
    }

    #[test]
    fn a_passage_of_the_place_alone_is_usable_at_arrival() {
        let path = path();

        let reach = reach_of(&passage(vec![place("Westfall")]), &path, 1);

        assert_eq!(reach, Reach::Usable);
    }

    #[test]
    fn a_passage_with_an_npc_waits_for_a_meeting() {
        let path = path();
        let links = vec![
            place("Westfall"),
            Link::Npc("Gryan Stoutmantle".to_string()),
        ];

        let reach = reach_of(&passage(links), &path, 1);

        assert_eq!(reach, Reach::Later);
    }

    #[test]
    fn a_passage_that_names_a_later_place_waits_for_it() {
        let path = path();
        let links = vec![place("Westfall"), place("Duskwood")];

        let reach = reach_of(&passage(links), &path, 1);

        assert_eq!(reach, Reach::Later);
    }

    #[test]
    fn a_dungeon_knows_the_zone_outside_it() {
        let path = path();
        let stop = stop_of(&path, "The Deadmines");
        let links = vec![place("The Deadmines"), place("Westfall"), Link::Common];

        let reach = reach_of(&passage(links), &path, stop);

        assert_eq!(reach, Reach::Usable);
    }

    #[test]
    fn an_outcome_passage_waits_for_its_deed() {
        let path = path();
        let mut outcome = passage(vec![place("The Deadmines")]);
        outcome.depends_on = vec![Dependency::Foe("Edwin VanCleef".to_string())];

        let reach = reach_of(&outcome, &path, stop_of(&path, "The Deadmines"));

        assert_eq!(reach, Reach::Outcome);
    }

    #[test]
    fn an_unresolved_passage_never_shows() {
        let path = path();
        let mut unresolved = passage(vec![place("Westfall")]);
        unresolved.depends_on = vec![Dependency::Unresolved];

        assert_eq!(reach_of(&unresolved, &path, 1), Reach::Never);
    }

    #[test]
    fn a_place_with_no_usable_passage_is_silent() {
        let path = path();
        let mut outcome = passage(vec![place("Duskwood")]);
        outcome.depends_on = vec![Dependency::Quest("The Night Watch".to_string())];

        let counts = counts_of(&[outcome], &path, stop_of(&path, "Duskwood"));

        assert!(counts.is_silent());
        assert_eq!(counts.gated(), 1);
    }

    #[test]
    fn a_usable_setup_counts_as_a_setup_and_as_usable() {
        let path = path();
        let stop = stop_of(&path, "The Deadmines");
        let mut setup = passage(vec![place("Westfall")]);
        setup.setup_for = Some(SetupFor {
            deed: Deed::Foe("Edwin VanCleef".to_string()),
            instance: "The Deadmines".to_string(),
        });

        let counts = counts_of(&[setup], &path, stop);

        assert_eq!((counts.usable, counts.setups), (1, 1));
    }

    #[test]
    fn the_own_page_of_a_place_counts_apart() {
        let path = path();
        let mut own = passage(vec![place("Westfall")]);
        own.about = Some("Westfall".to_string());

        let counts = counts_of(&[own, passage(vec![place("Westfall")])], &path, 1);

        assert_eq!((counts.usable, counts.own_page), (2, 1));
    }

    #[test]
    fn the_gaps_put_silent_places_first_in_path_order() {
        let path = path();
        let report: Vec<PlaceCoverage> = path
            .iter()
            .zip([3, 0, 1, 0, 2])
            .map(|(stop, usable)| PlaceCoverage {
                stop: stop.clone(),
                counts: Counts {
                    usable,
                    ..Counts::default()
                },
                silent: usable == 0,
            })
            .collect();

        let names: Vec<&str> = gaps(&report, 3)
            .iter()
            .map(|place| place.stop.name.as_str())
            .collect();

        assert_eq!(names, ["Westfall", "The Deadmines", "Moonbrook"]);
    }
}
