-- The laws of the shapes of a narrator line (docs/plans/narrator-templates.md 5.2):
-- the lore comes first, the hero stands only in a deed or a coda and at most once,
-- every slot has a value, and the pick fits and never repeats a main part within the
-- window.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.narrator_shapes

deriving instance DecidableEq for Token
deriving instance DecidableEq for PartKind

/-! ## The pure predicates -/

/-- The token has a value in the facts, as `has_value` decides. -/
def HasVal (f : Facts) : Token → Prop
  | .Word _ => True
  | .Lore => True
  | .Hero => f.named = true
  | .Slot k => ∃ h : k.val < f.has.val.length, f.has.val[k.val] = true

/-- Each token of `rest` comes from a part of the shape that is in the table. -/
def FromParts (t : Table) (s : Shape) (rest : List Token) : Prop :=
  ∀ x ∈ rest, ∃ id ∈ s.parts.val, ∃ p, t.parts.val[id.val]? = some p ∧ x ∈ p.tokens.val

/-- What `part_ok` checks: no lore, and the hero only in a deed or a coda. -/
def PartOk (p : Part) : Prop :=
  Token.Lore ∉ p.tokens.val ∧
    (Token.Hero ∈ p.tokens.val → p.kind = .Deed ∨ p.kind = .Coda)

/-- No word of `inside` stands right before a hero slot. -/
def NotInside (inside : List U16) (line : List Token) : Prop :=
  ∀ i, line[i + 1]? = some Token.Hero → ∀ w, line[i]? = some (Token.Word w) → w ∉ inside

/-! ## Small functions -/

@[simp]
theorem deref_val {α : Type} (v : alloc.vec.Vec α) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref, Slice.from_val]

@[step]
theorem holds_loop.spec (ids : Slice U16) (n : U16) (i : Usize)
    (h : ∀ j (hj : j < ids.val.length), j < i.val → ids.val[j] ≠ n) :
    holds_loop ids n i ⦃ b => (b = true ↔ n ∈ ids.val) ⦄ := by
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
termination_by ids.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem holds.spec (ids : Slice U16) (n : U16) :
    holds ids n ⦃ b => (b = true ↔ n ∈ ids.val) ⦄ := by
  unfold holds
  exact holds_loop.spec ids n 0#usize (fun j _ hj => by simp at hj)

@[step]
theorem with_tokens_loop.spec (line : alloc.vec.Vec Token) (tokens : Slice Token) (i : Usize)
    (hi : i.val ≤ tokens.val.length)
    (h : line.val.length + (tokens.val.length - i.val) ≤ Usize.max) :
    with_tokens_loop line tokens i ⦃ r => r.val = line.val ++ tokens.val.drop i.val ⦄ := by
  unfold with_tokens_loop
  dsimp only
  split
  · step as ⟨x, hx⟩
    step as ⟨line1, hl1⟩
    step as ⟨i1, hi1⟩
    apply WP.spec_mono (with_tokens_loop.spec line1 tokens i1 (by scalar_tac) (by
      simp only [hl1, List.length_append, List.length_singleton]; scalar_tac))
    intro r hr
    rw [hr, hl1, hx, List.append_assoc]
    congr 1
    rw [hi1]
    simp only [List.singleton_append]
    exact (List.drop_eq_getElem_cons (by scalar_tac)).symm
  · simp only [WP.spec_ok]
    have : tokens.val.length ≤ i.val := by scalar_tac
    simp [List.drop_eq_nil_of_le this]
termination_by tokens.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem with_tokens.spec (line : alloc.vec.Vec Token) (tokens : Slice Token)
    (h : line.val.length + tokens.val.length ≤ Usize.max) :
    with_tokens line tokens ⦃ r => r.val = line.val ++ tokens.val ⦄ := by
  unfold with_tokens
  apply WP.spec_mono (with_tokens_loop.spec line tokens 0#usize (by simp) (by simpa using h))
  intro r hr
  simpa using hr

theorem ok_some_inj {α : Type} {a b : α}
    (h : (ok (some a) : Result (Option α)) = ok (some b)) : a = b := by
  simpa using h

theorem most_tokens_val : MOST_TOKENS.val = 1024 := by simp [MOST_TOKENS]

/-! ## The skeleton -/

/-- A skeleton is the lore, then tokens of the parts of the shape, and it holds at most
`MOST_TOKENS` tokens. -/
def SkelOk (t : Table) (s : Shape) (line : List Token) : Prop :=
  (∃ rest, line = Token.Lore :: rest ∧ FromParts t s rest) ∧ line.length ≤ 1024

@[step]
theorem skeleton_loop.spec (t : Table) (s : Shape) (line : alloc.vec.Vec Token) (i : Usize)
    (hl : SkelOk t s line.val) :
    skeleton_loop t s line i ⦃ r => ∀ o, r = some o → SkelOk t s o.val ⦄ := by
  unfold skeleton_loop
  dsimp only
  have hm := most_tokens_val
  obtain ⟨⟨rest, hrest, hfrom⟩, hlen⟩ := hl
  split
  · step as ⟨id, hid⟩
    split
    · simp
    · rename_i hin
      step as ⟨p, hp⟩
      step as ⟨i4, hi4⟩
      split
      · simp
      · rename_i hfit
        have hd := deref_val p.tokens
        have h1 : p.tokens.len.val = p.tokens.val.length := by simp
        have h2 : line.len.val = line.val.length := by simp
        have hroom : line.val.length + (alloc.vec.Vec.deref p.tokens).val.length ≤ Usize.max := by
          rw [hd]; scalar_tac
        apply WP.spec_bind (with_tokens.spec line (alloc.vec.Vec.deref p.tokens) hroom)
        intro line1 hl1
        rw [hd] at hl1
        step as ⟨i1, hi1⟩
        apply skeleton_loop.spec
        refine ⟨⟨rest ++ p.tokens.val, by rw [hl1, hrest]; simp, ?_⟩, ?_⟩
        · intro x hx
          rcases List.mem_append.mp hx with hx | hx
          · exact hfrom x hx
          · refine ⟨id, ?_, p, ?_, hx⟩
            · rw [hid]; exact List.getElem_mem _
            · rw [hp]; simp
        · rw [hl1]; simp only [List.length_append]; scalar_tac
  · simp only [WP.spec_ok]
    intro o ho
    cases ho
    exact ⟨⟨rest, hrest, hfrom⟩, hlen⟩
termination_by s.parts.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem skeleton.spec (t : Table) (s : Shape) :
    skeleton t s ⦃ r => ∀ o, r = some o → SkelOk t s o.val ⦄ := by
  unfold skeleton
  step as ⟨line, hl⟩
  apply skeleton_loop.spec
  refine ⟨⟨[], by simp [hl], by simp [FromParts]⟩, by simp [hl]⟩

/-! ## Values, heroes, and the fit -/

@[step]
theorem has_value.spec (token : Token) (f : Facts) :
    has_value token f ⦃ b => (b = true ↔ HasVal f token) ⦄ := by
  unfold has_value
  cases token with
  | Word w => simp [HasVal]
  | Lore => simp [HasVal]
  | Hero => simp [HasVal]
  | Slot k =>
    dsimp only
    step as ⟨a, ha⟩
    have hak : a.val = k.val := by
      rw [ha]; exact UScalar.cast_val_mod_pow_of_inBounds_eq _ _ (by scalar_tac)
    split
    · rename_i hlt
      step as ⟨x, hx⟩
      simp only [HasVal]
      constructor
      · intro hxt
        refine ⟨by scalar_tac, ?_⟩
        rw [← hxt, hx]
        simp [hak]
      · rintro ⟨_, hk⟩
        rw [hx]
        simpa [hak] using hk
    · rename_i hlt
      simp only [WP.spec_ok, HasVal, Bool.false_eq_true, false_iff]
      rintro ⟨hk, _⟩
      apply hlt
      scalar_tac

@[step]
theorem every_value_is_known_loop.spec (line : Slice Token) (f : Facts) (i : Usize)
    (h : ∀ j (hj : j < line.val.length), j < i.val → HasVal f line.val[j]) :
    every_value_is_known_loop line f i ⦃ b => (b = true ↔ ∀ x ∈ line.val, HasVal f x) ⦄ := by
  unfold every_value_is_known_loop
  dsimp only
  split
  · step as ⟨x, hx⟩
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      step as ⟨i1, hi1⟩
      apply every_value_is_known_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje; rw [← hx]; exact hb.mp hbt
      · exact h j hj (by scalar_tac)
    · rename_i hbt
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro hall
      apply hbt
      exact hb.mpr (by rw [hx]; exact hall _ (List.getElem_mem _))
  · simp only [WP.spec_ok, true_iff]
    intro x hxm
    obtain ⟨j, hj, hjeq⟩ := List.getElem_of_mem hxm
    rw [← hjeq]
    exact h j hj (by scalar_tac)
termination_by line.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem every_value_is_known.spec (line : Slice Token) (f : Facts) :
    every_value_is_known line f ⦃ b => (b = true ↔ ∀ x ∈ line.val, HasVal f x) ⦄ := by
  unfold every_value_is_known
  exact every_value_is_known_loop.spec line f 0#usize (fun j _ hj => by simp at hj)

@[step]
theorem is_hero.spec (token : Token) : is_hero token ⦃ b => (b = true ↔ token = .Hero) ⦄ := by
  unfold is_hero
  cases token <;> simp

@[step]
theorem hero_count_loop.spec (line : Slice Token) (c i : Usize) (hi : i.val ≤ line.val.length)
    (hc : c.val = (line.val.take i.val).count Token.Hero) :
    hero_count_loop line c i ⦃ n => n.val = line.val.count Token.Hero ⦄ := by
  unfold hero_count_loop
  dsimp only
  have hcount : ∀ l : List Token, l.count Token.Hero ≤ l.length := fun l => List.count_le_length
  split
  · step as ⟨x, hx⟩
    step as ⟨b, hb⟩
    have htake : line.val.take (i.val + 1) = line.val.take i.val ++ [x] := by
      rw [hx]; exact List.take_succ_eq_append_getElem (by scalar_tac)
    have hci : c.val ≤ i.val := by
      rw [hc]
      exact le_trans (hcount _) (by simp)
    split
    · rename_i hbt
      step as ⟨c1, hc1⟩
      step as ⟨i1, hi1⟩
      apply hero_count_loop.spec
      · scalar_tac
      · rw [hc1, hi1, htake, List.count_append, ← hc, hb.mp hbt]; simp
    · rename_i hbt
      step as ⟨i1, hi1⟩
      apply hero_count_loop.spec
      · scalar_tac
      · rw [hi1, htake, List.count_append, ← hc]
        have : x ≠ Token.Hero := fun he => hbt (hb.mpr he)
        simp [this]
  · simp only [WP.spec_ok]
    rw [hc, List.take_of_length_le (by scalar_tac)]
termination_by line.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem hero_count.spec (line : Slice Token) :
    hero_count line ⦃ n => n.val = line.val.count Token.Hero ⦄ := by
  unfold hero_count
  exact hero_count_loop.spec line 0#usize 0#usize (by simp) (by simp)

@[step]
theorem named_right.spec (named : Bool) (heroes : Usize) :
    named_right named heroes ⦃ b => (b = true ↔ heroes.val = if named then 1 else 0) ⦄ := by
  unfold named_right
  cases named <;> simp [UScalar.eq_equiv]

@[step]
theorem needs_and_tags_hold_loop.spec (t : Table) (s : Shape) (f : Facts) (seen : U32)
    (i : Usize) :
    needs_and_tags_hold_loop t s f seen i ⦃ _ => True ⦄ := by
  unfold needs_and_tags_hold_loop
  dsimp only
  split
  · step as ⟨id, hid⟩
    split
    · simp
    · step as ⟨p, hp⟩
      simp only [lift, bind_tc_ok]
      split
      · simp
      · split
        · simp
        · step as ⟨i1, hi1⟩
          exact needs_and_tags_hold_loop.spec t s f _ i1
  · simp
termination_by s.parts.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem needs_and_tags_hold.spec (t : Table) (s : Shape) (f : Facts) :
    needs_and_tags_hold t s f ⦃ _ => True ⦄ := by
  unfold needs_and_tags_hold
  exact needs_and_tags_hold_loop.spec t s f 0#u32 0#usize

/-- `assemble` never panics. A line that it builds is a skeleton of the shape, and each
token of it has a value. -/
@[step]
theorem assemble.spec (t : Table) (s : Shape) (f : Facts) :
    assemble t s f ⦃ r => ∀ o, r = some o →
      SkelOk t s o.val ∧ skeleton t s = ok (some o) ∧ ∀ x ∈ o.val, HasVal f x ⦄ := by
  unfold assemble
  have hsk := skeleton.spec t s
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists hsk
  rw [hr]
  simp only [bind_tc_ok]
  cases r with
  | none => simp
  | some line =>
    dsimp only
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      simp only [WP.spec_ok, Option.some.injEq]
      rintro o rfl
      exact ⟨hpost line rfl, rfl, by simpa using hb.mp hbt⟩
    · simp

/-- `fits` never panics. A fitting shape builds a line that names the hero once on a named
turn and never on another turn. -/
@[step]
theorem fits.spec (t : Table) (s : Shape) (f : Facts) :
    fits t s f ⦃ b => b = true → ∃ o, skeleton t s = ok (some o) ∧ SkelOk t s o.val ∧
      (∀ x ∈ o.val, HasVal f x) ∧
      o.val.count Token.Hero = (if f.named then 1 else 0) ⦄ := by
  unfold fits
  have hsk := skeleton.spec t s
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists hsk
  rw [hr]
  simp only [bind_tc_ok]
  cases r with
  | none => simp
  | some line =>
    dsimp only
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      step as ⟨b1⟩
      split
      · step as ⟨n, hn⟩
        step as ⟨b2, hb2, hb2t⟩
        refine ⟨line, rfl, hpost line rfl, by simpa using hb.mp hbt, ?_⟩
        rw [← (hb2.mp hb2t)]
        simpa using hn.symm
      · simp
    · simp

@[step]
theorem assemble_arrival.spec : assemble_arrival ⦃ o => o.val = [Token.Lore] ⦄ := by
  unfold assemble_arrival
  step*

/-! ## The checks of the table -/

@[step]
theorem part_ok_loop.spec (v : alloc.vec.Vec Token) (may : Bool) (i : Usize) :
    part_ok_loop v may i ⦃ b => b = true → ∀ j (hj : j < v.val.length), i.val ≤ j →
      v.val[j] ≠ Token.Lore ∧ (may = false → v.val[j] ≠ Token.Hero) ⦄ := by
  unfold part_ok_loop
  dsimp only
  split
  · step as ⟨x, hx⟩
    have hcase : (∀ j (hj : j < v.val.length), i.val + 1 ≤ j →
        v.val[j] ≠ Token.Lore ∧ (may = false → v.val[j] ≠ Token.Hero)) →
        (x ≠ Token.Lore ∧ (may = false → x ≠ Token.Hero)) →
        (∀ j (hj : j < v.val.length), i.val ≤ j →
          v.val[j] ≠ Token.Lore ∧ (may = false → v.val[j] ≠ Token.Hero)) := by
      intro hrest hx' j hj hij
      by_cases hje : j = i.val
      · subst hje; rw [← hx]; exact hx'
      · exact hrest j hj (by omega)
    cases x with
    | Word w =>
      dsimp only
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (part_ok_loop.spec v may i1)
      intro b hb hbt
      exact hcase (fun j hj hij => hb hbt j hj (by scalar_tac)) (by simp)
    | Lore => simp
    | Hero =>
      dsimp only
      split
      · rename_i hmay
        step as ⟨i1, hi1⟩
        apply WP.spec_mono (part_ok_loop.spec v true i1)
        intro b hb hbt
        subst hmay
        exact hcase (fun j hj hij => hb hbt j hj (by scalar_tac)) (by simp)
      · simp
    | Slot k =>
      dsimp only
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (part_ok_loop.spec v may i1)
      intro b hb hbt
      exact hcase (fun j hj hij => hb hbt j hj (by scalar_tac)) (by simp)
  · simp only [WP.spec_ok]
    intro _ j hj hij
    exfalso
    scalar_tac
termination_by v.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem part_ok.spec (p : Part) : part_ok p ⦃ b => b = true → PartOk p ⦄ := by
  unfold part_ok
  have hgo : ∀ may : Bool, (may = false ↔ ¬ (p.kind = .Deed ∨ p.kind = .Coda)) →
      part_ok_loop p.tokens may 0#usize ⦃ b => b = true → PartOk p ⦄ := by
    intro may hmay
    apply WP.spec_mono (part_ok_loop.spec p.tokens may 0#usize)
    intro b hb hbt
    have hall := hb hbt
    refine ⟨?_, ?_⟩
    · intro hm
      obtain ⟨j, hj, he⟩ := List.getElem_of_mem hm
      exact (hall j hj (by simp)).1 he
    · intro hm
      by_contra hk
      obtain ⟨j, hj, he⟩ := List.getElem_of_mem hm
      exact (hall j hj (by simp)).2 (hmay.mpr hk) he
  cases hk : p.kind <;> simp only [bind_tc_ok] <;> apply hgo <;> simp [hk]

@[step]
theorem parts_ok_loop.spec (t : Table) (i : Usize)
    (h : ∀ j (hj : j < t.parts.val.length), j < i.val → PartOk t.parts.val[j]) :
    parts_ok_loop t i ⦃ b => b = true → ∀ p ∈ t.parts.val, PartOk p ⦄ := by
  unfold parts_ok_loop
  dsimp only
  split
  · step as ⟨p, hp⟩
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      step as ⟨i1, hi1⟩
      apply parts_ok_loop.spec
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje; rw [← hp]; exact hb hbt
      · exact h j hj (by scalar_tac)
    · simp
  · simp only [WP.spec_ok]
    intro _ p hpm
    obtain ⟨j, hj, he⟩ := List.getElem_of_mem hpm
    rw [← he]
    exact h j hj (by scalar_tac)
termination_by t.parts.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem parts_ok.spec (t : Table) :
    parts_ok t ⦃ b => b = true → ∀ p ∈ t.parts.val, PartOk p ⦄ := by
  unfold parts_ok
  exact parts_ok_loop.spec t 0#usize (fun j _ hj => by simp at hj)

@[step]
theorem is_inside_word.spec (token : Token) (ins : Slice U16) :
    is_inside_word token ins ⦃ b => (b = true ↔ ∃ w, token = .Word w ∧ w ∈ ins.val) ⦄ := by
  unfold is_inside_word
  cases token with
  | Word w =>
    dsimp only
    apply WP.spec_mono (holds.spec ins w)
    intro b hb
    simpa using hb
  | Lore => simp
  | Hero => simp
  | Slot k => simp

@[step]
theorem inside_the_hero_loop.spec (line : Slice Token) (ins : Slice U16) (i : Usize)
    (hi : 1 ≤ i.val) :
    inside_the_hero_loop line ins i ⦃ b => b = false → ∀ j, i.val ≤ j + 1 →
      line.val[j + 1]? = some Token.Hero → ∀ w, line.val[j]? = some (Token.Word w) →
      w ∉ ins.val ⦄ := by
  unfold inside_the_hero_loop
  dsimp only
  split
  · rename_i hlt
    step as ⟨i1, hi1⟩
    step as ⟨x, hx⟩
    step as ⟨b, hb⟩
    have hcase : (∀ j, i.val + 1 ≤ j + 1 → line.val[j + 1]? = some Token.Hero →
          ∀ w, line.val[j]? = some (Token.Word w) → w ∉ ins.val) →
        (line.val[i.val]? = some Token.Hero → ∀ w, x = Token.Word w → w ∉ ins.val) →
        ∀ j, i.val ≤ j + 1 → line.val[j + 1]? = some Token.Hero →
          ∀ w, line.val[j]? = some (Token.Word w) → w ∉ ins.val := by
      intro hrest hhere j hij hh w hw
      by_cases hje : j + 1 = i.val
      · have hj : j = i1.val := by scalar_tac
        rw [hje] at hh
        have hxj : line.val[j]? = some x := by
          rw [hj, hx]; exact List.getElem?_eq_getElem (by scalar_tac)
        rw [hxj] at hw
        exact hhere hh w (Option.some.inj hw)
      · exact hrest j (by omega) hh w hw
    split
    · rename_i hbt
      step as ⟨y, hy⟩
      step as ⟨c, hc⟩
      split
      · simp
      · rename_i hct
        step as ⟨i2, hi2⟩
        apply WP.spec_mono (inside_the_hero_loop.spec line ins i2 (by scalar_tac))
        intro r hr hrf
        apply hcase (fun j hj => hr hrf j (by scalar_tac))
        intro hh w _
        exfalso
        apply hct
        rw [hc, hy]
        have := List.getElem?_eq_getElem (l := line.val) (i := i.val) (by scalar_tac)
        rw [this] at hh
        exact Option.some.inj hh
    · rename_i hbt
      step as ⟨i2, hi2⟩
      apply WP.spec_mono (inside_the_hero_loop.spec line ins i2 (by scalar_tac))
      intro r hr hrf
      apply hcase (fun j hj => hr hrf j (by scalar_tac))
      intro _ w hxw hw
      exact hbt (hb.mpr ⟨w, hxw, hw⟩)
  · rename_i hge
    simp only [WP.spec_ok]
    intro _ j hij hh
    have : line.val.length ≤ j + 1 := by scalar_tac
    simp [List.getElem?_eq_none this] at hh
termination_by line.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem inside_the_hero.spec (line : Slice Token) (ins : Slice U16) :
    inside_the_hero line ins ⦃ b => b = false → NotInside ins.val line.val ⦄ := by
  unfold inside_the_hero
  apply WP.spec_mono (inside_the_hero_loop.spec line ins 1#usize (by simp))
  intro b hb hbf i hh w hw
  exact hb hbf i (by simp) hh w hw

@[step]
theorem shapes_ok_loop.spec (t : Table) (ss : Slice Shape) (i : Usize) :
    shapes_ok_loop t ss i ⦃ b => b = true → ∀ j (hj : j < ss.val.length), i.val ≤ j →
      ∃ o, skeleton t ss.val[j] = ok (some o) ∧ NotInside t.inside_words.val o.val ⦄ := by
  unfold shapes_ok_loop
  dsimp only
  split
  · step as ⟨x, hx⟩
    obtain ⟨r, hr, _⟩ := WP.spec_imp_exists (skeleton.spec t x)
    rw [hr]
    simp only [bind_tc_ok]
    cases r with
    | none => simp
    | some line =>
      dsimp only
      step as ⟨b, hb⟩
      split
      · simp
      · rename_i hbt
        step as ⟨i1, hi1⟩
        apply WP.spec_mono (shapes_ok_loop.spec t ss i1)
        intro c hc hct j hj hij
        by_cases hje : j = i.val
        · subst hje
          refine ⟨line, by rw [← hx]; exact hr, ?_⟩
          have := hb (by simpa using hbt)
          simpa using this
        · exact hc hct j hj (by scalar_tac)
  · simp only [WP.spec_ok]
    intro _ j hj hij
    exfalso; scalar_tac
termination_by ss.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem shapes_ok.spec (t : Table) (ss : Slice Shape) :
    shapes_ok t ss ⦃ b => b = true → ∀ j (hj : j < ss.val.length),
      ∃ o, skeleton t ss.val[j] = ok (some o) ∧ NotInside t.inside_words.val o.val ⦄ := by
  unfold shapes_ok
  apply WP.spec_mono (shapes_ok_loop.spec t ss 0#usize)
  intro b hb hbt j hj
  exact hb hbt j hj (by simp)

@[step]
theorem same_token.spec (a b : Token) : same_token a b ⦃ r => (r = true ↔ a = b) ⦄ := by
  unfold same_token
  cases a <;> cases b <;> simp

@[step]
theorem same_line_loop.spec (a b : Slice Token) (i : Usize)
    (hlen : a.val.length = b.val.length)
    (h : ∀ j (hj : j < a.val.length), j < i.val → a.val[j] = b.val[j]'(by omega)) :
    same_line_loop a b i ⦃ r => (r = true ↔ a.val = b.val) ⦄ := by
  unfold same_line_loop
  dsimp only
  split
  · step as ⟨x, hx⟩
    step as ⟨y, hy⟩
    step as ⟨c, hc⟩
    split
    · rename_i hct
      step as ⟨i1, hi1⟩
      apply same_line_loop.spec a b i1 hlen
      intro j hj hji
      by_cases hje : j = i.val
      · subst hje; rw [← hx, ← hy]; exact hc.mp hct
      · exact h j hj (by scalar_tac)
    · rename_i hct
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro heq
      apply hct
      apply hc.mpr
      rw [hx, hy]
      simp [heq]
  · simp only [WP.spec_ok, true_iff]
    apply List.ext_getElem hlen
    intro j hj _
    exact h j hj (by scalar_tac)
termination_by a.length - i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem same_line.spec (a b : Slice Token) :
    same_line a b ⦃ r => (r = true ↔ a.val = b.val) ⦄ := by
  unfold same_line
  dsimp only
  split
  · rename_i hne
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
    intro heq
    simp [Slice.len, heq] at hne
  · rename_i heq
    apply same_line_loop.spec a b 0#usize (by simpa [Slice.len] using heq)
    intro j _ hj
    simp at hj

@[step]
theorem differs_from_later_loop.spec (t : Table) (ss : Slice Shape)
    (line : alloc.vec.Vec Token) (later : Usize) :
    differs_from_later_loop t ss line later ⦃ b => b = true →
      ∀ j (hj : j < ss.val.length), later.val ≤ j →
      ∀ o, skeleton t ss.val[j] = ok (some o) → line.val ≠ o.val ⦄ := by
  unfold differs_from_later_loop
  dsimp only
  split
  · step as ⟨x, hx⟩
    obtain ⟨r, hr, _⟩ := WP.spec_imp_exists (skeleton.spec t x)
    rw [hr]
    simp only [bind_tc_ok]
    cases r with
    | none => simp
    | some other =>
      dsimp only
      step as ⟨c, hc⟩
      split
      · simp
      · rename_i hct
        step as ⟨l1, hl1⟩
        apply WP.spec_mono (differs_from_later_loop.spec t ss line l1)
        intro b hb hbt j hj hlj o ho
        by_cases hje : j = later.val
        · subst hje
          rw [← hx, hr] at ho
          obtain rfl := ok_some_inj ho
          intro heq
          exact hct (hc.mpr (by simpa using heq))
        · exact hb hbt j hj (by scalar_tac) o ho
  · simp only [WP.spec_ok]
    intro _ j hj hlj
    exfalso; scalar_tac
termination_by ss.length - later.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem differs_from_later.spec (t : Table) (ss : Slice Shape) (first : Usize)
    (hf : first.val < ss.val.length) :
    differs_from_later t ss first ⦃ b => b = true →
      ∀ j (hj : j < ss.val.length), first.val < j →
      ∀ o o', skeleton t ss.val[first.val] = ok (some o) →
        skeleton t ss.val[j] = ok (some o') → o.val ≠ o'.val ⦄ := by
  unfold differs_from_later
  step as ⟨x, hx⟩
  obtain ⟨r, hr, _⟩ := WP.spec_imp_exists (skeleton.spec t x)
  rw [hr]
  simp only [bind_tc_ok]
  cases r with
  | none => simp
  | some line =>
    dsimp only
    step as ⟨l1, hl1⟩
    apply WP.spec_mono (differs_from_later_loop.spec t ss line l1)
    intro b hb hbt j hj hfj o o' ho ho'
    rw [← hx, hr] at ho
    obtain rfl := ok_some_inj ho
    exact hb hbt j hj (by scalar_tac) o' ho'

@[step]
theorem distinct_skeletons_loop.spec (t : Table) (ss : Slice Shape) (first : Usize) :
    distinct_skeletons_loop t ss first ⦃ b => b = true →
      ∀ i j (hi : i < ss.val.length) (hj : j < ss.val.length), first.val ≤ i → i < j →
      ∀ o o', skeleton t ss.val[i] = ok (some o) →
        skeleton t ss.val[j] = ok (some o') → o.val ≠ o'.val ⦄ := by
  unfold distinct_skeletons_loop
  dsimp only
  split
  · rename_i hlt
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      step as ⟨f1, hf1⟩
      apply WP.spec_mono (distinct_skeletons_loop.spec t ss f1)
      intro c hc hct i j hi hj hfi hij o o' ho ho'
      by_cases hie : i = first.val
      · subst hie
        exact hb hbt j hj hij o o' ho ho'
      · exact hc hct i j hi hj (by scalar_tac) hij o o' ho ho'
    · simp
  · simp only [WP.spec_ok]
    intro _ i j hi hj hfi
    exfalso; scalar_tac
termination_by ss.length - first.val
decreasing_by all_goals scalar_decr_tac

/-- `distinct_skeletons` never panics. When it holds, two shapes never build one line. -/
@[step]
theorem distinct_skeletons.spec (t : Table) (ss : Slice Shape) :
    distinct_skeletons t ss ⦃ b => b = true →
      ∀ i j (hi : i < ss.val.length) (hj : j < ss.val.length), i < j →
      ∀ o o', skeleton t ss.val[i] = ok (some o) →
        skeleton t ss.val[j] = ok (some o') → o.val ≠ o'.val ⦄ := by
  unfold distinct_skeletons
  apply WP.spec_mono (distinct_skeletons_loop.spec t ss 0#usize)
  intro b hb hbt i j hi hj hij
  exact hb hbt i j hi hj (by simp) hij

/-- What the checks of the table give. -/
def TableOk (t : Table) (ss : Slice Shape) : Prop :=
  (∀ p ∈ t.parts.val, PartOk p) ∧
  (∀ j (hj : j < ss.val.length),
    ∃ o, skeleton t ss.val[j] = ok (some o) ∧ NotInside t.inside_words.val o.val) ∧
  (∀ i j (hi : i < ss.val.length) (hj : j < ss.val.length), i < j →
    ∀ o o', skeleton t ss.val[i] = ok (some o) →
      skeleton t ss.val[j] = ok (some o') → o.val ≠ o'.val)

/-- `table_ok` never panics, and it holds only when each check holds. -/
@[step]
theorem table_ok.spec (t : Table) (ss : Slice Shape) :
    table_ok t ss ⦃ b => b = true → TableOk t ss ⦄ := by
  unfold table_ok
  step as ⟨b, hb⟩
  split
  · rename_i hbt
    step as ⟨c, hc⟩
    split
    · rename_i hct
      apply WP.spec_mono (distinct_skeletons.spec t ss)
      intro d hd hdt
      exact ⟨hb hbt, hc hct, hd hdt⟩
    · simp
  · simp

/-! ## The laws of a built line -/

theorem assembled {t : Table} {s : Shape} {f : Facts} {o : alloc.vec.Vec Token}
    (h : assemble t s f = ok (some o)) :
    SkelOk t s o.val ∧ skeleton t s = ok (some o) ∧ ∀ x ∈ o.val, HasVal f x := by
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists (assemble.spec t s f)
  rw [h] at hr
  have : r = some o := by simpa using hr.symm
  exact hpost o this

theorem part_of_index {t : Table} {id : Usize} {p : Part}
    (h : t.parts.val[id.val]? = some p) : p ∈ t.parts.val := by
  obtain ⟨hlt, he⟩ := List.getElem?_eq_some_iff.mp h
  rw [← he]
  exact List.getElem_mem _

theorem index_of_mem {ss : Slice Shape} {s : Shape} (h : s ∈ ss.val) :
    ∃ j, ∃ hj : j < ss.val.length, ss.val[j] = s := by
  obtain ⟨j, hj, he⟩ := List.getElem_of_mem h
  exact ⟨j, hj, he⟩

/-- Law 2: with distinct skeletons, two shapes that build the same tokens are the same
shape. So each built line is an instance of exactly one template. -/
theorem a_line_is_one_shape (t : Table) (ss : Slice Shape) (i j : Nat)
    (hi : i < ss.val.length) (hj : j < ss.val.length) (f f' : Facts)
    (o : alloc.vec.Vec Token)
    (hd : distinct_skeletons t ss = ok true)
    (hai : assemble t ss.val[i] f = ok (some o))
    (haj : assemble t ss.val[j] f' = ok (some o)) :
    i = j := by
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists (distinct_skeletons.spec t ss)
  rw [hd] at hr
  have hr' : r = true := by simpa using hr.symm
  have hall := hpost hr'
  have hsi := (assembled hai).2.1
  have hsj := (assembled haj).2.1
  rcases Nat.lt_trichotomy i j with h | h | h
  · exact absurd rfl (hall i j hi hj h o o hsi hsj)
  · exact h
  · exact absurd rfl (hall j i hj hi h o o hsj hsi)

theorem table_holds {t : Table} {ss : Slice Shape} (h : table_ok t ss = ok true) :
    TableOk t ss := by
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists (table_ok.spec t ss)
  rw [h] at hr
  exact hpost (by simpa using hr.symm)

/-- Law 3: every built line starts with the lore slot, and holds it once. -/
theorem the_lore_comes_first (t : Table) (ss : Slice Shape) (s : Shape) (f : Facts)
    (o : alloc.vec.Vec Token) (hok : table_ok t ss = ok true)
    (h : assemble t s f = ok (some o)) :
    o.val.head? = some Token.Lore ∧ o.val.count Token.Lore = 1 := by
  obtain ⟨hparts, _, _⟩ := table_holds hok
  obtain ⟨⟨rest, hrest, hfrom⟩, _⟩ := (assembled h).1
  rw [hrest]
  refine ⟨rfl, ?_⟩
  have hnot : Token.Lore ∉ rest := by
    intro hm
    obtain ⟨id, _, p, hp, hx⟩ := hfrom _ hm
    exact (hparts p (part_of_index hp)).1 hx
  simp [List.count_eq_zero_of_not_mem hnot]

/-- Law 4: each hero slot of a built line comes from a deed part or a coda part of its
shape. -/
theorem the_hero_stands_only_in_a_deed (t : Table) (ss : Slice Shape) (s : Shape)
    (f : Facts) (o : alloc.vec.Vec Token) (hok : table_ok t ss = ok true)
    (h : assemble t s f = ok (some o)) (hh : Token.Hero ∈ o.val) :
    ∃ id ∈ s.parts.val, ∃ p, t.parts.val[id.val]? = some p ∧ Token.Hero ∈ p.tokens.val ∧
      (p.kind = .Deed ∨ p.kind = .Coda) := by
  obtain ⟨hparts, _, _⟩ := table_holds hok
  obtain ⟨⟨rest, hrest, hfrom⟩, _⟩ := (assembled h).1
  rw [hrest] at hh
  rcases List.mem_cons.mp hh with he | hm
  · cases he
  · obtain ⟨id, hid, p, hp, hx⟩ := hfrom _ hm
    exact ⟨id, hid, p, hp, hx, (hparts p (part_of_index hp)).2 hx⟩

/-- Law 5: the line of an arrival is the lore slot alone. -/
theorem an_arrival_holds_no_hero : assemble_arrival ⦃ o => o.val = [Token.Lore] ⦄ :=
  assemble_arrival.spec

/-- Law 6: no group part and no grow part holds a hero slot. -/
theorem no_group_clause_holds_the_hero (t : Table) (ss : Slice Shape)
    (hok : table_ok t ss = ok true) :
    ∀ p ∈ t.parts.val, (p.kind = .Group ∨ p.kind = .Grow) → Token.Hero ∉ p.tokens.val := by
  obtain ⟨hparts, _, _⟩ := table_holds hok
  intro p hp hk hh
  rcases (hparts p hp).2 hh with hd | hd <;> rcases hk with hk | hk <;> rw [hk] at hd <;>
    cases hd

/-- Law 7: no word of `inside_words` ("in", "inside", "within", "through", "into") stands
right before a hero slot. -/
theorem nothing_is_inside_the_hero (t : Table) (ss : Slice Shape) (s : Shape) (f : Facts)
    (o : alloc.vec.Vec Token) (hok : table_ok t ss = ok true) (hs : s ∈ ss.val)
    (h : assemble t s f = ok (some o)) :
    ∀ i, o.val[i + 1]? = some Token.Hero →
      ∀ w, o.val[i]? = some (Token.Word w) → w ∉ t.inside_words.val := by
  obtain ⟨_, hshapes, _⟩ := table_holds hok
  obtain ⟨j, hj, rfl⟩ := index_of_mem hs
  obtain ⟨o', ho', hnot⟩ := hshapes j hj
  rw [(assembled h).2.1] at ho'
  obtain rfl := ok_some_inj ho'
  exact hnot

/-- Law 8: a fitting shape names the hero once on a named turn, and never on another
turn. -/
theorem the_hero_is_named_at_most_once (t : Table) (s : Shape) (f : Facts)
    (o : alloc.vec.Vec Token) (hf : fits t s f = ok true)
    (h : assemble t s f = ok (some o)) :
    o.val.count Token.Hero = (if f.named then 1 else 0) := by
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists (fits.spec t s f)
  rw [hf] at hr
  obtain ⟨o', ho', _, _, hcount⟩ := hpost (by simpa using hr.symm)
  rw [(assembled h).2.1] at ho'
  obtain rfl := ok_some_inj ho'
  exact hcount

/-- A shape that fits builds a line: the pick never takes a shape with no line. -/
theorem a_fitting_shape_builds_a_line (t : Table) (s : Shape) (f : Facts)
    (hf : fits t s f = ok true) : ∃ o, assemble t s f = ok (some o) := by
  obtain ⟨r, hr, hpost⟩ := WP.spec_imp_exists (fits.spec t s f)
  rw [hf] at hr
  obtain ⟨o, ho, _, hvals, _⟩ := hpost (by simpa using hr.symm)
  refine ⟨o, ?_⟩
  unfold assemble
  rw [ho]
  simp only [bind_tc_ok]
  obtain ⟨b, hb, hbpost⟩ :=
    WP.spec_imp_exists (every_value_is_known.spec (alloc.vec.Vec.deref o) f)
  rw [hb]
  simp only [bind_tc_ok]
  have : b = true := hbpost.mpr (by simpa using hvals)
  simp [this]

/-- Law 9: each slot of a built line has a value in the facts. So no "{foe}" ever shows. -/
theorem every_slot_has_a_value (t : Table) (s : Shape) (f : Facts) (o : alloc.vec.Vec Token)
    (h : assemble t s f = ok (some o)) :
    (∀ k, Token.Slot k ∈ o.val → ∃ hk : k.val < f.has.val.length, f.has.val[k.val] = true) ∧
      (Token.Hero ∈ o.val → f.named = true) := by
  have hvals := (assembled h).2.2
  exact ⟨fun k hk => hvals _ hk, fun hh => hvals _ hh⟩

/-! ## The pick -/

/-- Shape `i` fits the moment. -/
def Fit (fits : Slice Bool) (i : Nat) : Prop := fits.val[i]? = some true

/-- The main part of shape `i` is not in `recent`. -/
def Fresh (mains recent : Slice U16) (i : Nat) : Prop :=
  ∃ m, mains.val[i]? = some m ∧ m ∉ recent.val

/-- Shape `i` fits, and its main part is not recent. -/
def FitFresh (fits : Slice Bool) (mains recent : Slice U16) (i : Nat) : Prop :=
  Fit fits i ∧ Fresh mains recent i

theorem around_reaches (start i n : Nat) (hs : start < n) (hi : i < n) :
    ∃ k, k < n ∧ (start + k) % n = i := by
  by_cases h : start ≤ i
  · refine ⟨i - start, by omega, ?_⟩
    rw [show start + (i - start) = i by omega, Nat.mod_eq_of_lt hi]
  · refine ⟨i + n - start, by omega, ?_⟩
    rw [show start + (i + n - start) = i + n by omega, Nat.add_mod_right, Nat.mod_eq_of_lt hi]

@[step]
theorem around.spec (start step count : Usize) (hs : start.val < count.val)
    (hst : step.val < count.val) :
    around start step count ⦃ r => r.val = (start.val + step.val) % count.val ∧
      r.val < count.val ⦄ := by
  unfold around
  step as ⟨d, hd⟩
  split
  · step as ⟨r, hr⟩
    refine ⟨?_, by scalar_tac⟩
    rw [hr, Nat.mod_eq_of_lt (by scalar_tac)]
  · step as ⟨r, hr⟩
    have h1 : start.val + step.val = r.val + count.val := by scalar_tac
    refine ⟨?_, by scalar_tac⟩
    rw [h1, Nat.add_mod_right, Nat.mod_eq_of_lt (by scalar_tac)]

@[step]
theorem first_fresh_loop.spec (fits : Slice Bool) (mains recent : Slice U16)
    (start step : Usize) (hn : mains.val.length = fits.val.length)
    (hs : start.val < fits.val.length)
    (hpre : ∀ k < step.val, ¬ FitFresh fits mains recent ((start.val + k) % fits.val.length)) :
    first_fresh_loop fits mains recent start step ⦃ r =>
      (r = none → ∀ k < fits.val.length,
        ¬ FitFresh fits mains recent ((start.val + k) % fits.val.length)) ∧
      (∀ i, r = some i → ∃ k < fits.val.length,
        i.val = (start.val + k) % fits.val.length ∧ FitFresh fits mains recent i.val ∧
        ∀ k' < k, ¬ FitFresh fits mains recent ((start.val + k') % fits.val.length)) ⦄ := by
  unfold first_fresh_loop
  dsimp only
  split
  · rename_i hlt
    step as ⟨idx, hidx, hidxlt⟩
    have hlen : fits.len.val = fits.val.length := by simp
    rw [hlen] at hidx hidxlt
    have hidx' : (start.val + step.val) % fits.val.length = idx.val := hidx.symm
    clear hidx
    step as ⟨b, hb⟩
    have hfit : Fit fits idx.val ↔ b = true := by
      simp only [Fit, hb, List.getElem?_eq_getElem hidxlt]
      simp
    split
    · rename_i hbt
      step as ⟨m, hm⟩
      step as ⟨c, hc⟩
      have hmi : mains.val[idx.val]? = some m := by
        rw [hm]; exact List.getElem?_eq_getElem (by scalar_tac)
      split
      · rename_i hct
        step as ⟨s1, hs1⟩
        apply first_fresh_loop.spec fits mains recent start s1 hn hs
        intro k hk
        by_cases hke : k = step.val
        · subst hke
          rw [hidx']
          rintro ⟨_, m', hm', hnot⟩
          rw [hmi] at hm'
          cases hm'
          exact hnot (hc.mp hct)
        · exact hpre k (by scalar_tac)
      · rename_i hct
        simp only [WP.spec_ok, reduceCtorEq, false_implies, Option.some.injEq, forall_eq',
          true_and]
        refine ⟨step.val, by scalar_tac, hidx'.symm, ⟨hfit.mpr hbt, m, hmi, ?_⟩, hpre⟩
        intro hmem
        exact hct (hc.mpr hmem)
    · rename_i hbt
      step as ⟨s1, hs1⟩
      apply first_fresh_loop.spec fits mains recent start s1 hn hs
      intro k hk
      by_cases hke : k = step.val
      · subst hke
        rw [hidx']
        rintro ⟨hf, _⟩
        exact hbt (hfit.mp hf)
      · exact hpre k (by scalar_tac)
  · simp only [WP.spec_ok, reduceCtorEq, false_implies, forall_const, and_true]
    intro k hk hff
    exact hpre k (by scalar_tac) hff
termination_by fits.length - step.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem last_use_loop.spec (recent : Slice U16) (main : U16) (i : Usize)
    (hi : i.val ≤ recent.val.length) :
    last_use_loop recent main i ⦃ _ => True ⦄ := by
  unfold last_use_loop
  split
  · step as ⟨i1, hi1⟩
    step as ⟨x, hx⟩
    split
    · simp
    · exact last_use_loop.spec recent main i1 (by scalar_tac)
  · simp
termination_by i.val
decreasing_by all_goals scalar_decr_tac

@[step]
theorem last_use.spec (recent : Slice U16) (main : U16) :
    last_use recent main ⦃ _ => True ⦄ := by
  unfold last_use
  exact last_use_loop.spec recent main _ (by simp)

@[step]
theorem longest_unused_loop.spec (fits : Slice Bool) (mains recent : Slice U16)
    (start : Usize) (best : Option Usize) (bestUse step : Usize)
    (hn : mains.val.length = fits.val.length) (hs : start.val < fits.val.length)
    (hbest : ∀ b, best = some b → Fit fits b.val)
    (hnone : best = none → ∀ k < step.val, k < fits.val.length →
      ¬ Fit fits ((start.val + k) % fits.val.length)) :
    longest_unused_loop fits mains recent start best bestUse step ⦃ r =>
      (r = none → ∀ k < fits.val.length, ¬ Fit fits ((start.val + k) % fits.val.length)) ∧
      (∀ i, r = some i → Fit fits i.val) ⦄ := by
  unfold longest_unused_loop
  dsimp only
  split
  · rename_i hlt
    step as ⟨idx, hidx, hidxlt⟩
    have hlen : fits.len.val = fits.val.length := by simp
    rw [hlen] at hidx hidxlt
    have hidx' : (start.val + step.val) % fits.val.length = idx.val := hidx.symm
    clear hidx
    step as ⟨b, hb⟩
    have hfit : Fit fits idx.val ↔ b = true := by
      simp only [Fit, hb, List.getElem?_eq_getElem hidxlt]
      simp
    have hext : best = none → ¬ Fit fits idx.val → ∀ k < step.val + 1, k < fits.val.length →
        ¬ Fit fits ((start.val + k) % fits.val.length) := by
      intro hbn hnf k hk hkn
      by_cases hke : k = step.val
      · subst hke
        rw [hidx']
        exact hnf
      · exact hnone hbn k (by omega) hkn
    split
    · rename_i hbt
      step as ⟨m, hm⟩
      step as ⟨used⟩
      cases best with
      | none =>
        simp only [bind_tc_ok, if_true]
        step as ⟨s1, hs1⟩
        apply longest_unused_loop.spec fits mains recent start (some idx) used s1 hn hs
        · rintro b' hb'
          cases hb'
          exact hfit.mpr hbt
        · simp
      | some bb =>
        simp only [bind_tc_ok]
        split
        · step as ⟨s1, hs1⟩
          apply longest_unused_loop.spec fits mains recent start (some idx) used s1 hn hs
          · rintro b' hb'
            cases hb'
            exact hfit.mpr hbt
          · simp
        · step as ⟨s1, hs1⟩
          apply longest_unused_loop.spec fits mains recent start (some bb) bestUse s1 hn hs
            hbest
          simp
    · rename_i hbt
      step as ⟨s1, hs1⟩
      apply longest_unused_loop.spec fits mains recent start best bestUse s1 hn hs hbest
      intro hbn k hk hkn
      exact hext hbn (fun hf => hbt (hfit.mp hf)) k (by scalar_tac) hkn
  · rename_i hge
    simp only [WP.spec_ok]
    refine ⟨?_, hbest⟩
    intro hbn k hk
    exact hnone hbn k (by scalar_tac) hk
termination_by fits.length - step.val
decreasing_by all_goals scalar_decr_tac

/-! ## The laws of the pick -/

theorem fit_lt {fits : Slice Bool} {i : Nat} (h : Fit fits i) : i < fits.val.length := by
  unfold Fit at h
  exact (List.getElem?_eq_some_iff.mp h).1

/-- What the pick gives: none only when no shape fits, a fitting shape always, and the
first fitting fresh shape from `turn mod count` when one exists. -/
def PickPost (fits : Slice Bool) (mains recent : Slice U16) (turn : U64)
    (r : Option Usize) : Prop :=
  (r = none → ∀ i, ¬ Fit fits i) ∧
  (∀ i, r = some i → Fit fits i.val) ∧
  ((∃ i, FitFresh fits mains recent i) → ∃ i, r = some i ∧
    ∃ k < fits.val.length,
      i.val = (turn.val % fits.val.length + k) % fits.val.length ∧
      FitFresh fits mains recent i.val ∧
      ∀ k' < k, ¬ FitFresh fits mains recent
        ((turn.val % fits.val.length + k') % fits.val.length))

/-- `pick` never panics, and gives `PickPost`. -/
@[step]
theorem pick.spec (fits : Slice Bool) (mains recent : Slice U16) (turn : U64)
    (hlen : mains.val.length = fits.val.length) :
    pick fits mains recent turn ⦃ r => PickPost fits mains recent turn r ⦄ := by
  unfold pick
  dsimp only
  split
  · rename_i h0
    have hn : fits.val.length = 0 := by
      have := congrArg UScalar.val h0
      simpa using this
    have hnone : ∀ i, ¬ Fit fits i := fun i hf => by have := fit_lt hf; omega
    simp only [WP.spec_ok, PickPost]
    refine ⟨fun _ => hnone, by simp, ?_⟩
    rintro ⟨i, hf, _⟩
    exact absurd hf (hnone i)
  · rename_i h0
    split
    · rename_i hne
      exfalso
      simp [Slice.len, hlen] at hne
    · have hpos : 0 < fits.val.length := by
        have : fits.len.val ≠ 0 := by
          intro h; apply h0; exact UScalar.eq_imp _ _ (by simpa using h)
        simpa using Nat.pos_of_ne_zero this
      have hu : fits.val.length ≤ U64.max := by
        have := fits.property
        rcases Usize.bounds_eq with h | h <;> scalar_tac
      step as ⟨c64, hc64⟩
      have hc : c64.val = fits.val.length := by
        rw [hc64, UScalar.cast_val_mod_pow_of_inBounds_eq _ _ (by scalar_tac)]
        simp
      step as ⟨r64, hr64⟩
      step as ⟨start, hstart⟩
      have hsv : start.val = turn.val % fits.val.length := by
        rw [hstart, UScalar.cast_val_mod_pow_of_inBounds_eq _ _ (by
          have : r64.val < fits.val.length := by rw [hr64, hc]; exact Nat.mod_lt _ hpos
          have := fits.property
          scalar_tac), hr64, hc]
      have hs : start.val < fits.val.length := by rw [hsv]; exact Nat.mod_lt _ hpos
      have hreach := fun i (hi : i < fits.val.length) => around_reaches start.val i _ hs hi
      obtain ⟨o, ho, ⟨hoNone, hoSome⟩⟩ := WP.spec_imp_exists
        (first_fresh_loop.spec fits mains recent start 0#usize hlen hs (by simp))
      unfold first_fresh
      rw [ho]
      simp only [bind_tc_ok]
      cases o with
      | some i =>
        simp only [WP.spec_ok, PickPost]
        obtain ⟨k, hk, hik, hff, hfirst⟩ := hoSome i rfl
        refine ⟨by simp, ?_, ?_⟩
        · rintro j hj
          cases hj
          exact hff.1
        · intro _
          refine ⟨i, rfl, k, hk, by rw [hik, hsv], hff, ?_⟩
          rw [← hsv]
          exact hfirst
      | none =>
        dsimp only
        unfold longest_unused
        apply WP.spec_mono (longest_unused_loop.spec fits mains recent start none 0#usize
          0#usize hlen hs (by simp) (by simp))
        rintro r ⟨hrNone, hrSome⟩
        refine ⟨?_, hrSome, ?_⟩
        · intro hrn i hf
          obtain ⟨k, hk, hki⟩ := hreach i (fit_lt hf)
          exact hrNone hrn k hk (by rw [hki]; exact hf)
        · rintro ⟨i, hff⟩
          exfalso
          obtain ⟨k, hk, hki⟩ := hreach i (fit_lt hff.1)
          exact hoNone rfl k hk (by rw [hki]; exact hff)

/-- Law 10: the pick returns a shape that fits, and returns one whenever a shape fits. -/
theorem a_pick_fits (fits : Slice Bool) (mains recent : Slice U16) (turn : U64)
    (hlen : mains.val.length = fits.val.length) :
    pick fits mains recent turn ⦃ r =>
      (r.isSome ↔ ∃ i : Nat, fits.val[i]? = some true) ∧
      ∀ i, r = some i → fits.val[i.val]? = some true ⦄ := by
  apply WP.spec_mono (pick.spec fits mains recent turn hlen)
  rintro r ⟨hnone, hsome, _⟩
  refine ⟨?_, hsome⟩
  constructor
  · intro hr
    obtain ⟨i, rfl⟩ := Option.isSome_iff_exists.mp hr
    exact ⟨i.val, hsome i rfl⟩
  · rintro ⟨i, hi⟩
    cases r with
    | none => exact absurd hi (hnone rfl i)
    | some _ => rfl

/-- Law 11: if a fitting shape has a main part outside the window, the pick takes such a
shape. -/
theorem no_main_repeats_within_n (fits : Slice Bool) (mains recent : Slice U16) (turn : U64)
    (i : Usize) (hlen : mains.val.length = fits.val.length)
    (hfresh : ∃ k : Nat, fits.val[k]? = some true ∧ ∃ m, mains.val[k]? = some m ∧ m ∉ recent.val)
    (h : pick fits mains recent turn = ok (some i)) :
    ∃ m, mains.val[i.val]? = some m ∧ m ∉ recent.val := by
  obtain ⟨r, hr, _, _, hpost⟩ := WP.spec_imp_exists (pick.spec fits mains recent turn hlen)
  rw [h] at hr
  obtain ⟨k, hk1, hk2⟩ := hfresh
  obtain ⟨j, hj, _, _, _, hff, _⟩ := hpost ⟨k, hk1, hk2⟩
  have : r = some i := by simpa using hr.symm
  rw [this] at hj
  cases hj
  exact hff.2

/-- Law 13: the pick is the first fitting fresh shape from `turn mod count`. -/
theorem the_pick_is_deterministic_from_the_turn (fits : Slice Bool) (mains recent : Slice U16)
    (turn : U64) (hlen : mains.val.length = fits.val.length)
    (hfresh : ∃ i, FitFresh fits mains recent i) :
    pick fits mains recent turn ⦃ r => ∃ i, r = some i ∧ ∃ k < fits.val.length,
      i.val = (turn.val % fits.val.length + k) % fits.val.length ∧
      FitFresh fits mains recent i.val ∧
      ∀ k' < k, ¬ FitFresh fits mains recent
        ((turn.val % fits.val.length + k') % fits.val.length) ⦄ := by
  apply WP.spec_mono (pick.spec fits mains recent turn hlen)
  rintro r ⟨_, _, hpost⟩
  exact hpost hfresh

/-! ## A run of picks -/

/-- One pick of a run: the fits and the main parts of the shapes of a moment, and the
turn. -/
structure PickStep where
  fits : Slice Bool
  mains : Slice U16
  turn : U64

/-- The window of the rotation: `WINDOW` in the Rust, 8 lines. -/
def windowSize : Nat := 8

theorem windowSize_le : windowSize ≤ Usize.max := by
  unfold windowSize
  scalar_tac

/-- The main parts of the last 8 lines, oldest first, as the story program reads them
from the calls of the character. -/
def window (used : List U16) : Slice U16 :=
  Slice.from (used.drop (used.length - windowSize)) (by
    have := windowSize_le
    simp only [List.length_drop]
    omega)

/-- A run of picks. Each pick reads the window of the main parts of the picks before it.
A pick gives its shape and the main part of that shape. -/
def runPicks : List PickStep → List U16 → Result (List (Option (Usize × U16)))
  | [], _ => ok []
  | st :: rest, used => do
    let r ← pick st.fits st.mains (window used) st.turn
    match r with
    | none => do
      let rs ← runPicks rest used
      ok (none :: rs)
    | some i => do
      let m := st.mains.val.getD i.val 0#u16
      let rs ← runPicks rest (used ++ [m])
      ok (some (i, m) :: rs)

/-- A step is wide when more than 8 distinct main parts fit. -/
def Wide (st : PickStep) : Prop :=
  st.mains.val.length = st.fits.val.length ∧
    ∃ L : List U16, L.Nodup ∧ windowSize < L.length ∧
      ∀ m ∈ L, ∃ k, Fit st.fits k ∧ st.mains.val[k]? = some m

/-- The main parts that a run picked, in order. -/
def mainsOf (outs : List (Option (Usize × U16))) : List U16 :=
  outs.map (fun o => (o.map Prod.snd).getD 0#u16)

theorem fresh_exists (st : PickStep) (hw : Wide st) (used : List U16) :
    ∃ i, FitFresh st.fits st.mains (window used) i := by
  obtain ⟨_, L, hnd, hlong, hfit⟩ := hw
  have hshort : (window used).val.length ≤ windowSize := by
    simp only [window, Slice.from_val, List.length_drop]
    omega
  have hex : ∃ m ∈ L, m ∉ (window used).val := by
    by_contra hall
    push Not at hall
    have := (hnd.subperm hall).length_le
    omega
  obtain ⟨m, hmL, hmw⟩ := hex
  obtain ⟨k, hk, hkm⟩ := hfit m hmL
  exact ⟨k, hk, m, hkm, hmw⟩

/-- Each pick of a wide run is some shape, and its main part is not in the window of the
picks before it. -/
theorem runPicks.spec (run : List PickStep) (used : List U16) (hw : ∀ st ∈ run, Wide st) :
    runPicks run used ⦃ outs => outs.length = run.length ∧
      ∀ (b : Nat) (st : PickStep), run[b]? = some st → ∃ (i : Usize) (m : U16),
        outs[b]? = some (some (i, m)) ∧ st.mains.val[i.val]? = some m ∧
        m ∉ (window (used ++ mainsOf (outs.take b))).val ⦄ := by
  induction run generalizing used with
  | nil =>
    simp [runPicks]
  | cons st rest ih =>
    have hst := hw st (by simp)
    unfold runPicks
    apply WP.spec_bind (pick.spec st.fits st.mains (window used) st.turn hst.1)
    rintro r ⟨_, _, hpost⟩
    obtain ⟨i, rfl, _, _, _, hff, _⟩ := hpost (fresh_exists st hst used)
    obtain ⟨_, m, hm, hmw⟩ := hff
    have hget : st.mains.val.getD i.val 0#u16 = m := by
      simp [List.getD, hm]
    simp only [hget]
    apply WP.spec_bind (ih (used ++ [m]) (fun s hs => hw s (by simp [hs])))
    rintro rs ⟨hlen, hall⟩
    simp only [WP.spec_ok, List.length_cons, hlen, true_and]
    intro b st' hb
    cases b with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hb
      subst hb
      exact ⟨i, m, rfl, hm, by simpa [mainsOf] using hmw⟩
    | succ b =>
      simp only [List.getElem?_cons_succ] at hb
      obtain ⟨i', m', h1, h2, h3⟩ := hall b st' hb
      refine ⟨i', m', by simpa using h1, h2, ?_⟩
      simpa [mainsOf, List.take_succ_cons, List.append_assoc] using h3

/-- Law 12: in a run of picks where each pick has more than 8 fitting main parts, every
pick takes a shape, and two picks at most 8 apart never share a main part. So no shape
repeats within 8 lines. -/
theorem a_run_never_repeats (run : List PickStep) (hw : ∀ st ∈ run, Wide st) :
    runPicks run [] ⦃ outs =>
      (∀ (b : Nat) (st : PickStep), run[b]? = some st → ∃ (i : Usize) (m : U16),
        outs[b]? = some (some (i, m)) ∧ st.mains.val[i.val]? = some m) ∧
      ∀ (a b : Nat) (ia ib : Usize) (ma mb : U16), a < b → b ≤ a + windowSize →
        outs[a]? = some (some (ia, ma)) → outs[b]? = some (some (ib, mb)) → ma ≠ mb ⦄ := by
  apply WP.spec_mono (runPicks.spec run [] hw)
  rintro outs ⟨hlen, hall⟩
  refine ⟨fun b st hb => ?_, ?_⟩
  · obtain ⟨i, m, h1, h2, _⟩ := hall b st hb
    exact ⟨i, m, h1, h2⟩
  · intro a b ia ib ma mb hab hwin ha hb
    have hbl : b < outs.length := (List.getElem?_eq_some_iff.mp hb).1
    obtain ⟨st, hst⟩ : ∃ st, run[b]? = some st :=
      List.getElem?_eq_some_iff.mpr ⟨by omega, rfl⟩ |> fun h => ⟨_, h⟩
    obtain ⟨i, m, h1, _, hnot⟩ := hall b st hst
    rw [hb] at h1
    simp only [Option.some.injEq, Prod.mk.injEq] at h1
    obtain ⟨_, rfl⟩ := h1
    intro heq
    subst heq
    apply hnot
    simp only [List.nil_append, window, Slice.from_val]
    set l := mainsOf (outs.take b) with hl
    have hll : l.length = b := by simp [hl, mainsOf]; omega
    have hla : l[a]? = some ma := by
      rw [hl, mainsOf, List.getElem?_map, List.getElem?_take_of_lt hab, ha]
      rfl
    rw [hll]
    obtain ⟨hal, hlav⟩ := List.getElem?_eq_some_iff.mp hla
    rw [List.mem_iff_getElem]
    refine ⟨a - (b - windowSize), by simp [List.length_drop]; omega, ?_⟩
    rw [List.getElem_drop]
    have : b - windowSize + (a - (b - windowSize)) = a := by omega
    simp only [this]
    exact hlav

end timeways_rules.narrator_shapes
