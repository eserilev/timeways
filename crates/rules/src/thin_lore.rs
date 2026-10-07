//! Silence when the lore is thin (GAMEPLAY.md 3.2): a deed gets no narrator line when no
//! passage of its prompt is about one of its subjects. The story program gives each name an
//! id, so the rule reads ids, never strings. Lean proves its laws
//! (lean/Timeways/ThinLore.lean).
//!
//! The functions walk by index: Aeneas translates no iterator adapter (lean/README.md).

/// What a moment is, as the rule reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MomentKind {
    /// The hero only came to a place. Such a line tells the place, with lore or without.
    Arrival,
    /// A small, silly moment (5.4.1). It has no lore by design.
    Flavor,
    /// The hero did something: a kill, a death, a quest, a mount, an item, a level.
    Deed,
}

/// True when the moment gets no line. `subjects` holds the ids of what the moment is about.
/// `lore` holds the ids of what the passages of the prompt are about: the page, the links,
/// and each subject that the shown text names. No passage gives an empty `lore`.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn is_silent(kind: MomentKind, subjects: &[u32], lore: &[u32]) -> bool {
    match kind {
        MomentKind::Arrival | MomentKind::Flavor => false,
        MomentKind::Deed => !shares_one(subjects, lore),
    }
}

/// True when an id of `lore` is in `subjects`. An index loop.
fn shares_one(subjects: &[u32], lore: &[u32]) -> bool {
    let mut index = 0;
    while index < lore.len() {
        if holds(subjects, lore[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// An index loop, because Aeneas translates no `contains`. The gate of outcome passages
/// reads it too.
pub(crate) fn holds(ids: &[u32], id: u32) -> bool {
    let mut index = 0;
    while index < ids.len() {
        if ids[index] == id {
            return true;
        }
        index += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deed_with_no_passage_is_silent() {
        assert!(is_silent(MomentKind::Deed, &[1], &[]));
    }

    #[test]
    fn a_deed_with_lore_about_another_subject_is_silent() {
        assert!(is_silent(MomentKind::Deed, &[1, 2], &[3, 4]));
    }

    #[test]
    fn a_deed_with_lore_about_one_of_its_subjects_speaks() {
        assert!(!is_silent(MomentKind::Deed, &[1, 2], &[3, 2]));
    }

    #[test]
    fn an_arrival_and_a_flavor_moment_are_never_silenced() {
        assert!(!is_silent(MomentKind::Arrival, &[1], &[]));
        assert!(!is_silent(MomentKind::Flavor, &[], &[]));
    }
}
