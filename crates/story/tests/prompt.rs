use timeways_story::check::Fault;
use timeways_story::pack::{Link, Origin, Passage};
use timeways_story::prompt::{Context, lore, retry};

fn passage(text: &str) -> Passage {
    let links = vec![Link::Place("Testvale".to_string())];
    Passage {
        text: text.to_string(),
        source: "https://example.test/1".to_string(),
        links,
        origin: Origin::Pack,
    }
}

#[test]
fn a_lore_prompt_numbers_each_passage_from_one() {
    let passages = [passage("The tower fell."), passage("Goblins came.")];

    let prompt = lore("why?", &Context::default(), &passages);

    assert!(
        prompt.contains("[1] The tower fell.\n[2] Goblins came."),
        "{prompt}"
    );
}

#[test]
fn a_lore_prompt_ends_with_the_question() {
    let prompt = lore(
        "why is this tower in ruins?",
        &Context::default(),
        &[passage("x")],
    );

    assert!(
        prompt.ends_with("Question:\n<<<\nwhy is this tower in ruins?\n>>>"),
        "{prompt}"
    );
}

#[test]
fn a_lore_prompt_holds_the_context() {
    let context = Context {
        places: vec!["Mockshire", "Testvale"],
        target: Some("Keeper Stubbs"),
        level: Some(12),
    };

    let prompt = lore("who?", &context, &[passage("x")]);

    assert!(
        prompt.contains("The player stands in: Mockshire, Testvale"),
        "{prompt}"
    );
    assert!(
        prompt.contains("The player looks at: Keeper Stubbs"),
        "{prompt}"
    );
    assert!(prompt.contains("The player is level 12."), "{prompt}");
}

#[test]
fn a_lore_prompt_leaves_out_what_it_does_not_know() {
    let prompt = lore("who?", &Context::default(), &[passage("x")]);

    assert!(!prompt.contains("stands in"), "{prompt}");
    assert!(!prompt.contains("looks at"), "{prompt}");
    assert!(!prompt.contains("level"), "{prompt}");
}

#[test]
fn a_retry_prompt_quotes_the_answer_and_names_each_fault() {
    let faults = [Fault::NoCitation, Fault::UnknownCitation { number: 4 }];

    let prompt = retry("FIRST PROMPT", "It fell [4].", &faults);

    assert!(prompt.starts_with("FIRST PROMPT"), "{prompt}");
    assert!(
        prompt.contains("Your last answer was:\nIt fell [4]."),
        "{prompt}"
    );
    assert!(
        prompt.contains("- The answer cites no passage."),
        "{prompt}"
    );
    assert!(
        prompt.contains("- No passage has the number [4]."),
        "{prompt}"
    );
}

#[test]
fn a_lore_prompt_says_where_the_player_learned_a_passage() {
    let read = Passage {
        source: "the text of \"The Kingdom of Stormwind\"".to_string(),
        origin: Origin::Read,
        ..passage("Long ago, the humans came.")
    };

    let prompt = lore(
        "why?",
        &Context::default(),
        &[read, passage("Goblins came.")],
    );

    let expected = "[1] (The player learned this from the text of \"The Kingdom of Stormwind\".) \
                    Long ago, the humans came.\n[2] Goblins came.";
    assert!(prompt.contains(expected), "{prompt}");
}
