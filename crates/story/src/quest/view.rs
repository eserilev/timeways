//! A side quest as the book shows it (docs/plans/quest-variety.md 8).

use super::{AnyOrder, Genre, Status, Step, Tracked};
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
    /// True when a step asks you to slap someone: the page says so before you accept.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub has_slap: bool,
    /// The span of the steps that can be done in any order, when every step of it is in
    /// `steps`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_order: Option<AnyOrder>,
    /// The steps of a mystery that stay hidden. The addon never gets them, so no other
    /// addon can read them from the saved variables.
    #[serde(skip_serializing_if = "is_zero")]
    pub hidden_steps: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StepView {
    #[serde(flatten)]
    pub step: Step,
    pub state: StepState,
    /// The kills so far of a kill step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kills: Option<u8>,
    /// When an open wait is over.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_at: Option<Tick>,
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
        let shown: Vec<usize> = (0..quest.steps.len())
            .filter(|step| is_shown(quest, *step))
            .collect();
        let steps = shown.iter().map(|step| step_view(quest, *step)).collect();
        let whole_set = |span: &AnyOrder| shown.contains(&span.last);
        QuestView {
            number: quest.number,
            offered_at: quest.offered_at,
            giver: quest.giver.clone(),
            title: quest.title.clone(),
            text: quest.text.clone(),
            status: quest.status,
            done_at: quest.done_at,
            steps,
            has_slap: quest
                .steps
                .iter()
                .any(|step| matches!(step, Step::Slap { .. })),
            any_order: quest.any_order.filter(whole_set),
            hidden_steps: quest.steps.len() - shown.len(),
        }
    }
}

/// A mystery shows its done and open steps, and an offer of a mystery its first stage.
/// Every other quest shows each step.
fn is_shown(quest: &Tracked, step: usize) -> bool {
    if !quest.genre.is_some_and(Genre::hides_later_steps) {
        return true;
    }
    if quest.status == Status::Offered {
        return quest.in_first_stage(step);
    }
    quest.is_done(step) || quest.is_open(step)
}

#[allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn is_zero(count: &usize) -> bool {
    *count == 0
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
        ready_at: quest.ready_at(step),
    }
}
