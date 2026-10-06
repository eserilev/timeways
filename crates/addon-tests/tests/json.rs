#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::Game;
use mlua::Value;

fn encode(game: &Game, object: &str) -> String {
    game.eval(&format!("ns.Json.Encode({object})"))
}

/// The decoded value, or the error text.
fn decode(game: &Game, text: &str) -> Result<Value, String> {
    let decode: mlua::Function = game.eval("ns.Json.Decode");
    let (value, error): (Value, Option<String>) = decode.call(text).unwrap();
    error.map_or(Ok(value), Err)
}

#[test]
fn encode_sorts_the_keys() {
    let game = Game::new();

    assert_eq!(
        encode(&game, "{ zone = 'Z', at = 5, type = 't' }"),
        r#"{"at":5,"type":"t","zone":"Z"}"#
    );
}

#[test]
fn encode_writes_a_boolean_for_the_mark_of_a_fake_line() {
    let game = Game::new();

    assert_eq!(
        encode(&game, "{ dev = true, other = false }"),
        r#"{"dev":true,"other":false}"#
    );
}

#[test]
fn encode_writes_a_table_as_an_object() {
    let game = Game::new();

    assert_eq!(
        encode(&game, "{ spot = { y = 2, map = 1420, x = 1 } }"),
        r#"{"spot":{"map":1420,"x":1,"y":2}}"#
    );
}

#[test]
fn encode_writes_a_list_as_an_array() {
    let game = Game::new();

    assert_eq!(
        encode(&game, "{ paragraphs = { 'One.', 'Two \"quoted\".' } }"),
        r#"{"paragraphs":["One.","Two \"quoted\"."]}"#
    );
}

#[test]
fn encode_escapes_quotes_backslashes_and_control_characters() {
    let game = Game::new();

    let encoded = encode(&game, r#"{ q = 'a"b\\c\n\1' }"#);

    assert_eq!(encoded, r#"{"q":"a\"b\\c\n\u0001"}"#);
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(value["q"], "a\"b\\c\n\u{1}");
}

#[test]
fn encode_keeps_utf8_as_it_is() {
    let game = Game::new();

    assert_eq!(
        encode(&game, "{ zone = \"Quel'Thalas é\" }"),
        "{\"zone\":\"Quel'Thalas é\"}"
    );
}

#[test]
fn encode_refuses_a_fraction() {
    let game = Game::new();

    let result = game.lua.load("ns.Json.Encode({ at = 1.5 })").exec();

    assert!(result.is_err());
}

#[test]
fn decode_reads_what_serde_writes() {
    let game = Game::new();
    let text = serde_json::json!({
        "type": "lore_answer",
        "id": 3,
        "text": "Quel'Thalas é \"quoted\" \u{1}\n[1]",
        "passages": [{ "text": "a", "source": "b" }],
    })
    .to_string();

    decode(&game, &text).unwrap();

    game.lua.globals().set("text", text).unwrap();
    let text: String = game.eval("ns.Json.Decode(text).text");
    let source: String = game.eval("ns.Json.Decode(text).passages[1].source");
    assert_eq!(text, "Quel'Thalas é \"quoted\" \u{1}\n[1]");
    assert_eq!(source, "b");
}

#[test]
fn decode_gives_nil_for_null() {
    let game = Game::new();

    let is_nil: bool = game.eval(r#"ns.Json.Decode('{"text":null}').text == nil"#);

    assert!(is_nil);
}

#[test]
fn decode_refuses_broken_text_without_raising() {
    let game = Game::new();

    for broken in [
        "",
        "{",
        r#"{"a":}"#,
        r#"{"a" 1}"#,
        r#"["a" "b"]"#,
        r#""\x""#,
        r#""\ud800""#,
        "{} x",
        "nope",
    ] {
        assert!(decode(&game, broken).is_err(), "{broken}");
    }
}
