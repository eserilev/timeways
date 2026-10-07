//! The cut of game talk from a wiki paragraph (GAMEPLAY.md 5.10). Every text is invented.

#![allow(clippy::unwrap_used)]

// These tests write whole sentences, so they need no `long`.
#[allow(dead_code)]
mod wiki_dump;

use timeways_story::game_talk::{Cut, cut_game_talk};
use timeways_story::pack_sources::{Sources, from_dump};
use wiki_dump::{article, write_dump};

fn says_mock(sentence: &str) -> bool {
    sentence.contains("mock game")
}

#[test]
fn a_game_sentence_goes_and_the_rest_of_the_paragraph_stays() {
    let paragraph = "The gnolls of Testvale raid its farms. In the mock game, they drop gold. \
                     The farmers of Testvale ask the town guard for help.";

    let cut = cut_game_talk(paragraph, says_mock);

    assert_eq!(
        cut,
        Cut::Trimmed {
            text: "The gnolls of Testvale raid its farms. The farmers of Testvale ask the town \
                   guard for help."
                .to_string(),
            dropped: 1,
        }
    );
}

#[test]
fn a_paragraph_with_no_game_sentence_stays_whole() {
    let paragraph = "The gnolls of Testvale raid its farms. The farmers ask for help.";

    assert_eq!(cut_game_talk(paragraph, says_mock), Cut::Whole);
}

#[test]
fn a_paragraph_with_one_sentence_left_goes() {
    let paragraph = "The gnolls of Testvale raid its farms. In the mock game, they drop gold.";

    assert_eq!(cut_game_talk(paragraph, says_mock), Cut::Dropped);
}

#[test]
fn a_paragraph_whose_next_sentence_points_back_to_the_cut_goes() {
    let paragraph = "The gnolls of Testvale raid its farms. The mock game names Old Fang \
                     their leader. He keeps their gold in a cave. The farmers ask for help.";

    assert_eq!(cut_game_talk(paragraph, says_mock), Cut::Dropped);
}

#[test]
fn a_sentence_that_opens_with_a_back_phrase_cannot_follow_a_cut() {
    let paragraph = "The gnolls of Testvale raid its farms. The mock game moved their camp. \
                     As a result, the farmers left. The guard of Testvale holds the road.";

    assert_eq!(cut_game_talk(paragraph, says_mock), Cut::Dropped);
}

#[test]
fn a_sentence_with_an_early_back_word_cannot_follow_a_cut() {
    let however = "The gnolls of Testvale raid its farms. The mock game moved their camp. \
                   Recently, however, the farmers left. The guard of Testvale holds the road.";
    let such = "The gnolls of Testvale raid its farms. The mock game plays a howl. \
                One such howl woke the farmers. The guard of Testvale holds the road.";

    assert_eq!(cut_game_talk(however, says_mock), Cut::Dropped);
    assert_eq!(cut_game_talk(such, says_mock), Cut::Dropped);
}

#[test]
fn a_cut_at_the_end_of_a_paragraph_keeps_the_sentences_before_it() {
    let paragraph = "The gnolls of Testvale raid its farms. They hold a cave in the hills. \
                     The mock game gives them three camps.";

    let cut = cut_game_talk(paragraph, says_mock);

    assert!(matches!(cut, Cut::Trimmed { dropped: 1, .. }), "{cut:?}");
}

/// The kept passages of an invented page "Ironforge", read with the bundled terms.
fn kept_with_bundled_terms(name: &str, paragraphs: &[&str]) -> Vec<String> {
    let page = format!("Lead.\n==History==\n{}\n", paragraphs.join("\n"));
    let dump = write_dump(
        name,
        &[
            article("History of Warcraft", "Intro."),
            article("Ironforge", &page),
        ],
    );
    let built = from_dump(&dump, &Sources::bundled().unwrap()).unwrap();
    built
        .passages
        .into_iter()
        .map(|passage| passage.text)
        .collect()
}

#[test]
fn a_bundled_game_term_cuts_only_its_sentence() {
    let paragraph = "The dwarves of the test hold carved their halls deep into the stone. \
                     Players can find a flight master near the gate of the hold. \
                     The smiths of the test hold work their forge day and night.";

    let kept = kept_with_bundled_terms("talk-sentence", &[paragraph]);

    assert_eq!(
        kept,
        [
            "The dwarves of the test hold carved their halls deep into the stone. The smiths \
          of the test hold work their forge day and night."
        ]
    );
}

#[test]
fn world_of_warcraft_is_game_talk_only_in_a_frame_of_the_game() {
    let framed = "In World of Warcraft, the test hold is the capital of the dwarves of the north.";
    let named = "The test hold stood long before the events of World of Warcraft took place, \
                 and the dwarves held it through every war.";

    let kept = kept_with_bundled_terms("talk-wow", &[framed, named]);

    assert_eq!(kept, [named]);
}

#[test]
fn a_quest_of_the_lore_stays_and_a_quest_of_the_game_goes() {
    let lore = "The dwarven king began a long quest to find the tomb of his fathers in the deep \
                roads below the test hold.";
    let game = "The quest giver stands by the gate, and the dwarves of the test hold ask \
                adventurers to complete his errand.";

    let kept = kept_with_bundled_terms("talk-quest", &[lore, game]);

    assert_eq!(kept, [lore]);
}

#[test]
fn grom_hellscream_stays_and_garrosh_and_his_places_go() {
    let grom = "Grom Hellscream led the Warsong clan into the forest of the test hold, and his \
                orcs cut its trees for timber.";
    let garrosh = "Garrosh Hellscream led the test hold into a war that the dwarves never wanted \
                   to fight.";
    let watch = "The orcs built Hellscream's Watch on a hill above the test hold, and they held \
                 it against the dwarves.";

    let kept = kept_with_bundled_terms("talk-hellscream", &[grom, garrosh, watch]);

    assert_eq!(kept, [grom]);
}

#[test]
fn a_later_term_still_drops_the_whole_paragraph() {
    let paragraph = "The dwarves of the test hold carved their halls deep into the stone. \
                     During the Cataclysm, the hold lost its eastern gate to a flood. \
                     The smiths of the test hold work their forge day and night.";

    let kept = kept_with_bundled_terms("talk-later", &[paragraph]);

    assert!(kept.is_empty(), "{kept:?}");
}
