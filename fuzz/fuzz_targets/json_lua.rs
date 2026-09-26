//! Random reply text through Json.lua of the addon, in Lua 5.1. A broken text gives nil and
//! a reason, never a Lua error.

#![no_main]

use libfuzzer_sys::fuzz_target;
use mlua::{Function, Lua, Value};

thread_local! {
    static DECODE: (Lua, Function) = {
        let lua = Lua::new();
        let source = include_str!("../../addon/Timeways/Json.lua");
        let ns = lua.create_table().unwrap();
        lua.load(source).set_name("Json.lua").call::<()>(("Timeways", ns.clone())).unwrap();
        let json: mlua::Table = ns.get("Json").unwrap();
        let decode: Function = json.get("Decode").unwrap();
        (lua, decode)
    };
}

fuzz_target!(|data: &[u8]| {
    DECODE.with(|(lua, decode)| {
        let text = lua.create_string(data).unwrap();
        let (value, reason): (Value, Option<String>) = decode.call(text).unwrap();
        assert!(value.is_nil() || reason.is_none());
    });
});
