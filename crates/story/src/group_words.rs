//! The words for the hero that also name a group (docs/plans/narrator-style.md 4.1). "The
//! Society gains strength, and the Forsaken has reached 225 in Alchemy" reads as if the
//! whole people reached it. So the hero is never named by a word that the same text uses
//! for a group or a people. The build picks another naming (`narrator::naming_in`), and
//! the prose check refuses a free text that does it (`group_word_hero_in`).

use crate::check::{mentions, words_of};
use crate::race_class::{Class, Race};
use crate::sentences::sentences;

/// Verbs right after "the <word>" that only one person takes: "the tauren has". A people
/// whose word is its own plural takes "have", "grow", or "hold".
const SINGULAR_VERBS: [&str; 19] = [
    "has", "is", "was", "does", "reaches", "holds", "finishes", "defeats", "earns", "gains",
    "carries", "rides", "wields", "bears", "keeps", "wins", "takes", "stands", "kills",
];

/// Past verbs that take a person or a people: "the dwarf had".
const PAST_VERBS: [&str; 2] = ["had", "did"];

/// What "the <word>" names in a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Names {
    /// "The Forsaken" names the whole people, so it never names the hero.
    OnlyThePeople,
    /// "The tauren" or "the paladin" names one person when nothing in the line clashes.
    OnePerson,
}

/// One word for the hero: a race, a class, or a title, and the words that name its group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeroWord {
    pub word: String,
    pub plural: String,
    /// More words for the people of the word: "undead" for the Forsaken.
    pub people: Vec<String>,
    pub names: Names,
}

impl HeroWord {
    #[must_use]
    pub fn race(race: Race) -> HeroWord {
        HeroWord {
            word: race.word().to_string(),
            plural: race.plural().to_string(),
            people: race
                .people_words()
                .iter()
                .map(|w| (*w).to_string())
                .collect(),
            names: if race.names_only_the_people() {
                Names::OnlyThePeople
            } else {
                Names::OnePerson
            },
        }
    }

    /// "shamans" is a plural that a line can use too.
    #[must_use]
    pub fn class(class: Class) -> HeroWord {
        let people = if class.plural() == class.word() {
            vec![format!("{}s", class.word())]
        } else {
            Vec::new()
        };
        HeroWord {
            word: class.word().to_string(),
            plural: class.plural().to_string(),
            people,
            names: Names::OnePerson,
        }
    }

    #[must_use]
    pub fn title(title: &str) -> HeroWord {
        HeroWord {
            word: title.to_string(),
            plural: format!("{title}s"),
            people: Vec::new(),
            names: Names::OnePerson,
        }
    }

    /// "Tauren" and "shaman" are their own plural, so "the tauren hunt" names the group.
    /// Only a verb that one person takes makes them name the hero.
    #[must_use]
    pub fn is_its_own_plural(&self) -> bool {
        self.word.eq_ignore_ascii_case(&self.plural)
    }

    /// True when the word cannot name the hero in a line whose other words are `rest`:
    /// the word names only the people, or the word, its plural, or its people stands in
    /// `rest`.
    #[must_use]
    pub fn clashes_with(&self, rest: &str) -> bool {
        self.names == Names::OnlyThePeople
            || mentions(rest, &self.word)
            || mentions(rest, &self.plural)
            || self.people.iter().any(|word| mentions(rest, word))
    }

    /// True when `rest` names the group of the word: "the warrior served, and the warrior
    /// owns" names one person twice, but "the tauren has, and the tauren hunt" names a
    /// group.
    fn group_in(&self, rest: &str) -> bool {
        self.names == Names::OnlyThePeople
            || mentions(rest, &self.plural)
            || self.people.iter().any(|word| mentions(rest, word))
    }
}

/// The first "the <word> <verb>" of `text` that names the hero by a word that names a
/// group of the text, in lower case: "the forsaken has". The words are every race and
/// class, and each title of `also_hero`.
#[must_use]
pub fn group_word_hero_in(text: &str, also_hero: &[String]) -> Option<String> {
    candidates(also_hero)
        .iter()
        .find_map(|word| clashing_naming(text, word))
}

fn candidates(also_hero: &[String]) -> Vec<HeroWord> {
    let races = Race::all().map(HeroWord::race);
    let classes = Class::all().map(HeroWord::class);
    let mut words: Vec<HeroWord> = races.into_iter().chain(classes).collect();
    let titles = also_hero
        .iter()
        .filter(|title| Race::from_word(title).is_none() && Class::from_word(title).is_none());
    words.extend(titles.map(|title| HeroWord::title(title)));
    words
}

/// The naming of the hero by `word` in `text`, when the rest of the text names its group.
fn clashing_naming(text: &str, word: &HeroWord) -> Option<String> {
    let naming = sentences(text)
        .into_iter()
        .find_map(|sentence| hero_naming_in(sentence, word))?;
    let rest = without_first(&words_of(text), &words_of(&naming));
    word.group_in(&rest).then_some(naming)
}

/// The words of a text, joined, with the first run of `cut` taken out.
fn without_first(words: &[String], cut: &[String]) -> String {
    let at = words
        .windows(cut.len())
        .position(|window| window == cut)
        .unwrap_or(words.len());
    let after = (at + cut.len()).min(words.len());
    let kept: Vec<&str> = words[..at]
        .iter()
        .chain(&words[after..])
        .map(String::as_str)
        .collect();
    kept.join(" ")
}

/// "the paladin has" in one sentence: "the", the word, and a verb that one person takes.
/// A word at the end of a sentence names a person too ("fell to the paladin."), unless
/// it is its own plural.
fn hero_naming_in(sentence: &str, word: &HeroWord) -> Option<String> {
    let words = words_of(sentence);
    let wanted = words_of(&word.word);
    let size = wanted.len() + 1;
    (0..words.len()).find_map(|at| {
        let run = words.get(at..at + size)?;
        if run[0] != "the" || run[1..] != wanted[..] {
            return None;
        }
        let next = words.get(at + size).map(String::as_str);
        if !takes_a_person(next, word.is_its_own_plural()) {
            return None;
        }
        let named = run.join(" ");
        Some(next.map_or(named.clone(), |verb| format!("{named} {verb}")))
    })
}

fn takes_a_person(next: Option<&str>, own_plural: bool) -> bool {
    let Some(verb) = next else {
        return !own_plural;
    };
    if SINGULAR_VERBS.contains(&verb) {
        return true;
    }
    let past = PAST_VERBS.contains(&verb) || verb.ends_with("ed");
    past && !own_plural
}
