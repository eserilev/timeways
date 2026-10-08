#![allow(clippy::unwrap_used)]

//! No invented names (GAMEPLAY.md 3.2.1): every proper name of a model text is a word of
//! what the prompt gave the model. Found by the local-model bench: "Deuce Waterman still
//! holds The Deadmines" passed with no fact that names him.

use timeways_story::check::Fault;
use timeways_story::grounding::{given_text, ungrounded_names};
use timeways_story::house::fenced;
use timeways_story::line_check::{Checked, Grounds, LineFault, checked_line};
use timeways_story::lore::{LoreCall, Next};
use timeways_story::moments::Moment;
use timeways_story::narrator::{Telling, Who};
use timeways_story::pack::{Link, Origin, Passage};
use timeways_story::prompt::{self, Context};
use timeways_story::samples::{self, Voice};
use timeways_story::{chronicle, summary, tale, talk, zone_history};

const DEADMINES_LORE: &str = "The Defias Brotherhood holds the Deadmines. Edwin VanCleef \
    built it from the stonemasons that Stormwind never paid, and Captain Greenskin sails \
    his ship there.";

const INVENTED: &str = "The Defias hold the mine, and Deuce Waterman still holds The Deadmines.";

fn json(field: &str, text: &str) -> String {
    serde_json::json!({ field: text, "trust": 0 }).to_string()
}

fn deadmines() -> Grounds {
    let moment = Moment::NewZone {
        zone: "The Deadmines".to_string(),
    };
    Grounds::of(
        &Telling {
            moment: &moment,
            lore: Some(DEADMINES_LORE),
            who: &Who::default(),
        },
        0,
    )
}

#[test]
fn a_narrator_line_with_an_invented_person_is_refused() {
    let Checked::Refused(faults) = checked_line(INVENTED, &deadmines(), "") else {
        panic!("the line passed");
    };

    assert_eq!(
        faults,
        [LineFault::UngroundedName("Deuce Waterman".to_string())]
    );
}

#[test]
fn a_narrator_line_whose_names_are_all_given_passes() {
    let line = "Captain Greenskin sails for the Brotherhood that VanCleef founded. The Defias \
        hold the Deadmines now.";

    assert_eq!(
        checked_line(line, &deadmines(), ""),
        Checked::Line(line.to_string())
    );
}

#[test]
fn the_reason_of_an_ungrounded_name_says_what_to_do() {
    let reason = LineFault::UngroundedName("Deuce Waterman".to_string()).to_string();

    assert!(reason.contains("Deuce Waterman"), "{reason}");
    assert!(reason.contains("no fact or lore"), "{reason}");
}

fn deadmines_lore_call() -> LoreCall {
    let passage = Passage {
        text: DEADMINES_LORE.to_string(),
        source: "the wiki page \"The Deadmines\"".to_string(),
        links: vec![Link::Place("Westfall".to_string())],
        origin: Origin::Pack,
        about: None,
        depends_on: Vec::new(),
        setup_for: None,
    };
    LoreCall::new(
        "Who holds the Deadmines?",
        &Context::default(),
        vec![passage],
    )
}

#[test]
fn a_lore_answer_with_an_invented_person_gets_a_retry_that_names_him() {
    let next = deadmines_lore_call().answered("Deuce Waterman holds the Deadmines [1].");

    let Next::Ask(retry) = next else {
        panic!("the answer passed");
    };
    assert!(retry.prompt().contains("Deuce Waterman"));
}

#[test]
fn a_lore_answer_whose_names_are_all_given_shows() {
    let next = deadmines_lore_call().answered("The Defias Brotherhood holds it [1].");

    assert!(matches!(next, Next::Done(answer) if answer.text.is_some()));
}

#[test]
fn the_fault_of_a_lore_answer_names_the_invented_name() {
    let fault = Fault::UngroundedName {
        name: "Deuce Waterman".to_string(),
    };

    assert!(fault.to_string().contains("no fact or lore"));
}

#[test]
fn a_saga_with_an_invented_person_is_refused() {
    let answer = json("saga", INVENTED);

    assert_eq!(
        chronicle::checked_saga(&answer, 0, "", "", DEADMINES_LORE),
        None
    );
}

#[test]
fn a_tale_with_an_invented_person_is_refused() {
    let answer = json("tale", INVENTED);

    assert_eq!(
        tale::checked_tale(&answer, "", "", &[], DEADMINES_LORE),
        None
    );
}

#[test]
fn a_summary_with_an_invented_person_is_refused() {
    let answer = json("summary", INVENTED);

    assert_eq!(
        summary::checked_summary(&answer, "", "", DEADMINES_LORE),
        None
    );
}

#[test]
fn a_zone_history_with_an_invented_person_is_refused_with_its_name() {
    let answer = json("history", INVENTED);

    let refused = zone_history::checked_history(&answer, "", "", &[], DEADMINES_LORE);

    let reasons = refused.unwrap_err();
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("Deuce Waterman"))
    );
}

#[test]
fn a_talk_reply_with_an_invented_person_is_refused() {
    let answer = json("say", "Aye, Deuce Waterman runs the mine now.");

    assert_eq!(talk::checked_answer(&answer, "", DEADMINES_LORE), None);
}

#[test]
fn a_talk_reply_that_names_only_what_the_npc_knows_passes() {
    let answer = json(
        "say",
        "Aye, the Defias hold the Deadmines. VanCleef leads them.",
    );

    assert!(talk::checked_answer(&answer, "", DEADMINES_LORE).is_some());
}

#[test]
fn the_given_text_is_every_fence_except_the_samples() {
    let samples = samples::section(Voice::Summary, 0);
    let prompt = format!(
        "Rules about Westfall.\n\nThe facts:\n{}\n\n{samples}\n\nThe hero:\n{}",
        fenced("The Defias hold the Deadmines."),
        fenced("A dwarf of Ironforge.")
    );

    let given = given_text(&prompt);

    assert_eq!(
        given,
        "The Defias hold the Deadmines.\nA dwarf of Ironforge."
    );
}

#[test]
fn a_name_of_a_golden_sample_is_not_given() {
    let samples = samples::section(Voice::Chapter, 0);
    let prompt = format!("{samples}\n\nThe facts:\n{}", fenced("Hogger is dead."));
    let sample_name = "Thistlenettle";
    assert!(
        Voice::Chapter
            .samples()
            .iter()
            .chain(&Voice::NarratorLine.samples())
            .any(|sample| sample.contains("Deadmines")),
        "the samples changed"
    );

    let given = given_text(&prompt);

    assert!(!given.contains("Deadmines"), "{given}");
    assert_eq!(
        ungrounded_names(&format!("Foreman {sample_name} is dead."), &given),
        [sample_name]
    );
}

#[test]
fn the_answer_of_a_retry_is_not_given() {
    let first = format!("The facts:\n{}", fenced("Hogger is dead."));
    let retry = prompt::retry(
        &first,
        "Deuce Waterman is dead.",
        &["A reason.".to_string()],
    );

    assert_eq!(given_text(&retry), "Hogger is dead.");
}

#[test]
fn the_last_word_of_a_given_name_is_given() {
    let given = "Edwin VanCleef leads the Defias.";

    assert!(ungrounded_names("The Defias follow VanCleef.", given).is_empty());
}

#[test]
fn a_title_before_a_given_name_is_allowed() {
    let given = "Greenskin sails for the Defias.";

    assert!(ungrounded_names("The Defias obey Captain Greenskin.", given).is_empty());
}

#[test]
fn a_known_last_name_does_not_ground_an_invented_first_name() {
    let names = ungrounded_names(
        "The orcs follow Durak Hellscream now.",
        "Grom Hellscream killed Cenarius.",
    );

    assert_eq!(names, ["Durak Hellscream"]);
}

#[test]
fn a_name_with_an_apostrophe_is_one_word() {
    let given = "Gath'Ilzogg leads the Blackrock orcs.";

    assert!(ungrounded_names("The orcs obey Gath\u{2019}Ilzogg.", given).is_empty());
    assert_eq!(
        ungrounded_names("The orcs obey Gath'Ilzorn.", given),
        ["Gath'Ilzorn"]
    );
}

#[test]
fn a_possessive_is_the_name() {
    let given = "Mor'Ladim haunts Duskwood.";

    assert!(ungrounded_names("The dead walk in Mor'Ladim's Duskwood.", given).is_empty());
}

#[test]
fn a_hyphen_splits_a_word_into_names() {
    assert!(ungrounded_names("The Stormwind-born guards hold it.", "Stormwind").is_empty());
    assert_eq!(
        ungrounded_names("The Stormwind-Varn guards hold it.", "Stormwind"),
        ["Stormwind-Varn"]
    );
}

#[test]
fn a_name_at_the_start_of_a_sentence_counts_when_a_name_follows() {
    assert_eq!(
        ungrounded_names("Deuce Waterman leads them.", "The Defias."),
        ["Deuce Waterman"]
    );
}

#[test]
fn a_capital_word_inside_a_sentence_that_no_fact_names_is_ungrounded() {
    let given = "The player arrived in Goldshire.";

    let names = ungrounded_names("$N met Varian in Goldshire. Then Varian left.", given);

    assert_eq!(names, ["Varian"]);
}

#[test]
fn a_given_name_in_another_case_is_grounded() {
    let given = "The player defeated Hogger for the first time.";

    let names = ungrounded_names("Hogger fell. $N took the ear of HOGGER. I saw.", given);

    assert!(names.is_empty(), "{names:?}");
}

#[test]
fn a_capital_word_of_one_letter_is_no_name() {
    let names = ungrounded_names("Then I left, and A came.", "The facts.");

    assert!(names.is_empty(), "{names:?}");
}

#[test]
fn common_words_of_the_world_need_no_fact() {
    let line = "The Light abandoned the Scourge after the Second War, and the Horde and the \
        Alliance remember it on Sunday. The dwarves and a Forsaken warlock agree.";

    assert!(ungrounded_names(line, "").is_empty());
}
