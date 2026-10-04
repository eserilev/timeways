//! Random addon messages of roleplay addons through MspParts.lua and MspWire.lua of the
//! addon, in Lua 5.1 (GAMEPLAY.md 3.7.1). A broken part or command is dropped, never a Lua
//! error. A command that passes names a field of two capital letters, and a field that
//! Timeways writes again reads back the same. The same parts also go through `Msp.Received`
//! of the real addon, and the tooltip line of each sender holds no lone `|`.

#![no_main]

#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mlua::{Function, Lua, Table};

#[derive(Arbitrary, Debug)]
struct Traffic<'a> {
    /// Parts of messages: the sender (one of four), whether it came logged, a step of time in
    /// seconds, and the part. A part is a slice of the input, so a token of the dictionary
    /// stays whole in it.
    parts: Vec<(u8, bool, u8, &'a [u8])>,
}

thread_local! {
    static CHECK: (Lua, Function) = {
        let lua = Lua::new();
        let bit: Table = lua.load(include_str!("../../addon/tests/bit.lua")).call(()).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let ns = lua.create_table().unwrap();
        for (name, source) in [
            ("PartCollector.lua", include_str!("../../addon/Timeways/PartCollector.lua")),
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

/// The game shows `||` as one `|`, and a lone `|` starts an escape, such as a fake link.
const RECEIVE: &str = r#"
    ns.MspProfile.SetSharing(true)
    return function(sender, logged, step, part)
        wow.now = wow.now + step
        ns.Msp.Received("MSP2", part, sender, logged)
        local line = ns.Msp.TooltipLine(sender)
        if line then
            assert(not line:gsub("||", ""):find("|", 1, true), "a lone | in the tooltip line")
        end
    end"#;

fn sender_of(sender: u8) -> String {
    format!("Peer{}-Realm", sender % 4)
}

fn check_parts(traffic: &Traffic<'_>) {
    CHECK.with(|(lua, check)| {
        let mut at = 0u32;
        for (sender, logged, step, part) in &traffic.parts {
            at += u32::from(*step);
            let part = lua.create_string(part).unwrap();
            check
                .call::<()>((sender_of(*sender), *logged, at, part))
                .unwrap();
        }
    });
}

thread_local! {
    /// One game for each fuzz process, as the collector of `CHECK`: a new game for each
    /// input is too slow to reach a kept field.
    static GAME: (common::Game, Function) = {
        let game = common::Game::new();
        let receive: Function = game.eval(RECEIVE);
        (game, receive)
    };
}

fn check_tooltips(traffic: &Traffic<'_>) {
    GAME.with(|(game, receive)| {
        for (sender, logged, step, part) in &traffic.parts {
            let part = game.lua.create_string(part).unwrap();
            receive
                .call::<()>((sender_of(*sender), *logged, *step, part))
                .unwrap();
        }
    });
}

fuzz_target!(|traffic: Traffic<'_>| {
    check_parts(&traffic);
    check_tooltips(&traffic);
});
