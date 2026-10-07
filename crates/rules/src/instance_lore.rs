//! The lore of an entry into a dungeon or a raid (GAMEPLAY.md 3.2). Each entry tells one
//! passage of the instance, and never one that the character was told before. Once every
//! passage was told, entries are silent. Lean proves its laws
//! (lean/Timeways/InstanceLore.lean).

/// The passage that the next entry tells: the first, in pack order, that the character was
/// never told. `told` holds how many times each candidate was told, in pack order. A told
/// passage never comes back, so the least used candidate is always one never told.
/// An index loop, because Aeneas translates no iterator adapter.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn next_passage(told: &[u32]) -> Option<usize> {
    let mut index = 0;
    while index < told.len() {
        if told[index] == 0 {
            return Some(index);
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_entry_after_the_first_tells_the_first_untold_passage() {
        assert_eq!(next_passage(&[1, 0, 0]), Some(1));
    }

    #[test]
    fn entries_are_silent_once_every_passage_was_told() {
        assert_eq!(next_passage(&[1, 2, 1]), None);
    }

    #[test]
    fn an_instance_with_no_passage_is_silent() {
        assert_eq!(next_passage(&[]), None);
    }
}
