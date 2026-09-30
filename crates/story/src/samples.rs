//! The golden samples of each voice (GAMEPLAY.md 3.2.1). A prompt carries a few of them,
//! in turn, so that no one sample sets the words of every answer.

use crate::check::data_lines;
use crate::house::{bulleted, fenced};

const NARRATOR_LINES: &str = include_str!("../data/samples/narrator_lines.txt");
const CHAPTERS: &str = include_str!("../data/samples/chapters.txt");
const NPC_REPLIES: &str = include_str!("../data/samples/npc_replies.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Voice {
    NarratorLine,
    Chapter,
    NpcReply,
}

impl Voice {
    #[must_use]
    pub fn samples(self) -> Vec<&'static str> {
        let file = match self {
            Voice::NarratorLine => NARRATOR_LINES,
            Voice::Chapter => CHAPTERS,
            Voice::NpcReply => NPC_REPLIES,
        };
        data_lines(file).collect()
    }

    /// A chapter sample is long, so a chapter prompt carries fewer of them.
    fn per_prompt(self) -> usize {
        match self {
            Voice::NarratorLine => 3,
            Voice::Chapter | Voice::NpcReply => 2,
        }
    }

    fn heading(self) -> &'static str {
        match self {
            Voice::NarratorLine | Voice::Chapter => {
                "Samples of your voice, about other heroes. Copy the manner, never the words:"
            }
            Voice::NpcReply => {
                "Samples of how people of this world speak, about other matters. Copy the \
                 manner, never the words:"
            }
        }
    }
}

/// The samples of one prompt. `turn` picks the first one, so two prompts in a row differ.
#[must_use]
pub fn rotated(voice: Voice, turn: usize) -> Vec<&'static str> {
    let all = voice.samples();
    let count = voice.per_prompt().min(all.len());
    let start = turn % all.len().max(1);
    (0..count)
        .map(|step| all[(start + step) % all.len()])
        .collect()
}

/// The samples of one prompt under their heading, fenced as data.
#[must_use]
pub fn section(voice: Voice, turn: usize) -> String {
    let samples = rotated(voice, turn);
    format!("{}\n{}", voice.heading(), fenced(&bulleted(&samples)))
}

/// Every sample of every voice, for the check against a copy.
#[must_use]
pub fn every_sample() -> Vec<&'static str> {
    let voices = [Voice::NarratorLine, Voice::Chapter, Voice::NpcReply];
    voices.into_iter().flat_map(Voice::samples).collect()
}
