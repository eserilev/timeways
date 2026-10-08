-- The axiom check. Each law depends only on the three standard axioms
-- of Lean. A `sorry` or a new axiom changes the output, and the build fails.
import Timeways.QuestLog
import Timeways.HeroHook
import Timeways.Budget
import Timeways.TrustBand
import Timeways.Prompts
import Timeways.Aliases
import Timeways.StoryShelf
import Timeways.EntryEdits
import Timeways.Chapters
import Timeways.ChaptersDeaths
import Timeways.NarratorShapes
import Timeways.ThinLore
import Timeways.GameNames
import Timeways.Outcomes
import Timeways.Setups
import Timeways.InstanceLore

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

/-- info: 'timeways_rules.budget.budget_on_load.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms budget.budget_on_load.spec

/-- info: 'timeways_rules.budget.the_budget_law_holds_after_any_load' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms budget.the_budget_law_holds_after_any_load

/-- info: 'timeways_rules.trust.next_trust.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms trust.next_trust.spec

/-- info: 'timeways_rules.trust.trust_stays_between_minus_one_hundred_and_one_hundred' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms trust.trust_stays_between_minus_one_hundred_and_one_hundred

/-- info: 'timeways_rules.prompts.the_newest_prompts_are_always_kept' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms prompts.the_newest_prompts_are_always_kept

/-- info: 'timeways_rules.aliases.an_id_is_never_reused' depends on axioms: [propext] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.an_id_is_never_reused

/-- info: 'timeways_rules.aliases.a_name_keeps_its_id' depends on axioms: [propext, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.a_name_keeps_its_id

/-- info: 'timeways_rules.aliases.two_names_never_share_an_id' depends on axioms: [propext, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.two_names_never_share_an_id

/-- info: 'timeways_rules.aliases.one_id_names_one_player' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.one_id_names_one_player

/-- info: 'timeways_rules.aliases.every_name_of_a_line_gets_an_id' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.every_name_of_a_line_gets_an_id

/-- info: 'timeways_rules.aliases.no_known_name_after_the_swap' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.no_known_name_after_the_swap

/-- info: 'timeways_rules.aliases.every_id_of_the_swap_is_in_the_table' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.every_id_of_the_swap_is_in_the_table

/-- info: 'timeways_rules.aliases.the_swap_and_back_keeps_the_text' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.the_swap_and_back_keeps_the_text

/-- info: 'timeways_rules.aliases.restore_keeps_what_is_no_known_name' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms aliases.restore_keeps_what_is_no_known_name

/-- info: 'timeways_rules.story_shelf.lands.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms story_shelf.lands.spec

/-- info: 'timeways_rules.story_shelf.standing.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms story_shelf.standing.spec

/-- info: 'timeways_rules.story_shelf.a_number_stands_at_most_once' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms story_shelf.a_number_stands_at_most_once

/-- info: 'timeways_rules.story_shelf.a_removed_story_never_comes_back' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms story_shelf.a_removed_story_never_comes_back

/-- info: 'timeways_rules.story_shelf.a_used_story_always_stands' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms story_shelf.a_used_story_always_stands

/-- info: 'timeways_rules.entry_edits.newest_row.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.newest_row.spec

/-- info: 'timeways_rules.entry_edits.newest_edit.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.newest_edit.spec

/-- info: 'timeways_rules.entry_edits.shown.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.shown.spec

/-- info: 'timeways_rules.entry_edits.pickEdit_tie' depends on axioms: [propext] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.pickEdit_tie

/-- info: 'timeways_rules.entry_edits.the_newest_edit_decides' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.the_newest_edit_decides

/-- info: 'timeways_rules.entry_edits.a_restore_shows_the_newest_narrator_text' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.a_restore_shows_the_newest_narrator_text

/-- info: 'timeways_rules.entry_edits.a_restore_shows_a_later_narrator_text' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.a_restore_shows_a_later_narrator_text

/-- info: 'timeways_rules.entry_edits.a_model_text_never_hides_player_words' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms entry_edits.a_model_text_never_hides_player_words

/-- info: 'timeways_rules.chapters.advance.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.advance.spec

/-- info: 'timeways_rules.chapters.chapters.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.chapters.spec

/-- info: 'timeways_rules.chapters.every_step_is_in_one_chapter' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.every_step_is_in_one_chapter

/-- info: 'timeways_rules.chapters.every_instance_step_is_in_one_visit' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.every_instance_step_is_in_one_visit

/-- info: 'timeways_rules.chapters.a_closed_chapter_never_changes' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_closed_chapter_never_changes

/-- info: 'timeways_rules.chapters.a_closed_visit_never_changes' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_closed_visit_never_changes

/-- info: 'timeways_rules.chapters.a_tale_changes_only_with_a_visit' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_tale_changes_only_with_a_visit

/-- info: 'timeways_rules.chapters.a_closed_chapter_has_min_weight' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_closed_chapter_has_min_weight

/-- info: 'timeways_rules.chapters.a_rule_change_closes_at_most_one_chapter' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_rule_change_closes_at_most_one_chapter

/-- info: 'timeways_rules.chapters.no_chapter_passes_max_and_one_step' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.no_chapter_passes_max_and_one_step

/-- info: 'timeways_rules.chapters.the_weight_of_a_chapter_is_the_sum_of_its_gains' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.the_weight_of_a_chapter_is_the_sum_of_its_gains

/-- info: 'timeways_rules.chapters.a_step_with_no_gain_closes_nothing' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_step_with_no_gain_closes_nothing

/-- info: 'timeways_rules.chapters.a_repeat_never_adds_weight' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_repeat_never_adds_weight

/-- info: 'timeways_rules.chapters.repeats_alone_never_make_an_entry' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.repeats_alone_never_make_an_entry

/-- info: 'timeways_rules.chapters.entries_grow_only_with_what_is_new' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.entries_grow_only_with_what_is_new

/-- info: 'timeways_rules.chapters.an_instance_step_never_adds_world_weight' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.an_instance_step_never_adds_world_weight

/-- info: 'timeways_rules.chapters.an_instance_has_at_most_one_tale' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.an_instance_has_at_most_one_tale

/-- info: 'timeways_rules.chapters.a_death_to_a_beaten_foe_weighs_nothing' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_death_to_a_beaten_foe_weighs_nothing

/-- info: 'timeways_rules.chapters.deaths_to_one_foe_weigh_at_most_three' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.deaths_to_one_foe_weigh_at_most_three

/-- info: 'timeways_rules.chapters.revenge_counts_once' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.revenge_counts_once

/-- info: 'timeways_rules.chapters.revenge_needs_an_earlier_death' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.revenge_needs_an_earlier_death

/-- info: 'timeways_rules.chapters.a_death_in_the_fold_is_a_step_of_the_log' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_death_in_the_fold_is_a_step_of_the_log

/-- info: 'timeways_rules.chapters.a_death_with_no_killer_weighs_two_then_one_then_nothing' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.a_death_with_no_killer_weighs_two_then_one_then_nothing

/-- info: 'timeways_rules.chapters.deaths_with_no_killer_weigh_at_most_three' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.deaths_with_no_killer_weigh_at_most_three

/-- info: 'timeways_rules.chapters.chapters_cover_the_steps' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.chapters_cover_the_steps

/-- info: 'timeways_rules.chapters.chapters_weigh_between_min_and_max' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.chapters_weigh_between_min_and_max

/-- info: 'timeways_rules.chapters.chapters_have_one_tale_for_each_instance' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.chapters_have_one_tale_for_each_instance

/-- info: 'timeways_rules.chapters.advance_one_line_at_a_time' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms chapters.advance_one_line_at_a_time

/-- info: 'timeways_rules.narrator_shapes.skeleton.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.skeleton.spec

/-- info: 'timeways_rules.narrator_shapes.assemble.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.assemble.spec

/-- info: 'timeways_rules.narrator_shapes.fits.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.fits.spec

/-- info: 'timeways_rules.narrator_shapes.pick.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.pick.spec

/-- info: 'timeways_rules.narrator_shapes.table_ok.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.table_ok.spec

/-- info: 'timeways_rules.narrator_shapes.distinct_skeletons.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.distinct_skeletons.spec

/-- info: 'timeways_rules.narrator_shapes.a_line_is_one_shape' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.a_line_is_one_shape

/-- info: 'timeways_rules.narrator_shapes.the_lore_comes_first' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_lore_comes_first

/-- info: 'timeways_rules.narrator_shapes.the_hero_stands_only_in_a_deed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_hero_stands_only_in_a_deed

/-- info: 'timeways_rules.narrator_shapes.an_arrival_holds_no_hero' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.an_arrival_holds_no_hero

/-- info: 'timeways_rules.narrator_shapes.no_group_clause_holds_the_hero' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.no_group_clause_holds_the_hero

/-- info: 'timeways_rules.narrator_shapes.nothing_is_inside_the_hero' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.nothing_is_inside_the_hero

/-- info: 'timeways_rules.narrator_shapes.nothing_is_inside_the_hero_in_any_case' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.nothing_is_inside_the_hero_in_any_case

/-- info: 'timeways_rules.narrator_shapes.the_hero_is_named_at_most_once' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_hero_is_named_at_most_once

/-- info: 'timeways_rules.narrator_shapes.a_fitting_shape_builds_a_line' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.a_fitting_shape_builds_a_line

/-- info: 'timeways_rules.narrator_shapes.every_slot_has_a_value' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.every_slot_has_a_value

/-- info: 'timeways_rules.narrator_shapes.a_pick_fits' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.a_pick_fits

/-- info: 'timeways_rules.narrator_shapes.no_main_repeats_within_n' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.no_main_repeats_within_n

/-- info: 'timeways_rules.narrator_shapes.a_run_never_repeats' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.a_run_never_repeats

/-- info: 'timeways_rules.narrator_shapes.a_setup_line_ends_on_its_coda' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.a_setup_line_ends_on_its_coda

/-- info: 'timeways_rules.narrator_shapes.pick_preferring.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.pick_preferring.spec

/-- info: 'timeways_rules.narrator_shapes.the_preferring_pick_never_panics' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_preferring_pick_never_panics

/-- info: 'timeways_rules.narrator_shapes.a_fresh_preferred_shape_wins' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.a_fresh_preferred_shape_wins

/-- info: 'timeways_rules.narrator_shapes.the_pick_falls_back_only_when_nothing_before_fits' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_pick_falls_back_only_when_nothing_before_fits

/-- info: 'timeways_rules.narrator_shapes.the_preferring_pick_fits' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_preferring_pick_fits

/-- info: 'timeways_rules.narrator_shapes.window.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.window.spec

/-- info: 'timeways_rules.narrator_shapes.windowSize_is_WINDOW' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.windowSize_is_WINDOW

/-- info: 'timeways_rules.narrator_shapes.the_pick_is_deterministic_from_the_turn' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms narrator_shapes.the_pick_is_deterministic_from_the_turn

/-- info: 'timeways_rules.thin_lore.is_silent.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms thin_lore.is_silent.spec

/-- info: 'timeways_rules.thin_lore.the_rule_never_panics' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms thin_lore.the_rule_never_panics

/-- info: 'timeways_rules.thin_lore.a_deed_with_no_lore_of_its_own_is_silent' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms thin_lore.a_deed_with_no_lore_of_its_own_is_silent

/-- info: 'timeways_rules.thin_lore.a_deed_with_lore_of_its_own_speaks' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms thin_lore.a_deed_with_lore_of_its_own_speaks

/-- info: 'timeways_rules.thin_lore.an_arrival_is_never_silenced_by_this_rule' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms thin_lore.an_arrival_is_never_silenced_by_this_rule

/-- info: 'timeways_rules.outcomes.outcome_usable.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.outcome_usable.spec

/-- info: 'timeways_rules.outcomes.the_gate_never_panics' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.the_gate_never_panics

/-- info: 'timeways_rules.outcomes.an_outcome_the_player_did_not_do_never_reaches_a_prompt' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.an_outcome_the_player_did_not_do_never_reaches_a_prompt

/-- info: 'timeways_rules.outcomes.an_unresolved_outcome_is_never_used' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.an_unresolved_outcome_is_never_used

/-- info: 'timeways_rules.outcomes.a_passage_with_no_outcome_is_not_gated_by_this_rule' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.a_passage_with_no_outcome_is_not_gated_by_this_rule

/-- info: 'timeways_rules.outcomes.an_outcome_the_player_did_passes' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.an_outcome_the_player_did_passes

/-- info: 'timeways_rules.setups.setup_usable.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.setup_usable.spec

/-- info: 'timeways_rules.setups.the_setup_gate_never_panics' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.the_setup_gate_never_panics

/-- info: 'timeways_rules.setups.a_setup_for_a_deed_the_player_did_never_reaches_a_prompt' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.a_setup_for_a_deed_the_player_did_never_reaches_a_prompt

/-- info: 'timeways_rules.setups.a_setup_for_a_deed_not_done_is_not_gated_by_this_rule' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.a_setup_for_a_deed_not_done_is_not_gated_by_this_rule

/-- info: 'timeways_rules.setups.a_passage_with_no_setup_is_not_gated_by_this_rule' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.a_passage_with_no_setup_is_not_gated_by_this_rule

/-- info: 'timeways_rules.instance_lore.next_passage.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms instance_lore.next_passage.spec

/-- info: 'timeways_rules.instance_lore.the_instance_pick_never_panics' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms instance_lore.the_instance_pick_never_panics

/-- info: 'timeways_rules.instance_lore.an_instance_passage_is_told_at_most_once' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms instance_lore.an_instance_passage_is_told_at_most_once

/-- info: 'timeways_rules.instance_lore.entries_are_silent_once_every_passage_is_told' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms instance_lore.entries_are_silent_once_every_passage_is_told

/-- info: 'timeways_rules.game_names.person.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms game_names.person.spec

/-- info: 'timeways_rules.game_names.holds_person.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms game_names.holds_person.spec

/-- info: 'timeways_rules.game_names.person_is_idempotent' depends on axioms: [propext, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms game_names.person_is_idempotent

/-- info: 'timeways_rules.game_names.either_name_of_a_row_names_one_person' depends on axioms: [propext] -/
#guard_msgs (whitespace := lax) in
#print axioms game_names.either_name_of_a_row_names_one_person

/-- info: 'timeways_rules.outcomes.outcomes_usable.spec' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.outcomes_usable.spec

/-- info: 'timeways_rules.outcomes.a_passage_with_two_deeds_waits_for_both' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.a_passage_with_two_deeds_waits_for_both

/-- info: 'timeways_rules.outcomes.a_kill_under_either_name_unlocks_the_outcome' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.a_kill_under_either_name_unlocks_the_outcome

/-- info: 'timeways_rules.outcomes.a_kill_of_another_person_unlocks_nothing' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms outcomes.a_kill_of_another_person_unlocks_nothing

/-- info: 'timeways_rules.setups.a_kill_under_either_name_makes_the_setup_stale' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.a_kill_under_either_name_makes_the_setup_stale

/-- info: 'timeways_rules.setups.a_kill_of_another_person_keeps_the_setup' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs (whitespace := lax) in
#print axioms setups.a_kill_of_another_person_keeps_the_setup
