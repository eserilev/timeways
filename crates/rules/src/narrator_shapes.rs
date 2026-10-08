//! The shapes of a narrator line (docs/plans/narrator-templates.md 5): the model writes
//! the lore, and the code writes the hero. A shape is a list of template parts. The rules
//! build a line of tokens from it, decide if it fits a moment, and pick one in turn. Lean
//! proves the laws of section 5.2 (lean/Timeways/NarratorShapes.lean).
//!
//! The rules work over token ids, never over strings: the story program renders the text.
//! The functions walk by index, because Aeneas translates no iterator adapter
//! (lean/README.md).

/// The window of the rotation: no main part comes back within this many lines. It is the
/// length of the naming rotation.
pub const WINDOW: usize = 8;

/// A built line never holds more tokens. So a line never overflows a `Vec`.
pub const MOST_TOKENS: usize = 1024;

/// One token of a line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Token {
    /// A word or a mark, by its id in the word list of the story program.
    Word(u16),
    /// The free sentence of history that the model wrote.
    Lore,
    /// The naming of the hero: `$N`, "the paladin", or a title.
    Hero,
    /// Any other value of the moment, by its slot id: a foe, a zone, a count.
    Slot(u8),
}

/// What a part does in a line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartKind {
    /// An opener of the deed sentence: "Now,".
    Connective,
    /// The clause of the deed: "{Foe} fell to {hero}."
    Deed,
    /// The clause of an order or a people: "{group}".
    Group,
    /// A verb that a group takes: "grows stronger."
    Grow,
    /// The level fact: "{Hero} has reached level {level}."
    Coda,
}

/// One part of a template. `tags` and `needs` are bit sets: two parts of one shape share
/// no tag, and each need of a part must hold for the moment.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Part {
    pub kind: PartKind,
    pub tokens: Vec<Token>,
    pub tags: u32,
    pub needs: u32,
}

/// The parts of a template, and the words that never stand right before the hero.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Table {
    pub parts: Vec<Part>,
    /// The ids of "in", "inside", "within", "through", and "into".
    pub inside_words: Vec<u16>,
}

/// A shape: the indexes of its parts in the table, in the order of the line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shape {
    pub parts: Vec<usize>,
}

/// What the moment holds. `has[k]` says if `Slot(k)` has a value. `holds` is the bit set
/// of the needs that hold. A named turn names the hero once, and any other turn not at all.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Facts {
    pub has: Vec<bool>,
    pub holds: u32,
    pub named: bool,
}

/// The tokens of a shape: the lore, then the tokens of each part in order. None when a
/// part is out of the table, or the line passes `MOST_TOKENS`. An index loop.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn skeleton(table: &Table, shape: &Shape) -> Option<Vec<Token>> {
    let mut line = Vec::new();
    line.push(Token::Lore);
    let mut index = 0;
    while index < shape.parts.len() {
        let id = shape.parts[index];
        if id >= table.parts.len() {
            return None;
        }
        let tokens = &table.parts[id].tokens;
        if tokens.len() > MOST_TOKENS - line.len() {
            return None;
        }
        line = with_tokens(line, tokens);
        index += 1;
    }
    Some(line)
}

/// An index loop.
fn with_tokens(mut line: Vec<Token>, tokens: &[Token]) -> Vec<Token> {
    let mut index = 0;
    while index < tokens.len() {
        line.push(tokens[index]);
        index += 1;
    }
    line
}

/// The line of an arrival: the lore alone. It never reads the table.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn assemble_arrival() -> Vec<Token> {
    let mut line = Vec::new();
    line.push(Token::Lore);
    line
}

/// The tokens of the line, or None when a slot of the shape has no value.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn assemble(table: &Table, shape: &Shape, facts: &Facts) -> Option<Vec<Token>> {
    let Some(line) = skeleton(table, shape) else {
        return None;
    };
    if !every_value_is_known(&line, facts) {
        return None;
    }
    Some(line)
}

/// True when the shape builds a line for this moment: each slot has a value, each need
/// holds, no two parts share a tag, and the line names the hero once on a named turn and
/// never on another turn.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn fits(table: &Table, shape: &Shape, facts: &Facts) -> bool {
    let Some(line) = skeleton(table, shape) else {
        return false;
    };
    every_value_is_known(&line, facts)
        && needs_and_tags_hold(table, shape, facts)
        && named_right(facts.named, hero_count(&line))
}

/// One hero on a named turn, and none on another turn.
fn named_right(named: bool, heroes: usize) -> bool {
    if named {
        return heroes == 1;
    }
    heroes == 0
}

/// An index loop.
fn every_value_is_known(line: &[Token], facts: &Facts) -> bool {
    let mut index = 0;
    while index < line.len() {
        if !has_value(line[index], facts) {
            return false;
        }
        index += 1;
    }
    true
}

/// The lore and each word always have a value. The hero has one on a named turn.
fn has_value(token: Token, facts: &Facts) -> bool {
    match token {
        Token::Word(_) | Token::Lore => true,
        Token::Hero => facts.named,
        Token::Slot(slot) => {
            let at = slot as usize;
            at < facts.has.len() && facts.has[at]
        }
    }
}

/// Each need of each part holds, and no two parts share a tag. An index loop.
fn needs_and_tags_hold(table: &Table, shape: &Shape, facts: &Facts) -> bool {
    let mut seen_tags: u32 = 0;
    let mut index = 0;
    while index < shape.parts.len() {
        let id = shape.parts[index];
        if id >= table.parts.len() {
            return false;
        }
        let part = &table.parts[id];
        if part.needs & facts.holds != part.needs || part.tags & seen_tags != 0 {
            return false;
        }
        seen_tags |= part.tags;
        index += 1;
    }
    true
}

/// The hero slots of a line. An index loop.
#[must_use]
pub fn hero_count(line: &[Token]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < line.len() {
        if is_hero(line[index]) {
            count += 1;
        }
        index += 1;
    }
    count
}

/// The checks of the data, run when the templates load: no part holds the lore, only a
/// deed or a coda holds the hero, each shape builds a line, no word of `inside_words`
/// stands right before the hero, and no two shapes build the same line.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn table_ok(table: &Table, shapes: &[Shape]) -> bool {
    parts_ok(table) && shapes_ok(table, shapes) && distinct_skeletons(table, shapes)
}

/// True when the last part of the shape is a coda of the table. A setup line ends on its
/// coda: the code tells what holds now, and the model never does
/// (docs/plans/lore-names-and-now.md 2.3 A).
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn ends_on_coda(table: &Table, shape: &Shape) -> bool {
    let count = shape.parts.len();
    if count == 0 {
        return false;
    }
    let id = shape.parts[count - 1];
    if id >= table.parts.len() {
        return false;
    }
    matches!(table.parts[id].kind, PartKind::Coda)
}

/// An index loop.
fn parts_ok(table: &Table) -> bool {
    let mut index = 0;
    while index < table.parts.len() {
        if !part_ok(&table.parts[index]) {
            return false;
        }
        index += 1;
    }
    true
}

/// No lore, and a hero only in a deed or a coda. An index loop.
fn part_ok(part: &Part) -> bool {
    let may_name = match part.kind {
        PartKind::Deed | PartKind::Coda => true,
        PartKind::Connective | PartKind::Group | PartKind::Grow => false,
    };
    let mut index = 0;
    while index < part.tokens.len() {
        match part.tokens[index] {
            Token::Lore => return false,
            Token::Hero => {
                if !may_name {
                    return false;
                }
            }
            Token::Word(_) | Token::Slot(_) => {}
        }
        index += 1;
    }
    true
}

/// An index loop.
fn shapes_ok(table: &Table, shapes: &[Shape]) -> bool {
    let mut index = 0;
    while index < shapes.len() {
        let Some(line) = skeleton(table, &shapes[index]) else {
            return false;
        };
        if inside_the_hero(&line, &table.inside_words) {
            return false;
        }
        index += 1;
    }
    true
}

/// True when a word of `inside_words` stands right before a hero slot. An index loop.
fn inside_the_hero(line: &[Token], inside_words: &[u16]) -> bool {
    let mut index = 1;
    while index < line.len() {
        if is_inside_word(line[index - 1], inside_words) && is_hero(line[index]) {
            return true;
        }
        index += 1;
    }
    false
}

fn is_inside_word(token: Token, inside_words: &[u16]) -> bool {
    match token {
        Token::Word(word) => holds(inside_words, word),
        Token::Lore | Token::Hero | Token::Slot(_) => false,
    }
}

fn is_hero(token: Token) -> bool {
    matches!(token, Token::Hero)
}

/// True when no two shapes build the same line. An index loop over the pairs.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn distinct_skeletons(table: &Table, shapes: &[Shape]) -> bool {
    let mut first = 0;
    while first < shapes.len() {
        if !differs_from_later(table, shapes, first) {
            return false;
        }
        first += 1;
    }
    true
}

/// True when the line of `shapes[first]` differs from the line of each shape after it.
/// A shape that builds no line differs from nothing. An index loop.
fn differs_from_later(table: &Table, shapes: &[Shape], first: usize) -> bool {
    let Some(line) = skeleton(table, &shapes[first]) else {
        return false;
    };
    let mut later = first + 1;
    while later < shapes.len() {
        let Some(other) = skeleton(table, &shapes[later]) else {
            return false;
        };
        if same_line(&line, &other) {
            return false;
        }
        later += 1;
    }
    true
}

/// An index loop, because Aeneas translates no `==` on a slice.
fn same_line(a: &[Token], b: &[Token]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if !same_token(a[index], b[index]) {
            return false;
        }
        index += 1;
    }
    true
}

fn same_token(a: Token, b: Token) -> bool {
    match a {
        Token::Word(x) => match b {
            Token::Word(y) => x == y,
            _ => false,
        },
        Token::Lore => matches!(b, Token::Lore),
        Token::Hero => matches!(b, Token::Hero),
        Token::Slot(x) => match b {
            Token::Slot(y) => x == y,
            _ => false,
        },
    }
}

/// An index loop, because Aeneas translates no `contains`.
fn holds(ids: &[u16], id: u16) -> bool {
    let mut index = 0;
    while index < ids.len() {
        if ids[index] == id {
            return true;
        }
        index += 1;
    }
    false
}

/// The shape of a line. `fits[i]` says if shape `i` fits the moment, and `mains[i]` is the
/// id of its main part. `recent` holds the main parts of the last lines, oldest first.
///
/// The walk starts at `turn mod count`. It takes the first fitting shape whose main part
/// is not in `recent`. When every fitting shape has a recent main part, it takes the one
/// whose main part was used longest ago. None when no shape fits, or the two lists differ
/// in length.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn pick(fits: &[bool], mains: &[u16], recent: &[u16], turn: u64) -> Option<usize> {
    let count = fits.len();
    if count == 0 || mains.len() != count {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the remainder is below `count`, so it fits a usize"
    )]
    let start = (turn % count as u64) as usize;
    if let Some(index) = first_fresh(fits, mains, recent, start) {
        return Some(index);
    }
    longest_unused(fits, mains, recent, start)
}

/// The set of fits that a preferring pick took its shape from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tier {
    Preferred,
    Usual,
    Fallback,
}

/// The shape of a line with a preference. `preferred`, `usual`, and `fallback` are three
/// sets of fits over the same shapes and `mains`. The pick takes a fresh fitting shape of
/// `preferred`. Else it takes the pick of `usual`, a recent shape too. Else the pick of
/// `fallback`. A kill on a named turn prefers the parts with no hero (docs/plans/
/// narrator-templates.md 3.5), and a turn whose naming fits no shape falls back to the other
/// naming.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn pick_preferring(
    preferred: &[bool],
    usual: &[bool],
    fallback: &[bool],
    mains: &[u16],
    recent: &[u16],
    turn: u64,
) -> Option<(usize, Tier)> {
    if let Some(index) = fresh_pick(preferred, mains, recent, turn) {
        return Some((index, Tier::Preferred));
    }
    if let Some(index) = pick(usual, mains, recent, turn) {
        return Some((index, Tier::Usual));
    }
    match pick(fallback, mains, recent, turn) {
        Some(index) => Some((index, Tier::Fallback)),
        None => None,
    }
}

/// The pick, when its main part is not recent.
fn fresh_pick(fits: &[bool], mains: &[u16], recent: &[u16], turn: u64) -> Option<usize> {
    let Some(index) = pick(fits, mains, recent, turn) else {
        return None;
    };
    if holds(recent, mains[index]) {
        return None;
    }
    Some(index)
}

/// The window of the rotation: the main parts of the last `WINDOW` lines, oldest first.
/// `lines` holds the main part of each accepted line of the character, oldest first, or
/// None for a line with no main part, such as an arrival. Such a line still takes its
/// place in the window. An index loop.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn window(lines: &[Option<u16>]) -> Vec<u16> {
    let mut mains = Vec::new();
    let mut index = lines.len().saturating_sub(WINDOW);
    while index < lines.len() {
        // A copy first: Aeneas cannot match on a slot of a slice in place.
        let line = lines[index];
        if let Some(main) = line {
            mains.push(main);
        }
        index += 1;
    }
    mains
}

/// The index `step` places after `start`, around the end of a list of `count`. It never
/// overflows, because `start` and `step` are below `count`.
fn around(start: usize, step: usize, count: usize) -> usize {
    if step < count - start {
        start + step
    } else {
        step - (count - start)
    }
}

/// The first fitting shape from `start` whose main part is not recent. An index loop.
fn first_fresh(fits: &[bool], mains: &[u16], recent: &[u16], start: usize) -> Option<usize> {
    let mut step = 0;
    while step < fits.len() {
        let index = around(start, step, fits.len());
        if fits[index] && !holds(recent, mains[index]) {
            return Some(index);
        }
        step += 1;
    }
    None
}

/// The fitting shape whose main part was last used longest ago. Of two, the first from
/// `start` wins. An index loop.
fn longest_unused(fits: &[bool], mains: &[u16], recent: &[u16], start: usize) -> Option<usize> {
    let mut best: Option<usize> = None;
    let mut best_use = 0;
    let mut step = 0;
    while step < fits.len() {
        let index = around(start, step, fits.len());
        if fits[index] {
            let used = last_use(recent, mains[index]);
            let older = match best {
                Some(_) => used < best_use,
                None => true,
            };
            if older {
                best = Some(index);
                best_use = used;
            }
        }
        step += 1;
    }
    best
}

/// The place of the last use of `main` in `recent`, or 0 when it has none. An index loop
/// from the end.
fn last_use(recent: &[u16], main: u16) -> usize {
    let mut index = recent.len();
    while index > 0 {
        index -= 1;
        if recent[index] == main {
            return index;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u16 = 1;
    const IN: u16 = 2;
    const DOT: u16 = 3;

    fn part(kind: PartKind, tokens: &[Token]) -> Part {
        Part {
            kind,
            tokens: tokens.to_vec(),
            tags: 0,
            needs: 0,
        }
    }

    fn table(parts: Vec<Part>) -> Table {
        Table {
            parts,
            inside_words: vec![IN],
        }
    }

    fn shape(parts: &[usize]) -> Shape {
        Shape {
            parts: parts.to_vec(),
        }
    }

    fn named() -> Facts {
        Facts {
            has: vec![true],
            holds: 0,
            named: true,
        }
    }

    fn deed() -> Part {
        part(
            PartKind::Deed,
            &[
                Token::Slot(0),
                Token::Word(9),
                Token::Hero,
                Token::Word(DOT),
            ],
        )
    }

    #[test]
    fn a_shape_ends_on_a_coda_only_when_its_last_part_is_one() {
        let coda = part(PartKind::Coda, &[Token::Slot(0), Token::Word(DOT)]);
        let table = table(vec![deed(), coda]);

        assert!(ends_on_coda(&table, &shape(&[0, 1])));
        assert!(!ends_on_coda(&table, &shape(&[1, 0])));
        assert!(!ends_on_coda(&table, &shape(&[])));
        assert!(!ends_on_coda(&table, &shape(&[2])));
    }

    #[test]
    fn a_line_starts_with_the_lore() {
        let table = table(vec![deed()]);

        let line = assemble(&table, &shape(&[0]), &named()).unwrap_or_default();

        assert_eq!(line[0], Token::Lore);
        assert_eq!(line.len(), 5);
    }

    #[test]
    fn an_arrival_is_the_lore_alone() {
        assert_eq!(assemble_arrival(), [Token::Lore]);
    }

    #[test]
    fn a_slot_with_no_value_builds_nothing() {
        let table = table(vec![deed()]);
        let facts = Facts {
            has: vec![false],
            ..named()
        };

        assert_eq!(assemble(&table, &shape(&[0]), &facts), None);
        assert!(!fits(&table, &shape(&[0]), &facts));
    }

    #[test]
    fn an_unnamed_turn_takes_no_part_with_the_hero() {
        let table = table(vec![deed()]);
        let unnamed = Facts {
            named: false,
            ..named()
        };

        assert!(fits(&table, &shape(&[0]), &named()));
        assert!(!fits(&table, &shape(&[0]), &unnamed));
    }

    #[test]
    fn two_parts_with_one_tag_never_fit() {
        let mut now = part(PartKind::Connective, &[Token::Word(NOW)]);
        now.tags = 1;
        let mut deed = deed();
        deed.tags = 1;
        let table = table(vec![now, deed]);

        assert!(!fits(&table, &shape(&[0, 1]), &named()));
    }

    #[test]
    fn a_part_needs_its_facts() {
        let mut deed = deed();
        deed.needs = 0b10;
        let table = table(vec![deed]);
        let holds = Facts {
            holds: 0b11,
            ..named()
        };

        assert!(!fits(&table, &shape(&[0]), &named()));
        assert!(fits(&table, &shape(&[0]), &holds));
    }

    #[test]
    fn the_hero_after_in_fails_the_table() {
        let inside = part(PartKind::Deed, &[Token::Word(IN), Token::Hero]);
        let table = table(vec![inside]);

        assert!(!table_ok(&table, &[shape(&[0])]));
    }

    #[test]
    fn a_group_that_holds_the_hero_fails_the_table() {
        let group = part(PartKind::Group, &[Token::Hero]);
        let table = table(vec![group]);

        assert!(!table_ok(&table, &[shape(&[0])]));
    }

    #[test]
    fn two_shapes_of_one_line_fail_the_table() {
        let table = table(vec![deed(), deed()]);

        assert!(!table_ok(&table, &[shape(&[0]), shape(&[1])]));
        assert!(table_ok(&table, &[shape(&[0])]));
    }

    #[test]
    fn the_pick_starts_at_the_turn_and_skips_recent_mains() {
        let fits = [true, true, true];
        let mains = [10, 11, 12];

        assert_eq!(pick(&fits, &mains, &[], 4), Some(1));
        assert_eq!(pick(&fits, &mains, &[11], 4), Some(2));
        assert_eq!(pick(&fits, &mains, &[11, 12], 4), Some(0));
    }

    #[test]
    fn when_all_mains_are_recent_the_oldest_wins() {
        let fits = [true, true, false];
        let mains = [10, 11, 12];

        assert_eq!(pick(&fits, &mains, &[11, 10, 12], 0), Some(1));
        assert_eq!(pick(&fits, &mains, &[10, 11], 1), Some(0));
    }

    #[test]
    fn a_fresh_preferred_shape_wins_over_the_usual_set() {
        let mains = [10, 11, 12];

        let picked = pick_preferring(
            &[false, true, false],
            &[true; 3],
            &[true; 3],
            &mains,
            &[],
            0,
        );

        assert_eq!(picked, Some((1, Tier::Preferred)));
    }

    #[test]
    fn a_recent_preferred_shape_gives_way_to_the_usual_set() {
        let mains = [10, 11, 12];

        let picked = pick_preferring(
            &[false, true, false],
            &[true; 3],
            &[false; 3],
            &mains,
            &[11],
            0,
        );

        assert_eq!(picked, Some((0, Tier::Usual)));
    }

    #[test]
    fn the_fallback_comes_only_when_nothing_else_fits() {
        let mains = [10, 11];

        let only_recent = pick_preferring(
            &[true, false],
            &[false; 2],
            &[true, false],
            &mains,
            &[10],
            0,
        );
        let nothing = pick_preferring(&[false; 2], &[false; 2], &[false; 2], &mains, &[], 0);

        assert_eq!(only_recent, Some((0, Tier::Fallback)));
        assert_eq!(nothing, None);
    }

    #[test]
    fn the_window_holds_the_main_parts_of_the_last_eight_lines() {
        let mut lines: Vec<Option<u16>> = (0..10).map(Some).collect();
        lines[8] = None;

        assert_eq!(window(&lines), [2, 3, 4, 5, 6, 7, 9]);
        assert_eq!(window(&[None, Some(4)]), [4]);
        assert!(window(&[]).is_empty());
    }

    #[test]
    fn nothing_fits_picks_nothing() {
        assert_eq!(pick(&[false, false], &[1, 2], &[], 0), None);
        assert_eq!(pick(&[], &[], &[], 0), None);
        assert_eq!(pick(&[true], &[], &[], 0), None);
    }
}
