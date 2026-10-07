-- The laws of game names (docs/plans/lore-names-and-now.md 1): a row says that two exact
-- names are one person, a name of no row stays as it is, and with rows that are a
-- function, the lookup is idempotent. The gates of outcome and setup passages read
-- names through it (Outcomes.lean and Setups.lean).
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.game_names

/-- The pure model of `person`: the wiki name of the first row with this game name, or
the name itself. -/
def personM : List NameRow → U32 → U32
  | [], id => id
  | r :: rest, id => if r.game = id then r.wiki else personM rest id

/-- The rows are a function: no game name sits in two rows, and no game name is the
wiki name of a row. A unit test checks this on the data file
(`the_game_names_are_a_function`). -/
def Functional (rows : List NameRow) : Prop :=
  (∀ a ∈ rows, ∀ b ∈ rows, a.game = b.game → a = b) ∧
    (∀ a ∈ rows, ∀ b ∈ rows, a.game ≠ b.wiki)

@[step]
theorem person_loop.spec (rows : Slice NameRow) (id : U32) (i : Usize) :
    person_loop rows id i ⦃ r => r = personM (rows.val.drop i.val) id ⦄ := by
  unfold person_loop
  dsimp only
  split
  · rename_i hi
    step as ⟨nr, hnr⟩
    have hdrop : rows.val.drop i.val = nr :: rows.val.drop (i.val + 1) := by
      rw [hnr]
      exact List.drop_eq_getElem_cons (by scalar_tac)
    split
    · rename_i he
      simp [hdrop, personM, he]
    · rename_i he
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (person_loop.spec rows id i1)
      intro r hr
      rw [hr, hdrop, hi1]
      simp [personM, he]
  · rename_i hi
    have hdrop : rows.val.drop i.val = [] := List.drop_eq_nil_of_le (by scalar_tac)
    simp [hdrop, personM]
termination_by rows.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The lookup, exactly: the pure model over every row. -/
@[step]
theorem person.spec (rows : Slice NameRow) (id : U32) :
    person rows id ⦃ r => r = personM rows.val id ⦄ := by
  unfold person
  simpa using person_loop.spec rows id 0#usize

@[step]
theorem holds_person_loop.spec (rows : Slice NameRow) (ids : Slice U32) (wanted : U32)
    (i : Usize) :
    holds_person_loop rows ids wanted i ⦃ b =>
      (b = true ↔ ∃ x ∈ ids.val.drop i.val, personM rows.val x = wanted) ⦄ := by
  unfold holds_person_loop
  dsimp only
  split
  · rename_i hi
    step as ⟨x, hx⟩
    have hdrop : ids.val.drop i.val = x :: ids.val.drop (i.val + 1) := by
      rw [hx]
      exact List.drop_eq_getElem_cons (by scalar_tac)
    step as ⟨p, hp⟩
    split
    · rename_i he
      simp only [WP.spec_ok, true_iff]
      exact ⟨x, by rw [hdrop]; exact List.mem_cons_self .., by rw [← hp, he]⟩
    · rename_i he
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (holds_person_loop.spec rows ids wanted i1)
      intro b hb
      rw [hb, hdrop, hi1]
      constructor
      · rintro ⟨y, hy, hyw⟩
        exact ⟨y, List.mem_cons_of_mem _ hy, hyw⟩
      · rintro ⟨y, hy, hyw⟩
        rcases List.mem_cons.mp hy with h | h
        · subst h
          exact absurd (hp.trans hyw) he
        · exact ⟨y, h, hyw⟩
  · rename_i hi
    have hdrop : ids.val.drop i.val = [] := List.drop_eq_nil_of_le (by scalar_tac)
    simp [hdrop]
termination_by ids.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The check, exactly: a name of `ids` names the same person as `id`. -/
@[step]
theorem holds_person.spec (rows : Slice NameRow) (ids : Slice U32) (id : U32) :
    holds_person rows ids id ⦃ b =>
      (b = true ↔ ∃ x ∈ ids.val, personM rows.val x = personM rows.val id) ⦄ := by
  unfold holds_person
  step as ⟨wanted, hw⟩
  apply WP.spec_mono (holds_person_loop.spec rows ids wanted 0#usize)
  intro b hb
  simpa [hw] using hb

theorem personM_of_no_row (rows : List NameRow) (id : U32)
    (h : ∀ r ∈ rows, r.game ≠ id) : personM rows id = id := by
  induction rows with
  | nil => rfl
  | cons r rest ih =>
    simp only [personM, h r List.mem_cons_self, if_false]
    exact ih (fun s hs => h s (List.mem_cons_of_mem _ hs))

theorem personM_of_row (rows : List NameRow) (r : NameRow) (hf : Functional rows)
    (hr : r ∈ rows) : personM rows r.game = r.wiki := by
  induction rows with
  | nil => simp at hr
  | cons s rest ih =>
    have hf' : Functional rest :=
      ⟨fun a ha b hb => hf.1 a (List.mem_cons_of_mem _ ha) b (List.mem_cons_of_mem _ hb),
       fun a ha b hb => hf.2 a (List.mem_cons_of_mem _ ha) b (List.mem_cons_of_mem _ hb)⟩
    simp only [personM]
    by_cases hg : s.game = r.game
    · rw [if_pos hg, hf.1 s List.mem_cons_self r hr hg]
    · rw [if_neg hg]
      rcases List.mem_cons.mp hr with h | h
      · exact absurd (h ▸ rfl) hg
      · exact ih hf' h

/-- The wiki name of a row names its own person. -/
theorem personM_of_wiki (rows : List NameRow) (r : NameRow) (hf : Functional rows)
    (hr : r ∈ rows) : personM rows r.wiki = r.wiki :=
  personM_of_no_row rows r.wiki (fun s hs => hf.2 s hs r hr)

/-- A name and its wiki name name one person, with rows that are a function. -/
theorem person_is_idempotent (rows : List NameRow) (id : U32) (hf : Functional rows) :
    personM rows (personM rows id) = personM rows id := by
  by_cases h : ∃ r ∈ rows, r.game = id
  · obtain ⟨r, hr, hg⟩ := h
    rw [← hg, personM_of_row rows r hf hr, personM_of_wiki rows r hf hr]
  · push Not at h
    rw [personM_of_no_row rows id h, personM_of_no_row rows id h]

/-- Either name of a row gives its wiki name. -/
theorem either_name_of_a_row_names_one_person (rows : List NameRow) (r : NameRow)
    (hf : Functional rows) (hr : r ∈ rows) :
    personM rows r.game = personM rows r.wiki := by
  rw [personM_of_row rows r hf hr, personM_of_wiki rows r hf hr]

end timeways_rules.game_names
