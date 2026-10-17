//! The alias table in the lines of the addon (GAMEPLAY.md 5.11): a line names players, and
//! the table gives each one an ID before any prompt is built.

use super::{Active, Output, Story, StoryError};
use crate::aliases::{self, AliasRow, Surname};
use crate::hero::{self, Hero};
use crate::store::StoreError;

impl Story {
    /// A text that is no player name is refused, and its input stays.
    pub(super) fn describe_player(&mut self, row: &AliasRow) -> Result<Vec<Output>, StoryError> {
        let active = self.active.as_mut().ok_or(StoryError::NoCharacter)?;
        active.aliases.learn(row)?.ok_or(StoryError::BadName)?;
        Ok(Vec::new())
    }
}

/// Learns the names that the addon marked.
pub(super) fn learn_names(active: &mut Active, marked: &[String]) -> Result<(), StoreError> {
    for name in marked {
        active.aliases.learn(&AliasRow::named(name))?;
    }
    Ok(())
}

/// Learns each player that a text of the hero names, as "First-Surname" or "First". Only
/// the first `MAX_NAMES` count, and a first name that the text does not hold is no player
/// of it.
pub(super) fn learn_players(
    active: &mut Active,
    text: &str,
    players: &[String],
) -> Result<(), StoreError> {
    let words: Vec<String> = aliases::words_in(text).map(aliases::key_of).collect();
    for player in players.iter().take(aliases::MAX_NAMES) {
        let (first, surname) = player.split_once('-').unwrap_or((player, ""));
        if !words.contains(&aliases::key_of(first)) {
            continue;
        }
        active.aliases.learn(&AliasRow::named(first))?;
        if aliases::player_name(surname).is_some() {
            active.aliases.learn(&AliasRow::surname(surname, first))?;
        }
    }
    Ok(())
}

/// The text with the name of the character as `$N`, for a text that the player wrote.
pub(super) fn without_own_name(active: &Active, text: &str) -> String {
    aliases::without_own_name(text, active.key.name(), Surname::AsWritten)
}

/// The text with each name that the table knows as its ID. A surname right after its first
/// name goes with it.
pub(super) fn with_ids(active: &Active, text: &str) -> String {
    let with_ids = aliases::without_names(active.aliases.table(), text);
    aliases::without_surnames(active.aliases.rows(), &with_ids)
}

/// The hero as a model reads it: each name of a player as its ID, and the name of the
/// character as `$N`. The disk keeps the words as the player typed them, for the sheet, so
/// a typed brace becomes a parenthesis only here.
pub(super) fn hero_for_model(active: &Active) -> Hero {
    let mut hero = hero::hero(active.hero.changes());
    for field in &mut hero.sheet {
        field.text = for_model(active, &field.text);
    }
    for entry in &mut hero.entries {
        entry.text = for_model(active, &entry.text);
    }
    hero
}

fn for_model(active: &Active, text: &str) -> String {
    let log = &active.aliases;
    aliases::typed_for_model(log.table(), log.rows(), active.key.name(), text)
}

/// The text for the player's own screen: each ID becomes the name of its player.
pub(super) fn with_names(active: &Active, text: &str) -> String {
    aliases::with_names(active.aliases.table(), text)
}

/// True when each ID of a model text names a player of the table, and the text with the
/// names still keeps its limits. A name is longer than its ID, and the page budget of the
/// journal counts on the limits.
pub(super) fn shows_with_names(active: &Active, text: &str, limits: TextLimits) -> bool {
    let table = active.aliases.table();
    let named = aliases::with_names(table, text);
    aliases::knows_every_id(table, text)
        && named.chars().count() <= limits.chars
        && named.len() <= limits.bytes
}

/// The most characters and bytes of one text of the narrator.
#[derive(Clone, Copy, Debug)]
pub(super) struct TextLimits {
    pub(super) chars: usize,
    pub(super) bytes: usize,
}

/// Learns the names that the addon marked, and gives the text with each name that the
/// table knows as its ID.
pub(super) fn without_names(
    active: &mut Active,
    marked: &[String],
    text: &str,
) -> Result<String, StoreError> {
    learn_names(active, marked)?;
    Ok(with_ids(active, text))
}
