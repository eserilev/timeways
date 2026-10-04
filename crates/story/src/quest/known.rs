//! What the world of the player holds, and the rules of 3.4 for each step against it.

use super::emotes::{is_cruel_target, quest_emotes};
use super::variety::Recent;
use super::{
    MAX_CARRY, MAX_KILLS, MAX_LEVELS_AHEAD, MAX_TOPIC_BYTES, MAX_TOPIC_CHARS, MAX_WAIT_DAYS,
    QuestFault, Step,
};
use crate::check::{mentions, plain_text};
use crate::vocabulary::LEVELS;

/// The highest level of the game.
const MAX_LEVEL: i64 = LEVELS.max;
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
    /// Your level, when the world knows it.
    pub level: Option<i64>,
    /// The dungeons and raids that you entered (`Character::dungeons_entered`).
    pub dungeons: Vec<&'a str>,
    /// The bosses that you defeated in a dungeon or a raid.
    pub bosses: Vec<&'a str>,
    /// The quests of the game that you hold or read, by title.
    pub game_quests: Vec<&'a str>,
    /// The quests of the game that you turned in, by title.
    pub game_quests_done: Vec<&'a str>,
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

    /// The dungeons and raids that an enter step can name, as the check allows them.
    #[must_use]
    pub fn dungeons(&self) -> Vec<&str> {
        let enter = |dungeon: &str| Step::Enter {
            dungeon: dungeon.to_string(),
        };
        let allowed = |dungeon: &&str| self.step_fault(&enter(dungeon)).is_none();
        self.dungeons.iter().copied().filter(allowed).collect()
    }

    /// The bosses that a defeat step can name, as the check allows them.
    #[must_use]
    pub fn bosses(&self) -> Vec<&str> {
        let defeat = |boss: &str| Step::Defeat {
            boss: boss.to_string(),
        };
        let allowed = |boss: &&str| self.step_fault(&defeat(boss)).is_none();
        self.bosses.iter().copied().filter(allowed).collect()
    }

    /// The quests of the game that a game quest step can name, as the check allows them.
    #[must_use]
    pub fn game_quest_titles(&self) -> Vec<&str> {
        let turn_in = |title: &str| Step::GameQuest {
            title: title.to_string(),
        };
        let allowed = |title: &&str| self.step_fault(&turn_in(title)).is_none();
        self.game_quests.iter().copied().filter(allowed).collect()
    }

    /// True when a level step can ask for a level: you have one, and it is below 60.
    #[must_use]
    pub fn can_level(&self) -> bool {
        self.level.is_some_and(|level| level < MAX_LEVEL)
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
            Step::Visit { place } | Step::VisitAt { place, .. } => self.visit_fault(place),
            Step::Meet { npc } => self.person_fault(npc),
            Step::Talk { npc, about } => self.talk_fault(npc, about.as_deref()),
            Step::Kill { creature, count } => self.kill_fault(creature, *count),
            Step::Carry { item, count, npc } => self.carry_fault(item, *count, npc),
            Step::Emote { emote, npc, place } => {
                self.emote_fault(emote, npc.as_deref(), place.as_deref())
            }
            Step::Slap { npc } => self.slap_fault(npc),
            Step::Level { level } => self.level_fault(*level),
            Step::Enter { dungeon } => {
                self.named_fault(dungeon, &self.dungeons, QuestFault::UnknownDungeon)
            }
            Step::Defeat { boss } => self.named_fault(boss, &self.bosses, QuestFault::UnknownBoss),
            Step::GameQuest { title } => self.game_quest_fault(title),
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

    fn emote_fault(
        &self,
        emote: &str,
        npc: Option<&str>,
        place: Option<&str>,
    ) -> Option<QuestFault> {
        if !quest_emotes().contains(&emote) {
            return Some(QuestFault::UnknownEmote(emote.to_string()));
        }
        match (npc, place) {
            (Some(npc), None) => self.person_fault(npc),
            (None, Some(place)) => self.visit_fault(place),
            _ => Some(QuestFault::EmoteTarget),
        }
    }

    /// A slap is a slap: it costs trust as any slap does. So it never names the giver, or
    /// an NPC whose name holds a word of the cruelty list.
    fn slap_fault(&self, npc: &str) -> Option<QuestFault> {
        if npc == self.giver {
            return Some(QuestFault::SlapGiver);
        }
        if is_cruel_target(npc) {
            return Some(QuestFault::CruelTarget(npc.to_string()));
        }
        self.person_fault(npc)
    }

    /// 1 to 3 levels above yours, and at most 60. With no level, no level step.
    fn level_fault(&self, level: u8) -> Option<QuestFault> {
        let wanted = i64::from(level);
        let reachable = self.level.is_some_and(|now| {
            wanted > now && wanted <= now + MAX_LEVELS_AHEAD && wanted <= MAX_LEVEL
        });
        (!reachable).then_some(QuestFault::LevelOutOfReach(level))
    }

    /// A dungeon or a boss of your world, and in no quest of the game that you read.
    fn named_fault(
        &self,
        name: &str,
        list: &[&str],
        unknown: fn(String) -> QuestFault,
    ) -> Option<QuestFault> {
        if !list.contains(&name) {
            return Some(unknown(name.to_string()));
        }
        in_game_quests(name, self.seen)
    }

    /// The step names a quest of the game by design, so the overlap rule skips its title.
    fn game_quest_fault(&self, title: &str) -> Option<QuestFault> {
        if self.game_quests_done.contains(&title) {
            return Some(QuestFault::GameQuestDone(title.to_string()));
        }
        if !self.game_quests.contains(&title) {
            return Some(QuestFault::UnknownGameQuest(title.to_string()));
        }
        None
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
