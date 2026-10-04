-- The laws of the quest log. `quest_log` folds the lines of the quest
-- file into quests. Each quest keeps the invariant `Good` after each
-- line, so every law below holds for every log, of any length.
import Timeways.QuestQueries

open Aeneas Aeneas.Std Result

namespace timeways_rules.quest_log

/-! ## The invariant of one quest -/

/-- A step that is done has every step before its stage done. -/
def Closed (ao : Option Span) (l : List (Option U64)) : Prop :=
  ∀ j x, l[j]? = some (some x) →
    ∀ k < stageStartOf ao j, ∃ y, l[k]? = some (some y)

/-- No any-order set holds a wait. -/
def NoWaitInSet (p : Progress) : Prop :=
  ∀ s, p.any_order = some s → ∀ j d, s.first.val ≤ j → j ≤ s.last.val →
    p.steps.val[j]? ≠ some (Goal.Wait d)

/-- The most kills that a step counts: its count for a kill step, and 0
for every other step. -/
def killLimit : Goal → Nat
  | .Kill count => count.val
  | _ => 0

def KillsFit (p : Progress) : Prop :=
  ∀ (j : Nat) (k : U8) g, p.kills.val[j]? = some k → p.steps.val[j]? = some g → k.val ≤ killLimit g

/-- A done wait was done no sooner than its opening time plus its days.
The add saturates at `u64::MAX`, as in the Rust code. -/
def WaitsHeld (p : Progress) : Prop :=
  ∀ (j : Nat) days (d : U64), p.steps.val[j]? = some (Goal.Wait days) →
    p.done_times.val[j]? = some (some d) →
    ∃ o, openedOf p j = some o ∧ min (o + waitSeconds days) U64.max ≤ d.val

/-- An offer that waits for an answer has no done step and no kill. -/
def OfferClean (p : Progress) : Prop :=
  p.status = Status.Offered →
    (∀ x ∈ p.done_times.val, x = none) ∧ ∀ k ∈ p.kills.val, k.val = 0

/-- A quest that you hold, or that has a done step, has an accept time. -/
def HasAccept (p : Progress) : Prop :=
  (p.status = Status.Accepted ∨ ∃ j, DoneAt p j) → p.accepted_at.isSome

/-- A quest is done exactly when it has a step and every step is done. -/
def DoneExact (p : Progress) : Prop :=
  p.status = Status.Done ↔ (0 < p.steps.length ∧ ∀ x ∈ p.done_times.val, x.isSome)

structure Good (p : Progress) : Prop where
  done_len : p.done_times.length = p.steps.length
  kills_len : p.kills.length = p.steps.length
  closed : Closed p.any_order p.done_times.val
  no_wait_in_set : NoWaitInSet p
  kills_fit : KillsFit p
  waits_held : WaitsHeld p
  offer_clean : OfferClean p
  has_accept : HasAccept p
  done_exact : DoneExact p

/-- Every quest of the list keeps the invariant. -/
def AllGood (qs : List Quest) : Prop := ∀ q ∈ qs, Good q.progress

/-- A kill counts only for a step that is open. -/
def KillsCounted (p p' : Progress) : Prop :=
  ∀ (k : Nat) (a b : U8), p.kills.val[k]? = some a → p'.kills.val[k]? = some b →
    a.val < b.val → OpenAt p k

theorem killsCounted_of_eq {p p' : Progress} (h : p'.kills = p.kills) : KillsCounted p p' := by
  intro k a b ha hb hab
  rw [h, ha] at hb
  cases hb
  omega

/-! ## A change of status -/

/-- A status that closes the quest, or declines the offer, keeps the
invariant. -/
theorem good_status (p : Progress) (s : Status) (h1 : s ≠ Status.Offered)
    (h2 : s ≠ Status.Accepted) (h3 : s ≠ Status.Done) (h4 : p.status ≠ Status.Done)
    (h : Good p) : Good { p with status := s } := by
  obtain ⟨g1, g2, g3, g4, g5, g6, _, g8, g9⟩ := h
  refine ⟨g1, g2, g3, g4, g5, g6, fun ho => absurd ho h1, ?_, ?_⟩
  · rintro (ha | hd)
    · exact absurd ha h2
    · exact g8 (Or.inr hd)
  · exact ⟨fun hs => absurd hs h3, fun hr => absurd (g9.mpr hr) h4⟩

theorem allGood_set {qs : List Quest} {i : Nat} {x : Quest}
    (h : AllGood qs) (hx : Good x.progress) : AllGood (qs.set i x) := by
  intro t ht
  rcases List.mem_or_eq_of_mem_set ht with h1 | h1
  · exact h t h1
  · exact h1 ▸ hx

theorem allGood_get {qs : List Quest} {i : Nat} (h : AllGood qs)
    (hi : i < qs.length) : Good qs[i].progress := h _ (List.getElem_mem hi)

/-! ## A step done -/

theorem closed_set {ao : Option Span} {l : List (Option U64)} {j : Nat} (a : U64)
    (hl : Closed ao l) (hj : ∀ k < stageStartOf ao j, ∃ y, l[k]? = some (some y)) :
    Closed ao (l.set j (some a)) := by
  intro j' x hj' k hk
  have hkj : ∀ k' < stageStartOf ao j, k' ≠ j := fun k' h => by
    have := stageStartOf_le ao j; omega
  rw [List.getElem?_set] at hj'
  by_cases hjj : j = j'
  · subst hjj
    obtain ⟨y, hy⟩ := hj k hk
    exact ⟨y, by rw [List.getElem?_set_ne (hkj k hk).symm]; exact hy⟩
  · simp only [hjj, if_false] at hj'
    obtain ⟨y, hy⟩ := hl j' x hj' k hk
    by_cases hkj' : j = k
    · subst hkj'
      have hlt : j < l.length := by
        rcases Nat.lt_or_ge j l.length with h | h
        · exact h
        · simp [List.getElem?_eq_none h] at hy
      exact ⟨a, by simp [hlt]⟩
    · exact ⟨y, by rw [List.getElem?_set_ne hkj']; exact hy⟩

/-- A wait is never in a set, so its stage starts at the wait. -/
theorem stage_of_wait {p : Progress} {i : Nat} {days : U8} (h : NoWaitInSet p)
    (hw : p.steps.val[i]? = some (Goal.Wait days)) : stageStartOf p.any_order i = i := by
  unfold stageStartOf
  rcases hs : p.any_order with _ | s
  · rfl
  · simp only
    split
    · rename_i hin
      exact absurd hw (h s hs i days hin.1 hin.2)
    · rfl

theorem take_set_of_le (l : List (Option U64)) (i j : Nat) (x : Option U64) (h : i ≤ j) :
    (l.set j x).take i = l.take i := by
  apply List.ext_getElem?
  intro k
  simp only [List.getElem?_take]
  split
  · rw [List.getElem?_set_ne (by omega)]
  · rfl

/-- `Some x` on a list of done times says that step `i` is done. -/
theorem doneAt_of {p : Progress} {i : Nat} {x : U64}
    (h : p.done_times.val[i]? = some (some x)) : DoneAt p i := ⟨x, h⟩

theorem good_finish (p : Progress) (j : Usize) (a : U64) (st : Status) (da : Option U64)
    (h : Good p) (hopen : OpenAt p j.val) (hst : st ≠ Status.Offered)
    (hready : ∀ days, p.steps.val[j.val]? = some (Goal.Wait days) →
      ∃ o, openedOf p j.val = some o ∧ min (o + waitSeconds days) U64.max ≤ a.val)
    (hdone : st = Status.Done ↔
      (0 < p.steps.length ∧ ∀ x ∈ (p.done_times.set j (some a)).val, x.isSome)) :
    Good { p with done_times := p.done_times.set j (some a), status := st, done_at := da } := by
  obtain ⟨g1, g2, g3, g4, g5, g6, _, g8, _⟩ := h
  obtain ⟨hacc, hj, hnot, hbefore⟩ := hopen
  refine ⟨by simp [g1], g2, ?_, g4, g5, ?_, fun ho => absurd ho hst, ?_, hdone⟩
  · simp only [alloc.vec.Vec.set_val_eq]
    exact closed_set a g3 hbefore
  · intro i days d hsi hdi
    simp only [alloc.vec.Vec.set_val_eq] at hdi
    by_cases hij : i = j.val
    · subst hij
      rw [List.getElem?_set_self (by scalar_tac)] at hdi
      cases hdi
      obtain ⟨o, ho, hle⟩ := hready days hsi
      refine ⟨o, ?_, hle⟩
      unfold openedOf at ho ⊢
      simp only [alloc.vec.Vec.set_val_eq, take_set_of_le _ _ _ _ (Nat.le_refl _)]
      exact ho
    · rw [List.getElem?_set_ne (Ne.symm hij)] at hdi
      obtain ⟨o, ho, hle⟩ := g6 i days d hsi hdi
      refine ⟨o, ?_, hle⟩
      have hlt : i < j.val := by
        rcases Nat.lt_or_ge i j.val with h | h
        · exact h
        · exfalso
          have hk := g3 i d hdi j.val (by rw [stage_of_wait g4 hsi]; omega)
          obtain ⟨y, hy⟩ := hk
          exact hnot ⟨y, hy⟩
      unfold openedOf at ho ⊢
      simp only [alloc.vec.Vec.set_val_eq, take_set_of_le _ _ _ _ (Nat.le_of_lt hlt)]
      exact ho
  · intro _
    exact g8 (Or.inl hacc)

/-- An open quest has an accept time, so an open step has an opening time. -/
theorem opened_some (p : Progress) (j : Nat) (h : Good p) (hopen : OpenAt p j) :
    ∃ o, openedOf p j = some o := by
  have hs := h.has_accept (Or.inl hopen.1)
  unfold openedOf
  rcases hl : latest (p.done_times.val.take j) with _ | x
  · rcases ha : p.accepted_at with _ | y
    · simp [ha] at hs
    · exact ⟨y.val, by simp⟩
  · exact ⟨x, by simp⟩

/-- A step that is not early is done no sooner than its wait. -/
theorem ready_of_not_early (p : Progress) (j : Usize) (a : U64) (o : Option U64)
    (h : Good p) (hopen : OpenAt p j.val)
    (ho : ∀ days, p.steps.val[j.val]? = some (Goal.Wait days) → OpenAt p j.val →
      o.map (·.val) = (openedAtOf p j.val).map (fun o => min (o + waitSeconds days) U64.max))
    (hearly : ∀ r, o = some r → ¬ a < r) :
    ∀ days, p.steps.val[j.val]? = some (Goal.Wait days) →
      ∃ o, openedOf p j.val = some o ∧ min (o + waitSeconds days) U64.max ≤ a.val := by
  intro days hw
  have hoe := ho days hw hopen
  have hat : openedAtOf p j.val = openedOf p j.val := by
    unfold openedAtOf openedOf
    rw [stage_of_wait h.no_wait_in_set hw]
  obtain ⟨x, hx⟩ := opened_some p j.val h hopen
  rw [hat, hx] at hoe
  rcases o with _ | r
  · simp at hoe
  · simp only [Option.map_some, Option.some.injEq] at hoe
    refine ⟨x, hx, ?_⟩
    have := hearly r rfl
    rw [← hoe]
    scalar_tac

theorem doneCount_eq_length (l : List (Option U64)) :
    doneCount l = l.length ↔ ∀ x ∈ l, x.isSome := by
  unfold doneCount
  simp [List.countP_eq_length]

/-- The count of done steps after a step is done decides whether the
quest is done. -/
theorem all_done_iff_count (p : Progress) (j : Usize) (a : U64) (h : Good p)
    (hj : j.val < p.steps.length) (n : Usize)
    (hn : n.val = 0 + doneCount ((p.done_times.set j (some a)).val.drop 0)) :
    n = p.steps.len ↔
      (0 < p.steps.length ∧ ∀ x ∈ (p.done_times.set j (some a)).val, x.isSome) := by
  have hl := h.done_len
  simp only [List.drop_zero, Nat.zero_add] at hn
  have hlen : (p.done_times.set j (some a)).val.length = p.steps.length := by
    simp only [alloc.vec.Vec.set_val_eq, List.length_set]
    exact hl
  rw [← doneCount_eq_length, hlen]
  constructor
  · intro he
    have := alloc.vec.Vec.len_val p.steps
    refine ⟨by omega, ?_⟩
    rw [← hn, he, this]
  · rintro ⟨_, hc⟩
    have := alloc.vec.Vec.len_val p.steps
    scalar_tac

@[step]
theorem Progress.finish_step.spec (p : Progress) (j : Usize) (a : U64) (h : Good p) :
    p.finish_step j a ⦃ p' => Good p' ∧ p'.kills = p.kills ∧
      (p'.status = Status.Offered → p.status = Status.Offered) ⦄ := by
  unfold Progress.finish_step Progress.steps_done
  step*
  rcases o with _ | r
  · simp only [bind_tc_ok]
    step as ⟨b, hb⟩
    split
    · simp only [Bool.false_eq_true, if_false]
      have hopen := hb.mp ‹_›
      have hready := ready_of_not_early p j a none h hopen o_post (by simp)
      step as ⟨_, back, _, hback⟩
      · have := h.done_len; have := hopen.2.1; scalar_tac
      step as ⟨i, hi⟩
      subst hback
      have hcount := all_done_iff_count p j a h hopen.2.1 i hi
      split
      · simp only [WP.spec_ok]
        refine ⟨good_finish p j a _ _ h hopen (by simp) hready ?_, by simp, by simp⟩
        exact iff_of_true rfl (hcount.mp ‹_›)
      · simp only [WP.spec_ok]
        refine ⟨good_finish p j a _ _ h hopen (by rw [hopen.1]; simp) hready ?_, by simp, by simp⟩
        exact iff_of_false (by rw [hopen.1]; simp) (fun hx => ‹¬ _› (hcount.mpr hx))
    · simp only [WP.spec_ok]; exact ⟨h, by simp, by simp⟩
  · simp only [bind_tc_ok]
    step as ⟨b, hb⟩
    split
    · split
      · simp only [WP.spec_ok]; exact ⟨h, by simp, by simp⟩
      · rename_i hne
        have hopen := hb.mp ‹_›
        have hready := ready_of_not_early p j a (some r) h hopen o_post
          (fun r' hr' => by cases hr'; simpa using hne)
        step as ⟨_, back, _, hback⟩
        · have := h.done_len; have := hopen.2.1; scalar_tac
        step as ⟨i, hi⟩
        subst hback
        have hcount := all_done_iff_count p j a h hopen.2.1 i hi
        split
        · simp only [WP.spec_ok]
          refine ⟨good_finish p j a _ _ h hopen (by simp) hready ?_, by simp, by simp⟩
          exact iff_of_true rfl (hcount.mp ‹_›)
        · simp only [WP.spec_ok]
          refine ⟨good_finish p j a _ _ h hopen (by rw [hopen.1]; simp) hready ?_, by simp, by simp⟩
          exact iff_of_false (by rw [hopen.1]; simp) (fun hx => ‹¬ _› (hcount.mpr hx))
    · simp only [WP.spec_ok]; exact ⟨h, by simp, by simp⟩

/-! ## A kill counted -/

theorem good_kill (p : Progress) (j : Usize) (count k : U8) (h : Good p) (hopen : OpenAt p j.val)
    (hstep : p.steps.val[j.val]? = some (Goal.Kill count)) (hk : k.val ≤ count.val) :
    Good { p with kills := p.kills.set j k } := by
  obtain ⟨g1, g2, g3, g4, g5, g6, _, g8, g9⟩ := h
  refine ⟨g1, by simp [g2], g3, g4, ?_, g6, fun ho => absurd ho (by rw [hopen.1]; simp), g8, g9⟩
  intro i k' g hki hsi
  simp only [alloc.vec.Vec.set_val_eq] at hki
  by_cases hij : i = j.val
  · subst hij
    have := hopen.2.1
    rw [List.getElem?_set_self (by scalar_tac)] at hki
    rw [hstep] at hsi
    cases hki; cases hsi
    exact hk
  · rw [List.getElem?_set_ne (Ne.symm hij)] at hki
    exact g5 i k' g hki hsi

theorem killsCounted_set (p : Progress) (j : Usize) (x : U8) (hopen : OpenAt p j.val) :
    KillsCounted p { p with kills := p.kills.set j x } := by
  intro k a b ha hb hab
  by_cases hk : k = j.val
  · subst hk; exact hopen
  · simp only [alloc.vec.Vec.set_val_eq] at hb
    rw [List.getElem?_set_ne (Ne.symm hk), ha] at hb
    cases hb; omega

@[step]
theorem Progress.count_kill.spec (p : Progress) (j : Usize) (h : Good p) :
    p.count_kill j ⦃ p' => Good p' ∧ p'.status = p.status ∧ KillsCounted p p' ⦄ := by
  unfold Progress.count_kill
  step*
  all_goals try exact ⟨h, killsCounted_of_eq rfl⟩
  all_goals try (have := h.kills_len; scalar_tac)
  subst index_mut_back_post
  have hs : p.steps.val[j.val]? = some (Goal.Kill count) := by
    rw [List.getElem?_eq_getElem (by scalar_tac)]
    simp only [Option.some.injEq]
    rw [← g_post, ‹g = Goal.Kill count›]
  exact ⟨good_kill p j count i3 h (b_post.mp ‹_›) hs (by scalar_tac),
    killsCounted_set p j i3 (b_post.mp ‹_›)⟩

/-! ## A new offer -/

@[step]
theorem holds_wait_loop.spec (steps : Slice Goal) (last i : Usize) (h : last.val < steps.length) :
    holds_wait_loop steps last i ⦃ b => b = false →
      ∀ k, i.val ≤ k → k ≤ last.val → ∀ d, steps.val[k]? ≠ some (Goal.Wait d) ⦄ := by
  unfold holds_wait_loop
  step*
  · rename_i k d
    by_cases hk : k = i.val
    · subst hk
      rw [List.getElem?_eq_getElem (by scalar_tac), ← g_post, ‹g = _›]
      simp
    · exact b_post b_post1 k (by scalar_tac) k_post1 d
  · rename_i k d
    by_cases hk : k = i.val
    · subst hk
      rw [List.getElem?_eq_getElem (by scalar_tac), ← g_post, ‹g = _›]
      simp
    · exact b_post b_post1 k (by scalar_tac) k_post1 d
termination_by last.val + 1 - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem fits.spec (span : Span) (steps : Slice Goal) :
    fits span steps ⦃ b => b = true →
      ∀ k, span.first.val ≤ k → k ≤ span.last.val → ∀ d, steps.val[k]? ≠ some (Goal.Wait d) ⦄ := by
  unfold fits holds_wait
  by_cases h1 : span.first < span.last
  · simp only [h1, if_true, bind_tc_ok]
    by_cases h2 : span.last < steps.len
    · simp only [h2, decide_true, if_true]
      step*
    · simp [h2]
  · simp [h1]

theorem good_fresh (v : alloc.vec.Vec Goal) (f : Option Span) (v1 : alloc.vec.Vec (Option U64))
    (v2 : alloc.vec.Vec U8) (n : Nat) (hv1 : v1.val = List.replicate n none)
    (hv2 : v2.val = List.replicate n 0#u8) (hn : v.length = n)
    (hf : ∀ s, f = some s → ∀ k, s.first.val ≤ k → k ≤ s.last.val → ∀ d,
      v.val[k]? ≠ some (Goal.Wait d)) :
    Good (⟨v, f, Status.Offered, none, v1, v2, none⟩ : Progress) := by
  have hnone : ∀ (j : Nat) (x : U64), v1.val[j]? ≠ some (some x) := by
    intro j x hj
    rw [hv1, List.getElem?_replicate] at hj
    split at hj <;> simp at hj
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · simp only [alloc.vec.Vec.length] at hn ⊢
    rw [hv1, List.length_replicate, hn]
  · simp only [alloc.vec.Vec.length] at hn ⊢
    rw [hv2, List.length_replicate, hn]
  · intro j x hj
    exact absurd hj (hnone j x)
  · intro s hs j d h1 h2
    exact hf s hs j h1 h2 d
  · intro j k g hk _
    simp only at hk
    rw [hv2, List.getElem?_replicate] at hk
    split at hk
    · cases hk; simp
    · simp at hk
  · intro j days d _ hd
    exact absurd hd (hnone j d)
  · intro _
    simp only [hv1, hv2]
    exact ⟨fun x hx => List.eq_of_mem_replicate hx,
      fun k hk => by rw [List.eq_of_mem_replicate hk]; rfl⟩
  · rintro (h | ⟨j, x, hj⟩)
    · cases h
    · exact absurd hj (hnone j x)
  · refine ⟨fun hs => (by cases hs), ?_⟩
    rintro ⟨hpos, hall⟩
    have hm : none ∈ v1.val := by
      rw [hv1]
      exact List.mem_replicate.mpr ⟨by simp only [alloc.vec.Vec.length] at hn hpos; omega, rfl⟩
    simpa using hall none hm

@[step]
theorem Progress.offered.spec (steps : Slice Goal) (ao : Option Span) :
    Progress.offered steps ao ⦃ p => Good p ∧ p.status = Status.Offered ⦄ := by
  unfold Progress.offered
  rcases ao with _ | span
  · simp only [bind_tc_ok]
    step*
    · intro x _; cases x <;> rfl
    · rfl
    · subst steps_post
      exact good_fresh v none v1 v2 _ v1_post v2_post (by simp; rfl) (by simp)
  · simp only
    step as ⟨x, hx⟩
    obtain ⟨f, hf, hfs⟩ : ∃ f, (if x = true then ok (some span) else ok none) = ok f ∧
        ∀ s, f = some s → ∀ k, s.first.val ≤ k → k ≤ s.last.val → ∀ d,
          steps.val[k]? ≠ some (Goal.Wait d) := by
      split
      · refine ⟨_, rfl, ?_⟩
        intro s hs
        cases hs
        exact hx ‹_›
      · exact ⟨_, rfl, by simp⟩
    rw [hf, bind_tc_ok]
    step*
    · intro x _; cases x <;> rfl
    · rfl
    · subst steps_post
      exact good_fresh v f v1 v2 _ v1_post v2_post (by simp; rfl) hfs

/-! ## The walks over the quests -/

@[step]
theorem find_loop.spec (qs : Slice Quest) (n : U64) (i : Usize) :
    find_loop qs n i ⦃ o => ∀ k, o = some k → k.val < qs.length ⦄ := by
  unfold find_loop
  step*
termination_by qs.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem waiting_offer_loop.spec (qs : Slice Quest) (n : U64) (i : Usize) :
    waiting_offer_loop qs n i ⦃ o => ∀ k, o = some k →
      ∃ hk : k.val < qs.length, (qs.val[k.val]'hk).progress.status = Status.Offered ⦄ := by
  unfold waiting_offer_loop
  step*
termination_by qs.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- What a walk over the quests keeps of each quest: the line of its
offer, its number, its giver, and its kills. A quest never goes back to
an offer. -/
structure Same (q q' : Quest) : Prop where
  line : q'.line = q.line
  number : q'.number = q.number
  giver : q'.giver = q.giver
  kills : q'.progress.kills = q.progress.kills
  offer : q'.progress.status = Status.Offered → q.progress.status = Status.Offered

theorem Same.refl (q : Quest) : Same q q := ⟨rfl, rfl, rfl, rfl, id⟩

theorem Same.trans {q q' q'' : Quest} (h1 : Same q q') (h2 : Same q' q'') : Same q q'' :=
  ⟨h2.line.trans h1.line, h2.number.trans h1.number, h2.giver.trans h1.giver,
    h2.kills.trans h1.kills, fun h => h1.offer (h2.offer h)⟩

/-- Each quest of the second list is the same quest as in the first. -/
def SameAll (qs qs' : List Quest) : Prop :=
  qs'.length = qs.length ∧ ∀ k (hk : k < qs.length) (hk' : k < qs'.length), Same qs[k] qs'[k]

theorem SameAll.trans {qs qs' qs'' : List Quest} (h1 : SameAll qs qs') (h2 : SameAll qs' qs'') :
    SameAll qs qs'' :=
  ⟨h2.1.trans h1.1, fun k hk hk'' =>
    (h1.2 k hk (by rw [h1.1]; exact hk)).trans (h2.2 k (by rw [h1.1]; exact hk) hk'')⟩

theorem sameAll_set (qs : List Quest) (i : Nat) (x : Quest) (hi : i < qs.length)
    (hx : Same qs[i] x) : SameAll qs (qs.set i x) := by
  refine ⟨by simp, fun k hk hk' => ?_⟩
  by_cases hki : k = i
  · subst hki
    simp only [List.getElem_set_self]
    exact hx
  · rw [List.getElem_set_ne (Ne.symm hki)]
    exact Same.refl _

/-- A new status for quest `i`. A status that leaves the offer keeps the
quest the same. -/
theorem sameAll_status (qs : List Quest) (i : Nat) (s : Status) (hi : i < qs.length)
    (hs : s = Status.Offered → qs[i].progress.status = Status.Offered) :
    SameAll qs (qs.set i { qs[i] with progress := { qs[i].progress with status := s } }) :=
  sameAll_set qs i _ hi ⟨rfl, rfl, rfl, rfl, hs⟩

theorem allGood_status (qs : Slice Quest) (i : Usize) (s : Status)
    (hi : i.val < qs.length) (h : AllGood qs.val)
    (hs : s = qs.val[i.val].progress.status ∨
      (s ≠ Status.Offered ∧ s ≠ Status.Accepted ∧ s ≠ Status.Done ∧
        qs.val[i.val].progress.status ≠ Status.Done)) :
    AllGood (qs.set i { qs.val[i.val] with
      progress := { qs.val[i.val].progress with status := s } }).val := by
  simp only [Slice.set_val_eq]
  apply allGood_set h
  have hg := allGood_get h hi
  rcases hs with hs | ⟨h1, h2, h3, h4⟩
  · rw [hs]; exact hg
  · exact good_status _ s h1 h2 h3 h4 hg

/-- After the walk of `decline_waiting_offers_of` from step `i`, the
quests from `i` on hold no waiting offer of the giver `g`. -/
def DeclinedFrom (g : String) (i : Nat) (qs : List Quest) : Prop :=
  ∀ k (hk : k < qs.length), i ≤ k → qs[k].giver = g → qs[k].progress.status ≠ Status.Offered

theorem declinedFrom_step (qs : List Quest) (g : String) (i : Nat) (hi : i < qs.length)
    (s : Status) (hs : s = Status.Offered → qs[i].giver ≠ g) (r : List Quest)
    (hr : SameAll (qs.set i { qs[i] with progress := { qs[i].progress with status := s } }) r)
    (hd : DeclinedFrom g (i + 1) r) : DeclinedFrom g i r := by
  intro k hk hik hg
  by_cases hki : k = i
  · subst hki
    have hsame := hr.2 k (by simp; exact hi) hk
    simp only [List.getElem_set_self] at hsame
    intro ho
    have hso := hsame.offer ho
    exact hs hso (hsame.giver ▸ hg)
  · exact hd k hk (by omega) hg

@[step]
theorem decline_waiting_offers_of_loop.spec (qs : Slice Quest) (g : String) (i : Usize)
    (h : AllGood qs.val) :
    decline_waiting_offers_of_loop qs g i ⦃ qs' =>
      AllGood qs'.val ∧ SameAll qs.val qs'.val ∧ DeclinedFrom g i.val qs'.val ⦄ := by
  unfold decline_waiting_offers_of_loop
  step*
  · subst quest_post index_mut_back_post
    have hi : i.val < qs.length := by scalar_tac
    by_cases hb : b = true
    · have hoff := b_post.mp hb
      simp only [hb, if_true, alloc.string.String.Insts.CoreCmpPartialEqString.eq, bind_tc_ok]
      split
      · step as ⟨i1, hi1⟩
        step as ⟨r, hr1, hr2, hr3⟩
        · exact allGood_status qs i _ hi h (Or.inr ⟨by simp, by simp, by simp, by simp [hoff]⟩)
        · simp only [Slice.set_val_eq] at hr2
          refine ⟨hr1, (sameAll_status _ _ _ hi (by simp)).trans hr2, ?_⟩
          exact declinedFrom_step _ g _ hi _ (by simp) r hr2 (by rw [← hi1]; exact hr3)
      · rename_i hne
        step as ⟨i1, hi1⟩
        step as ⟨r, hr1, hr2, hr3⟩
        · exact allGood_status qs i _ hi h (Or.inl rfl)
        · simp only [Slice.set_val_eq] at hr2
          refine ⟨hr1, (sameAll_status _ _ _ hi (fun h => h)).trans hr2, ?_⟩
          exact declinedFrom_step _ g _ hi _ (fun _ => by simpa using hne) r hr2
            (by rw [← hi1]; exact hr3)
    · simp only [hb, if_false, Bool.false_eq_true, bind_tc_ok]
      step as ⟨i1, hi1⟩
      step as ⟨r, hr1, hr2, hr3⟩
      · exact allGood_status qs i _ hi h (Or.inl rfl)
      · simp only [Slice.set_val_eq] at hr2
        refine ⟨hr1, (sameAll_status _ _ _ hi (fun h => h)).trans hr2, ?_⟩
        exact declinedFrom_step _ g _ hi _ (fun ho => absurd (b_post.mpr ho) hb) r hr2
          (by rw [← hi1]; exact hr3)
  · refine ⟨h, ⟨rfl, fun k _ _ => Same.refl _⟩, ?_⟩
    intro k hk hik
    scalar_tac
termination_by qs.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem abandon_loop.spec (qs : Slice Quest) (n : U64) (i : Usize) (h : AllGood qs.val) :
    abandon_loop qs n i ⦃ qs' => AllGood qs'.val ∧ SameAll qs.val qs'.val ⦄ := by
  unfold abandon_loop
  step*
  · subst quest_post index_mut_back_post
    have hi : i.val < qs.length := by scalar_tac
    have hsame : AllGood (qs.set i qs.val[i.val]).val := by
      simp only [Slice.set_val_eq]
      exact allGood_set h (allGood_get h hi)
    have hset : SameAll qs.val (qs.set i qs.val[i.val]).val := by
      simp only [Slice.set_val_eq]
      exact sameAll_set _ _ _ hi (Same.refl _)
    obtain ⟨o, ho, hop⟩ : ∃ o, (if b = true then ok true else
        Status.Insts.CoreCmpPartialEqStatus.eq qs.val[i.val].progress.status Status.Accepted) =
        ok o ∧ (o = true → qs.val[i.val].progress.status ≠ Status.Done) := by
      split
      · refine ⟨_, rfl, fun _ => ?_⟩
        rw [b_post.mp ‹_›]; simp
      · rcases hst : qs.val[i.val].progress.status <;>
          simp [Status.Insts.CoreCmpPartialEqStatus.eq, Status.read_discriminant]
    rw [ho, bind_tc_ok]
    split
    · split
      · simp only [WP.spec_ok]
        refine ⟨allGood_status qs i _ hi h
          (Or.inr ⟨by simp, by simp, by simp, hop ‹_›⟩), ?_⟩
        simp only [Slice.set_val_eq]
        exact sameAll_status _ _ _ hi (fun h => by cases h)
      · step as ⟨i1, hi1⟩
        step as ⟨r, hr1, hr2⟩
        exact ⟨hr1, hset.trans hr2⟩
    · step as ⟨i1, hi1⟩
      step as ⟨r, hr1, hr2⟩
      exact ⟨hr1, hset.trans hr2⟩
  · exact ⟨h, rfl, fun k _ _ => Same.refl _⟩
termination_by qs.length - i.val
decreasing_by all_goals scalar_decr_tac

/-! ## One line of the log -/

/-- The accept of a waiting offer keeps the invariant. -/
theorem good_accept (p : Progress) (a : U64) (h : Good p) (ho : p.status = Status.Offered) :
    Good { p with status := Status.Accepted, accepted_at := some a } := by
  obtain ⟨g1, g2, g3, g4, g5, _, g7, _, _⟩ := h
  have hnone := (g7 ho).1
  have hnot : ∀ (j : Nat) (x : U64), p.done_times.val[j]? ≠ some (some x) := by
    intro j x hj
    exact absurd (hnone _ (List.mem_of_getElem? hj)) (by simp)
  refine ⟨g1, g2, g3, g4, g5, ?_, ?_, ?_, ?_⟩
  · intro j days d _ hd
    exact absurd hd (hnot j d)
  · intro hs
    cases hs
  · intro _
    rfl
  · refine ⟨fun hs => (by cases hs), ?_⟩
    rintro ⟨hpos, hall⟩
    have hlen : 0 < p.done_times.val.length := by
      simp only [alloc.vec.Vec.length] at g1 hpos; omega
    have hm := List.getElem_mem hlen
    have := hall _ hm
    rw [hnone _ hm] at this
    cases this

@[simp] theorem deref_length (qs : alloc.vec.Vec Quest) : qs.deref.length = qs.length := by
  simp [alloc.vec.Vec.deref]

@[simp] theorem deref_val (qs : alloc.vec.Vec Quest) : qs.deref.val = qs.val := by
  simp [alloc.vec.Vec.deref]

/-- What a line keeps of each quest that was there before it: the line
of its offer, its number, and its giver. A quest never goes back to an
offer, and a kill counts only for an open step. -/
structure Kept (q q' : Quest) : Prop where
  line : q'.line = q.line
  number : q'.number = q.number
  giver : q'.giver = q.giver
  offer : q'.progress.status = Status.Offered → q.progress.status = Status.Offered
  kills : KillsCounted q.progress q'.progress

theorem Same.kept {q q' : Quest} (h : Same q q') : Kept q q' :=
  ⟨h.line, h.number, h.giver, h.offer, killsCounted_of_eq h.kills⟩

/-- The quest that an offer line adds. -/
def NewOffer (c : Change) (line : Usize) (q : Quest) : Prop :=
  q.line = line ∧ ∃ steps ao, c = Change.Offered q.number q.giver steps ao

/-- A line keeps each quest that was there, and adds at most its own
offer, at the end. -/
def Grown (c : Change) (line : Usize) (qs qs' : List Quest) : Prop :=
  qs.length ≤ qs'.length ∧ qs'.length ≤ qs.length + 1 ∧
  (∀ k (hk : k < qs.length) (hk' : k < qs'.length), Kept qs[k] qs'[k]) ∧
  ∀ hk : qs.length < qs'.length, NewOffer c line qs'[qs.length]

/-- Each giver holds at most one waiting offer. -/
def OneOfferEach (qs : List Quest) : Prop :=
  ∀ i j (hi : i < qs.length) (hj : j < qs.length), qs[i].giver = qs[j].giver →
    qs[i].progress.status = Status.Offered → qs[j].progress.status = Status.Offered → i = j

theorem oneOffer_of_kept {qs qs' : List Quest} (hl : qs'.length = qs.length)
    (hk : ∀ k (h : k < qs.length) (h' : k < qs'.length), Kept qs[k] qs'[k])
    (h : OneOfferEach qs) : OneOfferEach qs' := by
  intro i j hi hj hg ho1 ho2
  have ki := hk i (by omega) hi
  have kj := hk j (by omega) hj
  exact h i j _ _ (by rw [← ki.giver, ← kj.giver]; exact hg) (ki.offer ho1) (kj.offer ho2)

/-- After the old offers of the giver are declined, the new offer is the
only waiting offer of the giver. -/
theorem oneOffer_push {qs s1 : List Quest} {g : String} (x : Quest) (hs : SameAll qs s1)
    (hd : DeclinedFrom g 0 s1) (hx : x.giver = g) (h : OneOfferEach qs) :
    OneOfferEach (s1 ++ [x]) := by
  have h1 : OneOfferEach s1 := oneOffer_of_kept hs.1 (fun k a b => (hs.2 k a b).kept) h
  intro i j hi hj hg ho1 ho2
  simp only [List.length_append, List.length_singleton] at hi hj
  simp only [List.getElem_append] at hg ho1 ho2
  split at hg <;> split at hg
  · simp only [*, _root_.dite_true] at ho1 ho2
    exact h1 i j _ _ hg ho1 ho2
  · rename_i hi' hj'
    simp only [hi', _root_.dite_true] at ho1
    exact absurd ho1 (hd i hi' (Nat.zero_le _) (by rw [hg]; simpa using hx))
  · rename_i hi' hj'
    simp only [hj', _root_.dite_true] at ho2
    exact absurd ho2 (hd j hj' (Nat.zero_le _) (by rw [← hg]; simpa using hx))
  · rename_i hi' hj'
    omega

/-- Each quest points at the line of its offer. -/
def LinesOk (cs : List Change) (qs : List Quest) : Prop :=
  ∀ q ∈ qs, ∃ steps ao, cs[q.line.val]? = some (Change.Offered q.number q.giver steps ao)

theorem linesOk_grown {c : Change} {line : Usize} {cs : List Change} {qs qs' : List Quest}
    (hg : Grown c line qs qs') (hc : cs[line.val]? = some c) (h : LinesOk cs qs) :
    LinesOk cs qs' := by
  intro q hq
  obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hq
  rcases Nat.lt_or_ge k qs.length with hk' | hk'
  · have kp := hg.2.2.1 k hk' hk
    rw [kp.line, kp.number, kp.giver]
    exact h _ (List.getElem_mem hk')
  · have : k = qs.length := by have := hg.2.1; omega
    subst this
    obtain ⟨hl, steps, ao, hc'⟩ := hg.2.2.2 hk
    exact ⟨steps, ao, by rw [hl, hc, hc']⟩

/-- The post of one line that changes one quest. -/
theorem post_set (c : Change) (line : Usize) (qs : List Quest) (i : Nat) (x : Quest)
    (hi : i < qs.length) (h : AllGood qs) (hg : Good x.progress) (hx : Kept qs[i] x) :
    AllGood (qs.set i x) ∧ Grown c line qs (qs.set i x) ∧
      (OneOfferEach qs → OneOfferEach (qs.set i x)) := by
  have hk : ∀ k (h : k < qs.length) (h' : k < (qs.set i x).length),
      Kept qs[k] (qs.set i x)[k] := by
    intro k hk hk'
    by_cases hki : k = i
    · subst hki
      simp only [List.getElem_set_self]
      exact hx
    · rw [List.getElem_set_ne (Ne.symm hki)]
      exact (Same.refl _).kept
  refine ⟨allGood_set h hg, ⟨by simp, by simp, hk, fun hl => absurd hl (by simp)⟩, ?_⟩
  exact oneOffer_of_kept (by simp) hk

/-- The post of one line that keeps every quest the same. -/
theorem post_same (c : Change) (line : Usize) (qs qs' : List Quest) (h : AllGood qs')
    (hs : SameAll qs qs') :
    AllGood qs' ∧ Grown c line qs qs' ∧ (OneOfferEach qs → OneOfferEach qs') := by
  have hk : ∀ k (h : k < qs.length) (h' : k < qs'.length), Kept qs[k] qs'[k] :=
    fun k a b => (hs.2 k a b).kept
  refine ⟨h, ⟨by rw [hs.1], by rw [hs.1]; omega, hk, fun hl => absurd hl (by rw [hs.1]; omega)⟩,
    oneOffer_of_kept hs.1 hk⟩

@[step]
theorem apply.spec (qs : alloc.vec.Vec Quest) (c : Change) (line : Usize)
    (h : AllGood qs.val) (hl : qs.length < Usize.max) :
    apply qs c line ⦃ qs' => AllGood qs'.val ∧ Grown c line qs.val qs'.val ∧
      (OneOfferEach qs.val → OneOfferEach qs'.val) ⦄ := by
  unfold apply decline_waiting_offers_of waiting_offer abandon find
  cases c
  · rename_i n g st ao
    simp only [lift, bind_tc_ok, alloc.vec.Vec.deref_mut]
    step*
    · exact h
    simp only [alloc.string.String.Insts.CoreCloneClone.clone, bind_tc_ok]
    step as ⟨p, hp, _⟩
    have hlen : s1.length = qs.length := s1_post1.1
    have e1 : (↑({ slice := s1 } : alloc.vec.Vec Quest) : List Quest) = s1.val := rfl
    step as ⟨qs', hqs'⟩
    have hq : qs'.val = s1.val ++ [{ line := line, number := n, giver := g, progress := p }] :=
      hqs'
    have hlen' : s1.val.length = qs.val.length := hlen
    rw [hq]
    refine ⟨?_, ⟨by simp [hlen'], by simp [hlen'], ?_, ?_⟩, ?_⟩
    · intro q hq'
      rw [List.mem_append] at hq'
      rcases hq' with hq' | hq'
      · exact s1_post q hq'
      · simp only [List.mem_singleton] at hq'
        subst hq'
        exact hp
    · intro k hk hk'
      rw [List.getElem_append_left (by omega)]
      exact (s1_post1.2 k hk (by omega)).kept
    · intro hk
      rw [List.getElem_append_right (by omega)]
      simp only [hlen', Nat.sub_self, List.getElem_cons_zero]
      exact ⟨rfl, st, ao, rfl⟩
    · intro h1
      exact oneOffer_push _ s1_post1 s1_post2 rfl h1
  · step*
    all_goals try exact post_same _ _ _ _ h ⟨rfl, fun k _ _ => Same.refl _⟩
    · obtain ⟨hk, _⟩ := o_post index ‹_›; simpa using hk
    · subst index_mut_back_post; simp only [alloc.vec.Vec.set_length]
      obtain ⟨hk, _⟩ := o_post index ‹_›; simpa using hk
    · obtain ⟨hk, hoff⟩ := o_post index ‹_›
      simp only [deref_length, deref_val] at hk hoff
      subst index_mut_back1_post q1_post index_mut_back_post q_post
      simp only [alloc.vec.Vec.set_val_eq, List.getElem_set_self, List.set_set]
      exact post_set _ _ _ _ _ hk h (good_accept _ _ (allGood_get h hk) hoff)
        ⟨rfl, rfl, rfl, fun h => (by cases h), killsCounted_of_eq rfl⟩
  · step*
    all_goals try exact post_same _ _ _ _ h ⟨rfl, fun k _ _ => Same.refl _⟩
    · obtain ⟨hk, _⟩ := o_post index ‹_›; simpa using hk
    · obtain ⟨hk, hoff⟩ := o_post index ‹_›
      simp only [deref_length, deref_val] at hk hoff
      subst index_mut_back_post q_post
      simp only [alloc.vec.Vec.set_val_eq]
      exact post_set _ _ _ _ _ hk h
        (good_status _ _ (by simp) (by simp) (by simp) (by simp [hoff]) (allGood_get h hk))
        ⟨rfl, rfl, rfl, fun h => (by cases h), killsCounted_of_eq rfl⟩
  · step*
    all_goals try exact post_same _ _ _ _ h ⟨rfl, fun k _ _ => Same.refl _⟩
    · simpa using o_post index ‹_›
    · subst q_post; exact allGood_get h (by simpa using o_post index ‹_›)
    · subst index_mut_back_post q_post
      have hk : index.val < qs.length := by simpa using o_post index ‹_›
      simp only [alloc.vec.Vec.set_val_eq]
      exact post_set _ _ _ _ _ hk h p_post ⟨rfl, rfl, rfl, p_post2, killsCounted_of_eq p_post1⟩
  · step*
    all_goals try exact post_same _ _ _ _ h ⟨rfl, fun k _ _ => Same.refl _⟩
    · simpa using o_post index ‹_›
    · subst q_post; exact allGood_get h (by simpa using o_post index ‹_›)
    · subst index_mut_back_post q_post
      have hk : index.val < qs.length := by simpa using o_post index ‹_›
      simp only [alloc.vec.Vec.set_val_eq]
      exact post_set _ _ _ _ _ hk h p_post ⟨rfl, rfl, rfl, fun ho => p_post1 ▸ ho, p_post2⟩
  · simp only [lift, bind_tc_ok, alloc.vec.Vec.deref_mut]
    step*
    · exact h
    · exact post_same _ _ _ _ s1_post s1_post1

/-! ## The whole log -/

@[step]
theorem quest_log_loop.spec (cs : Slice Change) (qs : alloc.vec.Vec Quest) (i : Usize)
    (h : AllGood qs.val) (h1 : OneOfferEach qs.val) (h2 : LinesOk cs.val qs.val)
    (hl : qs.length ≤ i.val) (hi : i.val ≤ cs.length) :
    quest_log_loop cs qs i ⦃ qs' => AllGood qs'.val ∧ OneOfferEach qs'.val ∧
      LinesOk cs.val qs'.val ∧ qs'.length ≤ cs.length ⦄ := by
  unfold quest_log_loop
  step*
  · exact linesOk_grown quests1_post1 (by rw [c_post, List.getElem?_eq_getElem (by scalar_tac)]) h2
termination_by cs.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- Every log keeps the invariants: the fold never panics, each quest is
`Good` at the end, each giver holds at most one waiting offer, and each
quest points at the line of its offer. -/
theorem quest_log_inv (cs : Slice Change) :
    quest_log cs ⦃ qs => AllGood qs.val ∧ OneOfferEach qs.val ∧ LinesOk cs.val qs.val ∧
      qs.length ≤ cs.length ⦄ := by
  unfold quest_log
  step*
  · intro q hq
    simp [alloc.vec.Vec.new] at hq
  · intro i j hi
    simp [alloc.vec.Vec.new] at hi
  · intro q hq
    simp [alloc.vec.Vec.new] at hq

theorem quest_log_good (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, Good q.progress ⦄ :=
  WP.spec_mono (quest_log_inv cs) (fun _ h => h.1)

/-! ## The laws -/

set_option linter.dupNamespace false in
/-- (a) The log never panics, and every quest is well formed: one done
time and one kill count for each step. -/
theorem quest_log.spec (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val,
      q.progress.done_times.length = q.progress.steps.length ∧
      q.progress.kills.length = q.progress.steps.length ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq
  exact ⟨(h q hq).done_len, (h q hq).kills_len⟩

/-- (b) In a quest with no any-order set, no step is done before the step
before it. -/
theorem ordered_quest_is_done_in_order (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, q.progress.any_order = none →
      ∀ j k, k < j → DoneAt q.progress j → DoneAt q.progress k ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq hnone j k hkj ⟨x, hx⟩
  have := (h q hq).closed j x hx k
  simp only [stageStartOf, hnone] at this
  exact this hkj

/-- (c) A step of an any-order set is done only after every step before
the set is done. -/
theorem any_order_set_waits_for_the_steps_before_it (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, ∀ s, q.progress.any_order = some s →
      ∀ j, s.first.val ≤ j → j ≤ s.last.val → DoneAt q.progress j →
      ∀ k < s.first.val, DoneAt q.progress k ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq s hs j h1 h2 ⟨x, hx⟩ k hk
  have := (h q hq).closed j x hx k
  simp only [stageStartOf, hs, h1, h2, and_self, if_true] at this
  exact this hk

/-- (d) A step after an any-order set is done only when every step of the
set is done. -/
theorem a_step_after_a_set_waits_for_all_of_it (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, ∀ s, q.progress.any_order = some s →
      ∀ j, s.last.val < j → DoneAt q.progress j → ∀ k ≤ s.last.val, DoneAt q.progress k ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq s hs j hj ⟨x, hx⟩ k hk
  have := (h q hq).closed j x hx k
  have hn : ¬ (s.first.val ≤ j ∧ j ≤ s.last.val) := by omega
  simp only [stageStartOf, hs, hn, if_false] at this
  exact this (by omega)

/-- (e) A wait never ends early. A done wait step was done at its opening
time plus its days, or later. The opening time is the latest done time of
the steps before it, or the accept, as the property test
`a_wait_never_ends_early` reads it. Near `u64::MAX`, the Rust add
saturates: then the wait ends at `u64::MAX`, the last second there is. -/
theorem a_wait_never_ends_early (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, ∀ j days (d : U64),
      q.progress.steps.val[j]? = some (Goal.Wait days) →
      q.progress.done_times.val[j]? = some (some d) →
      ∃ o, openedOf q.progress j = some o ∧
        (o + days.val * 86400 ≤ d.val ∨
          (d.val = U64.max ∧ U64.max < o + days.val * 86400)) ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq j days d hs hd
  obtain ⟨o, ho, hle⟩ := (h q hq).waits_held j days d hs hd
  refine ⟨o, ho, ?_⟩
  unfold waitSeconds at hle
  have := d.hBounds
  by_cases hfit : o + days.val * 86400 ≤ U64.max
  · left; scalar_tac
  · right; scalar_tac

/-- (f) A kill step counts no more kills than its count, and every other
step counts none. -/
theorem kills_never_pass_the_count (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, ∀ (j : Nat) (k : U8) g,
      q.progress.kills.val[j]? = some k → q.progress.steps.val[j]? = some g →
      k.val ≤ killLimit g ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq
  exact (h q hq).kills_fit

/-- (g) An offer that waits for an answer has no done step and no kill. -/
theorem an_offer_has_no_done_steps (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, q.progress.status = Status.Offered →
      (∀ j, ¬ DoneAt q.progress j) ∧ ∀ k ∈ q.progress.kills.val, k.val = 0 ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq hoff
  obtain ⟨hnone, hk⟩ := (h q hq).offer_clean hoff
  refine ⟨?_, hk⟩
  rintro j ⟨x, hx⟩
  exact absurd (hnone _ (List.mem_of_getElem? hx)) (by simp)

/-- (h) A quest is done exactly when it has a step and every step is
done. A quest with no step comes only from a damaged file, and it is
never done. -/
theorem a_quest_is_done_exactly_when_every_step_is_done (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, q.progress.status = Status.Done ↔
      (0 < q.progress.steps.length ∧ ∀ j < q.progress.steps.length, DoneAt q.progress j) ⦄ := by
  apply WP.spec_mono (quest_log_good cs)
  intro qs h q hq
  have g := h q hq
  have hl : q.progress.done_times.val.length = q.progress.steps.length := g.done_len
  have hd : DoneExact q.progress := g.done_exact
  unfold DoneExact at hd
  rw [hd]
  constructor
  · rintro ⟨hpos, hall⟩
    refine ⟨hpos, fun j hj => ?_⟩
    have hj' : j < q.progress.done_times.val.length := by omega
    obtain ⟨x, hx⟩ := Option.isSome_iff_exists.mp (hall _ (List.getElem_mem hj'))
    exact ⟨x, by rw [List.getElem?_eq_getElem hj', hx]⟩
  · rintro ⟨hpos, hall⟩
    refine ⟨hpos, fun x hx => ?_⟩
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hx
    obtain ⟨y, hy⟩ := hall j (by omega)
    rw [List.getElem?_eq_getElem hj] at hy
    simp only [Option.some.injEq] at hy
    simp [hy]

/-- (i) Each giver holds at most one waiting offer. -/
theorem each_giver_holds_at_most_one_waiting_offer (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ i j (hi : i < qs.val.length) (hj : j < qs.val.length),
      qs.val[i].giver = qs.val[j].giver → qs.val[i].progress.status = Status.Offered →
      qs.val[j].progress.status = Status.Offered → i = j ⦄ :=
  WP.spec_mono (quest_log_inv cs) (fun _ h => h.2.1)

/-- (j) After any log, one more line counts a kill only for a step that
was open before the line. -/
theorem a_kill_counts_only_for_an_open_step (cs : Slice Change) (c : Change) (line : Usize)
    (hl : cs.length < Usize.max) :
    quest_log cs ⦃ qs => apply qs c line ⦃ qs' =>
      ∀ i (hi : i < qs.val.length) (hi' : i < qs'.val.length) (k : Nat) (a b : U8),
        qs.val[i].progress.kills.val[k]? = some a →
        qs'.val[i].progress.kills.val[k]? = some b →
        a.val < b.val → OpenAt qs.val[i].progress k ⦄ ⦄ := by
  apply WP.spec_mono (quest_log_inv cs)
  intro qs h
  apply WP.spec_mono (apply.spec qs c line h.1 (by have := h.2.2.2; omega))
  intro qs' h' i hi hi'
  exact (h'.2.1.2.2.1 i hi hi').kills

/-- (k) Each quest points at the line of its offer, with its number and
its giver. So the story program finds the words of every quest. -/
theorem every_quest_points_at_its_offer (cs : Slice Change) :
    quest_log cs ⦃ qs => ∀ q ∈ qs.val, ∃ steps ao,
      cs.val[q.line.val]? = some (Change.Offered q.number q.giver steps ao) ⦄ :=
  WP.spec_mono (quest_log_inv cs) (fun _ h => h.2.2.1)

end timeways_rules.quest_log
