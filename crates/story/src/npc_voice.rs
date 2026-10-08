//! The checks of the voice guide of an NPC (docs/plans/npc-voice.md 6). An NPC talks to the
//! player as a person of Azeroth, so these checks differ from the checks of the narrator.

use crate::check::{data_lines, mentions, words_of};
use crate::sentences::{sentences, word_count};
use std::fmt;

const NPC_SLOP: &str = include_str!("../data/npc_slop.txt");

/// The prompt asks for 40 words. A name of several words gets a margin.
pub const MOST_WORDS: usize = 60;

/// An NPC speaks in short turns, as the gossip of the game does.
pub const MOST_SENTENCES: usize = 4;

/// A spoken sentence is shorter than a sentence of the chronicle.
pub const MOST_SENTENCE_WORDS: usize = 25;

/// A greeting gets a short question back, such as "What do you want?"
pub const MOST_QUESTION_WORDS: usize = 6;

/// More marks of dialect than this in one answer make a caricature.
pub const MOST_DIALECT_WORDS: usize = 2;

/// A ban phrase counts within the part of a sentence between these marks.
const SENTENCE_MARKS: [char; 5] = ['.', '!', '?', ';', ':'];

/// The sounds that open a stock reply: "Ah, adventurer!"
const STOCK_OPENERS: [&str; 4] = ["ah", "ahh", "ahhh", "aah"];

/// The words of a dialect, spelled out: "mon", "lad", "ye".
const DIALECT_WORDS: [&str; 17] = [
    "mon", "mahn", "lad", "laddie", "lass", "lassie", "aye", "ye", "yer", "ach", "och", "nae",
    "cannae", "dinnae", "arr", "matey", "ja",
];

/// The words before a feeling that tell how the player looks or feels: "you look tired".
const SEEMING: [&str; 11] = [
    "look", "looks", "looking", "seem", "seems", "sound", "feel", "feeling", "must", "re", "are",
];

/// A "you" after one of these words asks or supposes, so it tells no feeling: "if you're
/// lost", "are you lost".
const NOT_TELLING: [&str; 7] = ["if", "when", "unless", "are", "do", "did", "can"];

/// The feelings, and the looks, that an NPC never tells the player that they have.
const FEELINGS: [&str; 30] = [
    "tired",
    "weary",
    "exhausted",
    "worn",
    "hungry",
    "lost",
    "troubled",
    "worried",
    "afraid",
    "frightened",
    "scared",
    "nervous",
    "curious",
    "eager",
    "anxious",
    "uneasy",
    "shaken",
    "sad",
    "angry",
    "burdened",
    "haunted",
    "determined",
    "restless",
    "desperate",
    "confused",
    "unsure",
    "lonely",
    "brave",
    "capable",
    "strong",
];

/// The words between "you" and its feeling: "you must be so tired".
const FEELING_GAP: usize = 3;

/// Whether the words of the player ask the NPC something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    Question,
    NoQuestion,
}

/// The first words of a question that a player types with no question mark.
const QUESTION_OPENERS: [&str; 23] = [
    "who", "what", "where", "when", "why", "how", "which", "whose", "any", "anything", "is", "are",
    "was", "were", "do", "does", "did", "can", "could", "will", "would", "have", "tell",
];

impl Asked {
    #[must_use]
    pub fn of(words: &str) -> Asked {
        let opener = words_of(words)
            .first()
            .is_some_and(|word| QUESTION_OPENERS.contains(&word.as_str()));
        if words.contains('?') || opener {
            Asked::Question
        } else {
            Asked::NoQuestion
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NpcFault {
    Slop(&'static str),
    /// The answer opens with "Ah".
    StockOpener,
    /// The last sentence asks the player something, after the player asked first, or at
    /// length.
    ClosingQuestion(String),
    TooManyWords(usize),
    TooManySentences(usize),
    LongSentence(String),
    /// The NPC tells the player how they look or feel.
    PlayerFeeling(String),
    /// A stage direction between asterisks: "*sighs*".
    StageDirection,
    /// More dialect words than `MOST_DIALECT_WORDS`.
    Caricature(Vec<String>),
}

impl fmt::Display for NpcFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NpcFault::Slop(phrase) => write!(
                f,
                "\"{phrase}\" is a stock phrase of an NPC. Say a plain fact instead."
            ),
            NpcFault::StockOpener => write!(f, "The words open with \"Ah\". Answer at once."),
            NpcFault::ClosingQuestion(sentence) => write!(
                f,
                "The words end on the question \"{sentence}\". End on your answer."
            ),
            NpcFault::TooManyWords(words) => write!(
                f,
                "The words have {words} words. Keep them under {MOST_WORDS}."
            ),
            NpcFault::TooManySentences(count) => write!(
                f,
                "The words have {count} sentences. Say at most {MOST_SENTENCES}."
            ),
            NpcFault::LongSentence(start) => write!(
                f,
                "The spoken sentence \"{start}...\" is too long. Keep it to \
                 {MOST_SENTENCE_WORDS} words."
            ),
            NpcFault::PlayerFeeling(clause) => write!(
                f,
                "\"{clause}\" tells the player how they look or feel. Speak of yourself and \
                 the world."
            ),
            NpcFault::StageDirection => write!(
                f,
                "The words hold a stage direction. Give only what the NPC says."
            ),
            NpcFault::Caricature(words) => write!(
                f,
                "The words hold {} words of dialect: {}. Use at most {MOST_DIALECT_WORDS}.",
                words.len(),
                words.join(", ")
            ),
        }
    }
}

/// Every fault of the words of an NPC, in order. `told` is what the prompt gave, so a
/// phrase of the lore or of the player stays allowed.
#[must_use]
pub fn npc_faults(say: &str, asked: Asked, told: &str) -> Vec<NpcFault> {
    let said = sentences(say);
    let mut faults: Vec<NpcFault> = slop_in(say, told).map(NpcFault::Slop).collect();
    if opens_with_a_stock_sound(say) {
        faults.push(NpcFault::StockOpener);
    }
    faults.extend(closing_question_in(&said, asked).map(NpcFault::ClosingQuestion));
    faults.extend(length_faults(say, &said));
    faults.extend(player_feeling_in(say).map(NpcFault::PlayerFeeling));
    if say.contains('*') {
        faults.push(NpcFault::StageDirection);
    }
    let dialect = dialect_words_in(say);
    if dialect.len() > MOST_DIALECT_WORDS {
        faults.push(NpcFault::Caricature(dialect));
    }
    faults
}

/// The phrases of the ban list of an NPC, as the data file holds them.
pub fn npc_slop_words() -> impl Iterator<Item = &'static str> {
    data_lines(NPC_SLOP)
}

/// A phrase counts within one sentence, across its commas: "Greetings, traveler".
fn slop_in<'a>(say: &'a str, told: &'a str) -> impl Iterator<Item = &'static str> + 'a {
    let parts: Vec<&str> = say.split(SENTENCE_MARKS).collect();
    npc_slop_words().filter(move |phrase| {
        parts.iter().any(|part| mentions(part, phrase)) && !mentions(told, phrase)
    })
}

fn opens_with_a_stock_sound(say: &str) -> bool {
    words_of(say)
        .first()
        .is_some_and(|word| STOCK_OPENERS.contains(&word.as_str()))
}

/// The last sentence, when it is a question that the NPC may not end on.
fn closing_question_in(said: &[&str], asked: Asked) -> Option<String> {
    let last = said.last()?;
    let question = last.trim_end_matches(['"', '\'', ')']).ends_with('?');
    let short = word_count(last) <= MOST_QUESTION_WORDS;
    let allowed = asked == Asked::NoQuestion && short;
    (question && !allowed).then(|| (*last).to_string())
}

fn length_faults(say: &str, said: &[&str]) -> Vec<NpcFault> {
    let mut faults = Vec::new();
    let words = word_count(say);
    if words > MOST_WORDS {
        faults.push(NpcFault::TooManyWords(words));
    }
    if said.len() > MOST_SENTENCES {
        faults.push(NpcFault::TooManySentences(said.len()));
    }
    if let Some(long) = said
        .iter()
        .find(|sentence| word_count(sentence) > MOST_SENTENCE_WORDS)
    {
        let start: Vec<&str> = long.split_whitespace().take(8).collect();
        faults.push(NpcFault::LongSentence(start.join(" ")));
    }
    faults
}

/// The first "you" with a feeling soon after a word of seeming: "you look tired", "you
/// must be weary", "you're lost".
fn player_feeling_in(say: &str) -> Option<String> {
    let words = words_of(say);
    (0..words.len()).find_map(|at| {
        let gap = tells_a_feeling(&words, at)?;
        Some(words[at..=at + gap].join(" "))
    })
}

/// The count of words after the "you" at `at` up to its feeling, when it tells one.
fn tells_a_feeling(words: &[String], at: usize) -> Option<usize> {
    let asks = at > 0 && NOT_TELLING.contains(&words[at - 1].as_str());
    if words[at] != "you" || asks {
        return None;
    }
    let seeming = words.get(at + 1)?;
    if !SEEMING.contains(&seeming.as_str()) {
        return None;
    }
    (2..=FEELING_GAP + 1).find(|gap| {
        words
            .get(at + gap)
            .is_some_and(|word| FEELINGS.contains(&word.as_str()))
    })
}

/// The words of dialect, and each word that drops its last "g": "nothin'".
fn dialect_words_in(say: &str) -> Vec<String> {
    let spelled = words_of(say)
        .into_iter()
        .filter(|word| DIALECT_WORDS.contains(&word.as_str()));
    let dropped = say
        .split_whitespace()
        .map(|word| word.trim_end_matches([',', '.', '!', '?', ';', ':']))
        .filter(|word| word.len() > 3 && (word.ends_with("in'") || word.ends_with("in\u{2019}")))
        .map(str::to_lowercase);
    spelled.chain(dropped).collect()
}
