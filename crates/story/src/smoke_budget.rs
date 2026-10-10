//! Whether the narrator budget holds. A `/twdev smoke` run tests every narrator step, so
//! the budget does not block its lines. Only dev mode with an open run lifts it, and
//! `narrator::Budget` keeps no line of such a run.

use crate::dev_mode::DevMode;

/// The state of a `/twdev smoke` run in the story program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmokeRunState {
    None,
    /// A step of the run came, and its end did not.
    Open,
    /// The end of the run came. The log stays open for a step that comes late.
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NarratorBudget {
    /// The proved budget decides (`timeways_rules::budget`).
    Holds,
    /// A smoke run in dev mode: every narrator step asks the model.
    LiftedForSmoke,
}

#[must_use]
pub fn narrator_budget(mode: DevMode, run: SmokeRunState) -> NarratorBudget {
    if mode == DevMode::On && run == SmokeRunState::Open {
        NarratorBudget::LiftedForSmoke
    } else {
        NarratorBudget::Holds
    }
}
