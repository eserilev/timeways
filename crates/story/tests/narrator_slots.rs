use timeways_story::narrator_slots::{Parsed, SlotFault, Tone, parse};
use timeways_story::narrator_templates::{Kind, Number};

const KILL: &str = r#"{"lore": "Hogger led the Riverpaw.", "there": false, "leads": "the Riverpaw", "leads_number": "many"}"#;

fn lore_of(parsed: Result<Parsed, SlotFault>) -> String {
    match parsed {
        Ok(Parsed::Answer { lore, .. }) => lore,
        other => panic!("expected an answer, got {other:?}"),
    }
}

#[test]
fn a_kill_answer_gives_its_lore_and_its_choices() {
    let Ok(Parsed::Answer { lore, choices }) = parse(KILL, Kind::Kill, &[]) else {
        panic!("no answer");
    };

    assert_eq!(lore, "Hogger led the Riverpaw.");
    assert_eq!(choices.there, Some(false));
    assert_eq!(choices.leads.as_deref(), Some("the Riverpaw"));
    assert_eq!(choices.leads_number, Some(Number::Many));
}

#[test]
fn silence_comes_bare_or_as_json() {
    assert_eq!(parse("SILENCE", Kind::Kill, &[]), Ok(Parsed::Silence));
    assert_eq!(
        parse(r#"{"lore": "SILENCE"}"#, Kind::Kill, &[]),
        Ok(Parsed::Silence)
    );
}

#[test]
fn one_code_fence_is_allowed_and_two_are_not() {
    let fenced = format!("```json\n{KILL}\n```");
    let twice = format!("```\n{fenced}\n```");

    assert_eq!(
        lore_of(parse(&fenced, Kind::Kill, &[])),
        "Hogger led the Riverpaw."
    );
    assert_eq!(parse(&twice, Kind::Kill, &[]), Err(SlotFault::BadAnswer));
}

#[test]
fn an_unknown_a_missing_or_a_doubled_field_is_a_bad_answer() {
    let unknown = r#"{"lore": "Hogger fell.", "verb": "fell"}"#;
    let missing = r#"{"lore": "Hogger fell.", "there": false}"#;
    let doubled =
        r#"{"lore": "A.", "lore": "B.", "there": false, "leads": "none", "leads_number": "one"}"#;

    assert_eq!(
        parse(unknown, Kind::Arrival, &[]),
        Err(SlotFault::BadAnswer)
    );
    assert_eq!(parse(missing, Kind::Kill, &[]), Err(SlotFault::BadAnswer));
    assert_eq!(parse(doubled, Kind::Kill, &[]), Err(SlotFault::BadAnswer));
}

#[test]
fn a_wrong_type_is_a_bad_answer_and_a_value_off_the_list_is_unknown() {
    let string_true = r#"{"lore": "A.", "there": "true", "leads": "none", "leads_number": "one"}"#;
    let off_list = r#"{"lore": "A.", "tone": "witty"}"#;
    let other_case = r#"{"lore": "A.", "group": "Silver_Hand"}"#;
    let groups = ["o.silver_hand".to_string()];

    assert_eq!(
        parse(string_true, Kind::Kill, &[]),
        Err(SlotFault::BadAnswer)
    );
    assert_eq!(
        parse(off_list, Kind::Revenge, &[]),
        Err(SlotFault::UnknownChoice("tone"))
    );
    assert_eq!(
        parse(other_case, Kind::Level, &groups),
        Err(SlotFault::UnknownChoice("group"))
    );
}

#[test]
fn trailing_text_or_two_objects_are_a_bad_answer() {
    let trailing = format!("{KILL} That is all.");
    let two = format!("{KILL}{KILL}");

    assert_eq!(parse(&trailing, Kind::Kill, &[]), Err(SlotFault::BadAnswer));
    assert_eq!(parse(&two, Kind::Kill, &[]), Err(SlotFault::BadAnswer));
}

#[test]
fn a_dry_tone_parses() {
    let dry = r#"{"lore": "A.", "tone": "dry"}"#;

    let Ok(Parsed::Answer { choices, .. }) = parse(dry, Kind::Revenge, &[]) else {
        panic!("no answer");
    };

    assert_eq!(choices.tone, Some(Tone::Dry));
}
