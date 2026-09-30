//! How busy the model budget of the bridge is, as the story program sees it (GAMEPLAY.md
//! 3.3). The bridge admits 10 calls in any 20 minutes, and it refuses a call over that
//! budget with a plain `model_failed`. So a failure counts as a sign of a tight window.

use hourglass::Tick;

/// The window of the budget of the bridge.
pub const WINDOW_SECONDS: u64 = 20 * 60;

/// This many calls in one window leave the rest of the budget to the player. The best of
/// two costs 3 calls, so 5 + 3 still leaves 2 of the 10.
pub const BUSY_CALLS: usize = 5;

/// The newest opened calls and the newest failure, in the time of the addon.
#[derive(Debug, Default)]
pub struct Pace {
    /// The times of the last calls, oldest first. Only the last `BUSY_CALLS` matter.
    opened: [Option<Tick>; BUSY_CALLS],
    failed: Option<Tick>,
}

impl Pace {
    pub fn opened(&mut self, at: Tick) {
        self.opened.rotate_left(1);
        self.opened[BUSY_CALLS - 1] = Some(at);
    }

    pub fn failed(&mut self, at: Tick) {
        self.failed = Some(at);
    }

    /// True when the saga of a chapter gets no extra call: no second draft and no judge.
    #[must_use]
    pub fn is_tight(&self, at: Tick) -> bool {
        let recent = |time: Option<Tick>| {
            time.is_some_and(|time| at.0.saturating_sub(time.0) < WINDOW_SECONDS)
        };
        recent(self.failed) || recent(self.opened[0])
    }
}
