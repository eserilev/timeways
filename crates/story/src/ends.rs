//! The sentences that tell an end (GAMEPLAY.md 5.10): a deed of adventurers, or the death
//! or the defeat of a foe. The builder tags each passage that tells one, so the one gate of
//! the spoiler limit (`spoiler::may_show`) keeps it from `/lore` and from every prompt
//! until the player did the deed.

use crate::name_match::Words;
use crate::outcome_passages::Npc;

/// The words for the people who did the deed. "adventurers" also covers "a group of
/// adventurers". A possessive, such as "the adventurer's guild", is another word.
const DOERS: [&[&str]; 12] = [
    &["adventurer"],
    &["adventurers"],
    &["heroes", "of", "the", "alliance"],
    &["heroes", "of", "the", "horde"],
    &["champions", "of", "the", "alliance"],
    &["champions", "of", "the", "horde"],
    &["band", "of", "heroes"],
    &["group", "of", "heroes"],
    &["party", "of", "heroes"],
    &["the", "heroes"],
    &["the", "hero"],
    &["the", "champions"],
];

/// More doers of a side: "agents", "soldiers", "forces", and "champions", after "Horde" or
/// "Alliance", or before "of the Horde" or "of the Alliance". They count only as the doers
/// of a kill: right before an active one, or after the "by" of a passive one. So "Alliance
/// soldiers killed VanCleef" is a deed, and "invaluable to Alliance forces, the orcs never
/// succeeded" is not. "The Horde" alone tells the history of the wars, not of a player.
const AGENTS: [&str; 4] = ["agents", "soldiers", "forces", "champions"];
const SIDES: [&str; 2] = ["horde", "alliance"];

/// The words of a result: the deed is done, not only asked for. "sent adventurers to
/// kill" asks, so the plain verb is no result.
const RESULTS: [&[&str]; 32] = [
    &["killed"],
    &["slain"],
    &["slew"],
    &["defeated"],
    &["recovered"],
    &["returned"],
    &["rescued"],
    &["destroyed"],
    &["ended"],
    &["freed"],
    &["retrieved"],
    &["eliminated"],
    &["vanquished"],
    &["liberated"],
    &["succeeded"],
    &["accomplished"],
    &["saved"],
    &["secured"],
    &["captured"],
    &["dispatched"],
    &["disabled"],
    &["executed"],
    &["banished"],
    &["avenged"],
    &["obtained"],
    &["beheaded"],
    &["solved"],
    &["warned"],
    &["put", "down"],
    &["put", "an", "end"],
    &["to", "justice"],
    &["brought", "back"],
];

/// "discovered" and "uncovered" are a result only before "that": "uncovered that he was
/// really Balnazzar". A clue is none: "discovered Blackrock documents".
const REVEALS: [&[&str]; 2] = [&["discovered", "that"], &["uncovered", "that"]];

/// The verbs of the end of a person, with the person after the active verb or before the
/// passive one: "killed VanCleef", "Arugal was defeated".
const KILLS: [&str; 16] = [
    "killed",
    "killing",
    "slaying",
    "defeating",
    "slain",
    "slew",
    "defeated",
    "destroyed",
    "beheaded",
    "executed",
    "murdered",
    "assassinated",
    "vanquished",
    "eliminated",
    "dispatched",
    "banished",
];

/// The verbs of an end with the person right before them: "Mograine died".
const DEATHS: [&str; 4] = ["died", "perished", "succumbed", "fell"];

/// "fell into a coma" and "fell in love" tell no end. "fell" is one only before one of
/// these, or at the end of the sentence: "Lothar fell in battle".
const FELL_ENDS: [&[&str]; 4] = [&["in", "battle"], &["in", "combat"], &["before"], &["to"]];

/// The nouns of an end, with the person after "of" or right before them: "the death of
/// Lothar", "Mograine's death", "the head of Balnazzar".
const END_NOUNS: [&str; 5] = ["death", "demise", "defeat", "execution", "head"];

/// A verb after one of these is passive.
const BE: [&str; 6] = ["was", "were", "been", "being", "is", "are"];

/// Words between a person and the verb of its end: "Arugal was eventually defeated",
/// "he had been killed".
const FILLERS: [&str; 3] = ["had", "has", "have"];

/// Adverbs with no "-ly" that stand inside a verb group: "he was later beheaded".
const ADVERBS: [&str; 4] = ["later", "soon", "then", "also"];

/// An end after one of these is a belief, not an end: "it was thought that Dal'rend had
/// been slain".
const BELIEFS: [&str; 6] = [
    "thought",
    "believed",
    "believing",
    "presumed",
    "rumored",
    "rumoured",
];

/// The subject of a passive end that names no one: "He was eventually killed." "They were
/// defeated" tells the history of a people.
const PRONOUNS: [&str; 2] = ["he", "she"];

/// A name after one of these is no subject of a deed.
const PREPOSITIONS: [&str; 6] = ["of", "by", "from", "with", "to", "for"];

/// A person stands this many words at most after an active kill, or after "of".
const REACH: usize = 6;

/// The name before a noun of an end, or before the verb, ends at most this many words
/// before it: "Mograine's mysterious death".
const NEAR: usize = 2;

/// What one sentence tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Told<'a> {
    Nothing,
    /// The end of this foe: "Alliance forces killed VanCleef".
    EndOf(&'a Npc),
    /// A deed or an end that names no foe of the list: "He was eventually killed."
    Deed,
}

/// What the sentence tells, with the foes that it can name: `Told::EndOf` for the first
/// foe whose end it tells, or for the first foe that is the subject of a deed: "Mor'Ladim
/// killed many until he was put down by adventurers". A name after "of" or "by" is no
/// subject: "a relic of Hakkar that was secured by adventurers". A sentence that holds a
/// setup cue (`asks`) tells no end by a head: "retrieve Rend's head" asks for one. A
/// belief tells only a deed of adventurers: "believed dead after an incursion by
/// adventurers, he may have been merely disabled".
#[must_use]
pub fn told<'a>(sentence: &str, foes: &'a [Npc], asks: bool) -> Told<'a> {
    let words = Words::of(sentence);
    let belief = is_belief(&words);
    let ended = foes.iter().find(|foe| ends_of(&words, &foe.name, asks));
    if let Some(foe) = ended.filter(|_| !belief) {
        return Told::EndOf(foe);
    }
    let Some(verb) = deed_verb(&words, belief) else {
        return Told::Nothing;
    };
    foes.iter()
        .filter_map(|foe| Some((words.name_at(0..verb, &foe.name)?, foe)))
        .filter(|(at, _)| !after_a_preposition(&words.lower, *at))
        .min_by_key(|(at, _)| *at)
        .map_or(Told::Deed, |(_, foe)| Told::EndOf(foe))
}

/// The word before the name, over "the", is a preposition.
fn after_a_preposition(lower: &[String], at: usize) -> bool {
    let mut before = at;
    while before > 0 && lower[before - 1] == "the" {
        before -= 1;
    }
    before
        .checked_sub(1)
        .is_some_and(|word| PREPOSITIONS.contains(&lower[word].as_str()))
}

/// The sentence tells a deed with no list of foes: adventurers or agents with a result, or
/// a passive end of a pronoun with no other doer.
#[must_use]
pub fn tells_a_deed(sentence: &str) -> bool {
    let words = Words::of(sentence);
    deed_verb(&words, is_belief(&words)).is_some()
}

/// The sentence tells some end, read loosely: a result or a word of an end, not after
/// "be". The builder reads the tags only of such paragraphs, and a setup is never told
/// after one (`setup_passages`).
#[must_use]
pub fn tells_an_end(sentence: &str) -> bool {
    let words = Words::of(sentence);
    let lower = &words.lower;
    (0..lower.len()).any(|at| {
        let after_be = at > 0 && lower[at - 1] == "be";
        let ends_here = is_result_at(lower, at)
            || KILLS.contains(&lower[at].as_str())
            || is_death_at(lower, at)
            || (END_NOUNS.contains(&lower[at].as_str()) && lower[at] != "head");
        ends_here && !after_be
    })
}

/// The index of the last verb of a deed: a result with adventurers in the sentence, a
/// kill by agents of a side, or a passive kill of "he" or "she" with no other doer. In a
/// belief, only adventurers count.
fn deed_verb(words: &Words, belief: bool) -> Option<usize> {
    let lower = &words.lower;
    let by_doers = holds_any(&words.without_possessives(), &DOERS);
    (0..lower.len()).rev().find(|at| {
        let result = is_result_at(lower, *at);
        let by_others = !belief && (has_agent(lower, *at) || pronoun_end_at(lower, *at));
        (result && by_doers) || by_others
    })
}

fn is_result_at(lower: &[String], at: usize) -> bool {
    let starts_here = |phrase: &&[&str]| starts_with(&lower[at..], phrase);
    RESULTS.iter().any(starts_here) || REVEALS.iter().any(starts_here)
}

fn starts_with(words: &[String], phrase: &[&str]) -> bool {
    words.len() >= phrase.len() && words[..phrase.len()] == *phrase
}

fn is_death_at(lower: &[String], at: usize) -> bool {
    let rest = &lower[at + 1..];
    match lower[at].as_str() {
        "fell" => rest.is_empty() || FELL_ENDS.iter().any(|end| starts_with(rest, end)),
        word => DEATHS.contains(&word),
    }
}

fn is_belief(words: &Words) -> bool {
    words
        .lower
        .iter()
        .any(|word| BELIEFS.contains(&word.as_str()))
}

/// The sentence tells the end of the person: after an active kill, right before a passive
/// kill or a death, after "the death of", or right before "'s death". `asks` is true for a
/// sentence with a setup cue: "retrieve Rend's head" asks for an end, and tells none.
fn ends_of(words: &Words, name: &str, asks: bool) -> bool {
    (0..words.len()).any(|at| ends_at(words, at, name, asks))
}

fn ends_at(words: &Words, at: usize, name: &str, asks: bool) -> bool {
    let lower = &words.lower;
    let word = lower[at].as_str();
    let after = at + 1..at + 1 + REACH;
    if KILLS.contains(&word) {
        return match passive_mark(lower, at) {
            Some(mark) => name_ends_before(words, mark, name),
            None => words
                .name_at(at + 1..at + 1 + REACH + 2, name)
                .is_some_and(|first| first <= at + REACH && !owns_more(words, first, name)),
        };
    }
    if is_death_at(lower, at) {
        return name_ends_before(words, at, name);
    }
    if !END_NOUNS.contains(&word) || (word == "head" && asks) {
        return false;
    }
    let of_the_person =
        lower.get(at + 1).is_some_and(|next| next == "of") && words.names(after, name);
    of_the_person || name_ends_before(words, at, name)
}

/// The first word of the verb group of a passive verb: "was" of "was eventually
/// defeated", "had" of "had been slain". None for an active verb: in "it is unclear who
/// murdered", "who" stands between.
fn passive_mark(lower: &[String], at: usize) -> Option<usize> {
    let mut before = at;
    while before > 0 && is_adverb(&lower[before - 1]) {
        before -= 1;
    }
    let mark = before.checked_sub(1)?;
    BE.contains(&lower[mark].as_str())
        .then(|| first_of_the_verb_group(lower, mark))
}

/// The name that starts at `first` is a possessive: "dispatched Isillien's Crimson Elites"
/// tells the end of the elites.
fn owns_more(words: &Words, first: usize, name: &str) -> bool {
    (first..words.len().min(first + 4))
        .take_while(|at| *at == first || words.name_ends_at(*at, name))
        .any(|at| words.possessive[at])
}

/// "had been killed" starts at "had".
fn first_of_the_verb_group(lower: &[String], mark: usize) -> usize {
    let mut first = mark;
    let in_the_group =
        |word: &String| FILLERS.contains(&word.as_str()) || BE.contains(&word.as_str());
    while first > 0 && in_the_group(&lower[first - 1]) {
        first -= 1;
    }
    first
}

fn is_adverb(word: &str) -> bool {
    word.ends_with("ly") || ADVERBS.contains(&word)
}

/// A name of the person ends at most `NEAR` words before `at`, over adverbs in "-ly":
/// "Arugal was", "Commander Mograine's mysterious death".
fn name_ends_before(words: &Words, at: usize, name: &str) -> bool {
    let lower = &words.lower;
    let mut end = at;
    while end > 0 && is_adverb(&lower[end - 1]) {
        end -= 1;
    }
    (end.saturating_sub(NEAR)..end).any(|last| words.name_ends_at(last, name))
}

/// An agent of a side does the kill at `at`: it ends right before an active kill, or
/// stands after the "by" of a passive one, where any doer counts: "secured by hapless
/// adventurers".
fn has_agent(lower: &[String], at: usize) -> bool {
    if !KILLS.contains(&lower[at].as_str()) {
        return false;
    }
    if passive_mark(lower, at).is_none() {
        return holds_side_agent(&lower[at.saturating_sub(REACH)..at]);
    }
    let reach = lower.len().min(at + 1 + REACH);
    let Some(by) = (at + 1..reach).find(|index| lower[*index] == "by") else {
        return false;
    };
    let doer = &lower[by + 1..lower.len().min(by + 1 + REACH)];
    holds_side_agent(doer) || holds_any(doer, &DOERS)
}

/// "Alliance forces", "agents of the Horde".
fn holds_side_agent(lower: &[String]) -> bool {
    (0..lower.len()).any(|at| {
        let is_agent = AGENTS.contains(&lower[at].as_str());
        let side_before = at > 0 && SIDES.contains(&lower[at - 1].as_str());
        let side_after = lower.get(at + 1).is_some_and(|word| word == "of")
            && lower
                .get(at + 3)
                .is_some_and(|word| SIDES.contains(&word.as_str()));
        is_agent && (side_before || side_after)
    })
}

/// A passive kill of "he" or "she" at `at`, with no "by" after it: the doer may be the
/// player. "He was killed by Arthas" names another doer.
fn pronoun_end_at(lower: &[String], at: usize) -> bool {
    let reach = lower.len().min(at + 1 + REACH);
    let by_another = lower[at + 1..reach].iter().any(|word| word == "by");
    pronoun_killed_at(lower, at) && !by_another
}

fn pronoun_killed_at(lower: &[String], at: usize) -> bool {
    let Some(mark) = passive_mark(lower, at).filter(|_| KILLS.contains(&lower[at].as_str())) else {
        return false;
    };
    let subject = mark.checked_sub(1).map(|before| lower[before].as_str());
    subject.is_some_and(|word| PRONOUNS.contains(&word))
}

/// The sentence tells a passive kill of "he" or "she", by any doer: "he was later beheaded
/// by Alexandros' vengeful spirit". On the page of a foe, it tells the end of that foe.
#[must_use]
pub fn kills_a_pronoun(sentence: &str) -> bool {
    let words = Words::of(sentence);
    !is_belief(&words) && (0..words.len()).any(|at| pronoun_killed_at(&words.lower, at))
}

pub(crate) fn holds_any(words: &[String], phrases: &[&[&str]]) -> bool {
    phrases.iter().any(|phrase| holds_phrase(words, phrase))
}

pub(crate) fn holds_phrase(words: &[String], phrase: &[&str]) -> bool {
    words.windows(phrase.len()).any(|window| window == phrase)
}
