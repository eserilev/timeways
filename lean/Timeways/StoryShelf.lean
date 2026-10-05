-- The laws of the story shelf (GAMEPLAY.md 4.8): a number stands at most
-- once, a removed story never comes back, and a story that a call used
-- stands after any line that lands.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.story_shelf

deriving instance DecidableEq for ShelfLine

/-! ## The pure model -/

/-- The number of an accepted line. -/
def acceptedOf : ShelfLine → Option U64
  | .Accepted n => some n
  | .Removed _ => none

/-- The numbers of the accepted lines, oldest first. -/
def accepted (ls : List ShelfLine) : List U64 := ls.filterMap acceptedOf

/-- The numbers that stand, as `standing` gives them. -/
def standingSpec (ls : List ShelfLine) : List U64 :=
  (accepted ls).filter (fun n => !decide (ShelfLine.Removed n ∈ ls))

/-- What `lands` decides. -/
def landsP (ls : List ShelfLine) (l : ShelfLine) (used : List U64) : Bool :=
  match l with
  | .Accepted n => !decide (ShelfLine.Accepted n ∈ ls)
  | .Removed n =>
    decide (ShelfLine.Accepted n ∈ ls) && !decide (ShelfLine.Removed n ∈ ls) && !decide (n ∈ used)

/-- Each line landed against the lines before it. The story program only appends a
line that lands, so every shelf of a world is `Landed`. -/
inductive Landed : List ShelfLine → Prop
  | nil : Landed []
  | snoc {ls : List ShelfLine} {l : ShelfLine} {used : List U64} :
      Landed ls → landsP ls l used = true → Landed (ls ++ [l])

theorem mem_accepted (ls : List ShelfLine) (n : U64) :
    n ∈ accepted ls ↔ ShelfLine.Accepted n ∈ ls := by
  unfold accepted
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨l, hl, he⟩
    cases l with
    | Accepted m => simp [acceptedOf] at he; subst he; exact hl
    | Removed m => simp [acceptedOf] at he
  · intro h
    exact ⟨_, h, rfl⟩

theorem mem_standingSpec (ls : List ShelfLine) (n : U64) :
    n ∈ standingSpec ls ↔ ShelfLine.Accepted n ∈ ls ∧ ShelfLine.Removed n ∉ ls := by
  unfold standingSpec
  simp [List.mem_filter, mem_accepted]

theorem accepted_nodup {ls : List ShelfLine} (h : Landed ls) : (accepted ls).Nodup := by
  induction h with
  | nil => simp [accepted]
  | @snoc ls l used _ hl ih =>
    unfold accepted at *
    rw [List.filterMap_append]
    cases l with
    | Removed m => simpa [acceptedOf] using ih
    | Accepted m =>
      simp only [landsP, Bool.not_eq_eq_eq_not, Bool.not_true, decide_eq_false_iff_not] at hl
      simp only [acceptedOf, List.filterMap_cons, List.filterMap_nil]
      rw [List.nodup_append]
      refine ⟨ih, List.nodup_singleton m, ?_⟩
      intro a ha b hb
      simp only [List.mem_singleton] at hb
      subst hb
      intro hab
      subst hab
      exact hl ((mem_accepted ls a).mp ha)

/-! ## The specs of the Rust functions -/

@[step]
theorem is_taken_loop.spec (ls : Slice ShelfLine) (n : U64) (i : Usize)
    (h : ∀ j (hj : j < ls.val.length), j < i.val → ls.val[j] ≠ .Accepted n) :
    is_taken_loop ls n i ⦃ b => (b = true ↔ ShelfLine.Accepted n ∈ ls.val) ⦄ := by
  unfold is_taken_loop
  dsimp only
  split
  · step as ⟨l, hl⟩
    have hrest : ∀ j (hj : j < ls.val.length), j < i.val + 1 → l ≠ .Accepted n →
        ls.val[j] ≠ .Accepted n := by
      intro j hj hji hne
      by_cases hje : j = i.val
      · subst hje; rw [← hl]; exact hne
      · exact h j hj (by omega)
    cases l with
    | Accepted m =>
      dsimp only
      split
      · rename_i hm
        subst hm
        simp only [WP.spec_ok, true_iff]
        rw [hl]
        exact List.getElem_mem _
      · rename_i hm
        step as ⟨i1, hi1⟩
        apply is_taken_loop.spec
        intro j hj hji
        exact hrest j hj (by scalar_tac) (by simp [hm])
    | Removed m =>
      dsimp only
      step as ⟨i1, hi1⟩
      apply is_taken_loop.spec
      intro j hj hji
      exact hrest j hj (by scalar_tac) (by simp)
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro hm
    obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hm
    exact h j hj (by scalar_tac) hjeq
termination_by ls.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem is_taken.spec (ls : Slice ShelfLine) (n : U64) :
    is_taken ls n ⦃ b => (b = true ↔ ShelfLine.Accepted n ∈ ls.val) ⦄ := by
  unfold is_taken
  exact is_taken_loop.spec ls n 0#usize (fun j _ hj => by simp at hj)

@[step]
theorem is_removed_loop.spec (ls : Slice ShelfLine) (n : U64) (i : Usize)
    (h : ∀ j (hj : j < ls.val.length), j < i.val → ls.val[j] ≠ .Removed n) :
    is_removed_loop ls n i ⦃ b => (b = true ↔ ShelfLine.Removed n ∈ ls.val) ⦄ := by
  unfold is_removed_loop
  dsimp only
  split
  · step as ⟨l, hl⟩
    have hrest : ∀ j (hj : j < ls.val.length), j < i.val + 1 → l ≠ .Removed n →
        ls.val[j] ≠ .Removed n := by
      intro j hj hji hne
      by_cases hje : j = i.val
      · subst hje; rw [← hl]; exact hne
      · exact h j hj (by omega)
    cases l with
    | Removed m =>
      dsimp only
      split
      · rename_i hm
        subst hm
        simp only [WP.spec_ok, true_iff]
        rw [hl]
        exact List.getElem_mem _
      · rename_i hm
        step as ⟨i1, hi1⟩
        apply is_removed_loop.spec
        intro j hj hji
        exact hrest j hj (by scalar_tac) (by simp [hm])
    | Accepted m =>
      dsimp only
      step as ⟨i1, hi1⟩
      apply is_removed_loop.spec
      intro j hj hji
      exact hrest j hj (by scalar_tac) (by simp)
  · simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro hm
    obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hm
    exact h j hj (by scalar_tac) hjeq
termination_by ls.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem is_removed.spec (ls : Slice ShelfLine) (n : U64) :
    is_removed ls n ⦃ b => (b = true ↔ ShelfLine.Removed n ∈ ls.val) ⦄ := by
  unfold is_removed
  exact is_removed_loop.spec ls n 0#usize (fun j _ hj => by simp at hj)

@[step]
theorem holds_loop.spec (ns : Slice U64) (n : U64) (i : Usize)
    (h : ∀ j (hj : j < ns.val.length), j < i.val → ns.val[j] ≠ n) :
    holds_loop ns n i ⦃ b => (b = true ↔ n ∈ ns.val) ⦄ := by
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
termination_by ns.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem holds.spec (ns : Slice U64) (n : U64) :
    holds ns n ⦃ b => (b = true ↔ n ∈ ns.val) ⦄ := by
  unfold holds
  exact holds_loop.spec ns n 0#usize (fun j _ hj => by simp at hj)

/-- `lands` never panics, always ends, and decides `landsP`. -/
@[step]
theorem lands.spec (ls : Slice ShelfLine) (l : ShelfLine) (used : Slice U64) :
    lands ls l used ⦃ b => b = landsP ls.val l used.val ⦄ := by
  unfold lands
  cases l with
  | Accepted n =>
    dsimp only
    step as ⟨b, hb⟩
    simp only [landsP]
    cases b <;> simp_all
  | Removed n =>
    dsimp only
    step as ⟨b, hb⟩
    split
    · step as ⟨b1, hb1⟩
      split
      · simp_all [landsP]
      · step as ⟨b2, hb2⟩
        simp only [landsP]
        cases b2 <;> simp_all
    · simp_all [landsP]

theorem accepted_take_succ (ls : List ShelfLine) (i : Nat) (hi : i < ls.length) :
    accepted (ls.take (i + 1)) = accepted (ls.take i) ++ (acceptedOf ls[i]).toList := by
  unfold accepted
  rw [List.take_add_one, List.filterMap_append, List.getElem?_eq_getElem hi]
  cases h : acceptedOf ls[i] <;> simp [h]

/-- The numbers of the first `i` lines that stand. -/
def standingUpTo (ls : List ShelfLine) (i : Nat) : List U64 :=
  (accepted (ls.take i)).filter (fun n => !decide (ShelfLine.Removed n ∈ ls))

theorem standingUpTo_length (ls : List ShelfLine) (i : Nat) : (standingUpTo ls i).length ≤ i := by
  unfold standingUpTo accepted
  calc _ ≤ ((ls.take i).filterMap acceptedOf).length := List.length_filter_le _ _
    _ ≤ (ls.take i).length := List.length_filterMap_le _ _
    _ ≤ i := List.length_take_le _ _

@[step]
theorem standing_loop.spec (ls : Slice ShelfLine) (out : alloc.vec.Vec U64) (i : Usize)
    (hl : out.val = standingUpTo ls.val i.val) (hi : i.val ≤ ls.length) :
    standing_loop ls out i ⦃ r => r.val = standingSpec ls.val ⦄ := by
  unfold standing_loop
  dsimp only
  split
  · step as ⟨l, hl'⟩
    have hlt : i.val < ls.val.length := by scalar_tac
    have hnext : standingUpTo ls.val (i.val + 1) =
        standingUpTo ls.val i.val ++
          ((acceptedOf l).toList.filter (fun n => !decide (ShelfLine.Removed n ∈ ls.val))) := by
      unfold standingUpTo
      rw [accepted_take_succ _ _ hlt, List.filter_append, ← hl']
    cases l with
    | Accepted m =>
      dsimp only
      step as ⟨b, hb⟩
      split
      · rename_i hbt
        step as ⟨i1, hi1⟩
        apply standing_loop.spec
        · rw [hi1, hnext, hl]
          simp [acceptedOf, hb.mp hbt]
        · scalar_tac
      · rename_i hbt
        have hroom : out.val.length < Usize.max := by
          have := standingUpTo_length ls.val i.val
          rw [← hl] at this
          scalar_tac
        step as ⟨out1, hout1⟩
        step as ⟨i1, hi1⟩
        apply standing_loop.spec
        · rw [hout1, hi1, hnext, hl]
          have : ShelfLine.Removed m ∉ ls.val := fun h => hbt (hb.mpr h)
          simp [acceptedOf, this]
        · scalar_tac
    | Removed m =>
      dsimp only
      step as ⟨i1, hi1⟩
      apply standing_loop.spec
      · rw [hi1, hnext, hl]
        simp [acceptedOf]
      · scalar_tac
  · simp only [WP.spec_ok]
    rw [hl]
    unfold standingUpTo standingSpec
    rw [List.take_of_length_le (by scalar_tac)]
termination_by ls.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- `standing` never panics, always ends, and gives `standingSpec`. -/
@[step]
theorem standing.spec (ls : Slice ShelfLine) :
    standing ls ⦃ r => r.val = standingSpec ls.val ⦄ := by
  unfold standing
  apply standing_loop.spec
  · simp [alloc.vec.Vec.new, standingUpTo, accepted]
  · simp

/-! ## The laws -/

/-- No number stands twice. The journal keys a story by its number, and the addon maps a
number to one author. -/
theorem a_number_stands_at_most_once (ls : Slice ShelfLine) (h : Landed ls.val) :
    standing ls ⦃ ns => ns.val.Nodup ⦄ := by
  apply WP.spec_mono (standing.spec ls)
  intro ns hns
  rw [hns]
  exact (accepted_nodup h).sublist List.filter_sublist

/-- After a removal, the number never stands again, whatever lines come after it. -/
theorem a_removed_story_never_comes_back (ls more : List ShelfLine) (shelf : Slice ShelfLine)
    (n : U64) (hs : shelf.val = ls ++ more) (hr : ShelfLine.Removed n ∈ ls) :
    standing shelf ⦃ ns => n ∉ ns.val ⦄ := by
  apply WP.spec_mono (standing.spec shelf)
  intro ns hns
  rw [hns, mem_standingSpec, hs]
  intro ⟨_, hnot⟩
  exact hnot (List.mem_append_left more hr)

/-- A story that a call used stands after any line that lands. -/
theorem a_used_story_always_stands (ls : Slice ShelfLine) (l : ShelfLine) (used : Slice U64)
    (n : U64) (hu : n ∈ used.val) (hs : n ∈ standingSpec ls.val) :
    lands ls l used ⦃ ok => ok = true → n ∈ standingSpec (ls.val ++ [l]) ⦄ := by
  apply WP.spec_mono (lands.spec ls l used)
  intro ok hok hlands
  rw [hok] at hlands
  rw [mem_standingSpec] at hs ⊢
  obtain ⟨hacc, hnot⟩ := hs
  refine ⟨List.mem_append_left _ hacc, ?_⟩
  rw [List.mem_append, List.mem_singleton]
  rintro (h | h)
  · exact hnot h
  · subst h
    simp [landsP, hu] at hlands

end timeways_rules.story_shelf
