-- The budget law: for any sequence of line times, the narrator never
-- speaks four times in one hour.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.budget

/-- The time of a line taken `back` lines ago: 1 is the last one. -/
def slot (taken : List U64) (back : Nat) : Option U64 :=
  if back ≤ taken.length then taken[taken.length - back]? else none

/-- The budget holds the times of the last three lines taken, oldest first. -/
def Holds (b : Budget) (taken : List U64) : Prop :=
  b.spoken.val = [slot taken 3, slot taken 2, slot taken 1]

/-- Any four lines in a row span an hour or more. -/
def Spread (taken : List U64) : Prop :=
  ∀ i (x y : U64), taken[i]? = some x → taken[i + 3]? = some y → x.val + 3600 ≤ y.val

theorem saturating_sub_val (x y : U64) :
    (core.num.U64.saturating_sub x y).val = x.val - y.val := by
  simp only [core.num.U64.saturating_sub, UScalar.saturating_sub, UScalar.val,
    BitVec.toNat_ofNat, Nat.zero_max]
  apply Nat.mod_eq_of_lt
  have := x.bv.isLt
  simp only [UScalarTy.numBits] at this ⊢
  omega

theorem slot_last (l : List U64) (a : U64) : slot (l ++ [a]) 1 = some a := by
  simp [slot]

theorem slot_append (l : List U64) (a : U64) (back : Nat) (h : 2 ≤ back) :
    slot (l ++ [a]) back = slot l (back - 1) := by
  unfold slot
  simp only [List.length_append, List.length_singleton]
  by_cases hb : back ≤ l.length + 1
  · rw [if_pos hb, if_pos (by omega), List.getElem?_append_left (by omega)]
    congr 1
    omega
  · rw [if_neg hb, if_neg (by omega)]

theorem spread_append (l : List U64) (a : U64) (h : Spread l)
    (ha : ∀ o, slot l 3 = some o → o.val + 3600 ≤ a.val) : Spread (l ++ [a]) := by
  intro i x y hx hy
  by_cases hi : i + 3 < l.length
  · rw [List.getElem?_append_left (by omega)] at hx hy
    exact h i x y hx hy
  · have hlt : i + 3 < l.length + 1 := by
      by_contra hc
      rw [List.getElem?_eq_none (by simp; omega)] at hy
      cases hy
    have hil : i < l.length := by omega
    rw [List.getElem?_append_left hil] at hx
    rw [List.getElem?_append_right (by omega)] at hy
    have hz : i + 3 - l.length = 0 := by omega
    rw [hz] at hy
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hy
    subst hy
    apply ha x
    unfold slot
    rw [if_pos (by omega)]
    have : l.length - 3 = i := by omega
    rw [this, hx]

theorem holds_after (b : Budget) (taken : List U64) (a : U64) (o o1 : Option U64)
    (h : Holds b taken) (ho : o = b.spoken.val[1]) (ho1 : o1 = b.spoken.val[2])
    (hl : [o, o1, some a].length = (3#usize).val) :
    Holds ⟨Array.make 3#usize [o, o1, some a] hl⟩ (taken ++ [a]) := by
  unfold Holds at h ⊢
  simp only [h] at ho ho1
  simp only [Array.make, ho, ho1]
  rw [slot_append _ _ 3 (by omega), slot_append _ _ 2 (by omega), slot_last]
  simp

@[step]
theorem Budget.take.spec (b : Budget) (a : U64) (taken : List U64) (h : Holds b taken)
    (hs : Spread taken) :
    b.take a ⦃ ok b' => Holds b' (if ok then taken ++ [a] else taken) ∧
      Spread (if ok then taken ++ [a] else taken) ⦄ := by
  unfold Budget.take
  step*
  have h0 : oldest = slot taken 3 := by
    rw [oldest_post]; unfold Holds at h; simp [h]
  rcases oldest with _ | first
  · simp only [bind_tc_ok, Bool.false_eq_true, if_false]
    step*
    refine ⟨holds_after b taken a _ _ h o_post o1_post _, spread_append taken a hs ?_⟩
    intro x hx
    rw [← h0] at hx
    cases hx
  · simp only [lift, bind_tc_ok]
    split
    next => simp only [WP.spec_ok]; exact ⟨h, hs⟩
    next hlate =>
      step*
      refine ⟨holds_after b taken a _ _ h o_post o1_post _, spread_append taken a hs ?_⟩
      intro x hx
      rw [← h0] at hx
      cases hx
      have hh : HOUR.val = 3600 := by simp [HOUR]
      simp only [decide_eq_true_eq] at hlate
      have := saturating_sub_val a first
      scalar_tac

/-- The times of the lines that the narrator speaks, when a line asks to
come at each time of the list, in order. -/
def linesSpoken (b : Budget) : List U64 → Result (List U64)
  | [] => ok []
  | a :: rest => do
    let (spoke, b') ← b.take a
    let later ← linesSpoken b' rest
    ok (if spoke then a :: later else later)

theorem linesSpoken.spec (b : Budget) (taken : List U64) (times : List U64)
    (h : Holds b taken) (hs : Spread taken) :
    linesSpoken b times ⦃ ts => Spread (taken ++ ts) ⦄ := by
  induction times generalizing b taken with
  | nil => simpa [linesSpoken] using hs
  | cons a rest ih =>
    unfold linesSpoken
    step as ⟨spoke, b', h1, h2⟩
    step as ⟨later, hl⟩
    cases spoke
    · simpa using hl
    · simpa using hl

/-- The budget of a new story: no line yet. `Budget::default()` in Rust. -/
def fresh : Budget := ⟨Array.make 3#usize [none, none, none]⟩

/-- For any sequence of times, in any order, any four lines that the
narrator speaks in a row span an hour or more. So it never speaks four
times in one hour. -/
theorem the_narrator_never_speaks_four_times_in_one_hour (times : List U64) :
    linesSpoken fresh times ⦃ ts => Spread ts ⦄ := by
  have h0 : Holds fresh [] := by simp [Holds, fresh, Array.make, slot]
  have hs : Spread [] := by intro i x y hx; simp at hx
  simpa using linesSpoken.spec fresh [] times h0 hs

end timeways_rules.budget
