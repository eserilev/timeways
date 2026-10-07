//! The narrator: one short line at a big moment, within a budget (GAMEPLAY.md 3.2). The
//! words of its prompt live here, and the checks of a line live in `line_check`.

use crate::character::Character;
use crate::house::{HOUSE_RULES, NAME_MARK, fenced, first_chars};
use crate::moments::Moment;
use crate::narrator_build::{self, Offer, Setup};
use crate::narrator_slots::{ChoiceField, fields_of};
use crate::narrator_templates::Kind;
use crate::places::InstanceKind;
use crate::race_class::{Class, Race};
use crate::samples;
use hourglass::Tick;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use timeways_rules::budget::{self as rules, LINES_PER_HOUR};

/// About 50 words. The prompt asks for 30.
pub const MAX_LINE_CHARS: usize = 300;

/// The limit of the bridge for a narrator line (Gnomish Relay SPEC.md 9.8).
pub const MAX_LINE_BYTES: usize = 1000;

/// The lore of a moment keeps at most this many characters: about 100 tokens.
pub const MAX_LORE_CHARS: usize = 400;

/// The persona of the narrator: its manner only, never lore (GAMEPLAY.md 3.2.1). The
/// chronicle shares it, and an NPC never gets it.
pub const PERSONA: &str = include_str!("../data/narrator.txt");

/// The task of a moment where the hero only arrived: the line tells the place alone.
const PLACE_TASK: &str = "Tell the moment below in one or two sentences, at most 30 words. \
Give a piece of the history of its place, from the lore. The place is the subject of the \
line, and the hero is not in it.";

const DEED_TASK: &str = "Tell the moment below in one or two sentences, at most 30 words. \
First give a piece of the history of its place, its foe, or its people, from the lore. Then \
tell what the hero did, in few words. Keep the place, the foe, or the people the subject \
where you can. $N stands for the name of the hero: the game puts the name there.";

/// The author's note of a place moment. A model weighs the end of a prompt most.
const PLACE_NOTE: &str = "\
Remember: the line is about the place alone. Never tell that the hero came, entered, or \
arrived. Take the history from the lore, and add nothing. End on what holds in the place \
now. When the lore gives you nothing true to tell, answer SILENCE.
Answer with the line only.";

const DEED_NOTE: &str = "\
Remember: the history first, then the deed. Take both from the moment and the lore, and \
add nothing. Name the hero only as the line above says, and at most once. When the deed \
reads well without the hero, say what changed and leave the hero out. When the moment and \
the lore give you nothing true to tell, answer SILENCE.
Answer with the line only.";

/// Counts the lines of the last hour of game time, so the narrator talks little. The rule
/// lives in timeways-rules, where Lean proves it (lean/README.md).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Budget {
    /// The times of the last lines, oldest first.
    spoken: [Option<Tick>; LINES_PER_HOUR],
}

impl Budget {
    /// True when the narrator has a line left at `at`. That line then counts.
    pub fn take(&mut self, at: Tick) -> bool {
        let mut rules = rules::Budget {
            spoken: self.spoken.map(|line| line.map(|line| line.0)),
        };
        let taken = rules.take(at.0);
        self.spoken = rules.spoken.map(|line| line.map(Tick));
        taken
    }
}

/// What the narrator knows of the hero. Never the name: a line writes `$N` for it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Who {
    pub race: Option<Race>,
    pub class: Option<Class>,
    /// The earned titles, oldest first.
    pub titles: Vec<String>,
}

impl Who {
    #[must_use]
    pub fn of(character: &Character) -> Who {
        Who {
            race: character.race(),
            class: character.class(),
            titles: character.titles().into_iter().map(str::to_string).collect(),
        }
    }

    /// "a Forsaken warlock", "an orc", or "a mage". None before the addon tells them.
    #[must_use]
    pub fn described(&self) -> Option<String> {
        let words: Vec<&str> = [self.race.map(Race::word), self.class.map(Class::word)]
            .into_iter()
            .flatten()
            .collect();
        let first = words.first()?;
        let article = if first.starts_with(['a', 'e', 'i', 'o', 'u']) {
            "an"
        } else {
            "a"
        };
        Some(format!("{article} {}", words.join(" ")))
    }
}

/// How one line names the hero (GAMEPLAY.md 3.2.1). A place moment leaves the hero out. A
/// deed takes its naming in turn, so the lines mix the name, the race or the class, no
/// name, and now and then a title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Naming {
    /// `$N`, which the addon swaps for the name.
    Name,
    /// A race or a class: "the Forsaken", "the paladin".
    Kind(&'static str),
    Title(String),
    /// The line tells the deed, but names nobody.
    Unnamed,
    /// The hero only arrived, so the line is about the place alone.
    Absent,
}

impl Naming {
    /// The words after "Name the hero:", as the samples write them.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Naming::Name => NAME_MARK.to_string(),
            Naming::Kind(word) => format!("the {word}"),
            Naming::Title(title) => format!("by the title \"{title}\""),
            Naming::Unnamed => "no name".to_string(),
            Naming::Absent => ABSENT.to_string(),
        }
    }

    /// The words that stand for the hero in this line, past `$N`: the race and the class
    /// of the hero, and the title of the naming. The check of an arrival reads them.
    #[must_use]
    pub fn hero_words(&self, who: &Who) -> Vec<String> {
        let kinds = [who.race.map(Race::word), who.class.map(Class::word)];
        let mut words: Vec<String> = kinds.into_iter().flatten().map(str::to_string).collect();
        if let Naming::Title(title) = self {
            words.push(title.clone());
        }
        words
    }
}

/// The naming of a place moment, as the prompt and the samples write it.
pub const ABSENT: &str = "not at all, the line is about the place";

#[derive(Clone, Copy)]
enum Turn {
    Name,
    Class,
    Race,
    Title,
    Unnamed,
}

/// Three names, two kinds, two lines with no name, and one title in each 8 lines.
const ROTATION: [Turn; 8] = [
    Turn::Name,
    Turn::Class,
    Turn::Unnamed,
    Turn::Name,
    Turn::Race,
    Turn::Name,
    Turn::Unnamed,
    Turn::Title,
];

/// A place moment leaves the hero out. A deed takes the naming of its turn, and a kind or
/// a title that the hero lacks gives the name.
#[must_use]
pub fn naming(moment: &Moment, who: &Who, turn: usize) -> Naming {
    if moment.is_arrival() {
        return Naming::Absent;
    }
    let race = who.race.map(|race| Naming::Kind(race.word()));
    let class = who.class.map(|class| Naming::Kind(class.word()));
    let named = match ROTATION[turn % ROTATION.len()] {
        Turn::Name => None,
        Turn::Unnamed => Some(Naming::Unnamed),
        Turn::Class => class.or(race),
        Turn::Race => race.or(class),
        Turn::Title => who.titles.last().cloned().map(Naming::Title),
    };
    named.unwrap_or(Naming::Name)
}

/// One moment to tell, with its lore and what the narrator knows of the hero.
#[derive(Clone, Copy, Debug)]
pub struct Telling<'a> {
    pub moment: &'a Moment,
    /// A short passage (`lore_excerpt`) about the place, the foe, or the people.
    pub lore: Option<&'a str>,
    pub who: &'a Who,
}

/// The task of an arrival with templates: the history of the place, which is the whole line.
const PLACE_LORE_TASK: &str = "Write one or two sentences of history about the place of the \
moment below, from the lore. The place is the subject, and the hero is not in it. End on \
what holds in the place now.";

/// The task of a deed with templates. The code adds the deed, so the model never names the
/// hero (docs/plans/narrator-templates.md 8.2).
const DEED_LORE_TASK: &str = "Write one sentence of history about the moment below, from \
the lore: its place, its foe, its people, or its order. Do not tell the deed, and do not \
name the hero: the game adds both after your sentence. End on what holds now.";

/// The task of a tenth level with templates (docs/plans/level-lines.md).
const LEVEL_LORE_TASK: &str = "Write one sentence of history about one of the groups below, \
from the lore. Tell of the group as a whole. The game adds after your sentence that the \
group grows stronger, and the level of the hero, so leave both out, and never name the \
hero. End on what holds now.";

const LORE_NOTE: &str = "\
Remember: history only, from the lore. Add nothing. Never name the hero, and never tell \
the deed. When the lore gives you nothing true to tell, answer {\"lore\": \"SILENCE\"}.
Answer with the JSON only.";

/// A word is about 6 characters, with its space.
const CHARS_PER_WORD: usize = 6;

/// The prompt of the moment. A moment with templates asks for the slot JSON; a flavor
/// moment keeps a line of free text.
#[must_use]
pub fn prompt(telling: &Telling<'_>, turn: usize) -> String {
    let setup = Setup {
        moment: telling.moment.clone(),
        who: telling.who.clone(),
        turn,
        recent: Vec::new(),
    };
    match narrator_build::offer(&setup) {
        Some(offer) => lore_prompt(telling, turn, &offer),
        None => line_prompt(telling, turn),
    }
}

/// The prompt that asks for the history and the choices as JSON
/// (docs/plans/narrator-templates.md 8.2).
#[must_use]
pub fn lore_prompt(telling: &Telling<'_>, turn: usize, offer: &Offer) -> String {
    let task = match offer.kind {
        Kind::Arrival => PLACE_LORE_TASK,
        Kind::Level => LEVEL_LORE_TASK,
        _ => DEED_LORE_TASK,
    };
    let words = (offer.budget / CHARS_PER_WORD).max(1);
    let mut prompt = format!("{PERSONA}\n{HOUSE_RULES}\n\n{task} Use at most {words} words.");
    let what = what_happened(telling.moment);
    let samples = samples::line_section(turn, &what);
    let _ = write!(prompt, "\n\n{samples}\n\nThe moment:\n{}", fenced(&what));
    if matches!(offer.kind, Kind::Level | Kind::ClassQuest)
        && let Some(who) = telling.who.described()
    {
        let _ = write!(prompt, "\n\nThe hero: {who}");
        if offer.strange {
            let _ = write!(
                prompt,
                "\nThe lore finds {who} strange. Tell the group as it is, and never as if \
                 the hero fought their own faction."
            );
        }
    }
    match telling.lore {
        Some(lore) => {
            let _ = write!(prompt, "\n\nThe lore:\n{}", fenced(lore));
        }
        None => prompt.push_str("\n\nThe lore: none"),
    }
    let choices = choice_lines(offer);
    if !choices.is_empty() {
        let _ = write!(prompt, "\n\nYour choices:\n{}", choices.join("\n"));
    }
    let _ = write!(
        prompt,
        "\n\nAnswer with JSON only, in this form:\n{}\n\n{LORE_NOTE}",
        answer_form(offer)
    );
    prompt
}

/// One line of meaning for each closed field of the moment (2.2).
fn choice_lines(offer: &Offer) -> Vec<String> {
    let mut lines = Vec::new();
    for field in fields_of(offer.kind) {
        let line = match field {
            ChoiceField::Group => {
                let groups: Vec<String> = offer
                    .groups
                    .iter()
                    .map(|group| format!("\"{}\" for {}", group.id, group.text))
                    .collect();
                format!(
                    "- \"group\": the group that your history tells of, and names. One of: {}.",
                    groups.join(", ")
                )
            }
            ChoiceField::There => "- \"there\": true when your history names the zone of \
                the moment, where it happened. Else false."
                .to_string(),
            ChoiceField::Leads => "- \"leads\": the group that the foe led, in the words of \
                your history, such as \"the Riverpaw\". Else \"none\"."
                .to_string(),
            ChoiceField::LeadsNumber => "- \"leads_number\": \"one\" when that group is one \
                body, such as \"the Brotherhood\", and \"many\" when it is many, such as \
                \"the Riverpaw\"."
                .to_string(),
            ChoiceField::Tone => "- \"tone\": \"plain\", or \"dry\" for a dry edge.".to_string(),
            ChoiceField::Killer => "- \"killer\": \"one\" when the killer is one named \
                person, and \"kind\" when it is one of many of its kind."
                .to_string(),
            ChoiceField::Breed => "- \"breed\": true when your history tells of the breed \
                of the mount, such as its rams or its wolves. Else false."
                .to_string(),
        };
        lines.push(line);
    }
    lines
}

/// The JSON form of an answer, with the first value of each list.
fn answer_form(offer: &Offer) -> String {
    let mut fields = vec!["\"lore\": \"<your history>\"".to_string()];
    for field in fields_of(offer.kind) {
        let value = match field {
            ChoiceField::Group => offer
                .groups
                .first()
                .map_or_else(|| "\"\"".to_string(), |group| format!("\"{}\"", group.id)),
            ChoiceField::There | ChoiceField::Breed => "false".to_string(),
            ChoiceField::Leads => "\"none\"".to_string(),
            ChoiceField::LeadsNumber | ChoiceField::Killer => "\"one\"".to_string(),
            ChoiceField::Tone => "\"plain\"".to_string(),
        };
        fields.push(format!("\"{}\": {value}", field.name()));
    }
    format!("{{{}}}", fields.join(", "))
}

/// The prompt of a line of free text, for a flavor moment. `turn` picks the golden samples
/// and the naming of the prompt. The hero sheet stays out: a line is about one moment, and
/// models pulled the sheet into every line (GAMEPLAY.md 3.2.1).
#[must_use]
pub fn line_prompt(telling: &Telling<'_>, turn: usize) -> String {
    let arrival = telling.moment.is_arrival();
    let task = if arrival { PLACE_TASK } else { DEED_TASK };
    let mut prompt = format!("{PERSONA}\n{HOUSE_RULES}\n\n{task}");
    let what = what_happened(telling.moment);
    let samples = samples::line_section(turn, &what);
    let _ = write!(prompt, "\n\n{samples}\n\nThe moment:\n{}", fenced(&what));
    if let Some(who) = telling.who.described() {
        let _ = write!(prompt, "\n\nThe hero: {who}");
    }
    match telling.lore {
        Some(lore) => {
            let _ = write!(prompt, "\n\nThe lore:\n{}", fenced(lore));
        }
        None => prompt.push_str("\n\nThe lore: none"),
    }
    let naming = naming(telling.moment, telling.who, turn).label();
    let note = if arrival { PLACE_NOTE } else { DEED_NOTE };
    let _ = write!(prompt, "\n\nName the hero: {naming}\n\n{note}");
    prompt
}

/// The start of a lore passage, cut after a sentence when it is long.
#[must_use]
pub fn lore_excerpt(text: &str) -> String {
    let head = first_chars(text.trim(), MAX_LORE_CHARS);
    if head.len() == text.trim().len() {
        return head.to_string();
    }
    let sentence_end = head
        .rmatch_indices(". ")
        .map(|(at, _)| at + 1)
        .next()
        .or_else(|| head.rfind(' '));
    sentence_end.map_or(head, |end| &head[..end]).to_string()
}

/// The moment in plain words, as the prompt gives it. It says "first" with no stock
/// phrase, because a line takes the words of its moment.
#[must_use]
pub fn what_happened(moment: &Moment) -> String {
    match moment {
        Moment::Flavor { what, .. } => what.clone(),
        Moment::Titled { title } => {
            format!("The player earned the title \"{title}\" in their journal.")
        }
        Moment::FirstKill { foe, zone, .. } => format!(
            "The player defeated {foe}{}. They never had before.",
            in_zone(zone.as_deref())
        ),
        Moment::Revenge { foe, deaths, zone } => format!(
            "The player defeated {foe}{}. {foe} had killed them {deaths} times before.",
            in_zone(zone.as_deref())
        ),
        Moment::SlainAgain {
            killer,
            times,
            zone,
        } => format!(
            "{killer} killed the player again{}. That makes {times} times.",
            in_zone(zone.as_deref())
        ),
        Moment::Slapped { npc, times, zone } => format!(
            "The player slapped {npc}{}. That makes {times} times.",
            in_zone(zone.as_deref())
        ),
        Moment::QuestDone { title, giver } => format!(
            "The player finished the quest \"{title}\"{}.",
            giver
                .as_deref()
                .map(|giver| format!(" for {giver}"))
                .unwrap_or_default()
        ),
        Moment::LevelUp { level, zone: None } => format!("The player reached level {level}."),
        Moment::LevelUp {
            level,
            zone: Some(zone),
        } => format!("The player reached level {level} in {zone}."),
        Moment::NewZone { zone } => {
            format!("The player arrived in {zone}. They had never been there before.")
        }
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Dungeon,
        } => format!("The player entered the dungeon {zone}. They had never been inside before."),
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Raid,
        } => format!("The player entered the raid {zone}. They had never been inside before."),
        Moment::FirstInstance {
            zone,
            kind: InstanceKind::Battleground,
        } => format!(
            "The player entered the battleground {zone}. They had never fought there before."
        ),
        Moment::QuestMarked { mark, quest } => {
            format!(
                "During the quest \"{quest}\", a lasting effect came on the player: \"{mark}\"."
            )
        }
        Moment::FirstCapital { city } => format!(
            "The player arrived in the capital city {city}. They had never been there before."
        ),
        Moment::ClassQuestDone { title } => {
            format!("The player finished \"{title}\", a quest of their class.")
        }
        Moment::FirstMount { mount, .. } => {
            format!("The player rode a mount of their own, {mount}. They never had one before.")
        }
        Moment::FirstEpicMount { mount, .. } => format!(
            "The player rode {mount}, a swift mount, twice as fast as a runner. They never had \
             one so fast before."
        ),
        Moment::FirstEpicItem { item, zone, .. } => format!(
            "The player put on {item}{}. It is of the finest kind of item, and they never \
             wore one before.",
            in_zone(zone.as_deref())
        ),
        Moment::BigUpgrade { item, zone, .. } => format!(
            "The player put on {item}{}. It is far better than the item it replaced.",
            in_zone(zone.as_deref())
        ),
    }
}

fn in_zone(zone: Option<&str>) -> String {
    zone.map(|zone| format!(" in {zone}")).unwrap_or_default()
}
