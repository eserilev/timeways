//! What a model call read, as rows (docs/plans/links.md 6). Each rule reads at least what
//! the prompt held. Reading too much only counts a row as used, which is the safe side.

use super::Active;
use crate::hero::{Change, Entry};
use crate::npc_memory::{Memory, Source};
use crate::pack::{Origin, Passage};
use crate::store::{Node, Table};
use hourglass::Tick;

/// The events behind what the world holds about each name.
pub(super) fn events_about<'a>(
    active: &Active,
    names: impl IntoIterator<Item = &'a str>,
) -> Vec<Node> {
    let mut events: Vec<u64> = names
        .into_iter()
        .flat_map(|name| active.character.events_about(name))
        .map(|event| event.0)
        .collect();
    events.sort_unstable();
    events.dedup();
    events
        .into_iter()
        .map(|event| Node::Row(Table::Events, event))
        .collect()
}

/// The `learned` rows of the passages from text that the player read. A pack passage
/// has no row: its id changes with each pack.
pub(super) fn passages_read(active: &Active, passages: &[Passage]) -> Vec<Node> {
    let sources: Vec<&str> = passages
        .iter()
        .filter(|passage| passage.origin == Origin::Read)
        .map(|passage| passage.source.as_str())
        .collect();
    active
        .learned
        .read_with_rows()
        .filter(|(_, read)| sources.contains(&read.text.passage().source.as_str()))
        .map(|(row, _)| Node::Row(Table::Learned, row))
        .collect()
}

/// Every `quests` row of these quests.
pub(super) fn quest_rows(active: &Active, numbers: &[u64]) -> Vec<Node> {
    (0..)
        .zip(active.quests.changes())
        .filter(|(_, change)| numbers.contains(&change.number()))
        .map(|(row, _)| Node::Row(Table::Quests, row))
        .collect()
}

/// The hero rows that add an entry that `about` picks.
pub(super) fn entries_read(active: &Active, about: impl Fn(&Entry) -> bool) -> Vec<Node> {
    (0..)
        .zip(active.hero.changes())
        .filter(|(_, change)| matches!(change, Change::Added(entry) if about(entry)))
        .map(|(row, _)| Node::Row(Table::Hero, row))
        .collect()
}

/// The rows behind each memory of a talk (GAMEPLAY.md 3.5).
pub(super) fn memories_read(memories: &[Memory]) -> Vec<Node> {
    let sources = memories.iter().flat_map(|memory| &memory.sources);
    sources
        .map(|source| match *source {
            Source::Event(event) => Node::Row(Table::Events, event.0),
            Source::Learned(row) => Node::Row(Table::Learned, row),
            Source::Quest(row) => Node::Row(Table::Quests, row),
        })
        .collect()
}

/// Every event, hero row, and flavor moment of a chapter. The hero rows before it count
/// too, because the portrait shows the sheet as it stands.
pub(super) fn chapter_read(active: &Active, began: Tick, next: Tick) -> Vec<Node> {
    let in_chapter = |at: Tick| at >= began && at < next;
    let events = active
        .character
        .world()
        .history()
        .iter()
        .filter(|event| in_chapter(event.tick))
        .map(|event| Node::Row(Table::Events, event.id.0));
    let hero = (0..)
        .zip(active.hero.changes())
        .filter(|(_, change)| change_at(change) < next)
        .map(|(row, _)| Node::Row(Table::Hero, row));
    let flavor = active
        .flavor
        .moments_with_rows()
        .filter(|(_, moment)| in_chapter(moment.at))
        .map(|(row, _)| Node::Row(Table::Flavor, row));
    events.chain(hero).chain(flavor).collect()
}

fn change_at(change: &Change) -> Tick {
    match change {
        Change::Set { at, .. } | Change::Removed { at, .. } => *at,
        Change::Added(entry) => entry.at,
    }
}
