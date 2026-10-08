//! The answer of the narrator model: one JSON object with a sentence of history and a few
//! choices from closed lists (docs/plans/narrator-templates.md 2). The parser is strict:
//! one object, with no unknown, missing, or doubled field. It allows one code fence
//! around the object, because small models add one.

use crate::line_check::SILENCE;
use crate::narrator_templates::{Kind, Number};
use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::Value;
use std::fmt;

/// A closed choice of the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceField {
    /// Which group the history tells of: an id of the pairing.
    Group,
    /// The history names the place where it happened.
    There,
    /// The group that the foe led, as the lore names it, or "none".
    Leads,
    LeadsNumber,
    Tone,
    /// One named person, or one of many.
    Killer,
    /// The history tells of the breed of the mount.
    Breed,
}

impl ChoiceField {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            ChoiceField::Group => "group",
            ChoiceField::There => "there",
            ChoiceField::Leads => "leads",
            ChoiceField::LeadsNumber => "leads_number",
            ChoiceField::Tone => "tone",
            ChoiceField::Killer => "killer",
            ChoiceField::Breed => "breed",
        }
    }
}

/// The closed fields of each moment, in the order of the prompt (2.2).
#[must_use]
pub fn fields_of(kind: Kind) -> &'static [ChoiceField] {
    match kind {
        Kind::Level => &[ChoiceField::Group],
        Kind::Kill => &[
            ChoiceField::There,
            ChoiceField::Leads,
            ChoiceField::LeadsNumber,
        ],
        Kind::Revenge => &[ChoiceField::Tone],
        Kind::Death => &[ChoiceField::Killer, ChoiceField::Tone, ChoiceField::There],
        Kind::Slap => &[ChoiceField::Tone, ChoiceField::There],
        Kind::Mount | Kind::EpicMount => &[ChoiceField::Breed],
        Kind::Item => &[ChoiceField::There],
        Kind::Arrival
        | Kind::Setup
        | Kind::SideQuest
        | Kind::ClassQuest
        | Kind::Title
        | Kind::Mark => &[],
    }
}

/// The dry tone allows the dry parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Dry,
}

/// The killer of a death.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillerKind {
    One,
    Kind,
}

/// The choices of an answer. A field that the moment does not offer stays None.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Choices {
    pub group: Option<String>,
    pub there: Option<bool>,
    /// The group that the foe led. None for "none".
    pub leads: Option<String>,
    pub leads_number: Option<Number>,
    pub tone: Option<Tone>,
    pub killer: Option<KillerKind>,
    pub breed: Option<bool>,
}

/// A parsed answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// The model had nothing true to tell.
    Silence,
    Answer {
        lore: String,
        choices: Choices,
    },
}

/// Why an answer does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotFault {
    /// No single JSON object, an unknown, missing, or doubled field, or a wrong type.
    BadAnswer,
    /// A value outside the list that the prompt offered.
    UnknownChoice(&'static str),
}

/// The answer of `text` for a moment of `kind`. `groups` holds the group ids that the
/// prompt offered.
///
/// # Errors
///
/// Returns the first fault of the answer.
pub fn parse(text: &str, kind: Kind, groups: &[String]) -> Result<Parsed, SlotFault> {
    let body = without_fence(text.trim());
    if body == SILENCE {
        return Ok(Parsed::Silence);
    }
    let Entries(entries) = serde_json::from_str(body).map_err(|_| SlotFault::BadAnswer)?;
    let lore = match find(&entries, "lore") {
        // The line shows on one line, as the checks read it.
        Some(Value::String(lore)) => lore.split_whitespace().collect::<Vec<_>>().join(" "),
        _ => return Err(SlotFault::BadAnswer),
    };
    if lore == SILENCE {
        return Ok(Parsed::Silence);
    }
    let wanted = fields_of(kind);
    let known = |key: &str| key == "lore" || wanted.iter().any(|field| field.name() == key);
    let doubled =
        (1..entries.len()).any(|at| entries[..at].iter().any(|(key, _)| *key == entries[at].0));
    if doubled || entries.len() != wanted.len() + 1 || !entries.iter().all(|(key, _)| known(key)) {
        return Err(SlotFault::BadAnswer);
    }
    let mut choices = Choices::default();
    for field in wanted {
        let value = find(&entries, field.name()).ok_or(SlotFault::BadAnswer)?;
        choose(&mut choices, *field, value, groups)?;
    }
    Ok(Parsed::Answer { lore, choices })
}

fn choose(
    choices: &mut Choices,
    field: ChoiceField,
    value: &Value,
    groups: &[String],
) -> Result<(), SlotFault> {
    let unknown = SlotFault::UnknownChoice(field.name());
    match field {
        ChoiceField::There => choices.there = Some(boolean(value)?),
        ChoiceField::Breed => choices.breed = Some(boolean(value)?),
        ChoiceField::Group => {
            let group = text(value)?;
            if !groups.iter().any(|known| known == group) {
                return Err(unknown);
            }
            choices.group = Some(group.to_string());
        }
        ChoiceField::Leads => {
            let leads = text(value)?.trim();
            choices.leads = (leads != "none").then(|| leads.to_string());
        }
        ChoiceField::LeadsNumber => {
            choices.leads_number = Some(match text(value)? {
                "one" => Number::One,
                "many" => Number::Many,
                _ => return Err(unknown),
            });
        }
        ChoiceField::Tone => {
            choices.tone = Some(match text(value)? {
                "plain" => Tone::Plain,
                "dry" => Tone::Dry,
                _ => return Err(unknown),
            });
        }
        ChoiceField::Killer => {
            choices.killer = Some(match text(value)? {
                "one" => KillerKind::One,
                "kind" => KillerKind::Kind,
                _ => return Err(unknown),
            });
        }
    }
    Ok(())
}

fn boolean(value: &Value) -> Result<bool, SlotFault> {
    value.as_bool().ok_or(SlotFault::BadAnswer)
}

fn text(value: &Value) -> Result<&str, SlotFault> {
    value.as_str().ok_or(SlotFault::BadAnswer)
}

fn find<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find(|(known, _)| known == key)
        .map(|(_, value)| value)
}

/// The text inside one code fence, with or without a language: "```json ... ```".
fn without_fence(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    let Some(inner) = rest.strip_suffix("```") else {
        return text;
    };
    let inner = inner.strip_prefix("json").unwrap_or(inner);
    inner.trim()
}

/// The fields of one JSON object, in order, doubles kept, so the parser can refuse them.
struct Entries(Vec<(String, Value)>);

impl<'de> Deserialize<'de> for Entries {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Entries, D::Error> {
        deserializer.deserialize_map(EntriesVisitor)
    }
}

struct EntriesVisitor;

impl<'de> Visitor<'de> for EntriesVisitor {
    type Value = Entries;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entries, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry::<String, Value>()? {
            entries.push(entry);
        }
        Ok(Entries(entries))
    }
}
