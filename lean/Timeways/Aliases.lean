-- The alias laws (GAMEPLAY.md 5.11): an ID is never reused, a name keeps
-- one ID over any sequence of lines, two names never share an ID, the
-- swap to IDs and back keeps the text, and the text for a model holds no
-- known name.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.aliases

/-! ## The pure model -/

/-- The first place of the key in the table, as `find` walks it. -/
def place : List Alias → String → Option Nat
  | [], _ => none
  | a :: t, k => if a.key = k then some 0 else (place t k).map (· + 1)

theorem place_some_iff (t : List Alias) (k : String) (n : Nat) :
    place t k = some n ↔ ∃ h : n < t.length, t[n].key = k ∧ ∀ j (hj : j < n), t[j].key ≠ k := by
  induction t generalizing n with
  | nil => simp [place]
  | cons a t ih =>
    simp only [place]
    split
    · rename_i ha
      constructor
      · intro h; cases h; exact ⟨by simp, ha, fun j hj => absurd hj (Nat.not_lt_zero _)⟩
      · rintro ⟨_, _, hb⟩
        cases n with
        | zero => rfl
        | succ n => exact absurd ha (hb 0 (Nat.succ_pos _))
    · rename_i ha
      cases n with
      | zero => simp [ha]
      | succ n =>
        simp only [Option.map_eq_some_iff, ih, Nat.add_right_cancel_iff, exists_eq_right,
          List.length_cons, List.getElem_cons_succ]
        constructor
        · rintro ⟨h, hk, hb⟩
          refine ⟨by omega, hk, fun j hj => ?_⟩
          cases j with
          | zero => simpa using ha
          | succ j => simpa using hb j (by omega)
        · rintro ⟨h, hk, hb⟩
          exact ⟨by omega, hk, fun j hj => by simpa using hb (j + 1) (by omega)⟩

theorem place_none_iff (t : List Alias) (k : String) :
    place t k = none ↔ ∀ a ∈ t, a.key ≠ k := by
  induction t with
  | nil => simp [place]
  | cons a t ih =>
    simp only [place]
    split
    · rename_i ha; simp [ha]
    · rename_i ha; simp [ha, ih]

theorem place_append_of_some {t : List Alias} {k : String} {n : Nat} (s : List Alias)
    (h : place t k = some n) : place (t ++ s) k = some n := by
  induction t generalizing n with
  | nil => simp [place] at h
  | cons a t ih =>
    simp only [place, List.cons_append] at h ⊢
    split
    · rename_i ha; simp only [ha, if_true] at h; exact h
    · rename_i ha
      simp only [ha, if_false, Option.map_eq_some_iff] at h
      obtain ⟨m, hm, rfl⟩ := h
      simp [ih hm]

theorem place_lt {t : List Alias} {k : String} {n : Nat} (h : place t k = some n) :
    n < t.length := ((place_some_iff t k n).mp h).1

theorem place_key {t : List Alias} {k : String} {n : Nat} (h : place t k = some n) :
    t[n]'(place_lt h) = t[n]'(place_lt h) ∧ (t[n]'(place_lt h)).key = k :=
  ⟨rfl, ((place_some_iff t k n).mp h).2.1⟩

/-- No key at two places of the table. -/
def Distinct (t : List Alias) : Prop := t.Pairwise (fun a b => a.key ≠ b.key)

/-- The table after one name, as `learn` makes it. -/
def learnP (t : List Alias) (a : Alias) : List Alias :=
  if (place t a.key).isSome then t else t ++ [a]

/-- The table after the names of a line, in order. -/
def learnAllP (t : List Alias) (names : List Alias) : List Alias :=
  names.foldl learnP t

/-- The table after a sequence of lines, each with its names. -/
def learnLines (t : List Alias) (lines : List (List Alias)) : List Alias :=
  lines.foldl learnAllP t

theorem learnP_prefix (t : List Alias) (a : Alias) : t <+: learnP t a := by
  unfold learnP; split
  · exact List.prefix_refl t
  · exact List.prefix_append t [a]

theorem learnAllP_prefix (t : List Alias) (names : List Alias) : t <+: learnAllP t names := by
  induction names generalizing t with
  | nil => exact List.prefix_refl t
  | cons a names ih => exact (learnP_prefix t a).trans (ih _)

theorem learnLines_prefix (t : List Alias) (lines : List (List Alias)) :
    t <+: learnLines t lines := by
  induction lines generalizing t with
  | nil => exact List.prefix_refl t
  | cons l lines ih => exact (learnAllP_prefix t l).trans (ih _)

theorem learnP_distinct (t : List Alias) (a : Alias) (h : Distinct t) : Distinct (learnP t a) := by
  unfold learnP Distinct at *
  split
  · exact h
  · rename_i hn
    rw [List.pairwise_append]
    refine ⟨h, List.pairwise_singleton _ _, fun x hx y hy => ?_⟩
    simp only [List.mem_singleton] at hy
    subst hy
    have hnone : place t y.key = none := by simpa using hn
    exact (place_none_iff t y.key).mp hnone x hx

theorem learnAllP_distinct (t : List Alias) (names : List Alias) (h : Distinct t) :
    Distinct (learnAllP t names) := by
  induction names generalizing t with
  | nil => exact h
  | cons a names ih => exact ih _ (learnP_distinct t a h)

theorem learnP_holds (t : List Alias) (a : Alias) : (place (learnP t a) a.key).isSome := by
  unfold learnP
  split
  · assumption
  · rename_i hn
    have hnone : place t a.key = none := by simpa using hn
    cases hp : place (t ++ [a]) a.key with
    | some _ => rfl
    | none =>
      exact absurd rfl ((place_none_iff _ _).mp hp a (by simp))

theorem place_of_prefix {t t' : List Alias} {k : String} {n : Nat} (hp : t <+: t')
    (h : place t k = some n) : place t' k = some n := by
  obtain ⟨s, rfl⟩ := hp
  exact place_append_of_some s h

theorem learnAllP_holds (t : List Alias) (names : List Alias) :
    ∀ a ∈ names, (place (learnAllP t names) a.key).isSome := by
  induction names generalizing t with
  | nil => simp
  | cons b names ih =>
    intro a ha
    simp only [List.mem_cons] at ha
    rcases ha with rfl | ha
    · obtain ⟨n, hn⟩ := Option.isSome_iff_exists.mp (learnP_holds t a)
      have hp := place_of_prefix (learnAllP_prefix (learnP t a) names) hn
      simp only [learnAllP, List.foldl_cons] at hp ⊢
      rw [hp]
      rfl
    · exact ih _ a ha

/-! ## The specs of the Rust functions -/

@[step]
theorem string_eq.spec (a b : String) :
    alloc.string.String.Insts.CoreCmpPartialEqString.eq a b ⦃ r => (r = true ↔ a = b) ⦄ := by
  simp [alloc.string.String.Insts.CoreCmpPartialEqString.eq]

@[step]
theorem find_loop.spec (t : Slice Alias) (k : String) (i : Usize)
    (h : ∀ j (hj : j < t.val.length), j < i.val → t.val[j].key ≠ k) :
    find_loop t k i ⦃ r => r.map (·.val) = place t.val k ⦄ := by
  unfold find_loop
  dsimp only
  split
  · step as ⟨a, ha⟩
    step as ⟨b, hb⟩
    split
    · simp only [WP.spec_ok, Option.map_some]
      symm
      rw [place_some_iff]
      refine ⟨by scalar_tac, ?_, fun j hj => h j (by scalar_tac) hj⟩
      rw [← ha]; exact hb.mp ‹_›
    · step as ⟨i1, hi1⟩
      apply find_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje
        intro hk
        apply ‹¬ b = true›
        exact hb.mpr (by rw [ha]; exact hk)
      · exact h j hj (by scalar_tac)
  · simp only [WP.spec_ok, Option.map_none]
    symm
    rw [place_none_iff]
    intro a ha
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem ha
    exact h j hj (by scalar_tac)
termination_by t.length - i.val
decreasing_by scalar_decr_tac

/-- `find` gives the first place of the key, or none. -/
@[step]
theorem find.spec (t : Slice Alias) (k : String) :
    find t k ⦃ r => r.map (·.val) = place t.val k ⦄ := by
  unfold find
  exact find_loop.spec t k 0#usize (fun j _ hj => by simp at hj)

@[simp] theorem deref_val {α : Type} (t : alloc.vec.Vec α) :
    (alloc.vec.Vec.deref t).val = t.val := by
  simp [alloc.vec.Vec.deref]

/-- `learn` makes the table of `learnP`, and gives the place of the name in it. -/
@[step]
theorem learn.spec (t : alloc.vec.Vec Alias) (a : Alias) (h : t.length < Usize.max) :
    learn t a ⦃ r => r.2.val = learnP t.val a ∧ place r.2.val a.key = some r.1.val ⦄ := by
  unfold learn
  step as ⟨o, ho⟩
  rcases o with _ | id
  · simp at ho
    step as ⟨t1, ht1⟩
    have hl : learnP t.val a = t.val ++ [a] := by
      unfold learnP; rw [← ho]; rfl
    refine ⟨by rw [ht1, hl], ?_⟩
    rw [ht1, place_some_iff]
    refine ⟨by simp, by simp, fun j hj => ?_⟩
    rw [List.getElem_append_left (by simpa using hj)]
    exact (place_none_iff _ _).mp ho.symm _ (List.getElem_mem _)
  · simp at ho
    simp only [WP.spec_ok]
    have hl : learnP t.val a = t.val := by
      unfold learnP; rw [← ho]; rfl
    exact ⟨hl.symm, ho.symm⟩

theorem learnP_length (t : List Alias) (a : Alias) : (learnP t a).length ≤ t.length + 1 := by
  unfold learnP; split <;> simp

theorem learnAllP_length (t : List Alias) (names : List Alias) :
    (learnAllP t names).length ≤ t.length + names.length := by
  induction names generalizing t with
  | nil => simp [learnAllP]
  | cons a names ih =>
    have := ih (learnP t a)
    have := learnP_length t a
    simp only [learnAllP, List.foldl_cons, List.length_cons] at *
    omega

theorem learnAllP_drop (t : List Alias) (names : List Alias) (i : Nat) (hi : i < names.length) :
    learnAllP (learnP t names[i]) (names.drop (i + 1)) =
      learnAllP t (names.drop i) := by
  rw [List.drop_eq_getElem_cons hi]
  rfl

@[step]
theorem learn_all_loop.spec (t : alloc.vec.Vec Alias) (names : Slice Alias) (i : Usize)
    (hi : i.val ≤ names.length) (h : t.length + (names.length - i.val) < Usize.max) :
    learn_all_loop t names i ⦃ t' => t'.val = learnAllP t.val (names.val.drop i.val) ⦄ := by
  unfold learn_all_loop
  dsimp only
  split
  · step as ⟨a, ha⟩
    simp only [Alias.Insts.CoreCloneClone.clone, alloc.string.String.Insts.CoreCloneClone.clone,
      bind_tc_ok]
    have ha' : (⟨a.key, a.shown⟩ : Alias) = a := rfl
    simp only [ha']
    have hroom : t.length < Usize.max := by scalar_tac
    step as ⟨id, t1, ht1, _⟩
    step as ⟨i1, hi1⟩
    have hl1 : t1.length ≤ t.length + 1 := by
      have := learnP_length t.val a
      simp only [alloc.vec.Vec.length] at this ⊢
      rw [ht1]; exact this
    apply WP.spec_mono (learn_all_loop.spec t1 names i1 (by scalar_tac) (by scalar_tac))
    intro t' ht'
    rw [ht', ht1, hi1, ha]
    exact learnAllP_drop t.val names.val i.val (by scalar_tac)
  · simp only [WP.spec_ok]
    rw [List.drop_of_length_le (by scalar_tac)]
    rfl
termination_by names.length - i.val
decreasing_by scalar_decr_tac

/-- `learn_all` makes the table of `learnAllP`, while the table has room. -/
@[step]
theorem learn_all.spec (t : alloc.vec.Vec Alias) (names : Slice Alias)
    (h : t.length + names.length < Usize.max) :
    learn_all t names ⦃ t' => t'.val = learnAllP t.val names.val ⦄ := by
  unfold learn_all
  apply WP.spec_mono (learn_all_loop.spec t names 0#usize (by simp) (by simpa using h))
  intro t' ht'
  simpa using ht'

/-- Is the piece an ID? -/
def IsPlayer : Piece → Prop
  | .Player _ => True
  | _ => False

/-- The swap of one piece to an ID, as `to_id` does it. -/
def ToId (t : List Alias) (p q : Piece) : Prop :=
  match p with
  | .Word k _ => (place t k = none ∧ q = p) ∨ ∃ id : Usize, place t k = some id.val ∧ q = .Player id
  | _ => q = p

/-- The swap back of one piece, as `to_name` does it. -/
def ToName (t : List Alias) (p q : Piece) : Prop :=
  match p with
  | .Player id => (∃ h : id.val < t.length, q = .Word t[id.val].key t[id.val].shown) ∨
      (t.length ≤ id.val ∧ q = p)
  | _ => q = p

theorem piece_clone (p : Piece) : Piece.Insts.CoreCloneClone.clone p = ok p := by
  cases p <;> simp [Piece.Insts.CoreCloneClone.clone, alloc.string.String.Insts.CoreCloneClone.clone,
    PlayerId.Insts.CoreCloneClone.clone]

@[step]
theorem to_id.spec (t : Slice Alias) (p : Piece) : to_id t p ⦃ q => ToId t.val p q ⦄ := by
  unfold to_id
  cases p with
  | Text s => simp [piece_clone, ToId]
  | Player id => simp [piece_clone, ToId]
  | Word k w =>
    simp only
    step as ⟨o, ho⟩
    rcases o with _ | id
    · simp only [piece_clone, WP.spec_ok, ToId]
      exact Or.inl ⟨by simpa using ho.symm, by simp⟩
    · simp only [WP.spec_ok, ToId]
      exact Or.inr ⟨id, by simpa using ho.symm, by simp⟩

@[step]
theorem to_name.spec (t : Slice Alias) (p : Piece) : to_name t p ⦃ q => ToName t.val p q ⦄ := by
  unfold to_name
  cases p with
  | Text s => simp [piece_clone, ToName]
  | Word k w => simp [piece_clone, ToName]
  | Player id =>
    simp only
    split
    · step as ⟨a, ha⟩
      simp only [alloc.string.String.Insts.CoreCloneClone.clone, bind_tc_ok, WP.spec_ok, ToName]
      exact Or.inl ⟨by scalar_tac, by rw [ha]⟩
    · simp only [piece_clone, WP.spec_ok, ToName]
      exact Or.inr ⟨by scalar_tac, by simp⟩

/-- Each piece of the output is the swap of the piece at its place. -/
def Swapped (R : Piece → Piece → Prop) (ps qs : List Piece) : Prop :=
  qs.length = ps.length ∧ ∀ i (hp : i < ps.length) (hq : i < qs.length), R ps[i] qs[i]

theorem swapped_push {R : Piece → Piece → Prop} {ps qs : List Piece} {i : Nat} {q : Piece}
    (hl : qs.length = i) (hi : i < ps.length)
    (h : ∀ j (hp : j < ps.length) (hq : j < qs.length), R ps[j] qs[j]) (hr : R ps[i] q) :
    (qs ++ [q]).length = i + 1 ∧
      ∀ j (hp : j < ps.length) (hq : j < (qs ++ [q]).length), R ps[j] (qs ++ [q])[j] := by
  refine ⟨by simp [hl], fun j hp hq => ?_⟩
  by_cases hj : j < qs.length
  · rw [List.getElem_append_left hj]; exact h j hp hj
  · have : j = i := by simp at hq; omega
    subst this
    rw [List.getElem_append_right (by omega)]
    simpa [hl] using hr

@[step]
theorem to_ids_loop.spec (t : Slice Alias) (ps : Slice Piece) (qs : alloc.vec.Vec Piece)
    (i : Usize) (hl : qs.length = i.val) (hi : i.val ≤ ps.length)
    (h : ∀ j (hp : j < ps.val.length) (hq : j < qs.val.length), ToId t.val ps.val[j] qs.val[j]) :
    to_ids_loop t ps qs i ⦃ out => Swapped (ToId t.val) ps.val out.val ⦄ := by
  unfold to_ids_loop
  dsimp only
  split
  · have hroom : qs.val.length < Usize.max := by
      simp only [alloc.vec.Vec.length] at hl; scalar_tac
    step as ⟨p, hp⟩
    step as ⟨q, hq⟩
    step as ⟨qs1, hqs1⟩
    step as ⟨i1, hi1⟩
    have hs := swapped_push (R := ToId t.val) (q := q) (by simpa using hl) (by scalar_tac) h
      (by rw [← hp]; exact hq)
    apply to_ids_loop.spec t ps qs1 i1
    · simp only [alloc.vec.Vec.length, hqs1]; scalar_tac
    · scalar_tac
    · rw [hqs1]; exact hs.2
  · simp only [WP.spec_ok]
    refine ⟨by simp only [alloc.vec.Vec.length] at hl; scalar_tac, fun j hp hq => h j hp hq⟩
termination_by ps.length - i.val
decreasing_by scalar_decr_tac

@[step]
theorem to_ids.spec (t : Slice Alias) (ps : Slice Piece) :
    to_ids t ps ⦃ out => Swapped (ToId t.val) ps.val out.val ⦄ := by
  unfold to_ids
  apply to_ids_loop.spec
  · simp [alloc.vec.Vec.new, alloc.vec.Vec.length]
  · simp
  · intro j _ hq; simp [alloc.vec.Vec.new] at hq

@[step]
theorem to_names_loop.spec (t : Slice Alias) (ps : Slice Piece) (qs : alloc.vec.Vec Piece)
    (i : Usize) (hl : qs.length = i.val) (hi : i.val ≤ ps.length)
    (h : ∀ j (hp : j < ps.val.length) (hq : j < qs.val.length), ToName t.val ps.val[j] qs.val[j]) :
    to_names_loop t ps qs i ⦃ out => Swapped (ToName t.val) ps.val out.val ⦄ := by
  unfold to_names_loop
  dsimp only
  split
  · have hroom : qs.val.length < Usize.max := by
      simp only [alloc.vec.Vec.length] at hl; scalar_tac
    step as ⟨p, hp⟩
    step as ⟨q, hq⟩
    step as ⟨qs1, hqs1⟩
    step as ⟨i1, hi1⟩
    have hs := swapped_push (R := ToName t.val) (q := q) (by simpa using hl) (by scalar_tac) h
      (by rw [← hp]; exact hq)
    apply to_names_loop.spec t ps qs1 i1
    · simp only [alloc.vec.Vec.length, hqs1]; scalar_tac
    · scalar_tac
    · rw [hqs1]; exact hs.2
  · simp only [WP.spec_ok]
    refine ⟨by simp only [alloc.vec.Vec.length] at hl; scalar_tac, fun j hp hq => h j hp hq⟩
termination_by ps.length - i.val
decreasing_by scalar_decr_tac

@[step]
theorem to_names.spec (t : Slice Alias) (ps : Slice Piece) :
    to_names t ps ⦃ out => Swapped (ToName t.val) ps.val out.val ⦄ := by
  unfold to_names
  apply to_names_loop.spec
  · simp [alloc.vec.Vec.new, alloc.vec.Vec.length]
  · simp
  · intro j _ hq; simp [alloc.vec.Vec.new] at hq

/-! ## The laws -/

/-- An ID is never reused: after any sequence of lines, each place of the
table holds the player that it held before. A new player only goes at
the end. -/
theorem an_id_is_never_reused (t : List Alias) (lines : List (List Alias)) :
    t <+: learnLines t lines :=
  learnLines_prefix t lines

/-- One name keeps one ID: once the table holds a name at a place, every
later table holds it at the same place, over any sequence of lines. -/
theorem a_name_keeps_its_id (t : List Alias) (lines : List (List Alias)) (k : String) (n : Nat)
    (h : place t k = some n) : place (learnLines t lines) k = some n :=
  place_of_prefix (learnLines_prefix t lines) h

/-- Two different names never share an ID: from an empty table, no
sequence of lines puts one key at two places. -/
theorem two_names_never_share_an_id (lines : List (List Alias)) : Distinct (learnLines [] lines) := by
  suffices ∀ t, Distinct t → Distinct (learnLines t lines) from this [] List.Pairwise.nil
  induction lines with
  | nil => intro t h; exact h
  | cons l lines ih => intro t h; exact ih _ (learnAllP_distinct t l h)

/-- The same, as a fact about IDs: the place that `find` gives for a key
holds that key, so two keys with the same ID are the same key. -/
theorem one_id_names_one_player (t : List Alias) (k1 k2 : String) (n : Nat)
    (h1 : place t k1 = some n) (h2 : place t k2 = some n) : k1 = k2 := by
  rw [← (place_key h1).2, ← (place_key h2).2]

/-- After a line, the table holds each name of the line. -/
theorem every_name_of_a_line_gets_an_id (t : alloc.vec.Vec Alias) (names : Slice Alias)
    (h : t.length + names.length < Usize.max) :
    learn_all t names ⦃ t' => ∀ a ∈ names.val, (place t'.val a.key).isSome ⦄ := by
  apply WP.spec_mono (learn_all.spec t names h)
  intro t' ht'
  rw [ht']
  exact learnAllP_holds t.val names.val

/-- The text for a model holds no name that the table knows: no word of
the swap has a key of the table. -/
theorem no_known_name_after_the_swap (t : Slice Alias) (ps : Slice Piece) :
    to_ids t ps ⦃ out => ∀ p ∈ out.val, ∀ k w, p = .Word k w → ∀ a ∈ t.val, a.key ≠ k ⦄ := by
  apply WP.spec_mono (to_ids.spec t ps)
  intro out ⟨hl, hs⟩ p hp k w hw
  obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hp
  have hr := hs j (by omega) hj
  generalize ps.val[j] = p0 at hr
  generalize out.val[j] = q at hr hw
  subst hw
  unfold ToId at hr
  cases p0 with
  | Word k' w' =>
    simp only at hr
    rcases hr with ⟨hn, he⟩ | ⟨id, _, he⟩
    · cases he
      exact (place_none_iff _ _).mp hn
    · cases he
  | Text s => simp only at hr; cases hr
  | Player id => simp only at hr; cases hr

/-- Each ID of the swap names a player of the table, when the text held
no ID before. So the swap back always finds the name. -/
theorem every_id_of_the_swap_is_in_the_table (t : Slice Alias) (ps : Slice Piece)
    (h : ∀ p ∈ ps.val, ¬ IsPlayer p) :
    to_ids t ps ⦃ out => ∀ p ∈ out.val, ∀ id, p = .Player id → id.val < t.length ⦄ := by
  apply WP.spec_mono (to_ids.spec t ps)
  intro out ⟨hl, hs⟩ p hp id hid
  obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hp
  have hj' : j < ps.val.length := by omega
  have hr := hs j hj' hj
  have hnp := h _ (List.getElem_mem hj')
  generalize ps.val[j] = p0 at hr hnp
  generalize out.val[j] = q at hr hid
  subst hid
  unfold ToId at hr
  cases p0 with
  | Word k' w' =>
    simp only at hr
    rcases hr with ⟨_, he⟩ | ⟨id', hpl, he⟩
    · cases he
    · cases he
      exact place_lt hpl
  | Text s => simp only at hr; cases hr
  | Player id' => exact absurd trivial hnp

/-- What the swap to IDs and back gives for one piece: a known name in
the form that the table holds, and every other piece as it was. -/
def restore (t : List Alias) (p : Piece) : Piece :=
  match p with
  | .Word k w =>
    match h : place t k with
    | some n => .Word k (t[n]'(place_lt h)).shown
    | none => .Word k w
  | _ => p

theorem restore_none {t : List Alias} {k w : String} (h : place t k = none) :
    restore t (.Word k w) = .Word k w := by
  dsimp only [restore]
  split
  · rename_i n hs; rw [hs] at h; cases h
  · rfl

theorem restore_some {t : List Alias} {k w : String} {n : Nat} (h : place t k = some n) :
    restore t (.Word k w) = .Word k (t[n]'(place_lt h)).shown := by
  dsimp only [restore]
  split
  · rename_i m hs
    rw [hs] at h
    cases h
    rfl
  · rename_i hs; rw [hs] at h; cases h

/-- The swap to IDs and back gives the text back, piece by piece, when it
held no ID: a known name comes back in the form of the table, with its
key, and every other piece comes back as it was. A full round trip is
not true: "ADA-Stormrage" comes back as "Ada", because the ID keeps who
the player is, not how the text wrote the name. -/
theorem the_swap_and_back_keeps_the_text (t : Slice Alias) (ps : Slice Piece)
    (h : ∀ p ∈ ps.val, ¬ IsPlayer p) :
    (do let ids ← to_ids t ps; to_names t (alloc.vec.Vec.deref ids)) ⦃ out =>
      out.val = ps.val.map (restore t.val) ⦄ := by
  apply WP.spec_bind (to_ids.spec t ps)
  intro ids ⟨hl1, hs1⟩
  apply WP.spec_mono (to_names.spec t (alloc.vec.Vec.deref ids))
  intro out ⟨hl2, hs2⟩
  have e : (alloc.vec.Vec.deref ids).val = ids.val := deref_val ids
  rw [e] at hl2 hs2
  apply List.ext_getElem (by simp [hl2, hl1])
  intro j hj1 hj2
  have hj : j < ps.val.length := by simpa using hj2
  have r1 := hs1 j hj (by omega)
  have r2 := hs2 j (by omega) hj1
  rw [List.getElem_map]
  have hnp := h _ (List.getElem_mem hj)
  generalize ps.val[j] = p at r1 hnp
  generalize ids.val[j] = q at r1 r2
  unfold ToId at r1
  cases p with
  | Text s =>
    simp only at r1; subst r1
    simp only [ToName] at r2; rw [r2]; rfl
  | Player id => exact absurd trivial hnp
  | Word k w =>
    simp only at r1
    rcases r1 with ⟨hn, rfl⟩ | ⟨id, hpl, rfl⟩
    · simp only [ToName] at r2
      rw [r2, restore_none hn]
    · simp only [ToName] at r2
      rcases r2 with ⟨hlt, he⟩ | ⟨hge, _⟩
      · rw [he, restore_some hpl, (place_key hpl).2]
      · exact absurd (place_lt hpl) (by omega)

/-- A piece that is no known name comes back exactly. -/
theorem restore_keeps_what_is_no_known_name (t : List Alias) (p : Piece)
    (h : ∀ k w, p = .Word k w → place t k = none) : restore t p = p := by
  cases p with
  | Word k w => exact restore_none (h k w rfl)
  | _ => rfl

end timeways_rules.aliases
