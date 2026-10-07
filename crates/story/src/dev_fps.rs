//! A run of `/twdev fps`: the frame rate of the game once a second (TESTING.md, "Testing the
//! local model and the frame rate"). The story program keeps each run as one line of a file
//! in the dev folder, where `timeways-dev bench-fps` reads it. No world holds it.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The folder of the dev benches, in the data folder of the story program.
pub const DEV_BENCH_FOLDER: &str = "dev-bench";
const FPS_FILE: &str = "fps.jsonl";
/// The addon sends at most this many samples. A longer run sends the means of equal groups.
pub const MAX_SAMPLES: usize = 240;
const MAX_LABEL_BYTES: usize = 32;
/// A full file starts over, so runs never fill the disk.
const MAX_FILE_BYTES: u64 = 1 << 20;

/// The numbers are whole, because the JSON of the addon has only whole numbers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FpsRun {
    pub label: String,
    /// `time()` at the start, in seconds since the Unix epoch.
    pub started: u64,
    /// `time()` at the stop. The samples spread evenly from `started` to here.
    pub ended: u64,
    /// Frames per second, once a second, or the means of equal groups in a long run.
    pub samples: Vec<u32>,
    /// The samples that the game hid (`issecretvalue`). They count in no number.
    pub hidden: u32,
    pub min: u32,
    pub p5: u32,
    pub median: u32,
    pub mean: u32,
    /// The memory of Timeways at the stop, in KB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_kb: Option<u64>,
    /// The CPU time of Timeways in the run, in ms. Only the `scriptProfile` cvar gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_ms: Option<u64>,
}

impl FpsRun {
    /// A label of plain letters, digits, `-`, and `_`, a stop after the start, and a sample
    /// list that the addon can send.
    #[must_use]
    pub fn is_sane(&self) -> bool {
        let plain_label = !self.label.is_empty()
            && self.label.len() <= MAX_LABEL_BYTES
            && self
                .label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        plain_label && self.started <= self.ended && self.samples.len() <= MAX_SAMPLES
    }
}

/// The file of the runs, in the data folder of the story program.
#[must_use]
pub fn fps_file(story_folder: &Path) -> PathBuf {
    story_folder.join(DEV_BENCH_FOLDER).join(FPS_FILE)
}

/// Adds the run as one JSON line at the end of the file.
///
/// # Errors
///
/// Returns the error of the file system.
pub fn keep_run(story_folder: &Path, run: &FpsRun) -> io::Result<()> {
    let file = fps_file(story_folder);
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    if fs::metadata(&file).is_ok_and(|meta| meta.len() > MAX_FILE_BYTES) {
        fs::remove_file(&file)?;
    }
    let line = serde_json::to_string(run).map_err(io::Error::other)?;
    let mut out = OpenOptions::new().create(true).append(true).open(&file)?;
    writeln!(out, "{line}")
}

/// Every run of the file that reads, oldest first. A line that does not read is skipped.
#[must_use]
pub fn read_runs(text: &str) -> Vec<FpsRun> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<FpsRun>(line).ok())
        .filter(FpsRun::is_sane)
        .collect()
}
