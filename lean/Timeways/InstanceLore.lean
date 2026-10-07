-- The laws of the lore of an entry into a dungeon or a raid (GAMEPLAY.md 3.2): an entry
-- tells the first passage that the character was never told, so no passage is told twice,
-- every entry is silent once every passage was told, and the pick never panics.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.instance_lore

/-- What the pick gives: the index of the first passage with no telling, or none when
every passage was told. -/
def picks (told : Slice U32) (r : Option Usize) : Prop :=
  match r with
  | some i =>
    ∃ hi : i.val < told.val.length, told.val[i.val] = 0#u32 ∧
      ∀ j (hj : j < told.val.length), j < i.val → told.val[j] ≠ 0#u32
  | none => ∀ j (hj : j < told.val.length), told.val[j] ≠ 0#u32

@[step]
theorem next_passage_loop.spec (told : Slice U32) (i : Usize)
    (h : ∀ j (hj : j < told.val.length), j < i.val → told.val[j] ≠ 0#u32) :
    next_passage_loop told i ⦃ r => picks told r ⦄ := by
  unfold next_passage_loop
  dsimp only
  split
  · step as ⟨m, hm⟩
    split
    · rename_i he
      simp only [WP.spec_ok, picks]
      refine ⟨by scalar_tac, ?_, h⟩
      rw [← hm]
      exact he
    · rename_i he
      step as ⟨i1, hi1⟩
      apply next_passage_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje
        rw [← hm]
        exact he
      · exact h j hj (by scalar_tac)
  · simp only [WP.spec_ok, picks]
    intro j hj
    exact h j hj (by scalar_tac)
termination_by told.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The pick, exactly: the first passage in pack order that was never told, or none. -/
@[step]
theorem next_passage.spec (told : Slice U32) :
    next_passage told ⦃ r => picks told r ⦄ := by
  unfold next_passage
  exact next_passage_loop.spec told 0#usize (fun j _ hj => by simp at hj)

/-- The pick never panics, and it always ends. -/
theorem the_instance_pick_never_panics (told : Slice U32) :
    ∃ r, next_passage told = ok r := by
  obtain ⟨r, hr, _⟩ := WP.spec_imp_exists (next_passage.spec told)
  exact ⟨r, hr⟩

/-- An entry tells only a passage that was never told. Telling it makes its count one,
so no later entry tells it again: each passage is told at most once. -/
theorem an_instance_passage_is_told_at_most_once (told : Slice U32) (i : Usize)
    (h_pick : next_passage told = ok (some i)) :
    ∃ hi : i.val < told.val.length, told.val[i.val] = 0#u32 := by
  obtain ⟨r, hr, hp⟩ := WP.spec_imp_exists (next_passage.spec told)
  rw [h_pick] at hr
  have hr' : r = some i := (Result.ok_injective hr).symm
  subst hr'
  obtain ⟨hi, h0, _⟩ := hp
  exact ⟨hi, h0⟩

/-- Once every passage of the instance was told, an entry tells nothing. -/
theorem entries_are_silent_once_every_passage_is_told (told : Slice U32)
    (h_all : ∀ j (hj : j < told.val.length), told.val[j] ≠ 0#u32) :
    next_passage told = ok none := by
  obtain ⟨r, hr, hp⟩ := WP.spec_imp_exists (next_passage.spec told)
  rw [hr]
  cases r with
  | none => rfl
  | some i =>
    obtain ⟨hi, h0, _⟩ := hp
    exact absurd h0 (h_all i.val hi)

end timeways_rules.instance_lore
