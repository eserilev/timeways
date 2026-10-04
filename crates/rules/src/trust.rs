//! How trust moves (GAMEPLAY.md 3.5): a change stops at the ends of the band.

/// The band of trust. `vocabulary::TRUST` in the story program is this band.
pub const MIN_TRUST: i64 = -100;
pub const MAX_TRUST: i64 = 100;

/// The trust after a change of `by`, inside the band. An NPC with no trust yet starts at 0.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn next_trust(held: Option<i64>, by: i64) -> i64 {
    held.unwrap_or(0)
        .saturating_add(by)
        .clamp(MIN_TRUST, MAX_TRUST)
}
