//! The desktop part of `/twdev smoke` (TESTING.md, "One command to test everything"). Each
//! step of the addon gets the check of what only the desktop sees, and goes into the log of
//! its run.

use super::{Output, Story, StoryError};
use crate::character::Character;
use crate::dev_smoke::{
    self, CallSeen, CallTally, Desk, GateAsk, GateSeen, Logged, MAX_STEPS, SmokeDone, SmokeReport,
    SmokeStep, judge,
};
use crate::pack::{Pack, PackError};
use crate::prompt::reasons_of_retry;
use crate::spoiler;
use crate::store::{CallRow, Store};
use std::collections::{BTreeMap, BTreeSet};

/// The lines of the bridge itself, which every batch has.
const PROTOCOL_KINDS: [&str; 2] = ["batch_end", "character_entered"];

/// The most outcome passages that the gate check reads for one subject.
const MOST_GATED: u32 = 50;

/// The run whose log is open.
pub(super) struct SmokeRun {
    report: SmokeReport,
    /// Where the step before ended: the next input, and the count of events.
    next_input: u64,
    events: usize,
    /// The first call of the run.
    first_call: u64,
    /// The foes that the world held as defeated at the start of the run.
    defeated_before: BTreeSet<String>,
    /// The result of each call that a desk line named, so a line names each change once.
    named: BTreeMap<u64, String>,
}

impl Story {
    pub(super) fn smoke_step(&mut self, step: SmokeStep) -> Result<Vec<Output>, StoryError> {
        self.check_dev_mode()?;
        if !step.is_sane() {
            return Err(StoryError::BadSmokeLine);
        }
        let same_run = self
            .smoke
            .as_ref()
            .is_some_and(|run| run.report.run == step.run);
        if !same_run {
            self.start_smoke(step.run)?;
        }
        let desk = self.desk_of(&step)?;
        let verdict = judge(&step, &desk);
        let logged = Logged {
            step,
            desk,
            verdict,
        };
        let run = self.smoke.as_mut().ok_or(StoryError::NoSmokeRun)?;
        if run.report.steps.len() >= usize::try_from(MAX_STEPS).unwrap_or(usize::MAX) {
            return Err(StoryError::BadSmokeLine);
        }
        run.report.steps.push(logged.clone());
        self.write_smoke(&dev_smoke::step_text(&logged))?;
        Ok(Vec::new())
    }

    pub(super) fn smoke_done(&mut self, done: SmokeDone) -> Result<Vec<Output>, StoryError> {
        self.check_dev_mode()?;
        if !done.is_sane() {
            return Err(StoryError::BadSmokeLine);
        }
        let first_call = match &self.smoke {
            Some(run) if run.report.run == done.run => run.first_call,
            _ => return Err(StoryError::NoSmokeRun),
        };
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let rows = active.database.calls_from(first_call)?;
        let tally = CallTally::of(&rows.iter().map(seen_of).collect::<Vec<_>>());
        let run = self.smoke.as_mut().ok_or(StoryError::NoSmokeRun)?;
        let text = dev_smoke::summary(&run.report.steps, &done, &tally);
        run.report.done = Some(done);
        run.report.calls = Some(tally);
        self.write_smoke(&text)?;
        self.smoke = None;
        Ok(Vec::new())
    }

    pub(super) fn check_dev_mode(&self) -> Result<(), StoryError> {
        if self.dev_mode.is_on() {
            Ok(())
        } else {
            Err(StoryError::DevModeOff)
        }
    }

    /// The marks start where the world stands now, so the first step sees only its own lines.
    fn start_smoke(&mut self, run: u64) -> Result<(), StoryError> {
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let defeated_before: BTreeSet<String> = active
            .character
            .foes_defeated()
            .into_iter()
            .map(str::to_string)
            .collect();
        if !defeated_before.is_empty() {
            self.notice = Some(dev_smoke::NOT_FRESH.to_string());
        }
        self.smoke = Some(SmokeRun {
            report: SmokeReport {
                run,
                ..SmokeReport::default()
            },
            next_input: active.next.input,
            events: active.character.world().history().len(),
            first_call: active.next.call,
            defeated_before,
            named: BTreeMap::new(),
        });
        self.write_smoke(&dev_smoke::header(run))
    }

    /// What landed since the step before: the input lines, the facts, the calls, and the
    /// gate of the subject that the step names.
    fn desk_of(&mut self, step: &SmokeStep) -> Result<Desk, StoryError> {
        let active = self.active.as_ref().ok_or(StoryError::NoCharacter)?;
        let run = self.smoke.as_mut().ok_or(StoryError::NoSmokeRun)?;
        let mut kept: BTreeMap<String, u32> = BTreeMap::new();
        for kind in active.database.input_kinds_from(run.next_input)? {
            if !PROTOCOL_KINDS.contains(&kind.as_str()) {
                *kept.entry(kind).or_default() += 1;
            }
        }
        let events = active.character.world().history().len();
        let facts = u64::try_from(events.saturating_sub(run.events)).unwrap_or_default();
        let mut calls = Vec::new();
        for row in active.database.calls_from(run.first_call)? {
            if run.named.get(&row.position) != Some(&row.result) {
                run.named.insert(row.position, row.result.clone());
                calls.push(seen_of(&row));
            }
        }
        run.next_input = active.next.input;
        run.events = events;
        let gate = step
            .gate
            .as_ref()
            .map(|ask| {
                let before = run.defeated_before.contains(&ask.subject);
                gate_of(&self.pack, &active.character, ask, before)
            })
            .transpose()?;
        Ok(Desk {
            kept,
            facts,
            calls,
            gate,
        })
    }

    /// A run with no data folder writes no file: that is a run of the tests.
    fn write_smoke(&self, text: &str) -> Result<(), StoryError> {
        let (Store::Folder(folder), Some(run)) = (&self.store, &self.smoke) else {
            return Ok(());
        };
        let number = run.report.run;
        dev_smoke::append_log(&dev_smoke::log_file(folder, number), text)
            .and_then(|()| {
                dev_smoke::write_report(&dev_smoke::json_file(folder, number), &run.report)
            })
            .map_err(StoryError::SmokeFile)
    }
}

fn seen_of(row: &CallRow) -> CallSeen {
    let retry_of = row
        .prompt
        .as_deref()
        .and_then(|prompt| reasons_of_retry(prompt).into_iter().next());
    CallSeen {
        kind: row.kind.clone(),
        result: row.result.clone(),
        retry_of,
    }
}

/// The outcome passages that wait for the defeat of the subject, and how many of them the
/// spoiler gate lets through for this character.
fn gate_of(
    pack: &Pack,
    character: &Character,
    ask: &GateAsk,
    defeated_before_the_run: bool,
) -> Result<GateSeen, PackError> {
    let passages = pack.waiting_on_foe(&ask.subject, MOST_GATED)?;
    let open = passages
        .iter()
        .filter(|passage| spoiler::outcome_allowed(character, &passage.depends_on))
        .count();
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    Ok(GateSeen {
        subject: ask.subject.clone(),
        open: count(open),
        blocked: count(passages.len() - open),
        defeated_before_the_run,
    })
}
