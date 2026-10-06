//! The template parts of a narrator line, from `data/narrator_templates.toml`
//! (docs/plans/narrator-templates.md 3 and 4). The loader turns each part into tokens,
//! builds every shape of each moment, and runs the checks of the rules. A table that
//! fails them makes the narrator silent, and the test `the_templates_load` fails first.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use thiserror::Error;
use timeways_rules::narrator_shapes::{self as rules, PartKind, Token};

const BUNDLED: &str = include_str!("../data/narrator_templates.toml");

/// The words that never stand right before the hero (docs/plans/narrator-templates.md 5.2).
const INSIDE_WORDS: [&str; 5] = ["in", "inside", "within", "through", "into"];

/// The marks that stand after a word with no space.
pub const MARKS: [&str; 4] = [".", ",", ":", ";"];

/// The templates, loaded once.
pub static TEMPLATES: LazyLock<Result<Templates, TemplateError>> =
    LazyLock::new(|| Templates::load(BUNDLED));

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("the templates are no valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("part {part}: {reason}")]
    Part { part: String, reason: String },
    #[error("the shapes of the moment {0:?} fail the checks of the rules")]
    Table(Kind),
}

/// The kind of a narrator moment, as the templates see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Arrival,
    Kill,
    Revenge,
    Death,
    SideQuest,
    ClassQuest,
    Title,
    Slap,
    Mark,
    Mount,
    EpicMount,
    Item,
    Level,
}

/// Every kind that has shapes. An arrival is the lore alone.
pub const DEED_KINDS: [Kind; 12] = [
    Kind::Kill,
    Kind::Revenge,
    Kind::Death,
    Kind::SideQuest,
    Kind::ClassQuest,
    Kind::Title,
    Kind::Slap,
    Kind::Mark,
    Kind::Mount,
    Kind::EpicMount,
    Kind::Item,
    Kind::Level,
];

impl Kind {
    fn from_name(name: &str) -> Option<Kind> {
        let kind = match name {
            "kill" => Kind::Kill,
            "revenge" => Kind::Revenge,
            "death" => Kind::Death,
            "side_quest" => Kind::SideQuest,
            "class_quest" => Kind::ClassQuest,
            "title" => Kind::Title,
            "slap" => Kind::Slap,
            "mark" => Kind::Mark,
            "mount" => Kind::Mount,
            "epic_mount" => Kind::EpicMount,
            "item" => Kind::Item,
            "level" => Kind::Level,
            _ => return None,
        };
        Some(kind)
    }
}

/// A value of a moment that a part can hold, past the lore and the hero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Foe,
    Killer,
    Zone,
    Item,
    /// A mount with its article: "a Gray Ram".
    MountA,
    Breed,
    Group,
    Order,
    Count,
    CountNum,
    Ordinal,
    Level,
    Quest,
    Title,
    Npc,
    Mark,
    /// The group that the foe led: "the Riverpaw".
    Led,
}

const SLOTS: [(&str, Slot); 17] = [
    ("foe", Slot::Foe),
    ("killer", Slot::Killer),
    ("zone", Slot::Zone),
    ("item", Slot::Item),
    ("mount_a", Slot::MountA),
    ("breed", Slot::Breed),
    ("group", Slot::Group),
    ("order", Slot::Order),
    ("count", Slot::Count),
    ("count_num", Slot::CountNum),
    ("ordinal", Slot::Ordinal),
    ("level", Slot::Level),
    ("quest", Slot::Quest),
    ("title", Slot::Title),
    ("npc", Slot::Npc),
    ("mark", Slot::Mark),
    ("led", Slot::Led),
];

impl Slot {
    /// The id of the slot in the tokens of the rules.
    #[must_use]
    pub fn id(self) -> u8 {
        let index = SLOTS.iter().position(|(_, slot)| *slot == self);
        index
            .and_then(|index| u8::try_from(index).ok())
            .unwrap_or(u8::MAX)
    }

    #[must_use]
    pub fn of_id(id: u8) -> Option<Slot> {
        SLOTS.get(usize::from(id)).map(|(_, slot)| *slot)
    }

    fn from_name(name: &str) -> Option<Slot> {
        let name = name.to_lowercase();
        SLOTS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, slot)| *slot)
    }

    /// The number of slots, for the list of values of the rules.
    pub const COUNT: usize = SLOTS.len();
}

/// A fact that a part requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Need {
    /// The lore names the place where the deed came.
    There,
    /// The class of the hero has a named order that the lore names.
    Order,
    /// The model chose the dry tone.
    Dry,
    Beast,
    Undead,
    Demon,
    Dragonkin,
    Elemental,
    Weapon,
    Worn,
    /// The lore names a group that the foe led.
    Leads,
    /// The lore names the breed of the mount.
    Breed,
    /// A count of two or more.
    Repeat,
    /// The group is one: "the Brotherhood".
    One,
    /// The group is many: "the Riverpaw".
    Many,
}

const NEEDS: [(&str, Need); 15] = [
    ("there", Need::There),
    ("order", Need::Order),
    ("dry", Need::Dry),
    ("beast", Need::Beast),
    ("undead", Need::Undead),
    ("demon", Need::Demon),
    ("dragonkin", Need::Dragonkin),
    ("elemental", Need::Elemental),
    ("weapon", Need::Weapon),
    ("worn", Need::Worn),
    ("leads", Need::Leads),
    ("breed", Need::Breed),
    ("repeat", Need::Repeat),
    ("one", Need::One),
    ("many", Need::Many),
];

impl Need {
    /// The bit of the need in the bit set of the rules.
    #[must_use]
    pub fn bit(self) -> u32 {
        let index = NEEDS.iter().position(|(_, need)| *need == self);
        index
            .and_then(|index| u32::try_from(index).ok())
            .map_or(0, |index| 1 << index)
    }

    fn from_name(name: &str) -> Option<Need> {
        NEEDS
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, need)| *need)
    }
}

/// The number of a group, for its grow verb.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Number {
    One,
    Many,
}

impl Number {
    #[must_use]
    pub fn need(self) -> Need {
        match self {
            Number::One => Need::One,
            Number::Many => Need::Many,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    /// Names that take "a" before a vowel: "a Unicorn".
    article_a: Vec<String>,
    connective: Vec<ConnectiveData>,
    deed: Vec<DeedData>,
    killer: Vec<KillerData>,
    grow: Vec<GrowData>,
    coda: Vec<CodaData>,
    people: Vec<PeopleData>,
    class: Vec<ClassData>,
    group: Vec<GroupData>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectiveData {
    id: String,
    text: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    needs: Vec<String>,
    moments: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeedData {
    id: String,
    moments: Vec<String>,
    text: Option<String>,
    one: Option<String>,
    many: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    needs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KillerData {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowData {
    id: String,
    one: String,
    many: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CodaData {
    id: String,
    alone: String,
    joined: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// A people of Classic, by the word of its race.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeopleData {
    pub race: String,
    pub text: String,
    pub number: Number,
    pub plural: String,
    pub faction: String,
    pub anchors: Vec<String>,
}

/// A class of Classic, with the races that Classic gave it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClassData {
    pub class: String,
    pub plural: String,
    pub races: Vec<String>,
}

/// A named order, or the phrase of a strange pairing.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupData {
    pub id: String,
    pub text: String,
    pub short: Option<String>,
    pub number: Number,
    pub anchors: Vec<String>,
    pub classes: Vec<String>,
    #[serde(default)]
    pub factions: Vec<String>,
    #[serde(default)]
    pub races: Vec<String>,
}

/// One shape of a moment: its parts in the rules, its main part, and the ids of its data
/// parts, for the review and the log.
#[derive(Clone, Debug)]
pub struct ShapeInfo {
    pub shape: rules::Shape,
    /// The index of the main part in `Templates::mains`.
    pub main: u16,
    pub parts: Vec<String>,
}

/// The loaded templates.
#[derive(Debug)]
pub struct Templates {
    words: Vec<String>,
    pub table: rules::Table,
    shapes: HashMap<Kind, Vec<ShapeInfo>>,
    /// The data id of each main part: "k.fell", "v.stronger".
    pub mains: Vec<String>,
    pub killers: Vec<(String, String)>,
    pub peoples: Vec<PeopleData>,
    pub classes: Vec<ClassData>,
    pub groups: Vec<GroupData>,
    pub article_a: Vec<String>,
}

/// The loader: words and parts so far, with each part built once.
struct Builder {
    words: Vec<String>,
    tags: Vec<String>,
    parts: Vec<rules::Part>,
    /// The key of each part: its data id, and its number or form.
    part_keys: Vec<String>,
    part_ids: HashMap<String, usize>,
    mains: Vec<String>,
}

impl Templates {
    /// Parses the data, builds each shape of each moment, and runs `table_ok` of the rules
    /// on the shapes of each moment.
    ///
    /// # Errors
    ///
    /// Returns the first fault of the data.
    pub fn load(text: &str) -> Result<Templates, TemplateError> {
        let data: Data = toml::from_str(text)?;
        let mut builder = Builder::new();
        let mut shapes = HashMap::new();
        for kind in DEED_KINDS {
            let built = match kind {
                Kind::Revenge => builder.revenge_shapes(&data)?,
                Kind::Level => builder.level_shapes(&data)?,
                _ => builder.deed_shapes(&data, kind)?,
            };
            shapes.insert(kind, built);
        }
        let table = rules::Table {
            parts: builder.parts,
            inside_words: INSIDE_WORDS
                .iter()
                .filter_map(|word| builder.words.iter().position(|known| known == word))
                .filter_map(|index| u16::try_from(index).ok())
                .collect(),
        };
        for kind in DEED_KINDS {
            let of_kind: Vec<rules::Shape> = shapes
                .get(&kind)
                .map(|all: &Vec<ShapeInfo>| all.iter().map(|info| info.shape.clone()).collect())
                .unwrap_or_default();
            if !rules::table_ok(&table, &of_kind) {
                return Err(TemplateError::Table(kind));
            }
        }
        Ok(Templates {
            words: builder.words,
            table,
            shapes,
            mains: builder.mains,
            killers: data
                .killer
                .into_iter()
                .map(|killer| (killer.id, killer.text))
                .collect(),
            peoples: data.people,
            classes: data.class,
            groups: data.group,
            article_a: data.article_a,
        })
    }

    /// The shapes of a moment, in the fixed order of the data.
    #[must_use]
    pub fn shapes(&self, kind: Kind) -> &[ShapeInfo] {
        self.shapes.get(&kind).map_or(&[], Vec::as_slice)
    }

    /// The word of an id.
    #[must_use]
    pub fn word(&self, id: u16) -> &str {
        self.words.get(usize::from(id)).map_or("", String::as_str)
    }

    /// The text of the killer phrase with this id, or the name alone.
    #[must_use]
    pub fn killer_phrase(&self, id: &str, killer: &str) -> String {
        let phrase = self
            .killers
            .iter()
            .find(|(known, _)| known == id)
            .map_or("{killer}", |(_, text)| text.as_str());
        phrase.replace("{killer}", killer)
    }
}

impl Builder {
    fn new() -> Builder {
        Builder {
            words: Vec::new(),
            tags: Vec::new(),
            parts: Vec::new(),
            part_keys: Vec::new(),
            part_ids: HashMap::new(),
            mains: Vec::new(),
        }
    }

    fn main(&mut self, id: &str) -> u16 {
        let index = self
            .mains
            .iter()
            .position(|known| known == id)
            .unwrap_or_else(|| {
                self.mains.push(id.to_string());
                self.mains.len() - 1
            });
        u16::try_from(index).unwrap_or(u16::MAX)
    }

    fn word_id(&mut self, word: &str) -> Result<u16, TemplateError> {
        let index = self
            .words
            .iter()
            .position(|known| known == word)
            .unwrap_or_else(|| {
                self.words.push(word.to_string());
                self.words.len() - 1
            });
        u16::try_from(index).map_err(|_| fault("words", "more than 65535 words"))
    }

    /// The tokens of a text: words, marks, and slots in braces.
    /// The first word of a part is a word of the middle of a sentence, so "That makes"
    /// after "There," renders "There, that makes". The render puts the capital back at the
    /// start of a sentence.
    fn tokens(&mut self, part: &str, text: &str) -> Result<Vec<Token>, TemplateError> {
        let mut tokens = Vec::new();
        for piece in text.split_whitespace() {
            let (body, mark) = split_mark(piece);
            if let Some(name) = body
                .strip_prefix('{')
                .and_then(|rest| rest.strip_suffix('}'))
            {
                tokens.push(slot_token(part, name)?);
            } else if !body.is_empty() {
                let word = if tokens.is_empty() {
                    body.to_lowercase()
                } else {
                    body.to_string()
                };
                tokens.push(Token::Word(self.word_id(&word)?));
            }
            if let Some(mark) = mark {
                tokens.push(Token::Word(self.word_id(mark)?));
            }
        }
        Ok(tokens)
    }

    fn bits_of_tags(&mut self, part: &str, tags: &[String]) -> Result<u32, TemplateError> {
        let mut bits = 0;
        for tag in tags {
            let index = self
                .tags
                .iter()
                .position(|known| known == tag)
                .unwrap_or_else(|| {
                    self.tags.push(tag.clone());
                    self.tags.len() - 1
                });
            let shift = u32::try_from(index)
                .ok()
                .filter(|shift| *shift < u32::BITS)
                .ok_or_else(|| fault(part, "more than 32 tags"))?;
            bits |= 1 << shift;
        }
        Ok(bits)
    }

    /// The part with this key, built once.
    fn part(
        &mut self,
        key: &str,
        kind: PartKind,
        text: &str,
        tags: &[String],
        needs: u32,
    ) -> Result<usize, TemplateError> {
        if let Some(index) = self.part_ids.get(key) {
            return Ok(*index);
        }
        let part = rules::Part {
            kind,
            tokens: self.tokens(key, text)?,
            tags: self.bits_of_tags(key, tags)?,
            needs,
        };
        self.parts.push(part);
        self.part_keys.push(key.to_string());
        let index = self.parts.len() - 1;
        self.part_ids.insert(key.to_string(), index);
        Ok(index)
    }

    fn connectives_of(&mut self, data: &Data, kind: Kind) -> Result<Vec<usize>, TemplateError> {
        let mut found = Vec::new();
        for connective in &data.connective {
            if !of_kind(&connective.moments, kind, &connective.id)? {
                continue;
            }
            let needs = need_bits(&connective.id, &connective.needs)?;
            let part = self.part(
                &connective.id,
                PartKind::Connective,
                &connective.text,
                &connective.tags,
                needs,
            )?;
            found.push(part);
        }
        Ok(found)
    }

    /// The deed parts of a moment: one for a deed with one text, and one for each number
    /// of a deed with a text for one and for many. Each with the id of its data part.
    fn deeds_of(&mut self, data: &Data, kind: Kind) -> Result<Vec<(usize, String)>, TemplateError> {
        let mut found = Vec::new();
        for deed in &data.deed {
            if !of_kind(&deed.moments, kind, &deed.id)? {
                continue;
            }
            let needs = need_bits(&deed.id, &deed.needs)?;
            for (variant, text, number) in deed_variants(deed)? {
                let key = format!("{}{variant}", deed.id);
                let needs = needs | number.map_or(0, |number| number.need().bit());
                let part = self.part(&key, PartKind::Deed, text, &deed.tags, needs)?;
                found.push((part, deed.id.clone()));
            }
        }
        Ok(found)
    }

    /// Each deed alone, then each deed after each connective.
    fn deed_shapes(&mut self, data: &Data, kind: Kind) -> Result<Vec<ShapeInfo>, TemplateError> {
        let connectives = self.connectives_of(data, kind)?;
        let deeds = self.deeds_of(data, kind)?;
        let mut shapes = Vec::new();
        for (deed, id) in &deeds {
            let main = self.main(id);
            shapes.push(shape_info(vec![*deed], main, &self.keys_of(&[*deed])));
            for connective in &connectives {
                let parts = vec![*connective, *deed];
                shapes.push(shape_info(parts.clone(), main, &self.keys_of(&parts)));
            }
        }
        Ok(shapes)
    }

    /// A before-clause, then a turn-clause, which is the main part.
    fn revenge_shapes(&mut self, data: &Data) -> Result<Vec<ShapeInfo>, TemplateError> {
        let turns = self.deeds_of(data, Kind::Revenge)?;
        let befores = self.deeds_of_name(data, "revenge_before")?;
        let mut shapes = Vec::new();
        for (before, _) in &befores {
            for (turn, turn_id) in &turns {
                let main = self.main(turn_id);
                let parts = vec![*before, *turn];
                shapes.push(shape_info(parts.clone(), main, &self.keys_of(&parts)));
            }
        }
        Ok(shapes)
    }

    fn deeds_of_name(
        &mut self,
        data: &Data,
        moment: &str,
    ) -> Result<Vec<(usize, String)>, TemplateError> {
        let mut found = Vec::new();
        for deed in &data.deed {
            if !deed.moments.iter().any(|known| known == moment) {
                continue;
            }
            let needs = need_bits(&deed.id, &deed.needs)?;
            for (variant, text, _) in deed_variants(deed)? {
                let key = format!("{}{variant}", deed.id);
                let part = self.part(&key, PartKind::Deed, text, &deed.tags, needs)?;
                found.push((part, deed.id.clone()));
            }
        }
        Ok(found)
    }

    /// `f.level`: [connective] group grow. [coda]. `f.level_joined`: [connective] group
    /// grow, and coda. The grow verb is the main part.
    fn level_shapes(&mut self, data: &Data) -> Result<Vec<ShapeInfo>, TemplateError> {
        let connectives = self.connectives_of(data, Kind::Level)?;
        let group = self.part("g.group", PartKind::Group, "{group}", &[], 0)?;
        let mut openers: Vec<Option<usize>> = vec![None];
        openers.extend(connectives.into_iter().map(Some));
        let mut shapes = Vec::new();
        for grow in &data.grow {
            let main = self.main(&grow.id);
            for number in [Number::One, Number::Many] {
                let verb = match number {
                    Number::One => &grow.one,
                    Number::Many => &grow.many,
                };
                let needs = number.need().bit();
                let closed_key = format!("{}.{number:?}.closed", grow.id);
                let closed = format!("{verb}.");
                let closed = self.part(&closed_key, PartKind::Grow, &closed, &grow.tags, needs)?;
                let open_key = format!("{}.{number:?}.open", grow.id);
                let open = self.part(&open_key, PartKind::Grow, verb, &grow.tags, needs)?;
                for opener in &openers {
                    let start: Vec<usize> = opener.iter().copied().chain([group]).collect();
                    let mut alone = start.clone();
                    alone.push(closed);
                    shapes.push(shape_info(alone.clone(), main, &self.keys_of(&alone)));
                    for coda in &data.coda {
                        let coda_part =
                            self.part(&coda.id, PartKind::Coda, &coda.alone, &coda.tags, 0)?;
                        let mut with_coda = alone.clone();
                        with_coda.push(coda_part);
                        let names = self.keys_of(&with_coda);
                        shapes.push(shape_info(with_coda, main, &names));
                        let Some(joined) = &coda.joined else {
                            continue;
                        };
                        let joined_key = format!("{}.joined", coda.id);
                        let joined_text = format!(", and {joined}");
                        let joined =
                            self.part(&joined_key, PartKind::Coda, &joined_text, &coda.tags, 0)?;
                        let mut together = start.clone();
                        together.extend([open, joined]);
                        let names = self.keys_of(&together);
                        shapes.push(shape_info(together, main, &names));
                    }
                }
            }
        }
        Ok(shapes)
    }

    /// The keys of the parts of a shape, for the review and the log.
    fn keys_of(&self, parts: &[usize]) -> Vec<String> {
        parts
            .iter()
            .filter_map(|part| self.part_keys.get(*part).cloned())
            .collect()
    }
}

fn shape_info(parts: Vec<usize>, main: u16, names: &[String]) -> ShapeInfo {
    ShapeInfo {
        shape: rules::Shape { parts },
        main,
        parts: names.to_vec(),
    }
}

/// The key suffix, the text, and the number of one text of a deed.
type Variant<'a> = (&'static str, &'a str, Option<Number>);

/// The texts of a deed: ("", text, None), or one for each number.
fn deed_variants(deed: &DeedData) -> Result<Vec<Variant<'_>>, TemplateError> {
    match (&deed.text, &deed.one, &deed.many) {
        (Some(text), None, None) => Ok(vec![("", text.as_str(), None)]),
        (None, Some(one), Some(many)) => Ok(vec![
            (".one", one.as_str(), Some(Number::One)),
            (".many", many.as_str(), Some(Number::Many)),
        ]),
        _ => Err(fault(
            &deed.id,
            "needs a text, or a text for one and for many",
        )),
    }
}

fn of_kind(moments: &[String], kind: Kind, part: &str) -> Result<bool, TemplateError> {
    let mut found = false;
    for moment in moments {
        if moment == "revenge_before" {
            continue;
        }
        let known = Kind::from_name(moment)
            .ok_or_else(|| fault(part, &format!("the moment {moment:?} is unknown")))?;
        found |= known == kind;
    }
    Ok(found)
}

fn need_bits(part: &str, needs: &[String]) -> Result<u32, TemplateError> {
    let mut bits = 0;
    for need in needs {
        let need = Need::from_name(need)
            .ok_or_else(|| fault(part, &format!("the need {need:?} is unknown")))?;
        bits |= need.bit();
    }
    Ok(bits)
}

fn slot_token(part: &str, name: &str) -> Result<Token, TemplateError> {
    match name.to_lowercase().as_str() {
        "hero" => Ok(Token::Hero),
        "lore" => Ok(Token::Lore),
        _ => Slot::from_name(name)
            .map(|slot| Token::Slot(slot.id()))
            .ok_or_else(|| fault(part, &format!("the slot {{{name}}} is unknown"))),
    }
}

/// A word and the mark at its end: "fell." is "fell" and ".".
fn split_mark(piece: &str) -> (&str, Option<&'static str>) {
    for mark in MARKS {
        if let Some(body) = piece.strip_suffix(mark) {
            return (body, Some(mark));
        }
    }
    (piece, None)
}

fn fault(part: &str, reason: &str) -> TemplateError {
    TemplateError::Part {
        part: part.to_string(),
        reason: reason.to_string(),
    }
}
