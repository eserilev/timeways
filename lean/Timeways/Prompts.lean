-- The prompt law: the newest 500 calls always keep their prompts, for
-- every position of the newest call, also at the edge of u64.
import Timeways.Funs
import Timeways.Budget

open Aeneas Aeneas.Std Result

namespace timeways_rules.prompts

/-- The oldest call that keeps its prompt is never after the newest one.
The newest 500 calls keep their prompts, or every call when there are
fewer. -/
theorem the_newest_prompts_are_always_kept (newest : U64) :
    oldest_prompt_kept newest ⦃ oldest =>
      oldest.val ≤ newest.val ∧ newest.val - oldest.val < 500 ∧
      (newest.val < 500 ∨ newest.val - oldest.val = 499) ⦄ := by
  unfold oldest_prompt_kept
  have hk : PROMPTS_KEPT.val = 500 := by simp [PROMPTS_KEPT]
  step as ⟨i, hi⟩
  have hs := budget.saturating_sub_val newest i
  omega

end timeways_rules.prompts
