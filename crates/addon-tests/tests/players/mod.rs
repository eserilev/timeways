//! Two players with Timeways in one test: each one a game of its own, with a hub that
//! carries the addon messages between them (GAMEPLAY.md 4.7).

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(
    dead_code,
    reason = "each test file uses a different part of the harness"
)]

use crate::common::Game;

pub const REALM: &str = "Stormrage";

/// The event of the normal addon channel.
pub const NORMAL: &str = "CHAT_MSG_ADDON";
/// The event of the logged addon channel, which Blizzard support can read.
pub const LOGGED: &str = "CHAT_MSG_ADDON_LOGGED";

/// An addon message that a game sent. `event` is the event that the receiver gets.
pub struct Sent {
    pub prefix: String,
    pub text: String,
    pub channel: String,
    pub target: Option<String>,
    pub event: String,
}

pub struct Player {
    pub game: Game,
    pub name: &'static str,
}

impl Player {
    /// A player of the realm, logged in, in a guild with the others of the test.
    pub fn new(name: &'static str) -> Player {
        let game = Game::new();
        game.run(&format!(
            "wow.units.player = {{ name = '{name}', player = true, guid = 'Player-1-{name}' }}
             wow.zone = 'Tirisfal Glades'
             wow.subzone = 'Brill'"
        ));
        Player { game, name }
    }

    pub fn full_name(&self) -> String {
        format!("{}-{REALM}", self.name)
    }

    pub fn run(&self, code: &str) {
        self.game.run(code);
    }

    pub fn eval<T: mlua::FromLua>(&self, code: &str) -> T {
        self.game.eval(code)
    }

    /// Makes the other players members of the same guild, online.
    pub fn in_guild_with(&self, others: &[&Player]) {
        let mut members = vec![format!(
            "{{ name = '{}', online = true }}",
            self.full_name()
        )];
        for other in others {
            members.push(format!(
                "{{ name = '{}', online = true }}",
                other.full_name()
            ));
        }
        self.run(&format!(
            "wow.guild = {{ {} }} wow.Fire('GUILD_ROSTER_UPDATE', false)",
            members.join(", ")
        ));
    }

    /// Puts the other player in the party of this one, as `party1`.
    pub fn in_party_with(&self, other: &Player) {
        self.run(&format!(
            "wow.units.party1 = {{ name = '{}', player = true, guid = 'Player-1-{}' }}",
            other.name, other.name
        ));
    }

    pub fn leave_party(&self) {
        self.run("wow.units.party1 = nil");
    }

    /// The addon messages that this game sent since the last delivery.
    pub fn take_sent(&self) -> Vec<Sent> {
        let sent: Vec<mlua::Table> = self.eval("wow.addonSent");
        self.run("wow.addonSent = {}");
        sent.into_iter()
            .map(|message| Sent {
                prefix: message.get("prefix").unwrap(),
                text: message.get("text").unwrap(),
                channel: message.get("channel").unwrap(),
                target: message.get("target").unwrap(),
                event: message.get("event").unwrap(),
            })
            .collect()
    }

    /// Fires `CHAT_MSG_ADDON` as the game does for a message of `sender`.
    pub fn hear(&self, prefix: &str, text: &str, channel: &str, sender: &str) {
        self.hear_on(NORMAL, prefix, text, channel, sender);
    }

    /// Fires `CHAT_MSG_ADDON_LOGGED` as the game does for a logged message of `sender`.
    pub fn hear_logged(&self, prefix: &str, text: &str, channel: &str, sender: &str) {
        self.hear_on(LOGGED, prefix, text, channel, sender);
    }

    /// Fires `event`, the event of one of the two addon channels.
    pub fn hear_on(&self, event: &str, prefix: &str, text: &str, channel: &str, sender: &str) {
        let fire: mlua::Function = self.eval(
            "return function(event, prefix, text, channel, sender)
                 wow.Fire(event, prefix, text, channel, sender, '', 0, 0, '', 0)
             end",
        );
        fire.call::<()>((event, prefix, text, channel, sender))
            .unwrap();
    }

    /// Runs the timers of the game, as a second of play does.
    pub fn tick(&self) {
        self.run("wow.now = wow.now + 1; wow.RunTickers()");
    }

    pub fn printed(&self) -> Vec<String> {
        self.game.printed()
    }
}

fn is_for(target: Option<&str>, player: &Player) -> bool {
    target.is_none_or(|target| target == player.full_name() || target == player.name)
}

/// Carries every message that `from` sent to `to`, on the channel that it went on: a whisper
/// to `to`, and each message to the group or the guild. Returns how many it carried.
pub fn deliver(from: &Player, to: &Player) -> usize {
    let sent = from.take_sent();
    let mut count = 0;
    for message in sent {
        if is_for(message.target.as_deref(), to) {
            to.hear_on(
                &message.event,
                &message.prefix,
                &message.text,
                &message.channel,
                &from.full_name(),
            );
            count += 1;
        }
    }
    count
}

/// Carries messages both ways until none is left, with a second of time between rounds, so
/// a message that waits for the rate limit goes too.
pub fn exchange(a: &Player, b: &Player) {
    for _ in 0..50 {
        let carried = deliver(a, b) + deliver(b, a);
        let waiting: usize = a.eval::<usize>("ns.TaskChannel.Waiting()")
            + b.eval::<usize>("ns.TaskChannel.Waiting()");
        if carried == 0 && waiting == 0 {
            return;
        }
        a.tick();
        b.tick();
    }
    panic!("the two players never stopped talking");
}

/// Ada and Corvin, in one guild, both online.
pub fn ada_and_corvin() -> (Player, Player) {
    let ada = Player::new("Ada");
    let corvin = Player::new("Corvin");
    ada.in_guild_with(&[&corvin]);
    corvin.in_guild_with(&[&ada]);
    (ada, corvin)
}

/// The key of a task of Ada in the journal of the doer.
pub fn received_key(id: &str) -> String {
    format!("Ada-Stormrage/{id}")
}

/// Ada gives Corvin a task with these steps, as a Lua list, and Corvin accepts it.
/// Returns the two players and the id of the task.
pub fn accepted_task(steps: &str) -> (Player, Player, String) {
    let (ada, corvin) = ada_and_corvin();
    let id: String = ada.eval(&format!(
        "return ns.PlayerTasks.Give({{ title = 'Trouble at Agamand Mills', text = 'Put Gregor to rest.',
             reward = '5 gold', steps = {steps} }}, 'Corvin-Stormrage')"
    ));
    exchange(&ada, &corvin);
    corvin.run(&format!("ns.PlayerTasks.Accept('{}')", received_key(&id)));
    exchange(&ada, &corvin);
    (ada, corvin, id)
}
