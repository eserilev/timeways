-- The laws of the gate of outcome passages (GAMEPLAY.md 5.10): a passage that tells a
-- deed of adventurers reaches a prompt only when the player did that deed, an unresolved
-- deed never does, a passage with no deed is not gated, and the gate never panics.
import Timeways.ThinLore

open Aeneas Aeneas.Std Result

namespace timeways_rules.outcomes

/-- The player did the deed that the passage depends on. `Nothing` and `Unresolved` name
no deed, so no player did them. -/
def did (depends_on : DependsOn) (facts : PlayerFacts) : Prop :=
  match depends_on with
  | DependsOn.Nothing => False
  | DependsOn.Unresolved => False
  | DependsOn.Foe foe => foe ∈ facts.defeated.val
  | DependsOn.Quest quest => quest ∈ facts.quests_done.val

/-- The rule, exactly: a passage with no deed passes, and any other passage passes exactly
when the player did its deed. -/
@[step]
theorem outcome_usable.spec (depends_on : DependsOn) (facts : PlayerFacts) :
    outcome_usable depends_on facts ⦃ b =>
      (b = true ↔ depends_on = DependsOn.Nothing ∨ did depends_on facts) ⦄ := by
  unfold outcome_usable
  cases depends_on with
  | Nothing => simp
  | Unresolved => simp [did]
  | Foe foe =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    simp [did, hb]
  | Quest quest =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    simp [did, hb]

/-- The gate never panics, and it always ends. -/
theorem the_gate_never_panics (depends_on : DependsOn) (facts : PlayerFacts) :
    ∃ b, outcome_usable depends_on facts = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (outcome_usable.spec depends_on facts)
  exact ⟨b, hb⟩

/-- A passage that tells a deed that the player did not do is refused, so it never
reaches a prompt. -/
theorem an_outcome_the_player_did_not_do_never_reaches_a_prompt
    (depends_on : DependsOn) (facts : PlayerFacts)
    (h_outcome : depends_on ≠ DependsOn.Nothing) (h_not_done : ¬ did depends_on facts) :
    outcome_usable depends_on facts = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (outcome_usable.spec depends_on facts)
  rw [hb]
  cases b
  · rfl
  · rcases hp.mp rfl with h | h
    · exact absurd h h_outcome
    · exact absurd h h_not_done

/-- An unresolved deed is refused, whatever the world of the player holds. -/
theorem an_unresolved_outcome_is_never_used (facts : PlayerFacts) :
    outcome_usable DependsOn.Unresolved facts = ok false := by
  rfl

/-- A passage with no deed passes this rule, whatever the world of the player holds. -/
theorem a_passage_with_no_outcome_is_not_gated_by_this_rule (facts : PlayerFacts) :
    outcome_usable DependsOn.Nothing facts = ok true := by
  rfl

/-- A deed that the player did lets its passage through. -/
theorem an_outcome_the_player_did_passes (depends_on : DependsOn) (facts : PlayerFacts)
    (h_done : did depends_on facts) :
    outcome_usable depends_on facts = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (outcome_usable.spec depends_on facts)
  rw [hb, hp.mpr (Or.inr h_done)]

end timeways_rules.outcomes
