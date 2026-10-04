//! When a step of a side quest holds (docs/plans/quest-variety.md 6.3 to 6.5).

use super::{Step, Tracked};
use hourglass::Tick;

/// What the world shows when a line arrives, as far as a step can use it.
#[derive(Debug, Default)]
pub struct Here<'a> {
    pub at: Tick,
    /// Where you stand: the subzone, then the zone.
    pub places: Vec<&'a str>,
}

/// What one line of the addon did, as far as a step can see it. It owns its names, because
/// the line is gone when the quests move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Encounter {
    None,
    /// A gossip window of the game: `npc_met`.
    Gossip(String),
    /// `/talk`: `talk_asked`.
    Talk(String),
    Slap(String),
}

impl Encounter {
    /// A gossip window, a talk, and a slap all meet the NPC.
    #[must_use]
    pub fn meets(&self, npc: &str) -> bool {
        match self {
            Encounter::Gossip(name) | Encounter::Talk(name) | Encounter::Slap(name) => name == npc,
            Encounter::None => false,
        }
    }
}

impl Tracked {
    /// Does the step hold now? A step that reads the world, such as a visit, holds at once
    /// when it opens, if the world already shows it.
    #[must_use]
    pub fn step_holds(&self, step: usize, here: &Here<'_>, met: &Encounter) -> bool {
        match self.steps.get(step) {
            Some(Step::Visit { place }) => here.places.contains(&place.as_str()),
            Some(Step::Meet { npc }) => met.meets(npc),
            Some(Step::Kill { count, .. }) => self.kills[step] >= *count,
            None => false,
        }
    }
}
