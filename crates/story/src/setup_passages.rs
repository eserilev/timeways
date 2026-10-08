//! Setup passages (GAMEPLAY.md 5.10). A setup tells who wants a deed done in a dungeon or
//! a raid, or what threat waits there: "Gryan Stoutmantle sent adventurers to kill
//! VanCleef." It is the hook before the deed. The builder tags it with the deed and the
//! instance, and the story program shows it only until the player did that deed
//! (`spoiler.rs`).

use crate::ends::{self, holds_phrase};
use crate::game_names;
use crate::name_match::{Words, lower_words, words_in_case};
use crate::narrator::MAX_LORE_CHARS;
use crate::outcome_passages::{Npc, PageKind, linked_foes, own_quest};
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
/// "VanCleef sought to overthrow Stormwind". A wish or a hope is no hook: "Thaurissan
/// wanted to free his people" tells a love story.
const INTENTS: [&str; 5] = ["seek", "seeks", "sought", "plans", "planned"];

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

/// The deeds that bind a foe: the foe of a commission stands after one of these.
/// "retrieve", "rescue", "free", "recover", "find", and "investigate" bind none: "to kill
/// Thaurissan and retrieve Moira" asks for the defeat of Thaurissan only. "protect" and
/// "guard" bind the ward of a foe.
const HOSTILE_DEEDS: [&str; 20] = [
    "kill",
    "slay",
    "destroy",
    "defeat",
    "stop",
    "end",
    "eliminate",
    "assassinate",
    "hunt",
    "confront",
    "attack",
    "raid",
    "assault",
    "punish",
    "weaken",
    "overthrow",
    "purge",
    "conquer",
    "protect",
    "guard",
];

/// After "and", one of these starts no second deed: "to destroy the Brotherhood and the
/// Edwin at their head".
const NO_VERBS: [&str; 9] = [
    "the", "a", "an", "his", "her", "their", "its", "all", "other",
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

/// Where in a sentence the foe of a cue must stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Span {
    /// From the word at this index to the end of its deed: the deed of a commission.
    After(usize, usize),
    /// A commission whose deed binds no foe: "sent a group to investigate".
    NoFoe,
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
    sentences(text)
        .iter()
        .any(|sentence| ends::tells_an_end(sentence))
}

/// The sentence holds a cue of a setup: it asks for a deed, and tells none.
#[must_use]
pub fn asks(sentence: &str) -> bool {
    !spans_of(sentence).is_empty()
}

fn before_the_end(text: &str) -> Vec<&str> {
    sentences(text)
        .into_iter()
        .take_while(|sentence| !ends::tells_an_end(sentence))
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
            spans.push(deed_span(&words, &lower, to));
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

/// The span of the deed after "to": up to the next "and" before a second verb, after the
/// verb of the deed. "to venture into the prison and kill Bazil" keeps "kill Bazil". A
/// deed that binds no foe gives `Span::NoFoe`, unless it asks for a head: "to retrieve
/// Rend's head".
fn deed_span(words: &[&str], lower: &[String], to: usize) -> Span {
    let first_deed = (to + 1..lower.len()).find(|at| DEED_VERBS.contains(&lower[*at].as_str()));
    let second_verb = (first_deed.unwrap_or(lower.len())..lower.len()).find(|at| {
        lower[*at] == "and"
            && words
                .get(at + 1)
                .is_some_and(|next| next.starts_with(char::is_lowercase))
            && !NO_VERBS.contains(&lower[at + 1].as_str())
    });
    let end = second_verb.unwrap_or(lower.len());
    let deed = &lower[to + 1..end];
    let verb = deed.iter().find(|word| DEED_VERBS.contains(&word.as_str()));
    let hostile = verb.is_some_and(|verb| HOSTILE_DEEDS.contains(&verb.as_str()));
    if hostile || deed.iter().any(|word| word == "head") {
        Span::After(to, end)
    } else {
        Span::NoFoe
    }
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
            let commission = first_commission(found.text)?;
            own_quest(&found.cites.of_sentence(commission), &kind_of).map(Deed::Quest)
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
            let words = lower.get(span_range(span, lower.len())).unwrap_or_default();
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

/// The first setup sentence with a commission: the quest of a setup is the one that it
/// cites.
fn first_commission(text: &str) -> Option<&str> {
    setup_sentences(text)
        .into_iter()
        .find(|sentence| is_commission(sentence))
}

fn is_commission(sentence: &str) -> bool {
    spans_of(sentence)
        .iter()
        .any(|span| matches!(span, Span::After(..) | Span::NoFoe))
}

/// The foe that the first setup sentence names in the span of a cue. Of two foes in one
/// span, the one that comes first wins: "sent a team to kill Thaurissan and destroy the
/// Brotherhood" sets up the defeat of Thaurissan.
fn named_foe(text: &str, foes: &[Npc]) -> Option<String> {
    setup_sentences(text)
        .into_iter()
        .find_map(|sentence| asked_foe(sentence, foes))
        .map(|foe| foe.name.clone())
}

/// The foe that a cue of the sentence asks to end, the first in the sentence, or None.
#[must_use]
pub fn asked_foe<'a>(sentence: &str, foes: &'a [Npc]) -> Option<&'a Npc> {
    let words = Words::of(sentence);
    let mut best: Option<(usize, &Npc)> = None;
    for span in spans_of(sentence) {
        let range = span_range(span, words.len());
        for foe in foes {
            let Some(at) = words.name_at(range.clone(), &foe.name) else {
                continue;
            };
            if best.is_none_or(|(kept, _)| at < kept) {
                best = Some((at, foe));
            }
        }
    }
    best.map(|(_, foe)| foe)
}

/// The sentence sets up the defeat of `foe`: one of its cues names the foe in its span.
fn sets_up(sentence: &str, foe: &str) -> bool {
    let words = Words::of(sentence);
    spans_of(sentence)
        .into_iter()
        .any(|span| words.names(span_range(span, words.len()), foe))
}

fn span_range(span: Span, len: usize) -> std::ops::Range<usize> {
    match span {
        Span::After(at, end) => at..end,
        Span::NoFoe => 0..0,
        Span::Before(at) => 0..at,
        Span::Between(from, to) => from..to,
        Span::Anywhere => 0..len,
    }
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
