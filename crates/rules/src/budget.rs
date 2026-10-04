//! The budget of the narrator (GAMEPLAY.md 3.2): at most three lines in an hour of game
//! time.

pub const LINES_PER_HOUR: usize = 3;
const HOUR: u64 = 3600;

/// The times of the last lines in seconds, oldest first. No heap, so Kani proves the
/// budget fast.
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
