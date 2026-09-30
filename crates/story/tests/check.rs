#![allow(clippy::unwrap_used)]

use hourglass::Tick;
use timeways_story::check::{
    Fault, MAX_CHARS, banned_words, banned_words_in, check, in_voice, later_names, mentions,
    names_after_cutoff, names_in_no_fact, plain_text, voice_text, without_citations,
};
use timeways_story::flavor::{self, Flavor, Kind};
use timeways_story::journal::{Chapter, Deed};
use timeways_story::moments::Moment;
use timeways_story::{chronicle, narrator};

#[test]
fn an_answer_that_cites_a_passage_passes() {
    assert_eq!(check("The tower fell to goblins [1].", 2), []);
}

#[test]
fn an_answer_with_no_citation_fails() {
    assert_eq!(check("The tower fell to goblins.", 2), [Fault::NoCitation]);
}

#[test]
fn nobody_knows_needs_no_citation() {
    assert_eq!(check("Nobody knows who built it.", 2), []);
}

#[test]
fn legend_says_needs_no_citation() {
    assert_eq!(check("Legend says a dragon slept here.", 2), []);
}

#[test]
fn a_citation_past_the_last_passage_fails() {
    assert_eq!(
        check("It fell [3].", 2),
        [Fault::UnknownCitation { number: 3 }]
    );
}

#[test]
fn a_citation_of_zero_fails() {
    assert_eq!(
        check("It fell [0].", 2),
        [Fault::UnknownCitation { number: 0 }]
    );
}

#[test]
fn an_empty_answer_fails() {
    assert_eq!(check("  \n", 2), [Fault::Empty]);
}

#[test]
fn a_runaway_answer_fails() {
    let answer = format!("{} [1]", "a".repeat(MAX_CHARS));

    let faults = check(&answer, 1);

    assert!(matches!(faults.as_slice(), [Fault::TooLong { .. }]));
}

#[test]
fn every_fault_comes_back_at_once() {
    let faults = check("They fled to Shattrath.", 1);

    let name = "Shattrath".to_string();
    assert_eq!(faults, [Fault::NoCitation, Fault::LaterName { name }]);
}

#[test]
fn a_later_name_matches_in_any_case() {
    let faults = check("They fled to SHATTRATH [1].", 1);

    assert_eq!(
        faults,
        [Fault::LaterName {
            name: "Shattrath".to_string()
        }]
    );
}

#[test]
fn every_later_name_in_the_list_is_caught() {
    for name in later_names() {
        let answer = format!("They spoke of {name} [1].");

        let faults = check(&answer, 1);

        assert_eq!(
            faults,
            [Fault::LaterName {
                name: name.to_string()
            }],
            "{name}"
        );
    }
}

#[test]
fn the_list_holds_names_and_no_comments() {
    let names: Vec<&str> = later_names().collect();

    assert!(!names.is_empty());
    assert!(names.iter().all(|name| !name.starts_with('#')));
}

#[test]
fn a_citation_of_several_passages_passes() {
    assert_eq!(check("It fell [1, 2].", 2), []);
}

#[test]
fn an_unknown_number_in_a_list_of_citations_fails() {
    assert_eq!(
        check("It fell [1, 9].", 2),
        [Fault::UnknownCitation { number: 9 }]
    );
}

#[test]
fn brackets_with_words_are_not_citations() {
    assert_eq!(check("It fell [see above].", 2), [Fault::NoCitation]);
}

#[test]
fn a_word_made_from_a_later_name_is_caught_too() {
    assert_eq!(
        names_after_cutoff("The Pandarian monks came."),
        ["Pandaria"]
    );
    assert_eq!(names_after_cutoff("A Shattrathi guard."), ["Shattrath"]);
}

#[test]
fn a_word_that_only_ends_like_a_later_name_is_not_caught() {
    assert!(names_after_cutoff("Xpandaria is no name.").is_empty());
}

#[test]
fn an_answer_of_exactly_the_limit_passes() {
    let answer = format!("{} [1]", "a".repeat(MAX_CHARS - 4));

    let faults = check(&answer, 1);

    assert_eq!(answer.chars().count(), MAX_CHARS);
    assert_eq!(faults, []);
}

#[test]
fn a_plain_text_of_exactly_max_chars_passes() {
    assert_eq!(plain_text("abc", 3, 100), Some("abc".to_string()));
    assert_eq!(plain_text("abcd", 3, 100), None);
}

#[test]
fn a_plain_text_of_exactly_max_bytes_passes() {
    assert_eq!(plain_text("é", 10, 2), Some("é".to_string()));
    assert_eq!(plain_text("é", 10, 1), None);
}

#[test]
fn a_name_counts_as_whole_words_in_any_case() {
    assert!(mentions("Hogger", "hogger"));
    assert!(mentions("They met Old Blue there.", "old blue"));
}

#[test]
fn a_name_that_the_text_lacks_is_not_mentioned() {
    assert!(!mentions("They met Hogger.", "Farley"));
    assert!(!mentions("Hoggers ran.", "Hogger"));
}

#[test]
fn an_empty_name_is_never_mentioned() {
    assert!(!mentions("They met Hogger.", ""));
    assert!(!mentions("They met Hogger.", " !? "));
}

#[test]
fn empty_brackets_before_a_citation_do_not_hide_it() {
    assert_eq!(check("It fell [] [1].", 1), []);
}

#[test]
fn two_citations_in_a_row_both_count() {
    assert_eq!(
        check("It fell [1][3] later.", 2),
        [Fault::UnknownCitation { number: 3 }]
    );
}

#[test]
fn a_plain_line_of_the_narrator_is_in_voice() {
    assert!(in_voice("Our hero reached Ironforge in the snow."));
}

#[test]
fn an_emoji_is_out_of_voice() {
    assert!(!in_voice("Our hero reached level 20 \u{1F389}"));
    assert!(!in_voice("Our hero rested \u{2615}"));
}

#[test]
fn modern_slang_is_out_of_voice() {
    assert_eq!(
        banned_words_in("That was literally epic, guys."),
        ["guys", "epic", "literally"]
    );
}

#[test]
fn the_hourglass_and_its_sand_are_out_of_voice() {
    assert!(!in_voice("The sands of time ran on."));
    assert!(!in_voice("An HOURGLASS turned."));
}

#[test]
fn a_banned_word_counts_only_as_a_whole_word() {
    assert!(in_voice("The Sandfury trolls watched the okra field."));
}

#[test]
fn every_banned_word_in_the_list_is_caught() {
    for banned in banned_words() {
        let text = format!("Our hero heard {banned} at the inn.");

        assert!(!in_voice(&text), "{banned}");
    }
}

#[test]
fn no_banned_word_is_a_word_of_the_facts() {
    let facts = fact_texts().join("\n");

    assert_eq!(banned_words_in(&facts), Vec::<&str>::new(), "{facts}");
}

#[test]
fn a_voice_text_is_a_plain_text_in_voice() {
    assert_eq!(
        voice_text(" Our hero\n rested. ", 50, 200).as_deref(),
        Some("Our hero rested.")
    );
    assert_eq!(
        voice_text("Our hero, like, literally rested.", 50, 200),
        None
    );
}

#[test]
fn a_capital_word_inside_a_sentence_that_no_fact_names_is_logged() {
    let prompt = "The player arrived in Goldshire.";

    let names = names_in_no_fact(
        "Our hero met Varian in Goldshire. Then Varian left.",
        prompt,
    );

    assert_eq!(names, ["Varian"]);
}

#[test]
fn the_first_word_of_a_sentence_and_a_known_name_are_no_unknown_names() {
    let prompt = "The player defeated Hogger for the first time.";

    let names = names_in_no_fact(
        "Hogger fell. Our hero took the ear of HOGGER. I saw.",
        prompt,
    );

    assert!(names.is_empty(), "{names:?}");
}

#[test]
fn the_names_of_the_keepers_of_time_are_after_the_cutoff() {
    for text in [
        "Murozond waited.",
        "The Infinite dragonflight came.",
        "Deep in the Caverns of Time.",
        "Another timeline.",
        "All the timelines.",
    ] {
        assert!(!names_after_cutoff(text).is_empty(), "{text}");
    }
}

#[test]
fn chromie_is_before_the_cutoff_because_she_stands_in_andorhal() {
    assert!(names_after_cutoff("Chromie smiled.").is_empty());
}

/// Each kind of fact that a prompt of the narrator or the chronicle can hold, in words.
fn fact_texts() -> Vec<String> {
    let moments = [
        Moment::Titled {
            title: "Dance Machine".to_string(),
        },
        Moment::FirstKill {
            foe: "Hogger".to_string(),
        },
        Moment::SlainAgain {
            killer: "Hogger".to_string(),
            times: 2,
        },
        Moment::Slapped {
            npc: "Innkeeper Farley".to_string(),
            times: 2,
        },
        Moment::LevelUp { level: 12 },
        Moment::NewZone {
            zone: "Westfall".to_string(),
        },
    ];
    let mut texts: Vec<String> = moments
        .iter()
        .map(|moment| narrator::prompt(moment, None, 0))
        .map(|prompt| fenced_part(&prompt, "The moment:\n"))
        .collect();
    texts.push(fenced_part(
        &chronicle::prompt(&chapter_of_every_deed(), &[], &[], None, &[]),
        "The facts of chapter 1:\n",
    ));
    texts.extend(flavor_kinds().iter().map(describe_flavor));
    texts
}

fn fenced_part(prompt: &str, heading: &str) -> String {
    let start = prompt.find(heading).unwrap() + heading.len();
    let rest = &prompt[start..];
    rest[..rest.find(">>>").unwrap()].to_string()
}

fn chapter_of_every_deed() -> Chapter {
    let at = Tick(1);
    Chapter {
        number: 1,
        began: at,
        ended: at,
        zones: vec!["Westfall".to_string()],
        people: vec!["Gryan Stoutmantle".to_string()],
        deeds: vec![
            Deed::Level {
                from: None,
                to: 5,
                at,
                place: None,
            },
            Deed::Level {
                from: Some(5),
                to: 6,
                at,
                place: None,
            },
            Deed::Defeated {
                foe: "Hogger".to_string(),
                times: 1,
                at,
                place: None,
            },
            Deed::Defeated {
                foe: "Hogger".to_string(),
                times: 2,
                at,
                place: None,
            },
            Deed::Titled {
                title: "Dance Machine".to_string(),
                at,
                place: None,
            },
            Deed::QuestDone {
                title: "The Lost Lantern".to_string(),
                at,
                place: None,
            },
            Deed::Died {
                killer: Some("Hogger".to_string()),
                at,
                place: None,
            },
            Deed::Died {
                killer: None,
                at,
                place: None,
            },
        ],
        left_out: 0,
        prose: None,
        footnotes: Vec::new(),
    }
}

fn flavor_kinds() -> [Kind; 4] {
    [
        Kind::Emote {
            emote: "dance".to_string(),
            target: Some("Innkeeper Farley".to_string()),
        },
        Kind::FellTo {
            cause: "falling".to_string(),
        },
        Kind::Humbled {
            killer: "Cow".to_string(),
            gap: 50,
        },
        Kind::Read {
            title: "The Kingdom of Stormwind".to_string(),
        },
    ]
}

fn describe_flavor(kind: &Kind) -> String {
    let flavor = Flavor {
        at: Tick(1),
        hour: Some(3),
        place: Some("Goldshire".to_string()),
        zone: None,
        kind: kind.clone(),
    };
    flavor::describe(&flavor, 2)
}

#[test]
fn citations_leave_the_answer_in_every_form() {
    let text =
        "[3] The Forsaken took Lordaeron [1]. They built a city below [8, 6], and wait [2,4].";

    assert_eq!(
        without_citations(text),
        "The Forsaken took Lordaeron. They built a city below, and wait."
    );
}

#[test]
fn brackets_that_are_no_citation_stay() {
    let text = "The [Scourge] came [soon], [] and [1a].";

    assert_eq!(without_citations(text), text);
}

#[test]
fn a_capital_word_of_one_letter_is_no_name() {
    let names = names_in_no_fact("Then I left, and A came.", "The facts.");

    assert!(names.is_empty(), "{names:?}");
}
