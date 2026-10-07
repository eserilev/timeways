//! The checks of a narrator line before it shows (GAMEPLAY.md 3.2.1). A line that fails
//! gets one retry with the reasons, and silence after a second failure.

use crate::arrival::arrival_in;
use crate::check::{
    banned_words_in, content_words, has_emoji, mentions, names_after_cutoff_except, one_line,
    slop_in, words_of,
};
use crate::house::NAME_MARK;
use crate::narrator::{MAX_LINE_BYTES, MAX_LINE_CHARS, Naming, Telling, naming, what_happened};
use crate::prose::{ProseFault, prose_faults};
use crate::samples;
use crate::sentences::sentences;
use std::fmt;

/// A narrator line is short, so 4 words in a row of a sample already make a copy.
pub const COPIED_LINE_WORDS: usize = 4;

/// A narrator line holds at most this many sentences (docs/plans/narrator-style.md 3).
pub const MOST_SENTENCES: usize = 3;

/// A line that shares this many words that tell something, in a row, with the player's own
/// story calls back to it.
pub const CALLBACK_WORDS: usize = 3;

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
    /// The names of a mount or an item, from the game (`Moment::outside_names`).
    pub outside: Vec<String>,
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
            outside: moment
                .outside_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }

    /// Everything that the prompt told about the moment, as the words that a line can
    /// take. A name from outside is left out, unless the lore holds it too: an item called
    /// "Destiny" allows no "destiny" in the line.
    fn text(&self) -> String {
        let lore = self.lore.as_deref().unwrap_or_default();
        let untrusted: Vec<&str> = self
            .outside
            .iter()
            .map(String::as_str)
            .filter(|name| !mentions(lore, name))
            .collect();
        let mut moment = self.moment.clone();
        for name in &untrusted {
            moment = moment.replace(name, " ");
        }
        let mut parts = vec![moment.as_str()];
        let names = self.names.iter().map(String::as_str);
        parts.extend(names.filter(|name| !untrusted.contains(name)));
        parts.extend(self.lore.as_deref());
        parts.join("\n")
    }

    /// The line with each name from outside cut out. The code put the name in its slot,
    /// and the history passed its own checks, so a word of the name is no slop there. The
    /// check of later names still reads the whole line.
    fn without_outside(&self, line: &str) -> String {
        let mut worded = line.to_string();
        for name in &self.outside {
            worded = worded.replace(name.as_str(), " ");
        }
        worded
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
    /// A fault of the style guide that every narrator text can have.
    Prose(ProseFault),
    TooManySentences(usize),
    /// More than one number: a count of levels, kills, or quests.
    Ledger,
    /// A run of words of the player's own story.
    Callback(String),
    /// The answer is no single JSON object with the fields of the prompt.
    BadAnswer,
    /// A choice holds a value that the prompt did not offer.
    UnknownChoice(String),
    /// A choice says that the history names something, and it does not.
    ChoiceNotInLore(String),
    /// The sentence of history names the hero. The code adds the deed.
    HeroInHistory,
    /// The sentence of history is longer than its budget of characters.
    OverBudget(usize),
    /// The history has more sentences than the moment allows: the count, and the most.
    HistorySentences(usize, usize),
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
            LineFault::Prose(fault) => fault.fmt(f),
            LineFault::TooManySentences(count) => write!(
                f,
                "The line has {count} sentences. Use 1 to {MOST_SENTENCES}."
            ),
            LineFault::Ledger => write!(
                f,
                "The line counts more than one number. Keep one number at most, the number of \
                 the moment."
            ),
            LineFault::Callback(words) => write!(
                f,
                "\"{words}\" repeats the player's own story. Leave it out, and tell the moment."
            ),
            LineFault::BadAnswer => write!(
                f,
                "The answer is not one JSON object with exactly the fields of the prompt. \
                 Answer with the JSON only."
            ),
            LineFault::UnknownChoice(field) => write!(
                f,
                "\"{field}\" holds a value that the prompt did not offer. Choose one of its \
                 list."
            ),
            LineFault::ChoiceNotInLore(field) => write!(
                f,
                "\"{field}\" says that the history names it, and the history does not. Name it \
                 in the history, or change the choice."
            ),
            LineFault::HeroInHistory => write!(
                f,
                "The history names the hero. Leave the hero out: the game adds the deed."
            ),
            LineFault::HistorySentences(count, 1) => write!(
                f,
                "The history has {count} sentences. Write one sentence of history."
            ),
            LineFault::HistorySentences(count, most) => write!(
                f,
                "The history has {count} sentences. Write at most {most}."
            ),
            LineFault::OverBudget(chars) => write!(
                f,
                "The history is longer than {chars} characters. Make it shorter."
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
    faults.extend(copied_phrase(line, &told, &grounds.moment).map(LineFault::Copy));
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
    let prose = prose_faults(line, &grounds.hero_words);
    faults.extend(prose.into_iter().map(LineFault::Prose));
    let sentence_count = sentences(line).len();
    if sentence_count > MOST_SENTENCES {
        faults.push(LineFault::TooManySentences(sentence_count));
    }
    if number_count(line) > 1 {
        faults.push(LineFault::Ledger);
    }
    faults.extend(callback_in(line, player_text, &told).map(LineFault::Callback));
    faults
}

/// The faults of the sentence of history of a templated answer
/// (docs/plans/narrator-templates.md 8.3): every check of a line, on the lore alone, with
/// no hero, at most `most_sentences` sentences, and at most `budget` characters.
#[must_use]
pub fn lore_faults(
    lore: &str,
    grounds: &Grounds,
    player_text: &str,
    most_sentences: usize,
    budget: usize,
) -> Vec<LineFault> {
    if lore.chars().count() > budget {
        return vec![LineFault::OverBudget(budget)];
    }
    let Some(lore) = one_line(lore, MAX_LINE_CHARS, MAX_LINE_BYTES) else {
        return vec![LineFault::Unreadable];
    };
    let mut found: Vec<LineFault> = faults(&lore, grounds, player_text)
        .into_iter()
        .filter(|fault| {
            !matches!(
                fault,
                LineFault::NamedTwice | LineFault::HeroAtAPlace | LineFault::TooManySentences(_)
            )
        })
        .collect();
    if lore.contains(NAME_MARK) {
        found.push(LineFault::HeroInHistory);
    }
    let count = sentences(&lore).len();
    if count > most_sentences {
        found.push(LineFault::HistorySentences(count, most_sentences));
    }
    found
}

/// The faults of a line that the code built from templates: the second guard. The lore
/// passed its own checks, so the copy, the numbers, the grounds, and the callback read
/// it alone. A sentence of a template can be short ("Hogger is dead."), so no fragment
/// counts, and the number of the template counts once in the ledger.
#[must_use]
pub fn built_faults(line: &str, lore: &str, grounds: &Grounds) -> Vec<LineFault> {
    let Some(line) = one_line(line, MAX_LINE_CHARS, MAX_LINE_BYTES) else {
        return vec![LineFault::Unreadable];
    };
    let told = format!("{}\n{lore}", grounds.text());
    let mut found: Vec<LineFault> = names_after_cutoff_except(&line, lore)
        .into_iter()
        .map(|name| LineFault::LaterName(name.to_string()))
        .collect();
    if has_emoji(&line) {
        found.push(LineFault::Emoji);
    }
    let worded = grounds.without_outside(&line);
    let banned = banned_words_in(&worded)
        .into_iter()
        .filter(|word| !mentions(&told, word));
    let banned = banned.chain(slop_in(&worded, &told));
    found.extend(banned.map(|word| LineFault::Banned(word.to_string())));
    if line.contains(['[', ']', '{', '}', '<', '>']) {
        found.push(LineFault::Bracket);
    }
    if line.matches(NAME_MARK).count() > 1 {
        found.push(LineFault::NamedTwice);
    }
    if grounds.naming == Naming::Absent && line.contains(NAME_MARK) {
        found.push(LineFault::HeroAtAPlace);
    }
    found.extend(arrival_in(&line, &grounds.hero_words).map(LineFault::Arrival));
    let prose = prose_faults(&line, &grounds.hero_words)
        .into_iter()
        .filter(|fault| !matches!(fault, ProseFault::Fragment(_)));
    found.extend(prose.map(LineFault::Prose));
    let count = sentences(&line).len();
    if count > MOST_SENTENCES {
        found.push(LineFault::TooManySentences(count));
    }
    let template_numbers = number_count(&line).saturating_sub(number_count(lore));
    if number_count(lore) + template_numbers.min(1) > 1 {
        found.push(LineFault::Ledger);
    }
    found
}

/// The words between spaces that are a number: "20," "1,000", and "11th" count once each.
/// A name with a digit, such as "SI:7", is no number.
fn number_count(line: &str) -> usize {
    line.split_whitespace()
        .filter(|word| is_number(word))
        .count()
}

fn is_number(word: &str) -> bool {
    let bare = word.trim_end_matches(|c: char| !c.is_alphanumeric());
    let bare = ["st", "nd", "rd", "th"]
        .iter()
        .find_map(|suffix| bare.strip_suffix(suffix))
        .unwrap_or(bare);
    bare.chars().any(|c| c.is_ascii_digit()) && !bare.chars().any(char::is_alphabetic)
}

/// The first run of `CALLBACK_WORDS` words that tell something, which the line shares with
/// one line of the player's own story, and the moment and its lore do not hold. Small
/// words between them do not count, so "a farm in Elwynn" is the run "farm elwynn".
#[must_use]
pub fn callback_in(line: &str, player_text: &str, told: &str) -> Option<String> {
    let words = content_words(line);
    let told = content_words(told);
    let shared = |run: &[String]| {
        let in_story = player_text
            .lines()
            .any(|story| holds_run(&content_words(story), run));
        in_story && !holds_run(&told, run)
    };
    let run = words.windows(CALLBACK_WORDS).find(|run| shared(run))?;
    Some(run.join(" "))
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
/// that the moment and its lore do not hold themselves. A run whose words that tell
/// something the lore holds in a row is no copy: "and the Defias Brotherhood" only names
/// what the lore names. A sample of the same moment is no source: the prompt left it out.
fn copied_phrase(line: &str, told: &str, moment: &str) -> Option<String> {
    let words = words_of(line);
    let told_words = words_of(told);
    let told_content = content_words(told);
    let shared = |phrase: &[String]| {
        samples::every_sample_shown_with(moment)
            .iter()
            .any(|sample| holds_run(&words_of(sample), phrase))
    };
    let names_the_lore = |phrase: &[String]| {
        let content = content_words(&phrase.join(" "));
        content.is_empty() || holds_run(&told_content, &content)
    };
    words
        .windows(COPIED_LINE_WORDS)
        .find(|phrase| shared(phrase) && !holds_run(&told_words, phrase) && !names_the_lore(phrase))
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
