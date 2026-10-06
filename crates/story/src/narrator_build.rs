//! The model writes the lore, and the code writes the hero
//! (docs/plans/narrator-templates.md 0). This module turns the answer of the model into
//! the line that the player sees: the parse of the slot JSON, the checks of the lore, the
//! pick of a shape in turn, the render, and the checks of the built line.

use crate::check::{mentions, words_of};
use crate::line_check::{Grounds, LineFault, built_faults, lore_faults};
use crate::moments::{Creature, Moment, SlotKind};
use crate::narrator::{MAX_LINE_CHARS, Naming, Who, naming};
use crate::narrator_groups::{Group, groups_of, is_strange, order_of};
use crate::narrator_render::{
    Values, count_number, count_words, ordinal, quoted, render, with_article,
};
use crate::narrator_slots::{Choices, KillerKind, Parsed, SlotFault, Tone, parse};
use crate::narrator_templates::{Kind, Need, Number, ShapeInfo, Slot, TEMPLATES, Templates};
use timeways_rules::narrator_shapes::{self as rules, Facts, Token};

/// The main part of the line of an arrival, for the window of the rotation.
pub const PLACE_SHAPE: &str = "f.place";

/// A leader's group is at most this many words: "the Riverpaw".
const MOST_LEADS_WORDS: usize = 4;

/// The longest group name that the budget plans for, before the model names one.
const LEADS_ROOM: &str = "the Defias Brotherhood of Westfall";

/// An arrival may take two sentences of history; a deed takes one.
const ARRIVAL_SENTENCES: usize = 2;
const DEED_SENTENCES: usize = 1;

/// What one call knows of its moment.
#[derive(Clone, Debug)]
pub struct Setup {
    pub moment: Moment,
    pub who: Who,
    /// The number of the call: it picks the naming and the start of the rotation.
    pub turn: usize,
    /// The main parts of the last accepted lines of the character, oldest first.
    pub recent: Vec<String>,
}

/// What the prompt offers: the kind, the groups of a pairing, and the budget of the lore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub kind: Kind,
    pub groups: Vec<Group>,
    /// The most characters of the lore, so the longest shape still fits the line.
    pub budget: usize,
    /// The pairing is one that the lore finds strange (docs/plans/level-lines.md 2).
    pub strange: bool,
}

impl Offer {
    /// The ids of the groups, as the prompt lists them.
    #[must_use]
    pub fn group_ids(&self) -> Vec<String> {
        self.groups.iter().map(|group| group.id.clone()).collect()
    }
}

/// A built line, the main part of its shape, its parts, and whether it names the hero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Built {
    pub line: String,
    /// The main part, which the window of the rotation keeps: "k.fell".
    pub shape: String,
    pub parts: Vec<String>,
    pub named: bool,
}

/// The verdict on an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answered {
    Line(Built),
    /// The model had nothing true to tell.
    Silent,
    Refused(Vec<LineFault>),
}

/// The kind of a moment. A flavor moment keeps free text (question 3 of the plan).
#[must_use]
pub fn kind_of(moment: &Moment) -> Option<Kind> {
    let kind = match moment {
        Moment::Flavor { .. } => return None,
        Moment::NewZone { .. } | Moment::FirstCapital { .. } | Moment::FirstInstance { .. } => {
            Kind::Arrival
        }
        Moment::FirstKill { .. } => Kind::Kill,
        Moment::Revenge { .. } => Kind::Revenge,
        Moment::SlainAgain { .. } => Kind::Death,
        Moment::QuestDone { .. } => Kind::SideQuest,
        Moment::ClassQuestDone { .. } => Kind::ClassQuest,
        Moment::Titled { .. } => Kind::Title,
        Moment::Slapped { .. } => Kind::Slap,
        Moment::QuestMarked { .. } => Kind::Mark,
        Moment::FirstMount { .. } => Kind::Mount,
        Moment::FirstEpicMount { .. } => Kind::EpicMount,
        Moment::FirstEpicItem { .. } | Moment::BigUpgrade { .. } => Kind::Item,
        Moment::LevelUp { .. } => Kind::Level,
    };
    Some(kind)
}

/// The offer of a moment, or None when no shape can fit it whatever the model answers:
/// then the moment is silent before any call. A tenth level needs the race and the class.
#[must_use]
pub fn offer(setup: &Setup) -> Option<Offer> {
    let templates = TEMPLATES.as_ref().ok()?;
    let kind = kind_of(&setup.moment)?;
    let (groups, strange) = match kind {
        Kind::Level | Kind::ClassQuest => {
            let (race, class) = (setup.who.race?, setup.who.class?);
            let strange = is_strange(templates, race, class);
            (groups_of(templates, race, class), strange)
        }
        _ => (Vec::new(), false),
    };
    if kind == Kind::Level && groups.is_empty() {
        return None;
    }
    let mut offer = Offer {
        kind,
        groups,
        budget: MAX_LINE_CHARS,
        strange,
    };
    if kind == Kind::Arrival {
        return Some(offer);
    }
    let longest = longest_shape(templates, setup, &offer)?;
    offer.budget = MAX_LINE_CHARS.checked_sub(longest + 1)?;
    Some(offer)
}

/// The longest line that a shape can build for the moment, with an empty lore and the
/// longest value of each choice.
fn longest_shape(templates: &Templates, setup: &Setup, offer: &Offer) -> Option<usize> {
    let mut longest = None;
    for group in offer.groups.iter().map(Some).chain([None]) {
        let plan = Plan::widest(templates, setup, offer, group);
        for info in templates.shapes(offer.kind) {
            for named in [true, false] {
                let facts = plan.facts(named);
                let Some(tokens) = rules::assemble(&templates.table, &info.shape, &facts) else {
                    continue;
                };
                if !rules::fits(&templates.table, &info.shape, &facts) {
                    continue;
                }
                let values = plan.values(named, "");
                let length = render(templates, &tokens, &values).chars().count();
                longest = Some(longest.unwrap_or(0).max(length));
            }
        }
    }
    longest
}

/// The line of an answer, or why it is refused.
#[must_use]
pub fn answered(
    text: &str,
    setup: &Setup,
    offer: &Offer,
    grounds: &Grounds,
    player_text: &str,
) -> Answered {
    let parsed = match parse(text, offer.kind, &offer.group_ids()) {
        Ok(parsed) => parsed,
        Err(SlotFault::BadAnswer) => return Answered::Refused(vec![LineFault::BadAnswer]),
        Err(SlotFault::UnknownChoice(field)) => {
            return Answered::Refused(vec![LineFault::UnknownChoice(field.to_string())]);
        }
    };
    let Parsed::Answer { lore, choices } = parsed else {
        return Answered::Silent;
    };
    let most = if offer.kind == Kind::Arrival {
        ARRIVAL_SENTENCES
    } else {
        DEED_SENTENCES
    };
    let mut faults = lore_faults(&lore, grounds, player_text, most, offer.budget);
    let built = build(setup, offer, &lore, &choices);
    match built {
        Ok(built) => {
            if faults.is_empty() {
                faults = built_faults(&built.line, &lore, grounds);
            }
            if faults.is_empty() {
                return Answered::Line(built);
            }
            Answered::Refused(faults)
        }
        Err(mut more) => {
            faults.append(&mut more);
            Answered::Refused(faults)
        }
    }
}

/// The line of a lore and its choices: the shape that the rotation picks, rendered.
///
/// # Errors
///
/// Returns the choices that the lore does not hold, or no fault when no shape fits: then
/// the moment is silent.
pub fn build(
    setup: &Setup,
    offer: &Offer,
    lore: &str,
    choices: &Choices,
) -> Result<Built, Vec<LineFault>> {
    let Ok(templates) = TEMPLATES.as_ref() else {
        return Err(Vec::new());
    };
    if offer.kind == Kind::Arrival {
        let values = Values {
            lore: lore.to_string(),
            ..Values::default()
        };
        let line = render(templates, &rules::assemble_arrival(), &values);
        return Ok(Built {
            line,
            shape: PLACE_SHAPE.to_string(),
            parts: vec![PLACE_SHAPE.to_string()],
            named: false,
        });
    }
    let plan = Plan::of(templates, setup, offer, lore, choices)?;
    let (index, named) = pick_shape(templates, &plan, setup).ok_or_else(Vec::new)?;
    let info = &templates.shapes(offer.kind)[index];
    let tokens =
        rules::assemble(&templates.table, &info.shape, &plan.facts(named)).ok_or_else(Vec::new)?;
    Ok(built(templates, &plan, info, &tokens, named, lore))
}

/// The line of every shape that fits the answer, on a named and on an unnamed turn, for
/// the review of the templates and the property tests.
///
/// # Errors
///
/// Returns the choices that the lore does not hold.
pub fn every_line(
    setup: &Setup,
    offer: &Offer,
    lore: &str,
    choices: &Choices,
) -> Result<Vec<Built>, Vec<LineFault>> {
    let Ok(templates) = TEMPLATES.as_ref() else {
        return Err(Vec::new());
    };
    let plan = Plan::of(templates, setup, offer, lore, choices)?;
    let mut lines = Vec::new();
    for info in templates.shapes(offer.kind) {
        for named in [true, false] {
            let facts = plan.facts(named);
            if !rules::fits(&templates.table, &info.shape, &facts) {
                continue;
            }
            let Some(tokens) = rules::assemble(&templates.table, &info.shape, &facts) else {
                continue;
            };
            lines.push(built(templates, &plan, info, &tokens, named, lore));
        }
    }
    Ok(lines)
}

fn built(
    templates: &Templates,
    plan: &Plan,
    info: &ShapeInfo,
    tokens: &[Token],
    named: bool,
    lore: &str,
) -> Built {
    Built {
        line: render(templates, tokens, &plan.values(named, lore)),
        shape: templates
            .mains
            .get(usize::from(info.main))
            .cloned()
            .unwrap_or_default(),
        parts: info.parts.clone(),
        named,
    }
}

/// The values and the facts of one answer.
struct Plan {
    kind: Kind,
    slots: Vec<(Slot, String)>,
    holds: u32,
    naming: Naming,
}

impl Plan {
    /// The plan of an answer: its choices must hold in the lore.
    fn of(
        templates: &Templates,
        setup: &Setup,
        offer: &Offer,
        lore: &str,
        choices: &Choices,
    ) -> Result<Plan, Vec<LineFault>> {
        let mut plan = Plan::base(templates, setup, offer.kind);
        let mut faults = Vec::new();
        let zone = zone_of(&setup.moment);
        if choices.there == Some(true) {
            match zone {
                Some(zone) if mentions(lore, zone) => plan.need(Need::There),
                _ => faults.push(not_in_lore("there")),
            }
        }
        if choices.tone == Some(Tone::Dry) {
            plan.need(Need::Dry);
        }
        if let Some(killer) = choices.killer {
            plan.killer(templates, &setup.moment, killer);
        }
        if let Some(leads) = &choices.leads {
            let named = words_of(leads).len() <= MOST_LEADS_WORDS
                && mentions(lore, leads.strip_prefix("the ").unwrap_or(leads));
            if named {
                plan.slot(Slot::Led, leads.clone());
                plan.need(Need::Leads);
                plan.need(choices.leads_number.unwrap_or(Number::Many).need());
            } else {
                faults.push(not_in_lore("leads"));
            }
        }
        if choices.breed == Some(true) {
            match breed_of(&setup.moment) {
                Some(breed) if names_breed(lore, &breed) => {
                    plan.slot(Slot::Breed, breed);
                    plan.need(Need::Breed);
                }
                _ => faults.push(not_in_lore("breed")),
            }
        }
        if let Some(id) = &choices.group {
            let group = offer.groups.iter().find(|group| group.id == *id);
            match group {
                Some(group) if group.is_named_in(lore) => plan.group(group, lore),
                _ => faults.push(not_in_lore("group")),
            }
        }
        if offer.kind == Kind::ClassQuest {
            plan.order(templates, &setup.who, lore);
        }
        if faults.is_empty() {
            Ok(plan)
        } else {
            Err(faults)
        }
    }

    /// The plan with the longest value of each choice, for the budget.
    fn widest(templates: &Templates, setup: &Setup, offer: &Offer, group: Option<&Group>) -> Plan {
        let mut plan = Plan::base(templates, setup, offer.kind);
        for need in [Need::There, Need::Dry, Need::Leads, Need::One, Need::Many] {
            plan.need(need);
        }
        plan.slot(Slot::Led, LEADS_ROOM.to_string());
        plan.killer(templates, &setup.moment, KillerKind::Kind);
        if let Some(breed) = breed_of(&setup.moment) {
            plan.slot(Slot::Breed, breed);
            plan.need(Need::Breed);
        }
        if let Some(group) = group {
            plan.slot(Slot::Group, group.text.clone());
            if offer.kind == Kind::ClassQuest && group.is_order() {
                plan.slot(Slot::Order, group.text.clone());
                plan.need(Need::Order);
            }
        }
        plan
    }

    /// The values that the moment holds itself, and its naming.
    fn base(templates: &Templates, setup: &Setup, kind: Kind) -> Plan {
        let mut plan = Plan {
            kind,
            slots: Vec::new(),
            holds: 0,
            naming: naming(&setup.moment, &setup.who, setup.turn),
        };
        if kind == Kind::Title && matches!(plan.naming, Naming::Title(_)) {
            plan.naming = Naming::Name;
        }
        plan.moment_values(templates, &setup.moment);
        plan
    }

    fn moment_values(&mut self, templates: &Templates, moment: &Moment) {
        if let Some(zone) = zone_of(moment) {
            self.slot(Slot::Zone, zone.to_string());
        }
        match moment {
            Moment::FirstKill { foe, creature, .. } => {
                self.slot(Slot::Foe, foe.clone());
                if let Some(creature) = creature {
                    self.need(creature_need(*creature));
                }
            }
            Moment::Revenge { foe, deaths, .. } => {
                self.slot(Slot::Foe, foe.clone());
                self.counts(*deaths);
                self.slot(Slot::Ordinal, ordinal(deaths.saturating_add(1)));
            }
            Moment::SlainAgain { times, .. } => {
                self.counts(*times);
                self.slot(Slot::Ordinal, ordinal(*times));
            }
            Moment::Slapped { npc, times, .. } => {
                self.slot(Slot::Npc, npc.clone());
                self.counts(*times);
            }
            Moment::QuestDone { title, .. } | Moment::ClassQuestDone { title } => {
                self.slot(Slot::Quest, quoted(title));
            }
            Moment::Titled { title } => self.slot(Slot::Title, quoted(title)),
            Moment::QuestMarked { mark, quest } => {
                self.slot(Slot::Mark, quoted(mark));
                self.slot(Slot::Quest, quoted(quest));
            }
            Moment::FirstMount { mount, .. } | Moment::FirstEpicMount { mount, .. } => {
                self.slot(Slot::MountA, with_article(templates, mount));
            }
            Moment::FirstEpicItem { item, slot, .. } | Moment::BigUpgrade { item, slot, .. } => {
                self.slot(Slot::Item, item.clone());
                match slot {
                    Some(SlotKind::Weapon) => self.need(Need::Weapon),
                    Some(SlotKind::Worn) => self.need(Need::Worn),
                    None => {}
                }
            }
            Moment::LevelUp { level, .. } => self.slot(Slot::Level, level.to_string()),
            Moment::Flavor { .. }
            | Moment::NewZone { .. }
            | Moment::FirstCapital { .. }
            | Moment::FirstInstance { .. } => {}
        }
    }

    fn counts(&mut self, count: i64) {
        self.slot(Slot::Count, count_words(count));
        self.slot(Slot::CountNum, count_number(count));
        if count >= 2 {
            self.need(Need::Repeat);
        }
    }

    fn killer(&mut self, templates: &Templates, moment: &Moment, kind: KillerKind) {
        let Moment::SlainAgain { killer, .. } = moment else {
            return;
        };
        let id = match kind {
            KillerKind::One => "kr.one",
            KillerKind::Kind => "kr.kind",
        };
        self.slot(Slot::Killer, templates.killer_phrase(id, killer));
    }

    fn group(&mut self, group: &Group, lore: &str) {
        self.slot(Slot::Group, group.value(lore));
        self.need(group.number.need());
    }

    fn order(&mut self, templates: &Templates, who: &Who, lore: &str) {
        let (Some(race), Some(class)) = (who.race, who.class) else {
            return;
        };
        if let Some(order) = order_of(templates, race, class, lore) {
            self.slot(Slot::Order, order.value(lore));
            self.need(Need::Order);
        }
    }

    fn slot(&mut self, slot: Slot, value: String) {
        self.slots.retain(|(known, _)| *known != slot);
        self.slots.push((slot, value));
    }

    fn need(&mut self, need: Need) {
        self.holds |= need.bit();
    }

    fn facts(&self, named: bool) -> Facts {
        let mut has = vec![false; Slot::COUNT];
        for (slot, _) in &self.slots {
            if let Some(known) = has.get_mut(usize::from(slot.id())) {
                *known = true;
            }
        }
        Facts {
            has,
            holds: self.holds,
            named,
        }
    }

    fn values(&self, named: bool, lore: &str) -> Values {
        let hero = match (named, hero_words(&self.naming)) {
            (false, _) => None,
            (true, Some(words)) => Some(words),
            (true, None) => hero_words(&Naming::Name),
        };
        Values {
            lore: lore.to_string(),
            hero,
            slots: self.slots.iter().cloned().collect(),
        }
    }

    fn is_named(&self) -> bool {
        !matches!(self.naming, Naming::Unnamed | Naming::Absent)
    }
}

/// The words of a naming: `$N`, "the paladin", "the Bookworm".
fn hero_words(naming: &Naming) -> Option<String> {
    match naming {
        Naming::Name => Some(crate::house::NAME_MARK.to_string()),
        Naming::Kind(word) => Some(format!("the {word}")),
        Naming::Title(title) => Some(format!("the {title}")),
        Naming::Unnamed | Naming::Absent => None,
    }
}

/// The shape of the line, and whether it names the hero. A kill prefers a part with no
/// hero: on a named turn it walks the unnamed parts first (3.5). A turn whose naming fits
/// no shape takes the other naming, so a moment with a lore sentence always has a line.
fn pick_shape(templates: &Templates, plan: &Plan, setup: &Setup) -> Option<(usize, bool)> {
    let shapes = templates.shapes(plan.kind);
    let recent = recent_mains(templates, &setup.recent);
    let named = plan.is_named();
    if plan.kind == Kind::Kill && named {
        let unnamed = pick_with(templates, shapes, plan, &recent, setup.turn, false);
        if let Some(index) = unnamed
            && !recent.contains(&shapes[index].main)
        {
            return Some((index, false));
        }
    }
    let first = pick_with(templates, shapes, plan, &recent, setup.turn, named);
    if let Some(index) = first {
        return Some((index, named));
    }
    pick_with(templates, shapes, plan, &recent, setup.turn, !named).map(|index| (index, !named))
}

fn pick_with(
    templates: &Templates,
    shapes: &[ShapeInfo],
    plan: &Plan,
    recent: &[u16],
    turn: usize,
    named: bool,
) -> Option<usize> {
    let facts = plan.facts(named);
    let fits: Vec<bool> = shapes
        .iter()
        .map(|info| rules::fits(&templates.table, &info.shape, &facts))
        .collect();
    let mains: Vec<u16> = shapes.iter().map(|info| info.main).collect();
    let turn = u64::try_from(turn).unwrap_or(u64::MAX);
    rules::pick(&fits, &mains, recent, turn)
}

/// The main parts of the recent lines, as ids of the templates. An old id that the data
/// no longer holds counts for nothing.
fn recent_mains(templates: &Templates, recent: &[String]) -> Vec<u16> {
    recent
        .iter()
        .filter_map(|main| templates.mains.iter().position(|known| known == main))
        .filter_map(|index| u16::try_from(index).ok())
        .collect()
}

fn zone_of(moment: &Moment) -> Option<&str> {
    match moment {
        Moment::FirstKill { zone, .. }
        | Moment::Revenge { zone, .. }
        | Moment::SlainAgain { zone, .. }
        | Moment::Slapped { zone, .. }
        | Moment::FirstEpicItem { zone, .. }
        | Moment::BigUpgrade { zone, .. }
        | Moment::LevelUp { zone, .. } => zone.as_deref(),
        _ => None,
    }
}

/// The breed noun of a mount: the last word of its name, "ram" for "Gray Ram".
fn breed_of(moment: &Moment) -> Option<String> {
    let (Moment::FirstMount { mount, .. } | Moment::FirstEpicMount { mount, .. }) = moment else {
        return None;
    };
    words_of(mount).pop()
}

/// True when a word of the lore is the breed, alone or in the plural: "ram", "rams".
fn names_breed(lore: &str, breed: &str) -> bool {
    words_of(lore)
        .iter()
        .any(|word| word == breed || word.strip_suffix('s') == Some(breed))
}

fn creature_need(creature: Creature) -> Need {
    match creature {
        Creature::Beast => Need::Beast,
        Creature::Undead => Need::Undead,
        Creature::Demon => Need::Demon,
        Creature::Dragonkin => Need::Dragonkin,
        Creature::Elemental => Need::Elemental,
    }
}

fn not_in_lore(field: &str) -> LineFault {
    LineFault::ChoiceNotInLore(field.to_string())
}
