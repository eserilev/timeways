-- The laws of the chapters and tales of the Chronicle
-- (docs/plans/chapters.md 8). They speak about the pure model of
-- `ChaptersModel.lean`. The bridge of `ChaptersBridge.lean` shows that
-- the Rust fold computes the model, so each law holds for the Rust fold.
import Timeways.ChaptersBridge

set_option linter.unusedSimpArgs false
set_option linter.dupNamespace false

open Aeneas Aeneas.Std Result
open timeways_rules.weights

namespace timeways_rules.chapters

open alloc.vec

deriving instance DecidableEq for KeyKind
deriving instance DecidableEq for Close

/-! ## Steps, gains, and reachable folds -/

/-- The gain record that a step pushes. -/
def gainOfStep (st : Fold) : Step → Gain
  | .Rule _ => ZERO_GAIN
  | .Play p => { amount := (gainM st.keys st.foes p.key).1.1, track := p.track,
                 revenge := (gainM st.keys st.foes p.key).1.2 }

theorem applyM_gains_eq (st : Fold) (s : Step) (h : st.gains.val.length < Usize.max) :
    (applyM st s).gains.val = st.gains.val ++ [gainOfStep st s] := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM, gainOfStep]; split <;> exact pushM_val _ _ h
  | Play p =>
    simp only [applyM, applyPlayM, gainOfStep]
    rcases ht : p.track with _ | i <;> simp only [ht] <;> exact pushM_val _ _ h

theorem runM_nil : runM [] = startM := rfl

theorem runM_snoc (ss : List Step) (s : Step) : runM (ss ++ [s]) = applyM (runM ss) s := by
  simp [runM, List.foldl_append]

/-- The fold of a log that fits a `usize`: one gain for each step, and room in
each vector. -/
theorem runM_sized (ss : List Step) (h : ss.length ≤ Usize.max) :
    (runM ss).gains.val.length = ss.length ∧ Sized (runM ss) := by
  induction ss using List.reverseRecOn with
  | nil => simp [runM, startM, Sized]
  | append_singleton ss s ih =>
    simp only [List.length_append, List.length_singleton] at h
    obtain ⟨hl, hs⟩ := ih (by omega)
    rw [runM_snoc]
    refine ⟨?_, applyM_sized _ _ hs (by omega)⟩
    rw [applyM_gains_length _ _ (by omega), hl]
    simp

theorem runM_gains (ss : List Step) (s : Step) (h : ss.length + 1 ≤ Usize.max) :
    (runM (ss ++ [s])).gains.val = (runM ss).gains.val ++ [gainOfStep (runM ss) s] := by
  rw [runM_snoc]
  apply applyM_gains_eq
  rw [(runM_sized ss (by omega)).1]
  omega

/-- Folding one line at a time gives the same fold as folding the whole log. -/
theorem folding_line_by_line (a b : List Step) : b.foldl applyM (runM a) = runM (a ++ b) := by
  simp [runM, List.foldl_append]

/-! ## Theorems 3 and 4: what is closed never changes -/

theorem applyM_closed_prefix (st : Fold) (s : Step) :
    st.closed.val <+: (applyM st s).closed.val := by
  obtain ⟨l, hl, _⟩ := applyM_closed st s
  exact ⟨l, hl.symm⟩

theorem applyM_visits_prefix (st : Fold) (s : Step) :
    st.visits.val <+: (applyM st s).visits.val := by
  obtain ⟨l, hl, _⟩ := applyM_visits st s
  exact ⟨l, hl.symm⟩

theorem foldl_closed_prefix (st : Fold) (more : List Step) :
    st.closed.val <+: (more.foldl applyM st).closed.val := by
  induction more generalizing st with
  | nil => exact List.prefix_refl _
  | cons s rest ih => exact (applyM_closed_prefix st s).trans (ih _)

theorem foldl_visits_prefix (st : Fold) (more : List Step) :
    st.visits.val <+: (more.foldl applyM st).visits.val := by
  induction more generalizing st with
  | nil => exact List.prefix_refl _
  | cons s rest ih => exact (applyM_visits_prefix st s).trans (ih _)

/-- Theorem 3. The closed chapters of a log are a prefix of the closed
chapters of the log with more steps. -/
theorem a_closed_chapter_never_changes (ss more : List Step) :
    (runM ss).closed.val <+: (runM (ss ++ more)).closed.val := by
  rw [← folding_line_by_line]
  exact foldl_closed_prefix _ _

/-- Theorem 4. The closed visits of a log are a prefix of the closed visits
of the log with more steps. -/
theorem a_closed_visit_never_changes (ss more : List Step) :
    (runM ss).visits.val <+: (runM (ss ++ more)).visits.val := by
  rw [← folding_line_by_line]
  exact foldl_visits_prefix _ _

/-! ## Theorem 14: an instance step adds nothing to a chapter -/

/-- Theorem 14. A step in an instance leaves the open chapter and the closed
chapters as they were. -/
theorem an_instance_step_never_adds_world_weight (st : Fold) (p : Play) (i : Usize)
    (h : p.track = .Instance i) :
    (applyM st (.Play p)).open = st.open ∧ (applyM st (.Play p)).closed = st.closed := by
  simp [applyM, applyPlayM, h]

/-! ## Theorem 10: a step with no gain closes nothing -/

theorem addChapterM_zero (zones : Vec ZoneRecord) (closed : Vec ClosedChapter) (opn : Chapter)
    (pending : Option Break) (zone : Usize) (amount : U16) (here : Usize) (h : amount.val = 0) :
    (addChapterM zones closed opn pending zone amount here).2.1 = closed := by
  simp [addChapterM, h]

/-- Theorem 10. A step that gains nothing closes no chapter. Only a rule step
closes one with no gain. This holds from any fold. -/
theorem a_step_with_no_gain_closes_nothing (st : Fold) (p : Play)
    (h : (gainOfStep st (.Play p)).amount.val = 0) :
    (applyM st (.Play p)).closed = st.closed := by
  simp only [gainOfStep] at h
  simp only [applyM, applyPlayM]
  rcases p.track with _ | i
  · exact addChapterM_zero _ _ _ _ _ _ _ h
  · rfl

/-! ## Theorems 6, 7, and 8: the weight of a closed chapter -/

theorem pushM_one {α : Type} (v : Vec α) (c : α) :
    ∃ l, (pushM v c).val = v.val ++ l ∧ ∀ x ∈ l, x = c := by
  unfold pushM; split
  · exact ⟨[c], by simp, by simp⟩
  · exact ⟨[], by simp, by simp⟩

/-- The chapters that a step of the open world closes: none that a rule
closed, each with the least weight, and at most `MAX - 1 + CAP_MAX` when the
open chapter was below `MAX`. The open chapter stays below `MAX`. -/
theorem addChapterM_new (zones : Vec ZoneRecord) (closed : Vec ClosedChapter) (opn : Chapter)
    (pending : Option Break) (zone : Usize) (amount : U16) (here : Usize) (ha : amount.val ≤ 7) :
    ∃ l, (addChapterM zones closed opn pending zone amount here).2.1.val = closed.val ++ l ∧
      (∀ c ∈ l, c.close ≠ .Rule ∧ MIN ≤ c.chapter.weight.val ∧
        (opn.weight.val < MAX → c.chapter.weight.val ≤ MAX - 1 + 7)) ∧
      (opn.weight.val < MAX →
        (addChapterM zones closed opn pending zone amount here).2.2.1.weight.val < MAX) := by
  unfold addChapterM
  split
  · exact ⟨[], by simp, by simp, fun h => h⟩
  · rename_i h0
    simp only
    generalize withBreak pending (zoneBreakM zones closed.val.length zone) = p1
    rcases p1 with _ | cut
    · simp only
      have hw := sat_add_val opn.weight amount
      rw [← u16_sat_add] at hw
      simp only [UScalar.max] at hw
      split
      · rename_i hm
        obtain ⟨l, hl, hx⟩ := pushM_one closed { chapter := { opn with
            weight := core.num.U16.saturating_add opn.weight amount,
            zone := if opn.zone.isNone then some zone else opn.zone }, last := here, close := .Max }
        refine ⟨l, hl, ?_, ?_⟩
        · intro c hc
          rw [hx c hc]
          simp only [ne_eq, reduceCtorEq, not_false_eq_true, true_and]
          simp only [MIN, MAX] at *
          constructor <;> intros <;> scalar_tac
        · intro _; simp [MAX]
      · rename_i hm
        refine ⟨[], by simp, by simp, ?_⟩
        intro _; simp only [MAX] at *; omega
    · simp only
      by_cases hb : MIN ≤ opn.weight.val ∧ opn.first.val < here.val
      · simp only [hb, and_self, if_true]
        have hw := sat_add_val (0#u16) amount
        rw [← u16_sat_add] at hw
        simp only [UScalar.max] at hw
        have : ¬ MAX ≤ (core.num.U16.saturating_add 0#u16 amount).val := by
          simp only [MAX]; scalar_tac
        simp only [this, if_false]
        obtain ⟨l, hl, hx⟩ := pushM_one closed
          (ClosedChapter.mk opn (core.num.Usize.saturating_sub here 1#usize) .Break)
        refine ⟨l, hl, ?_, ?_⟩
        · intro c hc
          rw [hx c hc]
          simp only [ne_eq, reduceCtorEq, not_false_eq_true, true_and]
          refine ⟨hb.1, ?_⟩
          intro h; simp only [MAX] at *; omega
        · intro _; simp only [MAX]; scalar_tac
      · rw [if_neg hb]
        simp only
        have hw := sat_add_val opn.weight amount
        rw [← u16_sat_add] at hw
        simp only [UScalar.max] at hw
        split
        · rename_i hm
          obtain ⟨l, hl, hx⟩ := pushM_one closed { chapter := { opn with
              weight := core.num.U16.saturating_add opn.weight amount,
              zone := if opn.zone.isNone then some zone else opn.zone }, last := here, close := .Max }
          refine ⟨l, hl, ?_, ?_⟩
          · intro c hc
            rw [hx c hc]
            simp only [ne_eq, reduceCtorEq, not_false_eq_true, true_and]
            simp only [MIN, MAX] at *
            constructor <;> intros <;> scalar_tac
          · intro _; simp [MAX]
        · rename_i hm
          refine ⟨[], by simp, by simp, ?_⟩
          intro _; simp only [MAX] at *; omega

/-- The chapters that a step of the open world closes. -/
theorem applyPlayM_new_closed (st : Fold) (p : Play) :
    ∃ l, (applyM st (.Play p)).closed.val = st.closed.val ++ l ∧
      (∀ c ∈ l, c.close ≠ .Rule ∧ MIN ≤ c.chapter.weight.val ∧
        (st.open.weight.val < MAX → c.chapter.weight.val ≤ MAX - 1 + 7)) ∧
      (st.open.weight.val < MAX → (applyM st (.Play p)).open.weight.val < MAX) := by
  simp only [applyM, applyPlayM]
  rcases p.track with _ | i
  · exact addChapterM_new _ _ _ _ _ _ _ (gainM_amount_le _ _ _)
  · exact ⟨[], by simp, by simp, fun h => h⟩

/-- The chapter that a rule step closes. -/
theorem applyRuleM_new_closed (st : Fold) (r : U8) :
    ∃ l, (applyM st (.Rule r)).closed.val = st.closed.val ++ l ∧ l.length ≤ 1 ∧
      (∀ c ∈ l, c.close = .Rule ∧ c.chapter = st.open) ∧
      (st.open.weight.val < MAX → (applyM st (.Rule r)).open.weight.val < MAX) := by
  simp only [applyM, applyRuleM]
  split
  · obtain ⟨l, hl, hx⟩ := pushM_one st.closed
      (ClosedChapter.mk st.open (core.num.Usize.saturating_sub (Vec.len st.gains) 1#usize) .Rule)
    have ⟨l2, hl2, hl21⟩ := grew_push st.closed
      (ClosedChapter.mk st.open (core.num.Usize.saturating_sub (Vec.len st.gains) 1#usize) .Rule)
    have : l = l2 := by rw [hl] at hl2; exact List.append_cancel_left hl2
    subst this
    refine ⟨l, hl, hl21, ?_, ?_⟩
    · intro c hc; rw [hx c hc]; simp
    · intro _; simp [MAX]
  · exact ⟨[], by simp, by simp, by simp, fun h => h⟩

/-- The invariant of the weights: every closed chapter keeps the rule of
theorem 6 and the bound of theorem 8, and the open chapter is below `MAX`. -/
def WeightsOk (st : Fold) : Prop :=
  (∀ c ∈ st.closed.val, (c.close = .Rule ∨ MIN ≤ c.chapter.weight.val) ∧
    c.chapter.weight.val ≤ MAX - 1 + 7) ∧ st.open.weight.val < MAX

theorem weightsOk_start : WeightsOk startM := by
  simp [WeightsOk, startM, MAX]

theorem weightsOk_apply (st : Fold) (s : Step) (h : WeightsOk st) : WeightsOk (applyM st s) := by
  obtain ⟨hc, ho⟩ := h
  cases s with
  | Play p =>
    obtain ⟨l, hl, hx, hopen⟩ := applyPlayM_new_closed st p
    refine ⟨?_, hopen ho⟩
    intro c hmem
    rw [hl, List.mem_append] at hmem
    rcases hmem with hmem | hmem
    · exact hc c hmem
    · have := hx c hmem
      exact ⟨Or.inr this.2.1, this.2.2 ho⟩
  | Rule r =>
    obtain ⟨l, hl, _, hx, hopen⟩ := applyRuleM_new_closed st r
    refine ⟨?_, hopen ho⟩
    intro c hmem
    rw [hl, List.mem_append] at hmem
    rcases hmem with hmem | hmem
    · exact hc c hmem
    · have := hx c hmem
      refine ⟨Or.inl this.1, ?_⟩
      rw [this.2]; simp only [MAX] at *; omega

theorem weightsOk_runM (ss : List Step) : WeightsOk (runM ss) := by
  induction ss using List.reverseRecOn with
  | nil => exact weightsOk_start
  | append_singleton ss s ih => rw [runM_snoc]; exact weightsOk_apply _ _ ih

/-- Theorem 6. Every closed chapter weighs `MIN` or more, or a rule step
closed it. -/
theorem a_closed_chapter_has_min_weight (ss : List Step) :
    ∀ c ∈ (runM ss).closed.val, c.close = .Rule ∨ MIN ≤ c.chapter.weight.val :=
  fun c hc => ((weightsOk_runM ss).1 c hc).1

/-- Theorem 7. A rule step closes at most one chapter. A step of play closes
none below `MIN`, and none as a rule. This holds from any fold. -/
theorem a_rule_change_closes_at_most_one_chapter (st : Fold) :
    (∀ r, GrewByOne st.closed (applyM st (.Rule r)).closed) ∧
    (∀ p, ∃ l, (applyM st (.Play p)).closed.val = st.closed.val ++ l ∧
      ∀ c ∈ l, c.close ≠ .Rule ∧ MIN ≤ c.chapter.weight.val) := by
  refine ⟨fun r => applyM_closed st (.Rule r), fun p => ?_⟩
  obtain ⟨l, hl, hx, _⟩ := applyPlayM_new_closed st p
  exact ⟨l, hl, fun c hc => ⟨(hx c hc).1, (hx c hc).2.1⟩⟩

/-- Theorem 8. No chapter weighs more than `MAX - 1 + W_MAX`, with `W_MAX` 7,
and the open chapter weighs less than `MAX`. -/
theorem no_chapter_passes_max_and_one_step (ss : List Step) :
    (∀ c ∈ (runM ss).closed.val, c.chapter.weight.val ≤ MAX - 1 + 7) ∧
      (runM ss).open.weight.val < MAX :=
  ⟨fun c hc => ((weightsOk_runM ss).1 c hc).2, (weightsOk_runM ss).2⟩

/-! ## Theorem 15: one tale for each instance -/

/-- The instances of the tales, in order. -/
def insts (t : Vec Tale) : List Usize := t.val.map (·.instance)

theorem findFrom_some (l : List Tale) (inst : Usize) (i j : Nat) (h : findFrom l inst i = some j) :
    ∃ hj : j < l.length, l[j].instance = inst := by
  induction i using findFrom.induct (l := l) (inst := inst) with
  | case1 i hi he =>
    rw [findFrom_found l inst i hi he] at h
    cases h; exact ⟨hi, he⟩
  | case2 i hi he ih =>
    rw [findFrom_next l inst i hi he] at h
    exact ih h
  | case3 i hi =>
    rw [findFrom_end l inst i hi] at h
    cases h

theorem findFrom_none (l : List Tale) (inst : Usize) (i : Nat) (h : findFrom l inst i = none) :
    ∀ j (hj : j < l.length), i ≤ j → l[j].instance ≠ inst := by
  induction i using findFrom.induct (l := l) (inst := inst) with
  | case1 i hi he =>
    rw [findFrom_found l inst i hi he] at h
    cases h
  | case2 i hi he ih =>
    rw [findFrom_next l inst i hi he] at h
    intro j hj hij
    by_cases hje : j = i
    · subst hje; exact he
    · exact ih h j hj (by omega)
  | case3 i hi =>
    intro j hj hij; omega

theorem insts_set (t : Vec Tale) (i : Usize) (x : Tale)
    (h : ∀ hi : i.val < t.val.length, x.instance = (t.val[i.val]'hi).instance) :
    insts (t.set i x) = insts t := by
  unfold insts
  simp only [Vec.set_val_eq]
  apply List.ext_getElem
  · simp
  · intro n h1 h2
    simp only [List.getElem_map, List.getElem_set]
    split
    · rename_i hn; subst hn; exact h (by simpa using h2)
    · rfl

theorem closeVisitM_insts (t : Vec Tale) (vs : Vec Visit) (v : Visit) :
    insts (closeVisitM t vs v).1 = insts t := by
  unfold closeVisitM
  split
  · rename_i hv
    apply insts_set
    intro hi
    simp [getM, List.getD, hi]
  · rfl

/-- A step adds a tale only for an instance that has none. -/
def InstsGrow (a b : Vec Tale) : Prop :=
  insts b = insts a ∨ ∃ inst, inst ∉ insts a ∧ insts b = insts a ++ [inst]

theorem instsGrow_refl (a : Vec Tale) : InstsGrow a a := Or.inl rfl

theorem taleOfM_insts (t : Vec Tale) (inst here : Usize) :
    InstsGrow t (taleOfM t inst here).2 := by
  unfold taleOfM taleFromM
  split
  · exact instsGrow_refl _
  · rename_i hn
    unfold pushM
    split
    · refine Or.inr ⟨inst, ?_, by simp [insts]⟩
      intro hm
      simp only [insts, List.mem_map] at hm
      obtain ⟨x, hx, hxi⟩ := hm
      obtain ⟨j, hj, hjx⟩ := List.getElem_of_mem hx
      exact findFrom_none _ _ _ hn j hj (Nat.zero_le _) (by rw [hjx]; exact hxi)
    · exact instsGrow_refl _

theorem leaveM_insts (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (now : U64) :
    insts (leaveM t vs o now).1 = insts t := by
  unfold leaveM
  rcases o with _ | v
  · rfl
  · simp only; split
    · exact closeVisitM_insts _ _ _
    · rfl

theorem enterM_insts (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) : InstsGrow t (enterM t vs o inst here now).1 := by
  have hg := taleOfM_insts t inst here
  unfold enterM
  rcases o with _ | v
  · exact hg
  · simp only; split
    · exact hg
    · unfold InstsGrow at *; rw [closeVisitM_insts]; exact hg

theorem applyM_insts (st : Fold) (s : Step) : InstsGrow st.tales (applyM st s).tales := by
  cases s with
  | Rule r => simp only [applyM, applyRuleM]; split <;> exact instsGrow_refl _
  | Play p =>
    simp only [applyM, applyPlayM, visitsOfM]
    rcases p.track with _ | i
    · exact Or.inl (leaveM_insts _ _ _ _)
    · exact enterM_insts _ _ _ _ _ _

theorem instsGrow_nodup (a b : Vec Tale) (h : InstsGrow a b) (hn : (insts a).Nodup) :
    (insts b).Nodup := by
  rcases h with h | ⟨inst, hi, h⟩
  · rw [h]; exact hn
  · rw [h, List.nodup_append]
    exact ⟨hn, List.nodup_singleton _, fun a ha b hb => by
      simp only [List.mem_singleton] at hb; subst hb; intro he; subst he; exact hi ha⟩

/-- Theorem 15. No two tales share an instance. -/
theorem an_instance_has_at_most_one_tale (ss : List Step) : (insts (runM ss).tales).Nodup := by
  induction ss using List.reverseRecOn with
  | nil => simp [runM, startM, insts]
  | append_singleton ss s ih =>
    rw [runM_snoc]
    exact instsGrow_nodup _ _ (applyM_insts _ _) ih

/-! ## Theorem 5: a tale changes only with a closed visit -/

/-- What one step may do to tale `j`: keep its instance and its first step,
and keep its weight and runs, or add one closed visit of the tale. -/
def TaleMoved (j : Nat) (t t' : Tale) (vs vs' : Vec Visit) : Prop :=
  t'.instance = t.instance ∧ t'.first = t.first ∧
    ((t'.weight = t.weight ∧ t'.runs = t.runs) ∨
     ∃ v, vs'.val = vs.val ++ [v] ∧ v.tale.val = j ∧
       t'.weight = core.num.U32.saturating_add t.weight v.gain ∧
       t'.runs = core.num.U32.saturating_add t.runs 1#u32)

theorem taleMoved_refl (j : Nat) (t : Tale) (vs : Vec Visit) : TaleMoved j t t vs vs :=
  ⟨rfl, rfl, Or.inl ⟨rfl, rfl⟩⟩

theorem taleOfM_get (t : Vec Tale) (inst here : Usize) (j : Nat) (x : Tale)
    (hx : t.val[j]? = some x) : (taleOfM t inst here).2.val[j]? = some x := by
  obtain ⟨l, hl, _⟩ := taleOfM_tales t inst here
  rw [hl, List.getElem?_append_left (by
    have := (List.getElem?_eq_some_iff.mp hx).1; exact this)]
  exact hx

theorem closeVisitM_get (t : Vec Tale) (vs : Vec Visit) (v : Visit) (j : Nat) (x : Tale)
    (hx : t.val[j]? = some x) (hroom : vs.val.length < Usize.max) :
    ∃ x', (closeVisitM t vs v).1.val[j]? = some x' ∧
      TaleMoved j x x' vs (closeVisitM t vs v).2 := by
  have hj := (List.getElem?_eq_some_iff.mp hx).1
  unfold closeVisitM
  split
  · rename_i hv
    by_cases hjv : v.tale.val = j
    · subst hjv
      have hg : getM t v.tale NO_TALE = x := by
        simp only [getM, List.getD]; rw [hx]; rfl
      simp only [hg]
      refine ⟨{ x with weight := core.num.U32.saturating_add x.weight v.gain,
                       runs := core.num.U32.saturating_add x.runs 1#u32 }, ?_, ?_⟩
      · simp only [Vec.set_val_eq]
        rw [List.getElem?_set_self (by simpa using hv)]
      · exact ⟨rfl, rfl, Or.inr ⟨v, pushM_val _ _ hroom, rfl, rfl, rfl⟩⟩
    · refine ⟨x, ?_, rfl, rfl, ?_⟩
      · simp only [Vec.set_val_eq]
        rw [List.getElem?_set_ne (by omega)]
        exact hx
      · exact Or.inl ⟨rfl, rfl⟩
  · rename_i hv
    have : ¬ v.tale.val = j := by omega
    exact ⟨x, hx, rfl, rfl, Or.inl ⟨rfl, rfl⟩⟩

theorem leaveM_get (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (now : U64) (j : Nat)
    (x : Tale) (hx : t.val[j]? = some x) (hroom : vs.val.length < Usize.max) :
    ∃ x', (leaveM t vs o now).1.val[j]? = some x' ∧
      TaleMoved j x x' vs (leaveM t vs o now).2.1 := by
  unfold leaveM
  rcases o with _ | v
  · exact ⟨x, hx, taleMoved_refl _ _ _⟩
  · simp only; split
    · exact closeVisitM_get _ _ _ _ _ hx hroom
    · exact ⟨x, hx, taleMoved_refl _ _ _⟩

theorem enterM_get (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) (j : Nat) (x : Tale) (hx : t.val[j]? = some x)
    (hroom : vs.val.length < Usize.max) :
    ∃ x', (enterM t vs o inst here now).1.val[j]? = some x' ∧
      TaleMoved j x x' vs (enterM t vs o inst here now).2.1 := by
  have h1 := taleOfM_get t inst here j x hx
  unfold enterM
  rcases o with _ | v
  · exact ⟨x, h1, taleMoved_refl _ _ _⟩
  · simp only; split
    · exact ⟨x, h1, taleMoved_refl _ _ _⟩
    · exact closeVisitM_get _ _ _ _ _ h1 hroom

/-- Theorem 5. A step keeps the instance and the first step of each tale. It
keeps its weight and its runs, or adds exactly one closed visit of that tale:
the weight grows by the gain of the visit, and the runs by one. So the weight
and the runs only grow. The visits have room, as in every fold of a log. -/
theorem a_tale_changes_only_with_a_visit (st : Fold) (s : Step)
    (hroom : st.visits.val.length < Usize.max) (j : Nat) (t : Tale)
    (hj : st.tales.val[j]? = some t) :
    ∃ t', (applyM st s).tales.val[j]? = some t' ∧
      TaleMoved j t t' st.visits (applyM st s).visits := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]
    split <;> exact ⟨t, hj, taleMoved_refl _ _ _⟩
  | Play p =>
    simp only [applyM, applyPlayM, visitsOfM]
    rcases p.track with _ | i
    · exact leaveM_get _ _ _ _ _ _ hj hroom
    · exact enterM_get _ _ _ _ _ _ _ _ hj hroom

/-- The weight and the runs of a tale only grow. -/
theorem taleMoved_grows (j : Nat) (t t' : Tale) (vs vs' : Vec Visit) (h : TaleMoved j t t' vs vs') :
    t.weight.val ≤ t'.weight.val ∧ t.runs.val ≤ t'.runs.val := by
  obtain ⟨_, _, h | ⟨v, _, _, hw, hr⟩⟩ := h
  · rw [h.1, h.2]; exact ⟨le_refl _, le_refl _⟩
  · rw [hw, hr, u32_sat_add, u32_sat_add, sat_add_val, sat_add_val]
    have h1 := t.weight.hBounds
    have h2 := t.runs.hBounds
    simp only [UScalar.max] at *
    constructor <;> omega

/-! ## Records only grow -/

/-- Each slot of `a` is in `b`, as large or larger. -/
def VecLe {α : Type} (le : α → α → Prop) (a b : Vec α) : Prop :=
  ∀ (j : Nat) x, a.val[j]? = some x → ∃ y, b.val[j]? = some y ∧ le x y

def KeyLe (r r' : KeyRecord) : Prop :=
  (r.seen = true → r'.seen = true) ∧ r.deaths.val ≤ r'.deaths.val ∧ r.gain.val ≤ r'.gain.val

def FoeLe (f f' : FoeRecord) : Prop :=
  (f.beaten = true → f'.beaten = true) ∧ f.deaths.val ≤ f'.deaths.val

theorem keyLe_refl (r : KeyRecord) : KeyLe r r := ⟨id, le_refl _, le_refl _⟩
theorem foeLe_refl (f : FoeRecord) : FoeLe f f := ⟨id, le_refl _⟩

theorem keyLe_trans {a b c : KeyRecord} (h1 : KeyLe a b) (h2 : KeyLe b c) : KeyLe a c :=
  ⟨fun h => h2.1 (h1.1 h), le_trans h1.2.1 h2.2.1, le_trans h1.2.2 h2.2.2⟩

theorem foeLe_trans {a b c : FoeRecord} (h1 : FoeLe a b) (h2 : FoeLe b c) : FoeLe a c :=
  ⟨fun h => h2.1 (h1.1 h), le_trans h1.2 h2.2⟩

theorem vecLe_refl {α : Type} (le : α → α → Prop) (hr : ∀ x, le x x) (a : Vec α) : VecLe le a a :=
  fun _ x hx => ⟨x, hx, hr x⟩

theorem vecLe_trans {α : Type} (le : α → α → Prop) (ht : ∀ {x y z}, le x y → le y z → le x z)
    {a b c : Vec α} (h1 : VecLe le a b) (h2 : VecLe le b c) : VecLe le a c := by
  intro j x hx
  obtain ⟨y, hy, hxy⟩ := h1 j x hx
  obtain ⟨z, hz, hyz⟩ := h2 j y hy
  exact ⟨z, hz, ht hxy hyz⟩

theorem vecLe_slotted {α : Type} (le : α → α → Prop) (hr : ∀ x, le x x) (a : Vec α) (i : Usize)
    (d : α) : VecLe le a (slotted a i d) := by
  intro j x hx
  refine ⟨x, ?_, hr x⟩
  unfold slotted
  split
  · obtain ⟨l, hl, _⟩ := grew_push a d
    rw [hl, List.getElem?_append_left (List.getElem?_eq_some_iff.mp hx).1]
    exact hx
  · exact hx

theorem vecLe_set {α : Type} (le : α → α → Prop) (hr : ∀ x, le x x) (a : Vec α) (i : Usize) (y : α)
    (h : ∀ x, a.val[i.val]? = some x → le x y) : VecLe le a (a.set i y) := by
  intro j x hx
  have hj := (List.getElem?_eq_some_iff.mp hx).1
  by_cases hij : i.val = j
  · subst hij
    exact ⟨y, by simp [Vec.set_val_eq, hj], h x hx⟩
  · exact ⟨x, by simp only [Vec.set_val_eq]; rw [List.getElem?_set_ne hij]; exact hx, hr x⟩

theorem getM_of_get {α : Type} (v : Vec α) (i : Usize) (d x : α) (h : v.val[i.val]? = some x) :
    getM v i d = x := by
  simp [getM, List.getD, h]

theorem oneMoreDeathM_ge (d : U8) : d.val ≤ (oneMoreDeathM d).val := by
  unfold oneMoreDeathM
  split
  · rw [u8_sat_add, sat_add_val]; simp only [UScalar.max]; scalar_tac
  · exact le_refl _

theorem slotted_get_some {α : Type} (a : Vec α) (i : Usize) (d : α) (j : Nat) (x : α)
    (hx : a.val[j]? = some x) : (slotted a i d).val[j]? = some x := by
  obtain ⟨y, hy, hxy⟩ := vecLe_slotted (fun x y => x = y) (fun _ => rfl) a i d j x hx
  rw [hy, hxy]

theorem killGainM_foes (foes : Vec FoeRecord) (r : KeyRecord) (key : Key) :
    VecLe FoeLe foes (killGainM foes r key).2 := by
  unfold killGainM
  rcases key.foe with _ | f
  · exact vecLe_refl _ foeLe_refl _
  · simp only
    split
    · apply vecLe_trans FoeLe foeLe_trans (vecLe_slotted FoeLe foeLe_refl foes f UNBEATEN)
      generalize slotted foes f UNBEATEN = V
      split_ifs <;> apply vecLe_set _ foeLe_refl <;> intro x hx <;>
        rw [getM_of_get _ _ _ _ hx] <;> simp [FoeLe]
    · exact vecLe_refl _ foeLe_refl _

theorem deathGainM_foes (foes : Vec FoeRecord) (r : KeyRecord) (foe : Option Usize) :
    VecLe FoeLe foes (deathGainM foes r foe).2.1 ∧ KeyLe r (deathGainM foes r foe).2.2 := by
  unfold deathGainM
  rcases foe with _ | f
  · exact ⟨vecLe_refl _ foeLe_refl _, id, oneMoreDeathM_ge _, le_refl _⟩
  · simp only
    split
    · refine ⟨?_, ?_⟩
      · apply vecLe_trans FoeLe foeLe_trans (vecLe_slotted FoeLe foeLe_refl foes f UNBEATEN)
        generalize slotted foes f UNBEATEN = V
        split_ifs with hb <;> apply vecLe_set _ foeLe_refl <;> intro x hx <;>
          rw [getM_of_get _ _ _ _ hx] at hb ⊢ <;> exact ⟨by simp [hb], oneMoreDeathM_ge _⟩
      · split_ifs <;> exact keyLe_refl _
    · exact ⟨vecLe_refl _ foeLe_refl _, keyLe_refl _⟩

theorem rawGainM_le (foes : Vec FoeRecord) (r : KeyRecord) (key : Key) :
    VecLe FoeLe foes (rawGainM foes r key).2.2.1 ∧ KeyLe r (rawGainM foes r key).2.2.2 := by
  unfold rawGainM
  rcases hk : key.kind
  all_goals simp only
  all_goals first
    | exact ⟨vecLe_refl _ foeLe_refl _, keyLe_refl _⟩
    | exact ⟨killGainM_foes _ _ _, keyLe_refl _⟩
    | exact deathGainM_foes _ _ _

theorem gainM_le (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Option Key) :
    VecLe KeyLe keys (gainM keys foes key).2.1 ∧ VecLe FoeLe foes (gainM keys foes key).2.2 := by
  unfold gainM
  rcases key with _ | key
  · exact ⟨vecLe_refl _ keyLe_refl _, vecLe_refl _ foeLe_refl _⟩
  · simp only
    split
    · have hraw := rawGainM_le foes (getM (slotted keys key.id UNSEEN) key.id UNSEEN) key
      generalize rawGainM foes (getM (slotted keys key.id UNSEEN) key.id UNSEEN) key = R at *
      obtain ⟨raw, rv, f1, r1⟩ := R
      simp only at hraw ⊢
      refine ⟨?_, hraw.1⟩
      apply vecLe_trans KeyLe keyLe_trans (vecLe_slotted KeyLe keyLe_refl keys key.id UNSEEN)
      apply vecLe_set _ keyLe_refl
      intro x hx
      rw [getM_of_get _ _ _ _ hx] at hraw
      refine keyLe_trans hraw.2 ⟨fun _ => rfl, le_refl _, ?_⟩
      simp only
      rw [u16_sat_add, sat_add_val]
      have := r1.gain.hBounds
      simp only [UScalar.max] at *
      omega
    · exact ⟨vecLe_refl _ keyLe_refl _, vecLe_refl _ foeLe_refl _⟩

/-- Key and foe records only grow with a step. -/
theorem applyM_le (st : Fold) (s : Step) :
    VecLe KeyLe st.keys (applyM st s).keys ∧ VecLe FoeLe st.foes (applyM st s).foes := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]
    split <;> exact ⟨vecLe_refl _ keyLe_refl _, vecLe_refl _ foeLe_refl _⟩
  | Play p =>
    simp only [applyM, applyPlayM]
    rcases p.track with _ | i <;> exact gainM_le _ _ _

/-! ## Theorems 11 and 16: a repeat and a death to a beaten foe gain nothing -/

theorem slotted_of_lt {α : Type} (v : Vec α) (i : Usize) (d : α) (h : i.val < v.val.length) :
    slotted v i d = v := by
  unfold slotted; simp; omega

theorem amount_of_raw_zero (raw room : U16) (h : raw.val = 0) :
    (if raw < room then raw else room).val = 0 := by
  split
  · exact h
  · rename_i hlt; scalar_tac

/-- The gain of a key when the raw gain is 0. -/
theorem gainM_zero (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Key)
    (h : ∀ r, keys.val[key.id.val]? = some r → (rawGainM foes r key).1.val = 0)
    (hlt : key.id.val < keys.val.length) :
    (gainM keys foes (some key)).1.1.val = 0 := by
  unfold gainM
  have hs : hasSlotM keys.val.length key.id.val = true := by simp [hasSlotM, hlt]
  simp only [hs, if_true, slotted_of_lt _ _ _ hlt]
  have hr : keys.val[key.id.val]? = some (keys.val[key.id.val]'hlt) := by simp
  rw [getM_of_get _ _ _ _ hr]
  have := h _ hr
  generalize rawGainM foes keys.val[key.id.val] key = R at *
  obtain ⟨raw, rv, f1, r1⟩ := R
  exact amount_of_raw_zero _ _ this

/-- A key that can gain nothing more: a seen key of any kind but a death, or
a death key whose foe is beaten or whose deaths reached 2. -/
def Spent (st : Fold) (key : Key) : Prop :=
  ∃ r, st.keys.val[key.id.val]? = some r ∧
    ((key.kind ≠ .Death ∧ r.seen = true) ∨
     (key.kind = .Death ∧ key.foe = none ∧ 2 ≤ r.deaths.val) ∨
     (key.kind = .Death ∧ ∃ f fr, key.foe = some f ∧ st.foes.val[f.val]? = some fr ∧
        (fr.beaten = true ∨ 2 ≤ fr.deaths.val)))

theorem deathWeightM_zero (d : U8) (h : 2 ≤ d.val) : (deathWeightM d).val = 0 := by
  unfold deathWeightM
  have h0 : ¬ d.val = 0 := by omega
  have h1 : ¬ d.val = 1 := by omega
  simp [h0, h1]

theorem rawGainM_beaten_death (foes : Vec FoeRecord) (r : KeyRecord) (key : Key) (f : Usize)
    (fr : FoeRecord) (hk : key.kind = .Death) (hf : key.foe = some f)
    (hfr : foes.val[f.val]? = some fr) (hb : fr.beaten = true ∨ 2 ≤ fr.deaths.val) :
    (rawGainM foes r key).1.val = 0 := by
  have hlt := (List.getElem?_eq_some_iff.mp hfr).1
  unfold rawGainM deathGainM
  simp only [hk, hf]
  have hs : hasSlotM foes.val.length f.val = true := by simp [hasSlotM, hlt]
  simp only [hs, if_true, slotted_of_lt _ _ _ hlt, getM_of_get _ _ _ _ hfr]
  split
  · rfl
  · rename_i hnb
    rcases hb with hb | hb
    · exact absurd hb hnb
    · exact deathWeightM_zero _ hb

theorem rawGainM_spent (st : Fold) (key : Key) (r : KeyRecord) (hr : st.keys.val[key.id.val]? = some r)
    (h : Spent st key) : (rawGainM st.foes r key).1.val = 0 := by
  obtain ⟨r0, hr0, h⟩ := h
  rw [hr] at hr0; cases hr0
  rcases h with ⟨hk, hs⟩ | ⟨hk, hf, hd⟩ | ⟨hk, f, fr, hf, hfr, hb⟩
  · unfold rawGainM
    rcases hkk : key.kind
    all_goals simp only [hkk] at hk ⊢
    all_goals first
      | exact absurd rfl hk
      | (simp [killGainM, firstTimeGainM, hs]; split <;> (try split_ifs) <;> rfl)
      | simp [firstTimeGainM, hs]
  · unfold rawGainM deathGainM
    simp only [hk, hf]
    exact deathWeightM_zero _ hd
  · exact rawGainM_beaten_death _ _ _ _ _ hk hf hfr hb

theorem spent_mono (st st' : Fold) (key : Key) (hk : VecLe KeyLe st.keys st'.keys)
    (hf : VecLe FoeLe st.foes st'.foes) (h : Spent st key) : Spent st' key := by
  obtain ⟨r, hr, h⟩ := h
  obtain ⟨r', hr', hle⟩ := hk _ _ hr
  refine ⟨r', hr', ?_⟩
  rcases h with ⟨h1, h2⟩ | ⟨h1, h2, h3⟩ | ⟨h1, f, fr, h2, h3, h4⟩
  · exact Or.inl ⟨h1, hle.1 h2⟩
  · exact Or.inr (Or.inl ⟨h1, h2, le_trans h3 hle.2.1⟩)
  · obtain ⟨fr', hfr', hfle⟩ := hf _ _ h3
    refine Or.inr (Or.inr ⟨h1, f, fr', h2, hfr', ?_⟩)
    rcases h4 with h4 | h4
    · exact Or.inl (hfle.1 h4)
    · exact Or.inr (le_trans h4 hfle.2)

/-- Theorem 11. A step with a spent key gains 0, and the key stays spent after
any step. This holds from any fold. -/
theorem a_repeat_never_adds_weight (st : Fold) (key : Key) (h : Spent st key) (s : Step) :
    (gainM st.keys st.foes (some key)).1.1.val = 0 ∧ Spent (applyM st s) key := by
  have h' := h
  obtain ⟨r, hr, _⟩ := h'
  refine ⟨?_, spent_mono _ _ _ (applyM_le st s).1 (applyM_le st s).2 h⟩
  apply gainM_zero _ _ _ _ (List.getElem?_eq_some_iff.mp hr).1
  intro r' hr'
  exact rawGainM_spent st key r' hr' h

/-- Foe `f` is beaten in the fold. -/
def Beaten (st : Fold) (f : Usize) : Prop :=
  ∃ fr, st.foes.val[f.val]? = some fr ∧ fr.beaten = true

/-- Theorem 16. Once a foe is beaten, a death to it gains 0, on either track
and under every rule, and the foe stays beaten after any step. -/
theorem a_death_to_a_beaten_foe_weighs_nothing (st : Fold) (f : Usize) (h : Beaten st f) :
    (∀ key, key.kind = .Death → key.foe = some f →
      (gainM st.keys st.foes (some key)).1.1.val = 0) ∧
    ∀ s, Beaten (applyM st s) f := by
  obtain ⟨fr, hfr, hb⟩ := h
  refine ⟨fun key hk hf => ?_, fun s => ?_⟩
  · by_cases hlt : key.id.val < st.keys.val.length
    · apply gainM_zero _ _ _ _ hlt
      intro r _
      exact rawGainM_beaten_death _ _ _ _ _ hk hf hfr (Or.inl hb)
    · unfold gainM
      simp only
      by_cases hs : hasSlotM st.keys.val.length key.id.val = true
      · simp only [hs, if_true]
        generalize getM (slotted st.keys key.id UNSEEN) key.id UNSEEN = r
        have := rawGainM_beaten_death st.foes r key f fr hk hf hfr (Or.inl hb)
        generalize rawGainM st.foes r key = R at *
        obtain ⟨raw, rv, f1, r1⟩ := R
        exact amount_of_raw_zero _ _ this
      · simp [hs]
  · obtain ⟨fr', hfr', hle⟩ := (applyM_le st s).2 _ _ hfr
    exact ⟨fr', hfr', hle.1 hb⟩

/-! ## Theorems 17 and 18: deaths and revenge of one foe -/

theorem gainM_some (keys : Vec KeyRecord) (foes : Vec FoeRecord) (k : Key)
    (hs : hasSlotM keys.val.length k.id.val = true) :
    let r := getM (slotted keys k.id UNSEEN) k.id UNSEEN
    (gainM keys foes (some k)).2.2 = (rawGainM foes r k).2.2.1 ∧
      (gainM keys foes (some k)).1.2 = (rawGainM foes r k).2.1 ∧
      (gainM keys foes (some k)).1.1.val ≤ (rawGainM foes r k).1.val := by
  simp only [gainM, hs, if_true]
  generalize rawGainM foes (getM (slotted keys k.id UNSEEN) k.id UNSEEN) k = R
  obtain ⟨raw, rv, f1, r1⟩ := R
  simp only [true_and]
  split <;> scalar_tac

theorem gainM_noslot (keys : Vec KeyRecord) (foes : Vec FoeRecord) (k : Key)
    (hs : ¬ hasSlotM keys.val.length k.id.val = true) :
    gainM keys foes (some k) = ((0#u16, false), keys, foes) := by
  simp [gainM, hs]

/-- The deaths to foe `f` that the foes record, or 0 out of range. -/
def foeDeaths (foes : Vec FoeRecord) (f : Usize) : Nat :=
  (foes.val[f.val]?.map (fun fr => fr.deaths.val)).getD 0

/-- What deaths to a foe can still add: 3, then 1, then 0. -/
def deathRoom (d : Nat) : Nat := if d = 0 then 3 else if d = 1 then 1 else 0

theorem deathRoom_anti {a b : Nat} (h : a ≤ b) : deathRoom b ≤ deathRoom a := by
  unfold deathRoom; split_ifs <;> omega

theorem foeDeaths_mono (a b : Vec FoeRecord) (h : VecLe FoeLe a b) (f : Usize) :
    foeDeaths a f ≤ foeDeaths b f := by
  unfold foeDeaths
  rcases ha : a.val[f.val]? with _ | x
  · simp
  · obtain ⟨y, hy, hxy⟩ := h _ _ ha
    simp [hy, hxy.2]

theorem oneMoreDeathM_val (d : U8) :
    (oneMoreDeathM d).val = if d.val < 2 then d.val + 1 else d.val := by
  unfold oneMoreDeathM
  split
  · rename_i h; rw [u8_sat_add, sat_add_val]; simp only [UScalar.max]; simp [h]; scalar_tac
  · rename_i h; simp [h]

theorem rawGainM_death_room (foes : Vec FoeRecord) (r : KeyRecord) (k : Key) (f : Usize)
    (hk : k.kind = .Death) (hf : k.foe = some f) :
    (rawGainM foes r k).1.val + deathRoom (foeDeaths (rawGainM foes r k).2.2.1 f) ≤
      deathRoom (foeDeaths foes f) := by
  unfold rawGainM deathGainM
  simp only [hk, hf]
  split
  · rename_i hs
    have hlt := slotted_lt foes f UNBEATEN hs
    have hbefore : foeDeaths foes f = (getM (slotted foes f UNBEATEN) f UNBEATEN).deaths.val := by
      unfold foeDeaths getM slotted
      split
      · rename_i he
        have : foes.val[f.val]? = none := by simp; omega
        rw [this, pushM_val _ _ (by simp [hasSlotM] at hs; omega)]
        simp [he, UNBEATEN]
      · rename_i he
        have hl : f.val < foes.val.length := by simp [hasSlotM] at hs; omega
        simp [hl, List.getD]
    generalize hV : slotted foes f UNBEATEN = V at *
    have hafter : ∀ x : FoeRecord, foeDeaths (V.set f x) f = x.deaths.val := by
      intro x; unfold foeDeaths; simp [Vec.set_val_eq, hlt]
    rw [hbefore]
    split
    · rw [hafter]
      have h0 : (0#u16 : U16).val = 0 := rfl
      rw [h0, Nat.zero_add]
      exact deathRoom_anti (oneMoreDeathM_ge _)
    · rw [hafter]; simp only
      rw [oneMoreDeathM_val]
      generalize (getM V f UNBEATEN).deaths = d
      unfold deathWeightM deathRoom
      split_ifs <;> simp_all
  · simp

/-- The deaths to foe `f` among the steps, with the gain of each step. -/
def deathOf (f : Usize) : Step → Bool
  | .Play p => match p.key with
    | some k => decide (k.kind = .Death) && decide (k.foe = some f)
    | none => false
  | .Rule _ => false

/-- The gain that the deaths to foe `f` of a log added. -/
def deathTotal (f : Usize) (ss : List Step) : Nat :=
  (((ss.zip (runM ss).gains.val).filter (fun sg => deathOf f sg.1)).map
    (fun sg => sg.2.amount.val)).sum

theorem zip_snoc {α β : Type} (a : List α) (b : List β) (x : α) (y : β) (h : a.length = b.length) :
    (a ++ [x]).zip (b ++ [y]) = a.zip b ++ [(x, y)] := by
  rw [List.zip_append h]; rfl

theorem deathTotal_snoc (f : Usize) (ss : List Step) (s : Step) (h : ss.length + 1 ≤ Usize.max) :
    deathTotal f (ss ++ [s]) = deathTotal f ss +
      (if deathOf f s then (gainOfStep (runM ss) s).amount.val else 0) := by
  unfold deathTotal
  rw [runM_gains ss s h, zip_snoc _ _ _ _ (runM_sized ss (by omega)).1.symm]
  rw [List.filter_append]
  split <;> simp_all

theorem death_step (st : Fold) (s : Step) (f : Usize) :
    (if deathOf f s then (gainOfStep st s).amount.val else 0) +
      deathRoom (foeDeaths (applyM st s).foes f) ≤ deathRoom (foeDeaths st.foes f) := by
  have hmono := foeDeaths_mono _ _ (applyM_le st s).2 f
  split
  · rename_i hd
    cases s with
    | Rule r => simp [deathOf] at hd
    | Play p =>
      rcases hkey : p.key with _ | k
      · simp [deathOf, hkey] at hd
      · simp only [deathOf, hkey, Bool.and_eq_true, decide_eq_true_eq] at hd
        have hfoes : (applyM st (.Play p)).foes = (gainM st.keys st.foes p.key).2.2 := by
          simp only [applyM, applyPlayM]; rcases p.track <;> rfl
        rw [hfoes]
        simp only [gainOfStep, hkey]
        by_cases hs : hasSlotM st.keys.val.length k.id.val = true
        · obtain ⟨h1, _, h3⟩ := gainM_some st.keys st.foes k hs
          rw [h1]
          have := rawGainM_death_room st.foes (getM (slotted st.keys k.id UNSEEN) k.id UNSEEN) k f
            hd.1 hd.2
          omega
        · rw [gainM_noslot _ _ _ hs]; simp
  · simp only [Nat.zero_add]; exact deathRoom_anti hmono

/-- Theorem 17. The deaths to one foe gain 3 at most in all: 2, then 1, then
0, and 0 once the foe is beaten. So 100 deaths to one mob never make a
chapter. -/
theorem deaths_to_one_foe_weigh_at_most_three (f : Usize) (ss : List Step)
    (h : ss.length ≤ Usize.max) : deathTotal f ss ≤ 3 := by
  have key : deathTotal f ss + deathRoom (foeDeaths (runM ss).foes f) ≤ 3 := by
    induction ss using List.reverseRecOn with
    | nil => simp [deathTotal, runM, startM, foeDeaths, deathRoom]
    | append_singleton ss s ih =>
      simp only [List.length_append, List.length_singleton] at h
      have ih := ih (by omega)
      rw [deathTotal_snoc f ss s h, runM_snoc]
      have := death_step (runM ss) s f
      omega
  omega

/-- Foe `f` is beaten in these foe records. -/
def BeatenIn (foes : Vec FoeRecord) (f : Usize) : Prop :=
  ∃ fr, foes.val[f.val]? = some fr ∧ fr.beaten = true

theorem killGainM_revenge (foes : Vec FoeRecord) (r : KeyRecord) (k : Key)
    (h : (killGainM foes r k).1.2 = true) :
    ∃ f, k.foe = some f ∧ ¬ BeatenIn foes f ∧ BeatenIn (killGainM foes r k).2 f := by
  unfold killGainM at *
  rcases hf : k.foe with _ | f
  · simp [hf] at h
  · simp only [hf] at h ⊢
    split at h
    · rename_i hs
      have hlt := slotted_lt foes f UNBEATEN hs
      simp only [hs, if_true]
      split_ifs at h ⊢ with h1 h2 h3 <;> try (simp at h; done)
      · refine ⟨f, rfl, ?_, ?_⟩
        · rintro ⟨fr, hfr, hb⟩
          have : getM (slotted foes f UNBEATEN) f UNBEATEN = fr :=
            getM_of_get _ _ _ _ (slotted_get_some _ _ _ _ _ hfr)
          rw [this] at h2; exact h2 hb
        · exact ⟨_, by simp only [Vec.set_val_eq]; exact List.getElem?_set_self hlt, rfl⟩
    · simp at h

theorem rawGainM_revenge (foes : Vec FoeRecord) (r : KeyRecord) (k : Key)
    (h : (rawGainM foes r k).2.1 = true) :
    ∃ f, k.foe = some f ∧ ¬ BeatenIn foes f ∧ BeatenIn (rawGainM foes r k).2.2.1 f := by
  unfold rawGainM at *
  rcases hk : k.kind
  all_goals simp only [hk] at h ⊢
  all_goals first
    | exact killGainM_revenge _ _ _ h
    | simp at h

/-- The foe of the key of a step, if any. -/
def keyFoe : Step → Option Usize
  | .Play p => match p.key with
    | some k => k.foe
    | none => none
  | .Rule _ => none

theorem revenge_step (st : Fold) (s : Step) (f : Usize)
    (h : (gainOfStep st s).revenge = true ∧ keyFoe s = some f) :
    ¬ BeatenIn st.foes f ∧ BeatenIn (applyM st s).foes f := by
  obtain ⟨hr, hf⟩ := h
  cases s with
  | Rule r => simp [keyFoe] at hf
  | Play p =>
    rcases hkey : p.key with _ | k
    · simp [keyFoe, hkey] at hf
    · simp only [keyFoe, hkey] at hf
      simp only [gainOfStep, hkey] at hr
      have hfoes : (applyM st (.Play p)).foes = (gainM st.keys st.foes p.key).2.2 := by
        simp only [applyM, applyPlayM]; rcases p.track <;> rfl
      rw [hfoes, hkey]
      by_cases hs : hasSlotM st.keys.val.length k.id.val = true
      · obtain ⟨h1, h2, _⟩ := gainM_some st.keys st.foes k hs
        rw [h2] at hr
        rw [h1]
        obtain ⟨f', hf', hnb, hb⟩ := rawGainM_revenge _ _ _ hr
        rw [hf] at hf'; cases hf'
        exact ⟨hnb, hb⟩
      · rw [gainM_noslot _ _ _ hs] at hr; simp at hr

/-- The steps of a log that added revenge for foe `f`. -/
def revengeCount (f : Usize) (ss : List Step) : Nat :=
  ((ss.zip (runM ss).gains.val).filter
    (fun sg => sg.2.revenge && decide (keyFoe sg.1 = some f))).length

theorem beatenIn_mono (a b : Vec FoeRecord) (h : VecLe FoeLe a b) (f : Usize)
    (hb : BeatenIn a f) : BeatenIn b f := by
  obtain ⟨fr, hfr, hbt⟩ := hb
  obtain ⟨fr', hfr', hle⟩ := h _ _ hfr
  exact ⟨fr', hfr', hle.1 hbt⟩

open Classical in
/-- Theorem 18. Only the first kill of a foe that had killed you adds revenge:
the steps of a log add revenge for one foe once at most. -/
theorem revenge_counts_once (f : Usize) (ss : List Step) (h : ss.length ≤ Usize.max) :
    revengeCount f ss ≤ 1 := by
  have key : revengeCount f ss + (if BeatenIn (runM ss).foes f then 0 else 1) ≤ 1 := by
    induction ss using List.reverseRecOn with
    | nil => simp [revengeCount, runM, startM, BeatenIn]
    | append_singleton ss s ih =>
      simp only [List.length_append, List.length_singleton] at h
      have ih := ih (by omega)
      have hsnoc : revengeCount f (ss ++ [s]) = revengeCount f ss +
          (if (gainOfStep (runM ss) s).revenge = true ∧ keyFoe s = some f then 1 else 0) := by
        unfold revengeCount
        rw [runM_gains ss s h, zip_snoc _ _ _ _ (runM_sized ss (by omega)).1.symm,
          List.filter_append]
        split <;> simp_all
      rw [hsnoc, runM_snoc]
      have hmono := beatenIn_mono _ _ (applyM_le (runM ss) s).2 f
      split
      · rename_i hrv
        obtain ⟨hnb, hb⟩ := revenge_step _ _ _ hrv
        simp only [hnb, hb, if_true, if_false] at ih ⊢
        omega
      · rename_i hrv
        simp only [Nat.add_zero]
        by_cases hb0 : BeatenIn (runM ss).foes f
        · simp only [hb0, hmono hb0, if_true] at ih ⊢; omega
        · simp only [hb0, if_false] at ih
          split <;> omega
  omega

/-! ## Theorems 1 and 9: chapter ranges and their weights -/

/-- What a step of the open world does to the chapters: it adds its gain to
the open chapter, or closes the open chapter at the most weight after it, or
closes the open chapter at a break before it. -/
theorem addChapterM_cases (zones : Vec ZoneRecord) (closed : Vec ClosedChapter) (opn : Chapter)
    (pending : Option Break) (zone : Usize) (amount : U16) (here : Usize)
    (ha : amount.val ≤ 7) (hw : opn.weight.val < MAX) (hc : closed.val.length < Usize.max) :
    let r := addChapterM zones closed opn pending zone amount here
    (r.2.1 = closed ∧ r.2.2.1.first = opn.first ∧ r.2.2.1.weight.val = opn.weight.val + amount.val) ∨
    (∃ x : Chapter, r.2.1.val = closed.val ++ [⟨x, here, .Max⟩] ∧ x.first = opn.first ∧
      x.weight.val = opn.weight.val + amount.val ∧
      r.2.2.1.first = core.num.Usize.saturating_add here 1#usize ∧ r.2.2.1.weight.val = 0) ∨
    (opn.first.val < here.val ∧
      r.2.1.val = closed.val ++ [⟨opn, core.num.Usize.saturating_sub here 1#usize, .Break⟩] ∧
      r.2.2.1.first = here ∧ r.2.2.1.weight.val = amount.val) := by
  have hsum : (core.num.U16.saturating_add opn.weight amount).val = opn.weight.val + amount.val := by
    rw [u16_sat_add, sat_add_val]; simp only [UScalar.max, MAX] at *; scalar_tac
  have hsum0 : (core.num.U16.saturating_add 0#u16 amount).val = amount.val := by
    rw [u16_sat_add, sat_add_val]; simp only [UScalar.max]; scalar_tac
  simp only
  unfold addChapterM
  split
  · rename_i h0
    left; refine ⟨rfl, rfl, ?_⟩; simp only; omega
  · simp only
    generalize withBreak pending (zoneBreakM zones closed.val.length zone) = p1
    rcases p1 with _ | cut
    · simp only
      split
      · exact Or.inr (Or.inl ⟨_, pushM_val _ _ hc, rfl, hsum, rfl, rfl⟩)
      · exact Or.inl ⟨rfl, rfl, hsum⟩
    · simp only
      by_cases hb : MIN ≤ opn.weight.val ∧ opn.first.val < here.val
      · rw [if_pos hb]
        simp only
        have : ¬ MAX ≤ (core.num.U16.saturating_add 0#u16 amount).val := by
          rw [hsum0]; simp only [MAX]; omega
        rw [if_neg this]
        exact Or.inr (Or.inr ⟨hb.2, pushM_val _ _ hc, rfl, hsum0⟩)
      · rw [if_neg hb]
        simp only
        split
        · exact Or.inr (Or.inl ⟨_, pushM_val _ _ hc, rfl, hsum, rfl, rfl⟩)
        · exact Or.inl ⟨rfl, rfl, hsum⟩

/-- The closed chapters cover the steps from `n` on in order, each with one
step or more, and the open chapter begins at `f`. -/
def Covers : Nat → List ClosedChapter → Nat → Prop
  | n, [], f => f = n
  | n, c :: cs, f => c.chapter.first.val = n ∧ n ≤ c.last.val ∧ Covers (c.last.val + 1) cs f

theorem covers_snoc (n : Nat) (cs : List ClosedChapter) (f : Nat) (c : ClosedChapter)
    (h : Covers n cs f) (hf : c.chapter.first.val = f) (hl : f ≤ c.last.val) :
    Covers n (cs ++ [c]) (c.last.val + 1) := by
  induction cs generalizing n with
  | nil => simp only [Covers] at *; subst h; exact ⟨hf, hl, rfl⟩
  | cons d ds ih =>
    simp only [Covers, List.cons_append] at *
    exact ⟨h.1, h.2.1, ih _ h.2.2⟩

theorem covers_bounds (n : Nat) (cs : List ClosedChapter) (f : Nat) (h : Covers n cs f) :
    n ≤ f ∧ ∀ c ∈ cs, n ≤ c.chapter.first.val ∧ c.chapter.first.val ≤ c.last.val ∧
      c.last.val + 1 ≤ f := by
  induction cs generalizing n with
  | nil => simp only [Covers] at h; simp [h]
  | cons d ds ih =>
    simp only [Covers] at h
    obtain ⟨h1, h2, h3⟩ := h
    obtain ⟨ih1, ih2⟩ := ih _ h3
    refine ⟨by omega, ?_⟩
    intro c hc
    rcases List.mem_cons.mp hc with hc | hc
    · subst hc; omega
    · have := ih2 c hc; omega

/-- The gain of a step for its chapter: its gain in the open world, else 0. -/
def worldGain (g : Gain) : Nat :=
  match g.track with
  | .World => g.amount.val
  | .Instance _ => 0

/-- The open world gain of the steps from `a` to `b`, `b` not in. -/
def worldSum (gs : List Gain) (a b : Nat) : Nat := (((gs.drop a).take (b - a)).map worldGain).sum

theorem worldSum_snoc_old (gs : List Gain) (g : Gain) (a b : Nat) (h : b ≤ gs.length) :
    worldSum (gs ++ [g]) a b = worldSum gs a b := by
  unfold worldSum
  by_cases ha : a ≤ gs.length
  · congr 2
    rw [List.drop_append_of_le_length ha]
    rw [List.take_append_of_le_length (by simp; omega)]
  · have : b - a = 0 := by omega
    simp [this]

theorem worldSum_snoc_new (gs : List Gain) (g : Gain) (a : Nat) (h : a ≤ gs.length) :
    worldSum (gs ++ [g]) a (gs.length + 1) = worldSum gs a gs.length + worldGain g := by
  unfold worldSum
  rw [List.drop_append_of_le_length h]
  rw [List.take_of_length_le (by simp; omega), List.take_of_length_le (by simp)]
  simp

theorem worldSum_snoc_new' (gs : List Gain) (g : Gain) (a : Nat) (h : a ≤ gs.length) :
    worldSum (gs ++ [g]) a (gs ++ [g]).length = worldSum gs a gs.length + worldGain g := by
  rw [List.length_append, List.length_singleton]; exact worldSum_snoc_new gs g a h

theorem worldSum_self (gs : List Gain) (a : Nat) : worldSum gs a a = 0 := by
  simp [worldSum]

/-- The invariant of the chapter ranges and their weights. -/
def ChaptersOk (st : Fold) : Prop :=
  Covers 0 st.closed.val st.open.first.val ∧ st.open.first.val ≤ st.gains.val.length ∧
    (∀ c ∈ st.closed.val,
      c.chapter.weight.val = worldSum st.gains.val c.chapter.first.val (c.last.val + 1)) ∧
    st.open.weight.val = worldSum st.gains.val st.open.first.val st.gains.val.length

theorem chaptersOk_start : ChaptersOk startM := by
  simp [ChaptersOk, startM, Covers, worldSum]

theorem usize_sat_add_one (x : Usize) (h : x.val < Usize.max) :
    (core.num.Usize.saturating_add x 1#usize).val = x.val + 1 := by
  rw [usize_sat_add, sat_add_val]
  have : UScalar.max UScalarTy.Usize = Usize.max := by
    simp [UScalar.max, Usize.max, Usize.numBits]
  rw [this]; simp; omega

theorem usize_sat_sub_one (x : Usize) :
    (core.num.Usize.saturating_sub x 1#usize).val = x.val - 1 := by
  rw [usize_sat_sub, sat_sub_val]; simp

theorem chaptersOk_apply (st : Fold) (s : Step) (h : ChaptersOk st) (hw : WeightsOk st)
    (hs : Sized st) (hg : st.gains.val.length < Usize.max) : ChaptersOk (applyM st s) := by
  obtain ⟨hcov, hfirst, hclosed, hopen⟩ := h
  have hcl : st.closed.val.length < Usize.max := lt_of_le_of_lt hs.1 hg
  have hgains := applyM_gains_eq st s hg
  have hb := covers_bounds _ _ _ hcov
  have hold : ∀ c ∈ st.closed.val, c.chapter.weight.val =
      worldSum (applyM st s).gains.val c.chapter.first.val (c.last.val + 1) := by
    intro c hc
    rw [hgains, worldSum_snoc_old _ _ _ _ (by have := (hb.2 c hc).2.2; omega)]
    exact hclosed c hc
  cases s with
  | Rule r =>
    by_cases hlt : st.open.first.val < st.gains.val.length
    · simp only [applyM, applyRuleM, Vec.len_val, Vec.length, hlt, if_true] at hgains hold ⊢
      refine ⟨?_, ?_, ?_, ?_⟩
      · rw [pushM_val _ _ hcl]
        have := covers_snoc 0 _ _ ⟨st.open, core.num.Usize.saturating_sub (Vec.len st.gains) 1#usize, .Rule⟩
          hcov rfl (by simp only [usize_sat_sub_one, Vec.len_val, Vec.length]; omega)
        simp only [usize_sat_sub_one, Vec.len_val, Vec.length] at this ⊢
        have he : st.gains.val.length - 1 + 1 = st.gains.val.length := by omega
        rw [he] at this; exact this
      · rw [hgains]; simp
      · intro c hc
        rw [pushM_val _ _ hcl, List.mem_append] at hc
        rcases hc with hc | hc
        · exact hold c hc
        · simp only [List.mem_singleton] at hc; subst hc
          simp only [usize_sat_sub_one, Vec.len_val, Vec.length]
          rw [hgains, worldSum_snoc_old _ _ _ _ (by omega)]
          have he : st.gains.val.length - 1 + 1 = st.gains.val.length := by omega
          rw [he]; exact hopen
      · rw [hgains]; simp only [Vec.len_val, Vec.length, List.length_append, List.length_singleton]
        rw [worldSum_snoc_new _ _ _ (le_refl _), worldSum_self]
        simp [worldGain, ZERO_GAIN, gainOfStep]
    · simp only [applyM, applyRuleM, Vec.len_val, Vec.length, hlt, if_false] at hgains hold ⊢
      have he : st.open.first.val = st.gains.val.length := by omega
      refine ⟨hcov, ?_, hold, ?_⟩
      · rw [hgains]; simp; omega
      · rw [hgains]; simp only [List.length_append, List.length_singleton]
        rw [he, worldSum_snoc_new _ _ _ (le_refl _), worldSum_self]
        rw [he, worldSum_self] at hopen
        simp [worldGain, ZERO_GAIN, gainOfStep, hopen]
  | Play p =>
    have ha := gainM_amount_le st.keys st.foes p.key
    rcases htr : p.track with _ | i
    · simp only [applyM, applyPlayM, gainOfStep, ChaptersOk, htr] at hgains hold ⊢
      rcases addChapterM_cases (noteZoneM st.zones p.zone) st.closed st.open (pendingOfM st p)
        p.zone (gainM st.keys st.foes p.key).1.1 (Vec.len st.gains) ha hw.2 hcl with
        ⟨h1, h2, h3⟩ | ⟨x, h1, h2, h3, h4, h5⟩ | ⟨h0, h1, h2, h3⟩
      · rw [h1, h2]
        refine ⟨hcov, by rw [hgains]; simp; omega, ?_, ?_⟩
        · intro c hc; exact hold c hc
        · rw [h3, hgains, worldSum_snoc_new' _ _ _ hfirst, ← hopen]
          simp [worldGain]
      · rw [h1, h4]
        try simp only [Vec.len_val, Vec.length] at h4 h1 ⊢
        have hh := usize_sat_add_one (Vec.len st.gains) (by simpa using hg)
        simp only [Vec.len_val, Vec.length] at hh
        refine ⟨?_, ?_, ?_, ?_⟩
        · rw [hh]
          exact covers_snoc 0 _ _ ⟨x, Vec.len st.gains, .Max⟩ hcov (by simp [h2]) (by simpa using hfirst)
        · rw [hh, hgains]; simp
        · intro c hc
          rw [List.mem_append] at hc
          rcases hc with hc | hc
          · exact hold c hc
          · simp only [List.mem_singleton] at hc; subst hc
            simp only [Vec.len_val, Vec.length]
            rw [h3, hgains, h2, worldSum_snoc_new _ _ _ hfirst, ← hopen]
            simp [worldGain]
        · rw [h5, hh, hgains]
          simp only [List.length_append, List.length_singleton]
          rw [worldSum_self]
      · rw [h1, h2]
        simp only [Vec.len_val, Vec.length] at h0 h1 ⊢
        refine ⟨?_, ?_, ?_, ?_⟩
        · have := covers_snoc 0 _ _ ⟨st.open, core.num.Usize.saturating_sub (Vec.len st.gains) 1#usize, .Break⟩
            hcov rfl (by simp only [usize_sat_sub_one, Vec.len_val, Vec.length]; omega)
          simp only [usize_sat_sub_one, Vec.len_val, Vec.length] at this ⊢
          have he : st.gains.val.length - 1 + 1 = st.gains.val.length := by omega
          rw [he] at this; exact this
        · rw [hgains]; simp
        · intro c hc
          rw [List.mem_append] at hc
          rcases hc with hc | hc
          · exact hold c hc
          · simp only [List.mem_singleton] at hc; subst hc
            simp only [usize_sat_sub_one, Vec.len_val, Vec.length]
            have he : st.gains.val.length - 1 + 1 = st.gains.val.length := by omega
            rw [he, hgains, worldSum_snoc_old _ _ _ _ (le_refl _)]
            exact hopen
        · rw [h3, hgains, worldSum_snoc_new' _ _ _ (le_refl _), worldSum_self]
          simp [worldGain]
    · simp only [applyM, applyPlayM, gainOfStep, ChaptersOk, htr] at hgains hold ⊢
      refine ⟨hcov, by rw [hgains]; simp; omega, ?_, ?_⟩
      · intro c hc; exact hold c hc
      · rw [hgains, worldSum_snoc_new' _ _ _ hfirst, ← hopen]
        simp [worldGain]

theorem chaptersOk_runM (ss : List Step) (h : ss.length ≤ Usize.max) : ChaptersOk (runM ss) := by
  induction ss using List.reverseRecOn with
  | nil => exact chaptersOk_start
  | append_singleton ss s ih =>
    simp only [List.length_append, List.length_singleton] at h
    obtain ⟨hl, hs⟩ := runM_sized ss (by omega)
    rw [runM_snoc]
    exact chaptersOk_apply _ _ (ih (by omega)) (weightsOk_runM ss) hs (by omega)

/-- A step is in a closed chapter when the range of the chapter holds it. -/
def InRange (p : Nat) (c : ClosedChapter) : Prop := c.chapter.first.val ≤ p ∧ p ≤ c.last.val

theorem covers_one (n : Nat) (cs : List ClosedChapter) (f : Nat) (h : Covers n cs f) (p : Nat)
    (hn : n ≤ p) :
    (p < f → ∃ c ∈ cs, InRange p c) ∧ (∀ c ∈ cs, ∀ d ∈ cs, InRange p c → InRange p d → c = d) ∧
      (f ≤ p → ∀ c ∈ cs, ¬ InRange p c) := by
  induction cs generalizing n with
  | nil =>
    simp only [Covers] at h
    exact ⟨fun hp => by omega, by simp, by simp⟩
  | cons d ds ih =>
    simp only [Covers] at h
    obtain ⟨h1, h2, h3⟩ := h
    have hb := covers_bounds _ _ _ h3
    by_cases hp : p ≤ d.last.val
    · have hd : InRange p d := ⟨by omega, hp⟩
      have hno : ∀ c ∈ ds, ¬ InRange p c := by
        intro c hc hr; have := (hb.2 c hc).1; simp only [InRange] at hr; omega
      refine ⟨fun _ => ⟨d, List.mem_cons_self, hd⟩, ?_, ?_⟩
      · intro c hc e he hrc hre
        rcases List.mem_cons.mp hc with hc | hc <;> rcases List.mem_cons.mp he with he | he
        · rw [hc, he]
        · exact absurd hre (hno e he)
        · exact absurd hrc (hno c hc)
        · exact absurd hrc (hno c hc)
      · intro hf; omega
    · obtain ⟨ih1, ih2, ih3⟩ := ih _ h3 (by omega)
      have hnd : ¬ InRange p d := fun hr => hp hr.2
      refine ⟨fun hpf => ?_, ?_, ?_⟩
      · obtain ⟨c, hc, hr⟩ := ih1 hpf
        exact ⟨c, List.mem_cons_of_mem _ hc, hr⟩
      · intro c hc e he hrc hre
        rcases List.mem_cons.mp hc with hc | hc
        · subst hc; exact absurd hrc hnd
        · rcases List.mem_cons.mp he with he | he
          · subst he; exact absurd hre hnd
          · exact ih2 c hc e he hrc hre
      · intro hf c hc
        rcases List.mem_cons.mp hc with hc | hc
        · subst hc; exact hnd
        · exact ih3 hf c hc

/-- Theorem 1. The chapter ranges cover the steps in order, with no gap, no
overlap, and none empty: the closed chapters chain from step 0 to the first
step of the open chapter, and the open chapter holds the steps after them. So
each step is in one closed chapter, or in the open chapter and in no closed
one. -/
theorem every_step_is_in_one_chapter (ss : List Step) (h : ss.length ≤ Usize.max) :
    Covers 0 (runM ss).closed.val (runM ss).open.first.val ∧
      (runM ss).open.first.val ≤ ss.length ∧
      ∀ p < ss.length,
        ((∃ c ∈ (runM ss).closed.val, InRange p c) ∧
          ∀ c ∈ (runM ss).closed.val, ∀ d ∈ (runM ss).closed.val,
            InRange p c → InRange p d → c = d) ∨
        ((runM ss).open.first.val ≤ p ∧ ∀ c ∈ (runM ss).closed.val, ¬ InRange p c) := by
  obtain ⟨hcov, hfirst, _, _⟩ := chaptersOk_runM ss h
  rw [(runM_sized ss h).1] at hfirst
  refine ⟨hcov, hfirst, fun p _ => ?_⟩
  obtain ⟨h1, h2, h3⟩ := covers_one _ _ _ hcov p (Nat.zero_le _)
  by_cases hp : p < (runM ss).open.first.val
  · exact Or.inl ⟨h1 hp, h2⟩
  · exact Or.inr ⟨by omega, h3 (by omega)⟩

/-- Theorem 9. The weight of a chapter is the sum of the gains of its steps in
the open world. A step in an instance adds 0 to it. -/
theorem the_weight_of_a_chapter_is_the_sum_of_its_gains (ss : List Step)
    (h : ss.length ≤ Usize.max) :
    (∀ c ∈ (runM ss).closed.val, c.chapter.weight.val =
      worldSum (runM ss).gains.val c.chapter.first.val (c.last.val + 1)) ∧
    (runM ss).open.weight.val = worldSum (runM ss).gains.val (runM ss).open.first.val ss.length := by
  obtain ⟨_, _, hc, ho⟩ := chaptersOk_runM ss h
  rw [(runM_sized ss h).1] at ho
  exact ⟨hc, ho⟩

/-! ## Theorem 19: the fold never panics, never overflows, and always ends -/

/-- Theorem 19 for `advance`. From a fold whose vectors are no longer than its
steps, with room for the new steps, `advance` gives the model. So it never
panics, never overflows, and always ends. -/
@[step]
theorem advance.spec (st : Fold) (ss : Slice Step) (hs : Sized st)
    (hr : st.gains.val.length + ss.length ≤ Usize.max) :
    advance st ss ⦃ r => r = ss.val.foldl applyM st ⦄ := by
  rw [advance_eq st ss hs hr]
  simp [WP.spec_ok]

/-- Theorem 19 for `chapters`. The fold of any log gives the model. -/
@[step]
theorem chapters.spec (ss : Slice Step) : chapters.chapters ss ⦃ r => r = runM ss.val ⦄ := by
  rw [chapters_eq]
  simp [WP.spec_ok]

/-! ## Theorem 13: entries grow only with what is new -/

theorem worldSum_split (gs : List Gain) (a b c : Nat) (hab : a ≤ b) (hbc : b ≤ c) :
    worldSum gs a c = worldSum gs a b + worldSum gs b c := by
  unfold worldSum
  have hc : c - a = (b - a) + (c - b) := by
    rw [Nat.add_comm, Nat.sub_add_sub_cancel hbc hab]
  rw [hc, List.take_add, List.map_append, List.sum_append, List.drop_drop]
  have hb : a + (b - a) = b := by omega
  rw [hb]

theorem covers_sum (gs : List Gain) (n : Nat) (cs : List ClosedChapter) (f : Nat)
    (h : Covers n cs f)
    (hw : ∀ c ∈ cs, c.chapter.weight.val = worldSum gs c.chapter.first.val (c.last.val + 1)) :
    (cs.map (fun c => c.chapter.weight.val)).sum = worldSum gs n f := by
  induction cs generalizing n with
  | nil => simp only [Covers] at h; subst h; simp [worldSum_self]
  | cons d ds ih =>
    simp only [Covers] at h
    obtain ⟨h1, h2, h3⟩ := h
    have hb := covers_bounds _ _ _ h3
    simp only [List.map_cons, List.sum_cons]
    rw [ih _ h3 (fun c hc => hw c (List.mem_cons_of_mem _ hc)), hw d List.mem_cons_self, h1,
      ← worldSum_split _ _ _ _ (by omega) hb.1]

theorem count_mul_le_sum (cs : List ClosedChapter) (m : Nat)
    (h : ∀ c ∈ cs, c.close = .Rule ∨ m ≤ c.chapter.weight.val) :
    (cs.filter (fun c => decide (c.close ≠ .Rule))).length * m ≤
      (cs.map (fun c => c.chapter.weight.val)).sum := by
  induction cs with
  | nil => simp
  | cons d ds ih =>
    have ih := ih (fun c hc => h c (List.mem_cons_of_mem _ hc))
    simp only [List.filter_cons, List.map_cons, List.sum_cons]
    split
    · rename_i hd
      simp only [List.length_cons, decide_eq_true_eq] at hd ⊢
      have := (h d List.mem_cons_self).resolve_left hd
      rw [Nat.add_mul]; omega
    · omega

/-- The steps of the log that are rule steps. -/
def isRule : Step → Bool
  | .Rule _ => true
  | .Play _ => false

theorem rule_closed_snoc (st : Fold) (s : Step) :
    ((applyM st s).closed.val.filter (fun c => decide (c.close = .Rule))).length ≤
      (st.closed.val.filter (fun c => decide (c.close = .Rule))).length + (if isRule s then 1 else 0) := by
  cases s with
  | Rule r =>
    obtain ⟨l, hl, hl1, _⟩ := applyRuleM_new_closed st r
    rw [hl, List.filter_append, List.length_append]
    simp only [isRule, if_true]
    have := List.length_filter_le (fun c => decide (c.close = .Rule)) l
    omega
  | Play p =>
    obtain ⟨l, hl, hx, _⟩ := applyPlayM_new_closed st p
    rw [hl, List.filter_append, List.length_append]
    have : l.filter (fun c => decide (c.close = .Rule)) = [] := by
      rw [List.filter_eq_nil_iff]; intro c hc; simp [(hx c hc).1]
    simp [this, isRule]

/-- The gain of the steps in the open world. -/
def worldGainOf (ss : List Step) : Nat := worldSum (runM ss).gains.val 0 ss.length

/-- Theorem 13, the chapters. Every closed chapter that no rule step closed
weighs `MIN` or more, so their number times `MIN` is at most the gain in the
open world. A rule step closes one chapter at most, so the chapters that rule
steps closed are no more than the rule steps. -/
theorem entries_grow_only_with_what_is_new_chapters (ss : List Step)
    (h : ss.length ≤ Usize.max) :
    ((runM ss).closed.val.filter (fun c => decide (c.close ≠ .Rule))).length * MIN ≤
        worldGainOf ss ∧
      ((runM ss).closed.val.filter (fun c => decide (c.close = .Rule))).length ≤
        (ss.filter isRule).length := by
  refine ⟨?_, ?_⟩
  · obtain ⟨hcov, hfirst, hc, _⟩ := chaptersOk_runM ss h
    have h1 := count_mul_le_sum _ MIN (a_closed_chapter_has_min_weight ss)
    rw [covers_sum _ _ _ _ hcov hc] at h1
    unfold worldGainOf
    rw [(runM_sized ss h).1] at hfirst
    rw [worldSum_split _ 0 (runM ss).open.first.val ss.length (Nat.zero_le _) hfirst]
    omega
  · clear h
    induction ss using List.reverseRecOn with
    | nil => simp [runM, startM]
    | append_singleton ss s ih =>
      rw [runM_snoc, List.filter_append, List.length_append]
      have := rule_closed_snoc (runM ss) s
      cases s <;> simp_all [isRule] <;> omega

/-- The instance ids of the steps of a log. -/
def instanceIds : List Step → List Usize
  | [] => []
  | .Play p :: rest => match p.track with
    | .Instance i => i :: instanceIds rest
    | .World => instanceIds rest
  | .Rule _ :: rest => instanceIds rest

theorem instanceIds_append (a b : List Step) :
    instanceIds (a ++ b) = instanceIds a ++ instanceIds b := by
  induction a with
  | nil => rfl
  | cons s rest ih =>
    cases s with
    | Rule r => simp [instanceIds, ih]
    | Play p => rcases ht : p.track with _ | i <;> simp [instanceIds, ih, ht]

theorem taleOfM_insts_by (t : Vec Tale) (inst here : Usize) :
    insts (taleOfM t inst here).2 = insts t ∨ insts (taleOfM t inst here).2 = insts t ++ [inst] := by
  unfold taleOfM taleFromM
  split
  · exact Or.inl rfl
  · unfold pushM
    split
    · exact Or.inr (by simp [insts])
    · exact Or.inl rfl

theorem enterM_insts_by (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) :
    insts (enterM t vs o inst here now).1 = insts t ∨
      insts (enterM t vs o inst here now).1 = insts t ++ [inst] := by
  have hg := taleOfM_insts_by t inst here
  unfold enterM
  rcases o with _ | v
  · exact hg
  · simp only; split
    · exact hg
    · rw [closeVisitM_insts]; exact hg

theorem applyM_insts_sub (st : Fold) (s : Step) :
    ∀ x ∈ insts (applyM st s).tales, x ∈ insts st.tales ∨ x ∈ instanceIds [s] := by
  intro x hx
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM] at hx; split at hx <;> exact Or.inl hx
  | Play p =>
    simp only [applyM, applyPlayM, visitsOfM] at hx
    rcases ht : p.track with _ | i
    · simp only [ht, leaveM_insts] at hx; exact Or.inl hx
    · simp only [ht] at hx
      rcases enterM_insts_by st.tales st.visits st.visit i (Vec.len st.gains) p.at with h | h <;>
        rw [h] at hx
      · exact Or.inl hx
      · rw [List.mem_append, List.mem_singleton] at hx
        rcases hx with hx | hx
        · exact Or.inl hx
        · right; simp [instanceIds, ht, hx]

theorem runM_insts_sub (ss : List Step) : ∀ x ∈ insts (runM ss).tales, x ∈ instanceIds ss := by
  induction ss using List.reverseRecOn with
  | nil => simp [runM, startM, insts]
  | append_singleton ss s ih =>
    intro x hx
    rw [runM_snoc] at hx
    rw [instanceIds_append, List.mem_append]
    rcases applyM_insts_sub _ _ x hx with h | h
    · exact Or.inl (ih x h)
    · exact Or.inr h

/-- Theorem 13, the tales. A log has no more tales than distinct instances. -/
theorem entries_grow_only_with_what_is_new_tales (ss : List Step) :
    (runM ss).tales.val.length ≤ (instanceIds ss).toFinset.card := by
  have hn := an_instance_has_at_most_one_tale ss
  have hl : (runM ss).tales.val.length = (insts (runM ss).tales).toFinset.card := by
    rw [List.toFinset_card_of_nodup hn]; simp [insts]
  rw [hl]
  apply Finset.card_le_card
  intro x hx
  simp only [List.mem_toFinset] at hx ⊢
  exact runM_insts_sub ss x hx

theorem map_sum_set {α : Type} (l : List α) (f : α → Nat) (i : Nat) (x : α) (hi : i < l.length) :
    ((l.set i x).map f).sum + f l[i] = (l.map f).sum + f x := by
  induction l generalizing i with
  | nil => simp at hi
  | cons y ys ih =>
    cases i with
    | zero => simp; omega
    | succ i =>
      simp only [List.set_cons_succ, List.map_cons, List.sum_cons, List.getElem_cons_succ]
      have := ih i (by simpa using hi)
      omega

theorem rawGainM_gain (foes : Vec FoeRecord) (r : KeyRecord) (k : Key) :
    (rawGainM foes r k).2.2.2.gain = r.gain := by
  unfold rawGainM
  rcases hk : k.kind
  all_goals simp only
  all_goals first
    | rfl
    | (unfold deathGainM; rcases k.foe with _ | f <;> simp only <;> (try split) <;>
        (try split) <;> rfl)

/-- The gain of a log so far, and the gain of its keys. -/
def gainTotal (gs : List Gain) : Nat := (gs.map (fun g => g.amount.val)).sum
def keyTotal (ks : List KeyRecord) : Nat := (ks.map (fun r => r.gain.val)).sum

theorem gainM_keys_total (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Option Key)
    (hk : ∀ r ∈ keys.val, r.gain.val ≤ 7) :
    keyTotal (gainM keys foes key).2.1.val = keyTotal keys.val + (gainM keys foes key).1.1.val ∧
      ∀ r ∈ (gainM keys foes key).2.1.val, r.gain.val ≤ 7 := by
  unfold gainM
  rcases key with _ | k
  · simp only; exact ⟨by simp, hk⟩
  · simp only
    split
    · rename_i hs
      have hlt := slotted_lt keys k.id UNSEEN hs
      have hgr := rawGainM_gain foes (getM (slotted keys k.id UNSEEN) k.id UNSEEN) k
      have hsl : keyTotal (slotted keys k.id UNSEEN).val = keyTotal keys.val ∧
          ∀ r ∈ (slotted keys k.id UNSEEN).val, r.gain.val ≤ 7 := by
        unfold slotted; split
        · unfold pushM; split
          · simp only [keyTotal, pushM_val _ _ (by assumption), List.map_append, List.sum_append]
            refine ⟨by simp [UNSEEN], ?_⟩
            intro r hr
            simp only [Vec.from_val] at hr
            rw [List.mem_append, List.mem_singleton] at hr
            rcases hr with hr | hr
            · exact hk r hr
            · subst hr; simp [UNSEEN]
          · exact ⟨rfl, hk⟩
        · exact ⟨rfl, hk⟩
      generalize slotted keys k.id UNSEEN = V at *
      generalize rawGainM foes (getM V k.id UNSEEN) k = R at *
      obtain ⟨raw, rv, f1, r1⟩ := R
      simp only at hgr ⊢
      rw [hgr]
      have hget : getM V k.id UNSEEN = V.val[k.id.val] := by
        simp [getM, List.getD, hlt]
      have hle7 : (getM V k.id UNSEEN).gain.val ≤ 7 := by
        rw [hget]; exact hsl.2 _ (List.getElem_mem _)
      generalize hG : getM V k.id UNSEEN = G at *
      have hroom := sat_sub_val CAP_MAX G.gain
      rw [← u16_sat_sub] at hroom
      have hcap : CAP_MAX.val = 7 := by unfold CAP_MAX; rfl
      have hamt : (if raw < core.num.U16.saturating_sub CAP_MAX G.gain then raw
          else core.num.U16.saturating_sub CAP_MAX G.gain).val ≤ 7 - G.gain.val := by
        split <;> scalar_tac
      generalize (if raw < core.num.U16.saturating_sub CAP_MAX G.gain then raw
          else core.num.U16.saturating_sub CAP_MAX G.gain) = a at *
      have hnew : (core.num.U16.saturating_add G.gain a).val = G.gain.val + a.val := by
        rw [u16_sat_add, sat_add_val]; simp only [UScalar.max]; scalar_tac
      refine ⟨?_, ?_⟩
      · have := map_sum_set V.val (fun r => r.gain.val) k.id.val
          { r1 with seen := true, gain := core.num.U16.saturating_add G.gain a } hlt
        simp only [Vec.set_val_eq]
        unfold keyTotal at *
        rw [← hsl.1]
        rw [← hget] at this
        simp only [hnew] at this
        omega
      · intro r hr
        simp only [Vec.set_val_eq] at hr
        rcases List.mem_or_eq_of_mem_set hr with hr | hr
        · exact hsl.2 r hr
        · subst hr; simp only [hnew]; omega
    · exact ⟨by simp, hk⟩

/-- The invariant of the key gains. -/
def KeysOk (st : Fold) : Prop :=
  gainTotal st.gains.val = keyTotal st.keys.val ∧ ∀ r ∈ st.keys.val, r.gain.val ≤ 7

theorem keysOk_apply (st : Fold) (s : Step) (h : KeysOk st) (hg : st.gains.val.length < Usize.max) :
    KeysOk (applyM st s) := by
  obtain ⟨h1, h2⟩ := h
  have hgains := applyM_gains_eq st s hg
  cases s with
  | Rule r =>
    by_cases hlt : st.open.first.val < (Vec.len st.gains).val
    · simp only [applyM, applyRuleM, hlt, if_true, KeysOk] at hgains ⊢
      refine ⟨?_, h2⟩
      rw [gainTotal, hgains]; simp [gainOfStep, ZERO_GAIN]; exact h1
    · simp only [applyM, applyRuleM, hlt, if_false, KeysOk] at hgains ⊢
      refine ⟨?_, h2⟩
      rw [gainTotal, hgains]; simp [gainOfStep, ZERO_GAIN]; exact h1
  | Play p =>
    obtain ⟨k1, k2⟩ := gainM_keys_total st.keys st.foes p.key h2
    rcases ht : p.track with _ | i
    all_goals
      simp only [applyM, applyPlayM, ht, KeysOk] at hgains ⊢
      refine ⟨?_, k2⟩
      rw [gainTotal, hgains, k1]
      simp only [List.map_append, List.sum_append, List.map_cons, List.map_nil, List.sum_cons,
        List.sum_nil, gainOfStep]
      unfold gainTotal at h1
      omega

theorem keysOk_runM (ss : List Step) (h : ss.length ≤ Usize.max) : KeysOk (runM ss) := by
  induction ss using List.reverseRecOn with
  | nil => simp [KeysOk, runM, startM, gainTotal, keyTotal]
  | append_singleton ss s ih =>
    simp only [List.length_append, List.length_singleton] at h
    rw [runM_snoc]
    exact keysOk_apply _ _ (ih (by omega)) (by rw [(runM_sized ss (by omega)).1]; omega)

/-- The key ids of the steps of a log. -/
def keyIds : List Step → List Nat
  | [] => []
  | .Play p :: rest => match p.key with
    | some k => k.id.val :: keyIds rest
    | none => keyIds rest
  | .Rule _ :: rest => keyIds rest

theorem keyIds_append (a b : List Step) : keyIds (a ++ b) = keyIds a ++ keyIds b := by
  induction a with
  | nil => rfl
  | cons s rest ih =>
    cases s with
    | Rule r => simp [keyIds, ih]
    | Play p => rcases hk : p.key with _ | k <;> simp [keyIds, ih, hk]

theorem gainM_keys_length (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Option Key) :
    ∀ j < (gainM keys foes key).2.1.val.length, j < keys.val.length ∨
      ∃ k, key = some k ∧ k.id.val = j := by
  intro j hj
  unfold gainM at hj
  rcases key with _ | k
  · exact Or.inl hj
  · simp only at hj
    split at hj
    · simp only [Vec.set_val_eq, List.length_set] at hj
      unfold slotted at hj
      split at hj
      · rename_i he
        have := pushM_length_le keys UNSEEN
        by_cases hjl : j < keys.val.length
        · exact Or.inl hjl
        · exact Or.inr ⟨k, rfl, by omega⟩
      · exact Or.inl hj
    · exact Or.inl hj

theorem runM_keys_sub (ss : List Step) : ∀ j < (runM ss).keys.val.length, j ∈ keyIds ss := by
  induction ss using List.reverseRecOn with
  | nil => simp [runM, startM]
  | append_singleton ss s ih =>
    intro j hj
    rw [runM_snoc] at hj
    rw [keyIds_append, List.mem_append]
    have hk : (applyM (runM ss) s).keys.val.length ≤ (runM ss).keys.val.length ∨
        ∃ p k, s = .Play p ∧ p.key = some k ∧
          ∀ j < (applyM (runM ss) s).keys.val.length, j < (runM ss).keys.val.length ∨ k.id.val = j := by
      cases s with
      | Rule r => left; simp only [applyM, applyRuleM]; split <;> simp
      | Play p =>
        have hkeys : (applyM (runM ss) (.Play p)).keys = (gainM (runM ss).keys (runM ss).foes p.key).2.1 := by
          simp only [applyM, applyPlayM]; rcases p.track <;> rfl
        rcases hpk : p.key with _ | k
        · left; rw [hkeys, hpk]; simp [gainM]
        · right; refine ⟨p, k, rfl, hpk, fun j hj => ?_⟩
          rw [hkeys] at hj
          rcases gainM_keys_length _ _ _ j hj with h | ⟨k', hk', hj'⟩
          · exact Or.inl h
          · rw [hpk] at hk'; cases hk'; exact Or.inr hj'
    rcases hk with hk | ⟨p, k, hs, hpk, hk⟩
    · exact Or.inl (ih j (by omega))
    · rcases hk j hj with h | h
      · exact Or.inl (ih j h)
      · right; subst hs; simp [keyIds, hpk, h]

/-- Theorem 13, the gain. The gain of all steps, in the open world and in
instances, is the sum of the gains of the keys. A key adds `CAP_MAX` at most,
and the fold has no more keys than the distinct key ids of the steps. -/
theorem entries_grow_only_with_what_is_new_gain (ss : List Step) (h : ss.length ≤ Usize.max) :
    gainTotal (runM ss).gains.val ≤ (runM ss).keys.val.length * 7 ∧
      (runM ss).keys.val.length ≤ (keyIds ss).toFinset.card := by
  obtain ⟨h1, h2⟩ := keysOk_runM ss h
  refine ⟨?_, ?_⟩
  · rw [h1]
    unfold keyTotal
    have : ∀ l : List KeyRecord, (∀ r ∈ l, r.gain.val ≤ 7) →
        (l.map (fun r => r.gain.val)).sum ≤ l.length * 7 := by
      intro l hl
      induction l with
      | nil => simp
      | cons x xs ih =>
        simp only [List.map_cons, List.sum_cons, List.length_cons]
        have := hl x List.mem_cons_self
        have := ih (fun r hr => hl r (List.mem_cons_of_mem _ hr))
        rw [Nat.add_mul]; omega
    exact this _ h2
  · have : Finset.range (runM ss).keys.val.length ⊆ (keyIds ss).toFinset := by
      intro j hj
      simp only [Finset.mem_range] at hj
      simp only [List.mem_toFinset]
      exact runM_keys_sub ss j hj
    simpa using Finset.card_le_card this

/-! ## Theorem 13, the visits -/

def visitSum (vs : List Visit) : Nat := (vs.map (fun v => v.gain.val)).sum

/-- The closed visits and the open visit. -/
def allVisits (st : Fold) : List Visit := st.visits.val ++ st.visit.toList

/-- The gain of a step in an instance, else 0. -/
def instanceGain (g : Gain) : Nat :=
  match g.track with
  | .World => 0
  | .Instance _ => g.amount.val

def instanceSum (gs : List Gain) : Nat := (gs.map instanceGain).sum

theorem visitSum_append (a b : List Visit) : visitSum (a ++ b) = visitSum a + visitSum b := by
  simp [visitSum]

theorem closeVisitM_sum (t : Vec Tale) (vs : Vec Visit) (v : Visit) :
    visitSum (closeVisitM t vs v).2.val ≤ visitSum vs.val + v.gain.val := by
  obtain ⟨l, hl, hx⟩ := pushM_one vs v
  have h2 : (closeVisitM t vs v).2 = pushM vs v := by unfold closeVisitM; split <;> rfl
  rw [h2, hl, visitSum_append]
  have : l = [] ∨ l = [v] := by
    have := grew_push vs v
    obtain ⟨l2, hl2, hl21⟩ := this
    rw [hl] at hl2
    have hle := List.append_cancel_left hl2
    subst hle
    rcases l with _ | ⟨a, _ | ⟨b, rest⟩⟩
    · exact Or.inl rfl
    · rw [hx a (by simp)]; exact Or.inr rfl
    · simp at hl21
  rcases this with h | h <;> subst h <;> simp [visitSum]

theorem leaveM_sum (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (now : U64) :
    visitSum ((leaveM t vs o now).2.1.val ++ (leaveM t vs o now).2.2.toList) ≤
      visitSum (vs.val ++ o.toList) := by
  unfold leaveM
  rcases o with _ | v
  · simp
  · simp only; split
    · have := closeVisitM_sum t vs v
      simp [visitSum_append] at *; simp [visitSum] at this ⊢; omega
    · simp

theorem enterM_sum (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) :
    visitSum ((enterM t vs o inst here now).2.1.val ++ (enterM t vs o inst here now).2.2.toList) ≤
      visitSum (vs.val ++ o.toList) := by
  unfold enterM
  rcases o with _ | v
  · simp [visitSum_append, visitSum]
  · simp only; split
    · simp [visitSum_append, visitSum]
    · have := closeVisitM_sum (taleOfM t inst here).2 vs v
      simp [visitSum_append] at *; simp [visitSum] at this ⊢; omega

theorem addVisitM_sum (o : Option Visit) (a : U16) :
    visitSum (addVisitM o a).toList ≤ visitSum o.toList + a.val := by
  unfold addVisitM
  rcases o with _ | v
  · simp [visitSum]
  · simp only [visitSum, Option.toList_some, List.map_cons, List.map_nil, List.sum_cons,
      List.sum_nil, Nat.add_zero]
    rw [u32_sat_add, sat_add_val]
    have : (UScalar.cast UScalarTy.U32 a).val = a.val := by
      simp only [UScalar.cast_val_eq]; scalar_tac
    rw [this]; omega

theorem visits_apply (st : Fold) (s : Step) (hg : st.gains.val.length < Usize.max)
    (h : visitSum (allVisits st) ≤ instanceSum st.gains.val) :
    visitSum (allVisits (applyM st s)) ≤ instanceSum (applyM st s).gains.val := by
  rw [applyM_gains_eq st s hg]
  unfold instanceSum at *
  simp only [List.map_append, List.sum_append, List.map_cons, List.map_nil, List.sum_cons,
    List.sum_nil, Nat.add_zero]
  unfold allVisits at *
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]; split <;> simp [gainOfStep, ZERO_GAIN, instanceGain] <;> omega
  | Play p =>
    simp only [applyM, applyPlayM, visitsOfM, gainOfStep]
    rcases ht : p.track with _ | i
    · simp only [instanceGain, Nat.add_zero]
      exact le_trans (leaveM_sum _ _ _ _) h
    · simp only [instanceGain]
      have h1 := enterM_sum st.tales st.visits st.visit i (Vec.len st.gains) p.at
      generalize enterM st.tales st.visits st.visit i (Vec.len st.gains) p.at = E at h1 ⊢
      have h2 := addVisitM_sum E.2.2 (gainM st.keys st.foes p.key).1.1
      rw [visitSum_append] at h1 ⊢
      exact le_trans (Nat.add_le_add_left h2 _)
        (by rw [← Nat.add_assoc]; exact Nat.add_le_add_right (le_trans h1 h) _)

theorem visits_runM (ss : List Step) (h : ss.length ≤ Usize.max) :
    visitSum (allVisits (runM ss)) ≤ instanceSum (runM ss).gains.val := by
  induction ss using List.reverseRecOn with
  | nil => simp [allVisits, runM, startM, visitSum, instanceSum]
  | append_singleton ss s ih =>
    simp only [List.length_append, List.length_singleton] at h
    rw [runM_snoc]
    exact visits_apply _ _ (by rw [(runM_sized ss (by omega)).1]; omega) (ih (by omega))

/-- Theorem 13, the visits. The closed visits with gain are no more than the
gain in instances. A tale writes its text only after a closed visit with
gain, so tale texts are no more than that either. -/
theorem entries_grow_only_with_what_is_new_visits (ss : List Step) (h : ss.length ≤ Usize.max) :
    ((runM ss).visits.val.filter (fun v => decide (0 < v.gain.val))).length ≤
      instanceSum (runM ss).gains.val := by
  have h1 := visits_runM ss h
  unfold allVisits at h1
  rw [visitSum_append] at h1
  have : ∀ l : List Visit, (l.filter (fun v => decide (0 < v.gain.val))).length ≤ visitSum l := by
    intro l
    induction l with
    | nil => simp [visitSum]
    | cons x xs ih =>
      simp only [List.filter_cons, visitSum, List.map_cons, List.sum_cons] at ih ⊢
      split <;> simp_all <;> omega
  have := this (runM ss).visits.val
  omega

/-- Theorem 13. Entries grow only with what is new: the bounds of section 7. -/
theorem entries_grow_only_with_what_is_new (ss : List Step) (h : ss.length ≤ Usize.max) :
    ((runM ss).closed.val.filter (fun c => decide (c.close ≠ .Rule))).length * MIN ≤
        worldGainOf ss ∧
    ((runM ss).closed.val.filter (fun c => decide (c.close = .Rule))).length ≤
        (ss.filter isRule).length ∧
    gainTotal (runM ss).gains.val ≤ (runM ss).keys.val.length * 7 ∧
    (runM ss).keys.val.length ≤ (keyIds ss).toFinset.card ∧
    (runM ss).tales.val.length ≤ (instanceIds ss).toFinset.card ∧
    ((runM ss).visits.val.filter (fun v => decide (0 < v.gain.val))).length ≤
        instanceSum (runM ss).gains.val :=
  ⟨(entries_grow_only_with_what_is_new_chapters ss h).1,
   (entries_grow_only_with_what_is_new_chapters ss h).2,
   (entries_grow_only_with_what_is_new_gain ss h).1,
   (entries_grow_only_with_what_is_new_gain ss h).2,
   entries_grow_only_with_what_is_new_tales ss,
   entries_grow_only_with_what_is_new_visits ss h⟩

/-! ## Theorem 12: repeats alone never make an entry -/

/-- A step that brings nothing new to fold `st`: a step of play with no key,
or with a key that is spent. -/
def Repeat (st : Fold) (s : Step) : Prop :=
  ∃ p, s = .Play p ∧ (p.key = none ∨ ∃ k, p.key = some k ∧ Spent st k)

theorem repeat_gain (st : Fold) (p : Play)
    (h : p.key = none ∨ ∃ k, p.key = some k ∧ Spent st k) :
    (gainM st.keys st.foes p.key).1.1.val = 0 := by
  rcases h with h | ⟨k, hk, hs⟩
  · rw [h]; simp [gainM]
  · rw [hk]; exact (a_repeat_never_adds_weight st k hs (.Rule 0#u8)).1

theorem repeat_mono (st : Fold) (s t : Step) (h : Repeat st t) : Repeat (applyM st s) t := by
  obtain ⟨p, hp, h⟩ := h
  refine ⟨p, hp, ?_⟩
  rcases h with h | ⟨k, hk, hs⟩
  · exact Or.inl h
  · exact Or.inr ⟨k, hk, (a_repeat_never_adds_weight st k hs s).2⟩

theorem insts_prefix (a b : Vec Tale) (h : InstsGrow a b) : insts a <+: insts b := by
  rcases h with h | ⟨_, _, h⟩
  · rw [h]
  · rw [h]; exact List.prefix_append _ _

theorem repeat_step (st : Fold) (s : Step) (h : Repeat st s) :
    (applyM st s).closed = st.closed ∧
    (∃ l, (applyM st s).gains.val = st.gains.val ++ l ∧ ∀ g ∈ l, g.amount.val = 0) ∧
    visitSum (allVisits (applyM st s)) ≤ visitSum (allVisits st) := by
  obtain ⟨p, rfl, hk⟩ := h
  have h0 := repeat_gain st p hk
  refine ⟨a_step_with_no_gain_closes_nothing st p (by simpa [gainOfStep] using h0), ?_, ?_⟩
  · simp only [applyM, applyPlayM]
    rcases p.track with _ | i
    all_goals
      obtain ⟨l, hl, hx⟩ := pushM_one st.gains
        { amount := (gainM st.keys st.foes p.key).1.1, track := _,
          revenge := (gainM st.keys st.foes p.key).1.2 }
      exact ⟨l, hl, fun g hg => by rw [hx g hg]; exact h0⟩
  · unfold allVisits
    simp only [applyM, applyPlayM, visitsOfM]
    rcases p.track with _ | i
    · exact leaveM_sum _ _ _ _
    · simp only
      have h1 := enterM_sum st.tales st.visits st.visit i (Vec.len st.gains) p.at
      generalize enterM st.tales st.visits st.visit i (Vec.len st.gains) p.at = E at h1 ⊢
      have h2 := addVisitM_sum E.2.2 (gainM st.keys st.foes p.key).1.1
      rw [h0] at h2
      rw [visitSum_append] at h1 ⊢
      exact le_trans (Nat.add_le_add_left h2 _) h1

/-- Theorem 12. From any fold, steps with spent keys or no key close no
chapter, add no gain (so no gain to a visit), and open a tale only for an
instance that had none: the instances of the tales only grow, with no
instance twice. -/
theorem repeats_alone_never_make_an_entry (st : Fold) (ss : List Step)
    (h : ∀ s ∈ ss, Repeat st s) :
    (ss.foldl applyM st).closed = st.closed ∧
    (∃ l, (ss.foldl applyM st).gains.val = st.gains.val ++ l ∧ ∀ g ∈ l, g.amount.val = 0) ∧
    insts st.tales <+: insts (ss.foldl applyM st).tales ∧
    ((insts st.tales).Nodup → (insts (ss.foldl applyM st).tales).Nodup) ∧
    visitSum (allVisits (ss.foldl applyM st)) ≤ visitSum (allVisits st) := by
  induction ss generalizing st with
  | nil => exact ⟨rfl, ⟨[], by simp, by simp⟩, List.prefix_refl _, id, le_refl _⟩
  | cons s rest ih =>
    simp only [List.foldl_cons]
    have hs := h s List.mem_cons_self
    obtain ⟨c1, ⟨l1, hl1, hx1⟩, v1⟩ := repeat_step st s hs
    obtain ⟨c2, ⟨l2, hl2, hx2⟩, p2, n2, v2⟩ :=
      ih (applyM st s) (fun t ht => repeat_mono st s t (h t (List.mem_cons_of_mem _ ht)))
    refine ⟨c2.trans c1, ⟨l1 ++ l2, by rw [hl2, hl1, List.append_assoc], ?_⟩,
      (insts_prefix _ _ (applyM_insts st s)).trans p2,
      fun hn => n2 (instsGrow_nodup _ _ (applyM_insts st s) hn), le_trans v2 v1⟩
    intro g hg
    rcases List.mem_append.mp hg with hg | hg
    · exact hx1 g hg
    · exact hx2 g hg

/-! ## Theorem 2: each instance step is in one visit -/

/-- The tale, the first step, and the last step of a visit. -/
def rng (v : Visit) : Nat × Nat × Nat := (v.tale.val, v.first.val, v.last.val)

def ranges (vs : List Visit) (o : Option Visit) : List (Nat × Nat × Nat) := (vs ++ o.toList).map rng

/-- The instance of a step in an instance. -/
def stepInstance : Step → Option Usize
  | .Play p => match p.track with
    | .Instance i => some i
    | .World => none
  | .Rule _ => none

/-- Tale `j` is the tale of instance `i`. -/
def TaleAt (tales : Vec Tale) (j : Nat) (i : Usize) : Prop :=
  ∃ t, tales.val[j]? = some t ∧ t.instance = i

/-- The tales after a step keep the instance at each index. -/
def KeepsTales (a b : Vec Tale) : Prop := ∀ j i, TaleAt a j i → TaleAt b j i

theorem keepsTales_refl (a : Vec Tale) : KeepsTales a a := fun _ _ h => h

theorem keepsTales_of_get (a b : Vec Tale) (vs vs' : Vec Visit)
    (h : ∀ j x, a.val[j]? = some x → ∃ x', b.val[j]? = some x' ∧ TaleMoved j x x' vs vs') :
    KeepsTales a b := by
  intro j i ⟨t, ht, hi⟩
  obtain ⟨t', ht', hm⟩ := h j t ht
  exact ⟨t', ht', hm.1.trans hi⟩

theorem closeVisitM_ranges (t : Vec Tale) (vs : Vec Visit) (v : Visit)
    (hroom : vs.val.length < Usize.max) :
    (closeVisitM t vs v).2.val = vs.val ++ [v] := by
  have : (closeVisitM t vs v).2 = pushM vs v := by unfold closeVisitM; split <;> rfl
  rw [this, pushM_val _ _ hroom]

theorem leaveM_ranges (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (now : U64)
    (hroom : vs.val.length < Usize.max) :
    ranges (leaveM t vs o now).2.1.val (leaveM t vs o now).2.2 = ranges vs.val o := by
  unfold leaveM ranges
  rcases o with _ | v
  · rfl
  · simp only; split
    · rw [closeVisitM_ranges _ _ _ hroom]; simp
    · rfl

theorem taleOfM_at (t : Vec Tale) (inst here : Usize) (hroom : t.val.length < Usize.max) :
    TaleAt (taleOfM t inst here).2 (taleOfM t inst here).1.val inst := by
  unfold taleOfM taleFromM
  split
  · rename_i j hj
    obtain ⟨hlt, he⟩ := findFrom_some _ _ _ _ hj
    simp only
    rw [usizeOf_val _ (by have := t.property; omega)]
    exact ⟨_, by simp [hlt], he⟩
  · simp only
    rw [usizeOf_val _ (by omega)]
    refine ⟨{ «instance» := inst, first := here, weight := 0#u32, runs := 0#u32 }, ?_, rfl⟩
    rw [pushM_val _ _ hroom]; simp

theorem enterM_facts (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) (ht : t.val.length < Usize.max) (hroom : vs.val.length < Usize.max) :
    let T := (taleOfM t inst here).1.val
    TaleAt (enterM t vs o inst here now).1 T inst ∧
    KeepsTales t (enterM t vs o inst here now).1 ∧
    (ranges (enterM t vs o inst here now).2.1.val (enterM t vs o inst here now).2.2 =
        ranges vs.val o ++ [(T, here.val, here.val)] ∨
      ∃ pre f l, ranges vs.val o = pre ++ [(T, f, l)] ∧
        ranges (enterM t vs o inst here now).2.1.val (enterM t vs o inst here now).2.2 =
          pre ++ [(T, f, here.val)]) := by
  have hat := taleOfM_at t inst here ht
  have hkeep : KeepsTales t (taleOfM t inst here).2 := by
    intro j i ⟨x, hx, hi⟩; exact ⟨x, taleOfM_get _ _ _ _ _ hx, hi⟩
  simp only
  refine ⟨?_, ?_, ?_⟩
  · unfold enterM
    rcases o with _ | v
    · exact hat
    · simp only; split
      · exact hat
      · obtain ⟨x, hx, hi⟩ := hat
        obtain ⟨x', hx', hm⟩ := closeVisitM_get _ vs v _ x hx hroom
        exact ⟨x', hx', hm.1.trans hi⟩
  · unfold enterM
    rcases o with _ | v
    · exact hkeep
    · simp only; split
      · exact hkeep
      · intro j i hj
        obtain ⟨x, hx, hi⟩ := hkeep j i hj
        obtain ⟨x', hx', hm⟩ := closeVisitM_get _ vs v _ x hx hroom
        exact ⟨x', hx', hm.1.trans hi⟩
  · unfold enterM ranges
    rcases o with _ | v
    · left; simp [rng]
    · simp only; split
      · rename_i hc
        right
        refine ⟨vs.val.map rng, v.first.val, v.last.val, ?_, ?_⟩
        · simp [rng, hc.1]
        · simp [rng, hc.1]
      · left
        rw [closeVisitM_ranges _ _ _ hroom]
        simp [rng]

theorem addVisitM_ranges (vs : List Visit) (o : Option Visit) (a : U16) :
    ranges vs (addVisitM o a) = ranges vs o := by
  unfold ranges addVisitM
  rcases o with _ | v <;> simp [rng]

/-- The invariant of the visits after the steps `ss`. -/
def VisitsInv (ss : List Step) (st : Fold) : Prop :=
  (ranges st.visits.val st.visit).Pairwise (fun a b => a.2.2 < b.2.1) ∧
  (∀ r ∈ ranges st.visits.val st.visit, r.2.1 ≤ r.2.2 ∧ r.2.2 < ss.length) ∧
  (∀ p (hp : p < ss.length) i, stepInstance ss[p] = some i →
    ∃ r ∈ ranges st.visits.val st.visit, r.2.1 ≤ p ∧ p ≤ r.2.2 ∧ TaleAt st.tales r.1 i)

theorem visitsInv_append (ss : List Step) (s : Step) (st st' : Fold) (n : Nat) (T : Nat)
    (hn : ss.length = n) (h : VisitsInv ss st)
    (hkeep : KeepsTales st.tales st'.tales)
    (hsi : ∀ i, stepInstance s = some i → TaleAt st'.tales T i)
    (hr : (ranges st'.visits.val st'.visit = ranges st.visits.val st.visit ∧ stepInstance s = none) ∨
      ranges st'.visits.val st'.visit = ranges st.visits.val st.visit ++ [(T, n, n)] ∨
      ∃ pre f l, ranges st.visits.val st.visit = pre ++ [(T, f, l)] ∧
        ranges st'.visits.val st'.visit = pre ++ [(T, f, n)]) :
    VisitsInv (ss ++ [s]) st' := by
  obtain ⟨hpw, hbd, hcov⟩ := h
  have hlen : (ss ++ [s]).length = n + 1 := by simp [hn]
  have hold : ∀ p (hp : p < ss.length), (ss ++ [s])[p]'(by simp; omega) = ss[p] := by
    intro p hp; simp [List.getElem_append_left hp]
  have hlast : ∀ p (hp : p = ss.length), (ss ++ [s])[p]'(by simp; omega) = s := by
    intro p hp; subst hp; simp
  rcases hr with ⟨hr, hnone⟩ | hr | ⟨pre, f, l, h1, h2⟩
  · refine ⟨hr ▸ hpw, ?_, ?_⟩
    · intro r hm; rw [hr] at hm; have := hbd r hm; rw [hlen]; omega
    · intro p hp i hi
      rw [hlen] at hp
      by_cases hpn : p < ss.length
      · rw [hold p hpn] at hi
        obtain ⟨r, hm, h1, h2, h3⟩ := hcov p hpn i hi
        exact ⟨r, hr ▸ hm, h1, h2, hkeep _ _ h3⟩
      · rw [hlast p (by omega)] at hi; rw [hnone] at hi; cases hi
  · refine ⟨?_, ?_, ?_⟩
    · rw [hr, List.pairwise_append]
      refine ⟨hpw, List.pairwise_singleton _ _, ?_⟩
      intro a ha b hb
      simp only [List.mem_singleton] at hb; subst hb
      have := hbd a ha; simp only; omega
    · intro r hm
      rw [hr, List.mem_append, List.mem_singleton] at hm
      rcases hm with hm | hm
      · have := hbd r hm; rw [hlen]; omega
      · subst hm; simp only [hlen]; omega
    · intro p hp i hi
      rw [hlen] at hp
      by_cases hpn : p < ss.length
      · rw [hold p hpn] at hi
        obtain ⟨r, hm, h1, h2, h3⟩ := hcov p hpn i hi
        exact ⟨r, by rw [hr]; exact List.mem_append_left _ hm, h1, h2, hkeep _ _ h3⟩
      · rw [hlast p (by omega)] at hi
        exact ⟨(T, n, n), by rw [hr]; simp, by simp only; omega, by simp only; omega, hsi i hi⟩
  · rw [h1] at hpw hbd hcov
    rw [List.pairwise_append] at hpw
    obtain ⟨hpw1, _, hpw3⟩ := hpw
    have hfl := hbd (T, f, l) (by simp)
    simp only at hfl
    refine ⟨?_, ?_, ?_⟩
    · rw [h2, List.pairwise_append]
      refine ⟨hpw1, List.pairwise_singleton _ _, ?_⟩
      intro a ha b hb
      simp only [List.mem_singleton] at hb; subst hb
      exact hpw3 a ha (T, f, l) (by simp)
    · intro r hm
      rw [h2, List.mem_append, List.mem_singleton] at hm
      rcases hm with hm | hm
      · have := hbd r (List.mem_append_left _ hm); rw [hlen]; omega
      · subst hm; simp only [hlen]; omega
    · intro p hp i hi
      rw [hlen] at hp
      by_cases hpn : p < ss.length
      · rw [hold p hpn] at hi
        obtain ⟨r, hm, k1, k2, k3⟩ := hcov p hpn i hi
        rw [List.mem_append, List.mem_singleton] at hm
        rcases hm with hm | hm
        · exact ⟨r, by rw [h2]; exact List.mem_append_left _ hm, k1, k2, hkeep _ _ k3⟩
        · subst hm
          exact ⟨(T, f, n), by rw [h2]; simp, k1, by simp only at k2 ⊢; omega, hkeep _ _ k3⟩
      · rw [hlast p (by omega)] at hi
        exact ⟨(T, f, n), by rw [h2]; simp, by simp only; omega, by simp only; omega, hsi i hi⟩

theorem visitsInv_apply (ss : List Step) (s : Step) (st : Fold) (h : VisitsInv ss st)
    (hn : st.gains.val.length = ss.length) (hs : Sized st) (hg : st.gains.val.length < Usize.max) :
    VisitsInv (ss ++ [s]) (applyM st s) := by
  have ht : st.tales.val.length < Usize.max := lt_of_le_of_lt hs.2.1 hg
  have hv : st.visits.val.length < Usize.max := lt_of_le_of_lt hs.2.2 hg
  cases s with
  | Rule r =>
    apply visitsInv_append ss (.Rule r) st _ ss.length 0 rfl h
    · simp only [applyM, applyRuleM]; split <;> exact keepsTales_refl _
    · intro i hi; simp [stepInstance] at hi
    · left
      refine ⟨?_, rfl⟩
      simp only [applyM, applyRuleM]; split <;> rfl
  | Play p =>
    rcases htr : p.track with _ | i
    · apply visitsInv_append ss (.Play p) st _ ss.length 0 rfl h
      · simp only [applyM, applyPlayM, visitsOfM, htr]
        exact keepsTales_of_get _ _ st.visits _ (fun j x hx => leaveM_get _ _ _ _ _ _ hx hv)
      · intro i hi; simp [stepInstance, htr] at hi
      · left
        refine ⟨?_, by simp [stepInstance, htr]⟩
        simp only [applyM, applyPlayM, visitsOfM, htr]
        exact leaveM_ranges _ _ _ _ hv
    · obtain ⟨hat, hkeep, hr⟩ := enterM_facts st.tales st.visits st.visit i (Vec.len st.gains)
        p.at ht hv
      apply visitsInv_append ss (.Play p) st _ ss.length
        (taleOfM st.tales i (Vec.len st.gains)).1.val rfl h
      · simp only [applyM, applyPlayM, visitsOfM, htr]; exact hkeep
      · intro i' hi'
        simp [stepInstance, htr] at hi'; subst hi'
        simp only [applyM, applyPlayM, visitsOfM, htr]; exact hat
      · right
        simp only [applyM, applyPlayM, visitsOfM, htr, addVisitM_ranges]
        simp only [Vec.len_val, Vec.length, hn] at hr
        exact hr

theorem visitsInv_runM (ss : List Step) (h : ss.length ≤ Usize.max) : VisitsInv ss (runM ss) := by
  induction ss using List.reverseRecOn with
  | nil => simp [VisitsInv, ranges, runM, startM]
  | append_singleton ss s ih =>
    simp only [List.length_append, List.length_singleton] at h
    obtain ⟨hl, hs⟩ := runM_sized ss (by omega)
    rw [runM_snoc]
    exact visitsInv_apply ss s _ (ih (by omega)) hl hs (by omega)

/-- A visit holds a step when its range holds it. -/
def Holds (p : Nat) (v : Visit) : Prop := v.first.val ≤ p ∧ p ≤ v.last.val

theorem pairwise_one (l : List Visit) (hl : l.Pairwise (fun a b => a.last.val < b.first.val))
    (p : Nat) : ∀ v ∈ l, ∀ w ∈ l, Holds p v → Holds p w → v = w := by
  induction l with
  | nil => simp
  | cons x xs ih =>
    rw [List.pairwise_cons] at hl
    intro v hv w hw h1 h2
    rcases List.mem_cons.mp hv with hv' | hv' <;> rcases List.mem_cons.mp hw with hw' | hw'
    · rw [hv', hw']
    · subst hv'; have := hl.1 w hw'; simp only [Holds] at h1 h2; omega
    · subst hw'; have := hl.1 v hv'; simp only [Holds] at h1 h2; omega
    · exact ih hl.2 v hv' w hw' h1 h2

/-- Theorem 2. Each step in an instance is in exactly one visit, closed or
open, and that visit is in the tale of its instance. -/
theorem every_instance_step_is_in_one_visit (ss : List Step) (h : ss.length ≤ Usize.max)
    (p : Nat) (hp : p < ss.length) (i : Usize) (hi : stepInstance ss[p] = some i) :
    (∃ v ∈ allVisits (runM ss), Holds p v ∧ TaleAt (runM ss).tales v.tale.val i) ∧
      ∀ v ∈ allVisits (runM ss), ∀ w ∈ allVisits (runM ss), Holds p v → Holds p w → v = w := by
  obtain ⟨hpw, _, hcov⟩ := visitsInv_runM ss h
  obtain ⟨r, hm, h1, h2, h3⟩ := hcov p hp i hi
  unfold ranges at hm hpw
  obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hm
  refine ⟨⟨v, hv, ⟨h1, h2⟩, h3⟩, ?_⟩
  rw [List.pairwise_map] at hpw
  exact pairwise_one _ hpw p

/-! ## The laws as triples on the Rust fold -/

/-- A law of the model holds for the Rust fold: `chapters` gives the model. -/
theorem chapters_holds (ss : Slice Step) (P : Fold → Prop) (h : P (runM ss.val)) :
    chapters.chapters ss ⦃ st => P st ⦄ := by
  rw [chapters_eq]
  simpa [WP.spec_ok] using h

/-- Theorem 1 on the Rust fold. -/
theorem chapters_cover_the_steps (ss : Slice Step) :
    chapters.chapters ss ⦃ st => Covers 0 st.closed.val st.open.first.val ∧
      st.open.first.val ≤ ss.length ⦄ :=
  chapters_holds ss _ (by
    obtain ⟨h1, h2, _⟩ := every_step_is_in_one_chapter ss.val (by simp)
    exact ⟨h1, by simpa using h2⟩)

/-- Theorems 6 and 8 on the Rust fold. -/
theorem chapters_weigh_between_min_and_max (ss : Slice Step) :
    chapters.chapters ss ⦃ st => (∀ c ∈ st.closed.val, (c.close = .Rule ∨ MIN ≤ c.chapter.weight.val) ∧
      c.chapter.weight.val ≤ MAX - 1 + 7) ∧ st.open.weight.val < MAX ⦄ :=
  chapters_holds ss _ (weightsOk_runM ss.val)

/-- Theorem 15 on the Rust fold. -/
theorem chapters_have_one_tale_for_each_instance (ss : Slice Step) :
    chapters.chapters ss ⦃ st => (insts st.tales).Nodup ⦄ :=
  chapters_holds ss _ (an_instance_has_at_most_one_tale ss.val)

/-- Folding one line at a time with `advance` gives the fold of the whole log. -/
theorem advance_one_line_at_a_time (a : List Step) (b : Slice Step)
    (h : a.length + b.length ≤ Usize.max) :
    advance (runM a) b ⦃ st => st = runM (a ++ b.val) ⦄ := by
  obtain ⟨hl, hs⟩ := runM_sized a (by omega)
  rw [advance_eq _ _ hs (by rw [hl]; exact h)]
  simp [WP.spec_ok, folding_line_by_line]

end timeways_rules.chapters
