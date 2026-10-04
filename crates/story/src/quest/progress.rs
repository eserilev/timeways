//! When a step of a side quest holds (docs/plans/quest-variety.md 6.3 to 6.5).

use super::{Step, Tracked};
use hourglass::Tick;

/// What the world shows when a line arrives, as far as a step can use it.
#[derive(Debug, Default)]
pub struct Here<'a> {
    pub at: Tick,
    /// Where you stand: the subzone, then the zone.
    pub places: Vec<&'a str>,
    /// The local hour of this line, when it carries one.
    pub hour: Option<u8>,
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
    /// `emote_done`, with its NPC target when it has one.
    Emote {
        emote: String,
        target: Option<String>,
    },
    /// `items_held`: the count of one item in your bags, at a meeting with the NPC.
    Carry {
        npc: String,
        item: String,
        count: u16,
    },
}

impl Encounter {
    /// A gossip window, a talk, and a slap all meet the NPC.
    #[must_use]
    pub fn meets(&self, npc: &str) -> bool {
        match self {
            Encounter::Gossip(name) | Encounter::Talk(name) | Encounter::Slap(name) => name == npc,
            Encounter::None | Encounter::Emote { .. } | Encounter::Carry { .. } => false,
        }
    }

    /// Was the line this emote, at this NPC or with no NPC?
    fn emotes(&self, wanted: &str, npc: Option<&str>) -> bool {
        matches!(self, Encounter::Emote { emote, target }
            if emote == wanted && (npc.is_none() || target.as_deref() == npc))
    }

    /// Did the line show at least `count` of the item in your bags, at this NPC?
    fn carries(&self, npc: &str, item: &str, count: u8) -> bool {
        matches!(self, Encounter::Carry { npc: met, item: held, count: have }
            if met == npc && held == item && *have >= u16::from(count))
    }
}

impl Tracked {
    /// Does the step hold now? A step that reads the world, such as a visit, holds at once
    /// when it opens, if the world already shows it.
    #[must_use]
    pub fn step_holds(&self, step: usize, here: &Here<'_>, met: &Encounter) -> bool {
        match self.steps.get(step) {
            Some(Step::Visit { place }) => here.places.contains(&place.as_str()),
            Some(Step::VisitAt { place, time }) => {
                here.places.contains(&place.as_str())
                    && here.hour.is_some_and(|hour| time.holds_at(hour))
            }
            Some(Step::Meet { npc }) => met.meets(npc),
            Some(Step::Talk { npc, .. }) => matches!(met, Encounter::Talk(name) if name == npc),
            Some(Step::Kill { count, .. }) => self.kills[step] >= *count,
            Some(Step::Carry { item, count, npc }) => met.carries(npc, item, *count),
            Some(Step::Slap { npc }) => matches!(met, Encounter::Slap(name) if name == npc),
            Some(Step::Emote { emote, npc, place }) => {
                emote_holds(here, met, emote, npc.as_deref(), place.as_deref())
            }
            Some(Step::Wait { .. }) => self.ready_at(step).is_some_and(|ready| here.at >= ready),
            None => false,
        }
    }
}

/// An emote at the NPC of the step, or anywhere in its place.
fn emote_holds(
    here: &Here<'_>,
    met: &Encounter,
    emote: &str,
    npc: Option<&str>,
    place: Option<&str>,
) -> bool {
    match (npc, place) {
        (Some(npc), _) => met.emotes(emote, Some(npc)),
        (None, Some(place)) => met.emotes(emote, None) && here.places.contains(&place),
        (None, None) => false,
    }
}
