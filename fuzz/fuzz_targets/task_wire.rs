//! Random addon messages through TaskChunks.lua and TaskWire.lua of the addon, in Lua 5.1
//! (GAMEPLAY.md 4.7). A broken part or message is dropped, never a Lua error. A message that
//! passes holds only clean, bounded text, and comes through the wire again unchanged. Each
//! part of it again is plain text that the logged addon channel takes.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mlua::{Function, Lua};

#[derive(Arbitrary, Debug)]
struct Traffic {
    /// Parts of messages, each from one of four senders, at a time in seconds.
    parts: Vec<(u8, u8, Vec<u8>)>,
}

thread_local! {
    static CHECK: (Lua, Function) = {
        let lua = Lua::new();
        let ns = lua.create_table().unwrap();
        for (name, source) in [
            ("TaskWire.lua", include_str!("../../addon/Timeways/TaskWire.lua")),
            ("TaskChunks.lua", include_str!("../../addon/Timeways/TaskChunks.lua")),
        ] {
            lua.load(source).set_name(name).call::<()>(("Timeways", ns.clone())).unwrap();
        }
        lua.globals().set("ns", ns).unwrap();
        let check: Function = lua
            .load(
                r#"
                local collector = ns.TaskChunks.NewCollector()
                local LIMIT = 400
                local function Clean(value)
                    if type(value) == "string" then
                        assert(#value <= LIMIT and not value:find("[%c|]"), "unclean text")
                    elseif type(value) == "table" then
                        for _, item in pairs(value) do
                            Clean(item)
                        end
                    end
                end
                return function(sender, at, part)
                    local whole = ns.TaskChunks.Add(collector, sender, part, at)
                    local message = whole and ns.TaskWire.Decode(whole)
                    if not message then
                        return
                    end
                    Clean(message)
                    local text = ns.TaskWire.Encode(message)
                    local again = assert(ns.TaskWire.Decode(text), "refused its own text")
                    assert(ns.TaskWire.Encode(again) == text, "not the same text")
                    return ns.TaskChunks.Split(text, 1)
                end"#,
            )
            .eval()
            .unwrap();
        (lua, check)
    };
}

/// The logged channel takes valid UTF-8 with no control character, no `|`, no `\`, no
/// U+FFFE or U+FFFF, and no sign that Blizzard bans (rules of the Chomp library).
fn is_loggable(part: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(part) else {
        return false;
    };
    !text.chars().any(|c| {
        matches!(
            c,
            '\0'..='\u{1F}' | '\u{7F}' | '|' | '\\' | '\u{FFFE}' | '\u{FFFF}' | '卍' | '卐'
        )
    })
}

fuzz_target!(|traffic: Traffic| {
    CHECK.with(|(lua, check)| {
        let mut at = 0u32;
        for (sender, step, part) in &traffic.parts {
            at += u32::from(*step);
            let sender = format!("Peer{}-Realm", sender % 4);
            let part = lua.create_string(part).unwrap();
            let parts: Option<Vec<mlua::LuaString>> = check.call((sender, at, part)).unwrap();
            for part in parts.unwrap_or_default() {
                assert!(is_loggable(&part.as_bytes()), "{part:?}");
            }
        }
    });
});
