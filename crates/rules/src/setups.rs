//! The gate of setup passages (GAMEPLAY.md 5.10). A setup tells who wants a deed done in a
//! dungeon or a raid: "Gryan Stoutmantle sent adventurers to kill the Defias kingpin." It is the
//! hook before the deed, so it goes stale once the player did that deed. The gate is the
//! inverse of the gate of outcome passages (`outcomes`). Lean proves its laws
//! (lean/Timeways/Setups.lean).

use crate::outcomes::PlayerFacts;
use crate::thin_lore::holds;

/// The deed that a passage of the pack sets up, as the builder tagged it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SetupFor {
    /// The passage sets up no deed.
    Nothing,
    /// The id of the foe that the setup wants defeated.
    Foe(u32),
    /// The id of the title of the quest that the setup leads to.
    Quest(u32),
}

/// True when a passage may reach a prompt by this rule. A passage with no setup always
/// may. A setup may only while the player has not done its deed.
#[cfg_attr(charon, verify::start_from)]
#[must_use]
pub fn setup_usable(setup_for: SetupFor, facts: &PlayerFacts) -> bool {
    match setup_for {
        SetupFor::Nothing => true,
        SetupFor::Foe(foe) => !holds(&facts.defeated, foe),
        SetupFor::Quest(quest) => !holds(&facts.quests_done, quest),
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
    fn a_passage_with_no_setup_passes_with_any_facts() {
        assert!(setup_usable(SetupFor::Nothing, &facts(&[], &[])));
        assert!(setup_usable(SetupFor::Nothing, &facts(&[1, 2], &[1, 2])));
    }

    #[test]
    fn a_setup_for_a_foe_goes_stale_after_its_defeat() {
        assert!(setup_usable(SetupFor::Foe(7), &facts(&[1], &[7])));
        assert!(!setup_usable(SetupFor::Foe(7), &facts(&[1, 7], &[])));
    }

    #[test]
    fn a_setup_for_a_quest_goes_stale_after_its_turn_in() {
        assert!(setup_usable(SetupFor::Quest(3), &facts(&[3], &[])));
        assert!(!setup_usable(SetupFor::Quest(3), &facts(&[], &[3])));
    }
}
