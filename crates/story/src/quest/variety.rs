//! No repeated quests: the shape of a quest, the main words of its title, and the rules
//! against the newest offers (docs/plans/quest-variety.md 3).

use super::{AnyOrder, Genre, Quest, QuestFault, Step, Tracked};
use crate::check::{data_lines, words_of};
use std::fmt;

const STOP_WORDS: &str = include_str!("../../data/stop_words.txt");

/// The newest offers that the prompt shows.
pub const RECENT_IN_PROMPT: usize = 3;
/// A new quest never has the shape of one of this many newest offers.
pub const SHAPES_TO_AVOID: usize = 2;
/// A new title shares no main word with this many newest titles.
pub const TITLES_TO_AVOID: usize = 3;

/// The kinds of the steps of a quest, in order. Two quests with the same shape feel the
/// same. The shape holds no target and no count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape(Vec<ShapePart>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShapePart {
    One(&'static str),
    /// The kinds of an any-order set, sorted.
    AnyOrder(Vec<&'static str>),
}

impl Shape {
    #[must_use]
    pub fn of(steps: &[Step], any_order: Option<AnyOrder>) -> Shape {
        let mut parts = Vec::new();
        let mut step = 0;
        while step < steps.len() {
            if let Some(span) = any_order.filter(|span| span.first == step) {
                let end = span.last.min(steps.len() - 1);
                parts.push(ShapePart::AnyOrder(sorted_goals(&steps[step..=end])));
                step = end + 1;
                continue;
            }
            parts.push(ShapePart::One(steps[step].goal()));
            step += 1;
        }
        Shape(parts)
    }
}

/// The order inside a set never changes its shape.
fn sorted_goals(steps: &[Step]) -> Vec<&'static str> {
    let mut goals: Vec<&'static str> = steps.iter().map(Step::goal).collect();
    goals.sort_unstable();
    goals
}

/// Plain words for the prompt: "visit, wait, talk" or "any order (kill, meet), carry".
impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .0
            .iter()
            .map(|part| match part {
                ShapePart::One(goal) => (*goal).to_string(),
                ShapePart::AnyOrder(goals) => format!("any order ({})", goals.join(", ")),
            })
            .collect();
        f.write_str(&parts.join(", "))
    }
}

/// A quest of the log, as the variety rules see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recent {
    pub number: u64,
    pub title: String,
    pub shape: Shape,
    /// None for a quest from before genres.
    pub genre: Option<Genre>,
}

/// The newest offers of the log, newest first, in any state: the player saw each of them.
#[must_use]
pub fn recent_quests(quests: &[Tracked], count: usize) -> Vec<Recent> {
    let mut newest: Vec<&Tracked> = quests.iter().collect();
    newest.sort_by_key(|quest| std::cmp::Reverse(quest.number));
    newest
        .into_iter()
        .take(count)
        .map(|quest| Recent {
            number: quest.number,
            title: quest.title.clone(),
            shape: Shape::of(&quest.steps, quest.any_order),
            genre: quest.genre,
        })
        .collect()
}

/// The words of a title that make it what it is: every word in lower case, except a stop
/// word and a word of one letter. A name in the title, such as the name of the giver, is a
/// main word too.
#[must_use]
pub fn main_words(title: &str) -> Vec<String> {
    let stop: Vec<&str> = data_lines(STOP_WORDS).collect();
    words_of(title)
        .into_iter()
        .filter(|word| word.chars().count() > 1 && !stop.contains(&word.as_str()))
        .collect()
}

/// The genre in a few words for the prompt: "An errand."
fn genre_words(genre: Genre) -> &'static str {
    match genre {
        Genre::Errand => "An errand.",
        Genre::Hunt => "A hunt.",
        Genre::Mystery => "A mystery.",
        Genre::Rescue => "A rescue.",
        Genre::Rivalry => "A rivalry.",
        Genre::Comic => "A comic quest.",
    }
}

/// One line of the recent block: the title, the shape, and the genre when there is one.
#[must_use]
pub fn recent_line(recent: &Recent) -> String {
    let genre = recent.genre.map_or("", genre_words);
    format!("\"{}\": {}. {genre}", recent.title, recent.shape)
        .trim_end()
        .to_string()
}

/// The first variety rule that the quest breaks.
#[must_use]
pub fn variety_fault(quest: &Quest, recent: &[Recent]) -> Option<QuestFault> {
    let shape = Shape::of(&quest.steps, quest.any_order);
    let shapes = recent.iter().take(SHAPES_TO_AVOID);
    if shapes.into_iter().any(|old| old.shape == shape) {
        return Some(QuestFault::SameShape(shape.to_string()));
    }
    let old_words: Vec<String> = recent
        .iter()
        .take(TITLES_TO_AVOID)
        .flat_map(|old| main_words(&old.title))
        .collect();
    let shared = main_words(&quest.title)
        .into_iter()
        .find(|word| old_words.contains(word));
    shared.map(QuestFault::SameTitleWord)
}
