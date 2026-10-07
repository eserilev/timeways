//! The gate of outcome passages (GAMEPLAY.md 5.10). The wiki often tells the deed of a
//! quest as history: "adventurers killed the Defias kingpin". Such a passage reaches a prompt
//! only when the world of the player shows that the player did that deed. The story
//! program gives each name an id, so the rule reads ids, never strings. Lean proves its
//! laws (lean/Timeways/Outcomes.lean).

use crate::thin_lore::holds;

/// What a passage of the pack depends on, as the builder tagged it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DependsOn {
    /// The passage tells no deed of adventurers.
    Nothing,
    /// The passage tells a deed, and the builder tied it to no foe and no quest.
    Unresolved,
    /// The id of the foe that the adventurers defeated.
    Foe(u32),
    /// The id of the title of the quest that ends the deed.
    Quest(u32),
}

/// The deeds of the player, as ids: the foes that the player defeated, and the quests of
/// the game that the player turned in.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PlayerFacts {
    pub defeated: Vec<u32>,
    pub quests_done: Vec<u32>,
}

/// True when a passage may reach a prompt by this rule. A passage with no deed always may.
/// An unresolved deed never may.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn outcome_usable(depends_on: DependsOn, facts: &PlayerFacts) -> bool {
    match depends_on {
        DependsOn::Nothing => true,
        DependsOn::Unresolved => false,
        DependsOn::Foe(foe) => holds(&facts.defeated, foe),
        DependsOn::Quest(quest) => holds(&facts.quests_done, quest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(defeated: &[u32], quests_done: &[u32]) -> PlayerFacts {
        PlayerFacts {
            defeated: defeated.to_vec(),
            quests_done: quests_done.to_vec(),
        }
    }

    #[test]
    fn a_passage_with_no_deed_passes_with_no_facts() {
        assert!(outcome_usable(DependsOn::Nothing, &facts(&[], &[])));
    }

    #[test]
    fn an_unresolved_deed_never_passes() {
        assert!(!outcome_usable(
            DependsOn::Unresolved,
            &facts(&[1, 2], &[1, 2])
        ));
    }

    #[test]
    fn a_foe_passes_only_after_its_defeat() {
        assert!(!outcome_usable(DependsOn::Foe(7), &facts(&[1], &[7])));
        assert!(outcome_usable(DependsOn::Foe(7), &facts(&[1, 7], &[])));
    }

    #[test]
    fn a_quest_passes_only_after_its_turn_in() {
        assert!(!outcome_usable(DependsOn::Quest(3), &facts(&[3], &[])));
        assert!(outcome_usable(DependsOn::Quest(3), &facts(&[], &[3])));
    }
}
