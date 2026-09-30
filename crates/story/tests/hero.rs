use hourglass::Tick;
use timeways_story::hero::{
    Change, Entry, Field, MAX_TEXT_BYTES, MAX_TEXT_CHARS, PROMPT_TEXT_CHARS, checked_text, hero,
    newest_texts, next_number, player_text, portrait,
};

fn set(field: &str, text: &str) -> Change {
    Change::Set {
        at: Tick(1),
        field: field.to_string(),
        text: text.to_string(),
    }
}

fn added(number: u64, text: &str) -> Change {
    Change::Added(Entry {
        number,
        at: Tick(number),
        text: text.to_string(),
        place: None,
        npc: None,
    })
}

fn field(field: &str, text: &str) -> Field {
    Field {
        field: field.to_string(),
        text: text.to_string(),
    }
}

#[test]
fn the_last_text_of_a_field_stands_and_an_empty_one_clears_it() {
    let changes = [
        set("goal", "Find my brother."),
        set("origin", "A farm."),
        set("goal", "Avenge him."),
        set("origin", ""),
    ];

    assert_eq!(hero(&changes).sheet, [field("goal", "Avenge him.")]);
}

#[test]
fn the_sheet_keeps_the_order_of_the_page() {
    let changes = [
        set("traits", "Loud."),
        set("origin", "A farm."),
        set("flaw", "Too trusting."),
    ];

    let order: Vec<String> = hero(&changes)
        .sheet
        .into_iter()
        .map(|field| field.field)
        .collect();

    assert_eq!(order, ["origin", "flaw", "traits"]);
}

#[test]
fn a_removed_entry_is_gone_and_its_number_is_never_used_again() {
    let changes = [
        added(1, "A"),
        added(2, "B"),
        Change::Removed {
            at: Tick(3),
            number: 2,
        },
    ];

    let texts: Vec<String> = hero(&changes)
        .entries
        .into_iter()
        .map(|entry| entry.text)
        .collect();

    assert_eq!(texts, ["A"]);
    assert_eq!(next_number(&changes), 3);
    assert_eq!(next_number(&[]), 1);
}

#[test]
fn a_text_goes_on_one_line() {
    assert_eq!(
        checked_text("  A farm\n near Goldshire. "),
        Ok("A farm near Goldshire.".to_string())
    );
}

#[test]
fn the_players_own_text_is_never_checked_for_its_words() {
    assert_eq!(
        checked_text("My father sailed to Pandaria."),
        Ok("My father sailed to Pandaria.".to_string())
    );
}

#[test]
fn a_text_holds_up_to_a_thousand_characters() {
    assert!(checked_text(&"a".repeat(MAX_TEXT_CHARS)).is_ok());

    let reason = checked_text(&"a".repeat(MAX_TEXT_CHARS + 1)).unwrap_err();

    assert_eq!(
        reason,
        "Couldn't save that: it's too long. Try a shorter version."
    );
}

#[test]
fn a_text_outside_ascii_stops_at_the_byte_limit() {
    let widest = "\u{10348}".repeat(MAX_TEXT_BYTES / 4);

    assert!(checked_text(&widest).is_ok());
    assert!(checked_text(&format!("{widest}a")).is_err());
}

#[test]
fn a_text_full_of_quotes_is_too_long_for_the_first_journal_page() {
    let quotes = "\"".repeat(700);

    assert!(checked_text(&quotes).is_err());
}

#[test]
fn an_empty_note_gets_its_own_reason() {
    assert_eq!(
        checked_text("  \n "),
        Err("Couldn't save an empty note.".to_string())
    );
}

#[test]
fn a_control_character_is_refused() {
    assert!(
        checked_text("A farm\u{7}.")
            .unwrap_err()
            .contains("special characters")
    );
}

#[test]
fn a_prompt_takes_the_start_of_a_long_text() {
    let long = "b".repeat(MAX_TEXT_CHARS);
    let changes = [set("goal", &long), added(1, &long)];
    let cut = "b".repeat(PROMPT_TEXT_CHARS);

    let portrait = portrait(&hero(&changes)).unwrap();

    assert_eq!(
        portrait,
        format!("- goal: {cut}\n- Told by the player: {cut}")
    );
    assert_eq!(newest_texts(&hero(&changes).entries, |_| true), [cut]);
}

#[test]
fn the_player_text_holds_the_sheet_and_every_entry() {
    let changes = (1..=7).map(|n| added(n, &format!("Entry {n}")));
    let mut changes: Vec<Change> = changes.collect();
    changes.push(set("origin", "Pandaria."));

    let text = player_text(&hero(&changes));

    assert!(
        text.contains("Pandaria.") && text.contains("Entry 1"),
        "{text}"
    );
}

#[test]
fn a_portrait_holds_the_sheet_and_the_five_newest_entries() {
    let mut changes = vec![set("goal", "Find my brother.")];
    changes.extend((1..=7).map(|n| added(n, &format!("Entry {n}"))));

    let portrait = portrait(&hero(&changes)).unwrap();

    assert!(
        portrait.starts_with("- goal: Find my brother.\n- Told by the player: Entry 7"),
        "{portrait}"
    );
    assert!(
        portrait.contains("Entry 3") && !portrait.contains("Entry 2"),
        "{portrait}"
    );
}

#[test]
fn an_empty_story_has_no_portrait() {
    assert_eq!(portrait(&hero(&[])), None);
}
