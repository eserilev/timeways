//! The common goods that a carry step can ask for (docs/plans/quest-variety.md 4.9). The
//! world knows no items, so the goods come from a list with the levels where a player meets
//! each one.

use crate::check::data_lines;

const CARRY_ITEMS: &str = include_str!("../../data/carry_items.txt");

/// The goods whose level band holds this level, in the order of the list. With no level,
/// none.
#[must_use]
pub fn goods_for(level: Option<i64>) -> Vec<&'static str> {
    let Some(level) = level else {
        return Vec::new();
    };
    data_lines(CARRY_ITEMS)
        .filter_map(good)
        .filter(|(_, low, high)| (*low..=*high).contains(&level))
        .map(|(name, _, _)| name)
        .collect()
}

/// One line of the list: the name, the lowest level, and the highest level.
fn good(line: &str) -> Option<(&str, i64, i64)> {
    let mut parts = line.split('|').map(str::trim);
    let name = parts.next()?;
    let low = parts.next()?.parse().ok()?;
    let high = parts.next()?.parse().ok()?;
    Some((name, low, high))
}
