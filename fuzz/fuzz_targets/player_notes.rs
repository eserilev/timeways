//! Remembered players (GAMEPLAY.md 4.9) through the real addon: a random `TimewaysPlayers`
//! from another addon, then the right-click menu, the note editor, and tooltips. No Lua
//! error comes of it. Each entry that stays has a known mark or a short note with no `|`,
//! the cap holds, and no note goes to the desktop or to another player.

#![no_main]

#[path = "any_value.rs"]
mod any_value;
#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;

use any_value::AnyValue;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use mlua::{Function, Lua, Table, Value};

const NAMES: [&str; 5] = [
    "Bram-Stormrage",
    "Cora-ArgentDawn",
    "Ada-Stormrage",
    "Bram",
    "Dax-Stormrage",
];
const MARKS: [&str; 5] = ["friendly", "neutral", "avoid", "Friendly", ""];
const MENUS: [&str; 5] = [
    "MENU_UNIT_PLAYER",
    "MENU_UNIT_ENEMY_PLAYER",
    "MENU_UNIT_PARTY",
    "MENU_UNIT_RAID_PLAYER",
    "MENU_UNIT_FRIEND",
];
/// More players than the cap of 300, so a saved table can be too long.
const MAX_FILLER: u16 = 400;

#[derive(Arbitrary, Debug)]
enum Entry {
    Note {
        mark: u8,
        note: Option<Vec<u8>>,
        at: f64,
    },
    Any(AnyValue),
}

#[derive(Arbitrary, Debug)]
enum Saved {
    Entries {
        entries: Vec<(u8, Entry)>,
        filler: u16,
    },
    Any(AnyValue),
}

/// A player as a menu names it: one of the known names, or any bytes.
#[derive(Arbitrary, Debug)]
enum Who {
    Known(u8),
    Raw(Vec<u8>),
}

#[derive(Arbitrary, Debug)]
enum Action {
    /// Opens the menu of a player and clicks an item of its Remember part.
    Menu {
        menu: u8,
        who: Who,
        server: Option<Vec<u8>>,
        item: u8,
    },
    /// Writes a note in the editor, when it is open, and clicks Save.
    Write(Vec<u8>),
    Tooltip(u8),
    /// Another addon writes the saved variable again.
    Replace(Saved),
}

#[derive(Arbitrary, Debug)]
struct Play {
    saved: Saved,
    actions: Vec<Action>,
}

fn note_table(lua: &Lua, mark: u8, note: Option<&[u8]>, at: f64) -> Value {
    let table = lua.create_table().unwrap();
    table
        .set("mark", MARKS[usize::from(mark) % MARKS.len()])
        .unwrap();
    if let Some(note) = note {
        table.set("note", lua.create_string(note).unwrap()).unwrap();
    }
    table.set("at", at).unwrap();
    Value::Table(table)
}

fn players_table(lua: &Lua, entries: &[(u8, Entry)], filler: u16) -> Value {
    let table = lua.create_table().unwrap();
    for n in 0..filler % MAX_FILLER {
        let entry = lua.create_table().unwrap();
        entry.set("mark", "neutral").unwrap();
        entry.set("at", n).unwrap();
        table.set(format!("Filler{n}-Stormrage"), entry).unwrap();
    }
    for (who, entry) in entries {
        let name = NAMES[usize::from(*who) % NAMES.len()];
        let entry = match entry {
            Entry::Note { mark, note, at } => note_table(lua, *mark, note.as_deref(), *at),
            Entry::Any(value) => value.to_lua(lua),
        };
        table.set(name, entry).unwrap();
    }
    Value::Table(table)
}

fn saved_value(lua: &Lua, saved: &Saved) -> Value {
    match saved {
        Saved::Entries { entries, filler } => players_table(lua, entries, *filler),
        Saved::Any(value) => value.to_lua(lua),
    }
}

fn context(lua: &Lua, who: &Who, server: Option<&Vec<u8>>) -> Table {
    let context = lua.create_table().unwrap();
    let name = match who {
        Who::Known(index) => lua.create_string(NAMES[usize::from(*index) % NAMES.len()]),
        Who::Raw(bytes) => lua.create_string(bytes),
    };
    context.set("name", name.unwrap()).unwrap();
    if let Some(server) = server {
        context
            .set("server", lua.create_string(server).unwrap())
            .unwrap();
    }
    context
}

const CLICK: &str = "return function(tag, context, item)
    local root = wow.OpenMenu(tag, context)
    for _, part in ipairs(root.items) do
        if part.text == 'Remember' then
            local items = {}
            for _, sub in ipairs(part.items) do
                if sub.callback then table.insert(items, sub) end
            end
            items[item % #items + 1].callback()
        end
    end
end";

const WRITE: &str = "return function(note)
    if ns.Editor.IsShown() then
        wow.EditBox():SetText(note)
        wow.Button('Save'):Click()
    end
end";

/// The rules of GAMEPLAY.md 4.9 that hold after every action. Opening the editor asks the
/// desktop for a journal page, so the check looks for the notes in what went out. A note of
/// 8 bytes or more is too long to show up there by chance.
const CHECK: &str = "local function WentOut(note)
    for _, text in ipairs(sent) do
        if text:find(note, 1, true) then return 'to the desktop' end
    end
    for _, message in ipairs(wow.addonSent) do
        if message.text:find(note, 1, true) then return 'to a player' end
    end
end
return function()
    ns.PlayerNotes.Of('')
    local count = 0
    for name, entry in pairs(TimewaysPlayers) do
        count = count + 1
        assert(type(name) == 'string' and ns.TaskPeople.Full(name) == name, 'odd name')
        assert(entry.mark == nil or ns.PlayerNotes.LABELS[entry.mark], 'odd mark')
        local note = entry.note
        assert(note == nil or type(note) == 'string', 'odd note')
        assert(note == nil or (note ~= '' and #note <= ns.PlayerNotes.MAX_BYTES), 'long note')
        assert(note == nil or not note:find('[%c|]'), 'note with a | or a control character')
        assert(entry.mark or note, 'empty entry')
        assert(type(entry.at) == 'number', 'no time')
        local out = note and #note >= 8 and WentOut(note)
        assert(not out, 'a note went out ' .. tostring(out))
    end
    assert(count <= ns.PlayerNotes.MAX_PLAYERS, 'more players than the cap')
end";

fn act(game: &common::Game, action: &Action) {
    let lua = &game.lua;
    match action {
        Action::Menu {
            menu,
            who,
            server,
            item,
        } => {
            let click: Function = game.eval(CLICK);
            let tag = MENUS[usize::from(*menu) % MENUS.len()];
            click
                .call::<()>((tag, context(lua, who, server.as_ref()), *item))
                .unwrap();
        }
        Action::Write(note) => {
            let write: Function = game.eval(WRITE);
            write.call::<()>(lua.create_string(note).unwrap()).unwrap();
        }
        Action::Tooltip(who) => {
            let short = ["Bram", "Cora", "Ada", "Dax"][usize::from(*who) % 4];
            game.run(&format!(
                "wow.units.mouseover = {{ name = '{short}', player = true }}
                 wow.ShowTooltip('mouseover')"
            ));
        }
        Action::Replace(saved) => {
            lua.globals()
                .set("TimewaysPlayers", saved_value(lua, saved))
                .unwrap();
        }
    }
}

fuzz_target!(|play: Play| {
    let game = common::Game::new();
    game.run("wow.units.player = { name = 'Ada', player = true, guid = 'Player-1-Ada' }");
    game.lua
        .globals()
        .set("TimewaysPlayers", saved_value(&game.lua, &play.saved))
        .unwrap();
    let check: Function = game.eval(CHECK);
    check.call::<()>(()).unwrap();
    for action in play.actions.iter().take(64) {
        act(&game, action);
        check.call::<()>(()).unwrap();
    }
});
