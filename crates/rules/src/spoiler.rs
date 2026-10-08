//! The spoiler limit of a pack passage (GAMEPLAY.md 3.1 and 5.10), as one gate: the links,
//! the deeds that an outcome passage tells, and the deed that a setup asks for. A foe that
//! the player defeated counts as known. An outcome passage whose deeds the player did needs
//! no visit to the place where the pack files it: the deed is its gate. The story program
//! gives each name an id, so the rule reads ids. Lean proves its laws
//! (lean/Timeways/Spoiler.lean).
//!
//! The functions walk by index: Aeneas translates no iterator adapter (lean/README.md).

use crate::game_names::holds_person;
use crate::outcomes::{DependsOn, PlayerFacts, outcomes_usable};
use crate::setups::{SetupFor, setup_usable};

/// A link of a passage, as an id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkTo {
    /// Common knowledge: every player knows it.
    Common,
    /// A place that the player must have visited.
    Place(u32),
    /// An NPC that the player must have met or defeated.
    Npc(u32),
}

/// What the world of the player holds, as ids. The name rows of `deeds` also tie the game
/// name of a place to its wiki name.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct WorldFacts {
    pub deeds: PlayerFacts,
    pub visited: Vec<u32>,
    pub met: Vec<u32>,
}

/// True when the world holds the link: a visit to the place, or a meeting with the NPC or
/// its defeat, under either name of a row.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn link_known(link: LinkTo, facts: &WorldFacts) -> bool {
    let names = &facts.deeds.names;
    match link {
        LinkTo::Common => true,
        LinkTo::Place(place) => holds_person(names, &facts.visited, place),
        LinkTo::Npc(npc) => {
            holds_person(names, &facts.met, npc) || holds_person(names, &facts.deeds.defeated, npc)
        }
    }
}

/// True when the tag names a deed: a foe or a quest.
#[must_use]
pub fn is_deed(tag: DependsOn) -> bool {
    matches!(tag, DependsOn::Foe(_) | DependsOn::Quest(_))
}

/// True when a tag names a deed. An index loop.
#[must_use]
pub fn tells_a_deed(tags: &[DependsOn]) -> bool {
    let mut index = 0;
    while index < tags.len() {
        if is_deed(tags[index]) {
            return true;
        }
        index += 1;
    }
    false
}

/// True when the link is a place, and the player did the deed that the passage tells: the
/// deed is the gate, so the place needs no visit.
#[must_use]
pub fn is_waived(link: LinkTo, deed_done: bool) -> bool {
    deed_done && matches!(link, LinkTo::Place(_))
}

/// True when the world holds every link that is not waived (`is_waived`). An index loop.
#[must_use]
pub fn links_known(links: &[LinkTo], deed_done: bool, facts: &WorldFacts) -> bool {
    let mut index = 0;
    while index < links.len() {
        if !is_waived(links[index], deed_done) && !link_known(links[index], facts) {
            return false;
        }
        index += 1;
    }
    true
}

/// The one gate of every pick of lore: `/lore`, the narrator, and each text that a
/// narrator line feeds.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn passage_usable(
    links: &[LinkTo],
    tags: &[DependsOn],
    setup_for: SetupFor,
    facts: &WorldFacts,
) -> bool {
    if !outcomes_usable(tags, &facts.deeds) {
        return false;
    }
    if !setup_usable(setup_for, &facts.deeds) {
        return false;
    }
    links_known(links, tells_a_deed(tags), facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_names::NameRow;

    const VANCLEEF: u32 = 1;
    const DEADMINES: u32 = 2;
    const MOONBROOK: u32 = 3;
    const STOUTMANTLE: u32 = 4;

    fn world(visited: &[u32], met: &[u32], defeated: &[u32]) -> WorldFacts {
        WorldFacts {
            deeds: PlayerFacts {
                defeated: defeated.to_vec(),
                ..PlayerFacts::default()
            },
            visited: visited.to_vec(),
            met: met.to_vec(),
        }
    }

    #[test]
    fn a_defeated_foe_is_known() {
        let facts = world(&[], &[], &[VANCLEEF]);

        assert!(link_known(LinkTo::Npc(VANCLEEF), &facts));
    }

    #[test]
    fn a_foe_defeated_under_its_game_name_is_known_under_its_wiki_name() {
        let mut facts = world(&[], &[], &[7]);
        facts.deeds.names = vec![NameRow { game: 7, wiki: 8 }];

        assert!(link_known(LinkTo::Npc(8), &facts));
    }

    #[test]
    fn a_place_visited_under_its_game_name_is_known_under_its_wiki_name() {
        let mut facts = world(&[7], &[], &[]);
        facts.deeds.names = vec![NameRow {
            game: 7,
            wiki: DEADMINES,
        }];

        assert!(link_known(LinkTo::Place(DEADMINES), &facts));
    }

    #[test]
    fn an_outcome_the_player_did_is_usable_wherever_it_is_filed() {
        let facts = world(&[DEADMINES], &[], &[VANCLEEF]);
        let tags = [DependsOn::Foe(VANCLEEF)];

        let usable = passage_usable(
            &[LinkTo::Place(MOONBROOK)],
            &tags,
            SetupFor::Nothing,
            &facts,
        );

        assert!(usable);
    }

    #[test]
    fn an_outcome_the_player_did_still_waits_for_its_npc() {
        let facts = world(&[], &[], &[VANCLEEF]);
        let tags = [DependsOn::Foe(VANCLEEF)];

        let usable = passage_usable(
            &[LinkTo::Npc(STOUTMANTLE)],
            &tags,
            SetupFor::Nothing,
            &facts,
        );

        assert!(!usable);
    }

    #[test]
    fn a_passage_with_no_deed_still_waits_for_its_place() {
        let facts = world(&[DEADMINES], &[], &[VANCLEEF]);

        let usable = passage_usable(&[LinkTo::Place(MOONBROOK)], &[], SetupFor::Nothing, &facts);

        assert!(!usable);
    }

    #[test]
    fn a_setup_keeps_its_place_gate_and_goes_stale_after_the_deed() {
        let setup = SetupFor::Foe(VANCLEEF);
        let moonbrook = [LinkTo::Place(MOONBROOK)];

        assert!(!passage_usable(
            &moonbrook,
            &[],
            setup,
            &world(&[], &[], &[])
        ));
        assert!(passage_usable(
            &moonbrook,
            &[],
            setup,
            &world(&[MOONBROOK], &[], &[])
        ));
        assert!(!passage_usable(
            &moonbrook,
            &[],
            setup,
            &world(&[MOONBROOK], &[], &[VANCLEEF])
        ));
    }

    #[test]
    fn an_unresolved_deed_never_waives_the_place() {
        let facts = world(&[], &[], &[VANCLEEF]);
        let tags = [DependsOn::Unresolved];

        assert!(!passage_usable(
            &[LinkTo::Place(MOONBROOK)],
            &tags,
            SetupFor::Nothing,
            &facts
        ));
    }

    #[test]
    fn common_knowledge_needs_nothing() {
        assert!(passage_usable(
            &[LinkTo::Common],
            &[],
            SetupFor::Nothing,
            &world(&[], &[], &[])
        ));
    }
}
