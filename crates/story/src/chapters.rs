//! Where the chronicle starts a new chapter (GAMEPLAY.md 3.3). The code decides from
//! the facts of the world, never a model.

use hourglass::Tick;

/// No event for this long ends a stretch of play. Time away never counts as play.
pub const SESSION_GAP_SECONDS: u64 = 30 * 60;

/// A milestone after less play than this joins the chapter that runs, so two milestones
/// close together make one chapter, not two thin ones.
pub const MIN_CHAPTER_PLAY_SECONDS: u64 = 45 * 60;

/// Every tenth level is a milestone.
pub const LEVEL_STEP: i64 = 10;

/// The first and last tick of each stretch of play, from the ticks of the history in
/// order. Tick 0 is the founding of the character, and belongs to no session.
#[must_use]
pub fn sessions(ticks: impl Iterator<Item = Tick>) -> Vec<(Tick, Tick)> {
    let mut sessions: Vec<(Tick, Tick)> = Vec::new();
    for tick in ticks.filter(|tick| tick.0 > 0) {
        match sessions.last_mut() {
            Some((_, last)) if tick.0.saturating_sub(last.0) < SESSION_GAP_SECONDS => *last = tick,
            _ => sessions.push((tick, tick)),
        }
    }
    sessions
}

/// The tick where each chapter begins. The first chapter begins with the first play. A
/// later one begins at a milestone, once the chapter before it had enough play.
#[must_use]
pub fn chapter_starts(sessions: &[(Tick, Tick)], milestones: &[Tick]) -> Vec<Tick> {
    let Some(&(first, _)) = sessions.first() else {
        return Vec::new();
    };
    let mut milestones = milestones.to_vec();
    milestones.sort();
    let mut starts = vec![first];
    for milestone in milestones {
        let began = starts[starts.len() - 1];
        if played(sessions, began, milestone) >= MIN_CHAPTER_PLAY_SECONDS {
            starts.push(milestone);
        }
    }
    starts
}

/// The seconds of play between two ticks, without the time away.
fn played(sessions: &[(Tick, Tick)], from: Tick, to: Tick) -> u64 {
    sessions
        .iter()
        .map(|&(began, ended)| {
            let start = began.max(from).0;
            let end = ended.min(to).0;
            end.saturating_sub(start)
        })
        .sum()
}

#[must_use]
pub fn is_level_milestone(level: i64) -> bool {
    level % LEVEL_STEP == 0
}
