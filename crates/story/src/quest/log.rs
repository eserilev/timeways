//! The quest log: each change of a side quest, and the state of each quest and each of its
//! steps (GAMEPLAY.md 3.4, docs/plans/quest-variety.md 6).

use super::{AnyOrder, Genre, Step};
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use timeways_rules::quest as rules;

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
        /// None in an old quest file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        genre: Option<Genre>,
        /// None in an old quest file.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        any_order: Option<AnyOrder>,
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

impl QuestChange {
    /// The number of the quest that the change belongs to.
    #[must_use]
    pub fn number(&self) -> u64 {
        match self {
            QuestChange::Offered { number, .. }
            | QuestChange::Accepted { number, .. }
            | QuestChange::Declined { number, .. }
            | QuestChange::StepDone { number, .. }
            | QuestChange::Killed { number, .. }
            | QuestChange::Abandoned { number, .. } => *number,
        }
    }
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
    pub genre: Option<Genre>,
    pub any_order: Option<AnyOrder>,
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
    /// The quest of the log, with the words of its offer. None when the line of the offer is
    /// no offer, which the rules never give.
    fn from_rules(quest: rules::Quest, changes: &[QuestChange]) -> Option<Self> {
        let Some(QuestChange::Offered {
            number,
            at,
            giver,
            title,
            text,
            steps,
            genre,
            ..
        }) = changes.get(quest.line)
        else {
            return None;
        };
        let progress = quest.progress;
        Some(Tracked {
            number: *number,
            offered_at: *at,
            giver: giver.clone(),
            title: title.clone(),
            text: text.clone(),
            steps: steps.clone(),
            genre: *genre,
            any_order: progress.any_order.map(AnyOrder::from_rules),
            status: Status::from_rules(progress.status),
            accepted_at: progress.accepted_at.map(Tick),
            done: progress
                .done_times
                .iter()
                .map(|done| done.map(Tick))
                .collect(),
            kills: progress.kills,
            done_at: progress.done_at.map(Tick),
        })
    }

    /// The quest as the rules see it. Each question about a step goes to the rules, so the
    /// answer is the one that the proofs read.
    fn progress(&self) -> rules::Progress {
        rules::Progress {
            steps: self.steps.iter().map(goal_of).collect(),
            any_order: self.any_order.map(AnyOrder::for_rules),
            status: self.status.for_rules(),
            accepted_at: self.accepted_at.map(|at| at.0),
            done_times: self.done.iter().map(|done| done.map(|at| at.0)).collect(),
            kills: self.kills.clone(),
            done_at: self.done_at.map(|at| at.0),
        }
    }

    #[must_use]
    pub fn steps_done(&self) -> usize {
        self.progress().steps_done()
    }

    #[must_use]
    pub fn is_done(&self, step: usize) -> bool {
        self.progress().is_done(step)
    }

    /// Is the step in the first stage: the first step, or the set that the quest starts
    /// with?
    #[must_use]
    pub fn in_first_stage(&self, step: usize) -> bool {
        self.progress().in_first_stage(step)
    }

    /// Can the player do this step now? The quest is accepted, the step is not done, and
    /// every step before its stage is done.
    #[must_use]
    pub fn is_open(&self, step: usize) -> bool {
        self.progress().is_open(step)
    }

    /// The steps that the player can do now, by index.
    #[must_use]
    pub fn open_steps(&self) -> Vec<usize> {
        let progress = self.progress();
        (0..self.steps.len())
            .filter(|step| progress.is_open(*step))
            .collect()
    }

    /// When the step became open: the accept, or the time of the last step done before its
    /// stage. None while it is not open.
    #[must_use]
    pub fn opened_at(&self, step: usize) -> Option<Tick> {
        self.progress().opened_at(step).map(Tick)
    }

    /// When an open wait step is over. None for another step, or a wait that is not open.
    #[must_use]
    pub fn ready_at(&self, step: usize) -> Option<Tick> {
        self.progress().ready_at(step).map(Tick)
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
/// these checks hold even when the story program is right. The rules live in
/// timeways-rules, where Lean proves them (lean/README.md).
#[must_use]
pub fn quest_log(changes: &[QuestChange]) -> Vec<Tracked> {
    let lines: Vec<rules::Change> = changes.iter().map(QuestChange::for_rules).collect();
    rules::quest_log(&lines)
        .into_iter()
        .filter_map(|quest| Tracked::from_rules(quest, changes))
        .collect()
}

impl QuestChange {
    fn for_rules(&self) -> rules::Change {
        match self {
            QuestChange::Offered {
                number,
                giver,
                steps,
                any_order,
                ..
            } => rules::Change::Offered {
                number: *number,
                giver: giver.clone(),
                steps: steps.iter().map(goal_of).collect(),
                any_order: any_order.map(AnyOrder::for_rules),
            },
            QuestChange::Accepted { number, at } => rules::Change::Accepted {
                number: *number,
                at: at.0,
            },
            QuestChange::Declined { number, .. } => rules::Change::Declined { number: *number },
            QuestChange::StepDone { number, step, at } => rules::Change::StepDone {
                number: *number,
                step: *step,
                at: at.0,
            },
            QuestChange::Killed { number, step, .. } => rules::Change::Killed {
                number: *number,
                step: *step,
            },
            QuestChange::Abandoned { number, .. } => rules::Change::Abandoned { number: *number },
        }
    }
}

fn goal_of(step: &Step) -> rules::Goal {
    match step {
        Step::Wait { days } => rules::Goal::Wait { days: *days },
        Step::Kill { count, .. } => rules::Goal::Kill { count: *count },
        _ => rules::Goal::Other,
    }
}

impl AnyOrder {
    fn for_rules(self) -> rules::Span {
        rules::Span {
            first: self.first,
            last: self.last,
        }
    }

    fn from_rules(span: rules::Span) -> Self {
        AnyOrder {
            first: span.first,
            last: span.last,
        }
    }
}

impl Status {
    fn for_rules(self) -> rules::Status {
        match self {
            Status::Offered => rules::Status::Offered,
            Status::Accepted => rules::Status::Accepted,
            Status::Declined => rules::Status::Declined,
            Status::Done => rules::Status::Done,
            Status::Abandoned => rules::Status::Abandoned,
        }
    }

    fn from_rules(status: rules::Status) -> Self {
        match status {
            rules::Status::Offered => Status::Offered,
            rules::Status::Accepted => Status::Accepted,
            rules::Status::Declined => Status::Declined,
            rules::Status::Done => Status::Done,
            rules::Status::Abandoned => Status::Abandoned,
        }
    }
}

/// Numbers are never reused, so a line names one quest for good.
#[must_use]
pub fn next_number(changes: &[QuestChange]) -> u64 {
    let offers = changes
        .iter()
        .filter(|change| matches!(change, QuestChange::Offered { .. }));
    u64::try_from(offers.count()).map_or(u64::MAX, |count| count.saturating_add(1))
}
