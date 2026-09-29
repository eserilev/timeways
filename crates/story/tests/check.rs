use timeways_story::check::{
    Fault, MAX_CHARS, check, later_names, mentions, names_after_cutoff, plain_text,
};

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
