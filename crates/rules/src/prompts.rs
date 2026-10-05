//! Which calls keep their prompts (GAMEPLAY.md 5.14): the newest ones.

/// After this many calls, the prompt of an older call is cleared. Its row, its answer,
/// and its links stay.
pub const PROMPTS_KEPT: u64 = 500;

/// The position of the oldest call that keeps its prompt, after the call at `newest`.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn oldest_prompt_kept(newest: u64) -> u64 {
    newest.saturating_sub(PROMPTS_KEPT - 1)
}
