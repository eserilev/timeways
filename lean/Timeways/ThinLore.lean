-- The laws of silence when the lore is thin (GAMEPLAY.md 3.2): a deed with no
-- lore of its own is silent, an arrival never is, and the rule never panics.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.thin_lore

@[step]
theorem holds_loop.spec (ids : Slice U32) (id : U32) (i : Usize)
    (h : ∀ j (hj : j < ids.val.length), j < i.val → ids.val[j] ≠ id) :
    holds_loop ids id i ⦃ b => (b = true ↔ id ∈ ids.val) ⦄ := by
  unfold holds_loop
  dsimp only
  split
  · step as ⟨m, hm⟩
    split
    · rename_i he
      subst he
      simp only [WP.spec_ok, true_iff]
      rw [hm]
      exact List.getElem_mem _
    · rename_i he
      step as ⟨i1, hi1⟩
      apply holds_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje; rw [← hm]; exact he
      · exact h j hj (by scalar_tac)
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro hm
    obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hm
    exact h j hj (by scalar_tac) hjeq
termination_by ids.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem holds.spec (ids : Slice U32) (id : U32) :
    holds ids id ⦃ b => (b = true ↔ id ∈ ids.val) ⦄ := by
  unfold holds
  exact holds_loop.spec ids id 0#usize (fun j _ hj => by simp at hj)

@[step]
theorem shares_one_loop.spec (subjects lore : Slice U32) (i : Usize)
    (h : ∀ j (hj : j < lore.val.length), j < i.val → lore.val[j] ∉ subjects.val) :
    shares_one_loop subjects lore i ⦃ b => (b = true ↔ ∃ x ∈ lore.val, x ∈ subjects.val) ⦄ := by
  unfold shares_one_loop
  dsimp only
  split
  · step as ⟨m, hm⟩
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      simp only [WP.spec_ok, true_iff]
      refine ⟨m, ?_, hb.mp hbt⟩
      rw [hm]
      exact List.getElem_mem _
    · rename_i hbt
      step as ⟨i1, hi1⟩
      apply shares_one_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje
        rw [← hm]
        intro hin
        exact hbt (hb.mpr hin)
      · exact h j hj (by scalar_tac)
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_exists, not_and]
    intro x hx hxs
    obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hx
    exact h j hj (by scalar_tac) (hjeq ▸ hxs)
termination_by lore.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem shares_one.spec (subjects lore : Slice U32) :
    shares_one subjects lore ⦃ b => (b = true ↔ ∃ x ∈ lore.val, x ∈ subjects.val) ⦄ := by
  unfold shares_one
  exact shares_one_loop.spec subjects lore 0#usize (fun j _ hj => by simp at hj)

/-- The rule, exactly: only a deed can be silent, and a deed is silent exactly when no
subject of its lore is a subject of the moment. -/
@[step]
theorem is_silent.spec (kind : MomentKind) (subjects lore : Slice U32) :
    is_silent kind subjects lore ⦃ b =>
      (b = true ↔ kind = MomentKind.Deed ∧ ¬ ∃ x ∈ lore.val, x ∈ subjects.val) ⦄ := by
  unfold is_silent
  cases kind with
  | Arrival => simp
  | Flavor => simp
  | Deed =>
    step as ⟨b, hb⟩
    cases b <;> simp_all

/-- The rule never panics, and it always ends. -/
theorem the_rule_never_panics (kind : MomentKind) (subjects lore : Slice U32) :
    ∃ b, is_silent kind subjects lore = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (is_silent.spec kind subjects lore)
  exact ⟨b, hb⟩

/-- A deed with no passage whose subject is among its subjects is silent. No passage at
all is the empty `lore`. -/
theorem a_deed_with_no_lore_of_its_own_is_silent (subjects lore : Slice U32)
    (h : ∀ x ∈ lore.val, x ∉ subjects.val) :
    is_silent MomentKind.Deed subjects lore = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (is_silent.spec MomentKind.Deed subjects lore)
  have : b = true := hp.mpr ⟨rfl, fun ⟨x, hx, hxs⟩ => h x hx hxs⟩
  rw [hb, this]

/-- A deed with a passage about one of its subjects speaks. -/
theorem a_deed_with_lore_of_its_own_speaks (subjects lore : Slice U32)
    (h : ∃ x ∈ lore.val, x ∈ subjects.val) :
    is_silent MomentKind.Deed subjects lore = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (is_silent.spec MomentKind.Deed subjects lore)
  rw [hb]
  cases b
  · rfl
  · exact absurd h (hp.mp rfl).2

/-- An arrival is never silenced by this rule, whatever its lore. -/
theorem an_arrival_is_never_silenced_by_this_rule (subjects lore : Slice U32) :
    is_silent MomentKind.Arrival subjects lore = ok false := by
  rfl

end timeways_rules.thin_lore
