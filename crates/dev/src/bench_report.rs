//! The report of `timeways-dev bench-model`: for each moment and kind of call, the rates of
//! the outcomes, the faults, the retries, and the times, and every line that a player would
//! read. One model alone, or several side by side.

use crate::asks::{Ask, Outcome};
use crate::stats::{Spread, spread};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelReport {
    pub model: String,
    /// The version of the bench moments.
    pub set: String,
    pub runs: usize,
    /// The first call, which loads the model. No number below counts it.
    pub warm_up_seconds: Option<f64>,
    /// One group for each moment and kind of call, in the order of the moments.
    pub groups: Vec<Group>,
    pub total: Group,
    /// The moments that made no model call in some run.
    pub quiet: Vec<String>,
    pub asks: Vec<Ask>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Group {
    pub moment: String,
    pub kind: String,
    pub asks: usize,
    pub shown: usize,
    pub silence: usize,
    pub refused: usize,
    pub failed: usize,
    /// The asks that needed a second call.
    pub retries: usize,
    /// The faults of every refused call, by short name.
    pub faults: BTreeMap<String, usize>,
    /// Seconds for each call.
    pub latency: Option<Spread>,
    pub first_byte: Option<Spread>,
    /// The tokens of the answers over the time of their calls, when the runner counts them.
    pub tokens_per_second: Option<f64>,
}

impl Group {
    fn of(moment: &str, kind: &str, asks: &[&Ask]) -> Group {
        let count = |outcome| asks.iter().filter(|ask| ask.outcome == outcome).count();
        let attempts: Vec<_> = asks.iter().flat_map(|ask| &ask.attempts).collect();
        let mut faults = BTreeMap::new();
        for fault in attempts.iter().flat_map(|attempt| &attempt.faults) {
            *faults.entry(fault.clone()).or_insert(0) += 1;
        }
        let latencies: Vec<f64> = attempts.iter().map(|a| a.latency_seconds).collect();
        let first_bytes: Vec<f64> = attempts
            .iter()
            .filter_map(|a| a.first_byte_seconds)
            .collect();
        Group {
            moment: moment.to_string(),
            kind: kind.to_string(),
            asks: asks.len(),
            shown: count(Outcome::Shown),
            silence: count(Outcome::Silence),
            refused: count(Outcome::Refused),
            failed: count(Outcome::Failed),
            retries: asks.iter().filter(|ask| ask.attempts.len() > 1).count(),
            faults,
            latency: spread(&latencies),
            first_byte: spread(&first_bytes),
            tokens_per_second: tokens_per_second(&attempts),
        }
    }

    fn key(&self) -> String {
        format!("{} / {}", self.moment, self.kind)
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a count of tokens is far below 2^52"
)]
fn tokens_per_second(attempts: &[&crate::asks::Attempt]) -> Option<f64> {
    let counted: Vec<_> = attempts.iter().filter(|a| a.tokens.is_some()).collect();
    let tokens: u64 = counted.iter().filter_map(|a| a.tokens).sum();
    let seconds: f64 = counted.iter().map(|a| a.latency_seconds).sum();
    (tokens > 0 && seconds > 0.0).then(|| tokens as f64 / seconds)
}

/// The report of the asks of every run of one model.
#[must_use]
pub fn report(
    model: &str,
    set: &str,
    runs: usize,
    warm_up: Option<f64>,
    asks: Vec<Ask>,
    quiet: Vec<String>,
) -> ModelReport {
    let mut keys: Vec<(String, String)> = Vec::new();
    for ask in &asks {
        let key = (ask.moment.clone(), ask.kind.clone());
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let groups = keys
        .iter()
        .map(|(moment, kind)| {
            let of_key: Vec<&Ask> = asks
                .iter()
                .filter(|ask| &ask.moment == moment && &ask.kind == kind)
                .collect();
            Group::of(moment, kind, &of_key)
        })
        .collect();
    let every: Vec<&Ask> = asks.iter().collect();
    ModelReport {
        model: model.to_string(),
        set: set.to_string(),
        runs,
        warm_up_seconds: warm_up,
        groups,
        total: Group::of("all", "all", &every),
        quiet,
        asks,
    }
}

fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "-".to_string();
    }
    #[allow(clippy::cast_precision_loss, reason = "counts of asks are small")]
    let share = part as f64 / whole as f64 * 100.0;
    format!("{share:.0}%")
}

fn seconds(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |value| format!("{value:.1}"))
}

fn faults_text(faults: &BTreeMap<String, usize>) -> String {
    let named: Vec<String> = faults
        .iter()
        .map(|(name, count)| format!("{name} {count}"))
        .collect();
    named.join(", ")
}

const HEADER: &str = "moment / kind                    asks  shown silence refused failed retries   p50 s   p95 s first s  tok/s";

fn group_line(group: &Group) -> String {
    format!(
        "{:32} {:>4} {:>6} {:>7} {:>7} {:>6} {:>7} {:>7} {:>7} {:>7} {:>6}",
        group.key(),
        group.asks,
        percent(group.shown, group.asks),
        percent(group.silence, group.asks),
        percent(group.refused, group.asks),
        percent(group.failed, group.asks),
        group.retries,
        seconds(group.latency.map(|s| s.median)),
        seconds(group.latency.map(|s| s.p95)),
        seconds(group.first_byte.map(|s| s.median)),
        seconds(group.tokens_per_second),
    )
}

/// The report as a person reads it.
#[must_use]
pub fn text(report: &ModelReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Model: {}", report.model);
    let _ = writeln!(
        out,
        "Moments: {}, runs: {}, warm-up call: {} s\n",
        report.set,
        report.runs,
        seconds(report.warm_up_seconds)
    );
    let _ = writeln!(out, "{HEADER}");
    for group in report.groups.iter().chain([&report.total]) {
        let _ = writeln!(out, "{}", group_line(group));
        if !group.faults.is_empty() {
            let _ = writeln!(
                out,
                "    refused calls by fault: {}",
                faults_text(&group.faults)
            );
        }
    }
    if !report.quiet.is_empty() {
        let _ = writeln!(
            out,
            "\nMoments with no model call: {}",
            report.quiet.join(", ")
        );
    }
    let _ = writeln!(out, "\nShown lines:");
    for ask in report
        .asks
        .iter()
        .filter(|ask| ask.outcome == Outcome::Shown)
    {
        let _ = writeln!(
            out,
            "[{} / {}] {}",
            ask.moment,
            ask.kind,
            ask.shown.as_deref().unwrap_or("")
        );
    }
    let _ = writeln!(out, "\nRefused answers, last try:");
    for ask in report
        .asks
        .iter()
        .filter(|ask| ask.outcome == Outcome::Refused)
    {
        let last = ask.attempts.last();
        let faults = last.map(|a| a.faults.join(", ")).unwrap_or_default();
        let answer = last.and_then(|a| a.answer.as_deref()).unwrap_or("");
        let _ = writeln!(out, "[{} / {}] ({faults}) {answer}", ask.moment, ask.kind);
    }
    out
}

/// Several models side by side: one row for each model under each moment and kind, a short
/// summary, and the lines of each model for each moment.
#[must_use]
pub fn compare_text(reports: &[ModelReport]) -> String {
    let mut out = String::new();
    for (letter, report) in letters().zip(reports) {
        let _ = writeln!(out, "{letter}: {}", report.model);
    }
    let _ = writeln!(out, "\n  {HEADER}");
    for key in group_keys(reports) {
        for (letter, report) in letters().zip(reports) {
            let found = report.groups.iter().find(|group| group.key() == key);
            let Some(group) = found else { continue };
            let _ = writeln!(out, "{letter} {}", group_line(group));
            if !group.faults.is_empty() {
                let _ = writeln!(
                    out,
                    "      refused calls by fault: {}",
                    faults_text(&group.faults)
                );
            }
        }
    }
    let _ = writeln!(out, "\nSummary:");
    for (letter, report) in letters().zip(reports) {
        let total = &report.total;
        let _ = writeln!(
            out,
            "{letter}: shown {}, silence {}, refused {}, failed {} of {} asks; {} retries; \
             p50 {} s, p95 {} s; {} tokens/s",
            percent(total.shown, total.asks),
            percent(total.silence, total.asks),
            percent(total.refused, total.asks),
            percent(total.failed, total.asks),
            total.asks,
            total.retries,
            seconds(total.latency.map(|s| s.median)),
            seconds(total.latency.map(|s| s.p95)),
            seconds(total.tokens_per_second),
        );
    }
    let _ = writeln!(out, "\nShown lines:");
    for key in group_keys(reports) {
        let _ = writeln!(out, "{key}");
        for (letter, report) in letters().zip(reports) {
            let shown = report.asks.iter().filter(|ask| {
                format!("{} / {}", ask.moment, ask.kind) == key && ask.outcome == Outcome::Shown
            });
            for ask in shown {
                let _ = writeln!(out, "  {letter}: {}", ask.shown.as_deref().unwrap_or(""));
            }
        }
    }
    out
}

fn letters() -> impl Iterator<Item = char> {
    'A'..='Z'
}

/// Every moment and kind of any report, in the order that they first come.
fn group_keys(reports: &[ModelReport]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for group in reports.iter().flat_map(|report| &report.groups) {
        if !keys.contains(&group.key()) {
            keys.push(group.key());
        }
    }
    keys
}
