//! Outcome passages (GAMEPLAY.md 5.10). The wiki often tells the deed of a quest as
//! history: "He was killed at the hands of adventurers sent by Gryan Stoutmantle." The
//! builder keeps such a passage, and tags it with the deed that it tells: a foe, a quest,
//! or unresolved. The story program shows it only after the player did that deed
//! (`spoiler.rs`).

use crate::check::mentions;
use crate::pack::Dependency;
use crate::sentences::sentences;
use crate::wikitext::{Cites, plain, template_fields};

/// The words for the people who did the deed. "adventurers" also covers "a group of
/// adventurers". A possessive, such as "the adventurer's guild", is another word.
const DOERS: [&[&str]; 11] = [
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
];

/// The words of a result: the deed is done, not only asked for. "sent adventurers to
/// kill" asks, so the plain verb is no result.
const RESULTS: [&[&str]; 29] = [
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
    &["saved"],
    &["secured"],
    &["captured"],
    &["dispatched"],
    &["disabled"],
    &["executed"],
    &["banished"],
    &["avenged"],
    &["obtained"],
    &["uncovered"],
    &["discovered"],
    &["put", "down"],
    &["put", "an", "end"],
    &["to", "justice"],
];

/// The infobox templates of the wiki.
const NPC_BOX: &str = "Npcbox";
const QUEST_BOX: &str = "Questbox";

/// The factions of an infobox that fight every adventurer.
const FOE_FACTIONS: [&str; 2] = ["combat", "boss"];

/// The `aggro` of an infobox for an NPC that is hostile to the Alliance and to the Horde.
const HOSTILE_TO_BOTH: &str = "{{Aggro|-1|-1}}";

/// The sentences of `text` that tell a deed: each names the doers and a result.
#[must_use]
pub fn deed_sentences(text: &str) -> Vec<&str> {
    sentences(text)
        .into_iter()
        .filter(|sentence| tells_a_deed(sentence))
        .collect()
}

#[must_use]
pub fn is_outcome(text: &str) -> bool {
    sentences(text)
        .iter()
        .any(|sentence| tells_a_deed(sentence))
}

fn tells_a_deed(sentence: &str) -> bool {
    let words = words_with_apostrophes(sentence);
    holds_any(&words, &DOERS) && holds_any(&words, &RESULTS)
}

/// The words in lower case. An apostrophe stays inside its word, so "adventurer's" is
/// not "adventurer".
fn words_with_apostrophes(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn holds_any(words: &[String], phrases: &[&[&str]]) -> bool {
    phrases.iter().any(|phrase| holds_phrase(words, phrase))
}

fn holds_phrase(words: &[String], phrase: &[&str]) -> bool {
    words.windows(phrase.len()).any(|window| window == phrase)
}

/// What the builder reads from the infobox of a page that a passage links to or cites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageKind {
    Npc(Npc),
    Quest(Quest),
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Npc {
    pub name: String,
    pub stance: Stance,
}

/// A foe fights adventurers in the game: its faction is "Combat" or "Boss", or it is
/// hostile to both sides. A known boss of the pack is a foe too (`with_known_bosses`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    Foe,
    NoFoe,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quest {
    /// The title of the quest in the game.
    pub name: String,
    /// Whether a quest comes after it.
    pub chain: Chain,
    /// The page title has a number, such as "The Defias Brotherhood (7)": other quests of
    /// the game have the same title, so the title alone cannot tell them apart.
    pub title: Title,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    End,
    Continues,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Title {
    Own,
    Shared,
}

/// The kind of the page `title` from its wikitext.
#[must_use]
pub fn page_kind(title: &str, text: &str) -> PageKind {
    if let Some(fields) = template_fields(text, NPC_BOX) {
        return PageKind::Npc(Npc {
            name: field(&fields, "name").unwrap_or_else(|| without_suffix(title).to_string()),
            stance: stance(&fields),
        });
    }
    let Some(fields) = template_fields(text, QUEST_BOX) else {
        return PageKind::Other;
    };
    let chain = match field(&fields, "next") {
        Some(_) => Chain::Continues,
        None => Chain::End,
    };
    let title_kind = if has_number_suffix(title) {
        Title::Shared
    } else {
        Title::Own
    };
    PageKind::Quest(Quest {
        name: field(&fields, "name").unwrap_or_else(|| without_suffix(title).to_string()),
        chain,
        title: title_kind,
    })
}

fn stance(fields: &[(String, String)]) -> Stance {
    let faction = field(fields, "faction").unwrap_or_default().to_lowercase();
    let aggro = raw_field(fields, "aggro").unwrap_or_default();
    if FOE_FACTIONS.contains(&faction.as_str()) || aggro.contains(HOSTILE_TO_BOTH) {
        Stance::Foe
    } else {
        Stance::NoFoe
    }
}

/// The plain text of a field with a value. An empty field counts as no field.
fn field(fields: &[(String, String)], key: &str) -> Option<String> {
    raw_field(fields, key)
        .map(|value| plain(value).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn raw_field<'a>(fields: &'a [(String, String)], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

/// A known boss of the pack is a foe, whatever its infobox says: a page of the list about
/// a person in a dungeon, such as "Mr. Smite".
#[must_use]
pub fn with_known_bosses(kind: PageKind, bosses: &[String]) -> PageKind {
    match kind {
        PageKind::Npc(npc) if bosses.contains(&npc.name) => PageKind::Npc(Npc {
            stance: Stance::Foe,
            ..npc
        }),
        other => other,
    }
}

/// "Kirtonos the Herald (Classic)" gives "Kirtonos the Herald".
fn without_suffix(title: &str) -> &str {
    match title.rfind(" (") {
        Some(at) if title.ends_with(')') => &title[..at],
        _ => title,
    }
}

fn has_number_suffix(title: &str) -> bool {
    let Some(inside) = title
        .strip_suffix(')')
        .and_then(|rest| rest.rsplit_once(" ("))
        .map(|(_, inside)| inside)
    else {
        return false;
    };
    !inside.is_empty() && inside.chars().all(|c| c.is_ascii_digit())
}

/// What an outcome passage depends on, best first:
///
/// 1. a foe that a sentence of the deed names: the page of the passage, then each link;
/// 2. the last cited quest that ends a chain, when its title is its own. A shared title
///    cannot tell the end of a chain from its start, so it counts as no quest;
/// 3. the page of the passage, when it is a foe;
/// 4. the first foe that the paragraph links to.
///
/// Else the deed is unresolved. `kind_of` gives the kind of a page that the paragraph
/// links to or cites, and of the page of the passage.
#[must_use]
pub fn dependency(
    text: &str,
    cites: &Cites,
    page: &str,
    kind_of: impl Fn(&str) -> Option<PageKind>,
) -> Dependency {
    let own = foe(kind_of(page));
    let linked: Vec<Npc> = cites
        .links
        .iter()
        .filter_map(|title| foe(kind_of(title)))
        .collect();
    let foes: Vec<&Npc> = own.iter().chain(&linked).collect();
    let deeds = deed_sentences(text);
    let named = foes
        .iter()
        .find(|foe| deeds.iter().any(|sentence| mentions(sentence, &foe.name)));
    if let Some(foe) = named {
        return Dependency::Foe(foe.name.clone());
    }
    let quest = last_chain_end(cites, &kind_of).filter(|quest| quest.title == Title::Own);
    if let Some(quest) = quest {
        return Dependency::Quest(quest.name);
    }
    foes.first().map_or(Dependency::Unresolved, |foe| {
        Dependency::Foe(foe.name.clone())
    })
}

fn foe(kind: Option<PageKind>) -> Option<Npc> {
    match kind {
        Some(PageKind::Npc(npc)) if npc.stance == Stance::Foe => Some(npc),
        _ => None,
    }
}

/// The last quest that ends a chain among the references, else among the links.
fn last_chain_end(cites: &Cites, kind_of: &impl Fn(&str) -> Option<PageKind>) -> Option<Quest> {
    last_chain_end_of(&cites.refs, kind_of).or_else(|| last_chain_end_of(&cites.links, kind_of))
}

fn last_chain_end_of(
    titles: &[String],
    kind_of: &impl Fn(&str) -> Option<PageKind>,
) -> Option<Quest> {
    let quests = titles.iter().filter_map(|title| match kind_of(title) {
        Some(PageKind::Quest(quest)) if quest.chain == Chain::End => Some(quest),
        _ => None,
    });
    quests.last()
}
