//! The JSON of a quest answer, and the flat list of its steps (docs/plans/quest-variety.md
//! 5.1).

use super::{AnyOrder, QuestFault, Step};
use serde::Deserialize;
use timeways_rules::quest_log::{MAX_SET_STEPS, MIN_SET_STEPS};

const SET_SIZES: std::ops::RangeInclusive<usize> = MIN_SET_STEPS..=MAX_SET_STEPS;

#[derive(Deserialize)]
pub(super) struct Reply {
    pub(super) title: String,
    #[serde(default)]
    pub(super) genre: Option<String>,
    pub(super) text: String,
    pub(super) steps: Vec<Entry>,
}

/// One entry of `steps`: a step, or `{"goal": "any_order", "steps": [...]}`. A set inside a
/// set is no step, so it fails the parse.
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Entry {
    Set(Set),
    One(Step),
}

#[derive(Deserialize)]
pub(super) struct Set {
    #[serde(rename = "goal")]
    _goal: SetGoal,
    steps: Vec<Step>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum SetGoal {
    AnyOrder,
}

/// The steps in order, and the span of the set. The steps of a set join the list in their
/// order.
pub(super) fn flattened(entries: Vec<Entry>) -> Result<(Vec<Step>, Option<AnyOrder>), QuestFault> {
    let mut steps = Vec::new();
    let mut any_order = None;
    for entry in entries {
        match entry {
            Entry::One(step) => steps.push(step),
            Entry::Set(set) => {
                if any_order.is_some() {
                    return Err(QuestFault::AnyOrderTwice);
                }
                any_order = Some(checked_set(steps.len(), &set.steps)?);
                steps.extend(set.steps);
            }
        }
    }
    Ok((steps, any_order))
}

fn checked_set(first: usize, steps: &[Step]) -> Result<AnyOrder, QuestFault> {
    if let Some(fault) = set_fault(steps) {
        return Err(fault);
    }
    Ok(AnyOrder {
        first,
        last: first + steps.len() - 1,
    })
}

/// The fault of a set that breaks the rule: 2 or 3 steps, and no wait. The quest log keeps
/// the same rule for a line of the quest file (timeways-rules).
fn set_fault(steps: &[Step]) -> Option<QuestFault> {
    if !SET_SIZES.contains(&steps.len()) {
        return Some(QuestFault::AnyOrderSize(steps.len()));
    }
    let has_wait = steps.iter().any(|step| matches!(step, Step::Wait { .. }));
    has_wait.then_some(QuestFault::AnyOrderWait)
}
