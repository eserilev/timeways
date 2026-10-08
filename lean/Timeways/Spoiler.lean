-- The laws of the one gate of the spoiler limit (GAMEPLAY.md 3.1 and 5.10): a foe that
-- the player defeated is known, an outcome whose deeds the player did needs no visit to
-- the place where the pack files it, and nothing else becomes visible.
import Timeways.Setups

open Aeneas Aeneas.Std Result

namespace timeways_rules.spoiler

/-- A name of `held` names the same person or place as `id`, by the rows of game names. -/
def holds (facts : WorldFacts) (held : List U32) (id : U32) : Prop :=
  ∃ x ∈ held,
    game_names.personM facts.deeds.names.val x = game_names.personM facts.deeds.names.val id

/-- The world holds the link: a visit to the place, or a meeting with the NPC or its
defeat. -/
def knows (link : LinkTo) (facts : WorldFacts) : Prop :=
  match link with
  | LinkTo.Common => True
  | LinkTo.Place place => holds facts facts.visited.val place
  | LinkTo.Npc npc => holds facts facts.met.val npc ∨ holds facts facts.deeds.defeated.val npc

/-- The tag names a deed: a foe or a quest. -/
def IsDeed : outcomes.DependsOn → Prop
  | outcomes.DependsOn.Foe _ => True
  | outcomes.DependsOn.Quest _ => True
  | _ => False

def IsPlace : LinkTo → Prop
  | LinkTo.Place _ => True
  | _ => False

/-- The check of one link, exactly. -/
@[step]
theorem link_known.spec (link : LinkTo) (facts : WorldFacts) :
    link_known link facts ⦃ b => (b = true ↔ knows link facts) ⦄ := by
  unfold link_known
  cases link with
  | Common => simp [knows]
  | Place place =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    simp [knows, holds, hb]
  | Npc npc =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    split
    · rename_i hbt
      simp_all [knows, holds]
    · rename_i hbt
      step as ⟨c, hc⟩
      simp only [alloc.vec.Vec.deref] at hc
      simp_all [knows, holds]

/-- The check of one tag, exactly. -/
@[step]
theorem is_deed.spec (tag : outcomes.DependsOn) :
    is_deed tag ⦃ b => (b = true ↔ IsDeed tag) ⦄ := by
  unfold is_deed
  cases tag <;> simp [IsDeed]

@[step]
theorem tells_a_deed_loop.spec (tags : Slice outcomes.DependsOn) (i : Usize) :
    tells_a_deed_loop tags i ⦃ b => (b = true ↔ ∃ t ∈ tags.val.drop i.val, IsDeed t) ⦄ := by
  unfold tells_a_deed_loop
  dsimp only
  split
  · rename_i hi
    step as ⟨t, ht⟩
    have hdrop : tags.val.drop i.val = t :: tags.val.drop (i.val + 1) := by
      rw [ht]
      exact List.drop_eq_getElem_cons (by scalar_tac)
    step as ⟨b, hb⟩
    split
    · rename_i hbt
      simp only [WP.spec_ok, true_iff]
      exact ⟨t, by rw [hdrop]; exact List.mem_cons_self .., hb.mp hbt⟩
    · rename_i hbt
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (tells_a_deed_loop.spec tags i1)
      intro c hc
      rw [hc, hdrop, hi1]
      constructor
      · rintro ⟨u, hu, hd⟩
        exact ⟨u, List.mem_cons_of_mem _ hu, hd⟩
      · rintro ⟨u, hu, hd⟩
        rcases List.mem_cons.mp hu with h | h
        · subst h
          exact absurd (hb.mpr hd) hbt
        · exact ⟨u, h, hd⟩
  · rename_i hi
    have hdrop : tags.val.drop i.val = [] := List.drop_eq_nil_of_le (by scalar_tac)
    simp [hdrop]
termination_by tags.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The check of the tags, exactly: one of them names a deed. -/
@[step]
theorem tells_a_deed.spec (tags : Slice outcomes.DependsOn) :
    tells_a_deed tags ⦃ b => (b = true ↔ ∃ t ∈ tags.val, IsDeed t) ⦄ := by
  unfold tells_a_deed
  simpa using tells_a_deed_loop.spec tags 0#usize

/-- The waiver, exactly: a place of a passage whose deed is done. -/
@[step]
theorem is_waived.spec (link : LinkTo) (deed_done : Bool) :
    is_waived link deed_done ⦃ b => (b = true ↔ deed_done = true ∧ IsPlace link) ⦄ := by
  unfold is_waived
  cases deed_done <;> cases link <;> simp [IsPlace]

@[step]
theorem links_known_loop.spec (links : Slice LinkTo) (deed_done : Bool) (facts : WorldFacts)
    (i : Usize) :
    links_known_loop links deed_done facts i ⦃ b =>
      (b = true ↔ ∀ l ∈ links.val.drop i.val,
        (deed_done = true ∧ IsPlace l) ∨ knows l facts) ⦄ := by
  unfold links_known_loop
  dsimp only
  split
  · rename_i hi
    step as ⟨l, hl⟩
    have hdrop : links.val.drop i.val = l :: links.val.drop (i.val + 1) := by
      rw [hl]
      exact List.drop_eq_getElem_cons (by scalar_tac)
    step as ⟨w, hw⟩
    split
    · rename_i hwt
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (links_known_loop.spec links deed_done facts i1)
      intro c hc
      rw [hc, hdrop, hi1]
      simp only [List.mem_cons, forall_eq_or_imp]
      exact ⟨fun h => ⟨Or.inl (hw.mp hwt), h⟩, fun h => h.2⟩
    · rename_i hwt
      step as ⟨k, hk⟩
      split
      · rename_i hkt
        step as ⟨i1, hi1⟩
        apply WP.spec_mono (links_known_loop.spec links deed_done facts i1)
        intro c hc
        rw [hc, hdrop, hi1]
        simp only [List.mem_cons, forall_eq_or_imp]
        exact ⟨fun h => ⟨Or.inr (hk.mp hkt), h⟩, fun h => h.2⟩
      · rename_i hkt
        simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
        intro h
        rw [hdrop] at h
        rcases h l List.mem_cons_self with h | h
        · exact hwt (hw.mpr h)
        · exact hkt (hk.mpr h)
  · rename_i hi
    have hdrop : links.val.drop i.val = [] := List.drop_eq_nil_of_le (by scalar_tac)
    simp [hdrop]
termination_by links.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The check of the links, exactly: each one is held, or it is a place and the deed is
done. -/
@[step]
theorem links_known.spec (links : Slice LinkTo) (deed_done : Bool) (facts : WorldFacts) :
    links_known links deed_done facts ⦃ b =>
      (b = true ↔ ∀ l ∈ links.val, (deed_done = true ∧ IsPlace l) ∨ knows l facts) ⦄ := by
  unfold links_known
  simpa using links_known_loop.spec links deed_done facts 0#usize

/-- The gate, exactly: every deed done, the setup not stale, and each link held, or a
place of a passage that tells a deed. -/
@[step]
theorem passage_usable.spec (links : Slice LinkTo) (tags : Slice outcomes.DependsOn)
    (setup_for : setups.SetupFor) (facts : WorldFacts) :
    passage_usable links tags setup_for facts ⦃ b =>
      (b = true ↔
        (∀ t ∈ tags.val, t = outcomes.DependsOn.Nothing ∨ outcomes.did t facts.deeds) ∧
        ¬ setups.did setup_for facts.deeds ∧
        ∀ l ∈ links.val, ((∃ t ∈ tags.val, IsDeed t) ∧ IsPlace l) ∨ knows l facts) ⦄ := by
  unfold passage_usable
  step as ⟨b, hb⟩
  split
  · rename_i hbt
    step as ⟨c, hc⟩
    split
    · rename_i hct
      step as ⟨d, hd⟩
      step as ⟨e, he⟩
      rw [he]
      simp only [hd]
      exact ⟨fun h => ⟨hb.mp hbt, hc.mp hct, h⟩, fun h => h.2.2⟩
    · rename_i hct
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
      intro _ h
      exact absurd (hc.mpr h) hct
  · rename_i hbt
    simp only [WP.spec_ok, Bool.false_eq_true, false_iff, not_and]
    intro h
    exact absurd (hb.mpr h) hbt

/-- The gate never panics, and it always ends. -/
theorem the_spoiler_gate_never_panics (links : Slice LinkTo) (tags : Slice outcomes.DependsOn)
    (setup_for : setups.SetupFor) (facts : WorldFacts) :
    ∃ b, passage_usable links tags setup_for facts = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (passage_usable.spec links tags setup_for facts)
  exact ⟨b, hb⟩

/-- A foe that the player defeated is known, under any name of the same person. -/
theorem a_defeated_foe_is_known (facts : WorldFacts) (npc : U32)
    (h_kill : holds facts facts.deeds.defeated.val npc) :
    link_known (LinkTo.Npc npc) facts = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (link_known.spec (LinkTo.Npc npc) facts)
  rw [hb, hp.mpr (Or.inr h_kill)]

/-- For a row (g, w) of game names that are a function: a kill under either name makes
the foe known under both. -/
theorem a_foe_defeated_under_either_name_is_known (facts : WorldFacts)
    (r : game_names.NameRow) (hf : game_names.Functional facts.deeds.names.val)
    (hr : r ∈ facts.deeds.names.val)
    (h_kill : r.game ∈ facts.deeds.defeated.val ∨ r.wiki ∈ facts.deeds.defeated.val) :
    link_known (LinkTo.Npc r.game) facts = ok true ∧
      link_known (LinkTo.Npc r.wiki) facts = ok true := by
  have one := game_names.either_name_of_a_row_names_one_person _ r hf hr
  rcases h_kill with h | h
  · exact ⟨a_defeated_foe_is_known facts _ ⟨r.game, h, rfl⟩,
      a_defeated_foe_is_known facts _ ⟨r.game, h, one⟩⟩
  · exact ⟨a_defeated_foe_is_known facts _ ⟨r.wiki, h, one.symm⟩,
      a_defeated_foe_is_known facts _ ⟨r.wiki, h, rfl⟩⟩

/-- An outcome passage whose deeds the player did passes, whatever place the pack files
it under: only its NPCs and its setup still count. -/
theorem an_outcome_the_player_did_is_usable_wherever_it_is_filed (links : Slice LinkTo)
    (tags : Slice outcomes.DependsOn) (setup_for : setups.SetupFor) (facts : WorldFacts)
    (h_deed : ∃ t ∈ tags.val, IsDeed t)
    (h_done : ∀ t ∈ tags.val, t = outcomes.DependsOn.Nothing ∨ outcomes.did t facts.deeds)
    (h_setup : ¬ setups.did setup_for facts.deeds)
    (h_links : ∀ l ∈ links.val, IsPlace l ∨ knows l facts) :
    passage_usable links tags setup_for facts = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (passage_usable.spec links tags setup_for facts)
  rw [hb]
  refine congrArg ok (hp.mpr ⟨h_done, h_setup, fun l hl => ?_⟩)
  rcases h_links l hl with h | h
  · exact Or.inl ⟨h_deed, h⟩
  · exact Or.inr h

/-- Nothing else becomes visible: a passage with a link that the player neither visited,
met, nor defeated is hidden, unless the link is the place of a deed that the passage tells
and the player did. -/
theorem a_passage_the_player_neither_met_nor_did_stays_hidden (links : Slice LinkTo)
    (tags : Slice outcomes.DependsOn) (setup_for : setups.SetupFor) (facts : WorldFacts)
    (l : LinkTo) (h_link : l ∈ links.val) (h_unknown : ¬ knows l facts)
    (h_not_done : ¬ (IsPlace l ∧ ∃ t ∈ tags.val, IsDeed t ∧ outcomes.did t facts.deeds)) :
    passage_usable links tags setup_for facts = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (passage_usable.spec links tags setup_for facts)
  rw [hb]
  cases b
  · rfl
  · obtain ⟨h_done, _, h_links⟩ := hp.mp rfl
    rcases h_links l h_link with ⟨⟨t, ht, hdeed⟩, hplace⟩ | h
    · rcases h_done t ht with hn | hd
      · subst hn
        simp [IsDeed] at hdeed
      · exact absurd ⟨hplace, t, ht, hdeed, hd⟩ h_not_done
    · exact absurd h h_unknown

end timeways_rules.spoiler
