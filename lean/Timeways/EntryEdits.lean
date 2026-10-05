-- The laws of the player edits (docs/plans/chapters.md 11): the newest row
-- decides what an entry shows, and a new text of the narrator never hides the
-- words of the player.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.entry_edits

/-! ## The pure model -/

/-- One turn of the loop of `newest_row`: a row as large as the best takes its place. -/
def pickRow : Option U64 → U64 → Option U64
  | none, r => some r
  | some b, r => if b > r then some b else some r

/-- One turn of the loop of `newest_edit`: an edit with a row as large as the best takes
its place. So of two edits with the same row, the later one in the list wins. -/
def pickEdit : Option EditRow → EditRow → Option EditRow
  | none, e => some e
  | some b, e => if b.row > e.row then some b else some e

def newestRowM (rows : List U64) : Option U64 := rows.foldl pickRow none

def newestEditM (edits : List EditRow) : Option EditRow := edits.foldl pickEdit none

/-- What an entry shows, from its newest narrator row and its newest edit. -/
def shownOf (narrator : Option U64) : Option EditRow → Shown
  | none => { title := none, narrator, player := none }
  | some e =>
    let title := if e.has_title then some e.row else none
    match e.text with
    | .Keep => { title, narrator, player := some e.row }
    | .Replace => { title, narrator := none, player := some e.row }
    | .Narrator => { title, narrator, player := none }

def shownM (rows : List U64) (edits : List EditRow) : Shown :=
  shownOf (newestRowM rows) (newestEditM edits)

/-- Ties: the later edit with the same row wins. -/
theorem pickEdit_tie (b e : EditRow) (h : b.row = e.row) : pickEdit (some b) e = some e := by
  simp [pickEdit, h]

theorem foldl_pickRow_mem (rows : List U64) (start : Option U64) (b : U64)
    (h : rows.foldl pickRow start = some b) : start = some b ∨ b ∈ rows := by
  induction rows generalizing start with
  | nil => simp at h; exact Or.inl h
  | cons r rs ih =>
    rcases ih _ h with h1 | h1
    · cases start with
      | none => simp [pickRow] at h1; subst h1; simp
      | some s =>
        simp only [pickRow] at h1
        split at h1
        · exact Or.inl h1
        · simp at h1; subst h1; simp
    · exact Or.inr (List.mem_cons_of_mem _ h1)

theorem newestRowM_mem (rows : List U64) (b : U64) (h : newestRowM rows = some b) :
    b ∈ rows := by
  rcases foldl_pickRow_mem rows none b h with h1 | h1
  · cases h1
  · exact h1

theorem newestRowM_append (rows : List U64) (n : U64) :
    newestRowM (rows ++ [n]) = pickRow (newestRowM rows) n := by
  simp [newestRowM, List.foldl_append]

theorem foldl_pickRow_some (rows : List U64) (b : U64) :
    ∃ c, rows.foldl pickRow (some b) = some c := by
  induction rows generalizing b with
  | nil => exact ⟨b, rfl⟩
  | cons r rs ih =>
    simp only [List.foldl_cons, pickRow]
    split
    · exact ih b
    · exact ih r

theorem foldl_pickEdit_some (edits : List EditRow) (b : EditRow) :
    ∃ c, edits.foldl pickEdit (some b) = some c := by
  induction edits generalizing b with
  | nil => exact ⟨b, rfl⟩
  | cons r rs ih =>
    simp only [List.foldl_cons, pickEdit]
    split
    · exact ih b
    · exact ih r

/-- The newest row of the one-row list of the newest row is that row. -/
theorem newestRowM_toList (rows : List U64) :
    newestRowM (newestRowM rows).toList = newestRowM rows := by
  cases newestRowM rows <;> simp [newestRowM, pickRow]

theorem newestEditM_toList (edits : List EditRow) :
    newestEditM (newestEditM edits).toList = newestEditM edits := by
  cases newestEditM edits <;> simp [newestEditM, pickEdit]

/-! ## The specs of the Rust functions -/

@[step]
theorem newest_row_loop.spec (rows : Slice U64) (newest : Option U64) (i : Usize)
    (hi : i.val ≤ rows.length) :
    newest_row_loop rows newest i ⦃ r => r = (rows.val.drop i.val).foldl pickRow newest ⦄ := by
  unfold newest_row_loop
  dsimp only
  split
  · step as ⟨row, hrow⟩
    have hlt : i.val < rows.val.length := by scalar_tac
    have hdrop : rows.val.drop i.val = row :: rows.val.drop (i.val + 1) := by
      rw [hrow]; exact List.drop_eq_getElem_cons hlt
    cases newest with
    | none =>
      simp only [bind_tc_ok, Bool.false_eq_true, ↓reduceIte]
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (newest_row_loop.spec rows (some row) i1 (by scalar_tac))
      intro r hr
      rw [hr, hdrop, hi1]
      simp [pickRow]
    | some best =>
      simp only [bind_tc_ok]
      by_cases hgt : best > row
      · simp only [hgt, decide_true, ↓reduceIte]
        step as ⟨i1, hi1⟩
        apply WP.spec_mono (newest_row_loop.spec rows (some best) i1 (by scalar_tac))
        intro r hr
        rw [hr, hdrop, hi1]
        simp [pickRow, hgt]
      · simp only [hgt, decide_false, Bool.false_eq_true, ↓reduceIte]
        step as ⟨i1, hi1⟩
        apply WP.spec_mono (newest_row_loop.spec rows (some row) i1 (by scalar_tac))
        intro r hr
        rw [hr, hdrop, hi1]
        simp [pickRow, hgt]
  · simp only [WP.spec_ok]
    rw [List.drop_of_length_le (by scalar_tac)]
    rfl
termination_by rows.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- `newest_row` never panics, always ends, and gives `newestRowM`. -/
@[step]
theorem newest_row.spec (rows : Slice U64) :
    newest_row rows ⦃ r => r = newestRowM rows.val ⦄ := by
  unfold newest_row
  apply WP.spec_mono (newest_row_loop.spec rows none 0#usize (by simp))
  intro r hr
  simpa [newestRowM] using hr

/-- One turn of the loop of `newest_edit`, as `pickEdit`. -/
theorem newest_edit_loop_unfold (edits : Slice EditRow) (newest : Option EditRow) (i : Usize)
    (h : i.val < edits.val.length) :
    newest_edit_loop edits newest i = (do
      let i1 ← i + 1#usize
      newest_edit_loop edits (pickEdit newest edits.val[i.val]) i1) := by
  rw [newest_edit_loop]
  have hl : i < Slice.len edits := by scalar_tac
  have hget : (edits.val)[i.val]? = some edits.val[i.val] := List.getElem?_eq_getElem h
  simp only [hl, ↓reduceIte, Slice.index_usize]
  split
  · rename_i hn
    simp_all
  rename_i x hx
  have hxe : x = edits.val[i.val] := by simp_all
  subst hxe
  simp only [bind_tc_ok]
  cases newest with
  | none => simp [pickEdit]; rfl
  | some b =>
    by_cases hg : b.row > (edits.val[i.val]).row
    · have : (edits.val[i.val]).row.val < b.row.val := hg
      simp [pickEdit, hg]; split <;> first | rfl | (exfalso; omega)
    · have : ¬ (edits.val[i.val]).row.val < b.row.val := hg
      simp [pickEdit, hg]; split <;> first | rfl | (exfalso; omega)

@[step]
theorem newest_edit_loop.spec (edits : Slice EditRow) (newest : Option EditRow) (i : Usize)
    (hi : i.val ≤ edits.length) :
    newest_edit_loop edits newest i ⦃ r => r = (edits.val.drop i.val).foldl pickEdit newest ⦄ := by
  by_cases hlt : i.val < edits.val.length
  · rw [newest_edit_loop_unfold edits newest i hlt]
    step as ⟨i1, hi1⟩
    apply WP.spec_mono
      (newest_edit_loop.spec edits (pickEdit newest edits.val[i.val]) i1 (by scalar_tac))
    intro r hr
    rw [hr, hi1, List.drop_eq_getElem_cons hlt]
    rfl
  · unfold newest_edit_loop
    dsimp only
    split
    · exfalso; scalar_tac
    · simp only [WP.spec_ok]
      rw [List.drop_of_length_le (by scalar_tac)]
      rfl
termination_by edits.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- `newest_edit` never panics, always ends, and gives `newestEditM`. -/
@[step]
theorem newest_edit.spec (edits : Slice EditRow) :
    newest_edit edits ⦃ r => r = newestEditM edits.val ⦄ := by
  unfold newest_edit
  apply WP.spec_mono (newest_edit_loop.spec edits none 0#usize (by simp))
  intro r hr
  simpa [newestEditM] using hr

/-- `shown` never panics, always ends, and gives `shownM`. -/
@[step]
theorem shown.spec (rows : Slice U64) (edits : Slice EditRow) :
    shown rows edits ⦃ s => s = shownM rows.val edits.val ⦄ := by
  unfold shown
  step as ⟨n, hn⟩
  step as ⟨o, ho⟩
  unfold shownM
  rw [← hn, ← ho]
  cases o with
  | none => simp [shownOf]
  | some e =>
    dsimp only
    cases ht : e.has_title <;> cases hx : e.text <;> simp [shownOf, ht, hx]

theorem shown_eq (rows : Slice U64) (edits : Slice EditRow) :
    shown rows edits = ok (shownM rows.val edits.val) := by
  obtain ⟨s, hs, he⟩ := WP.spec_imp_exists (shown.spec rows edits)
  rw [hs, he]

/-! ## The laws -/

/-- Theorem 20. `shown` of the whole lists equals `shown` of only the newest narrator row
and the newest edit row. -/
theorem the_newest_edit_decides (rows newestRows : Slice U64)
    (edits newestEdits : Slice EditRow)
    (hr : newestRows.val = (newestRowM rows.val).toList)
    (he : newestEdits.val = (newestEditM edits.val).toList) :
    shown rows edits = shown newestRows newestEdits := by
  rw [shown_eq, shown_eq, hr, he]
  simp [shownM, newestRowM_toList, newestEditM_toList]

/-- Theorem 20, after a restore: when the newest edit is the narrator's text with no title,
the entry shows the newest narrator row and nothing of the player. -/
theorem a_restore_shows_the_newest_narrator_text (rows : Slice U64) (edits : Slice EditRow)
    (e : EditRow) (hnew : newestEditM edits.val = some e) (htext : e.text = .Narrator)
    (htitle : e.has_title = false) :
    shown rows edits ⦃ s => s = { title := none, narrator := newestRowM rows.val, player := none } ⦄ := by
  apply WP.spec_mono (shown.spec rows edits)
  intro s hs
  simp [hs, shownM, hnew, shownOf, htext, htitle]

/-- Theorem 20, also for a narrator row that came after the edit: once restored, a new
narrator row larger than every other narrator row is the one that shows. -/
theorem a_restore_shows_a_later_narrator_text (rows : Slice U64) (edits : Slice EditRow)
    (n : U64) (hn : ∀ r ∈ rows.val, r < n)
    (e : EditRow) (hnew : newestEditM edits.val = some e) (htext : e.text = .Narrator) :
    shown rows edits ⦃ s => s.player = none ∧
      ∀ later : Slice U64, later.val = rows.val ++ [n] →
        shown later edits ⦃ s1 => s1.narrator = some n ∧ s1.player = none ⦄ ⦄ := by
  apply WP.spec_mono (shown.spec rows edits)
  intro s hs
  refine ⟨by simp [hs, shownM, hnew, shownOf, htext], ?_⟩
  intro later hl
  apply WP.spec_mono (shown.spec later edits)
  intro s1 hs1
  have hrow : newestRowM later.val = some n := by
    rw [hl, newestRowM_append]
    cases hb : newestRowM rows.val with
    | none => simp [pickRow]
    | some b =>
      have := hn b (newestRowM_mem _ _ hb)
      have : ¬ b > n := by scalar_tac
      simp [pickRow, this]
  simp [hs1, shownM, hnew, shownOf, htext, hrow]

/-- Theorem 21. A new narrator row never changes the shown title, never changes the shown
player row, and changes nothing at all under `Replace`. It holds for any appended row,
so also for a row newer than every other narrator row. -/
theorem a_model_text_never_hides_player_words (rows later : Slice U64)
    (edits : Slice EditRow) (n : U64) (_hl : later.val = rows.val ++ [n]) :
    shown rows edits ⦃ s => shown later edits ⦃ s1 =>
      s1.title = s.title ∧ s1.player = s.player ∧
      (∀ e, newestEditM edits.val = some e → e.text = .Replace → s1 = s) ⦄ ⦄ := by
  apply WP.spec_mono (shown.spec rows edits)
  intro s hs
  apply WP.spec_mono (shown.spec later edits)
  intro s1 hs1
  subst hs hs1
  unfold shownM
  cases he : newestEditM edits.val with
  | none => simp [shownOf]
  | some e =>
    refine ⟨?_, ?_, ?_⟩
    · cases hx : e.text <;> simp [shownOf, hx]
    · cases hx : e.text <;> simp [shownOf, hx]
    · intro e' he' hx
      simp at he'
      subst he'
      simp [shownOf, hx]

end timeways_rules.entry_edits
