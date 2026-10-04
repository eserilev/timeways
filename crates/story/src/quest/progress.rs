//! When a step of a side quest holds (docs/plans/quest-variety.md 6.5).

use super::{Step, Tracked};

impl Tracked {
    /// Does the step hold at this moment: you stand in its place, you meet its NPC, or you
    /// made its kills?
    #[must_use]
    pub fn step_holds(&self, step: usize, places_here: &[&str], npc: Option<&str>) -> bool {
        match self.steps.get(step) {
            Some(Step::Visit { place }) => places_here.contains(&place.as_str()),
            Some(Step::Meet { npc: wanted }) => npc == Some(wanted.as_str()),
            Some(Step::Kill { count, .. }) => self.kills[step] >= *count,
            None => false,
        }
    }
}
