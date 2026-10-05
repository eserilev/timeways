//! The narrator: one short line at a big moment, within a budget (GAMEPLAY.md 3.2). The
//! words of its prompt live here, and the checks of a line live in `line_check`.

use crate::character::Character;
use crate::house::{HOUSE_RULES, NAME_MARK, fenced, first_chars};
use crate::moments::Moment;
use crate::places::InstanceKind;
use crate::race_class::{Class, Race};
use crate::samples::{self, Voice};
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use timeways_rules::budget::{self as rules, LINES_PER_HOUR};

/// About 50 words. The prompt asks for 30.
pub const MAX_LINE_CHARS: usize = 300;

/// The limit of the bridge for a narrator line (Gnomish Relay SPEC.md 9.8).
pub const MAX_LINE_BYTES: usize = 1000;

/// The lore of a moment keeps at most this many characters: about 100 tokens.
pub const MAX_LORE_CHARS: usize = 400;

/// The persona of the narrator: its manner only, never lore (GAMEPLAY.md 3.2.1). The
/// chronicle shares it, and an NPC never gets it.
pub const PERSONA: &str = include_str!("../data/narrator.txt");

const TASK: &str = "Tell the moment below in one or two sentences, at most 30 words. First \
give a piece of the history of its place, its foe, or its people, from the lore. Then tell \
what the hero did. $N stands for the name of the hero: the game puts the name there.";

/// The author's note. A model weighs the end of a prompt most.
const NOTE: &str = "\
Remember: the history first, then the deed. Take both from the moment and the lore, and \
add nothing. Name the hero only as the line above says, and at most once. When the moment \
and the lore give you nothing true to tell, answer SILENCE.
Answer with the line only.";

/// Counts the lines of the last hour of game time, so the narrator talks little. The rule
/// lives in timeways-rules, where Lean proves it (lean/README.md).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Budget {
    /// The times of the last lines, oldest first.
    spoken: [Option<Tick>; LINES_PER_HOUR],
}

impl Budget {
    /// True when the narrator has a line left at `at`. That line then counts.
    pub fn take(&mut self, at: Tick) -> bool {
        let mut rules = rules::Budget {
            spoken: self.spoken.map(|line| line.map(|line| line.0)),
        };
        let taken = rules.take(at.0);
        self.spoken = rules.spoken.map(|line| line.map(Tick));
        taken
    }
}

/// What the narrator knows of the hero. Never the name: a line writes `$N` for it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Who {
    pub race: Option<Race>,
    pub class: Option<Class>,
    /// The earned titles, oldest first.
    pub titles: Vec<String>,
}

impl Who {
    #[must_use]
    pub fn of(character: &Character) -> Who {
        Who {
            race: character.race(),
            class: character.class(),
            titles: character.titles().into_iter().map(str::to_string).collect(),
        }
    }

    /// "a Forsaken warlock", "an orc", or "a mage". None before the addon tells them.
    #[must_use]
    pub fn described(&self) -> Option<String> {
        let words: Vec<&str> = [self.race.map(Race::word), self.class.map(Class::word)]
            .into_iter()
            .flatten()
            .collect();
        let first = words.first()?;
        let article = if first.starts_with(['a', 'e', 'i', 'o', 'u']) {
            "an"
        } else {
            "a"
        };
        Some(format!("{article} {}", words.join(" ")))
    }
}

/// How one line names the hero. The code picks it in turn, so the lines mix the name, the
/// race or the class, no name, and now and then a title (GAMEPLAY.md 3.2.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Naming {
    /// `$N`, which the addon swaps for the name.
    Name,
    /// A race or a class: "the Forsaken", "the paladin".
    Kind(&'static str),
    Title(String),
    Unnamed,
}

impl Naming {
    /// The words after "Name the hero:", as the samples write them.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Naming::Name => NAME_MARK.to_string(),
            Naming::Kind(word) => format!("the {word}"),
            Naming::Title(title) => format!("by the title \"{title}\""),
            Naming::Unnamed => "no name".to_string(),
        }
    }
}

#[derive(Clone, Copy)]
enum Turn {
    Name,
    Class,
    Race,
    Title,
    Unnamed,
}

/// Three names, two kinds, two lines with no name, and one title in each 8 lines.
const ROTATION: [Turn; 8] = [
    Turn::Name,
    Turn::Class,
    Turn::Unnamed,
    Turn::Name,
    Turn::Race,
    Turn::Name,
    Turn::Unnamed,
    Turn::Title,
];

/// A kind or a title that the hero lacks gives the name.
#[must_use]
pub fn naming(who: &Who, turn: usize) -> Naming {
    let race = who.race.map(|race| Naming::Kind(race.word()));
    let class = who.class.map(|class| Naming::Kind(class.word()));
    let named = match ROTATION[turn % ROTATION.len()] {
        Turn::Name => None,
        Turn::Unnamed => Some(Naming::Unnamed),
        Turn::Class => class.or(race),
        Turn::Race => race.or(class),
        Turn::Title => who.titles.last().cloned().map(Naming::Title),
    };
    named.unwrap_or(Naming::Name)
}

/// One moment to tell, with its lore and what the narrator knows of the hero.
#[derive(Clone, Copy, Debug)]
pub struct Telling<'a> {
    pub moment: &'a Moment,
    /// A short passage (`lore_excerpt`) about the place, the foe, or the people.
    pub lore: Option<&'a str>,
    pub who: &'a Who,
}

/// `turn` picks the golden samples and the naming of the prompt. The hero sheet stays out:
/// a line is about one moment, and models pulled the sheet into every line (GAMEPLAY.md
/// 3.2.1).
#[must_use]
pub fn prompt(telling: &Telling<'_>, turn: usize) -> String {
    let mut prompt = format!("{PERSONA}\n{HOUSE_RULES}\n\n{TASK}");
    let samples = samples::section(Voice::NarratorLine, turn);
    let what = fenced(&what_happened(telling.moment));
    let _ = write!(prompt, "\n\n{samples}\n\nThe moment:\n{what}");
    if let Some(who) = telling.who.described() {
        let _ = write!(prompt, "\n\nThe hero: {who}");
    }
    match telling.lore {
        Some(lore) => {
            let _ = write!(prompt, "\n\nThe lore:\n{}", fenced(lore));
        }
        None => prompt.push_str("\n\nThe lore: none"),
    }
    let naming = naming(telling.who, turn).label();
    let _ = write!(prompt, "\n\nName the hero: {naming}\n\n{NOTE}");
    prompt
}

/// The start of a lore passage, cut after a sentence when it is long.
#[must_use]
pub fn lore_excerpt(text: &str) -> String {
    let head = first_chars(text.trim(), MAX_LORE_CHARS);
    if head.len() == text.trim().len() {
        return head.to_string();
    }
    let sentence_end = head
        .rmatch_indices(". ")
        .map(|(at, _)| at + 1)
        .next()
        .or_else(|| head.rfind(' '));
    sentence_end.map_or(head, |end| &head[..end]).to_string()
}

/// The moment in plain words, as the prompt gives it. It says "first" with no stock
/// phrase, because a line takes the words of its moment.
#[must_use]
pub fn what_happened(moment: &Moment) -> String {
    match moment {
        Moment::Flavor { what } => what.clone(),
        Moment::Titled { title } => {
            format!("The player earned the title \"{title}\" in their journal.")
        }
        Moment::FirstKill { foe } => format!("The player defeated {foe}. They never had before."),
        Moment::SlainAgain { killer, times } => {
            format!("{killer} killed the player again. That makes {times} times.")
        }
        Moment::Slapped { npc, times } => {
            format!("The player slapped {npc}. That makes {times} times.")
        }
        Moment::LevelUp { level, zone: None } => format!("The player reached level {level}."),
        Moment::LevelUp {
            level,
            zone: Some(zone),
        } => format!("The player reached level {level} in {zone}."),
        Moment::NewZone { zone } => {
            format!("The player arrived in {zone}. They had never been there before.")
        }
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Dungeon,
        } => format!("The player entered the dungeon {zone}. They had never been inside before."),
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Raid,
        } => format!("The player entered the raid {zone}. They had never been inside before."),
        Moment::QuestMarked { mark, quest } => {
            format!(
                "During the quest \"{quest}\", a lasting effect came on the player: \"{mark}\"."
            )
        }
        Moment::FirstCapital { city } => format!(
            "The player arrived in the capital city {city}. They had never been there before."
        ),
        Moment::ClassQuestDone { title } => {
            format!("The player finished \"{title}\", a quest of their class.")
        }
    }
}
