//! Which answer of the hero sheet a hook takes, one call in three (GAMEPLAY.md 3.7).

/// A hook applies to one call in this many.
pub const HOOK_EVERY: u64 = 3;

/// `count` is the number of talk and quest calls of the character before this one. The
/// third, sixth, and ninth call get a hook.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn applies(count: u64) -> bool {
    count % HOOK_EVERY == HOOK_EVERY - 1
}

/// The index of the filled field that the call takes, or None when it takes none. Each
/// hook call takes the next filled field, and starts again after the last one.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn pick(filled: usize, count: u64) -> Option<usize> {
    if !applies(count) || filled == 0 {
        return None;
    }
    let turn = count / HOOK_EVERY;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the remainder is below `filled`, so it fits a usize"
    )]
    let index = (turn % filled as u64) as usize;
    Some(index)
}
