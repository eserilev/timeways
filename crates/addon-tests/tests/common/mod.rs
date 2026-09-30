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

pub const TEST_KEY: &str = "0123456789abcdef0123456789abcdef";

/// The fuzz crate loads this harness too, so the root is the first folder up that holds
/// the addon.
fn addon_path(file: &str) -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !root.join("addon/Timeways").is_dir() {
        assert!(root.pop(), "no addon folder above the crate");
    }
    root.join("addon").join(file)
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

/// The name and the source of each file of the addon, read once for each process: the
/// fuzzers make a game for each input.
fn addon_sources() -> &'static [(String, String)] {
    static SOURCES: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    SOURCES.get_or_init(|| {
        let files = toc_files();
        let read = |file: String| {
            let source = std::fs::read_to_string(addon_path(&format!("Timeways/{file}")));
            (file, source.unwrap())
        };
        files.into_iter().map(read).collect()
    })
}

fn test_source(file: &str) -> &'static str {
    static BIT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    static WOW: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let cell = if file == "bit.lua" { &BIT } else { &WOW };
    cell.get_or_init(|| std::fs::read_to_string(addon_path(&format!("tests/{file}"))).unwrap())
}

pub struct Game {
    pub lua: Lua,
}

impl Game {
    /// The addon after login, with a fake link to a quick bridge: each tick, the bridge
    /// answers every batch that it took. `linkUp` and `linkLimit` in Lua change what the
    /// link takes.
    pub fn new() -> Game {
        let game = Game::before_login();
        game.run("ns.Outbox.SetCharacter(ns.Inputs.Character('Stormrage', 'Ada'))");
        game
    }

    /// The addon before the login event named the character.
    pub fn before_login() -> Game {
        let game = Game::with_transport();
        game.run(
            r#"sent = {}
             answered = 0
             linkUp = true
             linkLimit = 3000
             ns.Link.Fits = function(text) return #text <= linkLimit end
             ns.Link.Send = function(text)
                 if not linkUp or #text > linkLimit then return nil end
                 table.insert(sent, text)
                 return #sent
             end
             C_Timer.NewTicker(1, function()
                 while answered < #sent do
                     answered = answered + 1
                     ns.Link.Receive(answered, "done", '{"type":"events_seen","narrator":null}')
                 end
             end)"#,
        );
        game
    }

    /// The addon with the real seam: the shared `Messages.lua` of Gnomish Relay.
    pub fn with_transport() -> Game {
        let lua = Lua::new();
        let bit: Table = lua
            .load(test_source("bit.lua"))
            .set_name("bit.lua")
            .call(())
            .unwrap();
        lua.globals().set("bit", bit).unwrap();
        let wow: Table = lua
            .load(test_source("wow.lua"))
            .set_name("wow.lua")
            .call(())
            .unwrap();
        lua.globals().set("wow", wow).unwrap();
        let ns = lua.create_table().unwrap();
        // Setup writes the real key into Key.lua. The tests sign with this one.
        ns.set("key", TEST_KEY).unwrap();
        for (file, source) in addon_sources() {
            lua.load(source.as_str())
                .set_name(file.as_str())
                .call::<()>((ADDON, ns.clone()))
                .unwrap();
        }
        lua.globals().set("ns", ns).unwrap();
        Game { lua }
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

    /// Each line of each message after the character line, read as the story program
    /// reads it, with the message id that the bridge adds.
    pub fn sent_inputs(&self) -> Vec<Input> {
        let mut inputs = Vec::new();
        for (id, message) in self.sent().iter().enumerate() {
            for line in message.lines().skip(1) {
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
