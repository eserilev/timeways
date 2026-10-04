//! The quest log: each change of a side quest, and the state of each quest and each of its
//! steps (GAMEPLAY.md 3.4, docs/plans/quest-variety.md 6).

use super::Step;
use hourglass::Tick;
use serde::{Deserialize, Serialize};

/// One change of the quest log. The quest file holds these lines, oldest first. The words
/// of a quest are not facts, so they live next to the history, as the hero does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum QuestChange {
    Offered {
        number: u64,
        at: Tick,
        giver: String,
        title: String,
        text: String,
        steps: Vec<Step>,
    },
    Accepted {
        number: u64,
        at: Tick,
    },
    Declined {
        number: u64,
        at: Tick,
    },
    StepDone {
        number: u64,
        step: usize,
        at: Tick,
    },
    /// One kill for the kill step with this index.
    Killed {
        number: u64,
        step: usize,
        at: Tick,
    },
    Abandoned {
        number: u64,
        at: Tick,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Offered,
    Accepted,
    Declined,
    Done,
    /// Given up after it was accepted, or before.
    Abandoned,
}

/// A quest as the log stands now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracked {
    pub number: u64,
    pub offered_at: Tick,
    pub giver: String,
    pub title: String,
    pub text: String,
    pub steps: Vec<Step>,
    pub status: Status,
    pub accepted_at: Option<Tick>,
    /// When each step was done, by index.
    pub done: Vec<Option<Tick>>,
    /// The kills so far of each step, by index. Only a kill step counts any.
    pub kills: Vec<u8>,
    /// When the last step was done.
    pub done_at: Option<Tick>,
}

impl Tracked {
    fn offered(
        number: u64,
        at: Tick,
        giver: &str,
        title: &str,
        text: &str,
        steps: &[Step],
    ) -> Self {
        Tracked {
            number,
            offered_at: at,
            giver: giver.to_string(),
            title: title.to_string(),
            text: text.to_string(),
            steps: steps.to_vec(),
            status: Status::Offered,
            accepted_at: None,
            done: vec![None; steps.len()],
            kills: vec![0; steps.len()],
            done_at: None,
        }
    }

    #[must_use]
    pub fn steps_done(&self) -> usize {
        self.done.iter().filter(|done| done.is_some()).count()
    }

    #[must_use]
    pub fn is_done(&self, step: usize) -> bool {
        self.done.get(step).is_some_and(Option::is_some)
    }

    /// Can the player do this step now? The quest is accepted, the step is not done, and
    /// every step before it is done.
    #[must_use]
    pub fn is_open(&self, step: usize) -> bool {
        self.status == Status::Accepted
            && step < self.steps.len()
            && !self.is_done(step)
            && (0..step).all(|before| self.is_done(before))
    }

    /// The steps that the player can do now, by index.
    #[must_use]
    pub fn open_steps(&self) -> Vec<usize> {
        (0..self.steps.len())
            .filter(|step| self.is_open(*step))
            .collect()
    }

    /// When the step became open: the accept, or the time of the last step done before it.
    /// None while it is not open.
    #[must_use]
    pub fn opened_at(&self, step: usize) -> Option<Tick> {
        if !self.is_open(step) {
            return None;
        }
        let before = self.done[..step].iter().flatten().max();
        before.copied().or(self.accepted_at)
    }

    /// The open kill step of this creature, by index.
    #[must_use]
    pub fn hunts(&self, name: &str) -> Option<usize> {
        let kill_of = |step: &usize| matches!(&self.steps[*step], Step::Kill { creature, .. } if creature == name);
        self.open_steps().into_iter().find(kill_of)
    }
}

/// The quests of a log, oldest first. At most one offer of each giver waits: a new offer
/// ends the old one of the same giver. A change that does not fit the state of its quest
/// changes nothing. A quest file is read again at each start, and a file can be damaged, so
/// these checks hold even when the story program is right.
#[must_use]
pub fn quest_log(changes: &[QuestChange]) -> Vec<Tracked> {
    let mut quests: Vec<Tracked> = Vec::new();
    for change in changes {
        apply(&mut quests, change);
    }
    quests
}

fn apply(quests: &mut Vec<Tracked>, change: &QuestChange) {
    match change {
        QuestChange::Offered {
            number,
            at,
            giver,
            title,
            text,
            steps,
        } => {
            let same_giver = |q: &&mut Tracked| q.status == Status::Offered && &q.giver == giver;
            for waiting in quests.iter_mut().filter(same_giver) {
                waiting.status = Status::Declined;
            }
            quests.push(Tracked::offered(*number, *at, giver, title, text, steps));
        }
        QuestChange::Accepted { number, at } => {
            if let Some(quest) = waiting_offer(quests, *number) {
                quest.status = Status::Accepted;
                quest.accepted_at = Some(*at);
            }
        }
        QuestChange::Declined { number, .. } => {
            if let Some(quest) = waiting_offer(quests, *number) {
                quest.status = Status::Declined;
            }
        }
        QuestChange::Abandoned { number, .. } => abandon(quests, *number),
        QuestChange::Killed { number, step, .. } => count_kill(quests, *number, *step),
        QuestChange::StepDone { number, step, at } => finish_step(quests, *number, *step, *at),
    }
}

fn find(quests: &mut [Tracked], number: u64) -> Option<&mut Tracked> {
    quests.iter_mut().find(|quest| quest.number == number)
}

/// A step counts only while it is open. The last step finishes the quest.
fn finish_step(quests: &mut [Tracked], number: u64, step: usize, at: Tick) {
    let Some(quest) = find(quests, number) else {
        return;
    };
    if !quest.is_open(step) {
        return;
    }
    quest.done[step] = Some(at);
    if quest.steps_done() == quest.steps.len() {
        quest.status = Status::Done;
        quest.done_at = Some(at);
    }
}

/// A kill counts only for an open kill step, and only up to its count.
fn count_kill(quests: &mut [Tracked], number: u64, step: usize) {
    let Some(quest) = find(quests, number) else {
        return;
    };
    let Some(Step::Kill { count, .. }) = quest.steps.get(step) else {
        return;
    };
    if quest.is_open(step) && quest.kills[step] < *count {
        quest.kills[step] += 1;
    }
}

/// Only an open quest can be abandoned: one that waits, or one that you hold.
fn abandon(quests: &mut [Tracked], number: u64) {
    let open = |q: &&mut Tracked| {
        q.number == number && matches!(q.status, Status::Offered | Status::Accepted)
    };
    if let Some(quest) = quests.iter_mut().find(open) {
        quest.status = Status::Abandoned;
    }
}

fn waiting_offer(quests: &mut [Tracked], number: u64) -> Option<&mut Tracked> {
    quests
        .iter_mut()
        .find(|q| q.number == number && q.status == Status::Offered)
}

/// Numbers are never reused, so a line names one quest for good.
#[must_use]
pub fn next_number(changes: &[QuestChange]) -> u64 {
    let offers = changes
        .iter()
        .filter(|change| matches!(change, QuestChange::Offered { .. }));
    u64::try_from(offers.count()).map_or(u64::MAX, |count| count.saturating_add(1))
}
