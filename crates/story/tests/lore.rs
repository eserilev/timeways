use timeways_story::lore::{Answer, LoreCall, Next};
use timeways_story::pack::{Link, Origin, Passage};
use timeways_story::prompt::Context;

fn passages() -> Vec<Passage> {
    let links = vec![Link::Place("Testvale".to_string())];
    let tower = Passage {
        text: "The tower fell.".to_string(),
        source: "https://example.test/1".to_string(),
        links,
        origin: Origin::Pack,
    };
    vec![tower]
}

fn call() -> LoreCall {
    LoreCall::new("why?", &Context::default(), passages())
}

fn done(next: Next) -> Answer {
    match next {
        Next::Done(answer) => answer,
        Next::Ask(call) => panic!("expected an answer, got a prompt: {}", call.prompt()),
    }
}

fn asked(next: Next) -> LoreCall {
    match next {
        Next::Ask(call) => call,
        Next::Done(answer) => panic!("expected a prompt, got {answer:?}"),
    }
}

#[test]
fn a_good_answer_shows_with_its_passages() {
    let answer = done(call().answered("  It fell to goblins [1].\n"));

    assert_eq!(
        answer,
        Answer {
            text: Some("It fell to goblins.".to_string()),
            passages: passages()
        }
    );
}

#[test]
fn a_bad_answer_gets_one_retry_that_names_the_fault() {
    let retry = asked(call().answered("It fell to goblins."));

    assert!(
        retry.prompt().contains("The answer cites no passage."),
        "{}",
        retry.prompt()
    );
}

#[test]
fn a_good_retry_shows() {
    let retry = asked(call().answered("It fell to goblins."));

    let answer = done(retry.answered("It fell to goblins [1]."));

    assert_eq!(answer.text.as_deref(), Some("It fell to goblins."));
}

#[test]
fn a_second_bad_answer_leaves_the_passages_alone() {
    let retry = asked(call().answered("It fell to goblins."));

    let answer = done(retry.answered("They fled to Shattrath [1]."));

    assert_eq!(
        answer,
        Answer {
            text: None,
            passages: passages()
        }
    );
}

#[test]
fn a_failed_call_leaves_the_passages_alone() {
    let answer = call().failed();

    assert_eq!(
        answer,
        Answer {
            text: None,
            passages: passages()
        }
    );
}
