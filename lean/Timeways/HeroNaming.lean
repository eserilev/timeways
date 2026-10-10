-- The laws of how a line names the hero (docs/plans/narrator-style.md 4.1): the hero is
-- never named by a race, a class, or a title that names a group of the line, a naming
-- always exists, and only an unnamed turn names nobody.
import Timeways.Funs

open Aeneas Aeneas.Std Result

namespace timeways_rules.hero_naming

/-- The choice is total: every turn and every state of the words gives a naming. -/
theorem a_naming_always_exists (turn : Turn) (words : Words) :
    ∃ naming, choose turn words = ok naming := by
  obtain ⟨race, cls, title⟩ := words
  cases turn <;> cases race <;> cases cls <;> cases title <;>
    simp [choose, race_first, class_first, title_first, is_clear, is_missing]

/-- A race, a class, or a title names the hero only when no group of the line uses it. -/
theorem the_hero_is_never_named_by_a_group_word_of_the_line (turn : Turn) (words : Words)
    (naming : Naming) (h : choose turn words = ok naming) :
    (naming = Naming.Race → words.race = Word.Clear) ∧
      (naming = Naming.Class → words.class = Word.Clear) ∧
      (naming = Naming.Title → words.title = Word.Clear) := by
  obtain ⟨race, cls, title⟩ := words
  cases turn <;> cases race <;> cases cls <;> cases title <;>
    simp [choose, race_first, class_first, title_first, is_clear, is_missing] at h <;>
    subst h <;> simp

/-- A turn that names the hero always names it: a word that clashes gives `$N`, never
no name. -/
theorem a_named_turn_always_names_the_hero (turn : Turn) (words : Words)
    (naming : Naming) (h : choose turn words = ok naming) (hn : turn ≠ Turn.Unnamed) :
    naming ≠ Naming.Unnamed := by
  obtain ⟨race, cls, title⟩ := words
  cases turn <;> cases race <;> cases cls <;> cases title <;>
    simp [choose, race_first, class_first, title_first, is_clear, is_missing] at h hn <;>
    subst h <;> simp

end timeways_rules.hero_naming
