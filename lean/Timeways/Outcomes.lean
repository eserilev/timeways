-- The laws of the gate of outcome passages (GAMEPLAY.md 5.10): a passage that tells a
-- deed of adventurers reaches a prompt only when the player did that deed, an unresolved
-- deed never does, a passage with no deed is not gated, and the gate never panics.
import Timeways.ThinLore
import Timeways.GameNames

open Aeneas Aeneas.Std Result

namespace timeways_rules.outcomes

/-- The player did the deed that the passage depends on. `Nothing` and `Unresolved` name
no deed, so no player did them. A kill counts under any name of the person
(`game_names.personM`). -/
def did (depends_on : DependsOn) (facts : PlayerFacts) : Prop :=
  match depends_on with
  | DependsOn.Nothing => False
  | DependsOn.Unresolved => False
  | DependsOn.Foe foe =>
    ∃ x ∈ facts.defeated.val,
      game_names.personM facts.names.val x = game_names.personM facts.names.val foe
  | DependsOn.Quest quest => quest ∈ facts.quests_done.val

/-- The rule, exactly: a passage with no deed passes, and any other passage passes exactly
when the player did its deed. -/
@[step]
theorem outcome_usable.spec (depends_on : DependsOn) (facts : PlayerFacts) :
    outcome_usable depends_on facts ⦃ b =>
      (b = true ↔ depends_on = DependsOn.Nothing ∨ did depends_on facts) ⦄ := by
  unfold outcome_usable
  cases depends_on with
  | Nothing => simp
  | Unresolved => simp [did]
  | Foe foe =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    simp [did, hb]
  | Quest quest =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    simp [did, hb]

/-- The gate never panics, and it always ends. -/
theorem the_gate_never_panics (depends_on : DependsOn) (facts : PlayerFacts) :
    ∃ b, outcome_usable depends_on facts = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (outcome_usable.spec depends_on facts)
  exact ⟨b, hb⟩

/-- A passage that tells a deed that the player did not do is refused, so it never
reaches a prompt. -/
theorem an_outcome_the_player_did_not_do_never_reaches_a_prompt
    (depends_on : DependsOn) (facts : PlayerFacts)
    (h_outcome : depends_on ≠ DependsOn.Nothing) (h_not_done : ¬ did depends_on facts) :
    outcome_usable depends_on facts = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (outcome_usable.spec depends_on facts)
  rw [hb]
  cases b
  · rfl
  · rcases hp.mp rfl with h | h
    · exact absurd h h_outcome
    · exact absurd h h_not_done

/-- An unresolved deed is refused, whatever the world of the player holds. -/
theorem an_unresolved_outcome_is_never_used (facts : PlayerFacts) :
    outcome_usable DependsOn.Unresolved facts = ok false := by
  rfl

/-- A passage with no deed passes this rule, whatever the world of the player holds. -/
theorem a_passage_with_no_outcome_is_not_gated_by_this_rule (facts : PlayerFacts) :
    outcome_usable DependsOn.Nothing facts = ok true := by
  rfl

/-- A deed that the player did lets its passage through. -/
theorem an_outcome_the_player_did_passes (depends_on : DependsOn) (facts : PlayerFacts)
    (h_done : did depends_on facts) :
    outcome_usable depends_on facts = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (outcome_usable.spec depends_on facts)
  rw [hb, hp.mpr (Or.inr h_done)]

@[step]
theorem outcomes_usable_loop.spec (tags : Slice DependsOn) (facts : PlayerFacts)
    (i : Usize) :
    outcomes_usable_loop tags facts i ⦃ b =>
      (b = true ↔ ∀ t ∈ tags.val.drop i.val, t = DependsOn.Nothing ∨ did t facts) ⦄ := by
  unfold outcomes_usable_loop
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
      step as ⟨i1, hi1⟩
      apply WP.spec_mono (outcomes_usable_loop.spec tags facts i1)
      intro c hc
      rw [hc, hdrop, hi1]
      simp only [List.mem_cons, forall_eq_or_imp]
      exact ⟨fun h => ⟨hb.mp hbt, h⟩, fun h => h.2⟩
    · rename_i hbt
      simp only [WP.spec_ok, Bool.false_eq_true, false_iff]
      intro h
      rw [hdrop] at h
      exact hbt (hb.mpr (h t List.mem_cons_self))
  · rename_i hi
    have hdrop : tags.val.drop i.val = [] := List.drop_eq_nil_of_le (by scalar_tac)
    simp [hdrop]
termination_by tags.length - i.val
decreasing_by all_goals scalar_decr_tac

/-- The gate of a passage with several tags, exactly: it passes when every tag passes.
So a passage that tells two deeds shows only after both. -/
@[step]
theorem outcomes_usable.spec (tags : Slice DependsOn) (facts : PlayerFacts) :
    outcomes_usable tags facts ⦃ b =>
      (b = true ↔ ∀ t ∈ tags.val, t = DependsOn.Nothing ∨ did t facts) ⦄ := by
  unfold outcomes_usable
  simpa using outcomes_usable_loop.spec tags facts 0#usize

/-- A passage with two deeds is refused while one of them is not done. -/
theorem a_passage_with_two_deeds_waits_for_both (tags : Slice DependsOn)
    (facts : PlayerFacts) (t : DependsOn) (h_tag : t ∈ tags.val)
    (h_deed : t ≠ DependsOn.Nothing) (h_not_done : ¬ did t facts) :
    outcomes_usable tags facts = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (outcomes_usable.spec tags facts)
  rw [hb]
  cases b
  · rfl
  · rcases hp.mp rfl t h_tag with h | h
    · exact absurd h h_deed
    · exact absurd h h_not_done

/-- For a row (g, w) of game names that are a function: a passage tagged with the wiki
name w passes after a kill under the game name g, and after a kill under w. -/
theorem a_kill_under_either_name_unlocks_the_outcome (facts : PlayerFacts)
    (r : game_names.NameRow) (hf : game_names.Functional facts.names.val)
    (hr : r ∈ facts.names.val)
    (h_kill : r.game ∈ facts.defeated.val ∨ r.wiki ∈ facts.defeated.val) :
    outcome_usable (DependsOn.Foe r.wiki) facts = ok true := by
  apply an_outcome_the_player_did_passes
  rcases h_kill with h | h
  · exact ⟨r.game, h, game_names.either_name_of_a_row_names_one_person _ r hf hr⟩
  · exact ⟨r.wiki, h, rfl⟩

/-- A kill of another person unlocks nothing: when no kill names the person of the
tag, the passage is refused. -/
theorem a_kill_of_another_person_unlocks_nothing (facts : PlayerFacts) (foe : U32)
    (h_other : ∀ x ∈ facts.defeated.val,
      game_names.personM facts.names.val x ≠ game_names.personM facts.names.val foe) :
    outcome_usable (DependsOn.Foe foe) facts = ok false := by
  apply an_outcome_the_player_did_not_do_never_reaches_a_prompt
  · simp
  · rintro ⟨x, hx, heq⟩
    exact h_other x hx heq

end timeways_rules.outcomes
