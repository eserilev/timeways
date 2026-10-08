//! Outcome passages (GAMEPLAY.md 5.10). The wiki often tells the deed of a quest as
//! history: "He was killed at the hands of adventurers sent by Gryan Stoutmantle." The
//! builder keeps such a passage, and tags it with each deed that it tells: a foe, a quest,
//! or unresolved. The story program shows it only after the player did every deed
//! (`spoiler.rs`). The sentences of an end are in `ends`.

use crate::ends::{self, Told, told};
use crate::game_names;
use crate::pack::Dependency;
use crate::sentences::sentences;
use crate::setup_passages;
use crate::wikitext::{Cites, plain, template_fields};

/// The infobox templates of the wiki.
const NPC_BOX: &str = "Npcbox";
const QUEST_BOX: &str = "Questbox";

/// The factions of an infobox that fight every adventurer.
const FOE_FACTIONS: [&str; 2] = ["combat", "boss"];

/// The `aggro` of an infobox for an NPC that is hostile to the Alliance and to the Horde.
const HOSTILE_TO_BOTH: &str = "{{Aggro|-1|-1}}";

/// The paragraph tells some end, read loosely, so the builder reads its tags
/// (`dependencies`).
#[must_use]
pub fn may_tell_an_end(text: &str) -> bool {
    sentences(text)
        .iter()
        .any(|sentence| ends::tells_an_end(sentence))
}

/// The text tells a deed with no list of foes: adventurers or agents with a result, or a
/// passive end of a pronoun. A line of passages by hand that tells one and has no tag is
/// unresolved (`timeways-pack`).
#[must_use]
pub fn is_outcome(text: &str) -> bool {
    sentences(text)
        .iter()
        .any(|sentence| ends::tells_a_deed(sentence))
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
/// a person in a dungeon, such as "Mr. Smite". The page title and the infobox name can be
/// two names of one person (`game_names`).
#[must_use]
pub fn with_known_bosses(kind: PageKind, bosses: &[String]) -> PageKind {
    let known = |name: &str| bosses.iter().any(|boss| game_names::same(boss, name));
    match kind {
        PageKind::Npc(npc) if known(&npc.name) => PageKind::Npc(Npc {
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

/// A paragraph that the builder tags, with what the list knows of its page.
pub struct Paragraph<'a> {
    pub text: &'a str,
    pub cites: &'a Cites,
    /// The title of the page of the paragraph.
    pub page: &'a str,
    /// The known bosses of the list (`pack_sources::known_bosses`).
    pub bosses: &'a [String],
    /// The foes of the page from the list, by their names in the game: "Archmage Arugal"
    /// for "Shadowfang Keep", and "Edwin VanCleef" for "Defias Brotherhood". A deed of the
    /// page that names no foe is the defeat of the first.
    pub foes: &'a [String],
}

/// Each deed that the paragraph tells, once, in the order of the text. Empty for no deed.
///
/// 1. A sentence that tells the end of a foe, or a deed that names a foe, gives that foe.
///    The foes are the page, the foes that the line links to, the known bosses with a page
///    of a person, and the foes of the page in the list.
/// 2. A deed sentence that names no foe takes the foe that a cue of the sentence before it
///    asks to end: "Stoutmantle called upon the heroes to destroy Edwin. The heroes
///    succeeded."
/// 3. Else it takes the last cited quest of its own part of the line that ends a chain,
///    when its title is its own (`own_quest`).
/// 4. A deed sentence with none of these adds no tag of its own, when another sentence
///    gave one. When none did, the passage depends on the page when it is a foe, else on
///    the first foe of the page in the list, else it is unresolved.
/// 5. On the page of a foe, a passive kill of "he" or "she" by any doer is the end of that
///    foe: "he was later beheaded by Alexandros' vengeful spirit".
///
/// `kind_of` gives the kind of a page that the line links to or cites, and of the page of
/// the passage.
#[must_use]
pub fn dependencies(
    paragraph: &Paragraph<'_>,
    kind_of: impl Fn(&str) -> Option<PageKind>,
) -> Vec<Dependency> {
    let foes = foes_of(paragraph, &kind_of);
    let sentences = sentences(paragraph.text);
    let mut tags = Vec::new();
    let mut untied = false;
    for (index, sentence) in sentences.iter().enumerate() {
        let asks = setup_passages::asks(sentence);
        let tag = match told(sentence, &foes, asks) {
            Told::Nothing => match pronoun_end_of_the_page(sentence, paragraph, &kind_of) {
                Some(tag) => Some(tag),
                None => continue,
            },
            Told::EndOf(foe) => Some(Dependency::Foe(foe.name.clone())),
            Told::Deed => {
                let before = index.checked_sub(1).map(|at| sentences[at]);
                tie(before, sentence, paragraph.cites, &foes, &kind_of)
            }
        };
        match tag {
            Some(tag) if !tags.contains(&tag) => tags.push(tag),
            Some(_) => {}
            None => untied = true,
        }
    }
    if tags.is_empty() && untied {
        tags.push(fallback(paragraph, &kind_of));
    }
    tags
}

/// The tag of a deed sentence that names no foe, by rules 2 and 3 of `dependencies`.
fn tie(
    before: Option<&str>,
    sentence: &str,
    cites: &Cites,
    foes: &[Npc],
    kind_of: &impl Fn(&str) -> Option<PageKind>,
) -> Option<Dependency> {
    let named_before = before.and_then(|before| setup_passages::asked_foe(before, foes));
    if let Some(foe) = named_before {
        return Some(Dependency::Foe(foe.name.clone()));
    }
    own_quest(&cites.of_sentence(sentence), kind_of).map(Dependency::Quest)
}

/// Rule 5 of `dependencies`.
fn pronoun_end_of_the_page(
    sentence: &str,
    paragraph: &Paragraph<'_>,
    kind_of: &impl Fn(&str) -> Option<PageKind>,
) -> Option<Dependency> {
    if !ends::kills_a_pronoun(sentence) {
        return None;
    }
    foe(kind_of(paragraph.page)).map(|npc| Dependency::Foe(npc.name))
}

/// Rule 4 of `dependencies`.
fn fallback(paragraph: &Paragraph<'_>, kind_of: &impl Fn(&str) -> Option<PageKind>) -> Dependency {
    let own = foe(kind_of(paragraph.page)).map(|npc| npc.name);
    own.or_else(|| paragraph.foes.first().cloned())
        .map_or(Dependency::Unresolved, Dependency::Foe)
}

/// The foes that a paragraph can name: its linked foes, then each known boss whose page
/// is about a person, and each foe of the page in the list. A known boss such as "Defias
/// Brotherhood" has no box of an NPC, so it is no foe.
fn foes_of(paragraph: &Paragraph<'_>, kind_of: &impl Fn(&str) -> Option<PageKind>) -> Vec<Npc> {
    let mut foes = linked_foes(paragraph.cites, paragraph.page, kind_of);
    let people = paragraph
        .bosses
        .iter()
        .filter(|boss| matches!(kind_of(boss), Some(PageKind::Npc(_))));
    let more = people.chain(paragraph.foes);
    for name in more {
        if !foes.iter().any(|foe| game_names::same(&foe.name, name)) {
            foes.push(Npc {
                name: name.clone(),
                stance: Stance::Foe,
            });
        }
    }
    foes
}

/// The foes of a paragraph: the page of the passage when it is a foe, then each foe that
/// the paragraph links to. A setup passage reads them too (`setup_passages`).
#[must_use]
pub fn linked_foes(
    cites: &Cites,
    page: &str,
    kind_of: &impl Fn(&str) -> Option<PageKind>,
) -> Vec<Npc> {
    let own = foe(kind_of(page));
    let linked = cites.links.iter().filter_map(|title| foe(kind_of(title)));
    own.into_iter().chain(linked).collect()
}

/// The title of the last cited quest that ends a chain, when the title is its own.
#[must_use]
pub fn own_quest(cites: &Cites, kind_of: &impl Fn(&str) -> Option<PageKind>) -> Option<String> {
    last_chain_end(cites, kind_of)
        .filter(|quest| quest.title == Title::Own)
        .map(|quest| quest.name)
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
