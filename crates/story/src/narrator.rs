//! The narrator: one short line at a big moment, within a budget (GAMEPLAY.md 3.2). The
//! words of its prompt live here.

use crate::check::voice_text;
use crate::hero::OWN_WORDS;
use crate::house::{HOUSE_RULES, fenced};
use crate::moments::Moment;
use crate::places::InstanceKind;
use crate::samples::{self, Voice};
use hourglass::Tick;
use std::fmt::Write;

/// About 50 words. The prompt asks for 25.
pub const MAX_LINE_CHARS: usize = 300;

/// The limit of the bridge for a narrator line (Gnomish Relay SPEC.md 9.8).
pub const MAX_LINE_BYTES: usize = 1000;

const LINES_PER_HOUR: usize = 3;
const HOUR: u64 = 3600;

/// The persona of the narrator: its manner only, never lore (GAMEPLAY.md 3.2.1). The
/// chronicle shares it, and an NPC never gets it.
pub const PERSONA: &str = include_str!("../data/narrator.txt");

const TASK: &str = "Tell the moment below in one line of at most 25 words.";

/// The author's note. A model weighs the end of a prompt most.
const NOTE: &str = "\
Remember: serious, concrete, and short. Call the player \"our hero\", and tell nothing of \
what comes next.
Answer with the line only.";

/// Counts the lines of the last hour of game time, so the narrator talks little.
#[derive(Debug, Default)]
pub struct Budget {
    /// The times of the last lines, oldest first. No heap, so Kani proves the budget fast.
    spoken: [Option<Tick>; LINES_PER_HOUR],
}

impl Budget {
    /// True when the narrator has a line left at `at`. That line then counts.
    pub fn take(&mut self, at: Tick) -> bool {
        let oldest = self.spoken[0];
        if oldest.is_some_and(|first| at.0.saturating_sub(first.0) < HOUR) {
            return false;
        }
        self.spoken.rotate_left(1);
        self.spoken[LINES_PER_HOUR - 1] = Some(at);
        true
    }
}

/// `turn` picks the golden samples of the prompt.
#[must_use]
pub fn prompt(moment: &Moment, portrait: Option<&str>, turn: usize) -> String {
    let mut prompt = format!("{PERSONA}\n{HOUSE_RULES}\n\n{TASK}");
    if let Some(portrait) = portrait {
        let _ = write!(prompt, "\n\n{OWN_WORDS}\n{}", fenced(portrait));
    }
    let samples = samples::section(Voice::NarratorLine, turn);
    let what = fenced(&what_happened(moment));
    let _ = write!(prompt, "\n\n{samples}\n\nThe moment:\n{what}\n\n{NOTE}");
    prompt
}

fn what_happened(moment: &Moment) -> String {
    match moment {
        Moment::Flavor { what } => what.clone(),
        Moment::Titled { title } => {
            format!("The player earned the title \"{title}\" in their journal.")
        }
        Moment::FirstKill { foe } => format!("The player defeated {foe} for the first time."),
        Moment::SlainAgain { killer, times } => {
            format!("{killer} killed the player again. That makes {times} times.")
        }
        Moment::Slapped { npc, times } => {
            format!("The player slapped {npc}. That makes {times} times.")
        }
        Moment::LevelUp { level } => format!("The player reached level {level}."),
        Moment::NewZone { zone } => format!("The player arrived in {zone} for the first time."),
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Dungeon,
        } => format!("The player entered the dungeon {zone} for the first time."),
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Raid,
        } => format!("The player entered the raid {zone} for the first time."),
        Moment::QuestMarked { mark, quest } => {
            format!(
                "During the quest \"{quest}\", a lasting effect came on the player: \"{mark}\"."
            )
        }
        Moment::FirstCapital { city } => {
            format!("The player arrived in the capital city {city} for the first time.")
        }
        Moment::ClassQuestDone { title } => {
            format!("The player finished \"{title}\", a quest of their class.")
        }
    }
}

/// The line as the player sees it, or None when it breaks a rule. A narrator line gets no
/// retry: silence costs nothing.
#[must_use]
pub fn checked_line(text: &str) -> Option<String> {
    voice_text(text, MAX_LINE_CHARS, MAX_LINE_BYTES)
}
