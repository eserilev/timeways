//! The model writes the lore, and the code writes the hero
//! (docs/plans/narrator-templates.md 0). This module turns the answer of the model into
//! the line that the player sees: the parse of the slot JSON, the checks of the lore, the
//! pick of a shape in turn, the render, and the checks of the built line.

use crate::check::{mentions, words_of};
use crate::line_check::{Grounds, LineFault, MOST_SENTENCES, built_faults, lore_faults};
use crate::moments::{Creature, Moment, SlotKind};
use crate::narrator::{MAX_LINE_CHARS, Naming, Who, naming, naming_in};
use crate::narrator_groups::{Group, groups_of, is_strange, order_of};
use crate::narrator_render::{
    Values, count_number, count_words, ordinal, quoted, render, with_article,
};
use crate::narrator_slots::{Choices, KillerKind, Parsed, SlotFault, Tone, parse};
use crate::narrator_templates::{Kind, Need, Number, ShapeInfo, Slot, TEMPLATES, Templates};
use crate::pack::{Deed, Passage};
use timeways_rules::narrator_shapes::{self as rules, Facts, Tier, Token};

/// The main part of the line of an arrival, for the window of the rotation.
pub const PLACE_SHAPE: &str = "f.place";

/// A leader's group is at most this many words: "the Riverpaw".
const MOST_LEADS_WORDS: usize = 4;

/// The longest group name that the budget plans for, before the model names one.
const LEADS_ROOM: &str = "the Defias Brotherhood of Westfall";

/// An arrival may take as many sentences as a line, because the loved Deadmines line has
/// three; a deed takes one.
const ARRIVAL_SENTENCES: usize = MOST_SENTENCES;
const DEED_SENTENCES: usize = 1;

/// The coda of a setup is a sentence of the line, so its lore takes one less.
const SETUP_SENTENCES: usize = MOST_SENTENCES - 1;

/// What one call knows of its moment.
#[derive(Clone, Debug)]
pub struct Setup {
    pub moment: Moment,
    pub who: Who,
    /// The number of the call: it picks the naming and the start of the rotation.
    pub turn: usize,
    /// The main parts of the last accepted lines of the character, oldest first.
    pub recent: Vec<String>,
    /// The foe whose defeat the lore of an arrival sets up. The gate shows a setup only
    /// while that foe lives, so the line ends on a coda that says so (`Kind::Setup`).
    pub setup_foe: Option<String>,
}

impl Setup {
    /// The kind of the moment, and `Kind::Setup` for an arrival with a setup foe.
    #[must_use]
    pub fn kind(&self) -> Option<Kind> {
        let kind = kind_of(&self.moment)?;
        if kind == Kind::Arrival && self.setup_foe.is_some() {
            return Some(Kind::Setup);
        }
        Some(kind)
    }
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

    /// The groups in the words of the prompt, such as "the Silver Hand".
    #[must_use]
    pub fn group_texts(&self) -> Vec<String> {
        self.groups.iter().map(|group| group.text.clone()).collect()
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
    /// The words for the hero in the line: `$N`, "the paladin". None when the line names
    /// nobody, or for a line of free text.
    pub hero: Option<String>,
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
        Moment::NewZone { .. }
        | Moment::FirstCapital { .. }
        | Moment::FirstInstance { .. }
        | Moment::InstanceAgain { .. } => Kind::Arrival,
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

/// The foe whose defeat the lore of an arrival sets up, for the coda of its line. A setup
/// of a quest gets no coda: the game knows no present of it to tell.
#[must_use]
pub fn setup_foe(moment: &Moment, passage: Option<&Passage>) -> Option<String> {
    if !moment.is_arrival() {
        return None;
    }
    match &passage?.setup_for.as_ref()?.deed {
        Deed::Foe(name) => Some(name.clone()),
        Deed::Quest(_) => None,
    }
}

/// The offer of a moment, or None when no shape can fit it whatever the model answers:
/// then the moment is silent before any call. A tenth level needs the race and the class.
#[must_use]
pub fn offer(setup: &Setup) -> Option<Offer> {
    let templates = TEMPLATES.as_ref().ok()?;
    let kind = setup.kind()?;
    let (groups, strange) = match kind {
        Kind::Level | Kind::ClassQuest => match (setup.who.race, setup.who.class) {
            (Some(race), Some(class)) => (
                groups_of(templates, race, class),
                is_strange(templates, race, class),
            ),
            _ => (Vec::new(), false),
        },
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
                let values = plan.widest_values(named);
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
    let most = match offer.kind {
        Kind::Arrival => ARRIVAL_SENTENCES,
        Kind::Setup => SETUP_SENTENCES,
        _ => DEED_SENTENCES,
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
            hero: None,
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
    let values = plan.values(templates, tokens, named, lore);
    Built {
        line: render(templates, tokens, &values),
        shape: templates
            .mains
            .get(usize::from(info.main))
            .cloned()
            .unwrap_or_default(),
        parts: info.parts.clone(),
        named,
        hero: values.hero,
    }
}

/// The values and the facts of one answer.
struct Plan {
    kind: Kind,
    slots: Vec<(Slot, String)>,
    holds: u32,
    hero: Hero,
}

/// What the naming of the hero reads: the moment, the hero, and the turn of the call.
struct Hero {
    moment: Moment,
    who: Who,
    turn: usize,
}

impl Hero {
    /// The naming in a line whose other words are `rest`.
    fn naming_in(&self, rest: &str) -> Naming {
        naming_in(&self.moment, &self.who, self.turn, rest)
    }

    /// Every naming that a line of this hero can take, for the budget.
    fn every_naming(&self) -> Vec<Naming> {
        let mut namings = vec![Naming::Name];
        namings.extend(self.who.race.map(|race| Naming::Kind(race.word())));
        namings.extend(self.who.class.map(|class| Naming::Kind(class.word())));
        namings.extend(self.who.titles.last().cloned().map(Naming::Title));
        namings
    }
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
            if names_group(lore, leads) {
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
        let mut who = setup.who.clone();
        if kind == Kind::Title {
            who.titles.clear();
        }
        let mut plan = Plan {
            kind,
            slots: Vec::new(),
            holds: 0,
            hero: Hero {
                moment: setup.moment.clone(),
                who,
                turn: setup.turn,
            },
        };
        plan.moment_values(templates, &setup.moment);
        if let Some(foe) = &setup.setup_foe {
            plan.slot(Slot::Foe, foe.clone());
        }
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
            | Moment::FirstInstance { .. }
            | Moment::InstanceAgain { .. } => {}
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

    /// The values of a line. A named line takes the naming that no other word of the
    /// line clashes with, so the line renders once without the hero first.
    fn values(&self, templates: &Templates, tokens: &[Token], named: bool, lore: &str) -> Values {
        let mut values = Values {
            lore: lore.to_string(),
            hero: None,
            slots: self.slots.iter().cloned().collect(),
        };
        if named {
            let rest = render(templates, tokens, &values);
            values.hero = hero_words(&self.hero.naming_in(&rest)).or_else(name_words);
        }
        values
    }

    /// The values with the longest naming that a line of the hero can take.
    fn widest_values(&self, named: bool) -> Values {
        let widest = self
            .hero
            .every_naming()
            .iter()
            .filter_map(hero_words)
            .max_by_key(|words| words.chars().count());
        Values {
            lore: String::new(),
            hero: widest.filter(|_| named),
            slots: self.slots.iter().cloned().collect(),
        }
    }

    fn is_named(&self) -> bool {
        let naming = naming(&self.hero.moment, &self.hero.who, self.hero.turn);
        !matches!(naming, Naming::Unnamed | Naming::Absent)
    }
}

fn name_words() -> Option<String> {
    hero_words(&Naming::Name)
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

/// The shape of the line, and whether it names the hero. The order of the choice lives in
/// the rules (`pick_preferring`): a kill on a named turn prefers a fresh part with no hero
/// (3.5), and a turn whose naming fits no shape takes the other naming, so a moment with a
/// lore sentence always has a line.
fn pick_shape(templates: &Templates, plan: &Plan, setup: &Setup) -> Option<(usize, bool)> {
    let shapes = templates.shapes(plan.kind);
    let named = plan.is_named();
    let own = fits_of(templates, shapes, plan, named);
    let other = fits_of(templates, shapes, plan, !named);
    let preferred = if plan.kind == Kind::Kill && named {
        other.clone()
    } else {
        vec![false; shapes.len()]
    };
    let mains: Vec<u16> = shapes.iter().map(|info| info.main).collect();
    let lines: Vec<Option<u16>> = setup
        .recent
        .iter()
        .map(|main| main_id(templates, main))
        .collect();
    let recent = rules::window(&lines);
    let turn = u64::try_from(setup.turn).unwrap_or(u64::MAX);
    let (index, tier) = rules::pick_preferring(&preferred, &own, &other, &mains, &recent, turn)?;
    let names_the_hero = match tier {
        Tier::Preferred => false,
        Tier::Usual => named,
        Tier::Fallback => !named,
    };
    Some((index, names_the_hero))
}

fn fits_of(templates: &Templates, shapes: &[ShapeInfo], plan: &Plan, named: bool) -> Vec<bool> {
    let facts = plan.facts(named);
    shapes
        .iter()
        .map(|info| rules::fits(&templates.table, &info.shape, &facts))
        .collect()
}

/// The id of a main part in the templates. An arrival, or an old main part that the data
/// no longer holds, has none: it still takes its place in the window.
fn main_id(templates: &Templates, main: &str) -> Option<u16> {
    let index = templates.mains.iter().position(|known| known == main)?;
    u16::try_from(index).ok()
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

/// True when `leads` is a name that the history writes, in the same case: "the Riverpaw"
/// or "Riverpaw gnolls". The line takes it as the model wrote it, so a mark, another case,
/// or a run of words that starts with no name never goes in.
fn names_group(lore: &str, leads: &str) -> bool {
    let name = ["the ", "The "]
        .iter()
        .find_map(|the| leads.strip_prefix(the))
        .unwrap_or(leads);
    let plain_words = name.split(' ').all(|word| {
        !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_alphabetic() || c == '\'' || c == '-')
    });
    let starts_a_name = name.chars().next().is_some_and(char::is_uppercase);
    let written = lore_words(lore);
    let wanted = lore_words(name);
    plain_words
        && starts_a_name
        && wanted.len() <= MOST_LEADS_WORDS
        && written.windows(wanted.len()).any(|run| run == wanted)
}

/// The words of a text as written: "Gath'Ilzogg" gives "Gath" and "Ilzogg".
fn lore_words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|word| !word.is_empty())
        .collect()
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
