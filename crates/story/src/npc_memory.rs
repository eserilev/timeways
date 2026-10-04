//! What an NPC remembers of the player in /talk (GAMEPLAY.md 3.5). Each memory is a row
//! of the world, so the model gets no past to invent.

use crate::character::Character;
use crate::check::mentions;
use crate::journal::{Deed, DeedRow, deeds_with_events};
use crate::learned::{Rumor, cut_at_word};
use crate::quest::{QuestChange, Status, Tracked, quest_log};
use hourglass::{EventId, Tick};
use std::cmp::Reverse;

/// The memories of one prompt. More costs tokens, and the NPC uses one at most.
pub const MAX_MEMORIES: usize = 5;

/// The answers of the NPC that it remembers.
const REMEMBERED_ANSWERS: usize = 2;

/// The quests of the NPC that it remembers: the newest offers.
const REMEMBERED_QUESTS: usize = 2;

/// The deeds in the zone of the NPC that it remembers.
const REMEMBERED_NEAR: usize = 2;

/// About 40 tokens. A rumor is cut at a whole word past this.
pub const RUMOR_CHARS: usize = 160;

/// The longest line of a memory: the time words, the template, and the longest rumor or
/// two names of `MAX_NAME_BYTES`.
pub const MAX_MEMORY_CHARS: usize = 280;

const HOUR_SECONDS: u64 = 3_600;
const DAY_SECONDS: u64 = 24 * HOUR_SECONDS;

/// The first meeting needs this age to be a memory. A younger one is this visit.
const FIRST_MEETING_AGE: u64 = HOUR_SECONDS;

/// What the player did, as far as an NPC can remember it.
pub struct Past<'a> {
    pub character: &'a Character,
    /// Each rumor with its `learned` row, oldest first.
    pub rumors: Vec<(u64, &'a Rumor)>,
    pub quests: &'a [QuestChange],
    /// The name of the character. A rumor that holds it is no memory (5.11).
    pub own_name: &'a str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    pub at: Tick,
    pub recall: Recall,
    /// The rows behind the memory. The talk call reads each one.
    pub sources: Vec<Source>,
}

/// The kinds of memory, in the order that a tie in time keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recall {
    Said {
        text: String,
    },
    GaveQuest {
        title: String,
        ending: QuestEnding,
    },
    FirstMet,
    KilledYou,
    YouDefeatedIt,
    DefeatedNear {
        foe: String,
        place: String,
    },
    DiedNear {
        place: String,
        killer: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestEnding {
    Waiting,
    OnIt,
    Finished,
    TurnedDown,
    GaveUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Event(EventId),
    Learned(u64),
    Quest(u64),
}

/// How close a memory is to the NPC. A closer tier comes first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    Between,
    Met,
    Near,
}

/// At most `MAX_MEMORIES`, newest first.
#[must_use]
pub fn memories(past: &Past<'_>, npc: &str, now: Tick) -> Vec<Memory> {
    let deeds = deeds_with_events(past.character.world(), past.character.you());
    let mut candidates = answers_heard(past, npc);
    candidates.extend(quests_given(past.quests, npc));
    candidates.extend(first_meeting(past.character, npc, now));
    candidates.extend(deeds_with(&deeds, npc));
    candidates.extend(deeds_near(&deeds, npc, past.character));
    chosen(candidates)
}

/// A rumor that names the character drops out, and no older one takes its place.
fn answers_heard(past: &Past<'_>, npc: &str) -> Vec<Memory> {
    let newest = past
        .rumors
        .iter()
        .rev()
        .filter(|(_, rumor)| rumor.npc == npc);
    newest
        .take(REMEMBERED_ANSWERS)
        .filter(|(_, rumor)| !mentions(&rumor.text, past.own_name))
        .map(|(row, rumor)| Memory {
            at: rumor.at,
            recall: Recall::Said {
                text: cut_at_word(&rumor.text, RUMOR_CHARS),
            },
            sources: vec![Source::Learned(*row)],
        })
        .collect()
}

fn quests_given(changes: &[QuestChange], npc: &str) -> Vec<Memory> {
    let quests = quest_log(changes);
    quests
        .iter()
        .filter(|quest| quest.giver == npc)
        .filter_map(|quest| {
            let answered = changes
                .iter()
                .any(|change| number_of(change) == quest.number && is_answer(change));
            let rows = rows_of_quest(changes, quest.number);
            let ending = quest_ending(quest, answered)?;
            Some(Memory {
                at: quest.offered_at,
                recall: Recall::GaveQuest {
                    title: quest.title.clone(),
                    ending,
                },
                sources: rows.into_iter().map(Source::Quest).collect(),
            })
        })
        .collect()
}

/// The row of a change is its place in the log.
fn rows_of_quest(changes: &[QuestChange], number: u64) -> Vec<u64> {
    (0..)
        .zip(changes)
        .filter(|(_, change)| number_of(change) == number)
        .map(|(row, _)| row)
        .collect()
}

fn number_of(change: &QuestChange) -> u64 {
    match change {
        QuestChange::Offered { number, .. }
        | QuestChange::Accepted { number, .. }
        | QuestChange::Declined { number, .. }
        | QuestChange::StepDone { number, .. }
        | QuestChange::Killed { number, .. }
        | QuestChange::Abandoned { number, .. } => *number,
    }
}

fn is_answer(change: &QuestChange) -> bool {
    matches!(
        change,
        QuestChange::Declined { .. } | QuestChange::Abandoned { .. }
    )
}

/// `answered` is true when a `Declined` or `Abandoned` row of the quest exists. A declined
/// quest with no such row is an offer that a newer one replaced: you did nothing.
fn quest_ending(quest: &Tracked, answered: bool) -> Option<QuestEnding> {
    match quest.status {
        Status::Offered => Some(QuestEnding::Waiting),
        Status::Accepted => Some(QuestEnding::OnIt),
        Status::Done => Some(QuestEnding::Finished),
        Status::Declined => answered.then_some(QuestEnding::TurnedDown),
        Status::Abandoned => Some(QuestEnding::GaveUp),
    }
}

fn first_meeting(character: &Character, npc: &str, now: Tick) -> Option<Memory> {
    let (event, at) = character.first_met(npc)?;
    (now.0.saturating_sub(at.0) >= FIRST_MEETING_AGE).then(|| Memory {
        at,
        recall: Recall::FirstMet,
        sources: vec![Source::Event(event)],
    })
}

/// The newest death that the NPC caused, and the newest defeat of the NPC.
fn deeds_with(deeds: &[DeedRow], npc: &str) -> Vec<Memory> {
    let newest = deeds.iter().rev();
    let killed_you = newest.clone().find_map(|row| match &row.deed {
        Deed::Died {
            killer: Some(killer),
            at,
            ..
        } if killer == npc => Some(memory_of(row, *at, Recall::KilledYou)),
        _ => None,
    });
    let you_defeated_it = newest.clone().find_map(|row| match &row.deed {
        Deed::Defeated { foe, at, .. } if foe == npc => {
            Some(memory_of(row, *at, Recall::YouDefeatedIt))
        }
        _ => None,
    });
    killed_you.into_iter().chain(you_defeated_it).collect()
}

/// In the zone of the NPC: the newest defeat of each other foe, and the newest death that
/// the NPC did not cause.
fn deeds_near(deeds: &[DeedRow], npc: &str, character: &Character) -> Vec<Memory> {
    let Some(zone) = character.zone_of_npc(npc) else {
        return Vec::new();
    };
    let mut near: Vec<Memory> = Vec::new();
    let mut died = None;
    for row in deeds.iter().rev() {
        let Some(place) = place_in_zone(&row.deed, zone, character) else {
            continue;
        };
        match &row.deed {
            Deed::Defeated { foe, at, .. } if foe != npc && !defeated_before(&near, foe) => {
                let recall = Recall::DefeatedNear {
                    foe: foe.clone(),
                    place,
                };
                near.push(memory_of(row, *at, recall));
            }
            Deed::Died { killer, at, .. } if died.is_none() && killer.as_deref() != Some(npc) => {
                let recall = Recall::DiedNear {
                    place,
                    killer: killer.clone(),
                };
                died = Some(memory_of(row, *at, recall));
            }
            _ => {}
        }
    }
    near.extend(died);
    near
}

/// The place of a deed, when it lies in `zone`.
fn place_in_zone(deed: &Deed, zone: &str, character: &Character) -> Option<String> {
    let place = match deed {
        Deed::Defeated { place, .. } | Deed::Died { place, .. } => place.as_deref()?,
        _ => return None,
    };
    (character.zone_of_place(place) == Some(zone)).then(|| place.to_string())
}

fn defeated_before(near: &[Memory], foe: &str) -> bool {
    near.iter().any(
        |memory| matches!(&memory.recall, Recall::DefeatedNear { foe: kept, .. } if kept == foe),
    )
}

fn memory_of(row: &DeedRow, at: Tick, recall: Recall) -> Memory {
    Memory {
        at,
        recall,
        sources: row.events.iter().copied().map(Source::Event).collect(),
    }
}

/// The caps by kind, then the tiers, then the cut. The kept memories come newest first.
fn chosen(mut candidates: Vec<Memory>) -> Vec<Memory> {
    // A stable sort, so a tie in time keeps the order of the kinds.
    candidates.sort_by_key(|memory| Reverse(memory.at));
    let mut kept = capped(candidates);
    kept.sort_by_key(|memory| tier(&memory.recall));
    kept.truncate(MAX_MEMORIES);
    kept.sort_by_key(|memory| Reverse(memory.at));
    kept
}

/// `newest_first` keeps the newest quests and the newest deeds nearby.
fn capped(newest_first: Vec<Memory>) -> Vec<Memory> {
    let mut quests = 0;
    let mut near = 0;
    newest_first
        .into_iter()
        .filter(|memory| {
            let (count, cap) = match memory.recall {
                Recall::GaveQuest { .. } => (&mut quests, REMEMBERED_QUESTS),
                Recall::DefeatedNear { .. } | Recall::DiedNear { .. } => {
                    (&mut near, REMEMBERED_NEAR)
                }
                _ => return true,
            };
            *count += 1;
            *count <= cap
        })
        .collect()
}

fn tier(recall: &Recall) -> Tier {
    match recall {
        Recall::Said { .. }
        | Recall::GaveQuest { .. }
        | Recall::KilledYou
        | Recall::YouDefeatedIt => Tier::Between,
        Recall::FirstMet => Tier::Met,
        Recall::DefeatedNear { .. } | Recall::DiedNear { .. } => Tier::Near,
    }
}

/// One line of the prompt. It says "the player", never a name.
#[must_use]
pub fn line(memory: &Memory, now: Tick) -> String {
    let when = when(memory.at, now);
    match &memory.recall {
        Recall::Said { text } => format!("{when}: you told the player \"{text}\""),
        Recall::GaveQuest { title, ending } => format!(
            "{when}: you gave the player your quest \"{title}\". {}",
            ending_words(*ending)
        ),
        Recall::FirstMet => format!("{when}: you met the player for the first time."),
        Recall::KilledYou => format!("{when}: you killed the player in a fight."),
        Recall::YouDefeatedIt => format!("{when}: the player defeated you in a fight."),
        Recall::DefeatedNear { foe, place } => {
            format!("{when}: the player defeated {foe} in {place}.")
        }
        Recall::DiedNear {
            place,
            killer: Some(killer),
        } => format!("{when}: the player died in {place}, killed by {killer}."),
        Recall::DiedNear {
            place,
            killer: None,
        } => format!("{when}: the player died in {place}."),
    }
}

fn ending_words(ending: QuestEnding) -> &'static str {
    match ending {
        QuestEnding::Waiting => "They have not answered yet.",
        QuestEnding::OnIt => "They are still on it.",
        QuestEnding::Finished => "They finished it.",
        QuestEnding::TurnedDown => "They turned it down.",
        QuestEnding::GaveUp => "They gave it up.",
    }
}

/// The age in words, never a date: the world is in 25 ADP. A `then` after `now` is a
/// clock that went back, and counts as no age.
#[must_use]
pub fn when(then: Tick, now: Tick) -> String {
    let age = now.0.saturating_sub(then.0);
    let days = age / DAY_SECONDS;
    let words = match days {
        _ if age < HOUR_SECONDS => "Less than an hour ago",
        0 => "Hours ago",
        1 => "Yesterday",
        2..=6 => return format!("{} days ago", number_word(days)),
        7..=13 => "A week ago",
        14..=29 => return format!("{} weeks ago", number_word(days / 7)),
        30..=59 => "A month ago",
        60..=364 => return format!("{} months ago", number_word(days / 30)),
        _ => "Over a year ago",
    };
    words.to_string()
}

/// "Two" to "Twelve", at the start of a line. `when` asks for no other number.
fn number_word(n: u64) -> &'static str {
    match n {
        2 => "Two",
        3 => "Three",
        4 => "Four",
        5 => "Five",
        6 => "Six",
        7 => "Seven",
        8 => "Eight",
        9 => "Nine",
        10 => "Ten",
        11 => "Eleven",
        _ => "Twelve",
    }
}
