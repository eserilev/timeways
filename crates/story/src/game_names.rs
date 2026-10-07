//! Game names (docs/plans/lore-names-and-now.md 1). The pack names people and places as the
//! wiki does, and the world as the game does: the kill of "High Inquisitor Whitemane" is
//! the kill of "Sally Whitemane". The hand-checked rows of `data/game_names.toml` tie the
//! exact names together. Both sides of every compare of a name go through `wiki_name`.
//! The gates of the spoiler limit read the rows as ids (`timeways_rules::game_names`).

use serde::Deserialize;
use std::sync::LazyLock;

const BUNDLED: &str = include_str!("../data/game_names.toml");

/// The rows of the bundled file. A test reads it, so it is never broken.
pub static ROWS: LazyLock<Result<Vec<Row>, toml::de::Error>> = LazyLock::new(|| parse(BUNDLED));

/// One person or place: its name in the pack, and its names in the game.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub wiki: String,
    pub game: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    person: Vec<Row>,
    #[serde(default)]
    place: Vec<Row>,
}

/// # Errors
///
/// Returns an error when the text is not a valid file of game names.
pub fn parse(text: &str) -> Result<Vec<Row>, toml::de::Error> {
    let file: File = toml::from_str(text)?;
    Ok([file.person, file.place].concat())
}

/// The rows of the bundled file, or none when it is broken.
#[must_use]
pub fn rows() -> &'static [Row] {
    ROWS.as_deref().unwrap_or_default()
}

/// The pack name of the person or place that `name` names: the wiki name of its row, or
/// the name itself.
#[must_use]
pub fn wiki_name(name: &str) -> &str {
    rows()
        .iter()
        .find(|row| row.game.iter().any(|game| game == name))
        .map_or(name, |row| row.wiki.as_str())
}

/// True when the two names name one person or place.
#[must_use]
pub fn same(one: &str, other: &str) -> bool {
    wiki_name(one) == wiki_name(other)
}
