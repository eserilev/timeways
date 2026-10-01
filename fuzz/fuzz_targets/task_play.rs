//! Random play of player tasks through the real addon (GAMEPLAY.md 4.7): addon messages of
//! peers, the answers of the player, trades, and a random saved file at the start. No Lua
//! error comes of it, each addon message fits in 255 bytes, and no line of a page holds a
//! `|`, which starts a WoW escape.

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

const TYPES: [&str; 10] = [
    "hello", "here", "offer", "accept", "decline", "block", "cancel", "step", "turnin", "result",
];
const IDS: [&str; 3] = ["a1", "b2", "zz9"];
const SENDERS: [&str; 3] = ["Ada-Stormrage", "Bram-Stormrage", "Mallory-Elsewhere"];

#[derive(Arbitrary, Debug)]
enum Action {
    /// Any text as an addon message.
    Raw {
        sender: u8,
        text: Vec<u8>,
    },
    /// A message of a known type and id, with any fields after them.
    Message {
        sender: u8,
        kind: u8,
        id: u8,
        fields: Vec<String>,
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
    Party(bool),
    Tick,
}

#[derive(Arbitrary, Debug)]
struct Play {
    saved: Option<Saved>,
    actions: Vec<Action>,
}

fn escape(field: &str) -> String {
    field.replace('%', "%25").replace(';', "%3B")
}

const ANSWERS: [&str; 5] = ["Accept", "Decline", "Block", "GiveUp", "AskTurnIn"];

fn act(corvin: &players::Player, action: &Action) {
    match action {
        Action::Raw { sender, text } => {
            let text = String::from_utf8_lossy(text);
            corvin.hear(
                "Timeways",
                &text,
                "WHISPER",
                SENDERS[usize::from(*sender) % 3],
            );
        }
        Action::Message {
            sender,
            kind,
            id,
            fields,
        } => {
            let mut parts = vec![
                "1".to_string(),
                TYPES[usize::from(*kind) % TYPES.len()].to_string(),
                IDS[usize::from(*id) % IDS.len()].to_string(),
            ];
            parts.extend(fields.iter().map(|field| escape(field)));
            let text = format!("1:1:1:{}", parts.join(";"));
            if text.len() <= 255 {
                corvin.hear(
                    "Timeways",
                    &text,
                    "WHISPER",
                    SENDERS[usize::from(*sender) % 3],
                );
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
             end",
    );
}

fuzz_target!(|play: Play| {
    let ada = players::Player::new("Ada");
    let corvin = players::Player::new("Corvin");
    corvin.in_guild_with(&[&ada]);
    if let Some(saved) = &play.saved {
        corvin.run(&format!("TimewaysTasks = {}", saved.lua(0)));
    }
    for action in play.actions.iter().take(64) {
        act(&corvin, action);
    }
    check_pages(&corvin);
});
