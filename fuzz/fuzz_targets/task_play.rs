//! Random play of player tasks and stories through the real addon (GAMEPLAY.md 4.7 and
//! 4.8): addon messages of peers on both addon channels, the answers of the player, trades,
//! and random saved files at the start. No Lua error comes of it, each addon message fits in
//! 255 bytes, no line of a page holds a `|`, which starts a WoW escape, and no type with
//! text of a player gets in on the normal channel.

#![no_main]

#[path = "../../crates/addon-tests/tests/common/mod.rs"]
mod common;
#[path = "../../crates/addon-tests/tests/players/mod.rs"]
mod players;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

/// A value of the saved file, as Lua writes it.
#[derive(Arbitrary, Debug)]
enum Saved {
    Number(i32),
    Text(String),
    Bool(bool),
    Table(Vec<(String, Saved)>),
    List(Vec<Saved>),
}

impl Saved {
    fn lua(&self, depth: u8) -> String {
        match self {
            _ if depth > 4 => "nil".to_string(),
            Saved::Number(n) => n.to_string(),
            Saved::Text(text) => format!("{text:?}"),
            Saved::Bool(b) => b.to_string(),
            Saved::Table(fields) => {
                let fields: Vec<String> = fields
                    .iter()
                    .map(|(key, value)| format!("[{key:?}] = {}", value.lua(depth + 1)))
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
            Saved::List(items) => {
                let items: Vec<String> = items.iter().map(|item| item.lua(depth + 1)).collect();
                format!("{{ {} }}", items.join(", "))
            }
        }
    }
}

const TYPES: [&str; 13] = [
    "hello",
    "here",
    "offer",
    "accept",
    "decline",
    "block",
    "cancel",
    "step",
    "turnin",
    "result",
    "story",
    "story_accept",
    "story_decline",
];
const IDS: [&str; 3] = ["a1", "b2", "zz9"];
const SENDERS: [&str; 3] = ["Ada-Stormrage", "Bram-Stormrage", "Mallory-Elsewhere"];

/// The addon channel that a message comes on.
#[derive(Arbitrary, Debug, Clone, Copy)]
enum Log {
    Normal,
    Logged,
}

impl Log {
    fn event(self) -> &'static str {
        match self {
            Log::Normal => players::NORMAL,
            Log::Logged => players::LOGGED,
        }
    }
}

#[derive(Arbitrary, Debug)]
enum Action {
    /// Any text as an addon message.
    Raw {
        sender: u8,
        text: Vec<u8>,
        log: Log,
    },
    /// A message of a known type and id, with any fields after them.
    Message {
        sender: u8,
        kind: u8,
        id: u8,
        fields: Vec<String>,
        log: Log,
    },
    Answer {
        which: u8,
        task: u8,
    },
    Trade {
        partner: u8,
        item: String,
        count: u8,
        money: u16,
    },
    /// Accept or decline a story that waits, or remove an accepted one.
    Story {
        which: u8,
        index: u8,
    },
    Party(bool),
    Tick,
}

#[derive(Arbitrary, Debug)]
struct Play {
    saved: Option<Saved>,
    actions: Vec<Action>,
}

fn escape(field: &str) -> String {
    field
        .replace('%', "%25")
        .replace(';', "%3B")
        .replace('\\', "%5C")
}

/// Fires the event of `log`, and tells the check of `watch_the_log` which event it is.
fn hear(corvin: &players::Player, text: &str, sender: u8, log: Log) {
    let event = log.event();
    corvin.run(&format!("fuzzEvent = '{event}'"));
    corvin.hear_on(
        event,
        "Timeways",
        text,
        "WHISPER",
        SENDERS[usize::from(sender) % 3],
    );
}

/// Every message that reaches the quests and the stories: a type with text of a player came
/// on the logged channel. The list of these types is written here again, apart from the addon.
fn watch_the_log(corvin: &players::Player) {
    corvin.run(
        "local logged = { offer = true, story = true, step = true, turnin = true }
         local receive = ns.PlayerTasks.Receive
         ns.PlayerTasks.Receive = function(sender, message, channel)
             assert(fuzzEvent == 'CHAT_MSG_ADDON_LOGGED' or not logged[message.type], message.type)
             return receive(sender, message, channel)
         end",
    );
}

const ANSWERS: [&str; 5] = ["Accept", "Decline", "Block", "GiveUp", "AskTurnIn"];

fn act(corvin: &players::Player, action: &Action) {
    match action {
        Action::Raw { sender, text, log } => {
            hear(corvin, &String::from_utf8_lossy(text), *sender, *log);
        }
        Action::Message {
            sender,
            kind,
            id,
            fields,
            log,
        } => {
            let mut parts = vec![
                "1".to_string(),
                TYPES[usize::from(*kind) % TYPES.len()].to_string(),
                IDS[usize::from(*id) % IDS.len()].to_string(),
            ];
            parts.extend(fields.iter().map(|field| escape(field)));
            let text = format!("1:1:1:{}", parts.join(";"));
            if text.len() <= 255 {
                hear(corvin, &text, *sender, *log);
            }
        }
        Action::Answer { which, task } => {
            let answer = ANSWERS[usize::from(*which) % ANSWERS.len()];
            corvin.run(&format!(
                "local list = ns.PlayerTasks.Received()
                 local entry = list[{} % math.max(#list, 1) + 1]
                 if entry then ns.PlayerTasks.{answer}(entry.key) end",
                task
            ));
        }
        Action::Trade {
            partner,
            item,
            count,
            money,
        } => {
            let partner = ["Ada", "Bram"][usize::from(*partner) % 2];
            let run: mlua::Function = corvin.eval(
                "return function(partner, item, count, money)
                     wow.Trade(partner, { gave = { { name = item, count = count } }, got = {}, money = money, moneyGot = 0 })
                 end",
            );
            run.call::<()>((partner, item.as_str(), *count, *money))
                .unwrap();
        }
        Action::Story { which, index } => {
            let answer = ["Accept", "Decline"][usize::from(*which) % 2];
            corvin.run(&format!(
                "local waiting = ns.PlayerStories.Waiting()
                 if #waiting > 0 then ns.PlayerStories.{answer}({index} % #waiting + 1) end"
            ));
        }
        Action::Party(on) => {
            corvin.run(if *on {
                "wow.units.party1 = { name = 'Ada', player = true, guid = 'Player-1-Ada' }"
            } else {
                "wow.units.party1 = nil"
            });
            corvin.run("wow.Fire('GROUP_ROSTER_UPDATE')");
        }
        Action::Tick => corvin.tick(),
    }
}

/// Every page of the Tasks section renders, and no line holds a `|`. An input of the form
/// has no text of its own.
fn check_pages(corvin: &players::Player) {
    corvin.run(
        "for _, row in ipairs(ns.Journal.Page('quests').list) do
                 if row.key then
                     ns.Journal.Select('quests', row.key)
                     for _, line in ipairs(ns.Journal.Page('quests').lines) do
                         local text = line.text or ''
                         assert(not text:find('|', 1, true), text)
                     end
                 end
             end
         for _, line in ipairs(ns.Journal.Lines('hero')) do
             assert(not (line.text or ''):find('|', 1, true), line.text)
         end",
    );
}

fuzz_target!(|play: Play| {
    let ada = players::Player::new("Ada");
    let corvin = players::Player::new("Corvin");
    corvin.in_guild_with(&[&ada]);
    watch_the_log(&corvin);
    if let Some(saved) = &play.saved {
        corvin.run(&format!("TimewaysTasks = {}", saved.lua(0)));
        corvin.run(&format!("TimewaysStories = {}", saved.lua(0)));
    }
    for action in play.actions.iter().take(64) {
        act(&corvin, action);
    }
    check_pages(&corvin);
});
