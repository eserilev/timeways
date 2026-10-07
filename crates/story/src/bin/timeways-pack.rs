//! Builds a lore pack (GAMEPLAY.md 5.10), in one of two ways.
//!
//! From passages, one JSON object per line:
//!
//! ```text
//! {"text": "...", "source": "https://...", "places": ["Goldshire"], "npcs": ["Innkeeper Farley"]}
//! {"text": "...", "source": "https://...", "places": ["Goldshire"], "about": "Goldshire"}
//! {"text": "...", "source": "https://...", "common": true}
//! ```
//!
//! Or from a MediaWiki dump (`.xml` or `.7z`), with the pages of `data/pack_sources.toml`:
//! `timeways-pack from-dump <dump> <new pack file>`.

use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use timeways_story::pack::{Link, Origin, Pack, Passage};
use timeways_story::pack_sources::{self, Built, Outcome, Sources};
use timeways_story::passage_limits;

const USAGE: &str = "usage: timeways-pack <passages.jsonl> <new pack file>
       timeways-pack from-dump <wiki dump .xml or .7z> <new pack file>";

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
}

impl PassageLine {
    fn into_passage(self) -> Passage {
        let places = self.places.into_iter().map(Link::Place);
        let npcs = self.npcs.into_iter().map(Link::Npc);
        let common = self.common.then_some(Link::Common);
        Passage {
            text: self.text,
            source: self.source,
            links: places.chain(npcs).chain(common).collect(),
            origin: Origin::Pack,
            about: self.about,
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let result = match args.as_slice() {
        [mode, dump, pack] if mode.as_os_str() == "from-dump" => from_dump(dump, pack),
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
        let passage = line.into_passage();
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
