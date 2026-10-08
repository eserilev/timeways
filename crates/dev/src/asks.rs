//! The verdict on each model call of a bench play: a first call and its retry make one ask,
//! which ends shown, silent, refused, or failed.

use crate::bench::{Call, Played, Row};
use crate::faults::{
    faults_of_answer, is_retry, is_silence, main_text, name_of_reason, reasons_of_retry,
    talk_faults_of_answer,
};
use serde::Serialize;

const LORE: &str = "lore";
const TALK: &str = "talk";
const RETRY_SUFFIX: &str = "_retry";
/// The kinds whose shown text is the reply of the batch, as the player reads it.
const REPLY_KINDS: [&str; 3] = ["narrator", TALK, LORE];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Shown,
    /// The model answered SILENCE, as the prompt allows.
    Silence,
    Refused,
    /// The model gave no answer.
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Attempt {
    pub answer: Option<String>,
    pub accepted: bool,
    /// The short names of the faults of a refused answer (`faults.rs`).
    pub faults: Vec<String>,
    pub latency_seconds: f64,
    pub first_byte_seconds: Option<f64>,
    pub tokens: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Ask {
    pub moment: String,
    /// The kind of the call, as the `calls` table names it, with no `_retry`.
    pub kind: String,
    pub attempts: Vec<Attempt>,
    pub outcome: Outcome,
    /// What the player reads, for a shown ask.
    pub shown: Option<String>,
}

/// Every ask of the play, in the order of its first call.
#[must_use]
pub fn asks_of(played: &Played) -> Vec<Ask> {
    let accepted = accepted_calls(played);
    let mut asks: Vec<Ask> = Vec::new();
    let mut chains: Vec<Vec<&Call>> = Vec::new();
    for (call, accepted) in played.calls.iter().zip(accepted) {
        let attempt = attempt_of(call, accepted);
        if let Some(at) = chains.iter().rposition(|chain| is_retry_of(call, chain)) {
            chains[at].push(call);
            asks[at].attempts.push(attempt);
            continue;
        }
        chains.push(vec![call]);
        asks.push(Ask {
            moment: call.moment.clone(),
            kind: base_kind(&call.kind).to_string(),
            attempts: vec![attempt],
            outcome: Outcome::Failed,
            shown: None,
        });
    }
    for (ask, chain) in asks.iter_mut().zip(&chains) {
        name_faults(ask, chain);
        ask.outcome = outcome_of(ask);
        ask.shown = shown_text(ask, chain, played);
    }
    asks
}

/// The measured moments that made no model call, such as a deed with thin lore.
#[must_use]
pub fn quiet_moments(played: &Played) -> Vec<String> {
    played
        .moments
        .iter()
        .filter(|(batch, _)| !played.calls.iter().any(|call| call.batch == **batch))
        .map(|(_, moment)| moment.clone())
        .collect()
}

fn base_kind(kind: &str) -> &str {
    kind.strip_suffix(RETRY_SUFFIX).unwrap_or(kind)
}

/// A retry repeats the first prompt of its chain, in the same batch.
fn is_retry_of(call: &Call, chain: &[&Call]) -> bool {
    let Some(last) = chain.last() else {
        return false;
    };
    is_retry(&call.prompt)
        && last.batch == call.batch
        && base_kind(&last.kind) == base_kind(&call.kind)
        && call.prompt.starts_with(chain[0].prompt.as_str())
}

fn attempt_of(call: &Call, accepted: bool) -> Attempt {
    Attempt {
        answer: call.asked.answer.clone(),
        accepted,
        faults: Vec::new(),
        latency_seconds: call.asked.latency.as_secs_f64(),
        first_byte_seconds: call.asked.first_byte.map(|time| time.as_secs_f64()),
        tokens: call.asked.tokens,
    }
}

/// A call is accepted when its row in the world says so. A `/lore` call has no row: the
/// last one of its batch is accepted when the answer shows words.
fn accepted_calls(played: &Played) -> Vec<bool> {
    let mut used = vec![false; played.rows.len()];
    played
        .calls
        .iter()
        .enumerate()
        .map(|(index, call)| {
            if call.kind == LORE {
                return is_last_lore_call(played, index)
                    && played.replies.contains_key(&call.batch);
            }
            take_row(&played.rows, &mut used, call)
        })
        .collect()
}

fn is_last_lore_call(played: &Played, index: usize) -> bool {
    let batch = played.calls[index].batch;
    !played.calls[index + 1..]
        .iter()
        .any(|later| later.batch == batch && later.kind == LORE)
}

fn take_row(rows: &[Row], used: &mut [bool], call: &Call) -> bool {
    let found = rows.iter().enumerate().position(|(at, row)| {
        !used[at] && row.kind == call.kind && row.prompt.as_deref() == Some(call.prompt.as_str())
    });
    let Some(at) = found else {
        return false;
    };
    used[at] = true;
    rows[at].accepted
}

/// A refused answer with a retry after it takes the reasons of the retry prompt. The last
/// one takes the faults that its text shows alone.
fn name_faults(ask: &mut Ask, chain: &[&Call]) {
    for (at, attempt) in ask.attempts.iter_mut().enumerate() {
        let Some(answer) = attempt.answer.as_deref() else {
            continue;
        };
        if attempt.accepted || is_silence(answer) {
            continue;
        }
        attempt.faults = match chain.get(at + 1) {
            Some(retry) => reasons_of_retry(&retry.prompt)
                .iter()
                .map(|reason| name_of_reason(reason).to_string())
                .collect(),
            None if ask.kind == TALK => talk_faults_of_answer(answer, &chain[at].prompt)
                .into_iter()
                .map(String::from)
                .collect(),
            None => faults_of_answer(answer, &chain[at].prompt)
                .into_iter()
                .map(String::from)
                .collect(),
        };
    }
}

fn outcome_of(ask: &Ask) -> Outcome {
    let Some(last) = ask.attempts.last() else {
        return Outcome::Failed;
    };
    match last.answer.as_deref() {
        None => Outcome::Failed,
        Some(_) if last.accepted => Outcome::Shown,
        Some(answer) if is_silence(answer) => Outcome::Silence,
        Some(_) => Outcome::Refused,
    }
}

fn shown_text(ask: &Ask, chain: &[&Call], played: &Played) -> Option<String> {
    if ask.outcome != Outcome::Shown {
        return None;
    }
    let batch = chain.first()?.batch;
    if REPLY_KINDS.contains(&ask.kind.as_str()) {
        return played.replies.get(&batch).cloned();
    }
    ask.attempts.last()?.answer.as_deref().map(main_text)
}
