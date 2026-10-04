//! What the world of the player holds, and the rules of 3.4 for each step against it.

use super::{MAX_KILLS, QuestFault, Step};
use crate::check::mentions;
use crate::seen::{SeenText, TextKind};

/// What the world of the player holds, as far as a quest can use it.
#[derive(Debug, Default)]
pub struct Known<'a> {
    pub giver: &'a str,
    /// The zones that you visited.
    pub zones: Vec<&'a str>,
    pub subzones: Vec<&'a str>,
    /// The NPCs that you can meet: you met them, or saw them friendly. Never a beast, and
    /// never the dead of your story.
    pub npcs: Vec<&'a str>,
    /// The creatures that you saw hostile, except the dead of your story.
    pub foes: Vec<&'a str>,
    /// The places, NPCs, and creatures of your newest task. The next task names none of
    /// them, so two tasks in a row never send you to the same target.
    pub last_targets: Vec<&'a str>,
    /// Every text that you read. Only the quests count.
    pub seen: &'a [SeenText],
}

impl Known<'_> {
    /// The places that a visit step can name, as the check allows them.
    #[must_use]
    pub fn places(&self) -> Vec<&str> {
        let all = self.zones.iter().chain(&self.subzones);
        let visit = |place: &str| Step::Visit {
            place: place.to_string(),
        };
        all.copied()
            .filter(|place| self.step_fault(&visit(place)).is_none())
            .collect()
    }

    /// The NPCs that a meet step can name, as the check allows them.
    #[must_use]
    pub fn people(&self) -> Vec<&str> {
        let meet = |npc: &str| Step::Meet {
            npc: npc.to_string(),
        };
        let allowed = |npc: &&str| self.step_fault(&meet(npc)).is_none();
        self.npcs.iter().copied().filter(allowed).collect()
    }

    /// The creatures that a kill step can name, as the check allows them.
    #[must_use]
    pub fn prey(&self) -> Vec<&str> {
        let kill = |creature: &str| Step::Kill {
            creature: creature.to_string(),
            count: 1,
        };
        let allowed = |creature: &&str| self.step_fault(&kill(creature)).is_none();
        self.foes.iter().copied().filter(allowed).collect()
    }

    /// The first rule of 3.4 that one step breaks. A zone never counts as a goal of a game
    /// quest: most quest texts name their zone, so the rule then bans every zone.
    pub(super) fn step_fault(&self, step: &Step) -> Option<QuestFault> {
        let target = step.target();
        if self.last_targets.contains(&target) {
            return Some(QuestFault::LastTask(target.to_string()));
        }
        match step {
            Step::Visit { place } if self.zones.contains(&place.as_str()) => None,
            Step::Visit { place } if self.subzones.contains(&place.as_str()) => {
                in_game_quests(place, self.seen)
            }
            Step::Visit { place } => Some(QuestFault::UnknownPlace(place.clone())),
            Step::Meet { npc } if npc == self.giver => Some(QuestFault::MeetGiver),
            Step::Meet { npc } if self.npcs.contains(&npc.as_str()) => {
                in_game_quests(npc, self.seen)
            }
            Step::Meet { npc } => Some(QuestFault::UnknownNpc(npc.clone())),
            Step::Kill { creature, .. } if creature == self.giver => Some(QuestFault::KillGiver),
            Step::Kill { count, .. } if !(1..=MAX_KILLS).contains(count) => {
                Some(QuestFault::KillCount(*count))
            }
            Step::Kill { creature, .. } if self.foes.contains(&creature.as_str()) => {
                in_game_quests(creature, self.seen)
            }
            Step::Kill { creature, .. } => Some(QuestFault::UnknownFoe(creature.clone())),
        }
    }
}

/// Only the quests that you read count. A quest of the game that you never saw can still
/// share a goal with a side quest.
fn in_game_quests(name: &str, seen: &[SeenText]) -> Option<QuestFault> {
    let named = game_quests(seen).any(|quest| {
        mentions(&quest.text, name) || quest.title.as_deref().is_some_and(|t| mentions(t, name))
    });
    named.then(|| QuestFault::GameQuest(name.to_string()))
}

pub(super) fn game_quests(seen: &[SeenText]) -> impl Iterator<Item = &SeenText> {
    seen.iter().filter(|text| text.kind == TextKind::Quest)
}
