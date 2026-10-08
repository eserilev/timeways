#![allow(clippy::unwrap_used)]

//! One list of later names (GAMEPLAY.md 5.9, layer 4): the pack builder drops a paragraph
//! with a name of `data/later_names.txt`, and every check of model text refuses it.

use timeways_story::check::{Fault, check, later_names, names_after_cutoff};
use timeways_story::line_check::{Checked, Grounds, LineFault, checked_line};
use timeways_story::moments::Moment;
use timeways_story::narrator::{Telling, Who};
use timeways_story::npc_voice::Asked;
use timeways_story::pack_sources::{Sources, later_pattern};
use timeways_story::{chronicle, summary, tale, talk, zone_history};

const GARROSH: &str =
    "Warchief Garrosh Hellscream raided the Deadmines, and the Defias hold it now.";

fn json(field: &str, text: &str) -> String {
    serde_json::json!({ field: text, "trust": 0 }).to_string()
}

fn deadmines() -> Grounds {
    let moment = Moment::NewZone {
        zone: "The Deadmines".to_string(),
    };
    let lore = "The Defias Brotherhood holds the Deadmines. Grom Hellscream and the Warchief \
        of the Horde never came there.";
    Grounds::of(
        &Telling {
            moment: &moment,
            lore: Some(lore),
            who: &Who::default(),
        },
        0,
    )
}

#[test]
fn a_narrator_line_that_names_garrosh_hellscream_is_refused() {
    let Checked::Refused(faults) = checked_line(GARROSH, &deadmines(), "") else {
        panic!("the line passed");
    };

    assert!(faults.contains(&LineFault::LaterName("Garrosh".to_string())));
}

#[test]
fn a_lore_answer_that_names_garrosh_is_refused() {
    let faults = check(&format!("{GARROSH} [1]"), 1);

    assert_eq!(
        faults,
        [Fault::LaterName {
            name: "Garrosh".to_string()
        }]
    );
}

#[test]
fn a_saga_that_names_garrosh_is_refused() {
    let answer = json("saga", GARROSH);

    assert_eq!(chronicle::checked_saga(&answer, 0, "", "", ""), None);
}

#[test]
fn a_tale_that_names_garrosh_is_refused() {
    let answer = json("tale", GARROSH);

    assert_eq!(tale::checked_tale(&answer, "", "", &[], ""), None);
}

#[test]
fn a_summary_that_names_garrosh_is_refused() {
    let answer = json("summary", GARROSH);

    assert_eq!(summary::checked_summary(&answer, "", "", ""), None);
}

#[test]
fn a_zone_history_that_names_garrosh_is_refused() {
    let answer = json("history", GARROSH);

    assert!(zone_history::checked_history(&answer, "", "", &[], "").is_err());
}

#[test]
fn a_talk_reply_that_names_garrosh_is_refused() {
    let answer = json("say", GARROSH);

    assert_eq!(
        talk::checked_answer(&answer, Asked::NoQuestion, "", ""),
        None
    );
}

#[test]
fn grom_hellscream_is_classic_and_stays_allowed() {
    assert!(names_after_cutoff("Grom Hellscream killed Cenarius in Ashenvale.").is_empty());
}

#[test]
fn every_later_name_that_the_pack_drops_is_refused_by_the_checks() {
    let pack = later_pattern(&Sources::bundled().unwrap())
        .unwrap()
        .unwrap();

    for name in later_names() {
        let text = format!("In those days they spoke of {name} at last.");

        assert!(pack.is_match(&text), "the pack keeps {name}");
        assert_eq!(names_after_cutoff(&text), [name], "a check allows {name}");
    }
}

#[test]
fn a_later_name_with_another_apostrophe_is_still_dropped_and_refused() {
    let pack = later_pattern(&Sources::bundled().unwrap())
        .unwrap()
        .unwrap();
    let text = "The orcs built Hellscream\u{2019}s Reach.";

    assert!(pack.is_match(text));
    assert_eq!(names_after_cutoff(text), ["Hellscream's Reach"]);
}

/// A plain name in the patterns of the pack would drop a paragraph that every check of
/// model text lets through: the two lists would drift apart again.
#[test]
fn no_pattern_of_the_pack_is_a_plain_name() {
    let later = Sources::bundled().unwrap().later;

    for term in &later.terms {
        assert!(
            !is_plain_name(term),
            "{term} belongs in data/later_names.txt"
        );
    }
}

#[test]
fn no_title_or_classic_name_is_also_a_later_name() {
    let later = Sources::bundled().unwrap().later;
    let allowed = later.titles.iter().chain(&later.classic_names);

    for name in allowed {
        assert!(names_after_cutoff(name).is_empty(), "{name}");
    }
}

#[test]
fn the_checks_allow_each_classic_name_that_the_pack_drops() {
    let later = Sources::bundled().unwrap().later;

    for name in &later.classic_names {
        let text = format!("The chronicle names {name} [1].");

        assert_eq!(check(&text, 1), [], "{name}");
    }
}

/// Words that each start with a capital letter, with only spaces and apostrophes between
/// them: a name, never a pattern of prose.
fn is_plain_name(term: &str) -> bool {
    let marks_of_a_name = term
        .chars()
        .all(|c| c.is_alphabetic() || c == ' ' || c == '\'');
    let capital_words = term
        .split([' ', '\''])
        .filter(|word| !word.is_empty())
        .all(|word| word.chars().next().is_some_and(char::is_uppercase));
    marks_of_a_name && capital_words
}
