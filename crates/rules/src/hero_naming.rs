//! How a line names the hero when a word for the hero also names a group of the line
//! (docs/plans/narrator-style.md 4.1). "The Society gains strength, and the Forsaken has
//! reached 225 in Alchemy" reads as if the whole people did it. So a race, a class, or a
//! title that clashes falls back, and `$N` always fits. Lean proves the laws
//! (lean/Timeways/HeroNaming.lean). The story program decides which word clashes.

/// The naming that the rotation asks for on one call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn {
    Name,
    Race,
    Class,
    Title,
    Unnamed,
}

/// One word for the hero, as the line sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Word {
    /// The hero has no such word: no race yet, or no title.
    Missing,
    /// The word, its plural, or its people names a group of the line.
    Clashes,
    Clear,
}

/// The race, the class, and the title of the hero, as the line sees them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Words {
    pub race: Word,
    pub class: Word,
    pub title: Word,
}

/// How the line names the hero.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Naming {
    Name,
    Race,
    Class,
    Title,
    Unnamed,
}

/// The naming of a turn. A race that is not clear gives the class, and then the name. A
/// class that clashes gives the name, and only a missing class gives the race.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn choose(turn: Turn, words: Words) -> Naming {
    match turn {
        Turn::Name => Naming::Name,
        Turn::Unnamed => Naming::Unnamed,
        Turn::Race => race_first(words),
        Turn::Class => class_first(words),
        Turn::Title => title_first(words),
    }
}

fn race_first(words: Words) -> Naming {
    if is_clear(words.race) {
        return Naming::Race;
    }
    if is_clear(words.class) {
        return Naming::Class;
    }
    Naming::Name
}

fn class_first(words: Words) -> Naming {
    if is_clear(words.class) {
        return Naming::Class;
    }
    if is_missing(words.class) && is_clear(words.race) {
        return Naming::Race;
    }
    Naming::Name
}

fn title_first(words: Words) -> Naming {
    if is_clear(words.title) {
        return Naming::Title;
    }
    Naming::Name
}

fn is_clear(word: Word) -> bool {
    match word {
        Word::Clear => true,
        Word::Missing | Word::Clashes => false,
    }
}

fn is_missing(word: Word) -> bool {
    match word {
        Word::Missing => true,
        Word::Clear | Word::Clashes => false,
    }
}
