-- The axiom check. Each law depends only on the three standard axioms
-- of Lean. A `sorry` or a new axiom changes the output, and the build fails.
import Timeways.QuestLog
import Timeways.HeroHook
import Timeways.Budget
import Timeways.TrustBand

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

/-- info: 'timeways_rules.quest_log.a_quest_is_done_exactly_when_every_step_is_done' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.a_quest_is_done_exactly_when_every_step_is_done

/-- info: 'timeways_rules.quest_log.each_giver_holds_at_most_one_waiting_offer' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.each_giver_holds_at_most_one_waiting_offer

/-- info: 'timeways_rules.quest_log.a_kill_counts_only_for_an_open_step' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.a_kill_counts_only_for_an_open_step

/-- info: 'timeways_rules.quest_log.every_quest_points_at_its_offer' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms quest_log.every_quest_points_at_its_offer

/-- info: 'timeways_rules.hero_hook.pick_is_never_out_of_range' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms hero_hook.pick_is_never_out_of_range

/-- info: 'timeways_rules.hero_hook.every_filled_field_takes_its_turn' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms hero_hook.every_filled_field_takes_its_turn

/-- info: 'timeways_rules.budget.the_narrator_never_speaks_four_times_in_one_hour' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms budget.the_narrator_never_speaks_four_times_in_one_hour

/-- info: 'timeways_rules.trust.next_trust.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms trust.next_trust.spec

/-- info: 'timeways_rules.trust.trust_stays_between_minus_one_hundred_and_one_hundred' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms trust.trust_stays_between_minus_one_hundred_and_one_hundred
