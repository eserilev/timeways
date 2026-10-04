//! The rules of the quest log (GAMEPLAY.md 3.4): which change of a quest counts, and when a
//! step is open. The story program turns its quest file into these changes, and the quests
//! of the answer back into its own.

/// A day of the clock of the lines from the addon, not a day of the calendar.
pub const DAY_SECONDS: u64 = 24 * 3600;

/// An any-order set holds 2 or 3 steps.
pub const MIN_SET_STEPS: usize = 2;
pub const MAX_SET_STEPS: usize = 3;

/// What the rules read of a step. Every goal that is not a wait or a kill is `Other`.
#[derive(Clone, Copy)]
pub enum Goal {
    Wait { days: u8 },
    Kill { count: u8 },
    Other,
}

/// The steps from `first` to `last`, both in, can be done in any order.
#[derive(Clone, Copy)]
pub struct Span {
    pub first: usize,
    pub last: usize,
}

impl Span {
    #[must_use]
    pub fn contains(self, step: usize) -> bool {
        self.first <= step && step <= self.last
    }
}

/// One line of the quest file, as the rules read it. A time is in seconds.
pub enum Change {
    Offered {
        number: u64,
        giver: String,
        steps: Vec<Goal>,
        any_order: Option<Span>,
    },
    Accepted {
        number: u64,
        at: u64,
    },
    Declined {
        number: u64,
    },
    StepDone {
        number: u64,
        step: usize,
        at: u64,
    },
    Killed {
        number: u64,
        step: usize,
    },
    Abandoned {
        number: u64,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Offered,
    Accepted,
    Declined,
    Done,
    Abandoned,
}

/// One quest of the log.
pub struct Quest {
    /// The index of its offer among the changes.
    pub line: usize,
    pub number: u64,
    pub giver: String,
    pub progress: Progress,
}

/// How far a quest got.
pub struct Progress {
    pub steps: Vec<Goal>,
    pub any_order: Option<Span>,
    pub status: Status,
    pub accepted_at: Option<u64>,
    /// When each step was done, by index. Not `done`: in Lean, `done` is a tactic.
    pub done_times: Vec<Option<u64>>,
    /// The kills so far of each step, by index. Only a kill step counts any.
    pub kills: Vec<u8>,
    /// When the last step was done.
    pub done_at: Option<u64>,
}

impl Progress {
    /// A new offer. A span from a damaged file that does not fit the steps, or that breaks
    /// the rule of a set, counts as no span. A wait in a set could end early.
    fn offered(steps: &[Goal], any_order: Option<Span>) -> Progress {
        let fitting = match any_order {
            Some(span) if fits(span, steps) => Some(span),
            _ => None,
        };
        Progress {
            steps: steps.to_vec(),
            any_order: fitting,
            status: Status::Offered,
            accepted_at: None,
            done_times: vec![None; steps.len()],
            kills: vec![0; steps.len()],
            done_at: None,
        }
    }

    /// An index loop: Aeneas translates no iterator adapter (lean/README.md).
    #[must_use]
    pub fn steps_done(&self) -> usize {
        let mut count = 0;
        let mut step = 0;
        while step < self.done_times.len() {
            if self.done_times[step].is_some() {
                count += 1;
            }
            step += 1;
        }
        count
    }

    #[must_use]
    pub fn is_done(&self, step: usize) -> bool {
        step < self.done_times.len() && self.done_times[step].is_some()
    }

    /// The first step of the stage of this step. A stage is one step, or the whole
    /// any-order set.
    #[must_use]
    pub fn stage_start(&self, step: usize) -> usize {
        match self.any_order {
            Some(span) if span.contains(step) => span.first,
            _ => step,
        }
    }

    /// Is the step in the first stage: the first step, or the set that the quest starts
    /// with?
    #[must_use]
    pub fn in_first_stage(&self, step: usize) -> bool {
        step < self.steps.len() && self.stage_start(step) == 0
    }

    /// Can the player do this step now? The quest is accepted, the step is not done, and
    /// every step before its stage is done.
    #[must_use]
    pub fn is_open(&self, step: usize) -> bool {
        self.status == Status::Accepted
            && step < self.steps.len()
            && !self.is_done(step)
            && self.all_done_before(self.stage_start(step))
    }

    /// An index loop: Aeneas translates no iterator adapter (lean/README.md).
    fn all_done_before(&self, end: usize) -> bool {
        let mut step = 0;
        while step < end {
            if !self.is_done(step) {
                return false;
            }
            step += 1;
        }
        true
    }

    /// When the step became open: the accept, or the time of the last step done before its
    /// stage. None while it is not open.
    #[must_use]
    pub fn opened_at(&self, step: usize) -> Option<u64> {
        if !self.is_open(step) {
            return None;
        }
        match self.last_done_before(self.stage_start(step)) {
            Some(at) => Some(at),
            None => self.accepted_at,
        }
    }

    /// The latest time among the steps before `end`. An index loop: Aeneas translates no
    /// iterator adapter (lean/README.md).
    fn last_done_before(&self, end: usize) -> Option<u64> {
        let mut latest = None;
        let mut step = 0;
        while step < end && step < self.done_times.len() {
            latest = later(latest, self.done_times[step]);
            step += 1;
        }
        latest
    }

    /// When an open wait step is over. None for another step, or a wait that is not open.
    #[must_use]
    pub fn ready_at(&self, step: usize) -> Option<u64> {
        if step >= self.steps.len() {
            return None;
        }
        let Goal::Wait { days } = self.steps[step] else {
            return None;
        };
        match self.opened_at(step) {
            // `days` is a u8, so the product never overflows.
            Some(opened) => Some(opened.saturating_add(u64::from(days) * DAY_SECONDS)),
            None => None,
        }
    }

    /// A step counts only while it is open, and a wait never ends early. The last step
    /// finishes the quest.
    fn finish_step(&mut self, step: usize, at: u64) {
        let early = match self.ready_at(step) {
            Some(ready) => at < ready,
            None => false,
        };
        if !self.is_open(step) || early {
            return;
        }
        self.done_times[step] = Some(at);
        if self.steps_done() == self.steps.len() {
            self.status = Status::Done;
            self.done_at = Some(at);
        }
    }

    /// A kill counts only for an open kill step, and only up to its count.
    fn count_kill(&mut self, step: usize) {
        if step >= self.steps.len() {
            return;
        }
        let Goal::Kill { count } = self.steps[step] else {
            return;
        };
        if self.is_open(step) && self.kills[step] < count {
            self.kills[step] += 1;
        }
    }
}

/// A span fits when it holds 2 or 3 steps of the quest, and no wait.
fn fits(span: Span, steps: &[Goal]) -> bool {
    let fits_steps = span.first < span.last && span.last < steps.len();
    if !fits_steps {
        return false;
    }
    let size = span.last - span.first + 1;
    MIN_SET_STEPS <= size && size <= MAX_SET_STEPS && !holds_wait(steps, span)
}

/// An index loop: Aeneas translates no iterator adapter (lean/README.md).
fn holds_wait(steps: &[Goal], span: Span) -> bool {
    let mut step = span.first;
    while step <= span.last {
        if let Goal::Wait { .. } = steps[step] {
            return true;
        }
        step += 1;
    }
    false
}

fn later(latest: Option<u64>, at: Option<u64>) -> Option<u64> {
    match (latest, at) {
        (Some(old), Some(new)) if new > old => Some(new),
        (Some(old), _) => Some(old),
        (None, at) => at,
    }
}

/// The quests of a log, oldest first. At most one offer of each giver waits: a new offer
/// ends the old one of the same giver. A change that does not fit the state of its quest
/// changes nothing. An index loop: Aeneas translates no iterator adapter (lean/README.md).
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn quest_log(changes: &[Change]) -> Vec<Quest> {
    let mut quests = Vec::new();
    let mut line = 0;
    while line < changes.len() {
        apply(&mut quests, &changes[line], line);
        line += 1;
    }
    quests
}

fn apply(quests: &mut Vec<Quest>, change: &Change, line: usize) {
    match change {
        Change::Offered {
            number,
            giver,
            steps,
            any_order,
        } => {
            decline_waiting_offers_of(quests, giver);
            quests.push(Quest {
                line,
                number: *number,
                giver: giver.clone(),
                progress: Progress::offered(steps, *any_order),
            });
        }
        Change::Accepted { number, at } => {
            if let Some(index) = waiting_offer(quests, *number) {
                quests[index].progress.status = Status::Accepted;
                quests[index].progress.accepted_at = Some(*at);
            }
        }
        Change::Declined { number } => {
            if let Some(index) = waiting_offer(quests, *number) {
                quests[index].progress.status = Status::Declined;
            }
        }
        Change::Abandoned { number } => abandon(quests, *number),
        Change::Killed { number, step } => {
            if let Some(index) = find(quests, *number) {
                quests[index].progress.count_kill(*step);
            }
        }
        Change::StepDone { number, step, at } => {
            if let Some(index) = find(quests, *number) {
                quests[index].progress.finish_step(*step, *at);
            }
        }
    }
}

/// An index loop: Aeneas translates no iterator adapter (lean/README.md).
#[allow(
    clippy::ptr_arg,
    reason = "Aeneas compares a String, and a &str only as bytes"
)]
fn decline_waiting_offers_of(quests: &mut [Quest], giver: &String) {
    let mut index = 0;
    while index < quests.len() {
        let quest = &mut quests[index];
        if quest.progress.status == Status::Offered && quest.giver == *giver {
            quest.progress.status = Status::Declined;
        }
        index += 1;
    }
}

/// The first quest with this number. An index loop: Aeneas translates no iterator adapter
/// (lean/README.md).
fn find(quests: &[Quest], number: u64) -> Option<usize> {
    let mut index = 0;
    while index < quests.len() {
        if quests[index].number == number {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// The offer with this number that waits for an answer. An index loop: Aeneas translates
/// no iterator adapter (lean/README.md).
fn waiting_offer(quests: &[Quest], number: u64) -> Option<usize> {
    let mut index = 0;
    while index < quests.len() {
        let quest = &quests[index];
        if quest.number == number && quest.progress.status == Status::Offered {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Only an open quest can be abandoned: one that waits, or one that you hold. An index
/// loop: Aeneas translates no iterator adapter (lean/README.md).
fn abandon(quests: &mut [Quest], number: u64) {
    let mut index = 0;
    while index < quests.len() {
        let quest = &mut quests[index];
        let status = quest.progress.status;
        let open = status == Status::Offered || status == Status::Accepted;
        if quest.number == number && open {
            quest.progress.status = Status::Abandoned;
            return;
        }
        index += 1;
    }
}
