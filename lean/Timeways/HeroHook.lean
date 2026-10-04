-- The hook laws: a hook call never picks a field out of range, and each
-- filled field of the hero sheet takes its turn.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.hero_hook

@[step]
theorem applies.spec (count : U64) :
    applies count ⦃ b => (b = true ↔ count.val % 3 = 2) ⦄ := by
  unfold applies
  have h3 : HOOK_EVERY.val = 3 := by simp [HOOK_EVERY]
  step*

/-- The pick of a call: none for a call with no hook or a sheet with no
filled field, and else the turn of the call modulo the filled fields. -/
@[step]
theorem pick.spec (filled : Usize) (count : U64) :
    pick filled count ⦃ r => (r = none ↔ ¬ (count.val % 3 = 2 ∧ 0 < filled.val)) ∧
      ∀ i, r = some i → i.val = (count.val / 3) % filled.val ⦄ := by
  unfold pick
  have h3 : HOOK_EVERY.val = 3 := by simp [HOOK_EVERY]
  have hu : filled.val ≤ U64.max := by
    have := filled.hBounds
    rcases Usize.bounds_eq with h | h <;> scalar_tac
  have hf : (UScalar.cast UScalarTy.U64 filled).val = filled.val :=
    UScalar.cast_val_mod_pow_of_inBounds_eq _ _ (by scalar_tac)
  step*
  · have hlt : i1.val < filled.val := by
      rw [i1_post, i_post, hf]
      exact Nat.mod_lt _ (by scalar_tac)
    have hi : index.val = i1.val := by
      rw [index_post]
      exact UScalar.cast_val_mod_pow_of_inBounds_eq _ _ (by scalar_tac)
    refine ⟨by simp; scalar_tac, ?_⟩
    intro j hj
    cases hj
    rw [hi, i1_post, turn_post, i_post, hf, h3]

/-- A hook call never picks a field out of range. -/
theorem pick_is_never_out_of_range (filled : Usize) (count : U64) :
    pick filled count ⦃ r => ∀ i, r = some i → i.val < filled.val ⦄ := by
  apply WP.spec_mono (pick.spec filled count)
  rintro r ⟨hnone, hval⟩ i hi
  rw [hval i hi]
  have hpos : 0 < filled.val := by
    by_contra h0
    have hn := hnone.mpr (fun h => h0 h.2)
    rw [hi] at hn
    cases hn
  exact Nat.mod_lt _ hpos

/-- The k-th hook call of a row of `filled` calls picks k modulo `filled`. -/
theorem turn_of (n : Nat) (k : Nat) : (3 * k + 2) / 3 % n = k % n := by
  congr 1
  omega

/-- Every filled field takes its turn: among any `filled` hook calls in a
row, each filled field is picked. The hook calls are the calls with a count
of 3k + 2, and the row starts at any k0, while the counts fit a `u64`. -/
theorem every_filled_field_takes_its_turn (filled : Usize) (k0 : Nat) (hn : 0 < filled.val)
    (hfit : 3 * (k0 + filled.val) ≤ U64.max) (i : Nat) (hi : i < filled.val) :
    ∃ (k : Nat) (c : U64), k0 ≤ k ∧ k < k0 + filled.val ∧ c.val = 3 * k + 2 ∧
      pick filled c ⦃ r => r.map (·.val) = some i ⦄ := by
  have hdm := Nat.div_add_mod k0 filled.val
  have hm := Nat.mod_lt k0 hn
  obtain ⟨k, hk1, hk2, hki⟩ : ∃ k, k0 ≤ k ∧ k < k0 + filled.val ∧ k % filled.val = i := by
    by_cases hle : k0 % filled.val ≤ i
    · refine ⟨filled.val * (k0 / filled.val) + i, by omega, by omega, ?_⟩
      rw [Nat.mul_add_mod, Nat.mod_eq_of_lt hi]
    · refine ⟨filled.val * (k0 / filled.val + 1) + i, ?_, ?_, ?_⟩
      · rw [Nat.mul_add]; omega
      · rw [Nat.mul_add]; omega
      · rw [Nat.mul_add_mod, Nat.mod_eq_of_lt hi]
  have hc : 3 * k + 2 ≤ U64.max := by omega
  refine ⟨k, U64.ofNat (3 * k + 2) (by scalar_tac), hk1, hk2, by simp, ?_⟩
  apply WP.spec_mono (pick.spec filled _)
  rintro r ⟨hnone, hval⟩
  have hsome : r ≠ none := by
    intro h
    exact (hnone.mp h) ⟨by simp, hn⟩
  obtain ⟨j, rfl⟩ := Option.ne_none_iff_exists'.mp hsome
  have hv : (U64.ofNat (3 * k + 2) (by scalar_tac)).val = 3 * k + 2 := by simp
  rw [Option.map_some, hval j rfl, hv, turn_of, hki]

end timeways_rules.hero_hook
