//! The spoiler limit of a pack passage (GAMEPLAY.md 3.1 and 5.10): the world of the player
//! holds every link of the passage, and the deed that an outcome passage tells, and not
//! the deed that a setup passage asks for. The rules of the deeds live in
//! `timeways_rules::outcomes` and `timeways_rules::setups`, where Lean proves them. This
//! module only turns names into ids, with the rows of game names (`game_names`), so a
//! kill under either name of a person counts.

use crate::character::Character;
use crate::game_names;
use crate::pack::{Deed, Dependency, Passage, SetupFor};
use timeways_rules::game_names::NameRow;
use timeways_rules::outcomes::{DependsOn, PlayerFacts, outcome_usable};
use timeways_rules::setups::{self, setup_usable};

/// Every pick of lore goes through this: the narrator, `/lore`, and so each text that a
/// narrator line feeds, such as a tale or a summary.
#[must_use]
pub fn may_show(character: &Character, passage: &Passage) -> bool {
    character.knows_all(&passage.links)
        && outcome_allowed(character, passage.depends_on.as_ref())
        && setup_allowed(character, passage.setup_for.as_ref())
}

/// True when the passage sets up no deed, or a deed that the player has not done: "Gryan
/// Stoutmantle wants VanCleef dead" is stale once VanCleef is dead.
#[must_use]
pub fn setup_allowed(character: &Character, setup_for: Option<&SetupFor>) -> bool {
    let mut names = Names::default();
    let facts = facts_of(character, &mut names);
    let tag = match setup_for.map(|setup| &setup.deed) {
        None => setups::SetupFor::Nothing,
        Some(Deed::Foe(name)) => setups::SetupFor::Foe(names.id(name)),
        Some(Deed::Quest(title)) => setups::SetupFor::Quest(names.id(title)),
    };
    setup_usable(tag, &facts)
}

fn facts_of<'a>(character: &'a Character, names: &mut Names<'a>) -> PlayerFacts {
    PlayerFacts {
        defeated: names.ids(character.foes_defeated()),
        quests_done: names.ids(character.game_quests_done()),
        names: name_rows(names),
    }
}

/// The rows of game names as ids: one row for each game name of a person or a place.
fn name_rows(names: &mut Names<'_>) -> Vec<NameRow> {
    let mut rows = Vec::new();
    for row in game_names::rows() {
        for game in &row.game {
            rows.push(NameRow {
                game: names.id(game),
                wiki: names.id(&row.wiki),
            });
        }
    }
    rows
}

/// True when the passage tells no deed, or a deed that the player did.
#[must_use]
pub fn outcome_allowed(character: &Character, depends_on: Option<&Dependency>) -> bool {
    let mut names = Names::default();
    let facts = facts_of(character, &mut names);
    let tag = match depends_on {
        None => DependsOn::Nothing,
        Some(Dependency::Unresolved) => DependsOn::Unresolved,
        Some(Dependency::Foe(name)) => DependsOn::Foe(names.id(name)),
        Some(Dependency::Quest(title)) => DependsOn::Quest(names.id(title)),
    };
    outcome_usable(tag, &facts)
}

/// One id for each distinct name.
#[derive(Default)]
struct Names<'a> {
    known: Vec<&'a str>,
}

impl<'a> Names<'a> {
    fn ids(&mut self, names: Vec<&'a str>) -> Vec<u32> {
        names.into_iter().map(|name| self.id(name)).collect()
    }

    fn id(&mut self, name: &'a str) -> u32 {
        let index = self
            .known
            .iter()
            .position(|known| *known == name)
            .unwrap_or_else(|| {
                self.known.push(name);
                self.known.len() - 1
            });
        u32::try_from(index).unwrap_or(u32::MAX)
    }
}
