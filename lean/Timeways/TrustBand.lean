-- The trust law: a change of trust always lands in the band -100..100,
-- for every trust held and every change, so for any sequence of changes.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.trust

theorem saturating_add_val (x y : I64) :
    (core.num.I64.saturating_add x y).val = max I64.min (min I64.max (x.val + y.val)) := by
  have hx : -9223372036854775808 ≤ x.val ∧ x.val ≤ 9223372036854775807 := by scalar_tac
  have hy : -9223372036854775808 ≤ y.val ∧ y.val ≤ 9223372036854775807 := by scalar_tac
  have hmin : I64.min = -9223372036854775808 := I64.min_eq
  have hmax : I64.max = 9223372036854775807 := I64.max_eq
  simp only [core.num.I64.saturating_add, IScalar.saturating_add]
  rw [IScalar.val, BitVec.toInt_ofInt]
  simp only [IScalar.min, IScalar.max, IScalarTy.numBits, hmin, hmax]
  norm_num
  rw [Int.bmod_def]
  split <;> omega

/-- The trust after one change is in the band, for every trust held and
every change. When the trust held is in the band, the change never moves
it against its sign. -/
@[step]
theorem next_trust.spec (held : Option I64) (by' : I64) :
    next_trust held by' ⦃ r => -100 ≤ r.val ∧ r.val ≤ 100 ∧
      (∀ h, held = some h → -100 ≤ h.val → h.val ≤ 100 →
        (0 ≤ by'.val → h.val ≤ r.val) ∧ (by'.val ≤ 0 → r.val ≤ h.val)) ⦄ := by
  unfold next_trust
  simp only [lift, bind_tc_ok, core.cmp.impls.OrdI64.clamp, IScalar.clamp]
  have h1 : MIN_TRUST.val = -100 := by simp [MIN_TRUST]
  have h2 : MAX_TRUST.val = 100 := by simp [MAX_TRUST]
  have hmin : I64.min = -9223372036854775808 := I64.min_eq
  have hmax : I64.max = 9223372036854775807 := I64.max_eq
  have hs := saturating_add_val (core.option.Option.unwrap_or held 0#i64) by'
  have hb : -9223372036854775808 ≤ by'.val ∧ by'.val ≤ 9223372036854775807 := by scalar_tac
  generalize core.num.I64.saturating_add (core.option.Option.unwrap_or held 0#i64) by' = s at hs ⊢
  simp only [hmin, hmax] at hs
  simp only [massert, h1, h2, Int.reduceLE, ↓reduceIte, bind_tc_ok]
  have hu : ∀ h : I64, held = some h → (core.option.Option.unwrap_or held 0#i64).val = h.val := by
    intro h hh; subst hh; rfl
  split
  · simp only [WP.spec_ok, h1]
    refine ⟨by omega, by omega, fun h hh _ _ => ?_⟩
    rw [hu h hh] at hs
    omega
  · split
    · simp only [WP.spec_ok, h2]
      refine ⟨by omega, by omega, fun h hh _ _ => ?_⟩
      rw [hu h hh] at hs
      omega
    · simp only [WP.spec_ok]
      refine ⟨by omega, by omega, fun h hh _ _ => ?_⟩
      rw [hu h hh] at hs
      omega

/-- The trust after each change of a sequence, applied one by one from the
trust held, as the story program applies them. -/
def trustsAfter (held : Option I64) : List I64 → Result (List I64)
  | [] => ok []
  | by' :: rest => do
    let t ← next_trust held by'
    let ts ← trustsAfter (some t) rest
    ok (t :: ts)

/-- For any trust held and any sequence of changes, the trust after each
change is in the band -100..100. -/
theorem trust_stays_between_minus_one_hundred_and_one_hundred
    (held : Option I64) (changes : List I64) :
    trustsAfter held changes ⦃ ts => ∀ t ∈ ts, -100 ≤ t.val ∧ t.val ≤ 100 ⦄ := by
  induction changes generalizing held with
  | nil => simp [trustsAfter]
  | cons b rest ih =>
    unfold trustsAfter
    step as ⟨t, ht1, ht2, _⟩
    step as ⟨ts, hts⟩
    intro x hx
    simp only [List.mem_cons] at hx
    rcases hx with hx | hx
    · subst hx; exact ⟨ht1, ht2⟩
    · exact hts x hx

end timeways_rules.trust
