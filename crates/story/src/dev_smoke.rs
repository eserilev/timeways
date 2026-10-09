//! A run of `/twdev smoke`: one command in the game runs many features, and the story
//! program writes what worked into one log of the dev folder (TESTING.md, "One command to
//! test everything"). The lines land only in dev mode, and in no world.
//!
//! The addon sends one `dev_smoke` line for each step: what it saw itself. The story program
//! adds what only the desktop sees: the input lines that landed, the new facts of the world,
//! the model calls and how they ended, and the spoiler gate. A `dev_smoke_done` line ends the
//! run with its summary.

use crate::dev_fps::DEV_BENCH_FOLDER;
use crate::house::first_chars;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The most steps of one run. The addon has fewer, so a run never fills the disk.
pub const MAX_STEPS: u32 = 120;
pub const MAX_NAME_BYTES: usize = 40;
/// The addon cuts a reason and a text of the desktop to these sizes.
pub const MAX_REASON_BYTES: usize = 160;
pub const MAX_TEXT_BYTES: usize = 300;
pub const MAX_ERROR_BYTES: usize = 300;
const MAX_EXPECTED: usize = 6;
const MAX_ERRORS: usize = 5;
/// A log is cut at this size.
pub const MAX_LOG_BYTES: u64 = 512 * 1024;
/// The desk line names at most this many kinds of kept lines, and of calls.
const MAX_LISTED: usize = 8;
const MAX_REFUSAL_CHARS: usize = 90;

const LOG_PREFIX: &str = "smoke-";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    /// The step had no answer in its time, or a part that it needs is off.
    Wait,
}

impl Verdict {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Wait => "WAIT",
        }
    }
}

/// What a `/lore` step expects of the spoiler gate: the lore of the death of its subject is
/// blocked before the kill, and open after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateExpect {
    Blocked,
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateAsk {
    pub subject: String,
    pub expect: GateExpect,
}

/// One step, as the addon saw it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeStep {
    /// `time()` at the start of the run. It names the files of the run.
    pub run: u64,
    pub step: u32,
    pub name: String,
    pub result: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// What came back from the desktop: a narrator line, a talk answer, or a lore answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The kinds of the input lines that the step sent, such as `zone_entered`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expect: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateAsk>,
}

impl SmokeStep {
    /// Small fields only, so a line of the run stays far inside the limits of the bridge.
    #[must_use]
    pub fn is_sane(&self) -> bool {
        let name_ok = is_token(&self.name, MAX_NAME_BYTES);
        let expect_ok = self.expect.len() <= MAX_EXPECTED
            && self
                .expect
                .iter()
                .all(|kind| is_token(kind, MAX_NAME_BYTES));
        let gate_ok = self
            .gate
            .as_ref()
            .is_none_or(|gate| is_text(&gate.subject, MAX_NAME_BYTES * 2));
        self.run > 0
            && self.step <= MAX_STEPS
            && name_ok
            && expect_ok
            && gate_ok
            && self
                .reason
                .as_deref()
                .is_none_or(|reason| is_text(reason, MAX_REASON_BYTES))
            && self
                .text
                .as_deref()
                .is_none_or(|text| is_text(text, MAX_TEXT_BYTES))
    }
}

/// Lower case letters, digits, `-`, and `_`: a name of a step or of a kind of line.
fn is_token(text: &str, most: usize) -> bool {
    !text.is_empty()
        && text.len() <= most
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn is_text(text: &str, most: usize) -> bool {
    !text.is_empty() && text.len() <= most && !text.chars().any(char::is_control)
}

/// Why a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stopped {
    /// Every step ran.
    Done,
    /// The run reached its most time.
    Timeout,
    /// The player typed `/twdev smoke stop`.
    Stopped,
}

/// The frame rate of the run, as `/twdev fps` measures it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FpsNumbers {
    pub samples: u32,
    pub hidden: u32,
    pub min: u32,
    pub p5: u32,
    pub median: u32,
    pub mean: u32,
    /// The cap of `maxFPSBk` when the run sat at it: the game window was in the background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_cap: Option<u32>,
}

/// The end of a run, as the addon counted it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeDone {
    pub run: u64,
    pub steps: u32,
    pub passed: u32,
    pub failed: u32,
    pub waited: u32,
    pub seconds: u64,
    pub stopped: Stopped,
    /// The Lua errors of the run, newest last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<FpsNumbers>,
}

impl SmokeDone {
    #[must_use]
    pub fn is_sane(&self) -> bool {
        self.run > 0
            && self.steps <= MAX_STEPS
            && self.errors.len() <= MAX_ERRORS
            && self
                .errors
                .iter()
                .all(|error| is_text(error, MAX_ERROR_BYTES))
    }
}

/// A model call of the run, as the database of the world keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSeen {
    pub kind: String,
    /// "open", "accepted", "refused", or "failed".
    pub result: String,
    /// Why the try before this one was refused, when this call is its retry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_of: Option<String>,
}

/// The outcome passages that wait for the defeat of the subject: how many the gate opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateSeen {
    pub subject: String,
    pub open: u32,
    pub blocked: u32,
    /// The world held the defeat before the run, so the gate is open by right.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub defeated_before_the_run: bool,
}

/// What the desktop saw for one step, since the step before it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Desk {
    /// The kinds of the input lines that landed, with their counts.
    pub kept: BTreeMap<String, u32>,
    /// The new events of the world.
    pub facts: u64,
    /// The calls that began, or that ended, since the step before.
    pub calls: Vec<CallSeen>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateSeen>,
}

/// The verdict of the desktop on a step, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeskVerdict {
    pub result: Verdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A missing kind of line fails the step, and waits when the game waited too: then the step
/// sent nothing. The gate fails only on lore that shows too soon, or that stays shut after
/// the deed. A pack with no such lore can't tell.
#[must_use]
pub fn judge(step: &SmokeStep, desk: &Desk) -> DeskVerdict {
    let missing: Vec<&str> = step
        .expect
        .iter()
        .filter(|kind| !desk.kept.contains_key(*kind))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        let result = if step.result == Verdict::Wait {
            Verdict::Wait
        } else {
            Verdict::Fail
        };
        return verdict(result, format!("no {} landed", missing.join(", ")));
    }
    let (Some(ask), Some(seen)) = (&step.gate, &desk.gate) else {
        return DeskVerdict {
            result: Verdict::Pass,
            reason: None,
        };
    };
    judge_gate(ask.expect, seen)
}

fn judge_gate(expect: GateExpect, seen: &GateSeen) -> DeskVerdict {
    let subject = &seen.subject;
    if seen.open + seen.blocked == 0 {
        let why = format!("the pack has no lore that waits for the defeat of {subject}");
        return verdict(Verdict::Wait, why);
    }
    match expect {
        GateExpect::Blocked if seen.defeated_before_the_run => verdict(
            Verdict::Wait,
            format!(
                "{subject} was already defeated in this world; \
                 run timeways-dev smoke --prepare for this check"
            ),
        ),
        GateExpect::Blocked if seen.open > 0 => verdict(
            Verdict::Fail,
            format!("lore of the defeat of {subject} shows before it"),
        ),
        GateExpect::Open if seen.open == 0 => verdict(
            Verdict::Fail,
            format!("lore of the defeat of {subject} stays shut after it"),
        ),
        _ => DeskVerdict {
            result: Verdict::Pass,
            reason: None,
        },
    }
}

fn verdict(result: Verdict, reason: String) -> DeskVerdict {
    DeskVerdict {
        result,
        reason: Some(reason),
    }
}

/// A step with the check of the desktop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Logged {
    pub step: SmokeStep,
    pub desk: Desk,
    pub verdict: DeskVerdict,
}

/// The calls of the whole run: for each kind, the count of each result, and the reasons
/// of the refused tries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallTally {
    pub results: BTreeMap<String, BTreeMap<String, u32>>,
    pub refusals: BTreeMap<String, u32>,
}

impl CallTally {
    #[must_use]
    pub fn of(calls: &[CallSeen]) -> CallTally {
        let mut tally = CallTally::default();
        for call in calls {
            let kind = tally.results.entry(call.kind.clone()).or_default();
            *kind.entry(call.result.clone()).or_default() += 1;
            if let Some(reason) = &call.retry_of {
                let key = format!("{}: {}", call.kind, refusal_text(reason));
                *tally.refusals.entry(key).or_default() += 1;
            }
        }
        tally
    }
}

/// Everything of a run, for the `.json` beside the log.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeReport {
    pub run: u64,
    pub steps: Vec<Logged>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done: Option<SmokeDone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls: Option<CallTally>,
}

impl SmokeReport {
    /// The steps stay in the order of the run, also when the outbox of the game sends one
    /// late.
    pub fn add(&mut self, logged: Logged) {
        let place = self
            .steps
            .partition_point(|before| before.step.step <= logged.step.step);
        self.steps.insert(place, logged);
    }
}

/// The line of the game at the start of a run on a world that already holds a defeat: the
/// checks of the gate can't tell then.
pub const NOT_FRESH: &str = "This world isn't fresh, so a few checks will wait. \
     For a clean run, first run timeways-dev smoke --prepare on your computer.";

// The text of the log ---------------------------------------------------------------------

#[must_use]
pub fn header(run: u64) -> String {
    format!(
        "Timeways smoke test, run {run}\n\
         Each step: the verdict of the game, then the check of the desktop.\n\n"
    )
}

/// Two lines: what the game saw, and what the desktop saw.
#[must_use]
pub fn step_text(logged: &Logged) -> String {
    let step = &logged.step;
    let mut detail = step.reason.clone().unwrap_or_default();
    if let Some(text) = &step.text {
        if !detail.is_empty() {
            detail.push_str("  ");
        }
        detail.push('"');
        detail.push_str(text);
        detail.push('"');
    }
    let game = format!(
        "{} {:>3} {:<24} {detail}",
        step.result.label(),
        step.step,
        step.name
    );
    format!(
        "{}\n           desk {}: {}\n",
        game.trim_end(),
        logged.verdict.result.label(),
        desk_text(&logged.desk, &logged.verdict)
    )
}

fn desk_text(desk: &Desk, verdict: &DeskVerdict) -> String {
    let mut parts: Vec<String> = verdict.reason.iter().cloned().collect();
    parts.push(kept_text(&desk.kept));
    parts.push(format!("facts +{}", desk.facts));
    parts.push(calls_text(&desk.calls));
    if let Some(gate) = &desk.gate {
        parts.push(format!(
            "gate {}: {} open, {} blocked",
            gate.subject, gate.open, gate.blocked
        ));
    }
    parts.join("; ")
}

fn kept_text(kept: &BTreeMap<String, u32>) -> String {
    if kept.is_empty() {
        return "kept nothing".to_string();
    }
    let mut named: Vec<String> = kept
        .iter()
        .take(MAX_LISTED)
        .map(|(kind, count)| counted(kind, *count))
        .collect();
    if kept.len() > MAX_LISTED {
        named.push(format!("{} more", kept.len() - MAX_LISTED));
    }
    format!("kept {}", named.join(", "))
}

fn counted(name: &str, count: u32) -> String {
    if count == 1 {
        name.to_string()
    } else {
        format!("{name} x{count}")
    }
}

fn calls_text(calls: &[CallSeen]) -> String {
    if calls.is_empty() {
        return "no calls".to_string();
    }
    let mut named: Vec<String> = calls.iter().take(MAX_LISTED).map(call_text).collect();
    if calls.len() > MAX_LISTED {
        named.push(format!("{} more", calls.len() - MAX_LISTED));
    }
    format!("calls {}", named.join(", "))
}

fn call_text(call: &CallSeen) -> String {
    match &call.retry_of {
        Some(reason) => format!(
            "{} {} (retry after \"{}\")",
            call.kind,
            call.result,
            refusal_text(reason)
        ),
        None => format!("{} {}", call.kind, call.result),
    }
}

fn refusal_text(reason: &str) -> &str {
    first_chars(reason, MAX_REFUSAL_CHARS)
}

/// The end of the log: the counts, what failed, the Lua errors, the calls, and the fps.
#[must_use]
pub fn summary(steps: &[Logged], done: &SmokeDone, calls: &CallTally) -> String {
    let count = |verdict: Verdict, of: fn(&Logged) -> Verdict| {
        steps.iter().filter(|logged| of(logged) == verdict).count()
    };
    let game = |logged: &Logged| logged.step.result;
    let desk = |logged: &Logged| logged.verdict.result;
    let mut lines = vec![
        String::new(),
        format!(
            "Summary: {} steps in {} s ({}).",
            steps.len(),
            done.seconds,
            stopped_text(done.stopped)
        ),
        format!(
            "Game: {} passed, {} failed, {} waited.",
            count(Verdict::Pass, game),
            count(Verdict::Fail, game),
            count(Verdict::Wait, game)
        ),
        format!(
            "Desk: {} passed, {} failed, {} waited.",
            count(Verdict::Pass, desk),
            count(Verdict::Fail, desk),
            count(Verdict::Wait, desk)
        ),
    ];
    if usize::try_from(done.steps).ok() != Some(steps.len()) {
        lines.push(format!(
            "The game ran {} steps, and {} reached the desktop so far. \
             The log puts the others in their place when they come.",
            done.steps,
            steps.len()
        ));
    }
    lines.push(failed_text(steps));
    lines.push(errors_text(&done.errors));
    lines.extend(tally_lines(calls));
    lines.push(fps_text(done.fps));
    lines.join("\n") + "\n"
}

fn stopped_text(stopped: Stopped) -> &'static str {
    match stopped {
        Stopped::Done => "every step ran",
        Stopped::Timeout => "it reached its most time",
        Stopped::Stopped => "stopped by hand",
    }
}

fn failed_text(steps: &[Logged]) -> String {
    let failed: Vec<String> = steps
        .iter()
        .filter(|logged| {
            logged.step.result == Verdict::Fail || logged.verdict.result == Verdict::Fail
        })
        .map(|logged| format!("{} {}", logged.step.step, logged.step.name))
        .collect();
    if failed.is_empty() {
        return "Failed: none".to_string();
    }
    format!("Failed: {}", failed.join(", "))
}

fn errors_text(errors: &[String]) -> String {
    if errors.is_empty() {
        return "Lua errors: none".to_string();
    }
    let quoted: Vec<String> = errors.iter().map(|error| format!("  {error}")).collect();
    format!("Lua errors: {}\n{}", errors.len(), quoted.join("\n"))
}

fn tally_lines(calls: &CallTally) -> Vec<String> {
    if calls.results.is_empty() {
        return vec!["Model calls: none".to_string()];
    }
    let kinds: Vec<String> = calls
        .results
        .iter()
        .map(|(kind, results)| {
            let total: u32 = results.values().sum();
            let each: Vec<String> = results
                .iter()
                .map(|(result, count)| format!("{count} {result}"))
                .collect();
            format!("{kind} {total} ({})", each.join(", "))
        })
        .collect();
    let mut lines = vec![format!("Model calls: {}", kinds.join("; "))];
    if calls.refusals.is_empty() {
        lines.push("Refusals: none".to_string());
    }
    for (reason, count) in &calls.refusals {
        lines.push(format!("Refused: {reason} ({count})"));
    }
    lines
}

fn fps_text(fps: Option<FpsNumbers>) -> String {
    let Some(fps) = fps else {
        return "FPS: no samples".to_string();
    };
    let numbers = format!(
        "FPS: {} samples ({} hidden): min {}, low 5% {}, median {}, mean {}.",
        fps.samples, fps.hidden, fps.min, fps.p5, fps.median, fps.mean
    );
    match fps.background_cap {
        Some(cap) => format!(
            "{numbers} The game window was in the background, \
             so maxFPSBk capped it at {cap}: not a real number."
        ),
        None => numbers,
    }
}

// The files of a run ------------------------------------------------------------------------

#[must_use]
pub fn log_file(story_folder: &Path, run: u64) -> PathBuf {
    story_folder
        .join(DEV_BENCH_FOLDER)
        .join(format!("{LOG_PREFIX}{run}.log"))
}

#[must_use]
pub fn json_file(story_folder: &Path, run: u64) -> PathBuf {
    log_file(story_folder, run).with_extension("json")
}

/// The whole log of a run: the steps, then the summary once the run ended. The file is
/// written new for each step, so a step that comes late still goes before the summary.
#[must_use]
pub fn log_text(report: &SmokeReport) -> String {
    let mut text = header(report.run);
    for logged in &report.steps {
        text.push_str(&step_text(logged));
    }
    if let (Some(done), Some(calls)) = (&report.done, &report.calls) {
        text.push_str(&summary(&report.steps, done, calls));
    }
    text
}

/// Writes the log, cut to its most size.
///
/// # Errors
///
/// Returns the error of the file system.
pub fn write_log(file: &Path, text: &str) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(file, first_bytes(text, MAX_LOG_BYTES))
}

/// The longest start of the text within `most` bytes that ends on a whole character.
fn first_bytes(text: &str, most: u64) -> &str {
    let most = usize::try_from(most).unwrap_or(usize::MAX);
    if text.len() <= most {
        return text;
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// # Errors
///
/// Returns the error of the file system.
pub fn write_report(file: &Path, report: &SmokeReport) -> io::Result<()> {
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let text = serde_json::to_string_pretty(report).map_err(io::Error::other)?;
    fs::write(file, text)
}

/// The log of the newest run in the dev folder, or None.
#[must_use]
pub fn newest_log(story_folder: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(story_folder.join(DEV_BENCH_FOLDER)).ok()?;
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| run_of(&entry.file_name().to_string_lossy()).map(|run| (run, entry)))
        .max_by_key(|(run, _)| *run)
        .map(|(_, entry)| entry.path())
}

/// 1790000000 for "smoke-1790000000.log".
fn run_of(file_name: &str) -> Option<u64> {
    file_name
        .strip_prefix(LOG_PREFIX)?
        .strip_suffix(".log")?
        .parse()
        .ok()
}
