//! `timeways-dev bench-fps`: what a model costs the frame rate of the game (TESTING.md,
//! "Testing the local model and the frame rate").
//!
//! The desktop cannot tell the addon when a phase starts: the bridge only answers batches.
//! So the player starts `/twdev fps start bench` first, and the addon samples once a second
//! with the time of the computer. The bench keeps the time of each phase: idle (baseline),
//! the bench moments back to back (load), and idle again (recovery). At `/twdev fps stop`,
//! the run lands in the dev folder, and the bench splits its samples by those times.

use crate::asks::asks_of;
use crate::bench_model::{Bench, BenchModelError, WARM_UP};
use crate::model_runner::Runner;
use crate::proc_stats::{GpuSource, Sample, Sampler};
use crate::stats::{Spread, drop_percent, spread};
use serde::Serialize;
use std::fmt::Write;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use timeways_story::dev_fps::{FpsRun, fps_file, read_runs};

const POLL: Duration = Duration::from_secs(2);

/// One phase of the timeline, in seconds since the Unix epoch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Phase {
    pub name: String,
    pub from: u64,
    pub to: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PhaseReport {
    pub phase: Phase,
    pub fps: Option<Spread>,
    /// How much lower than in the baseline, in percent.
    pub median_drop_percent: Option<f64>,
    pub mean_drop_percent: Option<f64>,
    pub cpu_percent: Option<Spread>,
    pub memory_mb: Option<Spread>,
    pub gpu_percent: Option<Spread>,
    pub gpu_memory_mb: Option<Spread>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FpsReport {
    pub model: String,
    pub seconds: u64,
    /// The run of `/twdev fps` that covers the load, if one came.
    pub run: Option<FpsRun>,
    pub phases: Vec<PhaseReport>,
    /// The seconds of each model call in the load.
    pub load_latency: Option<Spread>,
    pub load_calls: usize,
    pub gpu_source: GpuSource,
}

pub struct FpsBench {
    pub bench: Bench,
    pub runner: Runner,
    /// The seconds of each phase.
    pub seconds: u64,
    /// The seconds to wait before the baseline, for the player to start the run.
    pub lead: u64,
    /// The seconds to wait for the run after the recovery.
    pub wait: u64,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The samples of the run inside `[from, to)`. They spread evenly from its start to its
/// stop, so a second that the game skipped moves no sample far.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "times and counts are far below 2^52"
)]
pub fn samples_in(run: &FpsRun, from: u64, to: u64) -> Vec<f64> {
    let count = run.samples.len();
    if count == 0 {
        return Vec::new();
    }
    let span = run.ended.saturating_sub(run.started) as f64;
    run.samples
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let at = run.started as f64 + (*index as f64 + 0.5) * span / count as f64;
            (from as f64) <= at && at < to as f64
        })
        .map(|(_, fps)| f64::from(*fps))
        .collect()
}

/// The newest run that started before the load and stopped after it.
#[must_use]
pub fn covering_run<'r>(runs: &'r [FpsRun], load: &Phase) -> Option<&'r FpsRun> {
    runs.iter()
        .rev()
        .find(|run| run.started <= load.from && run.ended >= load.to)
}

fn usage_in(
    samples: &[Sample],
    phase: &Phase,
    value: impl Fn(&Sample) -> Option<f64>,
) -> Option<Spread> {
    let values: Vec<f64> = samples
        .iter()
        .filter(|sample| phase.from < sample.at && sample.at <= phase.to)
        .filter_map(value)
        .collect();
    spread(&values)
}

/// The numbers of each phase. The first phase is the baseline of the drops.
#[must_use]
pub fn phase_reports(phases: &[Phase], run: Option<&FpsRun>, usage: &[Sample]) -> Vec<PhaseReport> {
    let fps_of = |phase: &Phase| run.and_then(|run| spread(&samples_in(run, phase.from, phase.to)));
    let baseline = phases.first().and_then(fps_of);
    phases
        .iter()
        .map(|phase| {
            let fps = fps_of(phase);
            let drop = |pick: fn(&Spread) -> f64| drop_percent(pick(&baseline?), pick(&fps?));
            PhaseReport {
                phase: phase.clone(),
                fps,
                median_drop_percent: drop(|s| s.median),
                mean_drop_percent: drop(|s| s.mean),
                cpu_percent: usage_in(usage, phase, |s| Some(s.cpu_percent)),
                memory_mb: usage_in(usage, phase, |s| Some(s.memory_mb)),
                gpu_percent: usage_in(usage, phase, |s| s.gpu_percent),
                gpu_memory_mb: usage_in(usage, phase, |s| s.gpu_memory_mb),
            }
        })
        .collect()
}

/// The timeline, then the wait for the run of the addon. `say` tells the player each step.
///
/// # Errors
///
/// Returns `NoAnswer` when the model fails its first short call, and the error of a play.
pub fn bench_fps(
    fps: &FpsBench,
    story: &Path,
    say: &mut dyn FnMut(&str),
) -> Result<FpsReport, BenchModelError> {
    // The model loads before the baseline, so its load never counts as its cost.
    if fps.runner.ask(WARM_UP).answer.is_none() {
        return Err(BenchModelError::NoAnswer(fps.runner.name()));
    }
    say(&format!(
        "In the game, type /twdev fps start bench now. The baseline starts in {} s.",
        fps.lead
    ));
    std::thread::sleep(Duration::from_secs(fps.lead));
    let sampler = Sampler::start(fps.runner.process_names());
    let source = sampler.source;
    say(&format!("Baseline: {} s idle.", fps.seconds));
    let baseline = idle_phase("baseline", fps.seconds);
    say(&format!("Load: {} s of model calls.", fps.seconds));
    let loaded = load_phase(fps);
    say(&format!("Recovery: {} s idle.", fps.seconds));
    let recovery = idle_phase("recovery", fps.seconds);
    let usage = sampler.stop();
    let (load, latencies) = loaded?;
    say("Done. In the game, type /twdev fps stop.");
    let run = wait_for_run(story, &load, fps.wait);
    if run.is_none() {
        say("No run of /twdev fps covers the load, so the report has no frame rate.");
    }
    let phases = [baseline, load, recovery];
    Ok(FpsReport {
        model: fps.runner.name(),
        seconds: fps.seconds,
        phases: phase_reports(&phases, run.as_ref(), &usage),
        run,
        load_calls: latencies.len(),
        load_latency: spread(&latencies),
        gpu_source: source,
    })
}

fn idle_phase(name: &str, seconds: u64) -> Phase {
    let from = unix_now();
    std::thread::sleep(Duration::from_secs(seconds));
    Phase {
        name: name.to_string(),
        from,
        to: unix_now(),
    }
}

/// The bench moments back to back. A call after the end of the phase fails at once, so the
/// phase ends with the call that runs at its end. Gives the seconds of each call.
fn load_phase(fps: &FpsBench) -> Result<(Phase, Vec<f64>), BenchModelError> {
    let from = unix_now();
    let deadline = Instant::now() + Duration::from_secs(fps.seconds);
    let mut latencies = Vec::new();
    while Instant::now() < deadline {
        let played = fps.bench.play_once(&fps.runner, Some(deadline))?;
        latencies.extend(load_latencies(&played));
    }
    let phase = Phase {
        name: "load".to_string(),
        from,
        to: unix_now(),
    };
    Ok((phase, latencies))
}

fn load_latencies(played: &crate::bench::Played) -> Vec<f64> {
    asks_of(played)
        .iter()
        .flat_map(|ask| &ask.attempts)
        .filter(|attempt| attempt.answer.is_some())
        .map(|attempt| attempt.latency_seconds)
        .collect()
}

/// The run arrives a few seconds after `/twdev fps stop`.
fn wait_for_run(story: &Path, load: &Phase, wait: u64) -> Option<FpsRun> {
    let until = Instant::now() + Duration::from_secs(wait);
    loop {
        let text = std::fs::read_to_string(fps_file(story)).unwrap_or_default();
        let runs = read_runs(&text);
        if let Some(run) = covering_run(&runs, load) {
            return Some(run.clone());
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(POLL);
    }
}

fn one(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |value| format!("{value:.1}"))
}

/// The report as a person reads it.
#[must_use]
pub fn fps_text(report: &FpsReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Model: {}", report.model);
    let _ = writeln!(
        out,
        "Each phase: {} s. GPU numbers from: {:?}\n",
        report.seconds, report.gpu_source
    );
    let _ = writeln!(
        out,
        "phase      fps min   p5  median  mean  drop med  drop mean   cpu %  mem MB   gpu %  gpu MB"
    );
    for phase in &report.phases {
        let fps = phase.fps;
        let _ = writeln!(
            out,
            "{:10} {:>7} {:>4} {:>7} {:>5} {:>8}% {:>9}% {:>7} {:>7} {:>7} {:>7}",
            phase.phase.name,
            one(fps.map(|s| s.min)),
            one(fps.map(|s| s.p5)),
            one(fps.map(|s| s.median)),
            one(fps.map(|s| s.mean)),
            one(phase.median_drop_percent),
            one(phase.mean_drop_percent),
            one(phase.cpu_percent.map(|s| s.mean)),
            one(phase.memory_mb.map(|s| s.max)),
            one(phase.gpu_percent.map(|s| s.mean)),
            one(phase.gpu_memory_mb.map(|s| s.max)),
        );
    }
    let latency = report.load_latency;
    let _ = writeln!(
        out,
        "\nModel calls in the load: {}, p50 {} s, p95 {} s.",
        report.load_calls,
        one(latency.map(|s| s.median)),
        one(latency.map(|s| s.p95)),
    );
    match &report.run {
        Some(run) => {
            let memory = run
                .memory_kb
                .map_or("hidden".to_string(), |kb| format!("{kb} KB"));
            let cpu = run
                .cpu_ms
                .map_or("off (scriptProfile)".to_string(), |ms| format!("{ms} ms"));
            let _ = writeln!(
                out,
                "FPS run \"{}\": {} samples, {} hidden. Timeways memory {memory}, CPU {cpu}.",
                run.label,
                run.samples.len(),
                run.hidden
            );
        }
        None => {
            let _ = writeln!(out, "No run of /twdev fps covers the load.");
        }
    }
    out
}
