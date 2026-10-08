//! Builds a lore pack (GAMEPLAY.md 5.10), in one of two ways.
//!
//! From passages, one JSON object per line:
//!
//! ```text
//! {"text": "...", "source": "https://...", "places": ["Goldshire"], "npcs": ["Innkeeper Farley"]}
//! {"text": "...", "source": "https://...", "places": ["Goldshire"], "about": "Goldshire"}
//! {"text": "...", "source": "https://...", "common": true}
//! {"text": "...", "source": "https://...", "places": ["The Deadmines"], "depends_on": {"foe": "Edwin VanCleef"}}
//! {"text": "...", "source": "https://...", "places": ["Westfall"], "setup_for": {"foe": "Edwin VanCleef", "instance": "The Deadmines"}}
//! ```
//!
//! `depends_on` is `{"foe": name}`, `{"quest": title}`, or `"unresolved"`. A line that
//! tells a deed of adventurers and has no `depends_on` is unresolved. `setup_for` is
//! `{"foe": name, "instance": place}` or `{"quest": title, "instance": place}`.
//!
//! Or from a MediaWiki dump (`.xml` or `.7z`), with the pages of `data/pack_sources.toml`:
//! `timeways-pack from-dump <dump> <new pack file>`.
//!
//! `timeways-pack coverage <pack> [--json]` reports the passages of each place of the
//! leveling path, and the places where the narrator stays silent (`pack_coverage`).

use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use timeways_story::outcome_passages::is_outcome;
use timeways_story::pack::{Deed, Dependency, Link, Origin, Pack, Passage, SetupFor};
use timeways_story::pack_coverage::{self, ZoneLevels, leveling_path};
use timeways_story::pack_sources::{self, Built, Outcome, Sources};
use timeways_story::passage_limits;

const USAGE: &str = "usage: timeways-pack <passages.jsonl> <new pack file>
       timeways-pack from-dump <wiki dump .xml or .7z> <new pack file>
       timeways-pack coverage <pack> [--json]";

/// The gaps at the end of the text report.
const GAPS: usize = 20;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PassageLine {
    text: String,
    source: String,
    #[serde(default)]
    places: Vec<String>,
    #[serde(default)]
    npcs: Vec<String>,
    #[serde(default)]
    common: bool,
    /// The place or the person that the page of the passage is about.
    #[serde(default)]
    about: Option<String>,
    #[serde(default)]
    depends_on: Option<DependencyLine>,
    #[serde(default)]
    setup_for: Option<SetupLine>,
}

/// One of `foe` and `quest`, and the instance.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetupLine {
    #[serde(default)]
    foe: Option<String>,
    #[serde(default)]
    quest: Option<String>,
    instance: String,
}

impl SetupLine {
    fn into_setup(self) -> Result<SetupFor, String> {
        let deed = match (self.foe, self.quest) {
            (Some(name), None) => Deed::Foe(name),
            (None, Some(title)) => Deed::Quest(title),
            _ => return Err("setup_for needs one of foe and quest".to_string()),
        };
        Ok(SetupFor {
            deed,
            instance: self.instance,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum DependencyLine {
    Foe(String),
    Quest(String),
    Unresolved,
}

impl DependencyLine {
    fn into_dependency(self) -> Dependency {
        match self {
            DependencyLine::Foe(name) => Dependency::Foe(name),
            DependencyLine::Quest(title) => Dependency::Quest(title),
            DependencyLine::Unresolved => Dependency::Unresolved,
        }
    }
}

impl PassageLine {
    fn into_passage(self) -> Result<Passage, String> {
        let places = self.places.into_iter().map(Link::Place);
        let npcs = self.npcs.into_iter().map(Link::Npc);
        let common = self.common.then_some(Link::Common);
        let told_deed = is_outcome(&self.text).then_some(Dependency::Unresolved);
        let depends_on = self
            .depends_on
            .map(DependencyLine::into_dependency)
            .or(told_deed)
            .into_iter()
            .collect();
        let setup_for = self.setup_for.map(SetupLine::into_setup).transpose()?;
        Ok(Passage {
            text: self.text,
            source: self.source,
            links: places.chain(npcs).chain(common).collect(),
            origin: Origin::Pack,
            about: self.about,
            depends_on,
            setup_for,
        })
    }
}

fn main() -> ExitCode {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let result = match args.as_slice() {
        [mode, dump, pack] if mode.as_os_str() == "from-dump" => from_dump(dump, pack),
        [mode, pack] if mode.as_os_str() == "coverage" => print_coverage(pack, Format::Text),
        [mode, pack, json] if mode.as_os_str() == "coverage" && json.as_os_str() == "--json" => {
            print_coverage(pack, Format::Json)
        }
        [passages, pack] => from_lines(passages, pack),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// A pack is never written over, because an old pack with new passages mixes two sources.
fn refuse_existing(pack: &Path) -> Result<(), Box<dyn Error>> {
    if pack.exists() {
        return Err(format!("{} exists already", pack.display()).into());
    }
    Ok(())
}

fn from_lines(passages: &Path, pack: &Path) -> Result<(), Box<dyn Error>> {
    refuse_existing(pack)?;
    let text = fs::read_to_string(passages)?;
    let mut read = Vec::new();
    for (number, line) in text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        let line: PassageLine =
            serde_json::from_str(line).map_err(|error| format!("line {}: {error}", number + 1))?;
        let passage = line
            .into_passage()
            .map_err(|error| format!("line {}: {error}", number + 1))?;
        if let Some(fault) = passage_limits::fault(&passage) {
            return Err(format!("line {}: {fault}", number + 1).into());
        }
        read.push(passage);
    }
    Pack::write(pack, &read)?;
    println!("wrote {} passages to {}", read.len(), pack.display());
    Ok(())
}

fn from_dump(dump: &Path, pack: &Path) -> Result<(), Box<dyn Error>> {
    refuse_existing(pack)?;
    let built = pack_sources::from_dump(dump, &Sources::bundled()?)?;
    print_report(&built);
    print_outcomes(&built.passages);
    print_setups(&built.passages);
    for passage in &built.passages {
        if let Some(fault) = passage_limits::fault(passage) {
            return Err(format!("the passage from {}: {fault}", passage.source).into());
        }
    }
    Pack::write(pack, &built.passages)?;
    println!(
        "wrote {} passages to {}",
        built.passages.len(),
        pack.display()
    );
    Ok(())
}

/// Each outcome passage with each deed that it tells, for a check by eye. The last line
/// counts the tags of each kind, and the passages with two deeds or more.
fn print_outcomes(passages: &[Passage]) {
    let mut counts = [0usize; 3];
    let mut tagged = 0;
    let mut with_two = 0;
    for passage in passages
        .iter()
        .filter(|passage| !passage.depends_on.is_empty())
    {
        tagged += 1;
        if passage.depends_on.len() > 1 {
            with_two += 1;
        }
        let mut shown = Vec::new();
        for dependency in &passage.depends_on {
            let (index, tag) = match dependency {
                Dependency::Foe(name) => (0, format!("foe  {name}")),
                Dependency::Quest(title) => (1, format!("quest  {title}")),
                Dependency::Unresolved => (2, "unresolved".to_string()),
            };
            counts[index] += 1;
            shown.push(tag);
        }
        println!(
            "outcome  {}  |  {}  |  {}",
            shown.join(" + "),
            passage.source,
            passage.text
        );
    }
    let [foes, quests, unresolved] = counts;
    println!(
        "tagged {tagged} outcome passages, {with_two} with two deeds or more: \
         {foes} tags by foe, {quests} by quest, {unresolved} unresolved"
    );
}

/// Each setup passage with its deed and its instance, for a check by eye, and the count of
/// each instance.
fn print_setups(passages: &[Passage]) {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for passage in passages {
        let Some(setup) = &passage.setup_for else {
            continue;
        };
        let (kind, name) = match &setup.deed {
            Deed::Foe(name) => ("foe", name.as_str()),
            Deed::Quest(title) => ("quest", title.as_str()),
        };
        println!(
            "setup  {}  {kind}  {name}  |  {}  |  {}",
            setup.instance, passage.source, passage.text
        );
        match counts
            .iter_mut()
            .find(|(instance, _)| *instance == setup.instance)
        {
            Some((_, count)) => *count += 1,
            None => counts.push((&setup.instance, 1)),
        }
    }
    let total: usize = counts.iter().map(|(_, count)| count).sum();
    let each: Vec<String> = counts
        .iter()
        .map(|(instance, count)| format!("{instance} {count}"))
        .collect();
    println!("tagged {total} setup passages: {}", each.join(", "));
}

fn print_report(built: &Built) {
    for chapter in &built.missing_chapters {
        println!("missing chapter: {chapter}");
    }
    for line in &built.report {
        match line.outcome {
            Outcome::Read {
                passages,
                later: 0,
                game: 0,
                cut: 0,
            } => println!("{passages:>7}  {}", line.title),
            Outcome::Read {
                passages,
                later,
                game,
                cut,
            } => println!(
                "{passages:>7}  {}  (dropped: {later} later, {game} game, {cut} game sentences)",
                line.title
            ),
            Outcome::Missing => println!("missing  {}", line.title),
            Outcome::NoBook => println!("no book  {}", line.title),
        }
    }
    let read = built
        .report
        .iter()
        .filter(|line| matches!(line.outcome, Outcome::Read { .. }))
        .count();
    let skipped = built.report.len() - read;
    let later: usize = built
        .report
        .iter()
        .map(|line| dropped(&line.outcome).0)
        .sum();
    let game: usize = built
        .report
        .iter()
        .map(|line| dropped(&line.outcome).1)
        .sum();
    println!(
        "read {read} pages, skipped {skipped}, dropped {later} later paragraphs and {game} game paragraphs"
    );
}

/// The paragraphs that the later terms and the game terms dropped from one page.
fn dropped(outcome: &Outcome) -> (usize, usize) {
    match outcome {
        Outcome::Read { later, game, .. } => (*later, *game),
        Outcome::Missing | Outcome::NoBook => (0, 0),
    }
}

#[derive(Clone, Copy)]
enum Format {
    Text,
    Json,
}

fn print_coverage(pack: &Path, format: Format) -> Result<(), Box<dyn Error>> {
    let pack = Pack::open(pack)?;
    let path = leveling_path(&ZoneLevels::bundled()?);
    let bosses: Vec<(String, Vec<String>)> = pack_sources::boss_places(&Sources::bundled()?)
        .into_iter()
        .collect();
    let report = pack_coverage::coverage(&pack, &path, &bosses)?;
    match format {
        Format::Text => print!("{}", pack_coverage::text(&report, GAPS)),
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }
    Ok(())
}
