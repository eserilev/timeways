//! Random addon messages through TaskChunks.lua and TaskWire.lua of the addon, in Lua 5.1
//! (GAMEPLAY.md 4.7). A broken part or message is dropped, never a Lua error. A message that
//! passes holds only clean, bounded text, and comes through the wire again unchanged.

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
                end"#,
            )
            .eval()
            .unwrap();
        (lua, check)
    };
}

fuzz_target!(|traffic: Traffic| {
    CHECK.with(|(lua, check)| {
        let mut at = 0u32;
        for (sender, step, part) in &traffic.parts {
            at += u32::from(*step);
            let sender = format!("Peer{}-Realm", sender % 4);
            let part = lua.create_string(part).unwrap();
            check.call::<()>((sender, at, part)).unwrap();
        }
    });
});
