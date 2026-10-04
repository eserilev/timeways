-- The questions about one quest: is a step done, is it open, when did
-- it open, and when is a wait over. Each function of the Rust code
-- gives the value of a pure function here, and never panics.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.quest_log

/-! ## The pure model -/

/-- Step `i` has a done time. -/
def DoneAt (p : Progress) (i : Nat) : Prop := ∃ x, p.done_times.val[i]? = some (some x)

/-- The first step of the stage of step `j`: the start of the any-order
set that holds it, or `j` itself. -/
def stageStartOf (ao : Option Span) (j : Nat) : Nat :=
  match ao with
  | none => j
  | some s => if s.first.val ≤ j ∧ j ≤ s.last.val then s.first.val else j

theorem stageStartOf_le (ao : Option Span) (j : Nat) : stageStartOf ao j ≤ j := by
  unfold stageStartOf; split
  · simp
  · split <;> omega

/-- The larger of two optional times, as `later` in the Rust code. -/
def laterOf : Option Nat → Option Nat → Option Nat
  | none, b => b
  | some a, none => some a
  | some a, some b => some (max a b)

/-- The latest done time in a list of done times: the max of the times
that are there. -/
def latest (l : List (Option U64)) : Option Nat :=
  l.foldl (fun acc x => laterOf acc (x.map (·.val))) none

/-- When step `j` opened, as the property test `a_wait_never_ends_early`
reads it: the latest done time of the steps before it, or the accept. -/
def openedOf (p : Progress) (j : Nat) : Option Nat :=
  (latest (p.done_times.val.take j)).or (p.accepted_at.map (·.val))

/-! ## The specs -/

@[step]
theorem Progress.is_done.spec (p : Progress) (i : Usize) :
    p.is_done i ⦃ b => (b = true ↔ DoneAt p i.val) ⦄ := by
  unfold Progress.is_done DoneAt
  step*
  · have h : i.val < p.done_times.val.length := by scalar_tac
    simp [List.getElem?_eq_getElem h, o_post, Option.isSome_iff_exists]
  · have h : p.done_times.val.length ≤ i.val := by scalar_tac
    simp [List.getElem?_eq_none h]

@[step]
theorem Status.eq.spec (a b : Status) :
    Status.Insts.CoreCmpPartialEqStatus.eq a b ⦃ r => (r = true ↔ a = b) ⦄ := by
  unfold Status.Insts.CoreCmpPartialEqStatus.eq
  cases a <;> cases b <;> simp [read_discriminant, Status.read_discriminant]

@[step]
theorem Progress.stage_start.spec (p : Progress) (j : Usize) :
    p.stage_start j ⦃ r => r.val = stageStartOf p.any_order j.val ⦄ := by
  unfold Progress.stage_start stageStartOf Span.contains
  rcases h : p.any_order with _ | s
  · simp
  · simp only
    by_cases h1 : s.first ≤ j <;> by_cases h2 : j ≤ s.last <;> simp [h1, h2] <;> scalar_tac

@[step]
theorem Progress.all_done_before_loop.spec (p : Progress) (e i : Usize)
    (h : ∀ k < i.val, DoneAt p k) :
    p.all_done_before_loop e i ⦃ b => (b = true ↔ ∀ k < e.val, DoneAt p k) ⦄ := by
  unfold Progress.all_done_before_loop
  split
  · step as ⟨b, hb⟩
    split
    · step as ⟨i1, hi1⟩
      step as ⟨b1, hb1⟩
      exact hb1
    · simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_forall]
      exact ⟨i.val, by scalar_tac, fun hd => by simp_all⟩
  · simp only [WP.spec_ok, true_iff]
    intro k hk
    exact h k (by scalar_tac)
termination_by e.val - i.val
decreasing_by scalar_decr_tac

/-- An open step: the quest is accepted, the step is in the quest and not
done, and every step before its stage is done. -/
def OpenAt (p : Progress) (j : Nat) : Prop :=
  p.status = Status.Accepted ∧ j < p.steps.length ∧ ¬ DoneAt p j ∧
    ∀ k < stageStartOf p.any_order j, DoneAt p k

@[step]
theorem Progress.is_open.spec (p : Progress) (j : Usize) :
    p.is_open j ⦃ b => (b = true ↔ OpenAt p j.val) ⦄ := by
  unfold Progress.is_open Progress.all_done_before OpenAt
  step*

@[step]
theorem later.spec (a b : Option U64) :
    later a b ⦃ r => r.map (·.val) = laterOf (a.map (·.val)) (b.map (·.val)) ⦄ := by
  unfold later
  rcases a with _ | x <;> rcases b with _ | y
  · simp [laterOf]
  · simp [laterOf]
  · simp [laterOf]
  · simp only [laterOf, Option.map_some]
    split
    · simp only [WP.spec_ok, Option.map_some, Option.some.injEq]; scalar_tac
    · simp only [WP.spec_ok, Option.map_some, Option.some.injEq]; scalar_tac

theorem latest_take_succ (l : List (Option U64)) (i : Nat) (h : i < l.length) :
    latest (l.take (i + 1)) = laterOf (latest (l.take i)) (l[i].map (·.val)) := by
  unfold latest
  rw [List.take_add_one, List.foldl_append, List.getElem?_eq_getElem h]
  rfl

@[step]
theorem Progress.last_done_before_loop.spec (p : Progress) (e : Usize) (l : Option U64)
    (i : Usize) (h1 : i.val ≤ e.val) (h2 : i.val ≤ p.done_times.length)
    (h3 : l.map (·.val) = latest (p.done_times.val.take i.val)) :
    p.last_done_before_loop e l i ⦃ r =>
      r.map (·.val) = latest (p.done_times.val.take e.val) ⦄ := by
  unfold Progress.last_done_before_loop
  split
  · dsimp only
    split
    · step as ⟨o, ho⟩
      step as ⟨l1, hl1⟩
      step as ⟨i1, hi1⟩
      step as ⟨r, hr⟩
      · rw [hl1, h3, ho, hi1, latest_take_succ _ _ (by scalar_tac)]
      · exact hr
    · simp only [WP.spec_ok, h3]
      have : p.done_times.val.length ≤ e.val := by scalar_tac
      rw [List.take_of_length_le (by scalar_tac), List.take_of_length_le this]
  · simp only [WP.spec_ok, h3]
    have : i.val = e.val := by scalar_tac
    rw [this]
termination_by e.val - i.val
decreasing_by scalar_decr_tac

/-- When an open step opened: the latest done time before its stage, or
the accept. -/
def openedAtOf (p : Progress) (j : Nat) : Option Nat :=
  (latest (p.done_times.val.take (stageStartOf p.any_order j))).or (p.accepted_at.map (·.val))

@[step]
theorem Progress.opened_at.spec (p : Progress) (j : Usize) :
    p.opened_at j ⦃ r => (OpenAt p j.val → r.map (·.val) = openedAtOf p j.val) ∧
      (¬ OpenAt p j.val → r = none) ⦄ := by
  unfold Progress.opened_at Progress.last_done_before openedAtOf
  step as ⟨b, hb⟩
  split
  · step as ⟨i, hi⟩
    step as ⟨o, ho⟩
    · simp [latest]
    rcases o with _ | x
    · simp only [WP.spec_ok]
      refine ⟨fun _ => ?_, fun hn => absurd (hb.mp ‹_›) hn⟩
      rw [← hi, ← ho]
      rfl
    · simp only [WP.spec_ok]
      refine ⟨fun _ => ?_, fun hn => absurd (hb.mp ‹_›) hn⟩
      rw [← hi, ← ho]
      rfl
  · simp only [WP.spec_ok]
    exact ⟨fun ho => absurd (hb.mpr ho) ‹_›, fun _ => trivial⟩

theorem saturating_add_val (x y : U64) :
    (core.num.U64.saturating_add x y).val = min (x.val + y.val) U64.max := by
  simp only [core.num.U64.saturating_add, UScalar.saturating_add, UScalar.val,
    BitVec.toNat_ofNat, UScalar.max_UScalarTy_U64_eq]
  rw [Nat.min_comm]
  apply Nat.mod_eq_of_lt
  have : min (x.bv.toNat + y.bv.toNat) U64.max ≤ U64.max := Nat.min_le_right _ _
  simp only [U64.max_eq] at this ⊢
  simp only [UScalarTy.numBits]
  omega

/-- The seconds of a wait of `days` days. -/
def waitSeconds (days : U8) : Nat := days.val * 86400

@[step]
theorem DAY_SECONDS.spec : DAY_SECONDS ⦃ r => r.val = 86400 ⦄ := by
  unfold DAY_SECONDS
  step*

/-- An open wait is over at its opening time plus its days. The add
saturates at `u64::MAX`. -/
@[step]
theorem Progress.ready_at.spec (p : Progress) (j : Usize) :
    p.ready_at j ⦃ r => ∀ days, p.steps.val[j.val]? = some (Goal.Wait days) → OpenAt p j.val →
      r.map (·.val) = (openedAtOf p j.val).map (fun o => min (o + waitSeconds days) U64.max) ⦄ := by
  unfold Progress.ready_at
  step*
  · intro days _ hopen
    exact absurd hopen.2.1 (by scalar_tac)
  · intro d _ hopen
    rw [← o_post hopen, ‹o = none›]
    rfl
  · simp only [lift, bind_tc_ok]
    step as ⟨sec, hsec⟩
    step as ⟨w, hw⟩
    intro d hd hopen
    have hdd : d = days := by
      rw [List.getElem?_eq_getElem (by scalar_tac)] at hd
      simp only [Option.some.injEq] at hd
      rw [← g_post, ‹g = Goal.Wait days›] at hd
      cases hd
      rfl
    subst hdd
    rw [← o_post hopen, ‹o = some opened›]
    simp only [Option.map_some, saturating_add_val, hw, hsec,
      core.convert.num.FromU64U8.from_val_eq, waitSeconds]
  · intro d hd _
    rw [List.getElem?_eq_getElem (by scalar_tac)] at hd
    simp only [Option.some.injEq] at hd
    rw [← g_post, ‹g = Goal.Kill _›] at hd
    cases hd
  · intro d hd _
    rw [List.getElem?_eq_getElem (by scalar_tac)] at hd
    simp only [Option.some.injEq] at hd
    rw [← g_post, ‹g = Goal.Other›] at hd
    cases hd

@[step]
theorem Progress.steps_done_loop.spec (p : Progress) (count i : Usize) (h : count.val ≤ i.val) :
    p.steps_done_loop count i ⦃ _ => True ⦄ := by
  unfold Progress.steps_done_loop
  step*
  split
  · step*
  · step*
termination_by p.done_times.length - i.val
decreasing_by all_goals scalar_decr_tac

end timeways_rules.quest_log