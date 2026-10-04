//! The rules of a quest for its steps together: their order, and what each one names
//! (docs/plans/quest-variety.md 7.3, part 5).

use super::{Genre, QuestFault, Step};

/// The first rule of the order of the steps that the quest breaks.
pub(super) fn structure_fault(steps: &[Step], giver: &str, genre: Genre) -> Option<QuestFault> {
    if !waits_in_place(steps) {
        return Some(QuestFault::WaitPlace);
    }
    if names_giver_too_soon(steps, giver) {
        return Some(QuestFault::MeetGiver);
    }
    let slaps = steps.iter().any(|step| matches!(step, Step::Slap { .. }));
    if slaps && genre != Genre::Comic {
        return Some(QuestFault::SlapOutsideComic);
    }
    if repeats_a_target(steps) {
        return Some(QuestFault::RepeatedStep);
    }
    None
}

/// At most one wait, and never the first or the last step: a wait first only delays the
/// quest, and a wait last ends it with nothing to do.
fn waits_in_place(steps: &[Step]) -> bool {
    let waits: Vec<usize> = (0..steps.len())
        .filter(|n| matches!(steps[*n], Step::Wait { .. }))
        .collect();
    match waits[..] {
        [] => true,
        [wait] => wait > 0 && wait + 1 < steps.len(),
        _ => false,
    }
}

/// The giver stands next to you at the accept, so a step names the giver only after a
/// wait: "Come back in two days."
fn names_giver_too_soon(steps: &[Step], giver: &str) -> bool {
    let Some(first) = steps.iter().position(|step| step.person() == Some(giver)) else {
        return false;
    };
    !steps[..first]
        .iter()
        .any(|step| matches!(step, Step::Wait { .. }))
}

/// Each step names a different target, so one event never does two steps of one quest.
fn repeats_a_target(steps: &[Step]) -> bool {
    let targets: Vec<&str> = steps.iter().filter_map(Step::target).collect();
    (1..targets.len()).any(|n| targets[..n].contains(&targets[n]))
}
