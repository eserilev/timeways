//! Loads the Timeways addon into Lua 5.1 with the fake WoW API of `addon/tests/wow.lua`.

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different part of the harness"
)]

use mlua::{Lua, Table};
use std::path::PathBuf;
use timeways_story::input::Input;

pub const ADDON: &str = "Timeways";

fn addon_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../addon")
        .join(file)
}

/// The Lua files in the order of the TOC, as WoW loads them.
fn toc_files() -> Vec<String> {
    let toc = std::fs::read_to_string(addon_path("Timeways/Timeways.toc")).unwrap();
    toc.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

pub struct Game {
    pub lua: Lua,
}

impl Game {
    /// The addon with a fake link. `linkUp` and `linkLimit` in Lua change what it takes.
    pub fn new() -> Game {
        let lua = Lua::new();
        let wow = std::fs::read_to_string(addon_path("tests/wow.lua")).unwrap();
        let wow: Table = lua.load(wow).set_name("wow.lua").call(()).unwrap();
        lua.globals().set("wow", wow).unwrap();
        let ns = lua.create_table().unwrap();
        for file in toc_files() {
            let source = std::fs::read_to_string(addon_path(&format!("Timeways/{file}"))).unwrap();
            lua.load(source)
                .set_name(file)
                .call::<()>((ADDON, ns.clone()))
                .unwrap();
        }
        lua.globals().set("ns", ns).unwrap();
        let game = Game { lua };
        game.run(
            "sent = {}
             linkUp = true
             linkLimit = 3000
             ns.Link = {
                 Fits = function(text) return #text <= linkLimit end,
                 Send = function(text)
                     if linkUp then table.insert(sent, text) end
                     return linkUp
                 end,
             }",
        );
        game
    }

    pub fn run(&self, code: &str) {
        self.lua.load(code).exec().unwrap();
    }

    pub fn eval<T: mlua::FromLua>(&self, code: &str) -> T {
        self.lua.load(code).eval().unwrap()
    }

    /// The text of each message that the link took, oldest first.
    pub fn sent(&self) -> Vec<String> {
        self.eval("sent")
    }

    /// Each line of each message, read as the story program reads it, with the message
    /// id that the bridge adds.
    pub fn sent_inputs(&self) -> Vec<Input> {
        let mut inputs = Vec::new();
        for (id, message) in self.sent().iter().enumerate() {
            for line in message.lines() {
                let mut value: serde_json::Value = serde_json::from_str(line).unwrap();
                value["id"] = serde_json::json!(id + 1);
                inputs.push(serde_json::from_value(value).unwrap());
            }
        }
        inputs
    }

    pub fn printed(&self) -> Vec<String> {
        self.eval("wow.printed")
    }

    pub fn reply(&self, text: &str) {
        let on_reply: mlua::Function = self.eval("ns.OnReply");
        on_reply.call::<()>(text).unwrap();
    }
}
