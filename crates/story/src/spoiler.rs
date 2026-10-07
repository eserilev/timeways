//! The spoiler limit of a pack passage (GAMEPLAY.md 3.1 and 5.10): the world of the player
//! holds every link of the passage, and the deed that an outcome passage tells. The rule
//! of the deed lives in `timeways_rules::outcomes`, where Lean proves it. This module only
//! turns names into ids.

use crate::character::Character;
use crate::pack::{Dependency, Passage};
use timeways_rules::outcomes::{DependsOn, PlayerFacts, outcome_usable};

/// Every pick of lore goes through this: the narrator, `/lore`, and so each text that a
/// narrator line feeds, such as a tale or a summary.
#[must_use]
pub fn may_show(character: &Character, passage: &Passage) -> bool {
    character.knows_all(&passage.links) && outcome_allowed(character, passage.depends_on.as_ref())
}

/// True when the passage tells no deed, or a deed that the player did.
#[must_use]
pub fn outcome_allowed(character: &Character, depends_on: Option<&Dependency>) -> bool {
    let mut names = Names::default();
    let facts = PlayerFacts {
        defeated: names.ids(character.foes_defeated()),
        quests_done: names.ids(character.game_quests_done()),
    };
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
