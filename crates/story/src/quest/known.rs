//! What the world of the player holds, and the rules of 3.4 for each step against it.

use super::variety::Recent;
use super::{
    MAX_CARRY, MAX_KILLS, MAX_TOPIC_BYTES, MAX_TOPIC_CHARS, MAX_WAIT_DAYS, QuestFault, Step,
};
use crate::check::{mentions, plain_text};
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
    /// The newest offers, newest first (`variety::recent_quests`).
    pub recent: Vec<Recent>,
    /// The goods that a carry step can ask for, for your level.
    pub goods: Vec<&'a str>,
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

    /// The NPCs that a meet or a talk step can name, as the check allows them.
    #[must_use]
    pub fn people(&self) -> Vec<&str> {
        let meet = |npc: &str| Step::Meet {
            npc: npc.to_string(),
        };
        let allowed = |npc: &&str| *npc != self.giver && self.step_fault(&meet(npc)).is_none();
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
        if let Some(target) = step
            .target()
            .filter(|target| self.last_targets.contains(target))
        {
            return Some(QuestFault::LastTask(target.to_string()));
        }
        match step {
            Step::Visit { place } => self.visit_fault(place),
            Step::Meet { npc } => self.person_fault(npc),
            Step::Talk { npc, about } => self.talk_fault(npc, about.as_deref()),
            Step::Kill { creature, count } => self.kill_fault(creature, *count),
            Step::Carry { item, count, npc } => self.carry_fault(item, *count, npc),
            Step::Wait { days } => {
                (!(1..=MAX_WAIT_DAYS).contains(days)).then_some(QuestFault::WaitDays(*days))
            }
        }
    }

    fn visit_fault(&self, place: &str) -> Option<QuestFault> {
        if self.zones.contains(&place) {
            return None;
        }
        if self.subzones.contains(&place) {
            return in_game_quests(place, self.seen);
        }
        Some(QuestFault::UnknownPlace(place.to_string()))
    }

    /// An NPC to meet or talk to. The giver passes here: a step can name the giver after a
    /// wait, and the check of the order decides (`structure`).
    fn person_fault(&self, npc: &str) -> Option<QuestFault> {
        if npc == self.giver {
            return None;
        }
        if !self.npcs.contains(&npc) {
            return Some(QuestFault::UnknownNpc(npc.to_string()));
        }
        in_game_quests(npc, self.seen)
    }

    fn talk_fault(&self, npc: &str, about: Option<&str>) -> Option<QuestFault> {
        if about.is_some_and(|about| !is_topic(about)) {
            return Some(QuestFault::BadTopic);
        }
        self.person_fault(npc)
    }

    /// A good is the goal of many game quests, so it is exempt from the overlap rule. The
    /// NPC keeps it.
    fn carry_fault(&self, item: &str, count: u8, npc: &str) -> Option<QuestFault> {
        if !self.goods.contains(&item) {
            return Some(QuestFault::UnknownGood(item.to_string()));
        }
        if !(1..=MAX_CARRY).contains(&count) {
            return Some(QuestFault::CarryCount(count));
        }
        self.person_fault(npc)
    }

    fn kill_fault(&self, creature: &str, count: u8) -> Option<QuestFault> {
        if creature == self.giver {
            return Some(QuestFault::KillGiver);
        }
        if !(1..=MAX_KILLS).contains(&count) {
            return Some(QuestFault::KillCount(count));
        }
        if !self.foes.contains(&creature) {
            return Some(QuestFault::UnknownFoe(creature.to_string()));
        }
        in_game_quests(creature, self.seen)
    }
}

/// The topic shows in a step line of the book, so it is short plain text with no `|`, the
/// escape mark of the game.
fn is_topic(about: &str) -> bool {
    plain_text(about, MAX_TOPIC_CHARS, MAX_TOPIC_BYTES).is_some() && !about.contains('|')
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
