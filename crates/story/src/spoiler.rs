//! The spoiler limit of a pack passage (GAMEPLAY.md 3.1 and 5.10). The one gate lives in
//! `timeways_rules::spoiler::passage_usable`, where Lean proves it: the links, the deeds
//! that an outcome passage tells, and the deed that a setup passage asks for. This module
//! only turns names into ids, with the rows of game names (`game_names`), so a kill, a
//! meeting, or a visit under either name counts.

use crate::character::Character;
use crate::game_names;
use crate::pack::{Deed, Dependency, Link, Passage, SetupFor};
use timeways_rules::game_names::NameRow;
use timeways_rules::outcomes::{DependsOn, PlayerFacts, outcomes_usable};
use timeways_rules::setups::{self, setup_usable};
use timeways_rules::spoiler::{LinkTo, WorldFacts, passage_usable};

/// Every pick of lore goes through this: the narrator, `/lore`, and so each text that a
/// narrator line feeds, such as a tale or a summary.
#[must_use]
pub fn may_show(character: &Character, passage: &Passage) -> bool {
    let mut names = Names::default();
    let links: Vec<LinkTo> = passage
        .links
        .iter()
        .map(|link| link_to(link, &mut names))
        .collect();
    let tags = tags_of(&passage.depends_on, &mut names);
    let setup = setup_of(passage.setup_for.as_ref(), &mut names);
    let facts = WorldFacts {
        deeds: facts_of(character, &mut names),
        visited: names.ids(character.places_visited()),
        met: names.ids(character.npcs_met()),
    };
    passage_usable(&links, &tags, setup, &facts)
}

/// True when the passage sets up no deed, or a deed that the player has not done: "Gryan
/// Stoutmantle wants VanCleef dead" is stale once VanCleef is dead.
#[must_use]
pub fn setup_allowed(character: &Character, setup_for: Option<&SetupFor>) -> bool {
    let mut names = Names::default();
    let tag = setup_of(setup_for, &mut names);
    setup_usable(tag, &facts_of(character, &mut names))
}

/// True when the passage tells no deed, or only deeds that the player did. A passage
/// that tells two deeds shows only after both.
#[must_use]
pub fn outcome_allowed(character: &Character, depends_on: &[Dependency]) -> bool {
    let mut names = Names::default();
    let tags = tags_of(depends_on, &mut names);
    outcomes_usable(&tags, &facts_of(character, &mut names))
}

fn link_to<'a>(link: &'a Link, names: &mut Names<'a>) -> LinkTo {
    match link {
        Link::Common => LinkTo::Common,
        Link::Place(name) => LinkTo::Place(names.id(name)),
        Link::Npc(name) => LinkTo::Npc(names.id(name)),
    }
}

fn tags_of<'a>(depends_on: &'a [Dependency], names: &mut Names<'a>) -> Vec<DependsOn> {
    depends_on
        .iter()
        .map(|dependency| match dependency {
            Dependency::Unresolved => DependsOn::Unresolved,
            Dependency::Foe(name) => DependsOn::Foe(names.id(name)),
            Dependency::Quest(title) => DependsOn::Quest(names.id(title)),
        })
        .collect()
}

fn setup_of<'a>(setup_for: Option<&'a SetupFor>, names: &mut Names<'a>) -> setups::SetupFor {
    match setup_for.map(|setup| &setup.deed) {
        None => setups::SetupFor::Nothing,
        Some(Deed::Foe(name)) => setups::SetupFor::Foe(names.id(name)),
        Some(Deed::Quest(title)) => setups::SetupFor::Quest(names.id(title)),
    }
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
