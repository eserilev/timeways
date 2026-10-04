//! A side quest as the book shows it (docs/plans/quest-variety.md 8).

use super::{Status, Step, Tracked};
use hourglass::Tick;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QuestView {
    pub number: u64,
    pub offered_at: Tick,
    pub giver: String,
    pub title: String,
    pub text: String,
    pub status: Status,
    pub done_at: Option<Tick>,
    /// The steps that the player can see, in order.
    pub steps: Vec<StepView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StepView {
    #[serde(flatten)]
    pub step: Step,
    pub state: StepState,
    /// The kills so far of a kill step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kills: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Done,
    /// The player can do it now.
    Open,
    /// It waits for a step before it, or for the accept.
    Later,
}

impl QuestView {
    #[must_use]
    pub fn of(quest: &Tracked) -> QuestView {
        let steps = (0..quest.steps.len())
            .map(|step| step_view(quest, step))
            .collect();
        QuestView {
            number: quest.number,
            offered_at: quest.offered_at,
            giver: quest.giver.clone(),
            title: quest.title.clone(),
            text: quest.text.clone(),
            status: quest.status,
            done_at: quest.done_at,
            steps,
        }
    }
}

fn step_view(quest: &Tracked, step: usize) -> StepView {
    let state = if quest.is_done(step) {
        StepState::Done
    } else if quest.is_open(step) {
        StepState::Open
    } else {
        StepState::Later
    };
    let kills = match &quest.steps[step] {
        Step::Kill { count, .. } if state == StepState::Done => Some(*count),
        Step::Kill { .. } => Some(quest.kills[step]),
        _ => None,
    };
    StepView {
        step: quest.steps[step].clone(),
        state,
        kills,
    }
}
