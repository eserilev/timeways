-- The axiom check. Each law depends only on the three standard axioms
-- of Lean. A `sorry` or a new axiom changes the output, and the build fails.
import Timeways.QuestLog

open timeways_rules

/-- info: 'timeways_rules.quest_log.quest_log.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.quest_log.spec

/-- info: 'timeways_rules.quest_log.ordered_quest_is_done_in_order' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.ordered_quest_is_done_in_order

/-- info: 'timeways_rules.quest_log.any_order_set_waits_for_the_steps_before_it' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.any_order_set_waits_for_the_steps_before_it

/-- info: 'timeways_rules.quest_log.a_step_after_a_set_waits_for_all_of_it' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.a_step_after_a_set_waits_for_all_of_it

/-- info: 'timeways_rules.quest_log.a_wait_never_ends_early' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.a_wait_never_ends_early

/-- info: 'timeways_rules.quest_log.kills_never_pass_the_count' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.kills_never_pass_the_count

/-- info: 'timeways_rules.quest_log.an_offer_has_no_done_steps' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.an_offer_has_no_done_steps
