-- The laws of deaths and revenge in the chapter fold, past theorems 17 and
-- 18 of `Chapters.lean`: revenge needs an earlier death to the foe and adds
-- 2, and the deaths with no known killer weigh 2, then 1, then nothing.
import Timeways.Chapters

set_option linter.unusedSimpArgs false

open Aeneas Aeneas.Std Result
open timeways_rules.weights

namespace timeways_rules.chapters

open alloc.vec

/-! ## The deaths that the foe records hold -/

theorem foeDeaths_slotted (foes : Vec FoeRecord) (i f : Usize) :
    foeDeaths (slotted foes i UNBEATEN) f = foeDeaths foes f := by
  unfold slotted
  split
  · unfold pushM
    split
    · rename_i he hroom
      unfold foeDeaths
      simp only [Vec.from_val]
      by_cases hlt : f.val < foes.val.length
      · rw [List.getElem?_append_left hlt]
      · rw [List.getElem?_eq_none (by omega : foes.val.length ≤ f.val)]
        by_cases hf : f.val = foes.val.length
        · rw [List.getElem?_append_right (by omega), hf, Nat.sub_self]
          simp [UNBEATEN]
        · rw [List.getElem?_eq_none (by simp; omega)]
    · rfl
  · rfl

theorem foeDeaths_set_ne (v : Vec FoeRecord) (i f : Usize) (x : FoeRecord)
    (h : i.val ≠ f.val) : foeDeaths (v.set i x) f = foeDeaths v f := by
  unfold foeDeaths
  simp only [Vec.set_val_eq]
  rw [List.getElem?_set_ne h]

theorem foeDeaths_set_self (v : Vec FoeRecord) (f : Usize) (x : FoeRecord)
    (h : f.val < v.val.length) : foeDeaths (v.set f x) f = x.deaths.val := by
  unfold foeDeaths
  simp [Vec.set_val_eq, h]

theorem getM_deaths (v : Vec FoeRecord) (f : Usize) (h : f.val < v.val.length) :
    (getM v f UNBEATEN).deaths.val = foeDeaths v f := by
  simp [getM, foeDeaths, List.getD, h]

/-- A kill never changes the deaths of a foe. -/
theorem killGainM_deaths (foes : Vec FoeRecord) (r : KeyRecord) (k : Key) (f : Usize) :
    foeDeaths (killGainM foes r k).2 f = foeDeaths foes f := by
  unfold killGainM
  rcases hk : k.foe with _ | g
  · rfl
  · simp only
    split
    · rename_i hs
      have hlt := slotted_lt foes g UNBEATEN hs
      rw [← foeDeaths_slotted foes g f]
      generalize slotted foes g UNBEATEN = V at *
      have hset : ∀ x : FoeRecord, x.deaths = (getM V g UNBEATEN).deaths →
          foeDeaths (V.set g x) f = foeDeaths V f := by
        intro x hx
        by_cases hgf : g.val = f.val
        · have hgf' : g = f := UScalar.eq_of_val_eq hgf
          subst hgf'
          rw [foeDeaths_set_self _ _ _ hlt, hx, getM_deaths _ _ hlt]
        · exact foeDeaths_set_ne _ _ _ _ hgf
      split_ifs <;> exact hset _ rfl
    · rfl

/-- A death changes only the deaths of its own foe. -/
theorem deathGainM_deaths (foes : Vec FoeRecord) (r : KeyRecord) (foe : Option Usize) (f : Usize)
    (h : foe ≠ some f) : foeDeaths (deathGainM foes r foe).2.1 f = foeDeaths foes f := by
  unfold deathGainM
  rcases foe with _ | g
  · rfl
  · have hgf : g.val ≠ f.val := fun he => h (by rw [UScalar.eq_of_val_eq he])
    simp only
    split
    · rw [← foeDeaths_slotted foes g f]
      split <;> exact foeDeaths_set_ne _ _ _ _ hgf
    · rfl

theorem rawGainM_deaths (foes : Vec FoeRecord) (r : KeyRecord) (k : Key) (f : Usize)
    (h : ¬ (k.kind = .Death ∧ k.foe = some f)) :
    foeDeaths (rawGainM foes r k).2.2.1 f = foeDeaths foes f := by
  unfold rawGainM
  rcases hk : k.kind
  all_goals simp only
  all_goals first
    | rfl
    | exact killGainM_deaths _ _ _ _
    | exact deathGainM_deaths _ _ _ _ (fun he => h ⟨hk, he⟩)

theorem gainM_foes_some (keys : Vec KeyRecord) (foes : Vec FoeRecord) (k : Key) :
    (gainM keys foes (some k)).2.2 = foes ∨
      (gainM keys foes (some k)).2.2 =
        (rawGainM foes (getM (slotted keys k.id UNSEEN) k.id UNSEEN) k).2.2.1 := by
  by_cases hs : hasSlotM keys.val.length k.id.val = true
  · exact Or.inr (gainM_some keys foes k hs).1
  · exact Or.inl (by rw [gainM_noslot _ _ _ hs])

theorem applyM_foes (st : Fold) (p : Play) :
    (applyM st (.Play p)).foes = (gainM st.keys st.foes p.key).2.2 := by
  simp only [applyM, applyPlayM]; rcases p.track <;> rfl

/-- A step that is no death to foe `f` leaves the deaths of `f` as they were. -/
theorem applyM_deaths (st : Fold) (s : Step) (f : Usize) (h : deathOf f s = false) :
    foeDeaths (applyM st s).foes f = foeDeaths st.foes f := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]; split <;> rfl
  | Play p =>
    rw [applyM_foes]
    rcases hkey : p.key with _ | k
    · rfl
    · simp only [deathOf, hkey, Bool.and_eq_false_iff, decide_eq_false_iff_not] at h
      rcases gainM_foes_some st.keys st.foes k with he | he <;> rw [he]
      apply rawGainM_deaths
      rintro ⟨h1, h2⟩
      rcases h with h | h <;> contradiction

/-- A foe with a death in the fold of a log has a death step in the log. -/
theorem a_death_in_the_fold_is_a_step_of_the_log (f : Usize) (ss : List Step)
    (h : 0 < foeDeaths (runM ss).foes f) : ∃ d ∈ ss, deathOf f d = true := by
  induction ss using List.reverseRecOn with
  | nil => simp [runM, startM, foeDeaths] at h
  | append_singleton ss s ih =>
    rw [runM_snoc] at h
    by_cases hd : deathOf f s = true
    · exact ⟨s, by simp, hd⟩
    · rw [applyM_deaths _ _ _ (by simpa using hd)] at h
      obtain ⟨d, hd', hdd⟩ := ih h
      exact ⟨d, by simp [hd'], hdd⟩

/-! ## Revenge needs an earlier death, and adds 2 -/

/-- Every key that the fold has not seen has gained nothing yet. -/
def UnseenEmpty (st : Fold) : Prop :=
  ∀ r ∈ st.keys.val, r.seen = false → r.gain.val = 0

theorem mem_slotted {α : Type} (v : Vec α) (i : Usize) (d x : α)
    (h : x ∈ (slotted v i d).val) : x ∈ v.val ∨ x = d := by
  unfold slotted pushM at h
  split at h
  · split at h
    · simp only [Vec.from_val, List.mem_append, List.mem_singleton] at h; exact h
    · exact Or.inl h
  · exact Or.inl h

theorem slotted_unseen (keys : Vec KeyRecord) (i : Usize)
    (h : ∀ r ∈ keys.val, r.seen = false → r.gain.val = 0) :
    ∀ r ∈ (slotted keys i UNSEEN).val, r.seen = false → r.gain.val = 0 := by
  intro r hr
  rcases mem_slotted _ _ _ _ hr with hr | rfl
  · exact h r hr
  · intro _; simp [UNSEEN]

theorem gainM_unseen (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Option Key)
    (h : ∀ r ∈ keys.val, r.seen = false → r.gain.val = 0) :
    ∀ r ∈ (gainM keys foes key).2.1.val, r.seen = false → r.gain.val = 0 := by
  unfold gainM
  rcases key with _ | k
  · exact h
  · simp only
    split
    · have hsl := slotted_unseen keys k.id h
      generalize slotted keys k.id UNSEEN = V at *
      generalize rawGainM foes (getM V k.id UNSEEN) k = R
      obtain ⟨raw, rv, f1, r1⟩ := R
      intro r hr
      simp only [Vec.set_val_eq] at hr
      rcases List.mem_or_eq_of_mem_set hr with hr | hr
      · exact hsl r hr
      · subst hr; simp
    · exact h

theorem unseenEmpty_apply (st : Fold) (s : Step) (h : UnseenEmpty st) :
    UnseenEmpty (applyM st s) := by
  cases s with
  | Rule r => simp only [applyM, applyRuleM]; split <;> exact h
  | Play p =>
    have hk : (applyM st (.Play p)).keys = (gainM st.keys st.foes p.key).2.1 := by
      simp only [applyM, applyPlayM]; rcases p.track <;> rfl
    unfold UnseenEmpty
    rw [hk]
    exact gainM_unseen _ _ _ h

theorem unseenEmpty_runM (ss : List Step) : UnseenEmpty (runM ss) := by
  induction ss using List.reverseRecOn with
  | nil => simp [UnseenEmpty, runM, startM]
  | append_singleton ss s ih => rw [runM_snoc]; exact unseenEmpty_apply _ _ ih

theorem killGainM_revenge_facts (foes : Vec FoeRecord) (r : KeyRecord) (k : Key)
    (h : (killGainM foes r k).1.2 = true) :
    ∃ f, k.foe = some f ∧ 0 < foeDeaths foes f ∧ r.seen = false ∧
      (killGainM foes r k).1.1 = core.num.U16.saturating_add (weightM k.kind) 2#u16 := by
  unfold killGainM at *
  rcases hf : k.foe with _ | f
  · simp [hf] at h
  · simp only [hf] at h ⊢
    split at h
    · rename_i hs
      have hlt := slotted_lt foes f UNBEATEN hs
      simp only [hs, if_true]
      split_ifs at h ⊢ with h1 h2 h3 <;> try (simp at h; done)
      refine ⟨f, rfl, ?_, by simpa using h1, ?_⟩
      · rw [← foeDeaths_slotted foes f f, ← getM_deaths _ _ hlt]
        have : (getM (slotted foes f UNBEATEN) f UNBEATEN).deaths.val ≠ 0 := by
          intro h0; exact h3 (UScalar.eq_of_val_eq (by simpa using h0))
        omega
      · simp [firstTimeGainM, h1]
    · simp at h

/-- Theorem 18b. A step adds revenge only for the kill of a foe that has a death to it in
the fold, so an earlier step of the log is a death to that foe. The revenge adds 2 to the
weight of the kill: the gain of the step is the weight of its kind plus 2. -/
theorem revenge_needs_an_earlier_death (ss : List Step) (s : Step)
    (h : (gainOfStep (runM ss) s).revenge = true) :
    ∃ p k f, s = .Play p ∧ p.key = some k ∧ k.foe = some f ∧
      (k.kind = .Kill ∨ k.kind = .RaidKill) ∧
      0 < foeDeaths (runM ss).foes f ∧ (∃ d ∈ ss, deathOf f d = true) ∧
      (gainOfStep (runM ss) s).amount.val = (weightM k.kind).val + 2 := by
  cases s with
  | Rule r => simp [gainOfStep, ZERO_GAIN] at h
  | Play p =>
    rcases hkey : p.key with _ | k
    · simp [gainOfStep, hkey, gainM] at h
    · simp only [gainOfStep, hkey] at h ⊢
      have hs : hasSlotM (runM ss).keys.val.length k.id.val = true := by
        by_contra hs
        rw [gainM_noslot _ _ _ hs] at h
        simp at h
      have hlt := slotted_lt (runM ss).keys k.id UNSEEN hs
      have hun := unseenEmpty_runM ss
      simp only [gainM, hs, if_true] at h ⊢
      generalize hV : slotted (runM ss).keys k.id UNSEEN = V at *
      generalize hr : getM V k.id UNSEEN = r
      rw [hr] at h
      have hrun : r.seen = false → r.gain.val = 0 := by
        intro hsn
        rw [← hr]
        unfold getM
        rw [List.getD_eq_getElem _ _ hlt]
        apply (show ∀ x ∈ V.val, x.seen = false → x.gain.val = 0 by
          rw [← hV]; exact slotted_unseen _ _ hun) _ (List.getElem_mem _)
        rw [← hr] at hsn
        unfold getM at hsn
        rwa [List.getD_eq_getElem _ _ hlt] at hsn
      have hkind : k.kind = .Kill ∨ k.kind = .RaidKill := by
        unfold rawGainM at h
        rcases hk : k.kind <;> simp_all
      have hraw : (rawGainM (runM ss).foes r k).2.1 = (killGainM (runM ss).foes r k).1.2 ∧
          (rawGainM (runM ss).foes r k).1 = (killGainM (runM ss).foes r k).1.1 := by
        unfold rawGainM
        rcases hkind with hk | hk <;> simp [hk]
      rw [hraw.1] at h
      obtain ⟨f, hf, hd, hsn, hamt⟩ := killGainM_revenge_facts _ _ _ h
      refine ⟨p, k, f, rfl, hkey, hf, hkind, hd,
        a_death_in_the_fold_is_a_step_of_the_log f ss hd, ?_⟩
      have hg := hrun hsn
      have hgain := rawGainM_gain (runM ss).foes r k
      generalize hR : rawGainM (runM ss).foes r k = R at *
      obtain ⟨raw, rv, f1, r1⟩ := R
      simp only at hraw hgain ⊢
      rw [hraw.2, hamt] at *
      have hw := weightM_le k.kind
      have hsum : (core.num.U16.saturating_add (weightM k.kind) 2#u16).val =
          (weightM k.kind).val + 2 := by
        rw [u16_sat_add, sat_add_val]; simp only [UScalar.max]; scalar_tac
      have hroom : (core.num.U16.saturating_sub CAP_MAX r1.gain).val = 7 := by
        have hcap : CAP_MAX.val = 7 := by unfold CAP_MAX; rfl
        rw [u16_sat_sub, sat_sub_val, hgain, hg, hcap]
      split <;> scalar_tac

/-! ## Deaths with no known killer

The walk gives each death with no known killer the key `(DeathIn, zone)`, or
`NoZoneDeath` with no zone. So the deaths of one zone share one key, and the
key record counts them. -/

/-- The deaths with no known killer that the record of key `i` counts, or 0. -/
def keyDeaths (keys : Vec KeyRecord) (i : Usize) : Nat :=
  (keys.val[i.val]?.map (fun r => r.deaths.val)).getD 0

/-- The step is a death with no known killer, with key `i`. -/
def noKillerDeathOf (i : Usize) : Step → Bool
  | .Play p => match p.key with
    | some k => decide (k.kind = .Death) && decide (k.foe = none) && decide (k.id = i)
    | none => false
  | .Rule _ => false

/-- The gain that the deaths with no known killer of key `i` added in a log. -/
def noKillerTotal (i : Usize) (ss : List Step) : Nat :=
  (((ss.zip (runM ss).gains.val).filter (fun sg => noKillerDeathOf i sg.1)).map
    (fun sg => sg.2.amount.val)).sum

theorem keyDeaths_mono (a b : Vec KeyRecord) (h : VecLe KeyLe a b) (i : Usize) :
    keyDeaths a i ≤ keyDeaths b i := by
  unfold keyDeaths
  rcases ha : a.val[i.val]? with _ | x
  · simp
  · obtain ⟨y, hy, hxy⟩ := h _ _ ha
    simp [hy, hxy.2.1]

theorem keyDeaths_slotted (keys : Vec KeyRecord) (i j : Usize) :
    keyDeaths (slotted keys i UNSEEN) j = keyDeaths keys j := by
  unfold slotted
  split
  · unfold pushM
    split
    · unfold keyDeaths
      simp only [Vec.from_val]
      by_cases hlt : j.val < keys.val.length
      · rw [List.getElem?_append_left hlt]
      · rw [List.getElem?_eq_none (by omega : keys.val.length ≤ j.val)]
        by_cases hj : j.val = keys.val.length
        · rw [List.getElem?_append_right (by omega), hj, Nat.sub_self]
          simp [UNSEEN]
        · rw [List.getElem?_eq_none (by simp; omega)]
    · rfl
  · rfl

theorem deathWeightM_val (d : U8) :
    (deathWeightM d).val = if d.val = 0 then 2 else if d.val = 1 then 1 else 0 := by
  unfold deathWeightM
  split_ifs <;> rfl

/-- One death with no known killer, from any fold. When the key has room under its cap,
the death weighs 2 at the first death of the key, 1 at the second, and 0 after. The key
then counts one death more, up to 2. A damaged fold can hold more than 2: the count then
stays. -/
theorem a_death_with_no_killer_weighs_two_then_one_then_nothing (st : Fold) (k : Key)
    (hk : k.kind = .Death) (hf : k.foe = none)
    (hs : hasSlotM st.keys.val.length k.id.val = true)
    (hroom : ∀ r, st.keys.val[k.id.val]? = some r →
      (deathWeightM r.deaths).val + r.gain.val ≤ 7) :
    let d := keyDeaths st.keys k.id
    (gainM st.keys st.foes (some k)).1.1.val = (if d = 0 then 2 else if d = 1 then 1 else 0) ∧
      keyDeaths (gainM st.keys st.foes (some k)).2.1 k.id = (if d < 2 then d + 1 else d) := by
  have hlt := slotted_lt st.keys k.id UNSEEN hs
  have hd : (getM (slotted st.keys k.id UNSEEN) k.id UNSEEN).deaths.val =
      keyDeaths st.keys k.id := by
    rw [← keyDeaths_slotted st.keys k.id k.id]
    simp [getM, keyDeaths, List.getD, hlt]
  have hroom' : (deathWeightM (getM (slotted st.keys k.id UNSEEN) k.id UNSEEN).deaths).val +
      (getM (slotted st.keys k.id UNSEEN) k.id UNSEEN).gain.val ≤ 7 := by
    unfold slotted
    split
    · rename_i he
      have hnone : st.keys.val[k.id.val]? = none := by simp; omega
      have hget : getM (pushM st.keys UNSEEN) k.id UNSEEN = UNSEEN := by
        unfold getM pushM; split
        · simp [List.getD, he]
        · simp [List.getD, hnone]
      rw [hget]; simp [UNSEEN, deathWeightM]
    · rename_i he
      have hl : k.id.val < st.keys.val.length := by
        simp only [hasSlotM, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true] at hs; omega
      have := hroom _ (List.getElem?_eq_getElem hl)
      simpa [getM, List.getD, hl] using this
  simp only
  simp only [gainM, hs, if_true]
  rw [← hd]
  generalize hV : slotted st.keys k.id UNSEEN = V at *
  generalize hr : getM V k.id UNSEEN = r at *
  have hraw : rawGainM st.foes r k =
      (deathWeightM r.deaths, false, st.foes, { r with deaths := oneMoreDeathM r.deaths }) := by
    simp [rawGainM, hk, deathGainM, hf]
  rw [hraw]
  simp only
  have hcap : CAP_MAX.val = 7 := by unfold CAP_MAX; rfl
  have hroomv : (core.num.U16.saturating_sub CAP_MAX r.gain).val = 7 - r.gain.val := by
    rw [u16_sat_sub, sat_sub_val, hcap]
  refine ⟨?_, ?_⟩
  · rw [← deathWeightM_val]
    split <;> scalar_tac
  · unfold keyDeaths
    simp only [Vec.set_val_eq, List.getElem?_set_self hlt, Option.map_some, Option.getD_some]
    exact oneMoreDeathM_val _

/-- What the deaths with no known killer of a key can still add: 3, then 1, then 0. -/
theorem no_killer_step (st : Fold) (s : Step) (i : Usize) :
    (if noKillerDeathOf i s then (gainOfStep st s).amount.val else 0) +
      deathRoom (keyDeaths (applyM st s).keys i) ≤ deathRoom (keyDeaths st.keys i) := by
  have hmono := keyDeaths_mono _ _ (applyM_le st s).1 i
  split
  · rename_i hd
    cases s with
    | Rule r => simp [noKillerDeathOf] at hd
    | Play p =>
      rcases hkey : p.key with _ | k
      · simp [noKillerDeathOf, hkey] at hd
      · simp only [noKillerDeathOf, hkey, Bool.and_eq_true, decide_eq_true_eq] at hd
        obtain ⟨⟨hk, hf⟩, rfl⟩ := hd
        have hkeys : (applyM st (.Play p)).keys = (gainM st.keys st.foes p.key).2.1 := by
          simp only [applyM, applyPlayM]; rcases p.track <;> rfl
        rw [hkeys]
        simp only [gainOfStep, hkey]
        by_cases hs : hasSlotM st.keys.val.length k.id.val = true
        · have hlt := slotted_lt st.keys k.id UNSEEN hs
          have hd : (getM (slotted st.keys k.id UNSEEN) k.id UNSEEN).deaths.val =
              keyDeaths st.keys k.id := by
            rw [← keyDeaths_slotted st.keys k.id k.id]
            simp [getM, keyDeaths, List.getD, hlt]
          simp only [gainM, hs, if_true]
          rw [← hd]
          generalize hV : slotted st.keys k.id UNSEEN = V at *
          generalize hr : getM V k.id UNSEEN = r at *
          have hraw : rawGainM st.foes r k =
              (deathWeightM r.deaths, false, st.foes,
                { r with deaths := oneMoreDeathM r.deaths }) := by
            simp [rawGainM, hk, deathGainM, hf]
          rw [hraw]
          simp only
          have hamt : (if deathWeightM r.deaths < core.num.U16.saturating_sub CAP_MAX r.gain
              then deathWeightM r.deaths else core.num.U16.saturating_sub CAP_MAX r.gain).val ≤
              (deathWeightM r.deaths).val := by
            split <;> scalar_tac
          have hafter : ∀ x : KeyRecord, keyDeaths (V.set k.id x) k.id = x.deaths.val := by
            intro x
            unfold keyDeaths
            simp [Vec.set_val_eq, hlt]
          rw [hafter]
          simp only
          rw [oneMoreDeathM_val]
          rw [deathWeightM_val] at hamt
          revert hamt
          generalize (if deathWeightM r.deaths < core.num.U16.saturating_sub CAP_MAX r.gain
            then deathWeightM r.deaths else core.num.U16.saturating_sub CAP_MAX r.gain) = amt
          generalize r.deaths.val = d
          intro hamt
          unfold deathRoom
          rcases (by omega : d = 0 ∨ d = 1 ∨ 2 ≤ d) with h | h | h
          · subst h; simp at hamt ⊢; omega
          · subst h; simp at hamt ⊢; omega
          · simp [show ¬ d < 2 by omega, show d ≠ 0 by omega, show d ≠ 1 by omega] at hamt ⊢
            omega
        · rw [gainM_noslot _ _ _ hs]; simp
  · simp only [Nat.zero_add]; exact deathRoom_anti hmono

theorem noKillerTotal_snoc (i : Usize) (ss : List Step) (s : Step)
    (h : ss.length + 1 ≤ Usize.max) :
    noKillerTotal i (ss ++ [s]) = noKillerTotal i ss +
      (if noKillerDeathOf i s then (gainOfStep (runM ss) s).amount.val else 0) := by
  unfold noKillerTotal
  rw [runM_gains ss s h, zip_snoc _ _ _ _ (runM_sized ss (by omega)).1.symm]
  rw [List.filter_append]
  split <;> simp_all

/-- Theorem 17b. The deaths with no known killer of one key gain 3 at most in all: 2, then
1, then nothing. The walk gives one key to each zone, so the deaths of one zone with no
known killer weigh 3 at most, also 100 of them. -/
theorem deaths_with_no_killer_weigh_at_most_three (i : Usize) (ss : List Step)
    (h : ss.length ≤ Usize.max) : noKillerTotal i ss ≤ 3 := by
  have key : noKillerTotal i ss + deathRoom (keyDeaths (runM ss).keys i) ≤ 3 := by
    induction ss using List.reverseRecOn with
    | nil => simp [noKillerTotal, runM, startM, keyDeaths, deathRoom]
    | append_singleton ss s ih =>
      simp only [List.length_append, List.length_singleton] at h
      have ih := ih (by omega)
      rw [noKillerTotal_snoc i ss s h, runM_snoc]
      have := no_killer_step (runM ss) s i
      omega
  omega

end timeways_rules.chapters
