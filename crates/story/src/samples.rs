//! The golden samples of each voice (GAMEPLAY.md 3.2.1). A prompt carries a few of them,
//! in turn, so that no one sample sets the words of every answer. A sample names real
//! places and people of 25 ADP. The copy check and the log of names in no fact stop a
//! model that takes a name of a sample into its answer.

use crate::check::data_lines;
use crate::house::{bulleted, fenced};

const NARRATOR_LINES: &str = include_str!("../data/samples/narrator_lines.txt");
const CHAPTERS: &str = include_str!("../data/samples/chapters.txt");
const NPC_REPLIES: &str = include_str!("../data/samples/npc_replies.txt");
const SUMMARIES: &str = include_str!("../data/samples/summaries.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Voice {
    NarratorLine,
    Chapter,
    NpcReply,
    Summary,
}

/// A narrator line with what it was told from, so a model sees how a line uses the
/// moment and its lore.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineSample {
    pub moment: &'static str,
    /// The race and the class of the hero: "a Forsaken warlock".
    pub who: Option<&'static str>,
    pub lore: Option<&'static str>,
    /// How the line names the hero, as the prompt says it (`narrator::Naming`).
    pub hero: &'static str,
    pub line: &'static str,
}

impl Voice {
    #[must_use]
    pub fn samples(self) -> Vec<&'static str> {
        match self {
            Voice::NarratorLine => line_samples().iter().map(|sample| sample.line).collect(),
            Voice::Chapter => data_lines(CHAPTERS).collect(),
            Voice::NpcReply => data_lines(NPC_REPLIES).collect(),
            Voice::Summary => data_lines(SUMMARIES).collect(),
        }
    }

    /// A chapter sample is long, so a chapter prompt carries fewer of them.
    #[must_use]
    pub fn per_prompt(self) -> usize {
        match self {
            Voice::NarratorLine => 3,
            Voice::Chapter | Voice::NpcReply | Voice::Summary => 2,
        }
    }

    fn heading(self) -> &'static str {
        match self {
            Voice::NarratorLine => {
                "Samples of your voice, about other heroes. Each one shows a moment, its lore, \
                 how to name the hero, and the line. Copy the manner, never the words:"
            }
            Voice::Chapter | Voice::Summary => {
                "Samples of your voice, about other heroes. Copy the manner, never the words:"
            }
            Voice::NpcReply => {
                "Samples of how people of this world speak, about other matters. Copy the \
                 manner, never the words:"
            }
        }
    }
}

/// Every narrator sample, in the order of the file. A sample starts at its `moment:` line,
/// and one with no `line:` is left out.
#[must_use]
pub fn line_samples() -> Vec<LineSample> {
    let mut samples: Vec<LineSample> = Vec::new();
    for (key, value) in data_lines(NARRATOR_LINES).filter_map(|line| line.split_once(": ")) {
        if key == "moment" {
            samples.push(LineSample {
                moment: value,
                ..LineSample::default()
            });
        }
        let Some(sample) = samples.last_mut() else {
            continue;
        };
        match key {
            "who" => sample.who = Some(value),
            "lore" => sample.lore = Some(value),
            "hero" => sample.hero = value,
            "line" => sample.line = value,
            _ => {}
        }
    }
    samples.retain(|sample| !sample.line.is_empty());
    samples
}

/// The samples of one prompt. `turn` picks the first one, so two prompts in a row differ.
#[must_use]
pub fn rotated(voice: Voice, turn: usize) -> Vec<&'static str> {
    in_turn(&voice.samples(), voice.per_prompt(), turn)
}

fn in_turn<T: Copy>(all: &[T], count: usize, turn: usize) -> Vec<T> {
    let count = count.min(all.len());
    let start = turn % all.len().max(1);
    (0..count)
        .map(|step| all[(start + step) % all.len()])
        .collect()
}

/// The samples of one prompt under their heading, fenced as data.
#[must_use]
pub fn section(voice: Voice, turn: usize) -> String {
    let samples = match voice {
        Voice::NarratorLine => line_pairs(turn, ""),
        Voice::Chapter | Voice::NpcReply | Voice::Summary => bulleted(&rotated(voice, turn)),
    };
    format!("{}\n{}", voice.heading(), fenced(&samples))
}

/// The narrator samples of one prompt under their heading. A sample of the same moment is
/// left out: a model copies the line of a moment that it sees twice.
#[must_use]
pub fn line_section(turn: usize, moment: &str) -> String {
    let voice = Voice::NarratorLine;
    format!("{}\n{}", voice.heading(), fenced(&line_pairs(turn, moment)))
}

fn line_pairs(turn: usize, moment: &str) -> String {
    let others: Vec<LineSample> = line_samples()
        .into_iter()
        .filter(|sample| sample.moment != moment)
        .collect();
    let pairs = in_turn(&others, Voice::NarratorLine.per_prompt(), turn);
    pairs
        .iter()
        .map(shown_pair)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A sample with the same headings as the moment of the prompt.
fn shown_pair(sample: &LineSample) -> String {
    let mut lines = vec![format!("Moment: {}", sample.moment)];
    lines.extend(sample.who.map(|who| format!("The hero: {who}")));
    lines.push(format!("Lore: {}", sample.lore.unwrap_or("none")));
    lines.push(format!("Name the hero: {}", sample.hero));
    lines.push(format!("Line: {}", sample.line));
    lines.join("\n")
}

/// Every sample that a narrator prompt of `moment` can show: the narrator samples of the
/// same moment stay out of it (`line_section`), so a line cannot copy them.
#[must_use]
pub fn every_sample_shown_with(moment: &str) -> Vec<&'static str> {
    let left_out: Vec<&str> = line_samples()
        .into_iter()
        .filter(|sample| sample.moment == moment)
        .map(|sample| sample.line)
        .collect();
    every_sample()
        .into_iter()
        .filter(|sample| !left_out.contains(sample))
        .collect()
}

/// Every sample of every voice, for the check against a copy.
#[must_use]
pub fn every_sample() -> Vec<&'static str> {
    let voices = [
        Voice::NarratorLine,
        Voice::Chapter,
        Voice::NpcReply,
        Voice::Summary,
    ];
    voices.into_iter().flat_map(Voice::samples).collect()
}
