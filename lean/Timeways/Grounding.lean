-- The laws of the check against invented names (GAMEPLAY.md 3.2.1): a text with a
-- name that the prompt did not give is refused, a text whose names are all given passes
-- this check, and the check never panics.
import Timeways.ThinLore

open Aeneas Aeneas.Std Result

namespace timeways_rules.grounding

@[step]
theorem grounded_loop.spec (answer given : Slice U32) (i : Usize)
    (h : ∀ j (hj : j < answer.val.length), j < i.val → answer.val[j] ∈ given.val) :
    grounded_loop answer given i ⦃ b => (b = true ↔ ∀ x ∈ answer.val, x ∈ given.val) ⦄ := by
  unfold grounded_loop
  dsimp only
  split
  · step as ⟨m, hm⟩
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      step as ⟨i1, hi1⟩
      apply grounded_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje
        rw [← hm]
        exact hb.mp hbt
      · exact h j hj (by scalar_tac)
    · rename_i hbt
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
      refine ⟨m, ?_, fun hin => hbt (hb.mpr hin)⟩
      rw [hm]
      exact List.getElem_mem _
  · simp only [WP.spec_ok, true_iff]
    intro x hx
    obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hx
    exact hjeq ▸ h j hj (by scalar_tac)
termination_by answer.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The rule, exactly: a text passes exactly when each of its names is given. -/
@[step]
theorem grounded.spec (answer given : Slice U32) :
    grounded answer given ⦃ b => (b = true ↔ ∀ x ∈ answer.val, x ∈ given.val) ⦄ := by
  unfold grounded
  exact grounded_loop.spec answer given 0#usize (fun j _ hj => by simp at hj)

/-- A name is given exactly when the prompt holds it. -/
@[step]
theorem is_given.spec (n : U32) (given : Slice U32) :
    is_given n given ⦃ b => (b = true ↔ n ∈ given.val) ⦄ := by
  unfold is_given
  exact thin_lore.holds.spec given n

/-- The check never panics, and it always ends. -/
theorem the_grounding_check_never_panics (answer given : Slice U32) :
    ∃ b, grounded answer given = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (grounded.spec answer given)
  exact ⟨b, hb⟩

/-- A text with one name that the prompt did not give is refused. -/
theorem an_ungrounded_name_is_refused (answer given : Slice U32) (n : U32)
    (h_in : n ∈ answer.val) (h_new : n ∉ given.val) :
    grounded answer given = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (grounded.spec answer given)
  rw [hb]
  cases b
  · rfl
  · exact absurd ((hp.mp rfl) n h_in) h_new

/-- A text whose names are all given passes this check. Other checks can still refuse it. -/
theorem a_text_whose_names_are_all_given_passes_this_check (answer given : Slice U32)
    (h : ∀ x ∈ answer.val, x ∈ given.val) :
    grounded answer given = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (grounded.spec answer given)
  rw [hb, hp.mpr h]

/-- `is_given` never panics either, so a refusal can name each ungrounded name. -/
theorem is_given_never_panics (n : U32) (given : Slice U32) :
    ∃ b, is_given n given = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (is_given.spec n given)
  exact ⟨b, hb⟩

end timeways_rules.grounding
