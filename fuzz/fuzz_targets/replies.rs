//! Random answers of the story program, through the reply writer of the relay, into the
//! real addon. No reply makes a Lua error, and no text from the desktop starts a WoW
//! escape: each `|` from the data comes out doubled.

#![no_main]

#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod game;

use arbitrary::{Arbitrary, Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value, json};

/// Field names of the journal and its entries, so random objects look like real pages.
const KEYS: [&str; 24] = [
    "hero",
    "sheet",
    "entries",
    "field",
    "text",
    "number",
    "chapters",
    "prose",
    "footnotes",
    "places",
    "name",
    "within",
    "people",
    "trust",
    "deeds",
    "kind",
    "title",
    "learned",
    "quests",
    "status",
    "steps",
    "goal",
    "giver",
    "hero_refused",
];

const WORDS: [&str; 8] = [
    "|cffff0000red|r",
    "|Hitem:19019|h[Thunderfury]|h|r",
    "||",
    "é",
    "offered",
    "visit",
    "meet",
    "",
];

fn text(u: &mut Unstructured) -> Result<String> {
    if u.arbitrary()? {
        Ok((*u.choose(&WORDS)?).to_string())
    } else {
        u.arbitrary()
    }
}

fn value(u: &mut Unstructured, depth: u32) -> Result<Value> {
    let leaf = depth >= 4;
    Ok(match u.int_in_range(0..=if leaf { 3 } else { 5 })? {
        0 => Value::Null,
        1 => json!(u.arbitrary::<bool>()?),
        2 => json!(u.int_in_range(-3i64..=1_800_000_000)?),
        3 => json!(text(u)?),
        4 => {
            let len = u.int_in_range(0..=4)?;
            let items: Result<Vec<Value>> = (0..len).map(|_| value(u, depth + 1)).collect();
            Value::Array(items?)
        }
        _ => {
            let mut object = Map::new();
            for _ in 0..u.int_in_range(0..=5)? {
                object.insert((*u.choose(&KEYS)?).to_string(), value(u, depth + 1)?);
            }
            Value::Object(object)
        }
    })
}

/// One answer line of the story program, of any type.
#[derive(Debug)]
struct Answer(String);

impl<'a> Arbitrary<'a> for Answer {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let narrator = if u.arbitrary()? {
            json!(text(u)?)
        } else {
            Value::Null
        };
        let line = match u.int_in_range(0..=3)? {
            0 => json!({"type": "events_seen", "id": 1, "narrator": narrator}),
            1 => json!({"type": "talk_answer", "id": 1, "npc": text(u)?, "text": text(u)?,
                "narrator": narrator}),
            2 => {
                let passages =
                    vec![json!({"text": text(u)?, "source": text(u)?}); u.int_in_range(0..=2)?];
                json!({"type": "lore_answer", "id": 1, "text": text(u)?, "passages": passages,
                    "narrator": narrator})
            }
            _ => {
                let mut page = match value(u, 1)? {
                    Value::Object(page) => page,
                    _ => Map::new(),
                };
                page.insert("type".to_string(), json!("journal"));
                page.insert("id".to_string(), json!(1));
                page.insert("page".to_string(), json!(0));
                page.insert("pages".to_string(), json!(1));
                Value::Object(page)
            }
        };
        Ok(Answer(line.to_string()))
    }
}

/// Every `|` is a doubled one from the data, or a color code or a reset of the addon.
fn has_only_own_escapes(shown: &str) -> bool {
    let bytes = shown.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'|' {
            i += 1;
            continue;
        }
        match bytes.get(i + 1) {
            Some(b'|') | Some(b'r') => i += 2,
            Some(b'c')
                if bytes.len() >= i + 10
                    && bytes[i + 2..i + 10].iter().all(u8::is_ascii_hexdigit) =>
            {
                i += 10
            }
            _ => return false,
        }
    }
    true
}

fuzz_target!(|answer: Answer| {
    let Some(reply) = fake_bridge::game_reply(&answer.0) else {
        return;
    };
    let game = game::Game::new();
    let receive: mlua::Function = game.eval("ns.Link.Receive");
    receive.call::<()>((1, "done", reply.as_str())).unwrap();
    let shown: Vec<String> = game.eval(
        "local out = {}
         for _, text in ipairs(wow.printed) do table.insert(out, text) end
         for _, section in ipairs(ns.Journal.SECTIONS) do
             for _, line in ipairs(ns.Journal.Lines(section)) do
                 table.insert(out, line.text)
                 if line.action then line.action.run() end
             end
         end
         return out",
    );
    for text in shown {
        assert!(has_only_own_escapes(&text), "{text:?} from {reply}");
        assert!(!text.contains('\0'), "{text:?} from {reply}");
    }
});
