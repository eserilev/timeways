//! `timeways-dev bench-model`: the bench moments for each model, run after run, in scratch
//! worlds, and the results in the dev folder (TESTING.md, "Testing the local model and the
//! frame rate").

use crate::asks::{Ask, asks_of, quiet_moments};
use crate::bench::{BenchError, Played, SETS, play_bench};
use crate::bench_report::{ModelReport, report};
use crate::model_runner::Runner;
use crate::scenario::{Scenario, ScenarioError};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use timeways_story::dev_fps::DEV_BENCH_FOLDER;
use timeways_story::pack::{Pack, PackError};

/// The first call loads the model, so its time counts in no number.
pub const WARM_UP: &str = "Answer with the one word READY.";

#[derive(Debug, Error)]
pub enum BenchModelError {
    #[error("no bench moments named {0}: give v1, or a .jsonl file")]
    NoSet(String),
    #[error("{0}")]
    Scenario(#[from] ScenarioError),
    #[error("the lore pack: {0}")]
    Pack(#[from] PackError),
    #[error(transparent)]
    Bench(#[from] BenchError),
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{0} gave no answer to a first short call. Is it running?")]
    NoAnswer(String),
}

/// The moments, and where each run plays them.
pub struct Bench {
    /// The name of the set, or the file of a set of your own.
    pub set: String,
    pub scenario: Scenario,
    /// The lore pack of the narrator. With none, most moments get no lore and no call.
    pub pack: Option<PathBuf>,
    pub runs: usize,
    /// A folder for the scratch worlds. Each run empties it first.
    pub scratch: PathBuf,
}

impl Bench {
    /// # Errors
    ///
    /// Returns `NoSet` for an unknown name, and the error of the file or of its format.
    pub fn of_set(set: &str, pack: Option<PathBuf>, runs: usize) -> Result<Bench, BenchModelError> {
        let text = set_text(set)?;
        Ok(Bench {
            set: set.to_string(),
            scenario: Scenario::parse(&text)?,
            pack,
            runs,
            scratch: std::env::temp_dir().join(format!("timeways-bench-{}", std::process::id())),
        })
    }

    fn pack(&self) -> Result<Pack, PackError> {
        match &self.pack {
            Some(path) => Pack::open(path),
            None => Pack::empty(),
        }
    }

    /// One play of the moments, in a fresh scratch world. The times end a minute ago, so
    /// the clock check of the story program takes them.
    ///
    /// # Errors
    ///
    /// Returns the error of the pack or of the scratch world.
    pub fn play_once(
        &self,
        runner: &Runner,
        deadline: Option<Instant>,
    ) -> Result<Played, BenchModelError> {
        let _ = std::fs::remove_dir_all(&self.scratch);
        std::fs::create_dir_all(&self.scratch).map_err(|source| BenchModelError::Io {
            path: self.scratch.clone(),
            source,
        })?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let start = now.saturating_sub(self.scenario.span() + 60);
        let scenario = self.scenario.starting_at(start);
        let played = play_bench(&scenario, &self.scratch, self.pack()?, runner, deadline);
        let _ = std::fs::remove_dir_all(&self.scratch);
        Ok(played?)
    }
}

fn set_text(set: &str) -> Result<String, BenchModelError> {
    if let Some((_, _, text)) = SETS.iter().find(|(name, _, _)| *name == set) {
        return Ok((*text).to_string());
    }
    if Path::new(set).extension().is_some_and(|ext| ext == "jsonl") {
        return std::fs::read_to_string(set).map_err(|source| BenchModelError::Io {
            path: PathBuf::from(set),
            source,
        });
    }
    Err(BenchModelError::NoSet(set.to_string()))
}

/// Every run of the moments for one model.
///
/// # Errors
///
/// Returns `NoAnswer` when the model fails its first short call, and the error of a run.
pub fn bench_model(bench: &Bench, runner: &Runner) -> Result<ModelReport, BenchModelError> {
    let warm_up = runner.ask(WARM_UP);
    if warm_up.answer.is_none() {
        return Err(BenchModelError::NoAnswer(runner.name()));
    }
    let mut asks: Vec<Ask> = Vec::new();
    let mut quiet: Vec<String> = Vec::new();
    for _ in 0..bench.runs {
        let played = bench.play_once(runner, None)?;
        asks.extend(asks_of(&played));
        quiet.extend(quiet_moments(&played));
    }
    quiet.sort();
    quiet.dedup();
    Ok(report(
        &runner.name(),
        &bench.set,
        bench.runs,
        Some(warm_up.latency.as_secs_f64()),
        asks,
        quiet,
    ))
}

/// Writes `<stem>.json` and `<stem>.txt` into the dev folder of the story program, and
/// gives both paths.
///
/// # Errors
///
/// Returns the error of the file system.
pub fn write_results(
    story: &Path,
    stem: &str,
    value: &impl Serialize,
    text: &str,
) -> Result<(PathBuf, PathBuf), BenchModelError> {
    let folder = story.join(DEV_BENCH_FOLDER);
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| BenchModelError::Io { path, source }
    };
    std::fs::create_dir_all(&folder).map_err(io(&folder))?;
    let json = folder.join(format!("{stem}.json"));
    let readable = folder.join(format!("{stem}.txt"));
    let body = serde_json::to_string_pretty(value).unwrap_or_default();
    std::fs::write(&json, body).map_err(io(&json))?;
    std::fs::write(&readable, text).map_err(io(&readable))?;
    Ok((json, readable))
}
