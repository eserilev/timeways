//! Random names, a random text from the addon, and a random answer of a model, through the
//! alias table (GAMEPLAY.md 5.11). The cut joins back to the same text, the text for a
//! model holds no known name as a word, the player never reads an ID, and nothing panics.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use timeways_rules::aliases::{Alias, Piece, PlayerId, find, learn_all};
use timeways_story::aliases::{
    AliasRow, alias_of, id_pieces, joined, key_of, knows_every_id, plain_joined, text_pieces, unmarked,
    with_names, without_names,
};

#[derive(Arbitrary, Debug)]
struct Line {
    names: Vec<String>,
    /// A text of the addon, with its players marked: "{Ada} came".
    text: String,
    /// What a model wrote back, with IDs: "{P1} came".
    answer: String,
    /// A row of the table as another program can write it.
    row: Vec<u8>,
}

/// The words of a text, as `TaskNames.lua` cuts them.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| c.is_ascii() && !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
}

/// The player never reads an ID.
fn holds_an_id(text: &str) -> bool {
    id_pieces(text)
        .iter()
        .any(|piece| matches!(piece, Piece::Player(_)))
}

fuzz_target!(|line: Line| {
    let marked = unmarked(&line.text);
    let mut table: Vec<Alias> = Vec::new();
    let names: Vec<Alias> = line
        .names
        .iter()
        .chain(&marked.names)
        .filter_map(|name| alias_of(name))
        .collect();
    learn_all(&mut table, &names);

    assert!(!marked.text.contains(['{', '}']), "{:?}", marked.text);
    assert_eq!(plain_joined(&text_pieces(&marked.text, &table)), marked.text);

    let for_a_model = without_names(&table, &marked.text);
    for word in words(&for_a_model) {
        assert!(find(&table, &key_of(word)).is_none(), "{word} in {for_a_model}");
    }
    assert!(knows_every_id(&table, &for_a_model));

    let back = with_names(&table, &for_a_model);
    assert!(!holds_an_id(&back), "{back}");

    assert_eq!(joined(&id_pieces(&line.answer)), line.answer);
    let shown = with_names(&table, &line.answer);
    assert!(!holds_an_id(&shown), "{shown}");

    if let Ok(row) = serde_json::from_slice::<AliasRow>(&line.row) {
        let _ = row.card(PlayerId(table.len()));
    }
});
