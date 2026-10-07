//! The budget of the narrator (GAMEPLAY.md 3.2): at most three lines in an hour of game
//! time.

pub const LINES_PER_HOUR: usize = 3;
const HOUR: u64 = 3600;

/// The times of the last lines in seconds, oldest first.
#[derive(Default)]
pub struct Budget {
    pub spoken: [Option<u64>; LINES_PER_HOUR],
}

impl Budget {
    /// True when the narrator has a line left at `at`. That line then counts.
    #[cfg_attr(charon, verify::start_from)]
    pub fn take(&mut self, at: u64) -> bool {
        // A copy first: Aeneas cannot match on a slot of an array in place.
        let oldest = self.spoken[0];
        let too_soon = match oldest {
            Some(oldest) => at.saturating_sub(oldest) < HOUR,
            None => false,
        };
        if too_soon {
            return false;
        }
        // Aeneas translates no `rotate_left` of an array.
        self.spoken = [self.spoken[1], self.spoken[2], Some(at)];
        true
    }
}

/// The budget that a load keeps. A run of `take` from an empty budget fills the slots from
/// the end, so an empty slot after a line time comes only from a damaged file. Such a
/// budget starts again from empty, and the budget law then holds across the load.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn budget_on_load(loaded: Budget) -> Budget {
    // Copies first: Aeneas cannot match on a slot of an array in place.
    let first = loaded.spoken[0];
    let second = loaded.spoken[1];
    let third = loaded.spoken[2];
    if gap_after_line(first, second) || gap_after_line(second, third) {
        return Budget {
            spoken: [None, None, None],
        };
    }
    loaded
}

fn gap_after_line(earlier: Option<u64>, later: Option<u64>) -> bool {
    match earlier {
        Some(_) => later.is_none(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(spoken: [Option<u64>; LINES_PER_HOUR]) -> Budget {
        Budget { spoken }
    }

    #[test]
    fn a_load_keeps_a_budget_that_a_run_can_make() {
        for spoken in [
            [None, None, None],
            [None, None, Some(5)],
            [None, Some(9), Some(5)],
            [Some(1), Some(9), Some(5)],
        ] {
            assert_eq!(budget_on_load(budget(spoken)).spoken, spoken);
        }
    }

    #[test]
    fn a_load_empties_a_budget_with_a_gap_after_a_line() {
        for spoken in [
            [Some(1), None, None],
            [Some(1), None, Some(5)],
            [None, Some(1), None],
            [Some(1), Some(2), None],
        ] {
            assert_eq!(budget_on_load(budget(spoken)).spoken, [None, None, None]);
        }
    }
}
