-- The laws of the prologue (GAMEPLAY.md 3.3): a new character gets no
-- prologue, a prologue is written at most once, and the rule never panics.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.prologue

deriving instance DecidableEq for WorldAge
deriving instance DecidableEq for Prologue

/-- A past that earns a prologue, as a proposition. -/
def Long (p : Past) : Prop :=
  p.world = WorldAge.New ∧ 10 ≤ p.level.val ∧ (20 ≤ p.quests.val ∨ 2 ≤ p.zones.val)

instance (p : Past) : Decidable (Long p) := by
  unfold Long; exact inferInstance

theorem is_long.spec (p : Past) :
    is_long p ⦃ b => (b = true ↔ Long p) ⦄ := by
  unfold is_long Long
  have hl : PROLOGUE_LEVEL.val = 10 := by simp [PROLOGUE_LEVEL]
  have hq : PROLOGUE_QUESTS.val = 20 := by simp [PROLOGUE_QUESTS]
  have hz : PROLOGUE_ZONES.val = 2 := by simp [PROLOGUE_ZONES]
  rcases p with ⟨level, quests, zones, world⟩
  cases world
  · simp only []
    by_cases h1 : level < PROLOGUE_LEVEL
    · simp only [h1, if_true, WP.spec_ok]
      simp only [UScalar.lt_equiv] at h1
      simp; omega
    · simp only [h1, if_false]
      simp only [UScalar.lt_equiv] at h1
      by_cases h2 : quests >= PROLOGUE_QUESTS
      · simp only [h2, if_true, WP.spec_ok]
        simp only [ge_iff_le, UScalar.le_equiv] at h2
        simp; omega
      · simp only [h2, if_false, WP.spec_ok]
        simp only [ge_iff_le, UScalar.le_equiv] at h2
        simp only [ge_iff_le, decide_eq_true_eq, UScalar.le_equiv, true_and]
        omega
  · simp

/-- The pure model of `after_past`. -/
def pastM (s : Prologue) (p : Past) : Prologue :=
  match s with
  | .Unseen => if Long p then .Due else .Skipped
  | other => other

/-- The pure model of `after_written`. -/
def writtenM (s : Prologue) : Prologue :=
  match s with
  | .Due => .Written
  | other => other

def dueM (s : Prologue) : Bool :=
  match s with
  | .Due => true
  | _ => false

theorem after_past.spec (s : Prologue) (p : Past) :
    after_past s p ⦃ r => r = pastM s p ⦄ := by
  unfold after_past pastM
  cases s <;> simp only [WP.spec_ok]
  have h := is_long.spec p
  cases hb : decide (Long p) with
  | false =>
    have hn : ¬ Long p := of_decide_eq_false hb
    apply WP.spec_bind h
    intro b hbl
    have : b = false := by cases b <;> simp_all
    subst this
    simp [hn]
  | true =>
    have hy : Long p := of_decide_eq_true hb
    apply WP.spec_bind h
    intro b hbl
    have : b = true := hbl.mpr hy
    subst this
    simp [hy]

theorem after_written.spec (s : Prologue) :
    after_written s ⦃ r => r = writtenM s ⦄ := by
  unfold after_written writtenM
  cases s <;> simp

theorem is_due.spec (s : Prologue) :
    is_due s ⦃ b => b = dueM s ⦄ := by
  unfold is_due dueM
  cases s <;> simp

/-- The rule never panics: each function gives a result for every input. -/
theorem the_rule_never_panics (s : Prologue) (p : Past) :
    (∃ r, after_past s p = ok r) ∧ (∃ r, after_written s = ok r) ∧ (∃ b, is_due s = ok b) := by
  refine ⟨?_, ?_, ?_⟩
  · have h := after_past.spec s p
    cases hr : after_past s p <;> simp_all
  · have h := after_written.spec s
    cases hr : after_written s <;> simp_all
  · have h := is_due.spec s
    cases hr : is_due s <;> simp_all

/-- What can happen to the prologue of a character: a past comes at a login, or the
model writes the prologue. -/
inductive Act where
  | past (p : Past)
  | written

def step (s : Prologue) : Act → Prologue
  | .past p => pastM s p
  | .written => writtenM s

def run (s : Prologue) : List Act → Prologue
  | [] => s
  | a :: acts => run (step s a) acts

/-- The writes of a prologue in a run: the steps that make it written. -/
def writes (s : Prologue) : List Act → Nat
  | [] => 0
  | a :: acts =>
    (if step s a = .Written ∧ s ≠ .Written then 1 else 0) + writes (step s a) acts

theorem skipped_stays (acts : List Act) : run .Skipped acts = .Skipped := by
  induction acts with
  | nil => rfl
  | cons a acts ih => cases a <;> simpa [run, step, pastM, writtenM] using ih

theorem written_stays (acts : List Act) : run .Written acts = .Written := by
  induction acts with
  | nil => rfl
  | cons a acts ih => cases a <;> simpa [run, step, pastM, writtenM] using ih

theorem no_writes_after_written (acts : List Act) : writes .Written acts = 0 := by
  induction acts with
  | nil => rfl
  | cons a acts ih => cases a <;> simpa [writes, step, pastM, writtenM] using ih

/-- A new character gets no prologue: when the first past that comes is not long,
the prologue is never due, whatever comes after it. -/
theorem a_new_character_gets_no_prologue (p : Past) (h : ¬ Long p) (acts : List Act) :
    dueM (run .Unseen (.past p :: acts)) = false := by
  simp [run, step, pastM, h, skipped_stays, dueM]

/-- A character that Timeways saw with play gets no prologue, at any level. -/
theorem a_played_world_gets_no_prologue (p : Past) (h : p.world = WorldAge.Played)
    (acts : List Act) : dueM (run .Unseen (.past p :: acts)) = false := by
  apply a_new_character_gets_no_prologue
  intro hl
  rw [hl.1] at h
  cases h

/-- A prologue is written at most once, from any state and for any run. -/
theorem a_prologue_is_written_at_most_once (s : Prologue) (acts : List Act) :
    writes s acts ≤ 1 := by
  induction acts generalizing s with
  | nil => simp [writes]
  | cons a acts ih =>
    unfold writes
    by_cases hw : step s a = .Written ∧ s ≠ .Written
    · rw [if_pos hw, hw.1, no_writes_after_written]
    · rw [if_neg hw]
      simpa using ih (step s a)

/-- A written prologue is never due again. -/
theorem a_written_prologue_is_never_due (acts : List Act) :
    dueM (run .Written acts) = false := by
  simp [written_stays, dueM]

end timeways_rules.prologue
