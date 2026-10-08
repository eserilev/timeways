//! The sentences that tell an end (GAMEPLAY.md 5.10). The sentences come from the wiki
//! pages of the pack, or are invented.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use timeways_story::ends::{Told, tells_a_deed, tells_an_end, told};
use timeways_story::outcome_passages::{Npc, Stance};

fn foes(names: &[&str]) -> Vec<Npc> {
    names
        .iter()
        .map(|name| Npc {
            name: (*name).to_string(),
            stance: Stance::Foe,
        })
        .collect()
}

fn end_of(sentence: &str, names: &[&str]) -> Option<String> {
    let foes = foes(names);
    match told(sentence, &foes, false) {
        Told::EndOf(foe) => Some(foe.name.clone()),
        Told::Deed | Told::Nothing => None,
    }
}

#[test]
fn agents_of_the_horde_who_beheaded_arugal_tell_his_end() {
    let sentence = "Arugal was eventually defeated and beheaded by agents of the Horde.";

    assert_eq!(end_of(sentence, &["Arugal"]), Some("Arugal".to_string()));
}

#[test]
fn alliance_forces_who_killed_vancleef_tell_his_end() {
    let sentence = "VanCleef was considered one of the greatest threats to the kingdom of \
                    Stormwind until he was killed by Alliance forces.";

    assert_eq!(
        end_of(sentence, &["Edwin VanCleef"]),
        Some("Edwin VanCleef".to_string())
    );
}

#[test]
fn soldiers_who_killed_a_foe_tell_a_deed() {
    assert!(tells_a_deed(
        "Alliance soldiers stormed the mine and killed the kingpin."
    ));
}

#[test]
fn a_passive_end_of_a_pronoun_with_no_doer_is_a_deed() {
    let foes = foes(&["Edwin VanCleef"]);

    assert_eq!(told("He was eventually killed.", &foes, false), Told::Deed);
    assert!(tells_a_deed("He was killed."));
}

#[test]
fn a_passive_end_by_another_doer_is_no_deed_of_the_player() {
    assert!(!tells_a_deed(
        "He was killed along with his people by Arthas Menethil during the Culling."
    ));
}

#[test]
fn forces_that_were_destroyed_did_no_deed() {
    assert!(!tells_a_deed(
        "Crushed between two armies, the remaining Dark Iron forces were utterly destroyed."
    ));
}

#[test]
fn the_death_of_a_foe_under_his_game_name_is_his_end() {
    let sentence = "However, after Commander Mograine's mysterious death, he was eventually \
                    replaced by High Commander Goodchilde.";

    assert_eq!(
        end_of(sentence, &["Renault Mograine"]),
        Some("Renault Mograine".to_string())
    );
}

#[test]
fn a_foe_who_kills_another_tells_no_end_of_his_own() {
    let sentence = "Together with Freya, he battled and defeated Therazane the Stonemother.";

    assert_eq!(end_of(sentence, &["Archaedas"]), None);
}

#[test]
fn a_belief_of_a_death_tells_no_end() {
    let sentence = "This came as a shock, as it was previously thought within the Horde that \
                    Dal'rend had been slain decades earlier.";

    assert_eq!(end_of(sentence, &["Dal'rend Blackhand"]), None);
    assert!(!tells_a_deed(sentence));
}

#[test]
fn a_belief_keeps_a_deed_of_adventurers() {
    let sentence = "Although believed dead for some time following an incursion by adventurers \
                    into Uldaman, Archaedas may have been merely disabled.";

    assert_eq!(
        end_of(sentence, &["Archaedas"]),
        Some("Archaedas".to_string())
    );
}

#[test]
fn a_head_asked_for_is_no_end_and_a_head_sent_home_is_one() {
    let foes = foes(&["Balnazzar"]);

    let sent = told(
        "Adventurers sent the Head of Balnazzar to Duke Nicholas Zverenhoff as proof.",
        &foes,
        false,
    );
    let asked = told(
        "Thrall ordered them to retrieve the head of Balnazzar.",
        &foes,
        true,
    );

    assert_eq!(sent, Told::EndOf(&foes[0]));
    assert_eq!(asked, Told::Nothing);
}

#[test]
fn a_clue_is_no_result_and_a_reveal_is_one() {
    assert!(!tells_a_deed(
        "Horde adventurers in Blackrock Spire discovered Blackrock documents signed by Rend."
    ));
    assert!(tells_a_deed(
        "Adventurers confronted the Grand Crusader and uncovered that he was a dreadlord."
    ));
}

#[test]
fn leaders_who_succumbed_and_a_hero_who_fell_in_battle_tell_an_end() {
    assert!(tells_an_end(
        "Although in time the other leaders succumbed, Sally survived."
    ));
    assert!(tells_an_end(
        "Lothar fell in battle at the base of the Spire."
    ));
}

#[test]
fn a_fall_into_a_coma_or_into_love_is_no_end() {
    assert!(!tells_an_end("Malfurion fell into a coma."));
    assert!(!tells_an_end(
        "Zaetar fell in love with Princess Theradras."
    ));
}

#[test]
fn a_result_after_be_tells_no_end() {
    assert!(!tells_an_end(
        "The Crusade believes that outsiders must be destroyed."
    ));
    assert!(tells_an_end("Outsiders were destroyed in the square."));
}
