//! Past talks (GAMEPLAY.md 3.5) through the real addon: a random `TimewaysTalk` from
//! another addon, then talks, answers, and the talk window. No Lua error comes of it. Each
//! exchange that stays has a time, words, and an answer within their limits, with no
//! control character and no lone `|`, the caps hold, and the window shows no escape.

#![no_main]

#[path = "any_value.rs"]
mod any_value;
#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;

use any_value::AnyValue;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mlua::{Function, Lua, Value};

const NPCS: [&str; 4] = ["Innkeeper Farley", "Marshal Dughan", "Bad|Name", ""];
/// More exchanges than the cap of 200 in all, so a saved table can be too long.
const MAX_FILLER: u16 = 260;

#[derive(Arbitrary, Debug)]
enum Exchange {
    Fields {
        at: f64,
        said: Vec<u8>,
        heard: Vec<u8>,
    },
    Any(AnyValue),
}

#[derive(Arbitrary, Debug)]
enum Saved {
    Talks {
        talks: Vec<(u8, Vec<Exchange>)>,
        filler: u16,
    },
    Any(AnyValue),
}

#[derive(Arbitrary, Debug)]
enum Action {
    /// `/talk` with these words to one of the NPCs.
    Talk {
        npc: u8,
        words: Vec<u8>,
    },
    /// The desktop answers the oldest talk to this NPC, with words or none.
    Answer {
        npc: u8,
        text: Option<Vec<u8>>,
    },
    /// Another addon writes the saved variable again.
    Replace(Saved),
    Goodbye,
}

#[derive(Arbitrary, Debug)]
struct Play {
    saved: Saved,
    actions: Vec<Action>,
}

fn exchange_table(lua: &Lua, at: f64, said: &[u8], heard: &[u8]) -> Value {
    let table = lua.create_table().unwrap();
    table.set("at", at).unwrap();
    table.set("said", lua.create_string(said).unwrap()).unwrap();
    table
        .set("heard", lua.create_string(heard).unwrap())
        .unwrap();
    Value::Table(table)
}

fn talks_table(lua: &Lua, talks: &[(u8, Vec<Exchange>)], filler: u16) -> Value {
    let table = lua.create_table().unwrap();
    for n in 0..filler % MAX_FILLER {
        let list = lua.create_table().unwrap();
        list.set(1, exchange_table(lua, f64::from(n), b"Hi.", b"Hello."))
            .unwrap();
        table.set(format!("Filler {n}"), list).unwrap();
    }
    for (npc, exchanges) in talks {
        let list = lua.create_table().unwrap();
        for (n, exchange) in exchanges.iter().enumerate() {
            let value = match exchange {
                Exchange::Fields { at, said, heard } => exchange_table(lua, *at, said, heard),
                Exchange::Any(value) => value.to_lua(lua),
            };
            list.set(n + 1, value).unwrap();
        }
        table
            .set(NPCS[usize::from(*npc) % NPCS.len()], list)
            .unwrap();
    }
    Value::Table(table)
}

fn saved_value(lua: &Lua, saved: &Saved) -> Value {
    match saved {
        Saved::Talks { talks, filler } => talks_table(lua, talks, *filler),
        Saved::Any(value) => value.to_lua(lua),
    }
}

const TALK: &str = "return function(npc, words)
    wow.units.target = { name = npc }
    SlashCmdList.TIMEWAYSTALK(words)
end";

/// The bridge doubles each `|` of an answer (Gnomish Relay SPEC.md 9.8, S10).
const ANSWER: &str = "return function(npc, text)
    text = text and text:gsub('|', '||')
    ns.Talk.Show({ type = 'talk_answer', npc = npc, text = text })
end";

/// The rules that hold after every action. A text in the window has no lone `|`: each one
/// is doubled, or starts a color code of the addon.
const CHECK: &str = "local function Safe(text)
    return type(text) == 'string' and not text:find('%c') and not text:gsub('||', ''):find('|', 1, true)
end
return function()
    ns.TalkHistory.Of('')
    local count = 0
    for npc, list in pairs(TimewaysTalk) do
        assert(Safe(npc) and npc ~= '' and #npc <= 64, 'odd name')
        assert(#list >= 1 and #list <= ns.TalkHistory.MAX_PER_NPC, 'odd list')
        for _, exchange in ipairs(list) do
            count = count + 1
            assert(type(exchange.at) == 'number' and exchange.at >= 0, 'odd time')
            assert(Safe(exchange.said) and exchange.said ~= '' and #exchange.said <= 255, 'odd words')
            assert(Safe(exchange.heard) and exchange.heard ~= '' and #exchange.heard <= 3200, 'odd answer')
        end
    end
    assert(count <= ns.TalkHistory.MAX_TOTAL, 'more exchanges than the cap')
    if TimewaysTalkFrameScroll then
        for _, text in ipairs(wow.ShownTexts(TimewaysTalkFrameScroll:GetScrollChild())) do
            assert(not text:gsub('||', ''):find('|', 1, true) or text:find('^You: '), 'an escape in the window')
        end
    end
end";

fn act(game: &common::Game, action: &Action) {
    let lua = &game.lua;
    match action {
        Action::Talk { npc, words } => {
            let talk: Function = game.eval(TALK);
            let npc = NPCS[usize::from(*npc) % NPCS.len()];
            talk.call::<()>((npc, lua.create_string(words).unwrap()))
                .unwrap();
        }
        Action::Answer { npc, text } => {
            let answer: Function = game.eval(ANSWER);
            let npc = NPCS[usize::from(*npc) % NPCS.len()];
            let text = text.as_ref().map(|text| lua.create_string(text).unwrap());
            answer.call::<()>((npc, text)).unwrap();
        }
        Action::Replace(saved) => {
            lua.globals()
                .set("TimewaysTalk", saved_value(lua, saved))
                .unwrap();
        }
        Action::Goodbye => game.run("if TimewaysTalkFrame then TimewaysTalkFrame:Hide() end"),
    }
}

fuzz_target!(|play: Play| {
    let game = common::Game::new();
    game.lua
        .globals()
        .set("TimewaysTalk", saved_value(&game.lua, &play.saved))
        .unwrap();
    let check: Function = game.eval(CHECK);
    check.call::<()>(()).unwrap();
    for action in play.actions.iter().take(64) {
        act(&game, action);
        check.call::<()>(()).unwrap();
    }
});
