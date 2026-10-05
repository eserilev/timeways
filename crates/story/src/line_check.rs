//! The checks of a narrator line before it shows (GAMEPLAY.md 3.2.1). A line that fails
//! gets one retry with the reasons, and silence after a second failure.

use crate::arrival::arrival_in;
use crate::check::{
    banned_words_in, has_emoji, mentions, names_after_cutoff_except, one_line, slop_in, words_of,
};
use crate::house::NAME_MARK;
use crate::narrator::{MAX_LINE_BYTES, MAX_LINE_CHARS, Naming, Telling, naming, what_happened};
use crate::samples;
use std::fmt;

/// A narrator line is short, so 4 words in a row of a sample already make a copy.
pub const COPIED_LINE_WORDS: usize = 4;

/// The answer of a model that has nothing true to tell.
pub const SILENCE: &str = "SILENCE";

/// A name of the facts is mostly longer than this. Shorter words, such as "the" or "of",
/// ground nothing.
const SHORTEST_ANCHOR: usize = 4;

/// Words that start a sentence with a capital letter, and name nothing.
const NOT_NAMES: [&str; 32] = [
    "after", "also", "before", "both", "during", "each", "even", "every", "from", "here", "into",
    "many", "most", "much", "once", "only", "over", "since", "some", "still", "that", "their",
    "then", "there", "these", "they", "this", "those", "under", "until", "when", "with",
];

/// What a line was told from: the moment in words, its names, its lore, and how it names
/// the hero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grounds {
    pub moment: String,
    pub names: Vec<String>,
    pub lore: Option<String>,
    pub naming: Naming,
    /// The race, the class, and the title of the hero: words for the hero past `$N`.
    pub hero_words: Vec<String>,
}

impl Grounds {
    /// `turn` is the turn of the prompt, so the naming is the one that the prompt asked for.
    #[must_use]
    pub fn of(telling: &Telling<'_>, turn: usize) -> Grounds {
        let moment = telling.moment;
        let naming = naming(moment, telling.who, turn);
        Grounds {
            moment: what_happened(moment),
            names: moment.names().into_iter().map(str::to_string).collect(),
            lore: telling.lore.map(str::to_string),
            hero_words: naming.hero_words(telling.who),
            naming,
        }
    }

    /// Everything that the prompt told about the moment.
    fn text(&self) -> String {
        let mut parts = vec![self.moment.as_str()];
        parts.extend(self.names.iter().map(String::as_str));
        parts.extend(self.lore.as_deref());
        parts.join("\n")
    }

    /// The words that ground a line: each long word of a name of the moment, each name of
    /// the moment text or the lore, and each number of the moment.
    fn anchors(&self) -> Vec<String> {
        let mut anchors: Vec<String> = self.names.iter().flat_map(|name| words_of(name)).collect();
        anchors.extend(capital_words(&self.moment));
        anchors.extend(self.lore.as_deref().map(capital_words).unwrap_or_default());
        anchors.retain(|word| word.chars().count() >= SHORTEST_ANCHOR);
        anchors.extend(numbers(&self.moment).into_iter().map(str::to_string));
        anchors
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineFault {
    /// Empty, too long, or not one line.
    Unreadable,
    LaterName(String),
    Emoji,
    Banned(String),
    Copy(String),
    NewNumber(String),
    /// The line names nothing of the moment or its lore.
    Ungrounded,
    Bracket,
    /// The hero is named more than once.
    NamedTwice,
    /// A clause only tells that the hero came, such as "$N came to Westfall".
    Arrival(String),
    /// The moment is about a place, and the line names the hero.
    HeroAtAPlace,
}

impl fmt::Display for LineFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineFault::Unreadable => write!(
                f,
                "The line is empty, or longer than {MAX_LINE_CHARS} characters."
            ),
            LineFault::LaterName(name) => {
                write!(f, "\"{name}\" is from after the year 25 ADP. Leave it out.")
            }
            LineFault::Emoji => write!(f, "The line has an emoji. Use words only."),
            LineFault::Banned(word) => {
                write!(f, "\"{word}\" is empty or invented. Leave it out.")
            }
            LineFault::Copy(phrase) => {
                write!(f, "\"{phrase}\" copies a sample. Use your own words.")
            }
            LineFault::NewNumber(number) => write!(
                f,
                "The number {number} is not in the moment. Tell only its numbers."
            ),
            LineFault::Ungrounded => write!(
                f,
                "The line names nothing of the moment or its lore. Name its place, foe, person, \
                 or people."
            ),
            LineFault::Bracket => write!(
                f,
                "The line has a bracket. Write {NAME_MARK} for the name of the hero, and no other \
                 mark."
            ),
            LineFault::NamedTwice => {
                write!(
                    f,
                    "The line names the hero twice. Name the hero at most once."
                )
            }
            LineFault::Arrival(clause) => write!(
                f,
                "\"{clause}\" only tells that the hero came. Leave it out, and tell more of the \
                 place or the deed."
            ),
            LineFault::HeroAtAPlace => write!(
                f,
                "This moment is about the place. Leave the hero out, and tell the place alone."
            ),
        }
    }
}

/// The verdict on an answer of the narrator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checked {
    /// The line as the player sees it.
    Line(String),
    /// The model had nothing true to tell, as the prompt allows.
    Silent,
    /// Every fault at once, so one retry fixes all of them.
    Refused(Vec<LineFault>),
}

/// `player_text` is the hero in the player's own words: a later name there is allowed
/// (GAMEPLAY.md 3.7).
#[must_use]
pub fn checked_line(text: &str, grounds: &Grounds, player_text: &str) -> Checked {
    let Some(line) = one_line(text, MAX_LINE_CHARS, MAX_LINE_BYTES) else {
        return Checked::Refused(vec![LineFault::Unreadable]);
    };
    if words_of(&line) == [SILENCE.to_lowercase()] {
        return Checked::Silent;
    }
    let faults = faults(&line, grounds, player_text);
    if faults.is_empty() {
        Checked::Line(line)
    } else {
        Checked::Refused(faults)
    }
}

fn faults(line: &str, grounds: &Grounds, player_text: &str) -> Vec<LineFault> {
    let told = grounds.text();
    let mut faults: Vec<LineFault> = names_after_cutoff_except(line, player_text)
        .into_iter()
        .map(|name| LineFault::LaterName(name.to_string()))
        .collect();
    if has_emoji(line) {
        faults.push(LineFault::Emoji);
    }
    let banned = banned_words_in(line)
        .into_iter()
        .filter(|word| !mentions(&told, word));
    let banned = banned.chain(slop_in(line, &told));
    faults.extend(banned.map(|word| LineFault::Banned(word.to_string())));
    faults.extend(copied_phrase(line, &told).map(LineFault::Copy));
    let known = numbers(&told);
    let new_numbers = numbers(line)
        .into_iter()
        .filter(|number| !known.contains(number));
    faults.extend(new_numbers.map(|number| LineFault::NewNumber(number.to_string())));
    if !grounded(line, grounds) {
        faults.push(LineFault::Ungrounded);
    }
    if line.contains(['[', ']', '{', '}', '<', '>']) {
        faults.push(LineFault::Bracket);
    }
    if line.matches(NAME_MARK).count() > 1 {
        faults.push(LineFault::NamedTwice);
    }
    if grounds.naming == Naming::Absent && line.contains(NAME_MARK) {
        faults.push(LineFault::HeroAtAPlace);
    }
    faults.extend(arrival_in(line, &grounds.hero_words).map(LineFault::Arrival));
    faults
}

/// True when a word of the line starts with an anchor of the moment, so "murlocs" counts
/// for "Murloc Coastrunner", and "Elwynn's" for "Elwynn Forest".
#[must_use]
pub fn grounded(line: &str, grounds: &Grounds) -> bool {
    let anchors = grounds.anchors();
    words_of(line).iter().any(|word| {
        anchors
            .iter()
            .any(|anchor| word.starts_with(anchor.as_str()))
    })
}

/// The first run of `COPIED_LINE_WORDS` words that the line shares with a sample, and
/// that the moment and its lore do not hold themselves.
fn copied_phrase(line: &str, told: &str) -> Option<String> {
    let words = words_of(line);
    let told = words_of(told);
    let shared = |phrase: &[String]| {
        samples::every_sample()
            .iter()
            .any(|sample| holds_run(&words_of(sample), phrase))
    };
    words
        .windows(COPIED_LINE_WORDS)
        .find(|phrase| shared(phrase) && !holds_run(&told, phrase))
        .map(|phrase| phrase.join(" "))
}

fn holds_run(words: &[String], phrase: &[String]) -> bool {
    words.windows(phrase.len()).any(|window| window == phrase)
}

/// The words that start with a capital letter and are no common start of a sentence, in
/// lower case.
fn capital_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().next().is_some_and(char::is_uppercase))
        .map(str::to_lowercase)
        .filter(|word| !NOT_NAMES.contains(&word.as_str()))
        .collect()
}

fn numbers(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|number| !number.is_empty())
        .collect()
}
