//! The prologue of the Chronicle (GAMEPLAY.md 3.3): a character that Timeways first sees
//! with a long past gets one chapter 0 about it. The first look at the past decides, and a
//! prologue is written at most once. Lean proves the laws (lean/Timeways/Prologue.lean).

/// A prologue needs this level or more.
pub const PROLOGUE_LEVEL: u8 = 10;

/// With this many finished quests of the game, the past is long enough.
pub const PROLOGUE_QUESTS: u32 = 20;

/// With this many discovered zones, the past is long enough too.
pub const PROLOGUE_ZONES: u32 = 2;

/// Whether the world held play of Timeways when the past came.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorldAge {
    /// No closed chapter and no tale: Timeways sees the character for the first time.
    New,
    /// Timeways told this character already, so its past is no news.
    Played,
}

/// What the game told of the past of a character at a login, as the rule reads it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Past {
    pub level: u8,
    pub quests: u32,
    pub zones: u32,
    pub world: WorldAge,
}

/// Where the prologue of a character stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prologue {
    /// No past came yet.
    Unseen,
    /// The first past was short, or came to a world with play. No prologue ever comes.
    Skipped,
    /// The first past was long, and no prologue is written yet.
    Due,
    Written,
}

/// True for a past that earns a prologue: level 10 or more, 20 quests or 2 zones, and a
/// world with no play of Timeways yet.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn is_long(past: Past) -> bool {
    // Early returns, because Aeneas turns a `||` of two comparisons into a `Prop`.
    if let WorldAge::Played = past.world {
        return false;
    }
    if past.level < PROLOGUE_LEVEL {
        return false;
    }
    if past.quests >= PROLOGUE_QUESTS {
        return true;
    }
    past.zones >= PROLOGUE_ZONES
}

/// Only the first past decides. Every later one changes nothing.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn after_past(state: Prologue, past: Past) -> Prologue {
    match state {
        Prologue::Unseen => {
            if is_long(past) {
                Prologue::Due
            } else {
                Prologue::Skipped
            }
        }
        Prologue::Skipped => Prologue::Skipped,
        Prologue::Due => Prologue::Due,
        Prologue::Written => Prologue::Written,
    }
}

/// A prologue that the model wrote. It counts only while one is due.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn after_written(state: Prologue) -> Prologue {
    match state {
        Prologue::Due | Prologue::Written => Prologue::Written,
        Prologue::Unseen => Prologue::Unseen,
        Prologue::Skipped => Prologue::Skipped,
    }
}

#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn is_due(state: Prologue) -> bool {
    match state {
        Prologue::Due => true,
        Prologue::Unseen | Prologue::Skipped | Prologue::Written => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn past(level: u8, quests: u32, zones: u32) -> Past {
        Past {
            level,
            quests,
            zones,
            world: WorldAge::New,
        }
    }

    #[test]
    fn a_level_35_with_many_quests_gets_a_prologue() {
        assert_eq!(
            after_past(Prologue::Unseen, past(35, 212, 14)),
            Prologue::Due
        );
    }

    #[test]
    fn a_new_character_gets_no_prologue() {
        assert_eq!(
            after_past(Prologue::Unseen, past(1, 0, 1)),
            Prologue::Skipped
        );
    }

    #[test]
    fn level_9_is_too_early_for_a_prologue() {
        assert_eq!(
            after_past(Prologue::Unseen, past(9, 40, 5)),
            Prologue::Skipped
        );
    }

    #[test]
    fn level_10_with_two_zones_and_no_quests_is_enough() {
        assert_eq!(after_past(Prologue::Unseen, past(10, 0, 2)), Prologue::Due);
    }

    #[test]
    fn level_10_with_twenty_quests_in_one_zone_is_enough() {
        assert_eq!(after_past(Prologue::Unseen, past(10, 20, 1)), Prologue::Due);
    }

    #[test]
    fn nineteen_quests_in_one_zone_are_not_enough() {
        assert_eq!(
            after_past(Prologue::Unseen, past(30, 19, 1)),
            Prologue::Skipped
        );
    }

    #[test]
    fn a_world_with_play_gets_no_prologue() {
        let played = Past {
            world: WorldAge::Played,
            ..past(40, 300, 20)
        };

        assert_eq!(after_past(Prologue::Unseen, played), Prologue::Skipped);
    }

    #[test]
    fn a_later_past_never_brings_a_prologue_back() {
        let skipped = after_past(Prologue::Unseen, past(5, 2, 1));

        assert_eq!(after_past(skipped, past(40, 300, 20)), Prologue::Skipped);
    }

    #[test]
    fn a_prologue_is_written_at_most_once() {
        let written = after_written(after_past(Prologue::Unseen, past(35, 212, 14)));

        assert_eq!(written, Prologue::Written);
        assert!(!is_due(written));
        assert!(!is_due(after_past(written, past(36, 220, 15))));
    }

    #[test]
    fn a_written_prologue_before_any_past_counts_for_nothing() {
        assert_eq!(after_written(Prologue::Unseen), Prologue::Unseen);
    }
}
