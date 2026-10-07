//! Replies into the real addon: random answers through the reply writer of the relay, or
//! the replies of a real session through the fake bridge. No reply makes a Lua error, and
//! no text from the desktop starts a WoW escape: each `|` from the data comes out doubled.

#![no_main]

#[path = "common.rs"]
mod common;
#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod game;
#[path = "play.rs"]
mod play;

use arbitrary::{Arbitrary, Result, Unstructured};
use libfuzzer_sys::fuzz_target;
use serde_json::{Map, Value, json};

/// Field names of the journal and its entries, so random objects look like real pages.
const KEYS: [&str; 47] = [
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
    "talk_quest",
    "npc",
    "at",
    "state",
    "line",
    "lore",
    "about",
    "more",
    "histories",
    "zone",
    "zones",
    "spot",
    "map",
    "x",
    "y",
    "place",
    "first_visit",
    "first_met",
    "excerpt",
    "foe",
    "killer",
    "times",
    "began",
];

const WORDS: [&str; 11] = [
    "|cffff0000red|r",
    "|Hitem:19019|h[Thunderfury]|h|r",
    "||",
    "é",
    "offered",
    "visit",
    "meet",
    "",
    "writing",
    "refused",
    "Innkeeper Farley",
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

/// The start of a link of the journal, which the addon writes itself (`JournalLinks.lua`).
const OWN_LINK: &[u8] = b"|Htimeways:";

/// The end of the data of an own link at `i`, which holds no `|`, or None.
fn own_link_end(bytes: &[u8], i: usize) -> Option<usize> {
    let rest = bytes.get(i..)?;
    if !rest.starts_with(OWN_LINK) {
        return None;
    }
    let data = &rest[OWN_LINK.len()..];
    let close = data.iter().position(|byte| *byte == b'|')?;
    data[close..]
        .starts_with(b"|h")
        .then_some(i + OWN_LINK.len() + close + 2)
}

/// Every `|` is a doubled one from the data, or a color code, a reset, or a link of the
/// addon.
fn has_only_own_escapes(shown: &str) -> bool {
    let bytes = shown.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'|' {
            i += 1;
            continue;
        }
        if let Some(end) = own_link_end(bytes, i) {
            i = end;
            continue;
        }
        match bytes.get(i + 1) {
            Some(b'|') | Some(b'r') | Some(b'h') => i += 2,
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

/// A reply of any shape, as from a broken desktop, or the replies of a real session.
#[derive(Arbitrary, Debug)]
enum Input {
    Random(Answer),
    Session(play::Run),
}

/// The replies that the addon gets for this input.
fn replies(input: Input) -> Vec<String> {
    match input {
        Input::Random(answer) => fake_bridge::game_reply(&answer.0).into_iter().collect(),
        Input::Session(run) => {
            let mut played = play::play(run, play::story(common::pack()));
            let pages = play::journal_pages(&mut played.bridge, played.character);
            played.replies.into_iter().chain(pages).collect()
        }
    }
}

fuzz_target!(|input: Input| {
    let replies = replies(input);
    if replies.is_empty() {
        return;
    }
    let game = game::Game::new();
    // An open talk window shows the answers and the quest of a talk too.
    game.run("wow.units.target = { name = 'Innkeeper Farley' }; wow.Slash('/talk', 'hi')");
    let receive: mlua::Function = game.eval("ns.Link.Receive");
    for (id, reply) in (1..).zip(&replies) {
        receive.call::<()>((id, "done", reply.as_str())).unwrap();
    }
    let shown: Vec<String> = game.eval(
        "local out = {}
         for _, text in ipairs(wow.printed) do table.insert(out, text) end
         for _, section in ipairs(ns.Journal.SECTIONS) do
             local page = ns.Journal.Page(section)
             for _, row in ipairs(page.list or {}) do
                 table.insert(out, row.text)
                 table.insert(out, row.detail or '')
                 table.insert(out, row.mark or '')
             end
             for _, line in ipairs(page.lines) do
                 table.insert(out, line.text)
                 table.insert(out, line.detail or '')
                 if line.action then line.action.run() end
             end
             -- Each link and each pin opens its page, which draws with no error.
             for _, line in ipairs(page.lines) do
                 for _, link in ipairs(ns.JournalLinks.InText(line.text)) do
                     ns.JournalLinks.Open(link)
                     for _, opened in ipairs(ns.Journal.Lines('knowledge')) do
                         table.insert(out, opened.text)
                     end
                 end
             end
             for _, pin in ipairs(page.pins or {}) do
                 if pin.link then ns.JournalLinks.Open(pin.link) end
                 table.insert(out, pin.title or '')
                 table.insert(out, pin.text or '')
             end
             table.insert(out, page.footer)
             table.insert(out, page.crumb or '')
             for _, button in ipairs(page.buttons) do button.run() end
         end
         for _, text in ipairs(wow.ShownTexts(TimewaysTalkFrameScroll:GetScrollChild())) do
             table.insert(out, text)
         end
         table.insert(out, TimewaysTalkFrame.title.text or '')
         return out",
    );
    for text in shown {
        assert!(has_only_own_escapes(&text), "{text:?} from {replies:?}");
        assert!(!text.contains('\0'), "{text:?} from {replies:?}");
    }
});
