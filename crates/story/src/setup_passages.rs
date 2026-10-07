//! Setup passages (GAMEPLAY.md 5.10). A setup tells who wants a deed done in a dungeon or
//! a raid, or what threat waits there: "Gryan Stoutmantle sent adventurers to kill
//! VanCleef." It is the hook before the deed. The builder tags it with the deed and the
//! instance, and the story program shows it only until the player did that deed
//! (`spoiler.rs`).

use crate::check::words_of;
use crate::game_names;
use crate::narrator::MAX_LORE_CHARS;
use crate::outcome_passages::{Npc, PageKind, RESULTS, holds_phrase, linked_foes, own_quest};
use crate::pack::{Deed, Link, SetupFor};
use crate::sentences::sentences;
use crate::wikitext::Cites;
use std::collections::BTreeMap;

/// The verbs of a commission: someone wants others, or the world, rid of a foe. The foe
/// comes after "to": "sent adventurers to kill VanCleef". In the passive, the foe comes
/// before: "Mutanus has been ordered to prevent".
const COMMISSIONS: [&str; 27] = [
    "send",
    "sends",
    "sent",
    "ask",
    "asks",
    "asked",
    "order",
    "orders",
    "ordered",
    "task",
    "tasks",
    "tasked",
    "direct",
    "directs",
    "directed",
    "hire",
    "hires",
    "hired",
    "recruit",
    "recruits",
    "recruited",
    "enlist",
    "enlists",
    "enlisted",
    "determined",
    "vowed",
    "swore",
];

/// A commission after one of these words is passive.
const PASSIVE_MARKS: [&str; 5] = ["been", "was", "were", "is", "are"];

/// Two-word commissions, with the same rule.
const COMMISSION_PHRASES: [&[&str]; 2] = [&["called", "upon"], &["calls", "upon"]];

/// The verbs of an intent: the foe itself wants something. The foe comes before the verb:
/// "VanCleef sought to overthrow Stormwind".
const INTENTS: [&str; 10] = [
    "seek", "seeks", "sought", "want", "wants", "wanted", "plans", "planned", "hoping", "hopes",
];

/// A commission or an intent asks for one of these deeds within reach of its "to".
/// "ordered the shaman to transform his sons" asks for none of them.
const DEED_VERBS: [&str; 32] = [
    "kill",
    "slay",
    "destroy",
    "defeat",
    "stop",
    "end",
    "eliminate",
    "assassinate",
    "capture",
    "retrieve",
    "recover",
    "rescue",
    "free",
    "find",
    "investigate",
    "hunt",
    "confront",
    "attack",
    "raid",
    "assault",
    "punish",
    "weaken",
    "overthrow",
    "protect",
    "guard",
    "avenge",
    "purge",
    "cleanse",
    "seize",
    "conquer",
    "prevent",
    "claim",
];

/// "to" comes at most this many words after the verb, and the deed at most this many
/// words after "to": "called upon the heroes of the Alliance to destroy".
const REACH: usize = 8;

/// "wants VanCleef dead": "dead" comes at most this many words after the verb.
const DEAD_REACH: usize = 4;

/// The words of a threat or a plot. The foe may stand anywhere in the sentence.
const PREMISES: [&[&str]; 22] = [
    &["contend", "with"],
    &["guarded", "by"],
    &["attacks", "on"],
    &["negotiating", "with"],
    &["true", "master"],
    &["a", "bounty", "on"],
    &["a", "price", "on"],
    &["threat"],
    &["threats"],
    &["threaten"],
    &["threatens"],
    &["threatened"],
    &["threatening"],
    &["plot"],
    &["plots"],
    &["plotted"],
    &["plotting"],
    &["hatched", "a", "plan"],
    &["scheme"],
    &["schemes"],
    &["schemed"],
    &["scheming"],
];

/// "the plot of land" is no plot.
const NO_PLOT: [&str; 2] = ["plot", "of"];

/// Words of an end that the outcome detector does not count, because they need no doer.
const MORE_ENDS: [&[&str]; 4] = [
    &["brought", "back"],
    &["beheaded"],
    &["solved"],
    &["death", "of"],
];

/// Words of a name that name no one alone: "Emperor Dagran Thaurissan" is "Thaurissan", not
/// "Emperor".
const TITLES: [&str; 22] = [
    "lord",
    "lady",
    "king",
    "queen",
    "prince",
    "princess",
    "baron",
    "captain",
    "emperor",
    "high",
    "grand",
    "general",
    "commander",
    "master",
    "elder",
    "chief",
    "chieftain",
    "warchief",
    "the",
    "of",
    "devourer",
    "herald",
];

/// Words inside a name that start no name: "Aku'mai the Devourer".
const LINK_WORDS: [&str; 2] = ["the", "of"];

/// A word of a name names its owner alone when it has at least this many letters.
const NAME_WORD_CHARS: usize = 5;

/// Where in a sentence the foe of a cue must stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    /// After the word at this index: the deed of a commission.
    After(usize),
    /// Before the word at this index: the one with the intent.
    Before(usize),
    /// Between two words: "wants VanCleef dead".
    Between(usize, usize),
    Anywhere,
}

/// The sentences of `text` that set up a deed, before the first sentence that tells an
/// end: a setup after "adventurers killed him" is history, not a hook.
#[must_use]
pub fn setup_sentences(text: &str) -> Vec<&str> {
    before_the_end(text)
        .into_iter()
        .filter(|sentence| !spans_of(sentence).is_empty())
        .collect()
}

/// The passage tells the end of a deed or a foe somewhere: "adventurers killed him", or
/// "until he was killed". Such a passage is never a setup as a whole. Its setup, when it
/// has one, goes into the pack as a passage of its own (`window`).
#[must_use]
pub fn tells_an_end(text: &str) -> bool {
    sentences(text).iter().any(|sentence| ends(sentence))
}

/// A result after "be" is no end: "outsiders must be destroyed".
fn ends(sentence: &str) -> bool {
    let words = words_of(sentence);
    let ended = |phrase: &&[&str]| {
        (0..words.len()).any(|at| {
            holds_phrase(&words[at..words.len().min(at + phrase.len())], phrase)
                && (at == 0 || words[at - 1] != "be")
        })
    };
    RESULTS.iter().any(ended) || MORE_ENDS.iter().any(ended)
}

fn before_the_end(text: &str) -> Vec<&str> {
    sentences(text)
        .into_iter()
        .take_while(|sentence| !ends(sentence))
        .collect()
}

/// The words of a sentence with their case, so "to Stormwind" is no verb.
fn words_in_case(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '\u{2019}'))
        .filter(|word| !word.is_empty())
        .collect()
}

fn lower_words(sentence: &str) -> Vec<String> {
    words_in_case(sentence)
        .iter()
        .map(|word| word.to_lowercase())
        .collect()
}

/// Each cue of a setup in the sentence, with where its foe must stand.
fn spans_of(sentence: &str) -> Vec<Span> {
    let words = words_in_case(sentence);
    let lower = lower_words(sentence);
    let mut spans = Vec::new();
    for at in 0..lower.len() {
        let to = deed_after_to(&words, &lower, at + 1);
        if let Some(to) = to
            && is_commission_at(&lower, at)
        {
            spans.push(Span::After(to));
            let passive = at > 0 && PASSIVE_MARKS.contains(&lower[at - 1].as_str());
            if passive {
                spans.push(Span::Before(at));
            }
        }
        if to.is_some() && INTENTS.contains(&lower[at].as_str()) {
            spans.push(Span::Before(at));
        }
        if let Some(dead) = dead_after(&lower, at) {
            spans.push(Span::Between(at, dead));
        }
    }
    if holds_premise(&lower) {
        spans.push(Span::Anywhere);
    }
    spans
}

fn is_commission_at(lower: &[String], at: usize) -> bool {
    if COMMISSIONS.contains(&lower[at].as_str()) {
        return true;
    }
    let end = lower.len().min(at + 2);
    COMMISSION_PHRASES
        .iter()
        .any(|phrase| holds_phrase(&lower[at..end], phrase))
}

/// The index of a "to" within reach of `from`, with a verb in lower case after it, and a
/// deed within reach after it: "to venture into the prison and kill".
fn deed_after_to(words: &[&str], lower: &[String], from: usize) -> Option<usize> {
    let end = lower.len().min(from + REACH);
    (from..end).filter(|at| lower[*at] == "to").find(|at| {
        let starts_a_verb = words
            .get(at + 1)
            .is_some_and(|next| next.starts_with(char::is_lowercase));
        let deed_end = lower.len().min(at + 1 + REACH);
        let asks_a_deed = lower[at + 1..deed_end]
            .iter()
            .any(|word| DEED_VERBS.contains(&word.as_str()));
        starts_a_verb && asks_a_deed
    })
}

fn dead_after(lower: &[String], at: usize) -> Option<usize> {
    if !["want", "wants", "wanted"].contains(&lower[at].as_str()) {
        return None;
    }
    let end = lower.len().min(at + 1 + DEAD_REACH);
    (at + 1..end).find(|index| lower[*index] == "dead")
}

fn holds_premise(lower: &[String]) -> bool {
    let plot_of = lower.windows(NO_PLOT.len()).any(|pair| pair == NO_PLOT);
    PREMISES
        .iter()
        .filter(|phrase| !(plot_of && phrase.first() == Some(&"plot")))
        .any(|phrase| holds_phrase(lower, phrase))
}

/// The dungeons and raids of the pack, and the places of each known boss.
#[derive(Clone, Debug, Default)]
pub struct Instances {
    pub names: Vec<String>,
    pub bosses: BTreeMap<String, Vec<String>>,
}

impl Instances {
    /// The places of the known boss that `name` names, under either of its names
    /// (`game_names`): the page "Dagran Thaurissan" holds the infobox name "Emperor Dagran
    /// Thaurissan".
    fn places_of(&self, name: &str) -> Option<&Vec<String>> {
        self.bosses
            .iter()
            .find(|(boss, _)| game_names::same(boss, name))
            .map(|(_, places)| places)
    }
}

/// A paragraph with a setup sentence, as the builder found it: its text, its cites, its
/// page, and its links.
pub struct Found<'a> {
    pub text: &'a str,
    pub cites: &'a Cites,
    pub page: &'a str,
    pub links: &'a [Link],
}

/// The deed that a paragraph sets up, and its instance, or None: the paragraph then sets
/// up nothing.
///
/// 1. The deed is a foe of the page or of the links of the paragraph, when a setup
///    sentence names it where its cue says: after "to" for a commission, before the verb
///    for an intent, and anywhere for a threat or a plot.
/// 2. Else it is the foe of the page, when a setup sentence names its instance in the
///    span of a cue: on the page "Herod", "determined to eliminate the leadership of the
///    Scarlet Monastery" sets up the defeat of Herod.
/// 3. Else it is the quest of the paragraph, by the rule of outcome passages
///    (`outcome_passages::own_quest`), when a setup sentence holds a commission.
///
/// The instance is the one of the foe, when the foe is a known boss. Else it is the first
/// instance that the paragraph links to.
#[must_use]
pub fn setup_for(
    found: &Found<'_>,
    instances: &Instances,
    kind_of: impl Fn(&str) -> Option<PageKind>,
) -> Option<SetupFor> {
    let foes = linked_foes(found.cites, found.page, &kind_of);
    let deed = named_foe(found.text, &foes)
        .or_else(|| own_foe_by_its_instance(found, instances, &kind_of))
        .map(Deed::Foe)
        .or_else(|| {
            has_commission(found.text)
                .then(|| own_quest(found.cites, &kind_of))
                .flatten()
                .map(Deed::Quest)
        })?;
    let instance = instance_of(&deed, found.links, instances)?;
    Some(SetupFor { deed, instance })
}

fn own_foe_by_its_instance(
    found: &Found<'_>,
    instances: &Instances,
    kind_of: &impl Fn(&str) -> Option<PageKind>,
) -> Option<String> {
    let own = linked_foes(&Cites::default(), found.page, kind_of)
        .into_iter()
        .next()?;
    let places = instances.places_of(&own.name)?;
    let named = setup_sentences(found.text).into_iter().any(|sentence| {
        let lower = lower_words(sentence);
        spans_of(sentence).into_iter().any(|span| {
            let words = span_words(&lower, span);
            places.iter().any(|place| holds_name(words, place))
        })
    });
    named.then_some(own.name)
}

/// The words hold the whole name, with or without "the": "the Scarlet Monastery".
fn holds_name(words: &[String], name: &str) -> bool {
    let parts: Vec<String> = name.split_whitespace().map(str::to_lowercase).collect();
    let bare = parts.strip_prefix(&["the".to_string()]).unwrap_or(&parts);
    !bare.is_empty() && words.windows(bare.len()).any(|window| window == bare)
}

fn has_commission(text: &str) -> bool {
    setup_sentences(text)
        .iter()
        .any(|sentence| is_commission(sentence))
}

fn is_commission(sentence: &str) -> bool {
    spans_of(sentence)
        .iter()
        .any(|span| matches!(span, Span::After(_)))
}

/// The foe that the first setup sentence names in the span of a cue. Of two foes in one
/// span, the one that comes first wins: "sent a team to kill Thaurissan and retrieve
/// Moira" sets up the defeat of Thaurissan.
fn named_foe(text: &str, foes: &[Npc]) -> Option<String> {
    setup_sentences(text)
        .into_iter()
        .find_map(|sentence| first_named(sentence, foes))
}

fn first_named(sentence: &str, foes: &[Npc]) -> Option<String> {
    let lower = lower_words(sentence);
    let mut best: Option<(usize, &Npc)> = None;
    for span in spans_of(sentence) {
        let words = span_words(&lower, span);
        for foe in foes {
            let Some(at) = name_at(words, &foe.name) else {
                continue;
            };
            if best.is_none_or(|(kept, _)| at < kept) {
                best = Some((at, foe));
            }
        }
    }
    best.map(|(_, foe)| foe.name.clone())
}

/// The sentence sets up the defeat of `foe`: one of its cues names the foe in its span.
fn sets_up(sentence: &str, foe: &str) -> bool {
    let lower = lower_words(sentence);
    spans_of(sentence)
        .into_iter()
        .any(|span| names_in(span_words(&lower, span), foe))
}

fn span_words(lower: &[String], span: Span) -> &[String] {
    let range = match span {
        Span::After(at) => at..lower.len(),
        Span::Before(at) => 0..at,
        Span::Between(from, to) => from..to,
        Span::Anywhere => 0..lower.len(),
    };
    lower.get(range).unwrap_or_default()
}

/// The words name the person: the whole name, or one long word of it that is no title, so
/// "VanCleef" names "Edwin VanCleef".
fn names_in(words: &[String], name: &str) -> bool {
    name_at(words, name).is_some()
}

/// The index of the first word of `words` that names the person.
fn name_at(words: &[String], name: &str) -> Option<usize> {
    let parts: Vec<String> = name.split_whitespace().map(str::to_lowercase).collect();
    let whole = (!parts.is_empty())
        .then(|| {
            words
                .windows(parts.len())
                .position(|window| window == parts)
        })
        .flatten();
    // "Emperor Thaurissan" is not "Moira Thaurissan": a title before the word must be one
    // of this name.
    let titled_other = |at: usize| {
        at > 0 && TITLES.contains(&words[at - 1].as_str()) && !parts.contains(&words[at - 1])
    };
    let significant = parts
        .iter()
        .filter(|part| part.chars().count() >= NAME_WORD_CHARS)
        .filter(|part| !TITLES.contains(&part.as_str()))
        .any(|part| (0..words.len()).any(|at| words[at] == **part && !titled_other(at)));
    if !significant {
        return whole;
    }
    // A title counts for where the name starts: "Emperor Thaurissan" starts at "Emperor".
    let start = parts
        .iter()
        .filter(|part| !LINK_WORDS.contains(&part.as_str()))
        .filter_map(|part| words.iter().position(|word| word == part))
        .min();
    whole.into_iter().chain(start).min()
}

fn instance_of(deed: &Deed, links: &[Link], instances: &Instances) -> Option<String> {
    let is_instance = |name: &&String| instances.names.contains(name);
    if let Deed::Foe(name) = deed
        && let Some(places) = instances.places_of(name)
    {
        return places.iter().find(is_instance).cloned();
    }
    links
        .iter()
        .filter_map(|link| match link {
            Link::Place(name) => Some(name),
            Link::Npc(_) | Link::Common => None,
        })
        .find(is_instance)
        .cloned()
}

/// The part of a setup passage that a prompt shows: the first sentence that sets up
/// `deed`, with the sentence before it when both fit the lore of a prompt
/// (`narrator::MAX_LORE_CHARS`). A foe needs its name in the span of a cue, and a quest
/// needs a commission. None when no sentence before an end sets up the deed.
#[must_use]
pub fn window<'a>(text: &'a str, deed: &Deed) -> Option<&'a str> {
    let kept = before_the_end(text);
    let found = kept.iter().position(|sentence| match deed {
        Deed::Foe(name) => sets_up(sentence, name),
        Deed::Quest(_) => is_commission(sentence),
    })?;
    let last = kept[found];
    let before = kept[found.saturating_sub(1)];
    let both = offset_in(text, last)? + last.len() - offset_in(text, before)?;
    let first = if both <= MAX_LORE_CHARS { before } else { last };
    let start = offset_in(text, first)?;
    let end = offset_in(text, last)? + last.len();
    text.get(start..end)
}

/// Where `part`, a slice of `text`, starts in it. A search for the words could find an
/// earlier sentence with the same words.
fn offset_in(text: &str, part: &str) -> Option<usize> {
    part.as_ptr().addr().checked_sub(text.as_ptr().addr())
}
