//! The groups of a pairing of race and class: who grows stronger at a tenth level, and the
//! order of a class quest (docs/plans/narrator-templates.md 3.4, docs/plans/level-lines.md).
//! The set is built from the data, never listed for each of the 72 pairs.

use crate::check::mentions;
use crate::narrator_templates::{GroupData, Number, PeopleData, Templates};
use crate::race_class::{Class, Race};

/// The id of the group of the class of the faction: "the paladins of the Horde".
pub const CLASS_FACTION: &str = "g.class_faction";

/// The id of the group of the people: "the Forsaken".
pub const PEOPLE: &str = "g.people";

/// One group that a line can tell of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub text: String,
    /// The form when the lore already names the group: "the order".
    pub short: Option<String>,
    pub number: Number,
    /// The words of the lore that name the group.
    pub anchors: Vec<String>,
}

impl Group {
    /// True when the lore names the group: one of its anchors, as whole words.
    #[must_use]
    pub fn is_named_in(&self, lore: &str) -> bool {
        self.anchors.iter().any(|anchor| mentions(lore, anchor))
    }

    /// The words of the line: the short form when the lore names the group already, so
    /// "The Silver Hand ... The Silver Hand grows stronger." cannot happen.
    #[must_use]
    pub fn value(&self, lore: &str) -> String {
        match &self.short {
            Some(short) if mentions(lore, &self.text) || self.is_named_in(lore) => short.clone(),
            _ => self.text.clone(),
        }
    }

    /// A named order of the lore, such as the Silver Hand, and not a phrase.
    #[must_use]
    pub fn is_order(&self) -> bool {
        self.id.starts_with("o.")
    }
}

/// The groups of a pairing: the class of the faction, the people, and each named order or
/// strange phrase that fits.
#[must_use]
pub fn groups_of(templates: &Templates, race: Race, class: Class) -> Vec<Group> {
    let Some(people) = people_of(templates, race) else {
        return Vec::new();
    };
    let plural = class_plural(templates, class);
    let mut groups = vec![
        Group {
            id: CLASS_FACTION.to_string(),
            text: format!("the {plural} of the {}", people.faction),
            short: None,
            number: Number::Many,
            anchors: vec![people.faction.clone(), plural.to_string()],
        },
        Group {
            id: PEOPLE.to_string(),
            text: people.text.clone(),
            short: None,
            number: people.number,
            anchors: people.anchors.clone(),
        },
    ];
    let fitting = templates
        .groups
        .iter()
        .filter(|group| fits(group, people, class));
    groups.extend(fitting.map(|group| Group {
        id: group.id.clone(),
        text: group.text.replace("{people_pl}", &people.plural),
        short: group.short.clone(),
        number: group.number,
        anchors: group.anchors.clone(),
    }));
    groups
}

/// The named order of a class quest: the first order of the pairing that the lore names.
#[must_use]
pub fn order_of(templates: &Templates, race: Race, class: Class, lore: &str) -> Option<Group> {
    groups_of(templates, race, class)
        .into_iter()
        .find(|group| group.is_order() && group.is_named_in(lore))
}

/// True when the lore finds the pairing strange: a race that Classic never gave the class.
#[must_use]
pub fn is_strange(templates: &Templates, race: Race, class: Class) -> bool {
    templates
        .classes
        .iter()
        .find(|data| data.class == class.word())
        .is_some_and(|data| !data.races.iter().any(|known| known == race.word()))
}

/// The faction of a race, as in Classic: "Alliance" or "Horde".
#[must_use]
pub fn faction_of(templates: &Templates, race: Race) -> Option<&str> {
    people_of(templates, race).map(|people| people.faction.as_str())
}

fn people_of(templates: &Templates, race: Race) -> Option<&PeopleData> {
    templates
        .peoples
        .iter()
        .find(|people| people.race == race.word())
}

fn class_plural(templates: &Templates, class: Class) -> &str {
    templates
        .classes
        .iter()
        .find(|data| data.class == class.word())
        .map_or(class.word(), |data| data.plural.as_str())
}

fn fits(group: &GroupData, people: &PeopleData, class: Class) -> bool {
    let class_fits = group.classes.iter().any(|known| known == class.word());
    let faction_fits = group.factions.is_empty() || group.factions.contains(&people.faction);
    let race_fits = group.races.is_empty() || group.races.contains(&people.race);
    class_fits && faction_fits && race_fits
}
