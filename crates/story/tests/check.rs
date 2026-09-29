use timeways_story::check::{Fault, MAX_CHARS, check, later_names, names_after_cutoff};

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
