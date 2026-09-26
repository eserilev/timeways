use hourglass::Tick;
use timeways_story::hero::{
    Change, Entry, Field, MAX_TEXT_CHARS, checked_text, hero, next_number, portrait,
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
fn a_text_that_breaks_a_rule_gets_a_reason_for_the_player() {
    assert!(
        checked_text(&"a".repeat(MAX_TEXT_CHARS + 1))
            .unwrap_err()
            .starts_with("Not saved:")
    );
    assert!(checked_text("My father sailed to Pandaria.").is_err());
    assert_eq!(
        checked_text("  A farm\n near Goldshire. "),
        Ok("A farm near Goldshire.".to_string())
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
