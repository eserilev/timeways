//! Builds a lore pack from passages, one JSON object per line (GAMEPLAY.md 5.10):
//!
//! ```text
//! {"text": "...", "source": "https://...", "places": ["Goldshire"], "npcs": ["Innkeeper Farley"]}
//! ```

use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use timeways_story::pack::{Link, Pack, Passage};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PassageLine {
    text: String,
    source: String,
    #[serde(default)]
    places: Vec<String>,
    #[serde(default)]
    npcs: Vec<String>,
}

impl PassageLine {
    fn into_passage(self) -> Passage {
        let places = self.places.into_iter().map(Link::Place);
        let npcs = self.npcs.into_iter().map(Link::Npc);
        Passage {
            text: self.text,
            source: self.source,
            links: places.chain(npcs).collect(),
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [passages, pack] = args.as_slice() else {
        eprintln!("usage: timeways-pack <passages.jsonl> <new pack file>");
        return ExitCode::FAILURE;
    };
    match build(passages, pack) {
        Ok(count) => {
            println!("wrote {count} passages to {}", pack.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// A pack is never written over: an old pack with new passages would mix two sources.
fn build(passages: &Path, pack: &Path) -> Result<usize, Box<dyn Error>> {
    if pack.exists() {
        return Err(format!("{} exists already", pack.display()).into());
    }
    let text = fs::read_to_string(passages)?;
    let mut read = Vec::new();
    for (number, line) in text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        let line: PassageLine =
            serde_json::from_str(line).map_err(|error| format!("line {}: {error}", number + 1))?;
        read.push(line.into_passage());
    }
    Pack::write(pack, &read)?;
    Ok(read.len())
}
