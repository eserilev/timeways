//! Random addon messages of roleplay addons through MspParts.lua and MspWire.lua of the
//! addon, in Lua 5.1 (GAMEPLAY.md 3.7.1). A broken part or command is dropped, never a Lua
//! error. A command that passes names a field of two capital letters, and a field that
//! Timeways writes again reads back the same.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mlua::{Function, Lua, Table};

#[derive(Arbitrary, Debug)]
struct Traffic {
    /// Parts of messages: the sender (one of four), whether it came logged, a step of time in
    /// seconds, and the part.
    parts: Vec<(u8, bool, u8, Vec<u8>)>,
}

thread_local! {
    static CHECK: (Lua, Function) = {
        let lua = Lua::new();
        let bit: Table = lua.load(include_str!("../../addon/tests/bit.lua")).call(()).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let ns = lua.create_table().unwrap();
        for (name, source) in [
            ("MspWire.lua", include_str!("../../addon/Timeways/MspWire.lua")),
            ("MspParts.lua", include_str!("../../addon/Timeways/MspParts.lua")),
        ] {
            lua.load(source).set_name(name).call::<()>(("Timeways", ns.clone())).unwrap();
        }
        lua.globals().set("ns", ns).unwrap();
        let check: Function = lua
            .load(
                r#"
                local collector = ns.MspParts.NewCollector()
                local KINDS = { request = true, same = true, field = true }
                return function(sender, logged, at, part)
                    local whole = ns.MspParts.Add(collector, sender, part, logged, at)
                    if not whole then
                        return
                    end
                    local commands = ns.MspWire.Commands(whole)
                    assert(#commands <= ns.MspWire.MAX_COMMANDS, "too many commands")
                    for _, command in ipairs(commands) do
                        assert(KINDS[command.kind], "odd kind")
                        assert(command.field:find("^%u%u$"), "odd field")
                        assert(command.version:find("^%x*$"), "odd version")
                        if command.kind == "field" and command.text and not command.text:find("`") then
                            local again = ns.MspWire.Commands(ns.MspWire.Field(command.field, command.text))
                            assert(#again == 1 and again[1].text == command.text, "not the same text")
                        end
                    end
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
        for (sender, logged, step, part) in &traffic.parts {
            at += u32::from(*step);
            let sender = format!("Peer{}-Realm", sender % 4);
            let part = lua.create_string(part).unwrap();
            check.call::<()>((sender, *logged, at, part)).unwrap();
        }
    });
});
