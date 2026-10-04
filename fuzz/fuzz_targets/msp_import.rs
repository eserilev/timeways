//! The import of your own roleplay profile from Total RP 3 or MyRolePlay (GAMEPLAY.md
//! 3.7.1), through the real addon: random fields of any type in `msp.my`, then journals from
//! the desktop. No Lua error comes of it. The desktop takes every imported text, each text
//! goes at most once while `msp.my` stays the same, and origin and background never change.

#![no_main]

#[path = "any_value.rs"]
mod any_value;
#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;

use any_value::AnyValue;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mlua::{Table, Value};
use std::collections::BTreeSet;
use timeways_story::hero;

/// The codes of the sheet, and two that are not.
const CODES: [&str; 10] = ["HB", "HI", "NA", "NT", "CU", "DE", "AG", "MO", "VA", "GU"];
const FIELDS: [&str; 8] = [
    "origin",
    "background",
    "name",
    "title",
    "currently",
    "appearance",
    "age",
    "motto",
];

#[derive(Arbitrary, Debug)]
enum Action {
    /// The roleplay addon changes one field of `msp.my`.
    Set { code: u8, value: AnyValue },
    /// The roleplay addon puts any value in place of `msp.my`.
    Replace(AnyValue),
    /// A journal from the desktop, with these fields on the sheet.
    Journal(Vec<(u8, String)>),
}

/// The edits of the hero that went to the desktop, as (field, text).
type Sets = Vec<(String, String)>;

fn sheet_reply(fields: &[(u8, String)]) -> String {
    let sheet: Vec<serde_json::Value> = fields
        .iter()
        .map(|(field, text)| {
            let field = FIELDS[usize::from(*field) % FIELDS.len()];
            serde_json::json!({ "field": field, "text": text })
        })
        .collect();
    serde_json::json!({
        "type": "journal",
        "page": 0,
        "pages": 1,
        "hero": { "sheet": sheet, "entries": [] },
        "hero_refused": null,
    })
    .to_string()
}

/// The hero edits in the messages to the desktop after the first `from`. Each line must be
/// JSON that the desktop reads.
fn hero_sets(game: &common::Game, from: usize) -> Sets {
    let sent: Vec<mlua::LuaString> = game.eval("sent");
    let mut sets = Vec::new();
    for message in sent.iter().skip(from) {
        for line in message.as_bytes().split(|byte| *byte == b'\n').skip(1) {
            let value: serde_json::Value = serde_json::from_slice(line).unwrap();
            if value["type"] == "hero_set" {
                let text = |key: &str| value[key].as_str().unwrap().to_string();
                sets.push((text("field"), text("text")));
            }
        }
    }
    sets
}

/// The desktop refuses a text that is too long or holds a control character. An empty text
/// clears the field.
fn assert_the_desktop_takes(field: &str, text: &str) {
    assert!(
        field != "origin" && field != "background",
        "the import changed {field}"
    );
    if text.is_empty() {
        return;
    }
    let limit = hero::limit_of(field).unwrap();
    let checked = hero::checked_text(text, limit);
    assert_eq!(checked.as_deref(), Ok(text), "{field}");
}

fuzz_target!(|actions: Vec<Action>| {
    let game = common::Game::new();
    game.run("msp = { my = {} } msp_RPAddOn = 'Total RP 3'");
    let msp: Table = game.eval("msp");
    let mut sent = 0;
    let mut since_change = BTreeSet::new();
    for action in actions.iter().take(32) {
        match action {
            Action::Set { code, value } => {
                if let Value::Table(my) = msp.get("my").unwrap() {
                    let code = CODES[usize::from(*code) % CODES.len()];
                    my.set(code, value.to_lua(&game.lua)).unwrap();
                }
                since_change.clear();
            }
            Action::Replace(value) => {
                msp.set("my", value.to_lua(&game.lua)).unwrap();
                since_change.clear();
            }
            Action::Journal(fields) => {
                game.reply(&sheet_reply(fields));
                game.run("wow.now = wow.now + 1; wow.RunTickers()");
                for (field, text) in hero_sets(&game, sent) {
                    assert_the_desktop_takes(&field, &text);
                    let new = since_change.insert((field, text));
                    assert!(new, "a text went twice: {since_change:?}");
                }
                sent = game.eval("#sent");
            }
        }
    }
});
