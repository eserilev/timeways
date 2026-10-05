-- The bridge: the Rust fold, as Aeneas translates it, computes the pure
-- model of `ChaptersModel.lean`. So it never panics, never overflows,
-- and always ends, while the steps leave room (`Room`, `Sized`).
import Timeways.ChaptersModel

set_option linter.unusedSimpArgs false
open Aeneas Aeneas.Std Result
open timeways_rules.weights
namespace timeways_rules.chapters
open alloc.vec

theorem has_slot_eq (len id : Usize) : has_slot len id = ok (hasSlotM len.val id.val) := by
  unfold has_slot hasSlotM
  split
  · simp_all
  · split
    · simp_all
    · simp_all

theorem first_time_gain_eq (r : KeyRecord) (k : KeyKind) (rule : U8) :
    first_time_gain r k rule = ok (firstTimeGainM r k) := by
  unfold first_time_gain firstTimeGainM
  split <;> simp_all
  cases k <;> rfl

theorem index_eq {α : Type} (v : Vec α) (i : Usize) (d : α) (h : i.val < v.val.length) :
    Vec.index (core.slice.index.SliceIndexUsizeSlice α) v i = ok (getM v i d) := by
  rw [Vec.index_slice_index]
  obtain ⟨y, hy, hv⟩ := WP.spec_imp_exists (Vec.index_usize_spec v i h)
  rw [hy, hv]
  simp [getM, List.getD, h]

theorem index_mut_eq {α : Type} (v : Vec α) (i : Usize) (d : α) (h : i.val < v.val.length) :
    Vec.index_mut (core.slice.index.SliceIndexUsizeSlice α) v i = ok (getM v i d, Vec.set v i) := by
  rw [Vec.index_mut_slice_index]
  obtain ⟨y, hy, hv⟩ := WP.spec_imp_exists (Vec.index_mut_usize_spec v i h)
  rw [hy]
  obtain ⟨a, b⟩ := y
  rw [hv.1, hv.2]
  simp only [getM, List.getD, List.getElem?_eq_getElem h, Option.getD_some]


theorem slot_eq {α : Type} (v : Vec α) (i : Usize) (x : α)
    (h : hasSlotM v.val.length i.val = true) :
    (if i = Vec.len v then Vec.push v x else ok v) = ok (slotted v i x) := by
  unfold slotted
  simp only [hasSlotM, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true] at h
  by_cases he : i.val = v.val.length
  · have : i = Vec.len v := UScalar.eq_imp _ _ (by simp [he])
    simp only [this, if_true, Vec.len_val]
    apply push_eq
    omega
  · have : ¬ i = Vec.len v := fun h => he (by rw [h]; simp)
    simp [this, he]

theorem slotted_lt {α : Type} (v : Vec α) (i : Usize) (x : α)
    (h : hasSlotM v.val.length i.val = true) :
    i.val < (slotted v i x).val.length := by
  unfold slotted
  simp only [hasSlotM, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true] at h
  split
  · rename_i he
    rw [pushM_val _ _ (by omega)]
    simp; omega
  · omega

theorem weightM_le (k : KeyKind) : (weightM k).val ≤ 5 := by
  cases k <;> simp [weightM]

theorem firstTimeGainM_le (r : KeyRecord) (k : KeyKind) : (firstTimeGainM r k).val ≤ 5 := by
  unfold firstTimeGainM
  split
  · simp
  · exact weightM_le k

theorem u16_add_eq (x y : U16) (h : x.val + y.val ≤ U16.max) :
    x + y = ok (core.num.U16.saturating_add x y) := by
  have ⟨z, hz, hv⟩ := WP.spec_imp_exists (U16.add_spec (x := x) (y := y) h)
  rw [hz]
  congr 1
  apply UScalar.eq_imp
  rw [hv, u16_sat_add, sat_add_val]
  simp only [UScalar.max] at *
  scalar_tac

theorem index_mut_usize_eq {α : Type} (v : Vec α) (i : Usize) (d : α)
    (h : i.val < v.val.length) : Vec.index_mut_usize v i = ok (getM v i d, Vec.set v i) := by
  rw [← Vec.index_mut_slice_index]
  exact index_mut_eq v i d h

theorem kill_gain_eq (st : Fold) (r : KeyRecord) (key : Key) (rule : U8) :
    kill_gain st r key rule =
      ok ((killGainM st.foes r key).1, { st with foes := (killGainM st.foes r key).2 }) := by
  unfold kill_gain killGainM
  rw [first_time_gain_eq]
  simp only [bind_tc_ok]
  rcases hf : key.foe with _ | foe
  · simp
  · simp only [has_slot_eq, Vec.len_val]
    by_cases hs : hasSlotM st.foes.val.length foe.val = true
    · simp only [hs, if_true]
      rw [slot_eq _ _ _ hs]
      simp only [bind_tc_ok]
      have hlt := slotted_lt st.foes foe UNBEATEN hs
      rw [index_eq _ _ UNBEATEN hlt, index_mut_eq _ _ UNBEATEN hlt]
      simp only [bind_tc_ok, if_true]
      generalize slotted st.foes foe UNBEATEN = V at *
      split_ifs
      · rfl
      · rfl
      · rfl
      · have := firstTimeGainM_le r key.kind
        rw [u16_add_eq _ _ (by simp [REVENGE]; scalar_tac)]
        simp [REVENGE]
    · simp [hs]

theorem death_weight_eq (r d : U8) : weights.death_weight r d = ok (deathWeightM d) := by
  unfold weights.death_weight deathWeightM
  split
  · rfl
  · rfl
  · rename_i h0 h1
    have a : ¬ d.val = 0 := fun h => h0 (UScalar.eq_imp _ _ (by rw [h]; rfl))
    have b : ¬ d.val = 1 := fun h => h1 (UScalar.eq_imp _ _ (by rw [h]; rfl))
    simp [a, b]

theorem one_more_death_eq (d : U8) : one_more_death d = ok (oneMoreDeathM d) := by
  unfold one_more_death oneMoreDeathM
  simp only [weights.DEATHS_COUNTED]
  split
  · have h : d.val < 2 := by scalar_tac
    simp only [h, if_true]
    have ⟨x, hx, hv⟩ := WP.spec_imp_exists (U8.add_spec (x := d) (y := 1#u8) (by scalar_tac))
    rw [hx]
    congr 1
    apply UScalar.eq_imp
    rw [hv, u8_sat_add, sat_add_val]
    simp only [UScalar.max]
    scalar_tac
  · have h : ¬ d.val < 2 := by scalar_tac
    simp [h]

theorem death_gain_eq (st : Fold) (r : KeyRecord) (foe : Option Usize) (rule : U8) :
    death_gain st r foe rule =
      ok ((deathGainM st.foes r foe).1, { st with foes := (deathGainM st.foes r foe).2.1 },
        (deathGainM st.foes r foe).2.2) := by
  unfold death_gain deathGainM
  rcases foe with _ | foe
  · simp [death_weight_eq, one_more_death_eq]
  · simp only [has_slot_eq, Vec.len_val]
    by_cases hs : hasSlotM st.foes.val.length foe.val = true
    · simp only [hs, if_true]
      rw [slot_eq _ _ _ hs]
      simp only [bind_tc_ok]
      have hlt := slotted_lt st.foes foe UNBEATEN hs
      rw [index_eq _ _ UNBEATEN hlt, index_mut_eq _ _ UNBEATEN hlt]
      simp only [bind_tc_ok, one_more_death_eq, if_true]
      generalize slotted st.foes foe UNBEATEN = V at *
      split_ifs
      · rfl
      · simp [death_weight_eq]
    · simp [hs]

theorem gain_of_eq (st : Fold) (key : Option Key) :
    gain_of st key = ok ((gainM st.keys st.foes key).1,
      { st with keys := (gainM st.keys st.foes key).2.1,
                foes := (gainM st.keys st.foes key).2.2 }) := by
  unfold gain_of gainM
  rcases key with _ | key
  · simp
  · simp only [has_slot_eq, Vec.len_val]
    by_cases hs : hasSlotM st.keys.val.length key.id.val = true
    · simp only [hs, if_true]
      rw [slot_eq _ _ _ hs]
      simp only [bind_tc_ok]
      have hlt := slotted_lt st.keys key.id UNSEEN hs
      rw [index_eq _ _ UNSEEN hlt]
      simp only [bind_tc_ok]
      generalize hV : slotted st.keys key.id UNSEEN = V at *
      simp only [if_true]
      split
      all_goals
        rename_i hk
        simp only [hk, rawGainM, kill_gain_eq, death_gain_eq, first_time_gain_eq]
        try (generalize killGainM st.foes (getM V key.id UNSEEN) key = K; obtain ⟨⟨g, rv⟩, f⟩ := K)
        try (generalize deathGainM st.foes (getM V key.id UNSEEN) key.foe = D; obtain ⟨g, f, r1⟩ := D)
        simp [first_time_gain_eq, kill_gain_eq, death_gain_eq, rawGainM, hk, lift,
          index_mut_usize_eq _ _ UNSEEN hlt]
        split_ifs <;> (try simp only [bind_tc_ok])
    · simp [hs]

theorem RUN_GAP_SECONDS_eq : RUN_GAP_SECONDS = ok RUN_GAP := by
  unfold RUN_GAP_SECONDS RUN_GAP
  rfl

theorem AWAY_SECONDS_eq : AWAY_SECONDS = ok AWAY := by
  unfold AWAY_SECONDS AWAY
  rfl

theorem close_visit_eq (st : Fold) (v : Visit) (h : st.visits.val.length < Usize.max) :
    close_visit st v = ok { st with tales := (closeVisitM st.tales st.visits v).1,
                                    visits := (closeVisitM st.tales st.visits v).2,
                                    visit := none } := by
  unfold close_visit closeVisitM
  rw [push_eq _ _ h]
  simp only [bind_tc_ok, Vec.len_val]
  split
  · rename_i ht
    have ht' : v.tale.val < st.tales.val.length := by scalar_tac
    rw [index_eq _ _ NO_TALE ht', index_mut_eq _ _ NO_TALE ht']
    simp [lift, ht']
  · rename_i ht
    have ht' : ¬ v.tale.val < st.tales.val.length := by scalar_tac
    simp [ht']

theorem leave_instance_eq (st : Fold) (now : U64) (h : st.visits.val.length < Usize.max) :
    leave_instance st now = ok { st with tales := (leaveM st.tales st.visits st.visit now).1,
                                         visits := (leaveM st.tales st.visits st.visit now).2.1,
                                         visit := (leaveM st.tales st.visits st.visit now).2.2 } := by
  unfold leave_instance leaveM
  rcases hv : st.visit with _ | v
  · cases st; simp_all
  · simp only [RUN_GAP_SECONDS_eq, bind_tc_ok, lift]
    split
    · rename_i hle
      have : (core.num.U64.saturating_add v.left_at RUN_GAP).val ≤ now.val := by scalar_tac
      simp only [this, if_true, close_visit_eq _ _ h]
    · rename_i hle
      have : ¬ (core.num.U64.saturating_add v.left_at RUN_GAP).val ≤ now.val := by scalar_tac
      simp only [this, if_false]
      rw [← hv]

theorem usizeOf_val (n : Nat) (h : n ≤ Usize.max) : (usizeOf n).val = n := by
  simp only [usizeOf, UScalar.val, BitVec.toNat_ofNat]
  apply Nat.mod_eq_of_lt
  have : Usize.max = 2 ^ UScalarTy.Usize.numBits - 1 := by simp [Usize.max, Usize.numBits]
  have := Nat.one_le_two_pow (n := UScalarTy.Usize.numBits)
  omega

theorem usizeOf_eq (i : Usize) : usizeOf i.val = i := by
  apply UScalar.eq_imp
  rw [usizeOf_val _ (by scalar_tac)]

theorem findFrom_found (l : List Tale) (inst : Usize) (i : Nat) (hi : i < l.length)
    (he : l[i].instance = inst) : findFrom l inst i = some i := by
  rw [findFrom]; simp [hi, he]

theorem findFrom_next (l : List Tale) (inst : Usize) (i : Nat) (hi : i < l.length)
    (he : ¬ l[i].instance = inst) : findFrom l inst i = findFrom l inst (i + 1) := by
  rw [findFrom]; simp [hi, he]

theorem findFrom_end (l : List Tale) (inst : Usize) (i : Nat) (hi : ¬ i < l.length) :
    findFrom l inst i = none := by
  rw [findFrom]; simp [hi]

theorem tale_of_loop_eq (st : Fold) (inst here i : Usize) (hi : i.val ≤ st.tales.val.length)
    (h : st.tales.val.length < Usize.max) :
    tale_of_loop st inst here i =
      ok ((taleFromM st.tales inst here i.val).1, st.keys, st.foes, st.zones, st.closed, st.open,
        st.pending, (taleFromM st.tales inst here i.val).2, st.visits, st.visit, st.last_at,
        st.gains) := by
  unfold tale_of_loop
  simp only [Vec.len_val]
  split
  · rename_i hlt
    have hlt' : i.val < st.tales.val.length := by scalar_tac
    rw [index_eq _ _ NO_TALE hlt']
    simp only [bind_tc_ok]
    split
    · rename_i he
      have he' : st.tales.val[i.val].instance = inst := by
        simpa [getM, List.getD, hlt'] using he
      simp [taleFromM, findFrom_found _ _ _ hlt' he', usizeOf_eq]
    · rename_i he
      have he' : ¬ st.tales.val[i.val].instance = inst := by
        simpa [getM, List.getD, hlt'] using he
      have ⟨j, hj, hjv⟩ := WP.spec_imp_exists (Usize.add_spec (x := i) (y := 1#usize) (by scalar_tac))
      rw [hj]
      simp only [bind_tc_ok]
      rw [tale_of_loop_eq st inst here j (by scalar_tac) h]
      simp [taleFromM, findFrom_next _ _ _ hlt' he', hjv]
  · rename_i hge
    have hge' : ¬ i.val < st.tales.val.length := by scalar_tac
    rw [push_eq _ _ h]
    have hl : i.val = st.tales.val.length := by omega
    simp [taleFromM, findFrom_end _ _ _ hge', ← hl, usizeOf_eq]
termination_by st.tales.val.length - i.val
decreasing_by scalar_decr_tac

theorem tale_of_eq (st : Fold) (inst here : Usize) (h : st.tales.val.length < Usize.max) :
    tale_of st inst here =
      ok ((taleOfM st.tales inst here).1, { st with tales := (taleOfM st.tales inst here).2 }) := by
  unfold tale_of
  rw [tale_of_loop_eq _ _ _ _ (by simp) h]
  simp [taleOfM]

theorem enter_instance_eq (st : Fold) (inst here : Usize) (now : U64)
    (ht : st.tales.val.length < Usize.max) (hv : st.visits.val.length < Usize.max) :
    enter_instance st inst here now =
      ok { st with tales := (enterM st.tales st.visits st.visit inst here now).1,
                   visits := (enterM st.tales st.visits st.visit inst here now).2.1,
                   visit := (enterM st.tales st.visits st.visit inst here now).2.2 } := by
  unfold enter_instance enterM
  rw [tale_of_eq _ _ _ ht]
  simp only [bind_tc_ok]
  generalize taleOfM st.tales inst here = T
  obtain ⟨tale, tales1⟩ := T
  rcases hvis : st.visit with _ | v
  · simp
  · simp [RUN_GAP_SECONDS_eq, lift]
    by_cases h1 : v.tale = tale
    · by_cases h2 : now.val < (core.num.U64.saturating_add v.left_at RUN_GAP).val
      · have h2' : now < core.num.U64.saturating_add v.left_at RUN_GAP := h2
        simp [h1, h2, h2']
      · have h2' : ¬ now < core.num.U64.saturating_add v.left_at RUN_GAP := h2
        simp [h1, h2, h2', close_visit_eq { st with tales := tales1, visit := some v } v hv]
    · simp [h1, close_visit_eq { st with tales := tales1, visit := some v } v hv]

theorem add_to_visit_eq (st : Fold) (amount : U16) :
    add_to_visit st amount = ok { st with visit := addVisitM st.visit amount } := by
  unfold add_to_visit addVisitM
  rcases hv : st.visit with _ | v
  · cases st; simp_all
  · simp [lift]

theorem wait_for_cut_eq (st : Fold) (b : Break) :
    wait_for_cut st b = ok { st with pending := waitM st.pending b } := by
  unfold wait_for_cut waitM
  rcases hp : st.pending with _ | old
  · simp
  · cases old <;> cases b <;> simp [title_rank, rankM] <;> (cases st; simp_all)

theorem zone_break_eq (st : Fold) (zone : Usize) :
    zone_break st zone = ok (zoneBreakM st.zones st.closed.val.length zone) := by
  unfold zone_break zoneBreakM
  simp only [Vec.len_val]
  split
  · rename_i h
    have h' : st.zones.val.length ≤ zone.val := by scalar_tac
    simp [h']
  · rename_i h
    have h' : ¬ st.zones.val.length ≤ zone.val := by scalar_tac
    have hlt : zone.val < st.zones.val.length := by omega
    rw [index_eq _ _ UNSETTLED hlt]
    simp only [h', if_false, bind_tc_ok, lift]
    split
    · split
      · rename_i _ h1
        have : (core.num.Usize.saturating_add (getM st.zones zone UNSETTLED).last_chapter 1#usize).val
            < st.closed.val.length := by scalar_tac
        simp [this]
      · rename_i _ h1
        have : ¬ (core.num.Usize.saturating_add (getM st.zones zone UNSETTLED).last_chapter 1#usize).val
            < st.closed.val.length := by scalar_tac
        simp [this]
    · simp

theorem note_zone_eq (st : Fold) (zone : Usize) :
    note_zone st zone = ok { st with zones := noteZoneM st.zones zone } := by
  unfold note_zone noteZoneM
  simp only [Vec.len_val, has_slot_eq]
  split
  · rename_i h
    have h' : zone.val = st.zones.val.length := by scalar_tac
    by_cases hm : st.zones.val.length < Usize.max
    · simp [hasSlotM, h', hm, push_eq]
    · simp [hasSlotM, h', hm]
  · rename_i h
    have h' : ¬ zone.val = st.zones.val.length := by scalar_tac
    simp [h']

theorem settle_eq (st : Fold) (zone : Usize) :
    settle st zone = ok { st with zones := settleM st.zones (Vec.len st.closed) zone } := by
  unfold settle settleM
  simp only [Vec.len_val]
  split
  · rename_i h
    have h' : zone.val < st.zones.val.length := by scalar_tac
    rw [index_mut_eq _ _ UNSETTLED h']
    simp [h']
  · rename_i h
    have h' : ¬ zone.val < st.zones.val.length := by scalar_tac
    cases st; simp_all

theorem close_chapter_eq (st : Fold) (last : Usize) (c : Close)
    (h : st.closed.val.length < Usize.max) :
    close_chapter st last c =
      ok { st with closed := pushM st.closed { chapter := st.open, last, close := c } } := by
  unfold close_chapter
  rw [push_eq _ _ h]
  simp

theorem limits_eq (r : U8) : weights.limits r = ok { min := 15#u16, max := 40#u16 } := rfl

theorem usize_add_eq (x y : Usize) (h : x.val + y.val ≤ Usize.max) :
    x + y = ok (core.num.Usize.saturating_add x y) := by
  have ⟨z, hz, hv⟩ := WP.spec_imp_exists (Usize.add_spec (x := x) (y := y) h)
  rw [hz]
  congr 1
  apply UScalar.eq_imp
  rw [hv, usize_sat_add, sat_add_val]
  have : UScalar.max UScalarTy.Usize = Usize.max := by simp [UScalar.max, Usize.max, Usize.numBits]
  rw [this]
  omega

theorem usize_sub_eq (x y : Usize) (h : y.val ≤ x.val) :
    x - y = ok (core.num.Usize.saturating_sub x y) := by
  have ⟨z, hz, hv⟩ := WP.spec_imp_exists (Usize.sub_spec (x := x) (y := y) h)
  rw [hz]
  congr 1
  apply UScalar.eq_imp
  rw [usize_sat_sub, sat_sub_val]
  scalar_tac

theorem add_to_chapter_eq (st : Fold) (zone : Usize) (amount : U16) (here : Usize)
    (hc : st.closed.val.length < Usize.max) (ha : amount.val ≤ 7) (hh : here.val < Usize.max) :
    add_to_chapter st zone amount here =
      ok { st with
        zones := (addChapterM st.zones st.closed st.open st.pending zone amount here).1,
        closed := (addChapterM st.zones st.closed st.open st.pending zone amount here).2.1,
        «open» := (addChapterM st.zones st.closed st.open st.pending zone amount here).2.2.1,
        pending := (addChapterM st.zones st.closed st.open st.pending zone amount here).2.2.2 } := by
  unfold add_to_chapter addChapterM
  by_cases h0 : amount.val = 0
  · have : amount = 0#u16 := by scalar_tac
    simp [this]
  · have : ¬ amount = 0#u16 := by scalar_tac
    simp only [this, h0, if_false, zone_break_eq, bind_tc_ok]
    generalize zoneBreakM st.zones st.closed.val.length zone = o
    simp only [uncurry_apply_pair, limits_eq]
    rcases o with _ | c
    on_goal 1 =>
      simp only [withBreak, bind_tc_ok]
      generalize st.pending = p1
    on_goal 2 =>
      simp only [withBreak, wait_for_cut_eq, bind_tc_ok]
      generalize waitM st.pending c = p1
    all_goals
      rcases p1 with _ | cut
      · simp only [bind_tc_ok, uncurry_apply_pair, lift, settle_eq]
        by_cases hm : 40 ≤ (core.num.U16.saturating_add st.open.weight amount).val
        · have hm' : core.num.U16.saturating_add st.open.weight amount ≥ 40#u16 := by scalar_tac
          simp [hm, hm', MAX, close_chapter_eq, hc, usize_add_eq _ _ (by scalar_tac : here.val + (1#usize).val ≤ Usize.max), core.option.Option.is_none]
          split <;> simp_all [close_chapter_eq]
        · have hm' : ¬ core.num.U16.saturating_add st.open.weight amount ≥ 40#u16 := by scalar_tac
          simp [hm, hm', MAX, core.option.Option.is_none]
          split <;> simp_all
      · have hz : ¬ core.num.U16.saturating_add 0#u16 amount ≥ 40#u16 := by
          rw [u16_sat_add]
          have := sat_add_val (0#u16) amount
          simp only [UScalar.max] at this
          scalar_tac
        have hz' : ¬ MAX ≤ (core.num.U16.saturating_add 0#u16 amount).val := by
          simp only [MAX]; scalar_tac
        have hz'' : ¬ 40 ≤ (core.num.U16.saturating_add 0#u16 amount).val := hz'
        by_cases h15 : MIN ≤ st.open.weight.val
        · by_cases hf : st.open.first.val < here.val
          · have h15' : st.open.weight ≥ 15#u16 := by simp only [MIN] at h15; scalar_tac
            have hf' : st.open.first < here := by scalar_tac
            simp only [bind_tc_ok, uncurry_apply_pair, lift, h15', hf', if_true, h15, hf, and_self,
              usize_sub_eq _ _ (by scalar_tac : (1#usize).val ≤ here.val)]
            simp [close_chapter_eq, hc, settle_eq, hz, hz', hz'', core.option.Option.is_none]
          · have hf' : ¬ st.open.first < here := by scalar_tac
            have h15' : st.open.weight ≥ 15#u16 := by simp only [MIN] at h15; scalar_tac
            simp only [bind_tc_ok, uncurry_apply_pair, lift, h15', hf', if_true, if_false, h15, hf,
              and_false]
            by_cases hm : 40 ≤ (core.num.U16.saturating_add st.open.weight amount).val
            · have hm' : core.num.U16.saturating_add st.open.weight amount ≥ 40#u16 := by scalar_tac
              simp [hm, hm', MAX, close_chapter_eq, hc, settle_eq, usize_add_eq _ _ (by scalar_tac : here.val + (1#usize).val ≤ Usize.max), core.option.Option.is_none]
              split <;> simp_all [close_chapter_eq]
            · have hm' : ¬ core.num.U16.saturating_add st.open.weight amount ≥ 40#u16 := by scalar_tac
              simp [hm, hm', MAX, settle_eq, core.option.Option.is_none]
              split <;> simp_all
        · have h15' : ¬ st.open.weight ≥ 15#u16 := by simp only [MIN] at h15; scalar_tac
          simp only [bind_tc_ok, uncurry_apply_pair, lift, h15', if_false, h15, false_and]
          by_cases hm : 40 ≤ (core.num.U16.saturating_add st.open.weight amount).val
          · have hm' : core.num.U16.saturating_add st.open.weight amount ≥ 40#u16 := by scalar_tac
            simp [hm, hm', MAX, close_chapter_eq, hc, settle_eq, usize_add_eq _ _ (by scalar_tac : here.val + (1#usize).val ≤ Usize.max), core.option.Option.is_none]
            split <;> simp_all [close_chapter_eq]
          · have hm' : ¬ core.num.U16.saturating_add st.open.weight amount ≥ 40#u16 := by scalar_tac
            simp [hm, hm', MAX, settle_eq, core.option.Option.is_none]
            split <;> simp_all

theorem gainM_amount_le (keys : Vec KeyRecord) (foes : Vec FoeRecord) (key : Option Key) :
    (gainM keys foes key).1.1.val ≤ 7 := by
  unfold gainM
  rcases key with _ | key
  · simp
  · simp only
    split
    · generalize rawGainM foes (getM (slotted keys key.id UNSEEN) key.id UNSEEN) key = R
      obtain ⟨raw, rv, f, r1⟩ := R
      simp only
      have := sat_sub_val CAP_MAX r1.gain
      rw [← u16_sat_sub] at this
      have hc : CAP_MAX.val = 7 := by unfold CAP_MAX; rfl
      split <;> scalar_tac
    · simp

/-- The room that one step of the fold needs: the steps so far leave room for one more,
and no vector of the fold that a step can grow is longer than the steps. -/
def Room (st : Fold) : Prop :=
  st.gains.val.length < Usize.max ∧ st.closed.val.length ≤ st.gains.val.length ∧
    st.tales.val.length ≤ st.gains.val.length ∧ st.visits.val.length ≤ st.gains.val.length

theorem apply_play_eq (st : Fold) (play : Play) (h : Room st) :
    apply_play st play = ok (applyPlayM st play) := by
  obtain ⟨hg, hc, ht, hv⟩ := h
  have hc' : st.closed.val.length < Usize.max := by omega
  have ht' : st.tales.val.length < Usize.max := by omega
  have hv' : st.visits.val.length < Usize.max := by omega
  have ha := gainM_amount_le st.keys st.foes play.key
  unfold apply_play applyPlayM
  simp only [note_zone_eq, bind_tc_ok]
  rcases hl : st.last_at with _ | last <;> rcases hm : play.mark with _ | m <;>
    rcases htr : play.track with _ | i
  all_goals
    simp only [pendingOfM, visitsOfM, hl, hm, htr, AWAY_SECONDS_eq, lift, bind_tc_ok,
      uncurry_apply_pair, wait_for_cut_eq]
  all_goals
    try split_ifs
  all_goals
    try simp only [bind_tc_ok, wait_for_cut_eq, hm, htr, uncurry_apply_pair]
  all_goals
    try simp only [bind_tc_ok, wait_for_cut_eq]
    try simp only [leave_instance_eq, hv']
    try simp only [enter_instance_eq, ht', hv']
    simp only [gain_of_eq, bind_tc_ok, uncurry_apply_pair]
    generalize gainM st.keys st.foes play.key = G at *
    obtain ⟨⟨amount, revenge⟩, keys1, foes1⟩ := G
    simp only at ha
    simp [add_to_chapter_eq, add_to_visit_eq, push_eq, hc', ha, hg]
  all_goals (exfalso; scalar_tac)

theorem apply_rule_eq (st : Fold) (rule : U8) (h : Room st) :
    apply_rule st rule = ok (applyRuleM st rule) := by
  obtain ⟨hg, hc, ht, hv⟩ := h
  have hc' : st.closed.val.length < Usize.max := by omega
  unfold apply_rule applyRuleM
  simp only [Vec.len_val]
  split
  · rename_i hf
    have hf' : st.open.first.val < st.gains.val.length := by scalar_tac
    simp [hf', close_chapter_eq, hc', push_eq, hg, ZERO_GAIN]
    rw [usize_sub_eq _ 1#usize (by simp; omega)]
    simp
  · rename_i hf
    have hf' : ¬ st.open.first.val < st.gains.val.length := by scalar_tac
    simp [hf', push_eq, hg, ZERO_GAIN]

theorem apply_eq (st : Fold) (s : Step) (h : Room st) : apply st s = ok (applyM st s) := by
  unfold apply applyM
  cases s
  · exact apply_play_eq _ _ h
  · exact apply_rule_eq _ _ h

/-! ## Sizes -/

theorem set_length {α : Type} (v : Vec α) (i : Usize) (x : α) :
    (v.set i x).val.length = v.val.length := by
  simp

/-- A vector that grew by at most one item at its end. -/
def GrewByOne {α : Type} (a b : Vec α) : Prop := ∃ l : List α, b.val = a.val ++ l ∧ l.length ≤ 1

theorem grew_refl {α : Type} (a : Vec α) : GrewByOne a a := ⟨[], by simp, by simp⟩

theorem grew_push {α : Type} (a : Vec α) (x : α) : GrewByOne a (pushM a x) := by
  unfold pushM
  split
  · exact ⟨[x], by simp, by simp⟩
  · exact grew_refl a

theorem closeVisitM_visits (t : Vec Tale) (vs : Vec Visit) (v : Visit) :
    GrewByOne vs (closeVisitM t vs v).2 := by
  unfold closeVisitM; split <;> exact grew_push _ _

theorem closeVisitM_tales_length (t : Vec Tale) (vs : Vec Visit) (v : Visit) :
    (closeVisitM t vs v).1.val.length = t.val.length := by
  unfold closeVisitM; split <;> simp

theorem leaveM_visits (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (now : U64) :
    GrewByOne vs (leaveM t vs o now).2.1 := by
  unfold leaveM
  rcases o with _ | v
  · exact grew_refl _
  · simp only; split
    · exact closeVisitM_visits _ _ _
    · exact grew_refl _

theorem leaveM_tales_length (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (now : U64) :
    (leaveM t vs o now).1.val.length = t.val.length := by
  unfold leaveM
  rcases o with _ | v
  · rfl
  · simp only; split
    · exact closeVisitM_tales_length _ _ _
    · rfl

theorem taleOfM_tales (t : Vec Tale) (inst here : Usize) : GrewByOne t (taleOfM t inst here).2 := by
  unfold taleOfM taleFromM
  split
  · exact grew_refl _
  · exact grew_push _ _

theorem enterM_visits (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) : GrewByOne vs (enterM t vs o inst here now).2.1 := by
  unfold enterM
  rcases o with _ | v
  · exact grew_refl _
  · simp only; split
    · exact grew_refl _
    · exact closeVisitM_visits _ _ _

theorem enterM_tales_length (t : Vec Tale) (vs : Vec Visit) (o : Option Visit) (inst here : Usize)
    (now : U64) : (enterM t vs o inst here now).1.val.length ≤ t.val.length + 1 := by
  have ⟨l, hl, hl1⟩ := taleOfM_tales t inst here
  unfold enterM
  rcases o with _ | v
  · simp only; rw [hl]; simp; omega
  · simp only; split
    · rw [hl]; simp; omega
    · rw [closeVisitM_tales_length, hl]; simp; omega

theorem addChapterM_closed (zones : Vec ZoneRecord) (closed : Vec ClosedChapter) (opn : Chapter)
    (pending : Option Break) (zone : Usize) (amount : U16) (here : Usize) (ha : amount.val ≤ 7) :
    GrewByOne closed (addChapterM zones closed opn pending zone amount here).2.1 := by
  unfold addChapterM
  split
  · exact grew_refl _
  · simp only
    generalize withBreak pending (zoneBreakM zones closed.val.length zone) = p1
    rcases p1 with _ | cut
    · simp only; split
      · exact grew_push _ _
      · exact grew_refl _
    · simp only
      by_cases hb : MIN ≤ opn.weight.val ∧ opn.first.val < here.val
      · simp only [hb, and_self, if_true]
        have : ¬ MAX ≤ (core.num.U16.saturating_add 0#u16 amount).val := by
          rw [u16_sat_add, sat_add_val]; simp [MAX, UScalar.max]; omega
        simp only [this, if_false]
        exact grew_push _ _
      · simp only [hb, if_false]
        split
        · exact grew_push _ _
        · exact grew_refl _

theorem applyM_closed (st : Fold) (s : Step) : GrewByOne st.closed (applyM st s).closed := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]; split
    · exact grew_push _ _
    · exact grew_refl _
  | Play p =>
    simp only [applyM, applyPlayM]
    rcases p.track with _ | i
    · exact addChapterM_closed _ _ _ _ _ _ _ (gainM_amount_le _ _ _)
    · exact grew_refl _

theorem applyM_visits (st : Fold) (s : Step) : GrewByOne st.visits (applyM st s).visits := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]; split <;> exact grew_refl _
  | Play p =>
    simp only [applyM, applyPlayM, visitsOfM]
    rcases p.track with _ | i
    · exact leaveM_visits _ _ _ _
    · exact enterM_visits _ _ _ _ _ _

theorem applyM_tales_length (st : Fold) (s : Step) :
    (applyM st s).tales.val.length ≤ st.tales.val.length + 1 := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]; split <;> simp
  | Play p =>
    simp only [applyM, applyPlayM, visitsOfM]
    rcases p.track with _ | i
    · simp only; rw [leaveM_tales_length]; omega
    · exact enterM_tales_length _ _ _ _ _ _

theorem applyM_gains (st : Fold) (s : Step) (h : st.gains.val.length < Usize.max) :
    ∃ g, (applyM st s).gains.val = st.gains.val ++ [g] := by
  cases s with
  | Rule r =>
    simp only [applyM, applyRuleM]; split <;> exact ⟨_, pushM_val _ _ h⟩
  | Play p =>
    simp only [applyM, applyPlayM]
    rcases p.track with _ | i <;> exact ⟨_, pushM_val _ _ h⟩

theorem applyM_gains_length (st : Fold) (s : Step) (h : st.gains.val.length < Usize.max) :
    (applyM st s).gains.val.length = st.gains.val.length + 1 := by
  obtain ⟨g, hg⟩ := applyM_gains st s h
  rw [hg]; simp

/-- No vector of the fold that a step can grow is longer than the steps so far. -/
def Sized (st : Fold) : Prop :=
  st.closed.val.length ≤ st.gains.val.length ∧ st.tales.val.length ≤ st.gains.val.length ∧
    st.visits.val.length ≤ st.gains.val.length

theorem applyM_sized (st : Fold) (s : Step) (hs : Sized st)
    (h : st.gains.val.length < Usize.max) : Sized (applyM st s) := by
  obtain ⟨hc, ht, hv⟩ := hs
  have hg := applyM_gains_length st s h
  obtain ⟨lc, hlc, hlc1⟩ := applyM_closed st s
  obtain ⟨lv, hlv, hlv1⟩ := applyM_visits st s
  have htl := applyM_tales_length st s
  refine ⟨?_, ?_, ?_⟩
  · rw [hlc, hg]; simp; omega
  · rw [hg]; omega
  · rw [hlv, hg]; simp; omega

theorem advance_loop_eq (st : Fold) (ss : Slice Step) (i : Usize) (hi : i.val ≤ ss.length)
    (hs : Sized st) (hr : st.gains.val.length + (ss.length - i.val) ≤ Usize.max) :
    advance_loop st ss i = ok ((ss.val.drop i.val).foldl applyM st) := by
  unfold advance_loop
  simp only [Slice.len_val]
  split
  · rename_i hlt
    have hlt' : i.val < ss.length := by scalar_tac
    have hg : st.gains.val.length < Usize.max := by omega
    have ⟨x, hx, hxv⟩ := WP.spec_imp_exists (Slice.index_usize_spec ss i hlt')
    rw [hx]
    simp only [bind_tc_ok]
    rw [apply_eq _ _ ⟨hg, hs⟩]
    have ⟨j, hj, hjv⟩ := WP.spec_imp_exists (Usize.add_spec (x := i) (y := 1#usize) (by scalar_tac))
    simp only [bind_tc_ok, hj]
    rw [advance_loop_eq _ ss j (by scalar_tac) (applyM_sized _ _ hs hg)
      (by rw [applyM_gains_length _ _ hg]; scalar_tac)]
    have hjv' : j.val = i.val + 1 := by simpa using hjv
    rw [hjv', hxv, List.drop_eq_getElem_cons (i := i.val) (l := ss.val) (by simpa using hlt')]
    rfl
  · rename_i hge
    have : ss.val.length ≤ i.val := by scalar_tac
    simp [List.drop_eq_nil_of_le this]
termination_by ss.length - i.val
decreasing_by scalar_decr_tac

theorem advance_eq (st : Fold) (ss : Slice Step) (hs : Sized st)
    (hr : st.gains.val.length + ss.length ≤ Usize.max) :
    advance st ss = ok (ss.val.foldl applyM st) := by
  unfold advance
  rw [advance_loop_eq _ _ _ (by simp) hs (by simpa using hr)]
  simp

theorem start_eq : start = ok startM := by
  unfold start startM
  simp [weights.RULE_ONE]

theorem chapters_eq (ss : Slice Step) : chapters.chapters ss = ok (runM ss.val) := by
  unfold chapters.chapters
  rw [start_eq]
  simp only [bind_tc_ok]
  rw [advance_eq _ _ (by simp [Sized, startM]) (by simp [startM])]
  rfl

end timeways_rules.chapters
