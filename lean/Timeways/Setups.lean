-- The laws of the gate of setup passages (GAMEPLAY.md 5.10): a setup tells who wants a
-- deed done, so it reaches a prompt only while the player has not done that deed. A
-- passage with no setup is not gated, and the gate never panics.
import Timeways.Outcomes

open Aeneas Aeneas.Std Result

namespace timeways_rules.setups

/-- The player did the deed that the setup asks for. `Nothing` names no deed, so no
player did it. -/
def did (setup_for : SetupFor) (facts : outcomes.PlayerFacts) : Prop :=
  match setup_for with
  | SetupFor.Nothing => False
  | SetupFor.Foe foe =>
    ∃ x ∈ facts.defeated.val,
      game_names.personM facts.names.val x = game_names.personM facts.names.val foe
  | SetupFor.Quest quest => quest ∈ facts.quests_done.val

/-- The rule, exactly: a passage passes exactly when the player has not done the deed
that it sets up. -/
@[step]
theorem setup_usable.spec (setup_for : SetupFor) (facts : outcomes.PlayerFacts) :
    setup_usable setup_for facts ⦃ b => (b = true ↔ ¬ did setup_for facts) ⦄ := by
  unfold setup_usable
  cases setup_for with
  | Nothing => simp [did]
  | Foe foe =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    cases b <;> simp_all [did]
  | Quest quest =>
    step as ⟨b, hb⟩
    simp only [alloc.vec.Vec.deref] at hb
    cases b <;> simp_all [did]

/-- The gate never panics, and it always ends. -/
theorem the_setup_gate_never_panics (setup_for : SetupFor) (facts : outcomes.PlayerFacts) :
    ∃ b, setup_usable setup_for facts = ok b := by
  obtain ⟨b, hb, _⟩ := WP.spec_imp_exists (setup_usable.spec setup_for facts)
  exact ⟨b, hb⟩

/-- A setup for a deed that the player did is stale. It is refused, so it never reaches
a prompt. -/
theorem a_setup_for_a_deed_the_player_did_never_reaches_a_prompt
    (setup_for : SetupFor) (facts : outcomes.PlayerFacts) (h_done : did setup_for facts) :
    setup_usable setup_for facts = ok false := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (setup_usable.spec setup_for facts)
  rw [hb]
  cases b
  · rfl
  · exact absurd h_done (hp.mp rfl)

/-- A setup for a deed that the player has not done passes this rule. -/
theorem a_setup_for_a_deed_not_done_is_not_gated_by_this_rule
    (setup_for : SetupFor) (facts : outcomes.PlayerFacts) (h_not_done : ¬ did setup_for facts) :
    setup_usable setup_for facts = ok true := by
  obtain ⟨b, hb, hp⟩ := WP.spec_imp_exists (setup_usable.spec setup_for facts)
  rw [hb, hp.mpr h_not_done]

/-- A passage with no setup passes this rule, whatever the world of the player holds. -/
theorem a_passage_with_no_setup_is_not_gated_by_this_rule (facts : outcomes.PlayerFacts) :
    setup_usable SetupFor.Nothing facts = ok true := by
  rfl

/-- For a row (g, w) of game names that are a function: a setup tagged with the wiki
name w goes stale after a kill under the game name g, and after a kill under w. -/
theorem a_kill_under_either_name_makes_the_setup_stale (facts : outcomes.PlayerFacts)
    (r : game_names.NameRow) (hf : game_names.Functional facts.names.val)
    (hr : r ∈ facts.names.val)
    (h_kill : r.game ∈ facts.defeated.val ∨ r.wiki ∈ facts.defeated.val) :
    setup_usable (SetupFor.Foe r.wiki) facts = ok false := by
  apply a_setup_for_a_deed_the_player_did_never_reaches_a_prompt
  rcases h_kill with h | h
  · exact ⟨r.game, h, game_names.either_name_of_a_row_names_one_person _ r hf hr⟩
  · exact ⟨r.wiki, h, rfl⟩

/-- A kill of another person never makes a setup stale. -/
theorem a_kill_of_another_person_keeps_the_setup (facts : outcomes.PlayerFacts) (foe : U32)
    (h_other : ∀ x ∈ facts.defeated.val,
      game_names.personM facts.names.val x ≠ game_names.personM facts.names.val foe) :
    setup_usable (SetupFor.Foe foe) facts = ok true := by
  apply a_setup_for_a_deed_not_done_is_not_gated_by_this_rule
  rintro ⟨x, hx, heq⟩
  exact h_other x hx heq

end timeways_rules.setups
